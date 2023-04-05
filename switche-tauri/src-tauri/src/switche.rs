#![ allow (non_camel_case_types) ]

//#[macro_use] extern crate serde;
//#[macro_use] extern crate serde_json;
//#[macro_use] extern crate strum_macros;

use std::cell::{Ref, RefCell};
use std::collections::{HashMap, LinkedList};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
//use no_deadlocks::RwLock;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread::{sleep, spawn};
use std::time;
use std::time::{Duration, Instant};

use grouping_by::GroupingBy;
use linked_hash_map::LinkedHashMap;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize, Serializer};
use serde::ser::SerializeStruct;
use strum_macros::{AsRefStr, AsStaticStr};
use tauri::{App, AppHandle, Manager, RunEvent, WindowEvent, Wry};
use windows::Win32::Foundation::{BOOL, HINSTANCE, HWND, LPARAM};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, WINEVENTPROC};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, EVENT_OBJECT_CLOAKED, EVENT_OBJECT_CREATE, EVENT_OBJECT_DESTROY, EVENT_OBJECT_FOCUS, EVENT_OBJECT_HIDE, EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_SHOW, EVENT_OBJECT_UNCLOAKED, EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND, GetMessageW, MSG, WNDENUMPROC};
use crate::*;


pub type Hwnd = isize;




# [ derive (Debug, Default, Eq, PartialEq, Hash, Clone, Serialize, Deserialize) ]
pub struct ExePathName {
    pub full_path : String,
    pub name      : String
}

# [ derive (Debug, Default, Eq, PartialEq, Hash, Clone, Serialize, Deserialize) ]
pub struct WinDatEntry {
    pub hwnd              : Hwnd,
    pub is_vis            : Option<bool>,
    pub is_uncloaked      : Option<bool>,
    pub win_text          : Option<String>,
    pub exe_path_name     : Option<ExePathName>,
    pub should_exclude    : Option<bool>,
    pub icon_override_loc : Option<String>,
}

# [ derive (Debug, Clone, Serialize, Deserialize) ]
pub struct WinDatEntry_FE {
    pub hwnd           : Hwnd,
    pub win_text       : Option<String>,
    pub exe_path_name  : Option<ExePathName>,
    pub icon_cache_idx : u32,
}

#[allow(non_camel_case_types)]
#[derive (Debug, Eq, PartialEq, Hash, Copy, Clone, AsRefStr, Serialize, Deserialize)]
pub enum WinDatEntry_Msg {
    wde_msg__init_load,
    wde_msg__dat_update,
    wde_msg__title_change,
}
impl WinDatEntry_Msg {
    fn str (&self) -> &str { self.as_ref() }
}

# [ derive (Debug, Clone, Serialize, Deserialize) ]
pub struct WinDatEntry_P {
    pub msg : String,
    pub wde : WinDatEntry_FE
}




#[allow(non_camel_case_types)]
#[derive (Debug, Eq, PartialEq, Hash, Copy, Clone, AsRefStr, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend_Notice {
    hotkey_req__app_invoke,
    hotkey_req__scroll_down,
    hotkey_req__scroll_up,
    hotkey_req__scroll_end,
}
impl Backend_Notice {
    fn str (&self) -> &str { self.as_ref() }
}

# [ derive (Debug, Eq, PartialEq, Hash, Default, Clone, Serialize, Deserialize) ]
struct BackendNotice_Pl {
    msg: String
}




# [ derive (Debug, Eq, PartialEq, Hash, Default, Copy, Clone, Serialize, Deserialize) ]
pub struct RenderListEntry {
    hwnd : Hwnd,
    y    : u32,
}

# [ derive (Debug, Eq, PartialEq, Hash, Default, Clone, Serialize, Deserialize) ]
pub struct RenderList_P {
    rl  : Vec <RenderListEntry>,
    grl : Vec <Vec <RenderListEntry>>,
}




#[allow(non_camel_case_types)]
#[derive (Debug, Eq, PartialEq, Hash, Copy, Clone, AsRefStr, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend_Event {
    backend_notice,
    updated_win_dat_entry,
    updated_render_list,
    updated_icon_entry,
    updated_icon_lookup_entry
}
impl Backend_Event {
    fn str (&self) -> &str { self.as_ref() }
}




#[derive (Debug, Eq, PartialEq, Hash, Clone, Serialize, Deserialize)]
pub struct FrontendRequest {
    req  : String,
    hwnd : Option<i32>
}






# [ derive (Debug, Default, Clone) ]
/// pure sugar for representation of our atomic-bool flags
pub struct Flag (Arc <AtomicBool>);
// ^^ simple sugar that helps reduce clutter in code

impl Flag {
    pub fn check (&self) -> bool { self.0 .load (Ordering::SeqCst) }
    pub fn set   (&self) { self.0 .store (true,  Ordering::SeqCst) }
    pub fn clear (&self) { self.0 .store (false, Ordering::SeqCst) }
}

# [ derive ( ) ]
pub struct _SwitcheState {

    // note: should always use hwnd-map-prior as that only flips fully formed (unlike hamp-cur which might be getting slowly rebuilt)
    pub hwnd_map : Arc <RwLock <LinkedHashMap <Hwnd, WinDatEntry>>>,
    hwnd_map_acc : Arc <RwLock <LinkedHashMap <Hwnd, WinDatEntry>>>,

