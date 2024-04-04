
use tauri::{SystemTray, SystemTrayMenu, SystemTrayMenuItem, CustomMenuItem, AppHandle, Wry, GlobalShortcutManager, WindowEvent, RunEvent, SystemTrayEvent, Manager};

use crate::win_apis;
use crate::switche::{Hwnd, SwitcheState};



pub fn run_switche_tauri(ss:&SwitcheState) {

    // we'll setup tray-icon support to pass into app builder
    let tray = SystemTray::new() .with_tooltip("Switche") .with_menu (
        SystemTrayMenu::new ()
            // first we'll put the configs
            .add_item ( CustomMenuItem::new ( "auto-hide",  "Auto-Hide" ) )
            .add_native_item ( SystemTrayMenuItem::Separator )
            // then the actions
            .add_item ( CustomMenuItem::new ( "show",       "Show"    ) )
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

    // lets sync up the menu selection states w whats in switche-state
    app .tray_handle() .try_get_item("auto-hide") .map (|e| e.set_selected (ss.auto_hide_enabled.is_set()) );
    app .windows() .get("main") .map (|w| w.set_always_on_top (ss.auto_hide_enabled.is_set()));


    // we'll register the app with the switche engine so it can send back responses etc
    ss.register_app_handle(app.handle());



    // now lets finally actually start the app! .. note that the run call wont return!
    let ssc = ss.clone();
    app .run ( move |ah, event| { tauri_run_events_handler (&ssc, ah, event) } );

}





fn tauri_window_events_handler (ss:&SwitcheState, ev:&WindowEvent) {
    match ev {
        WindowEvent::Focused (true)       => { ss.proc_app_window_event__focus() }
        WindowEvent::Focused (false)      => { ss.proc_app_window_event__focus_lost() }
        WindowEvent::Moved (..)           => { } // todo: useful when want to store window pos/size in configs
        WindowEvent::Resized (..)         => { }
        WindowEvent::CloseRequested {..}  => { }
        _ => { }
    }
}


fn extract_self_hwnd (ss:&SwitcheState) -> Option<Hwnd> {
    ss.app_handle.read().unwrap().as_ref() .iter() .map (|ah| {
        ah.windows().values() .next() .map (|w| w.hwnd().ok()) .flatten()
    } ) .flatten() .next() .map (|h| h.0)
}
fn setup_self_window (self_hwnd:Hwnd) {
    let wa = win_apis::win_get_work_area();
    let (x, y, width, height) = ( wa.left + (wa.right-wa.left)/3, 0, (wa.right-wa.left)/2,  wa.bottom - wa.top);
    win_apis::win_move_to (self_hwnd, x, y, width, height);
    // todo: ^^ update this to check config first, and only do this if there's no config
    //println! ("setting self window to: x:{}, y:{}, w:{}, h:{}", x, y, width, height);
}
fn proc_event_app_ready (ss:&SwitcheState) {
    // we want to store a cached value of our hwnd for exclusions-mgr (and general use)
    if let Some(hwnd) = extract_self_hwnd(ss) {
        //println! ("App starting .. self-hwnd is : {:?}", hwnd );
        setup_self_window (hwnd);
        ss.store_self_hwnd(hwnd);
    }
}
pub fn tauri_run_events_handler (ss:&SwitcheState, _ah:&AppHandle<Wry>, event:RunEvent) {
    match event {
        RunEvent::Ready                          => { proc_event_app_ready(ss) }
        RunEvent::WindowEvent   { event, .. }    => { tauri_window_events_handler(ss, &event) }
        //RunEvent::ExitRequested { api,   .. }  => { api.prevent_exit() }
        _ => {}
    }
}




fn proc_tray_event_auto_hide_toggle (ss:&SwitcheState, ah:&AppHandle<Wry>) {
    let auto_hide_state = ss.toggle_auto_hide();
    ah .tray_handle() .try_get_item("auto-hide") .map (|e| e.set_selected (auto_hide_state) );
    ah .windows() .get("main") .map (|w| w.set_always_on_top (auto_hide_state));
}

fn proc_tray_event_show (ss:&SwitcheState) {
    ss.checked_self_activate();   // also updates the is-dismissed etc flags
}
fn proc_tray_event_reload (ss:&SwitcheState) {
    ss.handle_req__data_load();   // will also re-setup kbd/mouse hooks
}

fn proc_tray_event_menu_click (ss:&SwitcheState, ah:&AppHandle<Wry>, menu_id:String) {
    match menu_id.as_str() {
        "auto-hide" =>  { proc_tray_event_auto_hide_toggle (ss, ah) }
        "show"      =>  { proc_tray_event_show (ss) }
        "reload"    =>  { proc_tray_event_reload (ss) }
        "restart"   =>  { ah.restart() }
        "quit"      =>  { ah.exit(0) }
        _ => { }
    }
}



fn proc_tray_event_left_click (ss:&SwitcheState) {
    proc_tray_event_show(ss)
}
fn proc_tray_event_right_click (_ss:&SwitcheState) {
    // nothign to do to bring up the menu?
}
fn proc_tray_event_double_click (_ss:&SwitcheState) {
    // maybe eventually will want to bring up config?
}

pub fn tray_events_handler (ss:&SwitcheState, ah:&AppHandle<Wry>, event:SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick   { .. }  =>  { proc_tray_event_left_click (ss) },
        SystemTrayEvent::RightClick  { .. }  =>  { proc_tray_event_right_click (ss) },
        SystemTrayEvent::DoubleClick { .. }  =>  { proc_tray_event_double_click (ss) },
        SystemTrayEvent::MenuItemClick { id, .. }  =>  { proc_tray_event_menu_click (ss, ah, id) },
        _ => { }
    }
}







