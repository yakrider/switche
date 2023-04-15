#![ allow (non_camel_case_types) ]
#![ allow (non_snake_case) ]
#![ allow (non_upper_case_globals) ]

//#[macro_use] extern crate serde;
//#[macro_use] extern crate serde_json;
//#[macro_use] extern crate strum_macros;

use std::collections::{HashMap, HashSet};
use std::ops::{Deref};
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
use tauri::{AppHandle, Manager, RunEvent, SystemTrayEvent, WindowEvent, Wry};

use windows::Win32::Foundation::{BOOL, HINSTANCE, HWND, LPARAM};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, EVENT_OBJECT_CLOAKED, EVENT_OBJECT_CREATE, EVENT_OBJECT_DESTROY, EVENT_OBJECT_FOCUS,
    EVENT_OBJECT_HIDE, EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_SHOW, EVENT_OBJECT_UNCLOAKED,
    EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND, GetMessageW, MSG};

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
    pub win_text          : Option<String>,
    pub exe_path_name     : Option<ExePathName>,
    pub should_exclude    : Option<bool>,
    pub icon_override_loc : Option<Option<String>>,   // the actual value is an option, the wrapping option indicates whether initialized
    //pub icon_cache_idx    : usize,        // <- we'd rather populate this at render-list emission time
}

# [ derive (Debug, Clone, Serialize, Deserialize) ]
pub struct WinDatEntry_FE {
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
    pub fn new (state:bool) -> Flag { Flag ( Arc::new ( AtomicBool::new(state) ) ) }

    pub fn set   (&self) { self.0 .store (true,  Ordering::SeqCst) }
    pub fn clear (&self) { self.0 .store (false, Ordering::SeqCst) }
    pub fn store (&self, state:bool) { self.0 .store (state, Ordering::SeqCst) }

    pub fn check    (&self) -> bool { self.0 .load (Ordering::SeqCst) }
    pub fn is_set   (&self) -> bool { true  == self.0 .load (Ordering::SeqCst) }
    pub fn is_clear (&self) -> bool { false == self.0 .load (Ordering::SeqCst) }
}

# [ derive ( ) ]
pub struct _SwitcheState {

    // note: should always use hwnd-map-prior as that only flips fully formed (unlike hamp-cur which might be getting slowly rebuilt)
    pub hwnd_map      : Arc <RwLock <HashMap <Hwnd, WinDatEntry>>>,
    pub hwnds_ordered : Arc <RwLock <Vec <Hwnd>>>,
    pub hwnds_acc     : Arc <RwLock <Vec <Hwnd>>>,

    pub cur_call_id   : AtomicIsize,
    pub enum_do_light : Flag,
    pub is_dismissed  : Flag,

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
                hwnd_map       : Arc::new ( RwLock::new (HashMap::new())),
                hwnds_ordered  : Arc::new ( RwLock::new (Vec::new())),
                hwnds_acc      : Arc::new ( RwLock::new (Vec::new())),

                cur_call_id    : AtomicIsize::default(),
                enum_do_light  : Flag::default(),
                is_dismissed   : Flag::default(),

                render_lists_m : RenderReadyListsManager::instance(),
                icons_m        : IconsManager::instance(),
                excl_m         : ExclusionsManager::instance(),

                app_handle     : Arc::new ( RwLock::new (None)),
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
            if res == false { return }    // the call could have been superceded by a newer request
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

        if !check_window_visible   (hwnd)  { return false }
        if  check_window_cloaked   (hwnd)  { return false }
        if  check_window_has_owner (hwnd)  { return false }
        if  check_if_tool_window   (hwnd)  { return false }

        if self.excl_m.check_self_hwnd (hwnd) { return false }

        let mut hmap = self.hwnd_map.write().unwrap();

        if !hmap.contains_key(&hwnd) {
            let mut tmp = WinDatEntry::default();
            tmp.hwnd = hwnd;
            hmap.insert (hwnd,tmp);
        }
        let mut wde = hmap.get_mut(&hwnd).unwrap();

        let mut should_emit = false;

        // we'll refresh the title every time this runs
        let cur_title = Some (get_window_text(wde.hwnd)) .filter (|s| !s.is_empty());
        if wde.win_text != cur_title { should_emit = true }
        wde.win_text = cur_title;