    pub cur_call_id  : AtomicIsize,
    pub is_dismissed : Flag,

    pub render_lists_m : RenderReadyListsManager,
    pub icons_m        : IconsManager,
    pub excl_m         : ExclusionsManager,

    app_handle : Arc <RwLock < Option <AppHandle<Wry>>>>,

}

# [ derive (Clone) ]
pub struct SwitcheState ( Arc <_SwitcheState> );

impl Deref for SwitcheState {
    type Target = _SwitcheState;
    fn deref (&self) -> &Self::Target { &self.0 }
}





# [ derive (Debug, Default, Copy, Clone) ]
struct GroupSortingEntry {
    seen_count: u32,
    mean_perc_idx: f32
}

# [ derive ( ) ]
pub struct _RenderReadyListsManager {
    //switche_state_r  : &'a SwitcheState,
    grp_sorting_map  : Arc <RwLock <HashMap <String, GroupSortingEntry>>>,
    render_list      : Arc <RwLock <Vec <RenderListEntry>>>,
    grpd_render_list : Arc <RwLock <Vec <Vec <RenderListEntry>>>>,
    last_render_targ_stamp : Instant,
    render_pending   : Flag,
}


# [ derive (Clone) ]
pub struct RenderReadyListsManager ( Arc <_RenderReadyListsManager> );

impl Deref for RenderReadyListsManager {
    type Target = _RenderReadyListsManager;
    fn deref (&self) -> &Self::Target { &self.0 }
}







impl SwitcheState {

    pub fn instance () -> SwitcheState {
        static INSTANCE: OnceCell <SwitcheState> = OnceCell::new();
        INSTANCE .get_or_init ( || {
            let ss = SwitcheState ( Arc::new ( _SwitcheState {
                hwnd_map     : Arc::new ( RwLock::new (LinkedHashMap::new())),
                hwnd_map_acc : Arc::new ( RwLock::new (LinkedHashMap::new())),
                cur_call_id  : AtomicIsize::default(),
                is_dismissed : Flag::default(),
                render_lists_m: RenderReadyListsManager::instance(),
                icons_m: IconsManager::instance(),
                excl_m: ExclusionsManager::instance(),
                app_handle : Arc::new ( RwLock::new (None)),
            } ) );
            // lets do some init for the newly created singleton
            ss.setup_win_event_hooks();
            ss
        } ) .clone()
    }

    pub fn register_app_handle (&self, ah:AppHandle<Wry>) {
        *self.app_handle.write().unwrap() = Some(ah);
    }




    /*****  win-api windows-enumeration setup and processing  ******/

    fn trigger_enum_windows_query (&self) {
        println!("***** starting new enum-windows query!");
        let call_id_old = self.cur_call_id.fetch_add (1, Ordering::SeqCst);
        { *self.hwnd_map_acc.write().unwrap() = LinkedHashMap::new(); }
        // enum windows is blocking, and returns a bool which is false if it fails or either of its callback calls returns false
        // so we'll spawn out this call, and there, we'll wait till its done then trigger cleanup and rendering etc
        let ss = self.clone();
        spawn ( move || unsafe {
            let t = Instant::now();
            let res = EnumWindows ( Some(Self::enum_windows_streamed_callback), LPARAM (call_id_old + 1) );
            let dur = Instant::now().duration_since(t).as_millis();
            println! ("enum-windows query completed in {dur} ms, with success result: {:?}", res);
            if res == false { return }    // the call could have been superceded by a newer request
            sleep (Duration::from_millis(20)); // let any pending updates finish .. the actual call with fast cbs is just a few ms!
            ss.post_enum_win_call_cleanup();
        } );

    }


    pub unsafe extern "system" fn enum_windows_streamed_callback (hwnd:HWND, call_id:LPARAM) -> BOOL {
        let ss = SwitcheState::instance();
        let latest_call_id = ss.cur_call_id.load(Ordering::Relaxed);
        if call_id.0 > latest_call_id {
            println! ("WARNING: got win-api callback w higher call_id than last triggered .. will restart enum-call! !");
            ss.trigger_enum_windows_query();
            return BOOL ((false as i32))
        } else if call_id.0 < latest_call_id {
            // if we're still getting callbacks with stale call_id, signal that call to stop
            println! ("WARNING: got callbacks @cur-call-id {} from stale cb-id: {} .. ending it!!", latest_call_id, call_id.0);
            return BOOL (false as i32)
        };
        let mut tmp = WinDatEntry::default(); tmp.hwnd = hwnd.0;
        let wde = ss.hwnd_map.read().unwrap().get(&hwnd.0) .or_else (|| Some(&tmp)) .as_ref() .map (|&wde| ss.get_updated_dat(&wde)) .unwrap();
        let should_excl = wde.should_exclude.unwrap_or(false);
        // we want to update the map before calling icon updates in case they want to look back up there
        ss.hwnd_map_acc.write().unwrap() .insert (wde.hwnd, wde);
        if !should_excl { ss.icons_m.process_found_hwnd_exe_path (hwnd.0); }
        BOOL (true as i32)
    }
    // todo, prob add something to not show child windows either? .. alt-tab doesnt seem to show child IDE windows


