// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::ops::Deref;
use std::thread::{sleep, spawn};
use std::time;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::collections::{HashMap, LinkedList};

use grouping_by::GroupingBy;
use linked_hash_map::LinkedHashMap;
use once_cell::sync::OnceCell;
use serde::{Serialize, Deserialize};

use tauri::{Manager, Window, AppHandle, Runtime, Icon, GlobalShortcutManager, RunEvent};
use switche::SwitcheState;


#[tauri::command]
fn greet<R: Runtime>(ah: AppHandle<R>, name: &str) -> String {
    let ah = ah.clone();
    spawn ( move || {
        sleep ( time::Duration::from_millis(500) );
        let _ = ah.get_window("main").iter() .for_each (|w| {
            println!("emitting [greeter] : {:?}", time::Instant::now());
            //w.emit_all::<String>("ping", "ping from backend".into()).expect("emit failed");
            ah.emit_all::<String>("ping", "ping from backend".into()).unwrap_or_else(|_|{});
        });
    });
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn main() {

    let ssi = SwitcheState::instance();

    let ss = ssi.clone();
    let app = {
        tauri::Builder::default()
        .setup ( move |app| {
            let ah = app.handle();
            ss.setup_front_end_listener (&ah);
            ss.setup_global_shortcuts (&ah);
            Ok(())
        } )
        .invoke_handler (tauri::generate_handler![])
        .build (tauri::generate_context!())
        .expect ("error while building tauri application")
        //.run (tauri::generate_context!())
        //.expect ("error while running tauri application")
    };

    // we'll register the app with the switche engine so it can send back responses etc
    ssi.register_app_handle(app.handle());

    // now lets finally actually start the app!
    let ss = ssi.clone();
    app .run ( move |ah, event| { ss.tauri_run_events_handler(ah, event) } );

}
