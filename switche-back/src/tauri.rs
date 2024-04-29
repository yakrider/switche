#![ allow (non_snake_case) ]

use tauri::{SystemTray, SystemTrayMenu, SystemTrayMenuItem, CustomMenuItem, AppHandle, Wry, GlobalShortcutManager, WindowEvent, RunEvent, SystemTrayEvent, Manager};

use crate::{win_apis, autostart};
use crate::switche::{Hwnd, SwitcheState};



pub fn run_switche_tauri (ss:&SwitcheState) {

    // we'll setup tray-icon support to pass into app builder
    let tray = SystemTray::new() .with_tooltip("Switche") .with_menu (
        SystemTrayMenu::new ()
            // first we'll put the configs
            .add_item ( CustomMenuItem::new ( "auto_start",        "Auto-Start on Login" ) )
            .add_item ( CustomMenuItem::new ( "auto_start_admin",  "Auto-Start as Admin" ) )
            .add_native_item ( SystemTrayMenuItem::Separator )

            // the special entry to trigger opening the config file for editing
            .add_item ( CustomMenuItem::new ( "edit_conf",   "Edit Config"  ) )
            .add_item ( CustomMenuItem::new ( "reset_conf",  "Reset Config" ) )
            .add_native_item ( SystemTrayMenuItem::Separator )

            // then the actions
            .add_item ( CustomMenuItem::new ( "reload",     "Reload"  ) )
            .add_item ( CustomMenuItem::new ( "restart",    "Restart" ) )
            .add_item ( CustomMenuItem::new ( "quit",       "Quit"    ) )
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





fn tauri_window_events_handler (ss:&SwitcheState, ah:&AppHandle, ev:&WindowEvent) {
    match ev {
        WindowEvent::Focused (true)       => { ss.proc_app_window_event__focus() }
        WindowEvent::Focused (false)      => { ss.proc_app_window_event__focus_lost() }
        WindowEvent::Moved (..)           => { ss.conf.deferred_update_conf__switche_window(ah) }
        WindowEvent::Resized (..)         => { ss.conf.deferred_update_conf__switche_window(ah) }
        WindowEvent::CloseRequested {..}  => { }
        _ => { }
    }
}


fn extract_self_hwnd (ss:&SwitcheState) -> Option<Hwnd> {
    ss.app_handle.read().unwrap().as_ref() .iter() .map (|ah| {
        ah.windows().values() .next() .map (|w| w.hwnd().ok()) .flatten()
    } ) .flatten() .next() .map (|h| h.0)
}

pub fn auto_setup_self_window (self_hwnd:Hwnd) {
    let wa = win_apis::win_get_work_area();
    let (x, y, width, height) = ( wa.left + (wa.right-wa.left)/3, 0, (wa.right-wa.left)/2,  wa.bottom - wa.top);
    win_apis::win_move_to (self_hwnd, x, y, width, height);
}
fn setup_self_window (ss:&SwitcheState, self_hwnd:Hwnd) {
    // if there's a valid config, and the sizes are not zero (like at startup), we'll use those
    //if ss.conf.check_flag__restore_window_dimensions() {
    // ^^ disabled as we'd still rather use (valid) dimensions from configs even if not set to restore last-closed position
    if let Some ((x,y,w,h)) = ss.conf.read_conf__window_dimensions() {
        if w > 0 && h > 0 {
            win_apis::win_move_to (self_hwnd, x, y, w, h);
            return
    } }
    // else the default is to auto-calc window dimensions
    auto_setup_self_window (self_hwnd);
}

fn proc_event_app_ready (ss:&SwitcheState, _ah:&AppHandle<Wry>) {
    // we want to store a cached value of our hwnd for exclusions-mgr (and general use)
    if let Some(hwnd) = extract_self_hwnd(ss) {
        //println! ("App starting .. self-hwnd is : {:?}", hwnd );
        setup_self_window (ss, hwnd);
        //tauri_setup_self_window(ah);
        ss.store_self_hwnd(hwnd);
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




fn proc_tray_event__show (ss:&SwitcheState) {
    ss.checked_self_activate();   // also updates the is-dismissed etc flags
}
fn proc_tray_event__reload (ss:&SwitcheState) {
    ss.handle_req__data_load();   // will also re-setup kbd/mouse hooks
}

fn proc_tray_event__menu_click (ss:&SwitcheState, ah:&AppHandle<Wry>, menu_id:String) {

    match menu_id.as_str() {
        "auto_start"       =>  { autostart::proc_tray_event__toggle_switche_autostart (false, ah) }
        "auto_start_admin" =>  { autostart::proc_tray_event__toggle_switche_autostart (true,  ah) }

        "edit_conf"  =>  { ss.conf.trigger_config_file_edit() }
        "reset_conf" =>  { ss.conf.trigger_config_file_reset() }

        "reload"    =>  { proc_tray_event__reload(ss) }
        "restart"   =>  { ah.restart() }
        "quit"      =>  { ah.exit(0) }
        _ => { }
    }
}



fn proc_tray_event__left_click (ss:&SwitcheState) {
    proc_tray_event__show(ss)
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
        SystemTrayEvent::MenuItemClick { id, .. }  =>  { proc_tray_event__menu_click(ss, ah, id) },
        _ => { }
    }
}







pub fn setup_global_shortcuts (ssr:&SwitcheState, ah:&AppHandle<Wry>) {
    let mut gsm = ah.global_shortcut_manager();

    // todo: can update these to prob printout/notify an err msg when cant register global hotkey

    // we register all hotkeys specified in config file for switche invocation ..
    // .. typically should include F1 for invocation and Ctrl+Alt+F15 for krusty remapping of F1
    ssr.conf.get_switche_invocation_hotkeys() .iter() .for_each (|hotkey| {
        let ss = ssr.clone();
        let _ = gsm.register ( hotkey,  move || ss.proc_hot_key__invoke() );
    });

    // we register all hotkeys specified in config file for direct switch to last-window ..
    // .. typically should include Ctrl+Alt+F16
    ssr.conf.get_direct_last_window_switch_hotkeys() .iter() .for_each (|hotkey| {
        let ss = ssr.clone();
        let _ = gsm.register ( hotkey,  move || ss.proc_hot_key__switch_last() );
    });

    // finally, we'll also register any hotkeys specified in config file for direct switch to specific exe/title
    ssr.conf.get_direct_app_switch_hotkeys() .into_iter() .for_each (|(hotkey, exe, title)| {
        let ss = ssr.clone();
        let _ = gsm.register (hotkey.as_str(), move || ss.proc_hot_key__switch_app (exe.as_deref(), title.as_deref()) );
    });

}