    fn post_enum_win_call_cleanup (&self) {
        // note that among the steps below, some must be done before hmap/hmap-acc swap, and other afterwards!
        let dead_hwnds = self.hwnd_map.read().unwrap().keys() .filter (|k|
            !self.hwnd_map_acc.read().unwrap().contains_key(k)
        ) .map(|&i| i) .collect::<LinkedList<Hwnd>>();
        IconsManager::instance().clear_dead_hwnds (dead_hwnds);
        std::mem::swap (&mut *self.hwnd_map.write().unwrap(), &mut *self.hwnd_map_acc.write().unwrap());
        println!("hm:{}, hmacc:{}", self.hwnd_map.read().unwrap().len(), self.hwnd_map_acc.read().unwrap().len());

        // lets also find and cache our hwnd if we dont have it cached yet
        if self.excl_m.self_hwnd() == 0 || !self.hwnd_map.read().unwrap().contains_key (&self.excl_m.self_hwnd()) {
            self.excl_m.cache_self_hwnd(self)
        }
        self.emit_render_lists_immdt();
    }


    fn get_exe_path_name(hwnd:Hwnd) -> Option<ExePathName> {
        let exe_path = win_apis::get_exe_path_name(hwnd);
        exe_path .map ( |fp| {
            let name = fp .split(r"\") .last() .unwrap_or_default() .to_string();
            ExePathName { full_path: fp, name }
        } )
    }


    fn get_updated_dat (&self, wde:&WinDatEntry) -> WinDatEntry { //println!("updating dat.. {:?}", wde.hwnd);
        let is_vis = Some (win_apis::check_window_visible (wde.hwnd));
        let is_uncloaked = is_vis .filter(|&b| b) .map (|_| !win_apis::check_window_cloaked(wde.hwnd));
        let win_text = is_uncloaked .filter(|&b| b) .map ( |_| win_apis::get_window_text(wde.hwnd));
        let exe_path_name = if win_text.as_ref().map_or(false, |s| !s.is_empty()) && wde.exe_path_name.is_none() {
            Self::get_exe_path_name(wde.hwnd)
        } else { wde.exe_path_name.to_owned() };
        let mut updated_wde = WinDatEntry {
            hwnd: wde.hwnd, is_vis, is_uncloaked, win_text, exe_path_name,
            should_exclude: None, icon_override_loc: None,
        };
        let should_excl_flag = Some ( self.excl_m.calc_excl_flag (&updated_wde) );
        let icon_override_loc = should_excl_flag .filter(|&b| !b) .map (|_| IconPathOverridesManager::get_icon_override_path(wde)) .flatten();
        updated_wde.should_exclude = should_excl_flag;
        updated_wde.icon_override_loc = icon_override_loc;

        // for now, we'll set to emit from here, but should prob find a more conspicuous place to trigger this
        if updated_wde.should_exclude != Some(true) && &updated_wde != wde {
            self.emit_win_dat_entry(&updated_wde, WinDatEntry_Msg::wde_msg__dat_update)
        }
        updated_wde
    }





    /*****  common support functions  ******/

    fn handle_event__switche_fgnd (&self) {
        println! ("switche self-fgnd report .. refreshing window-list-top icon");
        self.is_dismissed.clear();
        // the idea below is that to keep icons mostly updated, we do icon-refresh for a window when it comes to fgnd ..
        // however, when switche is brought to fgnd, recent changes in the topmost window might not be updated yet .. so we'll trigger that here
        self .render_lists_m.render_list.read().unwrap() .first() .iter() .for_each ( |rle|
            IconsManager::instance() .queue_icon_refresh (rle.hwnd)
        );
    }

    fn activate_matching_window (&self, exe:Option<String>, title:Option<String>) {
        let hwnd_map = self.hwnd_map.read().unwrap();
        let hwnd : Option<Hwnd> = self.render_lists_m.render_list.read().unwrap() .iter() .map ( |rle| {
            //let hwnd = rle.hwnd;
            hwnd_map .get (&rle.hwnd)
        } ) .flatten() .filter ( |wde|
            ( exe.is_none()   || exe.as_ref() == wde.exe_path_name.as_ref().map(|p| &p.name) ) &&
            ( title.is_none() || title == wde.win_text )
        ) .next() .map (|wde| wde.hwnd);
        // if we found the hwnd, if its not already active, activate it, else switch to next window
        if hwnd.is_none() {
            // didnt find anything, so do nothing
        } else if hwnd == self.render_lists_m.render_list.read().unwrap().first().map(|rle| rle.hwnd) {
            // found it, its already at top, so toggle to the second top window instead
            self.handle_req__second_recent_window_activate()
        } else { // aight, found it, and its not top, switch to it
            hwnd .iter() .for_each (|&hwnd| self.handle_req__window_activate(hwnd))
        }
    }







    /*****   win-api reports handling  ******/

    pub fn setup_win_event_hooks (&self) {
        spawn ( move || unsafe {
            /* Reference:
                pub unsafe fn SetWinEventHook (
                    eventmin: u32, eventmax: u32, cb_dll: HINSTANCE, cb: WINEVENTPROC,
                    idprocess: u32, idthread: u32, dwflags: u32
                ) -> HWINEVENTHOOK

                We'll put these in two separate hooks because the (system and object) events are in two separate ranges
                    separating them this way avoids pointless calls for events that'd end up within the wider range of a single hook
                That said, we can process both hook callbacks in the message loop in this thread

                 System events we might be interested in:
                    0x03   : EVENT_SYSTEM_FOREGROUND
                    0x14   : EVENT_SYSTEM_SWITCHSTART       // alt-tab start
                    0x15   : EVENT_SYSTEM_SWITCHEND         // alt-tab end
                    0x16   : EVENT_SYSTEM_MINIMIZESTART
                    0x17   : EVENT_SYSTEM_MINIMIZEEND

                Object events we might be interested in
                    0x8000 : EVENT_OBJECT_CREATE
                    0x8001 : EVENT_OBJECT_DESTROY
                    0x8002 : EVENT_OBJECT_SHOW
                    0x8003 : EVENT_OBJECT_HIDE
                    0x8005 : EVENT_OBJECT_FOCUS
                    0x800C : EVENT_OBJECT_NAMECHANGE
                    0x8017 : EVENT_OBJECT_CLOAKED
                    0x8018 : EVENT_OBJECT_UNCLOAKED

                However, listening to a subset might be enough as events often gen in sets (e.g fgnd then focus etc)
             */

            SetWinEventHook( 0x0003, 0x0017, HINSTANCE::default(), Some(Self::win_hook_cb__system_events), 0, 0, 0);
            SetWinEventHook( 0x8001, 0x8018, HINSTANCE::default(), Some(Self::win_hook_cb__object_events), 0, 0, 0);

            // win32 sends hook events to a thread with a 'message loop', but we wont create any windows here to get window messages,
            //     so we'll just leave a forever waiting GetMessage instead of setting up a msg-loop
            // .. basically while its waiting, the thread is awakened simply to call kbd hook (for an actual msg, itd awaken give the msg)
            let mut msg: MSG = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
            while BOOL(0) != GetMessageW (&mut msg, HWND(0), 0, 0) { };

        } );
    }


    pub unsafe extern "system" fn win_hook_cb__system_events (
        _id_hook: HWINEVENTHOOK, event: u32, hwnd: HWND,
        id_object: i32, _id_child: i32, _id_thread: u32, _event_time: u32
    ) {
        if (id_object == 0) {
            //println!("hook callback .. event: 0x{:X}, id_object: 0x{:X}", event, id_object);
            //let ss = SwitcheState::instance();
            match event {
                EVENT_SYSTEM_FOREGROUND  => SwitcheState::instance().proc_win_report__fgnd_hwnd (hwnd.0),
                EVENT_SYSTEM_MINIMIZEEND => SwitcheState::instance().proc_win_report__fgnd_hwnd (hwnd.0),
                _ => { }
            }
        }
    }

    pub unsafe extern "system" fn win_hook_cb__object_events (
        _id_hook: HWINEVENTHOOK, event: u32, hwnd: HWND,
        id_object: i32, _id_child: i32, _id_thread: u32, _event_time: u32
    ) {
        if (id_object == 0) {
            //println!("hook callback .. event: 0x{:X}, id_object: 0x{:X}", event, id_object);
            //let ss = SwitcheState::instance();
            match event {
                EVENT_OBJECT_DESTROY    => SwitcheState::instance().proc_win_report__obj_destroyed  (hwnd.0),
                EVENT_OBJECT_SHOW       => SwitcheState::instance().proc_win_report__obj_shown      (hwnd.0),
                EVENT_OBJECT_HIDE       => SwitcheState::instance().proc_win_report__obj_destroyed  (hwnd.0),
                EVENT_OBJECT_FOCUS      => SwitcheState::instance().proc_win_report__fgnd_hwnd      (hwnd.0),
                EVENT_OBJECT_NAMECHANGE => SwitcheState::instance().proc_win_report__title_changed  (hwnd.0),
                EVENT_OBJECT_CLOAKED    => SwitcheState::instance().proc_win_report__obj_destroyed  (hwnd.0),
                EVENT_OBJECT_UNCLOAKED  => SwitcheState::instance().proc_win_report__obj_shown      (hwnd.0),
                _ => { }
            }
        }
    }


    pub fn proc_win_report__title_changed  (&self, hwnd:Hwnd) {
        let mut wde_opt = self.hwnd_map.read().unwrap() .get(&hwnd) .map(|wde| wde.clone());
        // ^^ doing this separately w a clone clears us of the hwnd-map read lock scope so we can write back below
        // todo: ugh do actual check if indeed necessary
        wde_opt.map ( |mut wde| {
            wde.win_text = Some (win_apis::get_window_text(hwnd));
            wde.should_exclude = Some (self.excl_m.calc_excl_flag(&wde));
            // lets send msgs out before the wde copy is consumed by putting into map
            self.emit_win_dat_entry (&wde, WinDatEntry_Msg::wde_msg__title_change);
            self.hwnd_map.write().unwrap() .insert (hwnd, wde);
            // but for icons, it must come afterwards as it might want to read/update the wde too
            self.icons_m.process_found_hwnd_exe_path(hwnd);
        } );
    }
    pub fn proc_win_report__fgnd_hwnd (&self, hwnd:Hwnd) {
        println! ("hwnd {:?} fgnd", hwnd); //return;
        // note that our hmaps are linked hash maps and the order is important there, so either we gotta re-query or rebuild the map
        // todo .. could prob update the whole setup to just hold a vector that gets reordered/swapped while the entries remain in some hashmap
        // .. makes more sense to do that now than before in scala because there we'd just build a new map with refs to old entries and
        // .. then swap the maps .. but in rust, we'd end up making copies (or else try to do some messy in-place reorg etc)
        let wde = self.hwnd_map.read().unwrap().get(&hwnd) .map(|r| r.clone()) .or_else ( || {
            let mut tmp = WinDatEntry::default();
            tmp.hwnd = hwnd;
            Some(tmp)
        } ) .map (|wde| self.get_updated_dat(&wde)) .unwrap();
        let is_excl = wde.should_exclude.as_ref() == Some(true).as_ref();
        let mut hmap_updated = LinkedHashMap::<Hwnd,WinDatEntry>::new();
        hmap_updated.insert (hwnd, wde);
        self.hwnd_map.read().unwrap() .values() .for_each ( |wde| {
            if wde.hwnd != hwnd { hmap_updated.insert (wde.hwnd, wde.clone()); }
        } );
        std::mem::swap (&mut *self.hwnd_map.write().unwrap(), &mut hmap_updated);
        if !is_excl {
            self.icons_m.queue_icon_refresh(hwnd);
            self.emit_render_lists_queued();
        }
        let ss = self.clone();
        spawn ( move || {
            sleep (Duration::from_millis(150));
            ss.handle_req__refresh();
        } );
    }
    pub fn proc_win_report__obj_shown (&self, hwnd:Hwnd) {

    }
    pub fn proc_win_report__obj_destroyed (&self, hwnd:Hwnd) {

    }






    /*****   Front-End Requests handling  ******/

    fn handle_req__window_activate (&self, hwnd:Hwnd) {
        // since this is only for non-self windows, we'll want to dimiss ourselves first
        self.handle_req__switche_dismiss();
        spawn ( move || {
            //sleep (Duration::from_millis(30));
            win_apis::window_activate(hwnd);
            sleep (Duration::from_millis(50));
            win_apis::window_activate(hwnd);
        } );
    }
    fn handle_req__window_peek (&self, hwnd:Hwnd) {
        let self_hwnd = self.excl_m.self_hwnd();
        spawn ( move || {
            //sleep (Duration::from_millis(30));
            win_apis::window_activate(hwnd);
            sleep (Duration::from_millis(50));
            win_apis::window_activate(hwnd);
            // after 'showing'some window for a bit, we'll bring back ourselves
            sleep (Duration::from_millis(1000));
            win_apis::window_activate(self_hwnd);
        } );
    }
    fn handle_req__window_minimize (&self, hwnd:Hwnd) {
        spawn ( move || {
            //sleep (Duration::from_millis(30));
            win_apis::window_minimize(hwnd);
            sleep (Duration::from_millis(60));
            win_apis::window_minimize(hwnd);
        } );
    }
    fn handle_req__window_close (&self, hwnd:Hwnd) {
        spawn ( move || {
            //sleep (Duration::from_millis(30));
            win_apis::window_activate(hwnd);
            sleep (Duration::from_millis(50));
            win_apis::window_activate(hwnd);

            sleep (Duration::from_millis(80));
            win_apis::window_close(hwnd);
            sleep (Duration::from_millis(120));
            win_apis::window_close(hwnd);
        } );
    }


    fn self_window_activate (&self) {
        self.is_dismissed.clear();
        //self.app_handle .read().unwrap() .iter().for_each (|ah| {
        //    ah.get_window("main") .iter() .for_each (|wh| { wh.show(); wh.set_focus(); } )
        //});
        // ^^ doing this via tauri seems less reliable, and it seems to inject some alt-key events .. so we'll do it outselves!
        let self_hwnd = self.excl_m.self_hwnd();
        spawn ( move || {
            //sleep (Duration::from_millis(40));
            win_apis::window_activate (self_hwnd);
            sleep (Duration::from_millis(40));
            win_apis::window_activate (self_hwnd);
        } );
    }
    fn self_window_hide (&self) {
        self.is_dismissed.set();
        //self.app_handle .read().unwrap() .iter().for_each (|ah| {
        //    ah.get_window("main") .iter() .for_each (|wh| { wh.hide(); } )
        //} );
        let self_hwnd = self.excl_m.self_hwnd();
        spawn ( move || {
            //sleep (Duration::from_millis(40));
            win_apis::window_hide (self_hwnd);
            sleep (Duration::from_millis(40));
            win_apis::window_hide (self_hwnd);
        } );
    }


    fn handle_req__data_load(&self) {
        self.render_lists_m.render_list.read().unwrap() .iter() .for_each ( |rle| {
            self.hwnd_map.read().unwrap() .get (&rle.hwnd) .iter() .for_each ( |wde| {
                self.emit_win_dat_entry (wde, WinDatEntry_Msg::wde_msg__init_load)
        }) });
        self.trigger_enum_windows_query();     // this will trigger a renderlist push once the call is done
    }
    fn handle_req__refresh (&self) {
        self.trigger_enum_windows_query()
    }
    fn handle_req__refresh_idle (&self) {
        if self.is_dismissed.check() { self.trigger_enum_windows_query() }
    }

    fn handle_req__nth_recent_window_activate (&self, n:usize) {
        let hwnd = self.render_lists_m.render_list.read().unwrap() .get(n) .map (|e| e.hwnd);
        spawn ( move || {
            //sleep (Duration::from_millis(40));
            hwnd .map ( |hwnd| win_apis::window_activate(hwnd) );
            sleep (Duration::from_millis(40));
            hwnd .map ( |hwnd| win_apis::window_activate(hwnd) );
        } );
    }



    fn handle_req__switche_dismiss (&self) {
        // note: this is called both after some window-activation, or after escaping switche w/o explicitly activating other window
        self.self_window_hide();
    }

    fn handle_req__switche_escape (&self) {
        // this is called specifically upon escape from switche, and we'll want to reactivate last active window before we dismiss
        self.handle_req__switche_dismiss();
        self.handle_req__nth_recent_window_activate(0);
    }

    fn handle_req__second_recent_window_activate (&self) {
        self.handle_req__nth_recent_window_activate(1);
    }

    fn handle_req__debug_print (&self) { }



    pub fn handle_frontend_request (&self, r:&FrontendRequest) {
        println! ("received front-end request : {:?}", r);

        match r.req.as_str() {
            "fe_req_window_activate"      => { r.hwnd .map (|h| self.handle_req__window_activate (h as Hwnd) ); }
            "fe_req_window_peek"          => { r.hwnd .map (|h| self.handle_req__window_peek     (h as Hwnd) ); }
            "fe_req_window_minimize"      => { r.hwnd .map (|h| self.handle_req__window_minimize (h as Hwnd) ); }
            "fe_req_window_close"         => { r.hwnd .map (|h| self.handle_req__window_close    (h as Hwnd) ); }

            "fe_req_data_load"            => { self.handle_req__data_load()      }
            "fe_req_refresh"              => { self.handle_req__refresh()        }
            "fe_req_switche_escape"       => { self.handle_req__switche_escape() }
            "fe_req_debug_print"          => { self.handle_req__debug_print()    }

            // for some backend global hotkeys, we might have frontend hotkeys keys set too
            "fe_req_switch_tabs_last"     => { self.proc_hot_key__switch_last()          }
            "fe_req_switch_tabs_outliner" => { self.proc_hot_key__switch_tabs_outliner() }
            "fe_req_switch_notepad_pp"    => { self.proc_hot_key__switch_notepad_pp()    }
            "fe_req_switch_ide"           => { self.proc_hot_key__switch_ide()           }
            "fe_req_switch_winamp"        => { self.proc_hot_key__switch_winamp()        }
            "fe_req_switch_browser"       => { self.proc_hot_key__switch_browser()       }

            _ => { println! ("unrecognized frontend cmd: {}", r.req) }
        }
    }

    pub fn setup_front_end_listener (&self, ah:&AppHandle<Wry>) {
        // todo: could consider refactoring and doing this in main if want to save the id and maybe do unlisten before exit etc?
        let ss = self.clone();
        let _ = ah .listen_global ( "frontend_request", move |event| {
            //println!("got event with raw payload {:?}", &event.payload());
            event .payload() .map ( |ev| serde_json::from_str::<FrontendRequest>(ev).ok() )
                .flatten() .iter() .for_each (|ev| ss.handle_frontend_request(ev))
        } );
    }







    /*****  tauri app-window events setup and handling   ******/

    pub fn proc_app_window_event__focus (&self) {
        if self.is_dismissed.check() { self.handle_event__switche_fgnd() }
    }
    pub fn proc_app_window_event__blur  (&self) {
        // nothing really .. this doesnt even count as dismissed (which triggers list-cur-elem reset etc)
    }
    pub fn proc_app_window_event__show  (&self) {
        if self.is_dismissed.check() { self.handle_event__switche_fgnd() }
    }
    pub fn proc_app_window_event__hide  (&self) {
        self.is_dismissed.set()
    }

    pub fn tauri_window_events_handler (&self, ev:&WindowEvent) {
        match ev {
            WindowEvent::Focused(true) => { self.proc_app_window_event__focus () }
            _ => { }
        }
    }
    pub fn tauri_run_events_handler (&self, ah:&AppHandle<Wry>, event:RunEvent) {
        match event {
            //RunEvent::ExitRequested { api,   .. } => { api.prevent_exit() }
            RunEvent::WindowEvent   { event, .. } => { self.tauri_window_events_handler(&event) }
            _ => {}
        }
    }





    /*****  tauri registered hotkeys handling   ******/


    pub fn proc_hot_key__invoke (&self) {
        // for all practical purposes invoke is the same as scroll-down
        // (as we want even the first invocation to highlight second top list entry for quick switch behavior)
        self.proc_hot_key__scroll_down();
        //self.emit_backend_notice(hotkey_req__app_invoke)
    }
    pub fn proc_hot_key__scroll_down (&self) {
        // we'll ensure the app window is up, then let the frontend deal w it
        if self.is_dismissed.check() { self.handle_event__switche_fgnd(); }
        self.emit_backend_notice (Backend_Notice::hotkey_req__scroll_down);
        self.self_window_activate();
    }
    pub fn proc_hot_key__scroll_up (&self) {
        // again, we'll ensure the app window is up, then let the frontend deal w it
        if self.is_dismissed.check() { self.handle_event__switche_fgnd(); }
        self.emit_backend_notice (Backend_Notice::hotkey_req__scroll_up);
        self.self_window_activate();
    }
    pub fn proc_hot_key__scroll_end (&self) {
        // this requires activating the current elem in frontend, so we'll just send a msg over
        self.emit_backend_notice (Backend_Notice::hotkey_req__scroll_end);
    }


    pub fn proc_hot_key__switch_last          (&self)  { self.handle_req__second_recent_window_activate() }
    pub fn proc_hot_key__switch_tabs_outliner (&self)  { self.activate_matching_window ( Some("chrome.exe".into()), Some("Tabs Outliner".into()) ) }
    pub fn proc_hot_key__switch_notepad_pp    (&self)  { self.activate_matching_window ( Some("notepad++.exe".into()), None ) }
    pub fn proc_hot_key__switch_ide           (&self)  { self.activate_matching_window ( Some("idea64.exe".into()), None ) }
    pub fn proc_hot_key__switch_winamp        (&self)  { self.activate_matching_window ( Some("winamp.exe".into()), None ) }
    pub fn proc_hot_key__switch_browser       (&self)  { self.activate_matching_window ( Some("chrome.exe".into()), None ) }


    pub fn setup_global_shortcuts (&self, ah:&AppHandle<Wry>) {
        use tauri::GlobalShortcutManager;
        let mut gsm = ah.global_shortcut_manager();
        // todo: can update these to prob printout/notify an err msg when cant register global hotkey
        let ss = self.clone();  let _ = gsm.register ( "Super+F12",       move || ss.proc_hot_key__invoke()               );
        let ss = self.clone();  let _ = gsm.register ( "Super+F12",       move || ss.proc_hot_key__scroll_down()          );
        let ss = self.clone();  let _ = gsm.register ( "Super+Shift+F12", move || ss.proc_hot_key__scroll_up()            );
        let ss = self.clone();  let _ = gsm.register ( "F15",             move || ss.proc_hot_key__invoke()               );
        let ss = self.clone();  let _ = gsm.register ( "F16",             move || ss.proc_hot_key__scroll_down()          );
        let ss = self.clone();  let _ = gsm.register ( "F17",             move || ss.proc_hot_key__scroll_up()            );
        let ss = self.clone();  let _ = gsm.register ( "Ctrl+F18",        move || ss.proc_hot_key__scroll_end()           );
        let ss = self.clone();  let _ = gsm.register ( "Ctrl+Alt+F19",    move || ss.proc_hot_key__switch_last()          );
        let ss = self.clone();  let _ = gsm.register ( "Ctrl+Alt+F20",    move || ss.proc_hot_key__switch_tabs_outliner() );
        let ss = self.clone();  let _ = gsm.register ( "Ctrl+Alt+F21",    move || ss.proc_hot_key__switch_notepad_pp()    );
        let ss = self.clone();  let _ = gsm.register ( "Ctrl+Alt+F22",    move || ss.proc_hot_key__switch_ide()           );
        let ss = self.clone();  let _ = gsm.register ( "Ctrl+Alt+F23",    move || ss.proc_hot_key__switch_winamp()        );
        let ss = self.clone();  let _ = gsm.register ( "Ctrl+Alt+F24",    move || ss.proc_hot_key__switch_browser()       );

    }






    /*****    emitting backend messages   ******/


    fn emit_win_dat_entry(&self, wde:&WinDatEntry, msg: WinDatEntry_Msg) {
        //println! ("emitting **{notice}** win_dat_entry for: {:?}", wde.hwnd);
        let pl = WinDatEntry_P {
            msg: msg.str().to_string(),
            wde: WinDatEntry_FE {
                hwnd: wde.hwnd,
                win_text: wde.win_text.clone(),
                exe_path_name: wde.exe_path_name.clone(),
                icon_cache_idx: 0,
        } };
        self.app_handle.read().unwrap() .iter().for_each ( |ah| {
            serde_json::to_string(&pl) .map ( |pl| {
                ah.emit_all::<String> (Backend_Event::updated_win_dat_entry.str(), pl ) .or_else ( |err| Err("emit failure") )
            } ) .or_else (|err| { Err("json parse error")}) .err() .iter() .for_each (|err| println!{"win_dat emit failed: {err}"});
        } );
    }

    pub fn emit_backend_notice (&self, notice: Backend_Notice) {
        let pl = BackendNotice_Pl { msg: notice.str().to_string() };
        self.app_handle.read().unwrap() .iter() .for_each ( |ah| {
            serde_json::to_string(&pl) .map ( |pl| {
                println!("sending backend notice: {}", &pl);
                ah.emit_all::<String> ( "backend_notice", pl ) .or_else(|_| Err("emit failure"))
            } ).or_else (|err| { Err(err) }) .err() .iter().for_each (|err| println!("rl emit failed: {}", err));
        } )
    }


    pub fn emit_render_lists (&self) {
        let rlp = RenderList_P {
            rl  : self.render_lists_m.render_list.read().unwrap().clone(),
            grl : self.render_lists_m.grpd_render_list.read().unwrap().clone()
        };
        //println!("emitting renderlist ({:?}): {:?}", rlp.rl.len(), serde_json::to_string(&rlp).unwrap());
        self.app_handle.read().unwrap().iter().for_each ( |ah| {
            serde_json::to_string(&rlp).map(|pl| {
                ah.emit_all::<String>(Backend_Event::updated_render_list.str(), pl).or_else(|_| Err("emit failure"))
            }).or_else (|err| { Err(err) }) .err().iter().for_each (|err| println!("rl emit failed: {}", err));
        } )
    }

    pub fn emit_render_lists_immdt (&self) {
        self.render_lists_m.update_render_ready_lists(self);
        self.emit_render_lists();
    }
    pub fn emit_render_lists_queued (&self) {
        if !self.render_lists_m.render_pending.check() {
            self.render_lists_m.render_pending.set();
            let ss = self.clone();
            spawn ( move || {
                sleep (Duration::from_millis(80));
                ss.render_lists_m.render_pending.clear(); // todo .. rethink this stuff so it doesnt go out while still active etc
                ss.emit_render_lists_immdt();
            } );
        }
    }







}










impl RenderReadyListsManager {

