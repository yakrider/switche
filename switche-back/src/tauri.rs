#![ allow (non_snake_case) ]

use std::{thread, time};
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use tracing::{error};

use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Threading::CreateMutexW;

use crate::{win_apis, autostart};
use crate::switche::{Hwnd, SwitcheState};

use tauri::{ AppHandle, Wry, WindowEvent, RunEvent, Manager, App };
use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{ MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent };
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState, ShortcutWrapper};




#[derive (Default)]
struct _TrayMenuState {
    pub auto_start       : Option <CheckMenuItem<Wry>>,
    pub auto_start_admin : Option <CheckMenuItem<Wry>>,
}
#[derive (Clone)]
pub struct TrayMenuState ( Arc<Mutex<_TrayMenuState>> );


impl TrayMenuState {

    pub fn instance () -> TrayMenuState {
        static INSTANCE : Lazy<TrayMenuState> = Lazy::new (|| TrayMenuState ( Arc::new ( Mutex::new (
            _TrayMenuState { auto_start: None, auto_start_admin: None }
        ) ) ) );
        INSTANCE.clone()
    }

    pub fn store__auto_start (&self, cmi : CheckMenuItem<Wry>) {
        self.0 .lock().unwrap() .auto_start = Some(cmi);
    }
    pub fn store__auto_start_admin (&self, cmi : CheckMenuItem<Wry>) {
        self.0 .lock().unwrap() .auto_start_admin = Some(cmi);
    }

    pub fn set_checked__auto_start (&self, checked:bool) {
        if let Some(c) = self.0 .lock().unwrap() .auto_start .as_ref()  {
            let _ = c.set_checked (checked);
        }
    }
    pub fn set_checked__auto_start_admin (&self, checked:bool) {
        if let Some(c) = self.0 .lock().unwrap() .auto_start_admin .as_ref()  {
            let _ = c.set_checked (checked);
        }
    }

    pub fn set_enabled__auto_start (&self, enabled:bool) {
        if let Some(c) = self.0 .lock().unwrap() .auto_start .as_ref()  {
            let _ = c.set_enabled (enabled);
        }
    }
    pub fn set_enabled__auto_start_admin (&self, enabled:bool) {
        if let Some(c) = self.0 .lock().unwrap() .auto_start_admin .as_ref()  {
            let _ = c.set_enabled (enabled);
        }
    }

}


const MENU_AUTO_START       : &str = "auto_start";
const MENU_AUTO_START_ADMIN : &str = "auto_start_admin";
const MENU_EDIT_CONF        : &str = "edit_conf";
const MENU_RESET_CONF       : &str = "reset_conf";
const MENU_RELOAD           : &str = "reload";
const MENU_RESTART          : &str = "restart";
const MENU_QUIT             : &str = "quit";
// note: ^^ its easier to define these as consts instead of enums as that makes it easier to match against id-strings later

fn menu_disp_str (id:&str) -> &str {
    match id {
        MENU_AUTO_START       => "Auto-Start on Login",
        MENU_AUTO_START_ADMIN => "Auto-Start as Admin",
        MENU_EDIT_CONF        => "Edit Config",
        MENU_RESET_CONF       => "Reset Config",
        MENU_RELOAD           => "Reload",
        MENU_RESTART          => "Restart",
        MENU_QUIT             => "Quit",
        _ => ""
    }
}
fn exec_menu_action (id:&str, ss:&SwitcheState, ah:&AppHandle<Wry>) {
    match id {
        MENU_AUTO_START       => { autostart::proc_tray_event__toggle_switche_autostart (false) }
        MENU_AUTO_START_ADMIN => { autostart::proc_tray_event__toggle_switche_autostart (true) }
        MENU_EDIT_CONF        => { ss.conf.trigger_config_file_edit() }
        MENU_RESET_CONF       => { ss.conf.trigger_config_file_reset() }
        MENU_RELOAD           => { ss.proc_menu_req__switche_reload() }
        MENU_RESTART          => { ah.restart() }
        MENU_QUIT             => { ah.exit(0) }
        _ => { }
    }
}

