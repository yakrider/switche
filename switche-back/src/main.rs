// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr (
    all ( not(debug_assertions), target_os = "windows" ),
    windows_subsystem = "windows"
)]


fn main() {

    let ss = switche::switche::SwitcheState::instance();

    // we want the non-blocking log-appender guard to be here in main, to ensure any pending logs get flushed upon crash etc
    let _guard = ss.conf.setup_log_subscriber();

    tracing::info! ("Starting Switche ...");

    switche::tauri::run_switche_tauri ( &ss );

}