        // but only query exe-path if we havent populated it before
        if wde.exe_path_name.is_none() {
            should_emit = true;
            wde.exe_path_name = Self::get_parsed_exe_path_name(wde.hwnd);
        }

        let excl_flag = Some ( self.excl_m.calc_excl_flag (&wde) );
        if wde.should_exclude != excl_flag { should_emit = true }
        wde.should_exclude = excl_flag;

        if wde.should_exclude == Some(false) {
            if wde.icon_override_loc.is_none() {
                let ov_loc = Some ( IconPathOverridesManager::get_icon_override_path(&wde) );
                if wde.icon_override_loc != ov_loc { should_emit = true }
                wde.icon_override_loc = ov_loc;
            }
        }
        drop(hmap); // clearing out write scope before lenghty calls

        if let Some(wde) = self.hwnd_map.read().unwrap().get(&hwnd) {
            if wde.should_exclude == Some(false) {
                self.icons_m .process_found_hwnd_exe_path (wde);
                if should_emit { self.emit_win_dat_entry (wde) }
                return true
        } }
        return false
    }


    fn get_parsed_exe_path_name(hwnd:Hwnd) -> Option<ExePathName> {
        let exe_path = win_apis::get_exe_path_name(hwnd);
        exe_path .map ( |fp| {
            let name = fp .split(r"\") .last() .unwrap_or_default() .to_string();
            ExePathName { full_path: fp, name }
        } )
    }


    fn post_enum_win_call_cleanup (&self) {
        // we want to clean up both map entries and any icon-cache mappings for any hwnds that are no longer present
        self.enum_do_light.set();   // for next call .. since it needs all pending calls to specify light, its init should be light too
        let cur_hwnds_set = self.hwnds_acc.read().unwrap() .iter() .map(|h| *h) .collect::<HashSet<Hwnd>>();
        let dead_hwnds = self.hwnds_ordered.read().unwrap() .iter() .map(|h| *h)
            .filter (|hwnd| !cur_hwnds_set.contains(hwnd)) .collect::<Vec<Hwnd>>();
        let icm = IconsManager::instance();
        dead_hwnds .iter() .for_each (|hwnd| {
            self.hwnd_map .read().unwrap()  .get(&hwnd) .map (|wde| icm.clear_dead_hwnd(wde));
            self.hwnd_map .write().unwrap() .remove(&hwnd);
        });
        // and we'll swap out the live order-list with the readied accumulator to make the new ordering current
        std::mem::swap (&mut *self.hwnds_ordered.write().unwrap(), &mut *self.hwnds_acc.write().unwrap());
        println!("hwnds:{}, hacc:{}", self.hwnds_ordered.read().unwrap().len(), self.hwnds_acc.read().unwrap().len());

        self.emit_render_lists_queued(false);    // we'll queue it as icon upates might tack on more in a bit
    }






    /*****  some support functions  ******/

    pub fn get_self_hwnd(&self) -> Option<Hwnd> {
        self.app_handle.read().unwrap().as_ref() .iter() .map (|ah| {
            ah.windows().values() .next() .map (|w| w.hwnd().ok()) .flatten()
        } ) .flatten() .next() .map (|h| h.0)
    }

    fn handle_event__switche_fgnd (&self) {
        println! ("switche self-fgnd report .. refreshing window-list-top icon");
        self.is_dismissed.clear();
        // the idea below is that to keep icons mostly updated, we do icon-refresh for a window when it comes to fgnd ..
        // however, when switche is brought to fgnd, recent changes in the topmost window might not be updated yet .. so we'll trigger that here
        let rleo = self .render_lists_m.render_list.read().unwrap() .first() .copied();
        rleo .map ( |rle| {
            self .hwnd_map .read().unwrap() .get(&rle.hwnd) .as_ref() .map (|wde| self.icons_m.queue_icon_refresh(wde));
            // note that this ^^ will extend read scope into icon-refresh and its children, but it avoids having to clone wde
        } );
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
            SetWinEventHook( 0x0017, 0x0017, HINSTANCE::default(), Some(Self::win_event_hook_cb), 0, 0, 0);

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
                EVENT_SYSTEM_FOREGROUND   =>  SwitcheState::instance().proc_win_report__fgnd_hwnd      (hwnd.0),
                EVENT_SYSTEM_MINIMIZEEND  =>  SwitcheState::instance().proc_win_report__fgnd_hwnd      (hwnd.0),
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

    pub fn proc_win_report__fgnd_hwnd (&self, hwnd:Hwnd) {
        //println!("@{:?} fgnd: {:?}",self._stamp(),hwnd);
        // we'll set its icon to be refreshed, then process it, and queue up a 'light' (ordering only) enum-windows call
        self.hwnd_map.read().unwrap() .get(&hwnd) .map (|wde| self.icons_m.mark_cached_icon_mapping_stale(wde));
        if self.process_discovered_hwnd (hwnd, false) {
            self.trigger_enum_windows_query_pending(true);
        }
    }

    pub fn proc_win_report__obj_shown (&self, hwnd:Hwnd) {
        //println!("@{:?} obj-shown: {:?}",self._stamp(),hwnd);
        self.hwnd_map.read().unwrap() .get(&hwnd) .map (|wde| self.icons_m.mark_cached_icon_mapping_stale(wde));
        // ^^ if this was in our list, we'll prime it to have icon refreshed on processing
        if self.process_discovered_hwnd (hwnd, false) {
            // we'll queue up an enum call only if it passed procesing checks
            self.trigger_enum_windows_query_pending(true);
        } // else can ignore it
    }

    pub fn proc_win_report__obj_destroyed (&self, hwnd:Hwnd) {
        //println!("@{:?} obj-destroyed: {:?}",self._stamp(),hwnd);
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
        // since this is only for non-self windows, we'll want to dimiss ourselves first
        self.handle_req__switche_dismiss();
        spawn ( move || {
            //sleep (Duration::from_millis(30));
            //win_apis::window_activate(hwnd);
            //sleep (Duration::from_millis(50));
            win_apis::window_activate(hwnd);
        } );
    }
    fn handle_req__window_peek (&self, hwnd:Hwnd) {
        let self_hwnd = self.excl_m.self_hwnd();
        spawn ( move || {
            //sleep (Duration::from_millis(30));
            //win_apis::window_activate(hwnd);
            //sleep (Duration::from_millis(50));
            win_apis::window_activate(hwnd);
            // after 'showing'some window for a bit, we'll bring back ourselves
            // the preview duration for this could prob be made configurable
            sleep (Duration::from_millis(1000));
            win_apis::window_activate(self_hwnd);
        } );
    }
    fn handle_req__window_minimize (&self, hwnd:Hwnd) {
        spawn ( move || {
            //sleep (Duration::from_millis(30));
            win_apis::window_minimize(hwnd);
            //sleep (Duration::from_millis(60));
            //win_apis::window_minimize(hwnd);
        } );
    }
    fn handle_req__window_close (&self, hwnd:Hwnd) {
        spawn ( move || {
            //sleep (Duration::from_millis(30));
            //win_apis::window_activate(hwnd);
            //sleep (Duration::from_millis(50));
            //win_apis::window_activate(hwnd);
            // ^^ previously we used to briefly activate the window before closing, ..
            // .. but not sure if that short preview before closing is worthwile, so disabling that for now
            // .. prob could be made configurable

            //sleep (Duration::from_millis(50));
            //win_apis::window_close(hwnd);
            //sleep (Duration::from_millis(120));
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
            //win_apis::window_activate (self_hwnd);
            //sleep (Duration::from_millis(40));
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
            //sleep (Duration::from_millis(30));
            //win_apis::window_hide (self_hwnd);
            //sleep (Duration::from_millis(40));
            win_apis::window_hide (self_hwnd);
        } );
    }
    fn _self_window_close (&self) {
        win_apis::window_close(self.excl_m.self_hwnd());
    }

    fn handle_req__nth_recent_window_activate (&self, n:usize) {
        let hwnd = self.render_lists_m.render_list.read().unwrap() .get(n) .map (|e| e.hwnd);
        //println!("rl;{:?}",self.render_lists_m.render_list.read().unwrap());
        spawn ( move || {
            //sleep (Duration::from_millis(40));
            //hwnd .map ( |hwnd| win_apis::window_activate(hwnd) );
            //sleep (Duration::from_millis(40));
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
        //self.icons_m.emit_all_icon_entries();
    }
    fn handle_req__second_recent_window_activate (&self) {
        self.handle_req__nth_recent_window_activate(1);
    }
    fn handle_req__debug_print (&self) { }


    fn handle_req__data_load(&self) {
        // first we'll send what data we have
        self.icons_m.emit_all_icon_entries();
        self.render_lists_m.render_list.read().unwrap() .iter() .for_each ( |rle| {
            self.hwnd_map.read().unwrap() .get (&rle.hwnd) .iter() .for_each ( |wde| {
                self.emit_win_dat_entry (wde)
        }) });
        self.emit_render_lists_queued(true);
        // then we'll trigger a refresh too
        self.icons_m.mark_all_cached_icon_mappings_stale();
        self.trigger_enum_windows_query_pending(false);    // this will also trigger a renderlist push once the call is done
    }
    fn handle_req__refresh (&self) {
        self.icons_m.mark_all_cached_icon_mappings_stale();
        self.trigger_enum_windows_query_pending(false)
    }
    # [ allow (dead_code) ]
    fn handle_req__refresh_idle (&self) {
        if self.is_dismissed.check() { self.trigger_enum_windows_query_pending(false) }
    }



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
            "fe_req_switche_quit"         => { self.proc_tray_event_quit()       }
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

    fn proc_app_window_event__focus (&self) {
        if self.is_dismissed.check() { self.handle_event__switche_fgnd() }
    }
    fn proc_app_window_event__focus_lost (&self) {
        // nothing really .. this doesnt even count as dismissed (which triggers list-cur-elem reset etc)
    }
    fn proc_event_app_ready (&self) {
        // we want to store a cached value of our hwnd for exclusions-mgr (and general use)
        if self.excl_m.self_hwnd() == 0 {
            let self_hwnd = self.get_self_hwnd();
            self_hwnd .map (|h| SwitcheState::instance().excl_m.store_self_hwnd(h));
            println! ("App starting .. self-hwnd is : {:?}", self_hwnd );
        }
    }

    fn tauri_window_events_handler (&self, ev:&WindowEvent) {
        match ev {
            WindowEvent::Focused (true)       => { self.proc_app_window_event__focus () }
            WindowEvent::Focused (false)      => { self.proc_app_window_event__focus_lost() }
            WindowEvent::Moved (..)           => { } // todo: useful when want to store window pos/size in configs
            WindowEvent::Resized (..)         => { }
            WindowEvent::CloseRequested {..}  => { }
            _ => { }
        }
    }
    pub fn tauri_run_events_handler (&self, _ah:&AppHandle<Wry>, event:RunEvent) {
        match event {
            RunEvent::Ready                          => { self.proc_event_app_ready() }
            RunEvent::WindowEvent   { event, .. }    => { self.tauri_window_events_handler(&event) }
            //RunEvent::ExitRequested { api,   .. }  => { api.prevent_exit() }
            _ => {}
        }
    }


    fn proc_tray_event_show (&self) {
        //self.app_handle.read().unwrap().as_ref() .map (|ah| ah.windows().get("main").map (|w| {w.show(); w.set_focus();} ) );
        // ^^ this works but causes an extra alt-up to be left straggling when krusty is running .. annoying
        //self.handle_event__switche_fgnd();
        //*
        if self.is_dismissed.check() || self.excl_m.self_hwnd()!= win_apis::get_fgnd_window() {
            self.self_window_activate();
            self.handle_event__switche_fgnd();
        }// */
    }
    fn proc_tray_event_quit (&self) {
        self.app_handle.read().unwrap().as_ref() .map (|ah| ah.windows().values() .for_each (|w| { w.close().ok(); }));
        //ah.exit(0)
        // ^^ looks like closing all windows is enough to let it exit .. (requires ofc for main event handling to not disable exit-request)
        //self.get_self_hwnd() .iter() .for_each (|hwnd| self.self_window_close())

    }
    fn proc_tray_event_restart (&self) {
        //self.app_handle.read().unwrap().as_ref() .map (|ah| ah.restart());
        self.app_handle.read().unwrap().as_ref() .map (|ah| tauri::api::process::restart(&ah.env()));
        // ^^ neither of these work .. seems to be ignored ,. maybe some unexposed permissions issue
    }
    fn proc_tray_event_left_click (&self) {
        self.proc_tray_event_show()
    }
    fn proc_tray_event_right_click (&self) {
        // nothign to do to bring up the menu?
    }
    fn proc_tray_event_double_click (&self) {
        // maybe eventually will want to bring up config?
    }
    fn proc_tray_event_menu_click (&self, _ah:&AppHandle<Wry>, menu_id:String) {
        match menu_id.as_str() {
            "show"    => { self.proc_tray_event_show() }
            "quit"    => { self.proc_tray_event_quit() }
            "restart" => { self.proc_tray_event_restart() }
            _ => { }
        }
    }

    pub fn tray_events_handler (&self, ah:&AppHandle<Wry>, event:SystemTrayEvent) {
        match event {
            SystemTrayEvent::LeftClick   { .. }  =>  { self.proc_tray_event_left_click() },
            SystemTrayEvent::RightClick  { .. }  =>  { self.proc_tray_event_right_click() },
            SystemTrayEvent::DoubleClick { .. }  =>  { self.proc_tray_event_double_click() },
            SystemTrayEvent::MenuItemClick { id, .. }  =>  { self.proc_tray_event_menu_click (ah, id) },
            _ => { }
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
        self.emit_backend_notice (Backend_Notice::hotkey_req__scroll_down);
        if self.is_dismissed.check() || self.excl_m.self_hwnd() != win_apis::get_fgnd_window() {
            self.self_window_activate();
            self.handle_event__switche_fgnd();
        }
    }
    pub fn proc_hot_key__scroll_up (&self) {
        // again, we'll ensure the app window is up, then let the frontend deal w it
        self.emit_backend_notice (Backend_Notice::hotkey_req__scroll_up);
        if self.is_dismissed.check() || self.excl_m.self_hwnd() != win_apis::get_fgnd_window() {
            self.self_window_activate();
            self.handle_event__switche_fgnd()
        }
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


    pub fn emit_win_dat_entry (&self, wde:&WinDatEntry) {
        //println! ("emitting **{notice}** win_dat_entry for: {:?}", wde.hwnd);
        let pl = WinDatEntry_FE {
                hwnd: wde.hwnd,
                win_text: wde.win_text.clone(),
                exe_path_name: wde.exe_path_name.clone(),
                icon_cache_idx: self.icons_m.get_cached_icon_idx(wde).unwrap_or(0) as u32,
        };
        println! ("** wde-update for hwnd {:8} : ico-idx: {:?}, {:?}", pl.hwnd, pl.icon_cache_idx, pl.exe_path_name.as_ref().map(|p|p.name.clone()));
        self.app_handle.read().unwrap() .iter().for_each ( |ah| {
            serde_json::to_string(&pl) .map ( |pl| {
                ah.emit_all::<String> (Backend_Event::updated_win_dat_entry.str(), pl )
            } ) .err() .iter() .for_each (|err| println!{"win_dat emit failed: {:?}", err});
        } );
    }

    pub fn emit_icon_entry (&self, ie:&IconEntry) {
        println! ("** icon-update for icon-id: {:?}", ie.ico_id);
        self.app_handle .read().unwrap() .iter() .for_each ( |ah| {
            serde_json::to_string(ie) .map (|pl| {
                ah.emit_all::<String> ( Backend_Event::updated_icon_entry.str(), pl )
            } ) .err() .iter().for_each (|err| println!("icon-entry emit failed: {:?}", err));
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
                grp_sorting_map        : Arc::new(RwLock::new(Default::default())),
                render_list            : Arc::new(RwLock::new(Default::default())),
                grpd_render_list       : Arc::new(RwLock::new(Default::default())),
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
        let is_dismissed = ss.is_dismissed.check();     // local copy to avoid guarded accesses in a loop
        let hwnds = ss.hwnds_ordered.read().unwrap();
        let hwnd_map = ss.hwnd_map.read().unwrap();
        let filt_wdes = hwnds .iter() .flat_map (|h| hwnd_map.get(h))
            .filter (|&wde| !em.runtime_should_excl_check(wde)) .collect::<Vec<&_>>();
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
        println!("render-ready-list recalc -- rl:{:?}", filt_rl.len() ); println!();
        ( filt_rl, grpd_render_list )
    }

    fn update_render_ready_lists (&self, ss:&SwitcheState) {
        let (rl, grl) = self.recalc_render_ready_lists (ss);
        *self.render_list.write().unwrap() = rl;
        *self.grpd_render_list.write().unwrap() = grl;
    }




}