pub fn handle_trayicon_action (ss:&SwitcheState, event:TrayIconEvent) {
    // we want to make left-click activate switche, the rest we can ignore .. (and default right click will bring menu)
    if let TrayIconEvent::Click { button: MouseButton::Left,  button_state: MouseButtonState::Up, .. } = event {
        ss.checked_self_activate()
    }
}




pub fn run_switche_tauri (ss:&SwitcheState) {

    // we'll setup tray-icon support to pass into app builder
    fn gen_tray (ss:&SwitcheState, ah:&App) -> tauri::Result<TrayIcon> {

        // utility closures to gen the menu items
        let make_menu_item  = |id| MenuItemBuilder::with_id (id, menu_disp_str(id)) .build(ah);
        let make_menu_check = |id| CheckMenuItemBuilder::with_id (id, menu_disp_str(id)) .build(ah);

        // first, lets build the menu items we might need to update later
        let menu_auto_start       = make_menu_check (MENU_AUTO_START) .expect("couldnt build tray-menu");
        let menu_auto_start_admin = make_menu_check (MENU_AUTO_START_ADMIN) .expect("couldnt build tray-menu");

        // and store them for reference later
        TrayMenuState::instance().store__auto_start (menu_auto_start.clone());
        TrayMenuState::instance().store__auto_start_admin (menu_auto_start_admin.clone());

        let menu = MenuBuilder::new(ah)
            // first we'll put the configs
            .item ( & menu_auto_start )
            .item ( & menu_auto_start_admin )
            .item ( & PredefinedMenuItem::separator(ah)? )

            // the special entry to trigger opening the config file for editing
            .item ( & make_menu_item (MENU_EDIT_CONF )? )
            .item ( & make_menu_item (MENU_RESET_CONF)? )
            .item ( & PredefinedMenuItem::separator(ah)? )

            // then the actions
            .item ( & make_menu_item (MENU_RELOAD )? )
            .item ( & make_menu_item (MENU_RESTART)? )
            .item ( & make_menu_item (MENU_QUIT   )? )
            .build()?;

        let (ss1, ss2) = (ss.clone(), ss.clone());

        TrayIconBuilder::new()
            .icon (ah.default_window_icon().unwrap().clone())
            .tooltip ("Switche")
            .menu(&menu)
            .menu_on_left_click(false)
            .on_menu_event ( move |ah, event| exec_menu_action (&event.id.0, &ss1, ah) )
            .on_tray_icon_event ( move |_tray, event| handle_trayicon_action (&ss2, event) )
            .build(ah)

    }

    let app = {
        tauri::Builder::default()
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_process::init())
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_global_shortcut::Builder::new().build())
            .device_event_filter (tauri::DeviceEventFilter::Always)
            // ^^ w/o this, our own LL input hooks will not receive events when tauri window is fgnd
            .setup ( {
                let ss = ss.clone();
                move |app| {
                    gen_tray (&ss, app)?;
                    enforce_single_instance (app);
                    autostart::update_tray_auto_start_admin_flags();
                    setup_global_shortcuts (&ss, app.handle());
                    ss.setup_front_end_listener (app.handle());
                    Ok(())
                }
            } )
            .invoke_handler (tauri::generate_handler![])
            .build (tauri::generate_context!())
            .expect ("error while building tauri application")
    };

    // we'll register the app with the switche engine so it can send back responses etc
    ss.register_app_handle(app.handle().clone());

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


