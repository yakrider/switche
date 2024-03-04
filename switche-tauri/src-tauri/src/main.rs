// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr (
    all ( not(debug_assertions), target_os = "windows" ),
    windows_subsystem = "windows"
)]

use tauri::{SystemTray, CustomMenuItem, SystemTrayMenu};
use switche::SwitcheState;


fn main() {

    let ssi = SwitcheState::instance();

    // we'll setup tray-icon support to pass into app builder
    let tray = SystemTray::new().with_menu (
        SystemTrayMenu::new ()
            .add_item ( CustomMenuItem::new ( "show",   "Show"    ) )
            .add_item ( CustomMenuItem::new ( "quit",   "Quit"    ) )
            .add_item ( CustomMenuItem::new ( "rstart", "Restart" ) )
    );

    let app = {
        tauri::Builder::default() .setup ( {
            let ss = ssi.clone();
            move |app| {
                ss.setup_front_end_listener (&app.handle());
                ss.setup_global_shortcuts   (&app.handle());
                app.set_device_event_filter(tauri::DeviceEventFilter::Always);
                // ^^ w/o this, our own LL input hooks will not receive events when tauri window is fgnd
                Ok(())
            }
        } )
        .invoke_handler (tauri::generate_handler![])
        .system_tray (tray)
        .on_system_tray_event ({
            let ss = ssi.clone();
            move |ah, event| { ss.tray_events_handler (ah, event) }
        })
        .build (tauri::generate_context!())
        .expect ("error while building tauri application")
        //.run (tauri::generate_context!())
        //.expect ("error while running tauri application")
    };

    // we'll register the app with the switche engine so it can send back responses etc
    ssi.register_app_handle(app.handle());



    // now lets finally actually start the app! .. note that the run call wont return!
    let ss = ssi.clone();
    app .run ( move |ah, event| { ss.tauri_run_events_handler (ah, event) } );

}
