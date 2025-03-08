#![ allow (non_snake_case) ]

use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

use tauri::{ AppHandle, Wry, App };
use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{ MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent };

use crate::autostart;
use crate::switche::SwitcheState;



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
fn exec_menu_action (id:&str, ss: &'static SwitcheState, ah:&AppHandle<Wry>) {
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

pub fn handle_trayicon_action (ss: &'static SwitcheState, event:TrayIconEvent) {
    // we want to make left-click activate switche, the rest we can ignore .. (and default right click will bring menu)
    if let TrayIconEvent::Click { button: MouseButton::Left,  button_state: MouseButtonState::Up, .. } = event {
        ss.checked_self_activate()
    }
}



// we'll setup tray-icon support to pass into app builder
pub fn gen_tray (ss: &'static SwitcheState, ah:&App) -> tauri::Result<TrayIcon> {

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

    TrayIconBuilder::new()
        .icon (ah.default_window_icon().unwrap().clone())
        .tooltip ("Switche")
        .menu(&menu)
        .menu_on_left_click(false)
        .on_menu_event ( move |ah, event| exec_menu_action (&event.id.0, ss, ah) )
        .on_tray_icon_event ( move |_tray, event| handle_trayicon_action (ss, event) )
        .build(ah)

}

