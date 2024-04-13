#![ allow (non_camel_case_types) ]
#![ allow (non_snake_case) ]
#![ allow (non_upper_case_globals) ]

//#[macro_use] extern crate serde;
//#[macro_use] extern crate serde_json;
//#[macro_use] extern crate strum_macros;

use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::sync::Arc;
//use no_deadlocks::RwLock;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::thread::{sleep, spawn};
use std::time::{Duration, SystemTime};

use grouping_by::GroupingBy;
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr};
use tauri::{AppHandle, Manager, Wry};

use windows::Win32::Foundation::{BOOL, HINSTANCE, HWND, LPARAM};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, EVENT_OBJECT_CLOAKED, EVENT_OBJECT_CREATE, EVENT_OBJECT_DESTROY, EVENT_OBJECT_FOCUS,
    EVENT_OBJECT_HIDE, EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_SHOW, EVENT_OBJECT_UNCLOAKED, EVENT_SYSTEM_FOREGROUND,
    EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND, GetMessageW, MSG
};

//use crate::*;
use crate::{win_apis, icons};
use crate::input_proc::InputProcessor;
use crate::icons::IconsManager;
use crate::config::Config;

pub type Hwnd = isize;




# [ derive (Debug, Default, Eq, PartialEq, Hash, Clone, Serialize, Deserialize) ]
pub struct ExePathName {
    pub full_path : String,
    pub name      : String
}

# [ derive (Debug, Default, Eq, PartialEq, Hash, Clone, Serialize, Deserialize) ]
pub struct WinDatEntry {
    pub hwnd              : Hwnd,
    pub win_text          : Option<String>,
    pub is_uwp_app        : Option<bool>,
    pub is_exe_queried    : bool,
    pub exe_path_name     : Option<ExePathName>,
    pub uwp_icon_path     : Option<String>,
    pub should_exclude    : Option<bool>,
    //pub icon_cache_idx    : usize,        // <- we'd rather populate this at render-list emission time
}

# [ derive (Debug, Clone, Serialize, Deserialize) ]
pub struct WinDatEntry_Pl {
    pub hwnd           : Hwnd,
    pub win_text       : Option<String>,
    pub exe_path_name  : Option<ExePathName>,
    pub icon_cache_idx : u32,
}




# [ derive (Debug, Default, Eq, PartialEq, Hash, Clone, Serialize, Deserialize) ]
pub struct IconEntry {
    pub ico_id  : usize,
    pub ico_str : String,
}




#[allow(non_camel_case_types)]
#[derive (Debug, Eq, PartialEq, Hash, Copy, Clone, AsRefStr, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend_Notice {
    hotkey_req__app_invoke,
    hotkey_req__scroll_down,
    hotkey_req__scroll_up,
    hotkey_req__scroll_end,
    hotkey_req__scroll_end_disarm,
    hotkey_req__switche_escape,
    switche_event__in_fgnd,
    switche_event__fgnd_lost,
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
pub struct RenderList_Pl {
    rl  : Vec <RenderListEntry>,
    grl : Vec <Vec <RenderListEntry>>,
}


# [ derive (Debug, Eq, PartialEq, Hash, Default, Clone, Serialize, Deserialize) ]
/// Configs-payload contains the subset of configs that is sent to the front-end
pub struct Configs_Pl {
    auto_hide_enabled      : bool,
    group_mode_enabled     : bool,
    n_grp_mode_top_recents : u32,
}
impl Configs_Pl {
    pub fn assemble (ss:&SwitcheState) -> Configs_Pl { Configs_Pl {
        auto_hide_enabled      : ss.conf.check_flag__auto_hide_enabled(),
        group_mode_enabled     : ss.conf.check_flag__group_mode_enabled(),
        n_grp_mode_top_recents : ss.conf.get_n_grp_mode_top_recents(),
    } }
}



#[allow(non_camel_case_types)]
#[derive (Debug, Eq, PartialEq, Hash, Copy, Clone, AsRefStr, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend_Event {
    backend_notice,
    updated_win_dat_entry,
    updated_render_list,
    updated_icon_entry,
    updated_configs,
}
impl Backend_Event {
    fn str (&self) -> &str { self.as_ref() }
}




#[derive (Debug, Eq, PartialEq, Hash, Clone, Serialize, Deserialize)]
pub struct FrontendRequest {
    req    : String,
    hwnd   : Option<i32>,
    params : Vec<String>,
}






# [ derive (Debug, Default, Clone) ]
/// pure sugar for representation of our atomic-bool flags
pub struct Flag (Arc <AtomicBool>);
// ^^ simple sugar that helps reduce clutter in code

impl Flag {
    pub fn new (state:bool) -> Flag { Flag ( Arc::new ( AtomicBool::new(state) ) ) }

    pub fn set   (&self) { self.0 .store (true,  Ordering::SeqCst) }
    pub fn clear (&self) { self.0 .store (false, Ordering::SeqCst) }
    pub fn store (&self, state:bool) { self.0 .store (state, Ordering::SeqCst) }

    pub fn check    (&self) -> bool { self.0 .load (Ordering::SeqCst) }
    pub fn is_set   (&self) -> bool { true  == self.0 .load (Ordering::SeqCst) }
    pub fn is_clear (&self) -> bool { false == self.0 .load (Ordering::SeqCst) }
    pub fn toggle   (&self) -> bool { ! self.0 .fetch_xor (true, Ordering::SeqCst) }
}

# [ derive ( ) ]
pub struct _SwitcheState {

    // note: should always use hwnd-map-prior as that only flips fully formed (unlike hamp-cur which might be getting slowly rebuilt)
    pub hwnd_map      : RwLock <HashMap <Hwnd, WinDatEntry>>,
    pub hwnds_ordered : RwLock <Vec <Hwnd>>,
    pub hwnds_acc     : RwLock <Vec <Hwnd>>,

    pub cur_call_id   : AtomicIsize,
    pub enum_do_light : Flag,
    pub is_dismissed  : Flag,
    pub is_fgnd       : Flag,
    pub in_alt_tab    : Flag,

    pub is_mouse_right_down       : Flag,
    pub in_right_btn_scroll_state : Flag,