pub fn setup_global_shortcuts (ssr:&SwitcheState, ah:&AppHandle<Wry>) {
    let mut gsm = ah.global_shortcut_manager();

    // todo: can update these to prob printout/notify an err msg when cant register global hotkey

    let ss = ssr.clone();  let _ = gsm.register ( "F1",              move || ss.proc_hot_key__invoke() );
    // ^^ F1 is for global invocatino

    let ss = ssr.clone();  let _ = gsm.register ( "Ctrl+F15",        move || ss.proc_hot_key__invoke() );
    // ^^ krusty remapping of F1 .. this allows krusty to use say ralt-F1 for actual F1 if we disable F1 above in switche configs

    //let ss = ssr.clone();  let _ = gsm.register ( "F16",             move || ss.proc_hot_key__scroll_down() );
    //let ss = ssr.clone();  let _ = gsm.register ( "Shift+F16",       move || ss.proc_hot_key__scroll_up()   );
    //let ss = ssr.clone();  let _ = gsm.register ( "F17",             move || ss.proc_hot_key__scroll_up()   );
    //let ss = ssr.clone();  let _ = gsm.register ( "Ctrl+F18",        move || ss.proc_hot_key__scroll_end()  );
    // ^^ these were krusty driven scrolling hotkeys, but we've direct impld alt-tab and right-mbtn-scroll in switche now

    let ss = ssr.clone();  let _ = gsm.register ( "Ctrl+Alt+F18",    move || ss.proc_hot_key__switche_escape() );
    // ^^ krusty-driven support for esc during alt-tab, to avoid the native alt-esc behavior

    // other misc krusty/ahk driven hotkeys for direct invocation of specific windows types
    let ss = ssr.clone();  let _ = gsm.register ( "Ctrl+Alt+F19",    move || ss.proc_hot_key__switch_last()          );
    let ss = ssr.clone();  let _ = gsm.register ( "Ctrl+Alt+F20",    move || ss.proc_hot_key__switch_tabs_outliner() );
    let ss = ssr.clone();  let _ = gsm.register ( "Ctrl+Alt+F21",    move || ss.proc_hot_key__switch_notepad_pp()    );
    let ss = ssr.clone();  let _ = gsm.register ( "Ctrl+Alt+F22",    move || ss.proc_hot_key__switch_ide()           );
    let ss = ssr.clone();  let _ = gsm.register ( "Ctrl+Alt+F23",    move || ss.proc_hot_key__switch_music()         );
    let ss = ssr.clone();  let _ = gsm.register ( "Ctrl+Alt+F24",    move || ss.proc_hot_key__switch_browser()       );

    //let ss = ssr.clone();  let _ = gsm.register ( "Alt+Tab",         move || ss.proc_hot_key__scroll_down()          );
    // ^^ ofc trying to set alt-tab like this wont take, but its useful here as a reminder

}
