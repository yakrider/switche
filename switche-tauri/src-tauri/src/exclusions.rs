

use std::ops::Deref;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::collections::{HashSet};

use once_cell::sync::{Lazy, OnceCell};


use crate::*;


# [ derive ( ) ]
pub struct _ExclusionsManager {
    self_hwnd : AtomicIsize,
}

# [ derive (Clone) ]
pub struct ExclusionsManager ( Arc <_ExclusionsManager> );

impl Deref for ExclusionsManager {
    type Target = _ExclusionsManager;
    fn deref(&self) -> &_ExclusionsManager { &self.0 }
}

impl ExclusionsManager {

    pub fn instance () -> ExclusionsManager {
        static INSTANCE: OnceCell <ExclusionsManager> = OnceCell::new();
        INSTANCE .get_or_init ( ||
            ExclusionsManager ( Arc::new ( _ExclusionsManager {
                self_hwnd : AtomicIsize::default(),
            } ) )
        ) .clone()
    }

    pub fn store_self_hwnd (&self, hwnd:Hwnd) { self.self_hwnd.store (hwnd, Ordering::Relaxed) }
    pub fn self_hwnd (&self) -> Hwnd { self.self_hwnd.load(Ordering::Relaxed) }
    pub fn check_self_hwnd (&self, hwnd:Hwnd) -> bool { hwnd == self.self_hwnd() }

    pub fn calc_excl_flag (&self, wde:&WinDatEntry) -> bool {
        //wde.is_vis == Some(false) ||  wde.is_uncloaked == Some(false) ||
        self.check_self_hwnd(wde.hwnd) || wde.win_text.is_none() ||
            wde.exe_path_name.as_ref() .filter (|p| !p.full_path.is_empty()) .is_none() ||
            wde.exe_path_name.as_ref() .filter (|p| !self.filter_exe_match(&p.name)) .is_none()
    }

    pub fn runtime_should_excl_check (&self, wde:&WinDatEntry) -> bool {
        // todo .. gotta flesh this out more?
        wde.should_exclude .unwrap_or_else ( move || self.calc_excl_flag(wde) )
    }

    fn filter_exe_match (&self, estr:&String) -> bool {
        static EXE_MATCH_SET : Lazy <RwLock <HashSet <String>>> = Lazy::new ( || {
            let mut m = HashSet::new();
            m.insert ("WDADesktopService.exe".to_string());
            RwLock::new(m)
        } );
        EXE_MATCH_SET.read().unwrap().contains(estr)
    }

    pub fn filter_exclusions (&self, _render_list: &Vec<WinDatEntry>) -> Vec<WinDatEntry> {
        // todo
        vec![]
    }

}
