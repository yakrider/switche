// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{Manager, Window, AppHandle, Runtime};

#[tauri::command]
fn greet<R: Runtime>(ah: AppHandle<R>, name: &str) -> String {
    let ah = ah.clone();
    std::thread::spawn (move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let _ = ah.get_window("main").iter() .for_each (|w| {
            println!("emitting [greeter] : {:?}",std::time::Instant::now());
            //w.emit_all::<String>("ping", "ping from backend".into()).expect("emit failed");
            ah.emit_all::<String>("ping", "ping from backend".into()).expect("emit failed");
        });
    });
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn main() {
    let app = tauri::Builder::default()
        .setup (|app| {
            let ah = app.handle();
            std::thread::spawn (move || {
                std::thread::sleep(std::time::Duration::from_millis(500));
                println!("emitting [setup  ] : {:?}",std::time::Instant::now());
                ah.emit_all::<String>("ping", "ping from setup".into()).unwrap();
            } );
            app.listen_global ("clear", |event| {
                println!("got event-name with msg {:?}", event.payload());
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        //.build(tauri::generate_context!())
        //.expect("error while building tauri application");
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