    pub fn instance () -> RenderReadyListsManager {
        static INSTANCE: OnceCell <RenderReadyListsManager> = OnceCell::new();
        INSTANCE .get_or_init ( ||
            RenderReadyListsManager ( Arc::new ( _RenderReadyListsManager {
                grp_sorting_map        : Arc::new(RwLock::new(Default::default())),
                render_list            : Arc::new(RwLock::new(Default::default())),
                grpd_render_list       : Arc::new(RwLock::new(Default::default())),
                last_render_targ_stamp : Instant::now(),
                render_pending         : Flag::default(),
            } ) )
        ) .clone()
    }

    fn update_entry (&self, exe_path:&String, ge:GroupSortingEntry, perc_idx:f32) {
        let mean_perc_idx = (ge.mean_perc_idx * ge.seen_count as f32 + perc_idx) / (ge.seen_count + 1) as f32;
        self.grp_sorting_map.write().unwrap() .insert (
            exe_path.clone(), GroupSortingEntry { seen_count: ge.seen_count+1, mean_perc_idx }
        );
    }

    fn register_entry (&self, exe_path:&String, idx:usize, list_size:u32, do_update:bool) {
        let perc_idx : f32 = (idx as f32) / list_size as f32;
        if !self.grp_sorting_map.read().unwrap().contains_key(exe_path) {
            self.update_entry (exe_path, GroupSortingEntry::default(), perc_idx);
        } else if do_update {
            let ge = self.grp_sorting_map.read().unwrap() .get(exe_path) .copied(); // split to end read scope
            ge .iter() .for_each (|ge| self.update_entry (exe_path, *ge, perc_idx))
        }
    }

