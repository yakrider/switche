#![ allow (non_snake_case) ]

use tauri::{SystemTray, SystemTrayMenu, SystemTrayMenuItem, CustomMenuItem, AppHandle, Wry, GlobalShortcutManager, WindowEvent, RunEvent, SystemTrayEvent, Manager};

use crate::{win_apis, autostart};
use crate::switche::{Hwnd, SwitcheState};



const MENU_AUTO_START       : &str = "auto_start";
const MENU_AUTO_START_ADMIN : &str = "auto_start_admin";
const MENU_EDIT_CONF        : &str = "edit_conf";
const MENU_RESET_CONF       : &str = "reset_conf";
const MENU_RELOAD           : &str = "reload";
const MENU_RESTART          : &str = "restart";
const MENU_QUIT             : &str = "quit";
// note: ^^ its easier to define these as consts instead of enums as that makes it easier to match against id-strings later


fn make_menu_item (id:&str) -> CustomMenuItem {
    let disp_str = match id {
        MENU_AUTO_START       => "Auto-Start on Login",
        MENU_AUTO_START_ADMIN => "Auto-Start as Admin",
        MENU_EDIT_CONF        => "Edit Config",
        MENU_RESET_CONF       => "Reset Config",
        MENU_RELOAD           => "Reload",
        MENU_RESTART          => "Restart",
        MENU_QUIT             => "Quit",
        _ => ""
    };
    CustomMenuItem::new (id, disp_str)
}

fn exec_menu_action (id:&str, ss:&SwitcheState, ah:&AppHandle<Wry>) {
    match id {
        MENU_AUTO_START       => { autostart::proc_tray_event__toggle_switche_autostart (false, ah) }
        MENU_AUTO_START_ADMIN => { autostart::proc_tray_event__toggle_switche_autostart (true,  ah) }
        MENU_EDIT_CONF        => { ss.conf.trigger_config_file_edit() }
        MENU_RESET_CONF       => { ss.conf.trigger_config_file_reset() }
        MENU_RELOAD           => { ss.proc_menu_req__switche_reload() }
        MENU_RESTART          => { ah.restart() }
        MENU_QUIT             => { ah.exit(0) }
        _ => { }
    }
}




pub fn run_switche_tauri (ss:&SwitcheState) {

    // we'll setup tray-icon support to pass into app builder
    let tray = SystemTray::new() .with_tooltip("Switche") .with_menu (
        SystemTrayMenu::new ()
            // first we'll put the configs
            .add_item ( make_menu_item (MENU_AUTO_START) )
            .add_item ( make_menu_item (MENU_AUTO_START_ADMIN) )
            .add_native_item ( SystemTrayMenuItem::Separator )

            // the special entry to trigger opening the config file for editing
            .add_item ( make_menu_item (MENU_EDIT_CONF) )
            .add_item ( make_menu_item (MENU_RESET_CONF) )
            .add_native_item ( SystemTrayMenuItem::Separator )

            // then the actions
            .add_item ( make_menu_item (MENU_RELOAD) )
            .add_item ( make_menu_item (MENU_RESTART) )
            .add_item ( make_menu_item (MENU_QUIT) )
    );


    let app = {
        tauri::Builder::default() .setup ( {
            let ss = ss.clone();
            move |app| {
                setup_global_shortcuts (&ss, &app.handle());
                ss.setup_front_end_listener (&app.handle());
                app.set_device_event_filter(tauri::DeviceEventFilter::Always);
                // ^^ w/o this, our own LL input hooks will not receive events when tauri window is fgnd
                Ok(())
            }
        } )
        .invoke_handler (tauri::generate_handler![])
        .system_tray (tray)
        .on_system_tray_event ({
            let ss = ss.clone();
            move |ah, event| { tray_events_handler (&ss, ah, event) }
        })
        .build (tauri::generate_context!())
        .expect ("error while building tauri application")
    };

    // we'll setup the switche window always-on-top behavior based on configs
    app .windows() .get("main") .map (|w|
        w.set_always_on_top (ss.conf.check_flag__auto_hide_enabled())
    );

    // we'll also check if we're admin, or the corresponding auto-start tasks are active and sync our tray flag accordingly
    autostart::update_tray_auto_start_admin_flags (&app.app_handle());

    // we'll register the app with the switche engine so it can send back responses etc
    ss.register_app_handle(app.handle());

    // just a reminder that configs load at instantiation, and everytime the app is reloaded

    // now lets finally actually start the app! .. note that the run call wont return!
    let ssc = ss.clone();
    app .run ( move |ah, event| { tauri_run_events_handler (&ssc, ah, event) } );

}





