

use std::collections::{HashMap, LinkedList};
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread::{sleep, spawn};
use std::time;

use grouping_by::GroupingBy;
use linked_hash_map::LinkedHashMap;
use once_cell::sync::OnceCell;

use tauri::{Manager, Window, AppHandle, Runtime};
use windows::Win32::Foundation::HWND;

use crate::*;

# [ derive ( ) ]
pub struct _IconsManager {
    // todo
}

# [ derive (Clone) ]
pub struct IconsManager ( Arc <_IconsManager> );

impl Deref for IconsManager {
    type Target = _IconsManager;
    fn deref(&self) -> &_IconsManager { &self.0 }
}

impl IconsManager {

    pub fn instance () -> IconsManager {
        static INSTANCE: OnceCell <IconsManager> = OnceCell::new();
        INSTANCE .get_or_init ( ||
            IconsManager ( Arc::new ( _IconsManager {
                // todo
            } ) )
        ) .clone()
    }

    pub fn clear_dead_hwnds (&self, hwnds:LinkedList<Hwnd>) {
        // todo
    }
    pub fn process_found_hwnd_exe_path (&self, hwnd:Hwnd) { //, dat:&WinDatEntry) {
        // todo
    }
    pub fn queue_icon_refresh (&self, hwnd:Hwnd) {

    }

}


pub struct IconPathOverridesManager ;

impl IconPathOverridesManager {

    pub fn instance() -> IconPathOverridesManager {
        // todo update this later
        IconPathOverridesManager
    }

    pub fn get_icon_override_path (wde:&WinDatEntry) -> Option<String> {
        // todo .. or remove if we dont need this anymore
        None
    }

}