fn check_another_instance_running () -> bool { unsafe {
    let switche_mutex = windows::core::w!("Global\\Switche_SingleInstance_Mutex");
    // the os-global-mutex can fail to create, or it can return prior-created mutex and have last-error say ERROR_ALREADY_EXISTS
    if CreateMutexW (None, true, switche_mutex) .is_err() {
        return true;
    }
    if GetLastError() == ERROR_ALREADY_EXISTS {
        return true;
    }
    false
} }
fn display_mult_instance_error (app: &App<Wry>) {
    let ah = app.handle().clone();
    // blocking dialog requires it be not in the main thread
    thread::spawn ( move || {
        // we ideally want to exit when the Ok button is pressed ..
        // .. but additionally we'll also setup a delayed exit in advance
        let ahc = ah.clone();
        thread::spawn ( move || {
            thread::sleep (time::Duration::from_secs(10));
            ahc.exit(1);
        } );

        ah.dialog()
            .message (
                "\nSwitche Startup Error:\n\
                \x20   Another instance of Switche is already running.\n\n\
                (This dialog will exit in 10 seconds)"
            )
            .kind (MessageDialogKind::Error)
            .title ("Switche Startup Error")
            .buttons (MessageDialogButtons::OkCustom("Press to Exit".into()))
            .blocking_show();

        ah.exit(1);
    } );
}
fn enforce_single_instance (app: &App<Wry>) {
    if check_another_instance_running() { display_mult_instance_error(app) }
}


fn extract_self_hwnd (ss:&SwitcheState) -> Option<Hwnd> {
    ss.app_handle.read().unwrap() .as_ref() .and_then (|ah| {
        ah.webview_windows().values() .next() .and_then (|w| w.hwnd().ok())
    } ) .map (|h| h.into())
}

pub fn auto_setup_self_window (ss:&SwitcheState) {
    let wa = win_apis::win_get_work_area();

    //let (x, y, width, height) = ( wa.left + (wa.right-wa.left)/3, 0, (wa.right-wa.left)/2,  wa.bottom - wa.top);
    // ^^ reasonable initial version .. half screen width, full screen height, starting 1/3 from left edge

    // vertically we'll span the full usable height
    let (y, height) = (0, wa.bottom - wa.top);

    // horizontally we'll span upto half the screen width, but no more than dpi scaled 860px
    let max_width = ss.app_handle.read().unwrap().as_ref() .and_then (|ah|
        ah .get_webview_window("main") .and_then (|w| w.scale_factor().ok())
    ) .unwrap_or (1.0)  * 860.0;
    let width = std::cmp::min ( (wa.right - wa.left)/2, max_width as i32 );

    // for x, ideally want centerline at 1/3 of the window width (aiming for icons column), as long as it go beyond right edge
    let max_x = wa.right - width;
    let x = std::cmp::min ( max_x, wa.left + (wa.right - wa.left)/2 - width/3 );

    win_apis::win_move_to (ss.get_self_hwnd(), x, y, width, height);
}
pub fn sync_self_always_on_top (ss:&SwitcheState) {
    ss.app_handle .read().unwrap() .iter() .for_each ( |ah| {
        ah .webview_windows() .get("main") .map (|w| {
            w.set_always_on_top (ss.conf.check_flag__auto_hide_enabled())
        });
    } )
}

pub fn setup_self_window (ss:&SwitcheState) {
    // if there's a valid config, and the sizes are not zero (like at startup), we'll use those
    //if ss.conf.check_flag__restore_window_dimensions() {
    // ^^ disabled as we'd still rather use (valid) dimensions from configs even if not set to restore last-closed position
    if let Some ((x,y,w,h)) = ss.conf.read_conf__window_dimensions() {
        if w > 0 && h > 0 {
            win_apis::win_move_to (ss.get_self_hwnd(), x, y, w, h);
        }
    } else {
        // else the default is to auto-calc window dimensions
        auto_setup_self_window (ss);
    }
    // finally, we'll also setup/sync the self-window always-on-top behavior
    sync_self_always_on_top (ss);
}