fn tauri_window_events_handler (ss:&SwitcheState, _ah:&AppHandle, ev:&WindowEvent) {
    match ev {
        WindowEvent::Focused (true)       => { ss.proc_app_window_event__focus() }
        WindowEvent::Focused (false)      => { ss.proc_app_window_event__focus_lost() }
        WindowEvent::Moved (..)           => { ss.conf.deferred_update_conf__switche_window(ss) }
        WindowEvent::Resized (..)         => { ss.conf.deferred_update_conf__switche_window(ss) }
        WindowEvent::CloseRequested {..}  => { }
        _ => { }
    }
}


fn extract_self_hwnd (ss:&SwitcheState) -> Option<Hwnd> {
    ss.app_handle.read().unwrap() .as_ref() .and_then (|ah| {
        ah.windows().values() .next() .and_then (|w| w.hwnd().ok())
    } ) .map (|h| h.0)
}

pub fn auto_setup_self_window (ss:&SwitcheState) {
    let wa = win_apis::win_get_work_area();

    //let (x, y, width, height) = ( wa.left + (wa.right-wa.left)/3, 0, (wa.right-wa.left)/2,  wa.bottom - wa.top);
    // ^^ reasonable initial version .. half screen width, full screen height, starting 1/3 from left edge

    // vertically we'll span the full usable height
    let (y, height) = (0, wa.bottom - wa.top);

    // horizontally we'll span upto half the screen width, but no more than dpi scaled 860px
    let max_width = ss.app_handle.read().unwrap().as_ref() .and_then (|ah|
        ah .get_window("main") .and_then (|w| w.scale_factor().ok())
    ) .unwrap_or (1.0)  * 860.0;
    let width = std::cmp::min ( (wa.right - wa.left)/2, max_width as i32 );

    // for x, ideally want centerline at 1/3 of the window width (aiming for icons column), as long as it go beyond right edge
    let max_x = wa.right - width;
    let x = std::cmp::min ( max_x, wa.left + (wa.right - wa.left)/2 - width/3 );

    win_apis::win_move_to (ss.get_self_hwnd(), x, y, width, height);
}
pub fn setup_self_window (ss:&SwitcheState) {
    // if there's a valid config, and the sizes are not zero (like at startup), we'll use those
    //if ss.conf.check_flag__restore_window_dimensions() {
    // ^^ disabled as we'd still rather use (valid) dimensions from configs even if not set to restore last-closed position
    if let Some ((x,y,w,h)) = ss.conf.read_conf__window_dimensions() {
        if w > 0 && h > 0 {
            win_apis::win_move_to (ss.get_self_hwnd(), x, y, w, h);
            return
    } }
    // else the default is to auto-calc window dimensions
    auto_setup_self_window (ss);
}

fn proc_event_app_ready (ss:&SwitcheState, _ah:&AppHandle<Wry>) {
    // we want to store a cached value of our hwnd for exclusions-mgr (and general use)
    if let Some(hwnd) = extract_self_hwnd(ss) {
        //println! ("App starting .. self-hwnd is : {:?}", hwnd );
        ss.store_self_hwnd(hwnd);
        setup_self_window (ss);
        //tauri_setup_self_window(ah);
    }
}
pub fn tauri_run_events_handler (ss:&SwitcheState, ah:&AppHandle<Wry>, event:RunEvent) {
    match event {
        RunEvent::Ready                        => { proc_event_app_ready (ss, ah) }
        RunEvent::WindowEvent   { event, .. }  => { tauri_window_events_handler (ss, ah, &event) }
        RunEvent::ExitRequested { .. }         => { /* api.prevent_exit() */ }
        _ => {}
    }
}




