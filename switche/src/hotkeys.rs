#![allow (non_snake_case)]

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};
use std::thread;

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use tracing::{error, info};

use crate::switche::SwitcheState;



// note: the manager is !Send/!Sync on windows (it owns a message-window), so we keep it
// .. alive by leaking it on the main thread instead of storing it in a static
static HOTKEY_ACTIONS : OnceLock <Mutex <HashMap <u32, HotkeyAction>>> = OnceLock::new();



#[derive (Debug, Clone)]
enum HotkeyAction {
    Invoke,
    SwitchZ (usize),
    SwitchNextNonMinimized,

    SnapListRefresh,
    SnapListNext,
    SnapListPrev,
    SnapListTop,
    SnapListBottom,

    SwitchApp { exes: Vec<String>, title: Option<String>, partial: bool },
}



fn dispatch_hotkey_action (ss: &'static SwitcheState, action: HotkeyAction) {
    match action {
        HotkeyAction::Invoke                  => ss.proc_hot_key__invoke(),
        HotkeyAction::SwitchZ (z)             => ss.proc_hot_key__switch_z_idx (z),
        HotkeyAction::SwitchNextNonMinimized  => ss.proc_hot_key__switch_next_non_minimized(),

        HotkeyAction::SnapListRefresh         => ss.proc_hot_key__snap_list_refresh(),
        HotkeyAction::SnapListNext            => ss.proc_hot_key__snap_list_switch (|sl| sl.next_hwnd()   ),
        HotkeyAction::SnapListPrev            => ss.proc_hot_key__snap_list_switch (|sl| sl.prev_hwnd()   ),
        HotkeyAction::SnapListTop             => ss.proc_hot_key__snap_list_switch (|sl| sl.top_hwnd()    ),
        HotkeyAction::SnapListBottom          => ss.proc_hot_key__snap_list_switch (|sl| sl.bottom_hwnd() ),

        HotkeyAction::SwitchApp { exes, title, partial } => {
            ss.proc_hot_key__switch_app (&exes, title.as_deref(), partial)
        }
    }
}



pub fn setup_global_shortcuts (ss: &'static SwitcheState) {

    let manager = match GlobalHotKeyManager::new() {
        Ok (m) => m,
        Err (e) => { error! ("Failed to create global hotkey manager: {}", e); return }
    };

    let mut actions : HashMap<u32, HotkeyAction> = HashMap::new();

    let mut register = |hotkeys: &[String], action: HotkeyAction| {
        for hotkey in hotkeys {
            match HotKey::from_str (hotkey) {
                Ok (hk) => {
                    let id = hk.id();
                    match manager.register (hk) {
                        Ok (()) => { actions.insert (id, action.clone()); }
                        Err (e) => { error! ("Failed to register hotkey {:?}: {}", hotkey, e) }
                    }
                }
                Err (e) => { error! ("Failed to parse hotkey {:?}: {}", hotkey, e) }
            }
        }
    };

    // we register all hotkeys specified in config file for switche invocation ..
    // .. typically should include F1 for invocation and Ctrl+Alt+F15 for krusty remapping of F1
    register (&ss.conf.get_switche_invocation_hotkeys(), HotkeyAction::Invoke);

    // we register all hotkeys specified in config file for direct switch to n-th last-windows ..
    // .. typically should include Ctrl+Alt+F16 (through F18) for the last (through third-last)
    register (&ss.conf.get_direct_last_window_switch_hotkeys(),   HotkeyAction::SwitchZ (1));
    register (&ss.conf.get_second_last_window_switch_hotkeys(),   HotkeyAction::SwitchZ (2));
    register (&ss.conf.get_third_last_window_switch_hotkeys(),    HotkeyAction::SwitchZ (3));

    register (&ss.conf.get_switch_next_non_minimized_hotkeys(),   HotkeyAction::SwitchNextNonMinimized);

    // and for hotkeys specified in config file to take snapshot of windows, and switch through them without bringing switche up
    register (&ss.conf.get_windows_list_snapshot_hotkeys(),       HotkeyAction::SnapListRefresh);
    register (&ss.conf.get_snap_list_switch_next_hotkeys(),       HotkeyAction::SnapListNext);
    register (&ss.conf.get_snap_list_switch_prev_hotkeys(),       HotkeyAction::SnapListPrev);
    register (&ss.conf.get_snap_list_switch_top_hotkeys(),        HotkeyAction::SnapListTop);
    register (&ss.conf.get_snap_list_switch_bottom_hotkeys(),     HotkeyAction::SnapListBottom);

    // finally, we'll also register any hotkeys specified in config file for direct switch to specific exe/title
    ss.conf.get_direct_app_switch_hotkeys() .into_iter() .for_each (|(hotkey, exe, title, partial)| {
        register (&[hotkey], HotkeyAction::SwitchApp { exes: exe.as_slice().to_vec(), title, partial })
    });

    let _ = HOTKEY_ACTIONS.set (Mutex::new (actions));
    // the manager must stay alive for the duration of the process
    std::mem::forget (manager);

    // global-hotkey dispatches events through a win32 message window created on this (the main) thread,
    // .. and exposes them on a channel, which we can drain on our own thread
    thread::spawn ( move || {
        let rx = GlobalHotKeyEvent::receiver();
        while let Ok (ev) = rx.recv() {
            if ev.state != HotKeyState::Pressed { continue }
            let action = HOTKEY_ACTIONS.get()
                .and_then (|m| m.lock().ok())
                .and_then (|m| m.get (&ev.id).cloned());
            if let Some (action) = action {
                info! ("global hotkey fired: [{:?}]", action);
                dispatch_hotkey_action (ss, action)
            }
        }
    } );
}
