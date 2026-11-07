#![allow (non_snake_case)]

use std::thread;

use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK, MB_SETFOREGROUND};

use crate::switche::SwitcheState;
use crate::win_apis;


pub fn check_another_instance_running () -> bool { unsafe {
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

pub fn display_mult_instance_error () {
    thread::spawn ( move || unsafe {
        let text = windows::core::w! (
            "\nSwitche Startup Error:\n\
            \x20   Another instance of Switche is already running.\n\n\
            (This dialog will exit in 10 seconds)"
        );
        let title = windows::core::w!("Switche Startup Error");
        MessageBoxW (None, text, title, MB_OK | MB_ICONERROR | MB_SETFOREGROUND);
        std::process::exit (1);
    } );
}


pub fn auto_setup_self_window (ss: &'static SwitcheState) {
    let hwnd = ss.get_self_hwnd();
    if hwnd.is_null() { return }

    let wa = win_apis::win_get_work_area();

    // vertically we'll span the full usable height
    let (y, height) = (0, wa.bottom - wa.top);

    // horizontally we'll span upto half the screen width, but no more than dpi scaled 860px
    let max_width = win_apis::win_get_dpi_scale (hwnd) * 860.0;
    let width = std::cmp::min ( (wa.right - wa.left)/2, max_width as i32 );

    // for x, ideally want centerline at 1/3 of the window width (aiming for icons column), as long as it go beyond right edge
    let max_x = wa.right - width;
    let x = std::cmp::min ( max_x, wa.left + (wa.right - wa.left)/2 - width/3 );

    win_apis::win_move_to (hwnd, x, y, width, height);
}

pub fn sync_self_always_on_top (ss: &'static SwitcheState, set_always_top: bool) {
    let hwnd = ss.get_self_hwnd();
    if hwnd.is_null() { return }
    win_apis::win_set_topmost (hwnd, set_always_top);
}


/// called once right after the GPUI window has been created (and its hwnd recorded)
pub fn setup_window_styles (ss: &'static SwitcheState) {
    let hwnd = ss.get_self_hwnd();
    if hwnd.is_null() { return }
    // keep ourselves out of the taskbar and out of the OS alt-tab list
    win_apis::win_set_toolwindow_style (hwnd);
}

pub fn setup_self_window (ss: &'static SwitcheState) {
    let hwnd = ss.get_self_hwnd();
    if hwnd.is_null() { return }

    // if there's a valid config, and the sizes are not zero (like at startup), we'll use those
    if let Some ((x,y,w,h)) = ss.conf.read_conf__window_dimensions() {
        if w > 0 && h > 0 {
            win_apis::win_move_to (hwnd, x, y, w, h);
        }
    } else {
        // else the default is to auto-calc window dimensions
        auto_setup_self_window (ss);
    }
    // finally, we'll also setup/sync the self-window always-on-top behavior
    sync_self_always_on_top (ss, ss.conf.check_flag__auto_hide_enabled());
}


/// persists the current window location/size to configs (if enabled there)
pub fn persist_self_window_dimensions (ss: &'static SwitcheState) {
    ss.conf.deferred_update_conf__switche_window (ss);
}