    pub render_lists_m : RenderReadyListsManager,
    pub i_proc         : InputProcessor,
    pub icons_m        : IconsManager,
    pub conf           : Config,

    pub app_handle : RwLock < Option <AppHandle<Wry>>>,

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
    self_hwnd        : AtomicIsize,
    grp_sorting_map  : RwLock <HashMap <String, GroupSortingEntry>>,
    render_list      : RwLock <Vec <RenderListEntry>>,
    grpd_render_list : RwLock <Vec <Vec <RenderListEntry>>>,
    exes_excl_set    : RwLock <HashSet <String>>,
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
                hwnd_map       : RwLock::new (HashMap::new()),
                hwnds_ordered  : RwLock::new (Vec::new()),
                hwnds_acc      : RwLock::new (Vec::new()),

                cur_call_id    : AtomicIsize::default(),
                enum_do_light  : Flag::default(),
                is_dismissed   : Flag::default(),
                is_fgnd        : Flag::default(),
                in_alt_tab     : Flag::default(),

                is_mouse_right_down       : Flag::default(),
                in_right_btn_scroll_state : Flag::default(),

                render_lists_m : RenderReadyListsManager::instance(),
                i_proc         : InputProcessor::instance(),
                icons_m        : IconsManager::instance(),
                conf           : Config::instance(),

