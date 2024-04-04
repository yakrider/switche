// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr (
    all ( not(debug_assertions), target_os = "windows" ),
    windows_subsystem = "windows"
)]



fn main() {

    switche::tauri::run_switche_tauri ( &switche::switche::SwitcheState::instance() );

}