fn proc_tray_event__left_click (ss:&SwitcheState) {
    ss.checked_self_activate();
}
fn proc_tray_event__right_click (_ss:&SwitcheState) {
    // nothign to do to bring up the menu?
}
fn proc_tray_event__double_click (_ss:&SwitcheState) {
    // nothing as we trigger 'show' upon single click
}

pub fn tray_events_handler (ss:&SwitcheState, ah:&AppHandle<Wry>, event:SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick   { .. }  =>  { proc_tray_event__left_click(ss) },
        SystemTrayEvent::RightClick  { .. }  =>  { proc_tray_event__right_click(ss) },
        SystemTrayEvent::DoubleClick { .. }  =>  { proc_tray_event__double_click(ss) },
        SystemTrayEvent::MenuItemClick { id, .. }  =>  { exec_menu_action (id.as_str(), ss, ah) },
        _ => { }
    }
}




pub fn setup_global_shortcuts (ss:&SwitcheState, ah:&AppHandle<Wry>) {

    fn register_hotkeys <HGF> (ah:&AppHandle<Wry>, ss: &SwitcheState, hotkeys: &Vec<String>, handler_gen: HGF) where
        HGF: FnOnce(SwitcheState) -> Box <dyn Fn() + Send + 'static> + Clone    // handler extraction function type
    {
        hotkeys .iter() .for_each (|hotkey| {
            if let Err(e) = ah.global_shortcut_manager().register (hotkey, handler_gen.clone() (ss.clone())) {
                println! ("Failed to register hotkey {:?}: {}", hotkey, e);
            }
        });
    }
    // before user config hotkeys, we'll register alt-space so

    // we register all hotkeys specified in config file for switche invocation ..
    // .. typically should include F1 for invocation and Ctrl+Alt+F15 for krusty remapping of F1
    register_hotkeys (ah, ss, &ss.conf.get_switche_invocation_hotkeys(), |ss| Box::new (move || ss.proc_hot_key__invoke()) );


    // we register all hotkeys specified in config file for direct switch to n-th last-windows ..
    // .. typically should include Ctrl+Alt+F16 (through F18) for the last (through third-last)
    register_hotkeys (ah, ss, &ss.conf.get_direct_last_window_switch_hotkeys(), |ss| Box::new (move || ss.proc_hot_key__switch_z_idx(1)) );
    register_hotkeys (ah, ss, &ss.conf.get_second_last_window_switch_hotkeys(), |ss| Box::new (move || ss.proc_hot_key__switch_z_idx(2)) );
    register_hotkeys (ah, ss, &ss.conf.get_third_last_window_switch_hotkeys(),  |ss| Box::new (move || ss.proc_hot_key__switch_z_idx(3)) );


    // and for hotkeys specified in config file to take snapshot of windows, and switch through them without bringing switche up
    register_hotkeys (ah, ss, &ss.conf.get_windows_list_snapshot_hotkeys(),  |ss| Box::new (move || ss.proc_hot_key__snap_list_refresh() ) );
    register_hotkeys (ah, ss, &ss.conf.get_snap_list_switch_next_hotkeys(),  |ss| Box::new (move || ss.proc_hot_key__snap_list_switch_next() ) );
    register_hotkeys (ah, ss, &ss.conf.get_snap_list_switch_prev_hotkeys(),  |ss| Box::new (move || ss.proc_hot_key__snap_list_switch_prev() ) );

    // finally, we'll also register any hotkeys specified in config file for direct switch to specific exe/title
    ss.conf.get_direct_app_switch_hotkeys() .into_iter() .for_each (|(hotkey, exe, title, partial)| {
        register_hotkeys (ah, ss, &vec![hotkey], |ss| Box::new ( move || ss.proc_hot_key__switch_app (exe.as_deref(), title.as_deref(), partial) ) );
    });

}