                app_handle     : RwLock::new (None),
            } ) );
            // lets do some init for the new instance
            ss.setup_win_event_hooks();
            //ss.i_proc.begin_input_processing(&ss);
            // ^^ instead, we do this everytime on reload (which front-end requests on first load too)
            ss
        } ) .clone()
    }

    pub fn register_app_handle (&self, ah:AppHandle<Wry>) {
        *self.app_handle.write().unwrap() = Some(ah);
    }
    pub fn store_self_hwnd (&self, hwnd:Hwnd) {
        self.render_lists_m.store_self_hwnd(hwnd);
    }



    /*****  win-api windows-enumeration setup and processing  ******/

    fn trigger_enum_windows_query_pending (&self, do_light:bool) {
        static trigger_pending : Lazy<Flag> = Lazy::new (|| {Flag::default()});
        // we'll first update the do_light flag whether we're ready to trigger or not
        self.enum_do_light.0 .fetch_and (do_light, Ordering::Relaxed);
        // and if we're not already pending, we'll set it up
        if !trigger_pending.is_set() {
            trigger_pending.set();
            let ss = self.clone();
            let tp = trigger_pending.clone();
            // we'll set up a delay to reduce thrashing from bunched up trains of events that the OS often sends
            let delay = if self.enum_do_light.check() {20} else {100};
            spawn ( move || {
                sleep (Duration::from_millis(delay));
                tp.clear();
                ss .trigger_enum_windows_query_immdt (ss.enum_do_light.check());
            } );
        }
    }
    fn trigger_enum_windows_query_immdt (&self, do_light:bool) {
        println!("***** starting new enum-windows query! **** (doLight:{:?})",do_light);
        self.enum_do_light.store(do_light);    // we might be getting called directly, so gotta cache it up for that
        let call_id_old = self.cur_call_id.fetch_add (1, Ordering::Relaxed);
        *self.hwnds_acc.write().unwrap() = Vec::new();
        // enum windows is blocking, and returns a bool which is false if it fails or either of its callback calls returns false
        // so we'll spawn out this call, and there, we'll wait till its done then trigger cleanup and rendering etc
        let ss = self.clone();
        spawn ( move || unsafe {
            //let t = Instant::now();
            let res = EnumWindows ( Some(Self::enum_windows_streamed_callback), LPARAM (call_id_old + 1) );
            //let dur = Instant::now().duration_since(t).as_millis();
            //println! ("enum-windows query completed in {dur} ms, with success result: {:?}", res); // --> 'light' ones now finish < 1ms
            if res.is_err() { return }    // the call could have been superceded by a newer request
            ss.post_enum_win_call_cleanup();
        } );
    }

    pub unsafe extern "system" fn enum_windows_streamed_callback (hwnd:HWND, call_id:LPARAM) -> BOOL {
        let ss = SwitcheState::instance();
        let latest_call_id = ss.cur_call_id.load(Ordering::Relaxed);
        if call_id.0 > latest_call_id {
            println! ("WARNING: got win-api callback w higher call_id than last triggered .. will restart enum-call! !");
            ss.trigger_enum_windows_query_immdt (ss.enum_do_light.check());
            return BOOL (false as i32)
        } else if call_id.0 < latest_call_id {
            // if we're still getting callbacks with stale call_id, signal that call to stop
            println! ("WARNING: got callbacks @cur-call-id {} from stale cb-id: {} .. ending it!!", latest_call_id, call_id.0);
            return BOOL (false as i32)
        };
        let passed = ss.process_discovered_hwnd (hwnd.0, ss.enum_do_light.check());
        if passed { ss.hwnds_acc .write().unwrap() .push (hwnd.0) }
        BOOL (true as i32)
    }


    fn process_discovered_hwnd (&self, hwnd:Hwnd, do_light:bool) -> bool {
        use win_apis::*;

        if do_light { // for light enum calls, passing condition is to already have excl flag set Some(false) beforehand
            return self.hwnd_map.read().unwrap() .get(&hwnd) .map (|wde| wde.should_exclude != Some(true)) .is_some();
        }

        if self.render_lists_m.check_self_hwnd (hwnd) { return false }

        if !check_window_visible  (hwnd)  { return false }
        if  check_window_cloaked  (hwnd)  { return false }

        if !check_if_app_window (hwnd) {
            if  check_window_has_owner (hwnd)  { return false }
            if  check_if_tool_window   (hwnd)  { return false }
        }

        let mut hmap = self.hwnd_map.write().unwrap();

        if !hmap.contains_key(&hwnd) {
            let mut tmp = WinDatEntry::default();
            tmp.hwnd = hwnd;
            hmap.insert (hwnd,tmp);
        }
        let wde = hmap.get_mut(&hwnd).unwrap();

        let mut should_emit = false;

        // we'll refresh the title every time this runs
        let cur_title = Some (get_window_text(wde.hwnd)) .filter (|s| !s.is_empty());
        if wde.win_text != cur_title { should_emit = true }
        wde.win_text = cur_title;

        // but only query exe-path if we havent populated it before
        if !wde.is_exe_queried {
            should_emit = true; wde.is_exe_queried = true;
            wde.exe_path_name = get_hwnd_exe_path(wde.hwnd) .and_then (|s| Self::parse_exe_path(s));
            if wde.exe_path_name .iter() .find (|ep| ep.name.as_str() == "ApplicationFrameHost.exe") .is_some() { //dbg!(hwnd);
                wde.is_uwp_app = Some(true);
                if let Some(pkg_path) = get_package_path_from_hwnd(hwnd).as_ref() { //dbg!(&pkg_path);
                    if let Some(mfp) = icons::uwp_processing::get_uwp_manifest_parse(pkg_path) {
                        wde.exe_path_name = Self::parse_exe_path(mfp.exe.to_string_lossy().into());
                        wde.uwp_icon_path = Some(mfp.ico.to_string_lossy().into());
                } }
            } else {
                wde.is_uwp_app = Some(false);
            }
        }

        let excl_flag = Some ( self.render_lists_m.calc_excl_flag (&wde) );
        if wde.should_exclude != excl_flag { should_emit = true }
        wde.should_exclude = excl_flag;

        drop(hmap); // clearing out write scope before lenghty calls

        if let Some(wde) = self.hwnd_map.read().unwrap().get(&hwnd) {
            if wde.should_exclude == Some(false) {
                self.icons_m .process_found_hwnd_exe_path (wde);
                if should_emit { self.emit_win_dat_entry (wde) }
                return true
        } }
        return false
    }


    fn parse_exe_path (exe_path:String) -> Option<ExePathName> {
        let name = exe_path .split(r"\") .last() .unwrap_or_default() .to_string();
        if name.is_empty() { None } else { Some (ExePathName { full_path: exe_path, name }) }
    }


    fn post_enum_win_call_cleanup (&self) {
        // we want to clean up both map entries and any icon-cache mappings for any hwnds that are no longer present
        self.enum_do_light.set();   // for next call .. since it needs all pending calls to specify light, its init should be light too
        let cur_hwnds_set = self.hwnds_acc.read().unwrap() .iter() .map(|h| *h) .collect::<HashSet<Hwnd>>();
        let dead_hwnds = self.hwnds_ordered.read().unwrap() .iter() .map(|h| *h)
            .filter (|hwnd| !cur_hwnds_set.contains(hwnd)) .collect::<Vec<Hwnd>>();
        dead_hwnds .iter() .for_each (|hwnd| {
            self.hwnd_map .read().unwrap()  .get(&hwnd) .map (|wde| self.icons_m.clear_dead_hwnd(wde));
            self.hwnd_map .write().unwrap() .remove(&hwnd);
        });
        // and we'll swap out the live order-list with the readied accumulator to make the new ordering current
        std::mem::swap (&mut *self.hwnds_ordered.write().unwrap(), &mut *self.hwnds_acc.write().unwrap());
        //println!("hwnds:{}, hacc:{}", self.hwnds_ordered.read().unwrap().len(), self.hwnds_acc.read().unwrap().len());

        self.emit_render_lists_queued(false);    // we'll queue it as icon upates might tack on more in a bit
    }






    /*****  some support functions  ******/


    fn handle_event__switche_fgnd (&self) {
        //println! ("switche self-fgnd report .. refreshing window-list-top icon");
        if self.is_fgnd.is_set() { return }

        self.is_dismissed.clear(); self.is_fgnd.set();
        self.emit_backend_notice (Backend_Notice::switche_event__in_fgnd);
        // the idea below is that to keep icons mostly updated, we do icon-refresh for a window when it comes to fgnd ..
        // however, when switche is brought to fgnd, recent changes in the topmost window might not be updated yet .. so we'll trigger that here
        let rleo = self .render_lists_m.render_list.read().unwrap() .first() .copied();
        rleo .map ( |rle| {
            self .hwnd_map .read().unwrap() .get(&rle.hwnd) .as_ref() .map (|wde| self.icons_m.queue_icon_refresh(wde));
            // note that this ^^ will extend read scope into icon-refresh and its children, but it avoids having to clone wde
        } );
    }
    fn handle_event__switche_fgnd_lost (&self) {
        if self.is_fgnd.is_clear() { return }

        self.is_fgnd.clear();
        if self.conf.check_flag__auto_hide_enabled() {
            self.handle_req__switche_escape()
        } else {
            //self.app_handle .read().unwrap() .as_ref() .map ( |ah| {
            //    ah .windows() .get("main") .map (|w| w.set_always_on_top (false))
            //} );
            // ^^ disabled because removeing always-on-top here seems to be too late for the window coming to fgnd ..
            // .. in theory we could then try to re-bring the 'fgnd' to fgnd, but eitherway, the topmost enablement here has minimal utility
            // .. esp since, w auto-hide-enabled all those issues are avoided anyway (by always keeping always-on-top)
        }
        self.emit_backend_notice (Backend_Notice::switche_event__fgnd_lost);
    }

    fn activate_matching_window (&self, exe:Option<&str>, title:Option<&str>) {
        let hwnd_map = self.hwnd_map.read().unwrap();
        let top2 : Vec<Hwnd> = self.render_lists_m.render_list.read().unwrap() .iter() .map ( |rle| {
            //let hwnd = rle.hwnd;
            hwnd_map .get (&rle.hwnd)
        } ) .flatten() .filter ( |wde|
            ( exe.is_none()   || exe .filter (|&p| wde.exe_path_name.as_ref().filter(|_p| _p.name.as_str() == p).is_some()).is_some() ) &&
            ( title.is_none() || title .filter (|&t| wde.win_text.as_ref().filter(|_t| _t.as_str() == t).is_some()).is_some() )
        ) .take(2) .map (|wde| wde.hwnd) .collect::<Vec<_>>();
        // if we found the hwnd, if its not already active, activate it, else switch to next window
        if top2.first().is_none() {
            // didnt find anything, so do nothing
        } else if top2.first() != self.render_lists_m.render_list.read().unwrap().first().map(|rle| rle.hwnd).as_ref() {
            // found it, and its not top, switch to it
            top2 .first() .map (|&hwnd| self.handle_req__window_activate(hwnd));
        } else if top2.get(1).is_some() && top2.get(1) != self.render_lists_m.render_list.read().unwrap().first().map(|rle| rle.hwnd).as_ref() {
            // first was already on top, but there's a second one matching, so lets switch to that (so toggling effect on matching)
            top2 .get(1) .map (|&hwnd| self.handle_req__window_activate(hwnd));
        } else {
            // found it, its already at top, and there are no other matches, so toggle to the second top window instead
            self.handle_req__second_recent_window_activate();
        }
    }







    /*****   win-api reports handling  ******/

    pub fn setup_win_event_hooks (&self) {
        /* Reference:
            pub unsafe fn SetWinEventHook (
                eventmin: u32, eventmax: u32, cb_dll: HINSTANCE, cb: WINEVENTPROC,
                idprocess: u32, idthread: u32, dwflags: u32
            ) -> HWINEVENTHOOK

            We'll put these split into separate hooks because the (system and object) events are in separate ranges
                separating them this way avoids pointless calls for events that'd end up within the wider range of a single hook
                (esp considering there are events like 0x800B that sometimes fire continuously on pointer motion!)
            That said, we can process all hook callbacks in the message loop in this thread

             System events we might be interested in:
                0x03   : EVENT_SYSTEM_FOREGROUND
                0x14   : EVENT_SYSTEM_SWITCHSTART       // alt-tab start
                0x15   : EVENT_SYSTEM_SWITCHEND         // alt-tab end
                0x16   : EVENT_SYSTEM_MINIMIZESTART
                0x17   : EVENT_SYSTEM_MINIMIZEEND

            Object events we might be interested in
                0x8000 : EVENT_OBJECT_CREATE            // maybe can skip
                0x8001 : EVENT_OBJECT_DESTROY           // maybe can skip
                0x8002 : EVENT_OBJECT_SHOW
                0x8003 : EVENT_OBJECT_HIDE
                0x8005 : EVENT_OBJECT_FOCUS
                0x800B : EVENT_OBJECT_LOCATIONCHANGE    // this can fire continuously on mouse motion!
                0x800C : EVENT_OBJECT_NAMECHANGE
                0x8017 : EVENT_OBJECT_CLOAKED
                0x8018 : EVENT_OBJECT_UNCLOAKED

            However, listening to a subset might be enough as events often gen in sets (e.g fgnd then focus etc)
         */

        spawn ( move || unsafe {
            SetWinEventHook( 0x0003, 0x0003, HINSTANCE::default(), Some(Self::win_event_hook_cb), 0, 0, 0);
            SetWinEventHook( 0x0016, 0x0017, HINSTANCE::default(), Some(Self::win_event_hook_cb), 0, 0, 0);

            SetWinEventHook( 0x8000, 0x8005, HINSTANCE::default(), Some(Self::win_event_hook_cb), 0, 0, 0);
            SetWinEventHook( 0x800C, 0x800C, HINSTANCE::default(), Some(Self::win_event_hook_cb), 0, 0, 0);
            SetWinEventHook( 0x8017, 0x8018, HINSTANCE::default(), Some(Self::win_event_hook_cb), 0, 0, 0);

            // win32 sends hook events to a thread with a 'message loop', but we wont create any windows here to get window messages,
            //     so we'll just leave a forever waiting GetMessage instead of setting up a msg-loop
            // .. basically while its waiting, the thread is awakened simply to call kbd hook (for an actual msg, itd awaken give the msg)
            let mut msg: MSG = MSG::default();
            while BOOL(0) != GetMessageW (&mut msg, HWND(0), 0, 0) { };
        } );
    }


    pub unsafe extern "system" fn win_event_hook_cb (
        _id_hook: HWINEVENTHOOK, event: u32, hwnd: HWND,
        id_object: i32, id_child: i32, _id_thread: u32, _event_time: u32
    ) {
        if id_object == 0 && id_child == 0 {
            //let t = std::time::UNIX_EPOCH.elapsed().unwrap().as_millis();
            //println!("--> {:16} : hook event: 0x{:X}, hwnd:{:?}, id_object: 0x{:4X}", t, event, hwnd, id_object);
            // todo: prob need to figure out actual logging w debug/run switches .. theres samples incl in the other repo
            match event {
                //
                EVENT_SYSTEM_FOREGROUND    =>  SwitcheState::instance().proc_win_report__fgnd_hwnd     (hwnd.0),
                EVENT_SYSTEM_MINIMIZESTART =>  SwitcheState::instance().proc_win_report__minimized     (hwnd.0),
                EVENT_SYSTEM_MINIMIZEEND   =>  SwitcheState::instance().proc_win_report__minimize_end  (hwnd.0),
                //
                EVENT_OBJECT_CREATE       =>  SwitcheState::instance().proc_win_report__obj_shown      (hwnd.0),
                EVENT_OBJECT_DESTROY      =>  SwitcheState::instance().proc_win_report__obj_destroyed  (hwnd.0),
                EVENT_OBJECT_SHOW         =>  SwitcheState::instance().proc_win_report__obj_shown      (hwnd.0),
                EVENT_OBJECT_HIDE         =>  SwitcheState::instance().proc_win_report__obj_destroyed  (hwnd.0),
                EVENT_OBJECT_FOCUS        =>  SwitcheState::instance().proc_win_report__fgnd_hwnd      (hwnd.0),
                EVENT_OBJECT_NAMECHANGE   =>  SwitcheState::instance().proc_win_report__title_changed  (hwnd.0),
                EVENT_OBJECT_CLOAKED      =>  SwitcheState::instance().proc_win_report__obj_destroyed  (hwnd.0),
                EVENT_OBJECT_UNCLOAKED    =>  SwitcheState::instance().proc_win_report__obj_shown      (hwnd.0),
                //
                _ => { }
            }
        }
    }

    pub fn _stamp (&self) -> u128 { SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis() }

    pub fn proc_win_report__title_changed (&self, hwnd:Hwnd) {
        //println! ("@{:?} title-changed: {:?}", self._stamp(), hwnd);
        // first off, if its for something not in renderlist, just ignore it
        if self.hwnd_map.read().unwrap() .get(&hwnd) .filter (|wde| wde.should_exclude == Some(false)) .is_none() { return }
        // things like this IDE seem to give piles of window-level title-change events just while typing, w/o title change .. so filter them
        if let Some(wde) = self.hwnd_map.read().unwrap().get(&hwnd) {
            if wde.win_text.as_ref() == Some(& win_apis::get_window_text(hwnd)) {
                return
        } }
        // we'll fully update this hwnd, but only queue up a light enum-query (only ordering update, no data updates)
        self.hwnd_map.read().unwrap() .get(&hwnd) .map (|wde| self.icons_m.mark_cached_icon_mapping_stale(wde));
        // calling processing will get this hwnd updated, incl updating icons if marked stale (which happens off-thread)
        if self.process_discovered_hwnd (hwnd, false) {
            self.emit_render_lists_queued (true);
        }
    }

    pub fn check_owner_chain_in_render_list (&self, hwnd:Hwnd) -> bool {
        if self.process_discovered_hwnd(hwnd, true) { return true }
        let owner_hwnd = win_apis::get_window_owner(hwnd);
        //println! ("owner-chain: {:?} -> {:?}", hwnd, owner_hwnd);
        if owner_hwnd == 0 || owner_hwnd == hwnd { return false }
        self.check_owner_chain_in_render_list (owner_hwnd)
    }
    pub fn check_parent_chain_in_render_list (&self, hwnd:Hwnd) -> bool {
        if self.process_discovered_hwnd(hwnd, true) { return true }
        let parent_hwnd = win_apis::get_window_parent(hwnd);
        //println! ("parent-chain: {:?} -> {:?}", hwnd, owner_hwnd);
        if parent_hwnd == 0 || parent_hwnd == hwnd { return false }
        self.check_parent_chain_in_render_list (parent_hwnd)
    }
    pub fn proc_win_report__fgnd_hwnd (&self, hwnd:Hwnd) {
        println! ("@{:?} fgnd: {:?}", self._stamp(), hwnd);
        // we'll set its icon to be refreshed, then process it, and queue up a 'light' (ordering only) enum-windows call
        // plus, we'll update self-fgnd state if either this is self hwnd, or if its a valid renderable hwnd coming to fgnd
        self.hwnd_map.read().unwrap() .get(&hwnd) .map (|wde| self.icons_m.mark_cached_icon_mapping_stale(wde));
        if self.render_lists_m.check_self_hwnd(hwnd) {
            self.handle_event__switche_fgnd();
        //} else if self.process_discovered_hwnd (hwnd, false) {
        //} else if self.process_discovered_hwnd (hwnd, false) || self.check_owner_chain_in_render_list(hwnd) {
        //} else if self.process_discovered_hwnd (hwnd, false) || self.check_parent_chain_in_render_list(hwnd) {
        } else {
            // ^^ we want to catch all z-order changes, it seems only possible if we check (light) everytime a fgnd report comes in
            self.trigger_enum_windows_query_pending(true);
            if self.is_fgnd.is_set() { self.handle_event__switche_fgnd_lost() }
        }
    }
    pub fn proc_win_report__minimize_end (&self, hwnd:Hwnd) {
        println! ("@{:?} minimize-ended: {:?}", self._stamp(), hwnd);
        // ehh, we can just treat this as a fgnd report (other than the printout above for identification)
        self.proc_win_report__fgnd_hwnd(hwnd);
    }

    pub fn proc_win_report__minimized (&self, hwnd:Hwnd) {
        println! ("@{:?} minimized: {:?}", self._stamp(), hwnd);
        // we only really want to query/update z-order here if this was in our windows list
        if self.process_discovered_hwnd (hwnd, false) {
            self.trigger_enum_windows_query_pending(true);
        }
    }

    pub fn proc_win_report__obj_shown (&self, hwnd:Hwnd) {
        //println! ("@{:?} obj-shown: {:?}", self._stamp(), hwnd);

        // windows can get into nothing-in-fgnd state, and in such cases, if the new fgnd is the same as what was fgnd last ..
        // .. then it will not send a new fgnd report .. if this happens to switche, our is_fgnd flags get out of sync ..
        // .. so we'll handle this here directly .. and in fgnd report we'll ignore if our is_fgnd flag is already set
        if self.render_lists_m.check_self_hwnd(hwnd) {
            self.handle_event__switche_fgnd();
            return
        }
        fn setup_backed_off_icon_requeries (hwnd:Hwnd, ss:SwitcheState) {
            println!("## triggering backed-off icon requeries for {:?}", hwnd);
            // for first created/shown/uncloaked valid windows, we'll set up exponentially backing off icon requeries ..
            // (.. since things like chrome apps seem to take many seconds at times to put up real icons instead of just placeholders)
            spawn ( move || {
                for i in 1..10 {
                    sleep (Duration::from_millis (200 * i*i));      // quadratic backoff .. 6 backoffs accumulates to x100, 11 to x500
                    ss.hwnd_map.read().unwrap().get(&hwnd) .map (|wde| ss.icons_m.queue_icon_refresh(wde));
                }
            } );
        }
        // now before we reprocess, if this was in our list, we'll prime it to have icon refreshed upon processing
        self.hwnd_map.read().unwrap() .get(&hwnd) .map (|wde| self.icons_m.mark_cached_icon_mapping_stale(wde));

        // now we can do the initial processing attempt and set up the requeries
        if self.process_discovered_hwnd(hwnd, false) {
            self.trigger_enum_windows_query_pending(true);
            setup_backed_off_icon_requeries (hwnd, self.clone());
        }
    }

    pub fn proc_win_report__obj_destroyed (&self, hwnd:Hwnd) {
        //println! ("@{:?} obj-destroyed: {:?}", self._stamp(), hwnd);

        // this is counterpart to special swi handling in obj-shown
        if self.render_lists_m.check_self_hwnd(hwnd) {
            self.handle_event__switche_fgnd_lost();
            return
        }

        // if we werent even showing this object, we're done, else queue up a enum-trigger
        if self.hwnd_map.read().unwrap() .get(&hwnd) .filter (|wde| wde.should_exclude == Some(false)) .is_some() {
            // its in our maps, so lets process it, but if processing now rejects it, we should remove it from map
            if !self.process_discovered_hwnd (hwnd, false) { self.hwnd_map.write().unwrap().remove(&hwnd); }
            // and a 'light' enum call should take care of ordering changes (whether it is now passing or not)
            self.trigger_enum_windows_query_pending(true);
        } // else the destoryed hwnd wasnt even in our list .. we can ignore it
    }















    /*****   Front-End Requests handling  ******/

    // note that in prior incarnations, these were somewhat unreliable and we needed repeated spaced out attempts ..
    // .. however, in this impl, so far things seem to work pretty consistently and without need for delays ..

    fn handle_req__window_activate (&self, hwnd:Hwnd) {
        // this call is only for non-self windows .. we'll want to dimiss ourselves
        let ss = self.clone();
        spawn ( move || {
            win_apis::window_activate(hwnd);
            ss.handle_req__switche_dismiss();
        } );
    }
    fn handle_req__window_peek (&self, hwnd:Hwnd) {
        let self_hwnd = self.render_lists_m.self_hwnd();
        spawn ( move || {
            win_apis::window_activate(hwnd);
            // after 'showing'some window for a bit, we'll bring back ourselves
            // the preview duration for this could prob be made configurable
            sleep (Duration::from_millis(1000));
            win_apis::window_activate(self_hwnd);
        } );
    }
    fn handle_req__window_minimize (&self, hwnd:Hwnd) {
        spawn ( move || { win_apis::window_minimize(hwnd) } );
    }
    fn handle_req__window_maximize (&self, hwnd:Hwnd) {
        spawn ( move || { win_apis::window_maximize(hwnd) } );
    }
    fn handle_req__window_close (&self, hwnd:Hwnd) {
        spawn ( move || { win_apis::window_close(hwnd) } );
    }

    fn self_window_activate (&self) {
        self.is_dismissed.clear();
        self.app_handle .read().unwrap() .as_ref() .map ( |ah| {
            ah .windows() .get("main") .map (|w| {
                let (_, _) = ( w.show(), w.set_focus() );
                //let (_, _, _) = ( w.show(), w.set_focus(), w.set_always_on_top(true) );
                // ^^ disabling setting always-on-top since it doesnt play too well when auto-hide is disabled ..
                // (instead, we will always set always-on-top when auto-hide is enabled)
            } );
        } );
    }
    fn self_window_hide (&self) {
        self.is_dismissed.set();
        self.app_handle .read().unwrap() .as_ref() .map ( |ah| {
            ah .windows() .get("main") .map (|w| w.hide() )
        } );
    }

    fn handle_req__nth_recent_window_activate (&self, n:usize) {
        let hwnd = self.render_lists_m.render_list.read().unwrap() .get(n) .map (|e| e.hwnd);
        spawn ( move || {
            hwnd .map ( |hwnd| win_apis::window_activate(hwnd) );
        } );
    }


    fn handle_req__switche_dismiss (&self) {
        // this is called after some window-activation
        self.self_window_hide();
        self.trigger_enum_windows_query_immdt(true);
        // ^^ we'll do an immdt light query so we'll have the latest ordering when we come back
    }
    fn handle_req__switche_escape (&self) {
        // this is called specifically upon escape from switche
        self.self_window_hide();
        //self.handle_req__nth_recent_window_activate(0);
        // ^^ should we reactivate last active window before we dismiss .. nah .. should rather maintain 'least-surprise'
        self.trigger_enum_windows_query_pending(false);
        // ^^ this is also a good time to do a full query to keep things in sync if any weird events etc have fallen through the cracks
    }
    fn handle_req__switche_quit (&self) {
        self.app_handle.read().unwrap().as_ref() .map (|ah| ah.exit(0));
    }
    fn handle_req__self_auto_resize (&self) {
        crate::tauri::auto_setup_self_window (self.render_lists_m.self_hwnd.load(Ordering::Relaxed));
    }
    fn handle_req__second_recent_window_activate (&self) {
        self.handle_req__nth_recent_window_activate(1);
    }
    fn handle_req__debug_print (&self) { }


    pub(crate) fn handle_req__data_load(&self) {
        // ^ this triggers on reload .. we'll use that to refresh our hooks, configs etc too
        self.conf.load();
        self.render_lists_m.reload_exes_excl_set(self.conf.get_exe_exclusions_list());
        self.i_proc.re_set_hooks();
        // first we'll send what data we have
        self.emit_configs();
        self.icons_m.emit_all_icon_entries();
        self.render_lists_m.render_list.read().unwrap() .iter() .for_each ( |rle| {
            self.hwnd_map.read().unwrap() .get (&rle.hwnd) .iter() .for_each ( |wde| {
                self.emit_win_dat_entry (wde)
        }) });
        self.emit_render_lists_queued(true);
        // then we'll trigger a refresh too
        self.render_lists_m.clear_grouping();
        self.icons_m.mark_all_cached_icon_mappings_stale();
        self.trigger_enum_windows_query_pending(false);    // this will also trigger a renderlist push once the call is done
    }
    fn handle_req__refresh (&self) {
        self.icons_m.mark_all_cached_icon_mappings_stale();
        self.trigger_enum_windows_query_pending(false)
    }



    pub fn handle_frontend_request (&self, r:&FrontendRequest) {
        println! ("received {:?}", r);

        match r.req.as_str() {
            "fe_req_window_activate"      => { r.hwnd .map (|h| self.handle_req__window_activate (h as Hwnd) ); }
            "fe_req_window_peek"          => { r.hwnd .map (|h| self.handle_req__window_peek     (h as Hwnd) ); }
            "fe_req_window_minimize"      => { r.hwnd .map (|h| self.handle_req__window_minimize (h as Hwnd) ); }
            "fe_req_window_maximize"      => { r.hwnd .map (|h| self.handle_req__window_maximize (h as Hwnd) ); }
            "fe_req_window_close"         => { r.hwnd .map (|h| self.handle_req__window_close    (h as Hwnd) ); }

            "fe_req_data_load"            => { self.handle_req__data_load()        }
            "fe_req_refresh"              => { self.handle_req__refresh()          }
            "fe_req_switche_escape"       => { self.handle_req__switche_escape()   }
            "fe_req_switche_quit"         => { self.handle_req__switche_quit()     }
            "fe_req_self_auto_resize"     => { self.handle_req__self_auto_resize() }
            "fe_req_debug_print"          => { self.handle_req__debug_print()      }

            "fe_req_edit_config"          => { self.conf.trigger_config_file_edit() }
            "fe_req_grp_mode_enable"      => { self.conf.deferred_update_conf__grp_mode (true)  }
            "fe_req_grp_mode_disable"     => { self.conf.deferred_update_conf__grp_mode (false) }

            _ => { println! ("unrecognized frontend cmd: {}", r.req) }
        }
    }

    pub fn setup_front_end_listener (&self, ah:&AppHandle<Wry>) {
        let ss = self.clone();
        let _ = ah .listen_global ( "frontend_request", move |event| {
            //println!("got event with raw payload {:?}", &event.payload());
            event .payload() .map ( |ev| serde_json::from_str::<FrontendRequest>(ev).ok() )
                .flatten() .iter() .for_each (|ev| ss.handle_frontend_request(ev))
        } );
    }









    /*****  tauri registered hotkeys and app event handling   ******/

    pub fn proc_app_window_event__focus (&self) {
        //if self.is_dismissed.is_set() || self.is_fgnd.is_clear() { self.handle_event__switche_fgnd() }
        // ^^ first off this is unnecessary given our win-event fgnd notice ..
        //  .. plus it will race or get out-of-sync w the actual fgnd notice w/o eqv focus_lost handling, which too would be pointless
    }
    pub fn proc_app_window_event__focus_lost (&self) {
        // nothing really .. this doesnt even count as dismissed (which triggers list-cur-elem reset etc)
    }

    pub fn checked_self_activate (&self) {
        if self.is_dismissed.is_set() || self.is_fgnd.is_clear() || self.render_lists_m.self_hwnd() != win_apis::get_fgnd_window() {
            self.self_window_activate();
            //self.handle_event__switche_fgnd();
            // ^^ this should trigger from win-event fgnd report anyway .. and calling this triggers icon refresh, so we'll let it happen then
        }
    }
    pub fn proc_hot_key__invoke (&self) {
        // we'll ensure the app window is up, then let the frontend deal w it
        self.checked_self_activate();
        self.emit_backend_notice(Backend_Notice::hotkey_req__app_invoke)
    }
    pub fn proc_hot_key__scroll_down (&self) {
        self.checked_self_activate();
        self.emit_backend_notice (Backend_Notice::hotkey_req__scroll_down);
    }
    pub fn proc_hot_key__scroll_up (&self) {
        self.checked_self_activate();
        self.emit_backend_notice (Backend_Notice::hotkey_req__scroll_up);
    }
    pub fn proc_hot_key__scroll_end (&self) {
        if self.is_dismissed.is_clear() && self.is_fgnd.is_set() {
            self.emit_backend_notice (Backend_Notice::hotkey_req__scroll_end)
        }
    }
    pub fn proc_hot_key__scroll_end_disarm (&self) {
        if self.is_dismissed.is_clear() && self.is_fgnd.is_set() {
            self.emit_backend_notice (Backend_Notice::hotkey_req__scroll_end_disarm)
        }
    }
    pub fn proc_hot_key__switche_escape (&self) {
        self.handle_req__switche_escape();
        self.emit_backend_notice (Backend_Notice::hotkey_req__switche_escape)
    }

    pub fn proc_hot_key__switch_last (&self)  {
        self.handle_req__second_recent_window_activate()
    }
    pub fn proc_hot_key__switch_app (&self, exe:Option<&str>, title:Option<&str>) {
        self.activate_matching_window (exe, title)
    }








    /*****    emitting backend messages   ******/


    pub fn emit_win_dat_entry (&self, wde:&WinDatEntry) {
        //println! ("emitting **{notice}** win_dat_entry for: {:?}", wde.hwnd);
        let pl = WinDatEntry_Pl {
                hwnd: wde.hwnd,
                win_text: wde.win_text.clone(),
                exe_path_name: wde.exe_path_name.clone(),
                icon_cache_idx: self.icons_m.get_cached_icon_idx(wde).unwrap_or(0) as u32,
        };
        //println! ("** wde-update for hwnd {:8} : ico-idx: {:?}, {:?}", pl.hwnd, pl.icon_cache_idx, pl.exe_path_name.as_ref().map(|p|p.name.clone()));
        self.app_handle.read().unwrap() .iter().for_each ( |ah| {
            serde_json::to_string(&pl) .map ( |pl| {
                ah.emit_all::<String> (Backend_Event::updated_win_dat_entry.str(), pl )
            } ) .err() .iter() .for_each (|err| println!{"win_dat emit failed: {:?}", err});
        } );
    }

    pub fn emit_icon_entry (&self, ie:&IconEntry) {
        //println! ("** icon-update for icon-id: {:?}", ie.ico_id);
        self.app_handle .read().unwrap() .iter() .for_each ( |ah| {
            serde_json::to_string(ie) .map (|pl| {
                ah.emit_all::<String> ( Backend_Event::updated_icon_entry.str(), pl )
            } ) .err() .iter().for_each (|err| println!("icon-entry emit failed: {:?}", err));
        } );
    }

    pub fn emit_configs (&self) {
        println! ("** emitting .. configs-update");
        self.app_handle .read().unwrap() .iter() .for_each ( |ah| {
            serde_json::to_string (&Configs_Pl::assemble(&self)) .map (|pl| {
                ah.emit_all::<String> ( Backend_Event::updated_configs.str(), pl )
            } ) .err() .iter() .for_each (|err| println!("configs emit failed: {:?}", err));
        } );
    }


    pub fn emit_backend_notice (&self, notice: Backend_Notice) {
        let pl = BackendNotice_Pl { msg: notice.str().to_string() };
        self.app_handle.read().unwrap() .iter() .for_each ( |ah| {
            serde_json::to_string(&pl) .map ( |pl| {
                println!("sending backend notice: {}", &pl);
                ah.emit_all::<String> ( Backend_Event::backend_notice.str(), pl ) .or_else(|_| Err("emit failure"))
            } ) .or_else (|err| { Err(err) }) .err() .iter().for_each (|err| println!("render-list emit failed: {}", err));
        } )
    }


    fn emit_render_lists (&self) {
        let rlp = RenderList_Pl {
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

    pub fn emit_render_lists_immdt (&self, force:bool) {
        // at reload etc on a data-load req we want to forcibly emit renderlist even if it hasnt changed
        if force {
            self.render_lists_m.update_render_ready_lists(self);
            self.emit_render_lists();
            return
        }
        // if not forced, then we'll do a diff and only emit on change
        let rl_cf  = self.render_lists_m.render_list.read().unwrap().clone();
        let grl_cf = self.render_lists_m.grpd_render_list.read().unwrap().clone();

        self.render_lists_m.update_render_ready_lists(self);
        //println!("rl;{:?}",self.render_lists_m.render_list.read().unwrap());

        if rl_cf != *self.render_lists_m.render_list.read().unwrap() ||
            grl_cf != *self.render_lists_m.grpd_render_list.read().unwrap()
        { self.emit_render_lists() }
    }

    pub fn emit_render_lists_queued (&self, force:bool) {
        static do_forced      : Lazy<Flag> = Lazy::new (|| {Flag::default()});
        static render_pending : Lazy<Flag> = Lazy::new (|| {Flag::default()});
        if force { do_forced.set() }
        // note that we dont want to keep pushing this out while there are updates ..
        // (we just want to bunch up some but keep pushing when/if there's a stream of updates)
        if render_pending.is_clear() {
            render_pending.set();
            let ss = self.clone();
            spawn ( move || {
                sleep (Duration::from_millis(100));
                render_pending.clear();
                ss.emit_render_lists_immdt(do_forced.check());
                do_forced.clear();
            } );
        }
    }



}










impl RenderReadyListsManager {

    pub fn instance () -> RenderReadyListsManager {
        static INSTANCE: OnceCell <RenderReadyListsManager> = OnceCell::new();
        INSTANCE .get_or_init ( ||
            RenderReadyListsManager ( Arc::new ( _RenderReadyListsManager {
                self_hwnd         : AtomicIsize::default(),
                grp_sorting_map   : RwLock::new(Default::default()),
                render_list       : RwLock::new(Default::default()),
                grpd_render_list  : RwLock::new(Default::default()),
                exes_excl_set     : RwLock::new(Default::default()),
            } ) )
        ) .clone()
    }

    // --- rendering exclusions ---

    pub fn store_self_hwnd (&self, hwnd:Hwnd) { self.self_hwnd.store (hwnd, Ordering::Relaxed) }
    pub fn self_hwnd (&self) -> Hwnd { self.self_hwnd.load(Ordering::Relaxed) }
    pub fn check_self_hwnd (&self, hwnd:Hwnd) -> bool { hwnd == self.self_hwnd() }

    pub fn calc_excl_flag (&self, wde:&WinDatEntry) -> bool {
        //wde.is_vis == Some(false) ||  wde.is_uncloaked == Some(false) ||    // already covered during enum-filtering
        self.check_self_hwnd(wde.hwnd) || wde.win_text.is_none() ||
            wde.exe_path_name.as_ref() .filter (|p| !p.full_path.is_empty()) .is_none() ||
            wde.exe_path_name.as_ref() .filter (|p| !self.exes_excl_check(&p.name)) .is_none()
    }
    pub fn runtime_should_excl_check (&self, wde:&WinDatEntry) -> bool {
        wde.should_exclude .unwrap_or_else ( move || self.calc_excl_flag(wde) )
    }
    pub fn reload_exes_excl_set (&self, exes:Vec<String>) {
        let mut ees = self.exes_excl_set.write().unwrap();
        ees.clear();
        exes.into_iter() .for_each (|e| { ees.insert(e); });
    }
    fn exes_excl_check (&self, estr:&str) -> bool {
        self.exes_excl_set.read().unwrap().contains(estr)
    }





    // --- groups ordering registry ----

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

    fn clear_grouping (&self) {
        self.grp_sorting_map.write().unwrap().clear()
    }



    // --- render lists calculations ---

    fn grl_cmp_ext (&self, po:&Option<&String> ) -> Option<f32> {
        po.as_ref() .map (|&p| self.grp_sorting_map.read().unwrap().get(p).copied()) .flatten() .map (|gse| gse.mean_perc_idx)
    }

    fn recalc_render_ready_lists (&self, ss:&SwitcheState) -> (Vec<RenderListEntry>, Vec<Vec<RenderListEntry>>) {
        let is_dismissed = ss.is_dismissed.check();     // local copy to avoid guarded accesses in a loop
        let hwnds = ss.hwnds_ordered.read().unwrap();
        let hwnd_map = ss.hwnd_map.read().unwrap();
        let filt_wdes = hwnds .iter() .flat_map (|h| hwnd_map.get(h))
            .filter (|&wde| !self.runtime_should_excl_check(wde)) .collect::<Vec<&_>>();
        let filt_rlp = filt_wdes .iter() .enumerate() .map ( |(i,wde)| {
            // we'll also register these while we're creating the RLEs
            let fp = wde.exe_path_name .as_ref() .map (|p| &p.full_path);
            fp .iter() .for_each ( |fp| {
                self.register_entry (fp, i, filt_wdes.len() as u32, is_dismissed)
            } );
            (fp, RenderListEntry { hwnd: wde.hwnd, y: 1+i as u32 })
            // ^^ note that renderlist entries are 1 based corresponding to how we want them shown in the UI
            // (and we calc and send from here instead of just wde vecs as grpd-render-list still should use recents-ordered/sorted ids!)
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
        //println!("render-ready-list recalc -- rl:{:?}", filt_rl.len() ); println!();
        ( filt_rl, grpd_render_list )
    }

    fn update_render_ready_lists (&self, ss:&SwitcheState) {
        let (rl, grl) = self.recalc_render_ready_lists (ss);
        *self.render_list.write().unwrap() = rl;
        *self.grpd_render_list.write().unwrap() = grl;
    }


}






