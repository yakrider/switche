#![allow (non_snake_case)]

use std::sync::OnceLock;
use std::thread;

use windows::Win32::Foundation::{CloseHandle, GetLastError, SetLastError, ERROR_ALREADY_EXISTS, ERROR_SUCCESS, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK, MB_SETFOREGROUND};

use crate::switche::SwitcheState;
use crate::win_apis;


// handle to the global single-instance mutex, kept for the process lifetime
// .. (and closed explicitly when restarting, so the new instance can start right away)
static INSTANCE_MUTEX : OnceLock <isize> = OnceLock::new();


pub fn check_another_instance_running () -> bool { unsafe {
    let switche_mutex = windows::core::w!("Global\\Switche_SingleInstance_Mutex");
    SetLastError (ERROR_SUCCESS);
    match CreateMutexW (None, true, switche_mutex) {
        Ok (handle) => {
            // the os-global-mutex can return a prior-created mutex and have last-error say ERROR_ALREADY_EXISTS
            if GetLastError() == ERROR_ALREADY_EXISTS {
                let _ = CloseHandle (handle);
                true
            } else {
                let _ = INSTANCE_MUTEX.set (handle.0 as isize);
                false
            }
        }
        Err (_) => true,
    }
} }

/// closes our single-instance mutex handle, so a restarted instance doesn't see us as still running
pub fn release_single_instance_mutex () {
    if let Some (&h) = INSTANCE_MUTEX.get() {
        unsafe { let _ = CloseHandle (HANDLE (h as *mut _)); }
    }
}

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
    // the drag indicator is a caption-drag region; don't let a double click maximize
    win_apis::win_set_not_maximizable (hwnd);
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


/// starts a native window-move loop (driven by the physical mouse button state)
/// .. gpui's start_window_move() is a no-op on windows, and the HTCAPTION hit-test
/// .. route makes double-click auto-size impossible, so we do the move ourselves
pub fn start_window_drag (ss: &'static SwitcheState) {
    let hwnd = ss.get_self_hwnd();
    if hwnd.is_null() { return }
    let Some ((cursor_x0, cursor_y0)) = win_apis::win_get_cursor_pos() else { return };
    let rect = win_apis::win_get_window_rect (hwnd);
    let (w, h) = (rect.right - rect.left, rect.bottom - rect.top);
    if w <= 0 || h <= 0 { return }

    thread::spawn ( move || {
        loop {
            if !win_apis::win_left_button_down() { break }
            if let Some ((cursor_x, cursor_y)) = win_apis::win_get_cursor_pos() {
                let x = rect.left + (cursor_x - cursor_x0);
                let y = rect.top  + (cursor_y - cursor_y0);
                win_apis::win_move_to (hwnd, x, y, w, h);
            }
            thread::sleep (std::time::Duration::from_millis (4));
        }
        persist_self_window_dimensions_now (ss);
    } );
}


/// persists the current window location/size to configs (if enabled there)
pub fn persist_self_window_dimensions (ss: &'static SwitcheState) {
    ss.conf.deferred_update_conf__switche_window (ss);
}

/// synchronous variant, for when the process is about to exit
pub fn persist_self_window_dimensions_now (ss: &'static SwitcheState) {
    if ss.conf.check_flag__restore_window_dimensions() {
        ss.conf.update_conf__switche_window_now (ss);
    }
}
