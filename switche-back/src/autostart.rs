#![ allow (non_snake_case) ]

use std::error::Error;
use std::thread::spawn;
use tauri::{AppHandle, Wry};

use planif::enums::TaskCreationFlags;
use planif::schedule::TaskScheduler;
use planif::schedule_builder::{Action, ScheduleBuilder};
use planif::settings::{Duration, LogonType, PrincipalSettings, RunLevel};

use crate::win_apis;




fn autostart_task_name (elev:bool) -> Option<String> {
    let user = win_apis::get_cur_user_name()?;
    if elev { Some ("\\Switche_elev__".to_string() + &user) }
    else    { Some ("\\Switche__".to_string()      + &user) }
}
fn autostart_task_folder() -> &'static str { "\\Switche" }



fn check_sched_task_enabled (name:&str) -> bool {
    match _check_sched_task_enabled (name) {
        Ok(b) => b,
        Err(_e) => {
            //println!("Error checking task enabled state : {:?}", _e);
            false
        }
    }
}
fn _check_sched_task_enabled (name:&str) -> Result<bool, Box<dyn Error>> {
    let ts = TaskScheduler::new()?;
    let com = ts.get_com();
    let sb = ScheduleBuilder::new(&com)?;
    sb.in_folder(autostart_task_folder())?.check_task_enabled(name)
}


fn set_sched_task_enabled_state (name:&str, state:bool) -> Result<(), Box<dyn Error>> {
    let res = _set_sched_task_enabled_state (name, state);
    if let Err(&ref e) = res.as_ref() {
        println!("While setting task enabled state of {:?} to {:?}, got error {:?}", name, state, e);
    }
    res
}
fn _set_sched_task_enabled_state (name:&str, state:bool) -> Result<(), Box<dyn Error>> {
    let ts = TaskScheduler::new()?;
    let com = ts.get_com();
    let sb = ScheduleBuilder::new(&com)?;
    sb.in_folder(autostart_task_folder())?.set_task_enabled(name, state)
}



fn setup_task__switche_autostart (elev:bool) -> Result<(), Box<dyn Error>> {
    let res = _setup_task__switche_autostart(elev);
    if let Err(e) = res.as_ref() {
        println!("Error setting up switche autostart (with admin={}) : {:?}", elev, e);
    }
    res
}
fn _setup_task__switche_autostart (elev:bool) -> Result<(), Box<dyn Error>> {
    if elev && win_apis::check_cur_proc_elevated() != Some(true) {
        return Err("Current process is not elevated".into())
    };
    let ts = TaskScheduler::new()?;
    let com = ts.get_com();
    let sb = ScheduleBuilder::new(&com)?;

    let user = win_apis::get_cur_user_name().ok_or("Error getting user name")?;
    let task = autostart_task_name(elev).ok_or("Error getting task name")?;
    let swi_exe = std::env::current_exe().unwrap().to_string_lossy().to_string();
    println! (".. Creating auto start on logon task for:{:?}, elev:{:?}, exe:{:?}", user, elev, swi_exe);

    let principal = PrincipalSettings {
        display_name: "".to_string(),
        group_id: None,
        id: "".to_string(),
        logon_type: LogonType::InteractiveToken,
        run_level: if elev { RunLevel::Highest } else { RunLevel::LUA },
        user_id: Some(user.clone()),
    };

    sb.create_logon()
        .trigger ("", true)?
        .author ("Switche")?
        .description ("Switche auto-start")?
        //.settings(_settings)?
        .principal (principal)?
        .user_id (&*user)?
        .in_folder (autostart_task_folder())?
        .action (Action::new ("Switche.exe", &*swi_exe, "", ""))?
        .delay ( Duration { seconds : Some(10), ..Default::default() } )?
        .build()?
        .register (&*task, TaskCreationFlags::CreateOrUpdate as i32)

    // ^^ note that the order in which the actions are called seem to be important ..
    // .. for example, calling .trigger() later in the call seems to not have the trigger available for the delay call
}



pub fn proc_tray_event__toggle_switche_autostart (elev:bool, ah:&AppHandle<Wry>) {
    /*  for elev .. if elev-task enabled  .. disable it
        for elev .. if elev-task disabled .. enable it, and if successful, check as and disable it if enabled
        for n-el .. if n-el-task enabled  .. disable it
        for n-el .. if n-el-task disabled .. if as-e enabled, ignore req, else enable it
     */
    let ah = ah.clone();
    spawn ( move || {
        // lets again get the state of the tasks first
        let (el_t, nel_t) = (autostart_task_name(true), autostart_task_name(false));
        let el_en  = el_t  .as_ref() .map_or (false, |s| check_sched_task_enabled (s));
        let nel_en = nel_t .as_ref() .map_or (false, |s| check_sched_task_enabled (s));

        if elev {
            if el_en {
                let _ = set_sched_task_enabled_state (el_t.as_ref().unwrap().as_str(), false);
            } else {
                let res = setup_task__switche_autostart(true);
                if res.is_ok() && nel_en {
                    let _ = set_sched_task_enabled_state (nel_t.as_ref().unwrap().as_str(), false);
                }
            }
        } else {
            if nel_en {
                let _ = set_sched_task_enabled_state (nel_t.as_ref().unwrap().as_str(), false);
            } else if !el_en {
                // we'll only enable the non-elev task if the elev task is not enabled
                let _ = setup_task__switche_autostart(false);
            }
        }
        update_tray_auto_start_admin_flags (&ah);
    } );
}



pub fn update_tray_auto_start_admin_flags (ah:&AppHandle<Wry>) {
    // note that this must be on a spawned thread because checking task-sched via the planif crate seems to change thread mode
    let ah = ah.clone();
    spawn ( move || {
        // first we'll check both tasks whether they are enabled
        let el_en = autostart_task_name(true) .map_or (false, |n| check_sched_task_enabled (&n));
        ah .tray_handle() .try_get_item("auto_start_admin") .map (|e| e.set_selected (el_en));

        let nel_en = autostart_task_name(false) .map_or (false, |n| check_sched_task_enabled (&n));
        ah .tray_handle() .try_get_item("auto_start") .map (|e| e.set_selected (nel_en));

        if win_apis::check_cur_proc_elevated().is_some_and(|b| b==true) {
            // while elev, elev tray-menu will be enabled, and non-elev will be disabled unless non-elev task is already enabled
            ah .tray_handle() .try_get_item("auto_start_admin") .map (|e| e.set_enabled (true) );
            ah .tray_handle() .try_get_item("auto_start") .map (|e| e.set_enabled (nel_en) );
        } else {
            // if not elev, elev tray-menu will be disabled, but non-elev will still be enabled if elev task is enabled and it is not
            ah .tray_handle() .try_get_item("auto_start_admin") .map (|e| e.set_enabled (false) );
            ah .tray_handle() .try_get_item("auto_start") .map (|e| e.set_enabled (nel_en || !el_en) );
        }
    } );
}
