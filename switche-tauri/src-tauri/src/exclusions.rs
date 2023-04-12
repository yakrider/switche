

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

    pub fn cache_self_hwnd (&self, ss:&SwitcheState) {
        let self_hwnd_res = ss.hwnd_map .read().unwrap() .values() .find ( |&wde|
            //wde.win_text.as_ref() .filter (|s| s.as_str() == "Switche - Searchable Task Switcher") .is_some() && (
            wde.win_text == Some("Switche - Searchable Task Switcher".to_string()) && (
                wde.exe_path_name.as_ref() .filter (|ep| ep.name.as_str() == "switche.exe") .is_some() ||
                wde.exe_path_name.as_ref() .filter (|ep| ep.name.as_str() == "msedgewebview2.exe") .is_some()
        ) ) .map (|wde| wde.hwnd) ;
        // note: we're breaking up these sections simply to end the scope of the above hwnd-map read so we can do a write below!
        self_hwnd_res .and_then ( |hwnd| {
            self.self_hwnd.store(hwnd, Ordering::Relaxed);
            ss.hwnd_map.write().unwrap() .get_mut(&hwnd) .unwrap() .should_exclude = Some(true);
            Some(())
        } ) .or_else (|| { println!("WARNING: cannot find self hwnd!!"); None } );

    }
    pub fn self_hwnd (&self) -> Hwnd { self.self_hwnd.load(Ordering::Relaxed) }
    pub fn check_self_hwnd (&self, hwnd:Hwnd) -> bool { hwnd == self.self_hwnd() }

    pub fn calc_excl_flag (&self, wde:&WinDatEntry) -> bool {
        wde.is_vis == Some(false) ||  wde.is_uncloaked == Some(false) ||
            self.check_self_hwnd(wde.hwnd) || wde.win_text.is_none() || wde.win_text == Some("".to_string()) ||
            wde.exe_path_name.as_ref() .filter (|p| !p.full_path.is_empty()) .is_none() ||
            wde.exe_path_name.as_ref() .filter (|p| !self.filter_exe_match(&p.name)) .is_none()
    }

    pub fn runtime_should_excl_check (&self, wde:&WinDatEntry) -> bool {
        // todo .. gotta flesh this out more
        //if !wde.should_exclude.unwrap_or_else (move || self.calc_excl_flag(wde)) { println!("found a live one")}
        wde.should_exclude.unwrap_or_else ( move || self.calc_excl_flag(wde) )
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
