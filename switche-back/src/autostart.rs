#![allow (non_snake_case)]

use std::error::Error;
use std::thread::spawn;

use tracing::{info, error, warn};

use planif::enums::TaskCreationFlags;
use planif::schedule::TaskScheduler;
use planif::schedule_builder::{Action, ScheduleBuilder};
use planif::settings::{Duration, LogonType, PrincipalSettings, RunLevel, Settings};

use crate::{win_apis, tray, config};


fn autostart_task_name () -> Option<String> {
    win_apis::get_cur_user_name() .map (|user| format! ("\\Switche__{user}"))
}
fn autostart_elev_task_name () -> Option<String> {
    win_apis::get_cur_user_name() .map (|user| format! ("\\Switche_elev__{user}"))
}
fn autostart_task_folder() -> &'static str { "\\Switche" }


/// WARNING: checking task-sched via the planif crate seems to change some thread mode .. so better to call this in spawned thread
fn check_sched_task_enabled (name:&str) -> bool {
    _check_sched_task_enabled(name) .unwrap_or_else (|e| {
        error!("Error checking task enabled state : {:?}", e);
        false
    })
}
fn _check_sched_task_enabled (name:&str) -> Result<bool, Box<dyn Error>> {
    let ts = TaskScheduler::new()?;
    let com = ts.get_com();
    let sb = ScheduleBuilder::new(&com)?;
    sb.in_folder(autostart_task_folder())?.check_task_enabled(name)
}


fn set_sched_task_enabled_state (name:&str, state:bool) -> Result<(), Box<dyn Error>> {
    let res = _set_sched_task_enabled_state (name, state);
    if let Err(e) = res.as_ref() {
        error!("While setting task enabled state of {:?} to {:?}, got error {:?}", name, state, e);
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
        error!("Error setting up switche autostart (with admin={}) : {:?}", elev, e);
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
    let task = if elev { autostart_elev_task_name() } else { autostart_task_name() } .ok_or("Error getting task name")?;
    let swi_exe = std::env::current_exe().unwrap().to_string_lossy().to_string();
    info! (".. Creating auto start on logon task for:{:?}, elev:{:?}, exe:{:?}", user, elev, swi_exe);

    let principal = PrincipalSettings {
        display_name : "".to_string(),
        group_id : None,
        id : "".to_string(),
        logon_type : LogonType::InteractiveToken,
        run_level : if elev { RunLevel::Highest } else { RunLevel::LUA },
        user_id : Some(user.clone()),
    };

    let settings = Settings {
        allow_demand_start : Some(true),                // <- should be on by default too, but meh
        start_when_available : Some(true),              // <- if startup is missed, start whenever feasible
        restart_count : Some(5),                        // <- attempt to start 5 times
        restart_interval : Some("PT1M".into()),         // <- try every 1 minute if startup fails
        execution_time_limit : Some("PT0S".into()),     // <- set to no-force-kill .. OS seems to auto-set 3mins etc
        stop_if_going_on_batteries : Some(false),       // <- os would sets this on by default if unspecified
        disallow_start_if_on_batteries : Some(false),   // <- os would sets this on by default if unspecified
        .. Settings::default()
    };


    sb.create_logon()
        .trigger ("", true)?
        .author ("Switche")?
        .description ("Switche auto-start")?
        .settings (settings)?
        .principal (principal)?
        .user_id (&user)?
        .in_folder (autostart_task_folder())?
        .action (Action::new ("Switche.exe", &swi_exe, "", ""))?
        .delay ( Duration { seconds : Some(30), ..Default::default() } )?
        .build()?
        .register (&task, TaskCreationFlags::CreateOrUpdate as i32)

    // ^^ note that the order in which the actions are called seem to be important ..
    // .. for example, calling .trigger() later in the call seems to not have the trigger available for the delay call
}



pub fn proc_req__toggle_switche_autostart (toggle_elev:bool) {
    /*  for elev .. if elev-task enabled  .. disable it
        for elev .. if elev-task disabled .. enable it, and if successful, check as and disable it if enabled
        for n-el .. if n-el-task enabled  .. disable it
        for n-el .. if n-el-task disabled .. if as-e enabled, ignore req, else enable it
     */
    spawn ( move || {
        // lets again get the state of the tasks first
        let (el_t, nel_t) = (autostart_elev_task_name(), autostart_task_name());
        let el_en  = el_t  .as_ref() .is_some_and (|s| check_sched_task_enabled (s));
        let nel_en = nel_t .as_ref() .is_some_and (|s| check_sched_task_enabled (s));

        let menu = if toggle_elev { "elev" } else { "non-elev" };
        info! ("autostart toggle req on {menu:?} menu .. menu-states: el={el_en:?}, nel={nel_en:?})");

        if toggle_elev {
            if el_en {
                let _ = set_sched_task_enabled_state (el_t.as_ref().unwrap().as_str(), false);
            } else {
                let res = setup_task__switche_autostart(true);
                if res.is_ok() {
                    if nel_en {
                        let _ = set_sched_task_enabled_state (nel_t.as_ref().unwrap().as_str(), false);
                    }
                    if let Err(e) = create_elev_autostart_shortcut() {
                        warn! ("Failed to create elevated launcher link .. {e:?}");
                    }
                    let _ = set_sched_task_enabled_state (nel_t.as_ref().unwrap().as_str(), false);
                }
            }
        } else if nel_en {
            let _ = set_sched_task_enabled_state (nel_t.as_ref().unwrap().as_str(), false);
        } else if !el_en {
            // we'll only enable the non-elev task if the elev task is not enabled
            let _ = setup_task__switche_autostart(false);
        }
        tray::update_auto_start_flags ();
    } );
}



pub fn check_status_auto_start_task () -> bool {
    autostart_task_name() .is_some_and (|n| check_sched_task_enabled (&n))
}
pub fn check_status_auto_start_elev_task () -> bool {
    autostart_elev_task_name() .is_some_and (|n| check_sched_task_enabled (&n))
}


fn create_elev_autostart_shortcut () -> Result<(), String> {
    let Some(task_name) = autostart_elev_task_name() else {
        return Err ("Failed to get autostart elev task name".to_string());
    };
    let task = autostart_task_folder().to_string() + &task_name;

    let Some(mut link_loc) = config::Config::instance().get_log_loc() else {
        return Err ("Failed to get log location for creating shortcut".to_string());
    };
    link_loc .push ("Switche_elev.lnk");
    info! ("Creating Switche elevated launch shortcut at : {}", link_loc.display());
    if link_loc.exists() {
        std::fs::remove_file (&link_loc) .map_err (|e| format!("Failed to remove existing shortcut: {e}"))?;
    }

    let target_path = "C:\\Windows\\System32\\schtasks.exe";
    let args = format!("/run /tn \"{task}\"");
    let icon_loc = std::env::current_exe() .map (|p| p.to_string_lossy().to_string()) .map_err(|e| e.to_string())?;

    let mut link = mslnk::ShellLink::new (target_path) .map_err(|e| e.to_string())?;
    link.set_arguments (Some (args));
    link.set_icon_location (Some (icon_loc));

    link.create_lnk (link_loc.to_str().unwrap()) .map_err (|e| e.to_string())
}
