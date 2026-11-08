#![allow (non_snake_case)]

use std::thread;

use tray_icon::menu::{
    CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem,
};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tracing::error;

use crate::autostart;
use crate::switche::{SwitcheState, UiRequest};



pub const MENU_AUTO_START       : &str = "auto_start";
pub const MENU_AUTO_START_ADMIN : &str = "auto_start_admin";
pub const MENU_EDIT_CONF        : &str = "edit_conf";
pub const MENU_RESET_CONF       : &str = "reset_conf";
pub const MENU_RELOAD           : &str = "reload";
pub const MENU_RESTART          : &str = "restart";
pub const MENU_QUIT             : &str = "quit";



pub struct TrayState {
    _tray            : TrayIcon,
    pub auto_start       : CheckMenuItem,
    pub auto_start_admin : CheckMenuItem,
}



/// must be called on the main thread (the one running the win32 message loop)
pub fn setup_tray (ss: &'static SwitcheState) -> Option<TrayState> {

    let icon = {
        let bytes = include_bytes!("../switche.ico");
        let img = match image::load_from_memory (bytes) {
            Ok (img) => img.to_rgba8(),
            Err (e) => { error! ("failed to decode tray icon: {:?}", e); return None }
        };
        // the ico is multi-resolution; pin the tray rendition to the standard 32x32
        // .. (a 128/256px source can end up blank in the shell notification area)
        let img = image::imageops::resize (&img, 32, 32, image::imageops::FilterType::Triangle);
        match tray_icon::Icon::from_rgba (img.into_raw(), 32, 32) {
            Ok (icon) => icon,
            Err (e) => { error! ("failed to create tray icon: {:?}", e); return None }
        }
    };

    let menu = Menu::new();
    let auto_start       = CheckMenuItem::new ("Auto-Start on Login", true, false, None);
    let auto_start_admin = CheckMenuItem::new ("Auto-Start as Admin", true, false, None);

    let sep_1  = PredefinedMenuItem::separator();
    let sep_2  = PredefinedMenuItem::separator();
    let edit   = MenuItem::with_id (MENU_EDIT_CONF , "Edit Config" , true, None);
    let reset  = MenuItem::with_id (MENU_RESET_CONF, "Reset Config", true, None);
    let reload = MenuItem::with_id (MENU_RELOAD    , "Reload"      , true, None);
    let restart= MenuItem::with_id (MENU_RESTART   , "Restart"     , true, None);
    let quit   = MenuItem::with_id (MENU_QUIT      , "Quit"        , true, None);

    let items : [&dyn IsMenuItem; 9] = [
        &auto_start, &auto_start_admin, &sep_1,
        &edit, &reset, &sep_2,
        &reload, &restart, &quit,
    ];
    if let Err (e) = menu.append_items (&items) {
        error! ("failed to build tray menu: {:?}", e);
        return None
    }

    let tray = match TrayIconBuilder::new()
        .with_icon (icon)
        .with_tooltip ("Switche")
        .with_menu (Box::new (menu))
        .with_menu_on_left_click (false)
        .build()
    {
        Ok (tray) => tray,
        Err (e) => { error! ("failed to create tray icon: {:?}", e); return None }
    };

    spawn_tray_listeners (ss);

    tracing::info! ("tray icon created");

    Some (TrayState { _tray: tray, auto_start, auto_start_admin })
}



fn spawn_tray_listeners (ss: &'static SwitcheState) {

    thread::spawn ( move || {
        let rx = TrayIconEvent::receiver();
        while let Ok (ev) = rx.recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left, button_state: MouseButtonState::Up, ..
            } = ev {
                ss.checked_self_activate();
            }
        }
    } );

    thread::spawn ( move || {
        let rx = MenuEvent::receiver();
        while let Ok (ev) = rx.recv() {
            dispatch_menu_action (&ev.id.0, ss);
        }
    } );
}



fn dispatch_menu_action (id:&str, ss: &'static SwitcheState) {
    match id {
        MENU_AUTO_START       => autostart::proc_req__toggle_switche_autostart (false),
        MENU_AUTO_START_ADMIN => autostart::proc_req__toggle_switche_autostart (true),

        MENU_EDIT_CONF        => ss.conf.trigger_config_file_edit(),
        MENU_RESET_CONF       => {
            ss.conf.trigger_config_file_reset();
            ss.handle_ui_request (UiRequest::DataLoad);
        }

        MENU_RELOAD           => ss.proc_menu_req__switche_reload(),
        MENU_RESTART          => restart_app (ss),
        MENU_QUIT             => ss.handle_ui_request (UiRequest::SwitcheQuit),

        _ => { }
    }
}



fn restart_app (ss: &'static SwitcheState) {
    crate::app_window::persist_self_window_dimensions_now (ss);
    // release our single-instance mutex first, then start the new process directly
    // .. (no shell involved, which also avoids the cmd/start quote-mangling)
    crate::app_window::release_single_instance_mutex();
    if let Ok (exe) = std::env::current_exe() {
        let _ = std::process::Command::new (exe).spawn();
    }
    std::process::exit (0);
}



pub fn apply_auto_start_flags (tray:&TrayState, is_elevated:bool, elev_task_enabled:bool, normal_task_enabled:bool) {
    tray.auto_start_admin .set_checked (elev_task_enabled);
    tray.auto_start       .set_checked (normal_task_enabled);

    // the elevated entry can only be acted on while we're elevated
    tray.auto_start_admin .set_enabled (is_elevated);

    // the normal entry is usable if it's already on, or if the elevated task isn't on
    tray.auto_start       .set_enabled (normal_task_enabled || !elev_task_enabled);
}
