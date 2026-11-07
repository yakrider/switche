#![allow (non_snake_case)]

use std::env;

use gpui::prelude::*;
use gpui::{
    px, size, Bounds, Point, WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
};
use raw_window_handle::RawWindowHandle;

use crate::switche::{register_ui_event_sender, Hwnd, SwitcheState, UiEvent, UiRequest};
use crate::ui::SwitcheApp;
use crate::{app_window, hotkeys, win_apis};



pub fn run_switche_gpui (ss: &'static SwitcheState) {

    // dev convenience: lets a locally built instance run while the installed one is up
    let skip_single_instance = env::var ("SWITCHE_SKIP_SINGLE_INSTANCE").is_ok();
    if !skip_single_instance && app_window::check_another_instance_running() {
        app_window::display_mult_instance_error();
        return
    }

    let (ui_event_tx, ui_event_rx) = async_channel::unbounded::<UiEvent>();
    register_ui_event_sender (ui_event_tx);

    gpui::Application::new().run (move |cx| {

        let bounds = Bounds {
            origin: Point { x: px(300.), y: px(10.) },
            size: size(px(600.), px(650.)),
        };
        let options = WindowOptions {
            window_bounds       : Some (WindowBounds::Windowed (bounds)),
            titlebar            : None,
            focus               : false,
            show                : false,
            kind                : WindowKind::Normal,
            is_movable          : true,
            is_resizable        : true,
            is_minimizable      : false,
            window_background   : WindowBackgroundAppearance::Transparent,
            window_min_size     : Some (size(px(600.), px(650.))),
            ..Default::default()
        };

        let rx = ui_event_rx.clone();
        let app_entity = std::rc::Rc::new (std::cell::RefCell::new (None));
        let app_entity_for_closure = app_entity.clone();

        let _win = cx.open_window (options, move |window, cx| {

            // grab the native handle while we have it, so all the win32 based
            // window management (hide/show/topmost/move) can keep working as before
            if let Ok (handle) = raw_window_handle::HasWindowHandle::window_handle (&*window) {
                if let RawWindowHandle::Win32 (h) = handle.as_raw() {
                    ss.store_self_hwnd (Hwnd (h.hwnd.get()));
                }
            }

            let app = cx.new (|cx| SwitcheApp::new (cx));
            window.focus (&app.read (cx).focus_handle);
            *app_entity_for_closure.borrow_mut() = Some (app.clone());
            app
        }) .expect ("failed to open switche window");

        // IMPORTANT: all of this has to run after the launch callback returns.
        // .. Moving/resizing the native window from within the callback fires GPUI's
        // .. window callbacks while the app borrow is still held, which logs
        // .. "RefCell already borrowed" and drops those bounds updates.
        let show_on_start = env::var ("SWITCHE_SHOW_ON_START").is_ok();
        cx.spawn (async move |cx:&mut gpui::AsyncApp| {

            // style/position before anything can show the window
            app_window::setup_window_styles (ss);
            app_window::setup_self_window (ss);

            // global hotkeys (invocation, direct-switch etc) via global-hotkey
            hotkeys::setup_global_shortcuts (ss);

            // initial data-load: reloads configs, hooks, lists, icons etc
            ss.handle_ui_request (UiRequest::DataLoad);

            // handy for manual runs while developing
            if show_on_start {
                win_apis::window_activate (ss.get_self_hwnd());
                // the startup fgnd event triggers a light enum query, so force a full one for dev runs
                ss.handle_ui_request (UiRequest::Refresh);
            }

            // start draining engine events now that the window and app borrow are settled
            if let Some (app) = app_entity.borrow().as_ref().cloned() {
                let _ = app.update (cx, |app, cx| app.start_event_pump (cx, rx));
            }
        }) .detach();
    });
}