    fn grl_cmp_ext (&self, po:&Option<&String> ) -> Option<f32> {
        po.as_ref() .map (|&p| self.grp_sorting_map.read().unwrap().get(p).copied()) .flatten() .map (|gse| gse.mean_perc_idx)
    }

    fn recalc_render_ready_lists (&self, ss:&SwitcheState) -> (Vec<RenderListEntry>, Vec<Vec<RenderListEntry>>) {

        let em = ExclusionsManager::instance();
        let is_dismissed = ss.is_dismissed.check();     // local copy so we're not repeatedly doing atomic-reads in the filt_rlp loop below
        let hwnd_map = ss.hwnd_map.read().unwrap();     // borrowed to extend lifetime beyond single statement
        let filt_wdes = hwnd_map .iter() .filter (|(_,wde)| !em.runtime_should_excl_check(wde)) .map(|(_,wde)| wde) .collect::<Vec<_>>();
        let filt_rlp = filt_wdes .iter() .enumerate() .map ( |(i,wde)| {
            // we'll also register these while we're creating the RLEs
            let fp = wde.exe_path_name .as_ref() .map (|p| &p.full_path);
            fp .iter() .for_each ( |fp| {
                self.register_entry (fp, i, filt_wdes.len() as u32, is_dismissed)
            } );
            (fp, RenderListEntry { hwnd: wde.hwnd, y: i as u32 })
        } ) .collect::<Vec<(Option<&String>, RenderListEntry)>>();

        let filt_rl = filt_rlp .iter() .map (|(_,rle)| *rle) .collect::<Vec<RenderListEntry>>();

        let mut grpd_render_list_builder = {
            filt_rlp .iter() .grouping_by (|(fp,_)| fp) .iter()
                .map (|(fp,rlepv)| (*fp, rlepv.iter().map(|&(_,rle)| *rle).collect::<Vec<RenderListEntry>>()))
                .collect::<Vec<(&Option<&String>,Vec<RenderListEntry>)>>()
        };
        grpd_render_list_builder .sort_by ( |&(pa,_), &(pb,_)| {
            self.grl_cmp_ext (pa) .partial_cmp (&self.grl_cmp_ext(pb)) .unwrap()
            // note ^^ that unwrap is ok because it would fail only for NaN
        } );
        let grpd_render_list = grpd_render_list_builder .into_iter() .map (|(_,rlev)| rlev) .collect::<Vec<Vec<RenderListEntry>>>();
        println!("rrl recalc: rl:{:?}", filt_rl.len() );
        ( filt_rl, grpd_render_list )
    }

    fn update_render_ready_lists (&self, ss:&SwitcheState) {
        let (rl, grl) = self.recalc_render_ready_lists (ss);
        *self.render_list.write().unwrap() = rl;
        *self.grpd_render_list.write().unwrap() = grl;
    }




}