fn proc_event_app_ready (ss:&SwitcheState, _ah:&AppHandle<Wry>) {
    // we want to store a cached value of our hwnd for exclusions-mgr (and general use)
    if let Some(hwnd) = extract_self_hwnd(ss) {
        //debug! ("App starting .. self-hwnd is : {:?}", hwnd );
        ss.store_self_hwnd(hwnd);
        setup_self_window (ss);
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




pub fn setup_global_shortcuts (ss:&SwitcheState, ah:&AppHandle<Wry>) {

    fn register_hotkeys <SSF> (ah:&AppHandle<Wry>, ss:&SwitcheState, hotkeys:&[String], ssf:SSF) where
        SSF : Fn (&SwitcheState) + Send + Sync + 'static,
    {
        // we'll generate the actual handler-fn from the simpler handling action generator we take in
        type HF = Box <dyn Fn(&AppHandle, &Shortcut, ShortcutEvent) + Send + Sync + 'static>;

        // we'll need everything to be cloneable to use in loop iteration
        let (ss, ssf) = (ss.clone(), Arc::new(ssf));

        // now we can build a registration closure that returns a Result for easier err handling
        let reg_fn = |hk:&str| {
            let hf : HF = Box::new (move |_,_,e| { if e.state == ShortcutState::Pressed { ssf(&ss) } } );
            let hk = ShortcutWrapper::try_from(hk)?;
            ah .global_shortcut() .on_shortcut (hk, hf)
        };
        // and finally we can go through and register each hotkey, handling any errors
        hotkeys .iter() .for_each (|hotkey| {
            if let Err(e) = reg_fn.clone() (hotkey.as_str()) {
                error! ("Failed to register hotkey {:?}: {}", hotkey, e);
            }
        });
    }

    // we register all hotkeys specified in config file for switche invocation ..
    // .. typically should include F1 for invocation and Ctrl+Alt+F15 for krusty remapping of F1
    register_hotkeys (ah, ss, &ss.conf.get_switche_invocation_hotkeys(), |ss| ss.proc_hot_key__invoke() );

    // we register all hotkeys specified in config file for direct switch to n-th last-windows ..
    // .. typically should include Ctrl+Alt+F16 (through F18) for the last (through third-last)
    register_hotkeys (ah, ss, &ss.conf.get_direct_last_window_switch_hotkeys(), |ss| ss.proc_hot_key__switch_z_idx(1) );
    register_hotkeys (ah, ss, &ss.conf.get_second_last_window_switch_hotkeys(), |ss| ss.proc_hot_key__switch_z_idx(2) );
    register_hotkeys (ah, ss, &ss.conf.get_third_last_window_switch_hotkeys(),  |ss| ss.proc_hot_key__switch_z_idx(3) );


    // and for hotkeys specified in config file to take snapshot of windows, and switch through them without bringing switche up
    register_hotkeys (ah, ss, &ss.conf.get_windows_list_snapshot_hotkeys(),   |ss| ss.proc_hot_key__snap_list_refresh() );
    register_hotkeys (ah, ss, &ss.conf.get_snap_list_switch_next_hotkeys(),   |ss| ss.proc_hot_key__snap_list_switch (|sl| sl.next_hwnd()) );
    register_hotkeys (ah, ss, &ss.conf.get_snap_list_switch_prev_hotkeys(),   |ss| ss.proc_hot_key__snap_list_switch (|sl| sl.prev_hwnd()) );
    register_hotkeys (ah, ss, &ss.conf.get_snap_list_switch_top_hotkeys(),    |ss| ss.proc_hot_key__snap_list_switch (|sl| sl.top_hwnd()) );
    register_hotkeys (ah, ss, &ss.conf.get_snap_list_switch_bottom_hotkeys(), |ss| ss.proc_hot_key__snap_list_switch (|sl| sl.bottom_hwnd()) );

    // finally, we'll also register any hotkeys specified in config file for direct switch to specific exe/title
    ss.conf.get_direct_app_switch_hotkeys() .into_iter() .for_each (|(hotkey, exe, title, partial)| {
        register_hotkeys (ah, ss, &[hotkey], move |ss| ss.proc_hot_key__switch_app (exe.as_deref(), title.as_deref(), partial) )
    });

}
