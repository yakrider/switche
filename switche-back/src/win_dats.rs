#![ allow (non_camel_case_types, non_snake_case, non_upper_case_globals) ]

use std::collections::{HashMap, HashSet};
//use no_deadlocks::RwLock;
use std::sync::RwLock;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::thread::{sleep, spawn};
use std::time::{Duration, SystemTime};

use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use windows::Win32::Foundation::{BOOL, HINSTANCE, HWND, LPARAM};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, EVENT_OBJECT_CLOAKED, EVENT_OBJECT_CREATE, EVENT_OBJECT_DESTROY, EVENT_OBJECT_FOCUS,
    EVENT_OBJECT_HIDE, EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_SHOW, EVENT_OBJECT_UNCLOAKED, EVENT_SYSTEM_FOREGROUND,
    EVENT_OBJECT_REORDER, EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND, GetMessageW, MSG
};

use crate::{win_apis, icons};
use crate::switche::{Flag, SwitcheState};





#[derive (Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Hwnd (pub isize);

impl Hwnd {
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
    pub fn HWND (&self) -> HWND {
        HWND (self.0 as *mut _)
    }
}

impl From<HWND> for Hwnd {
    fn from (h:HWND) -> Self {
        Hwnd (h.0 as isize)
    }
}





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
    pub icons_requeried   : bool,
    //pub icon_cache_idx    : usize,        // <- we'd rather populate this at render-list emission time
}





# [ atomic_enum::atomic_enum ]
# [ derive (PartialEq) ]
pub enum EnumWindowsReqType { Light, Full }
// ^^ the atomic_enum crate will generate an AtomicEnumWindowsReqType for us

# [ derive ( ) ]
pub struct EnumWindowsReqType_A (AtomicEnumWindowsReqType);

impl EnumWindowsReqType_A {
    pub fn new () -> EnumWindowsReqType_A {
        EnumWindowsReqType_A ( AtomicEnumWindowsReqType::new (EnumWindowsReqType::Light) )
    }
    pub fn get (&self) -> EnumWindowsReqType {
        self.0 .load (Ordering::SeqCst)
    }
    pub fn set (&self, req_type: EnumWindowsReqType) {
        self.0 .store (req_type, Ordering::SeqCst);
    }
    pub fn is_light (&self) -> bool {
        self.0 .load (Ordering::SeqCst) == EnumWindowsReqType::Light
    }
}
impl Default for EnumWindowsReqType_A {
    fn default () -> Self { Self::new() }
}






# [derive ()]
pub struct WinDatsManager {

    // note: should always use hwnds_ordered as that only flips fully formed (unlike hwnds_acc which might be getting slowly rebuilt)
    pub hwnd_map      : RwLock <HashMap <Hwnd, WinDatEntry>>,
    pub hwnds_ordered : RwLock <Vec <Hwnd>>,
    pub hwnds_acc     : RwLock <Vec <Hwnd>>,

    pub cur_call_id       : AtomicIsize,
    pub cur_win_enum_type : EnumWindowsReqType_A,

}




impl WinDatsManager {

    pub fn instance () -> &'static WinDatsManager {
        static INSTANCE: OnceCell <WinDatsManager> = OnceCell::new();
        INSTANCE .get_or_init ( || {
            WinDatsManager {
                hwnd_map      : RwLock::new (HashMap::new()),
                hwnds_ordered : RwLock::new (Vec::default()),
                hwnds_acc     : RwLock::new (Vec::default()),
                cur_call_id       : AtomicIsize::default(),
                cur_win_enum_type : EnumWindowsReqType_A::default(),
            }
        } )
    }



    /*****  win-api windows-enumeration setup and processing  ******/

    pub(crate) fn trigger_enum_windows_query_pending (&'static self, req_type: EnumWindowsReqType) {
        static trigger_pending : Lazy<Flag> = Lazy::new (|| {Flag::default()});
        // we'll first update the enum type whether we're ready to trigger or not
        if req_type == EnumWindowsReqType::Full { self.cur_win_enum_type .set (EnumWindowsReqType::Full); }
        // ^^ default is light, and if any pending call wants full, we set it to full
        // and if we're not already pending, we'll set it up
        if !trigger_pending.is_set() {
            trigger_pending.set();
            let tp = trigger_pending.clone();
            // we'll set up a delay to reduce thrashing from bunched up trains of events that the OS often sends
            let delay = if self.cur_win_enum_type.is_light() {20} else {100};
            spawn ( move || {
                sleep (Duration::from_millis(delay));
                tp.clear();
                self .trigger_enum_windows_query_immdt (self.cur_win_enum_type.get());
            } );
        }
    }

    pub(crate) fn trigger_enum_windows_query_immdt (&'static self, qt: EnumWindowsReqType) {
        info!("***** starting new enum-windows query! **** (query type: {:?})",qt);
        self.cur_win_enum_type.set(qt);    // we might be getting called directly, so gotta cache it up for that
        let call_id_old = self.cur_call_id.fetch_add (1, Ordering::Relaxed);
        *self.hwnds_acc.write().unwrap() = Vec::new();
        // enum windows is blocking, and returns a bool which is false if it fails or either of its callback calls returns false
        // so we'll spawn out this call, and there, we'll wait till its done then trigger cleanup and rendering etc
        spawn ( move || unsafe {
            //let t = Instant::now();
            let res = EnumWindows ( Some(Self::enum_windows_streamed_callback), LPARAM (call_id_old + 1) );
            //let dur = Instant::now().duration_since(t).as_millis();
            //debug! ("enum-windows query completed in {dur} ms, with success result: {:?}", res); // --> 'light' ones now finish < 1ms
            if res.is_err() { return }    // the call could have been superceded by a newer request
            self.post_enum_win_call_cleanup();
        } );
    }


    #[ allow (clippy::missing_safety_doc) ]
    pub unsafe extern "system" fn enum_windows_streamed_callback (hwnd:HWND, call_id:LPARAM) -> BOOL {
        let ss = SwitcheState::instance();
        let wdm = ss.win_dats_m;
        let latest_call_id = wdm.cur_call_id .load (Ordering::Relaxed);
        if call_id.0 > latest_call_id {
            warn! ("WARNING: got win-api callback w higher call_id than last triggered .. will restart enum-call! !");
            wdm.trigger_enum_windows_query_immdt (wdm.cur_win_enum_type.get());
            return BOOL (false as i32)
        };
        if call_id.0 < latest_call_id {
            // if we're still getting callbacks with stale call_id, signal that call to stop
            warn! ("WARNING: got callbacks @cur-call-id {} from stale cb-id: {} .. ending it!!", latest_call_id, call_id.0);
            return BOOL (false as i32)
        };
        let passed = {
            if wdm.cur_win_enum_type.is_light() { wdm.check_hwnd_renderable_pre_passed(hwnd.into()) }
            else { wdm.process_discovered_hwnd (hwnd.into(), ss) }
        };
        if passed { wdm.hwnds_acc .write().unwrap() .push (hwnd.into()) }
        BOOL (true as i32)
    }


    pub(crate) fn check_hwnd_renderable_pre_passed (&'static self, hwnd:Hwnd) -> bool {
        if let Some(wde) = self.hwnd_map.read().unwrap().get(&hwnd) {
            if wde.should_exclude == Some(false) {
                return true
        }  }
        false
    }



    pub(crate) fn process_discovered_hwnd (&'static self, hwnd:Hwnd, ss: &'static SwitcheState) -> bool {
        use win_apis::*;

        if ss.check_self_hwnd (hwnd) { return false }

        if !check_window_visible  (hwnd)  { return false }
        if  check_window_cloaked  (hwnd)  { return false }

        if !check_if_app_window (hwnd) {
            if  check_window_has_owner (hwnd)  { return false }
            if  check_if_tool_window   (hwnd)  { return false }
            if  check_if_tool_tip      (hwnd)  { return false }
        }

        let mut hmap = self.hwnd_map.write().unwrap();

        let wde = hmap .entry(hwnd) .or_insert_with (|| WinDatEntry { hwnd, ..WinDatEntry::default() } );

        let mut should_emit = false;

        // we'll refresh the title every time this runs
        let cur_title = Some (get_window_text(wde.hwnd)) .filter (|s| !s.is_empty());
        if wde.win_text != cur_title { should_emit = true }
        wde.win_text = cur_title;

        // but only query exe-path if we havent populated it before
        if !wde.is_exe_queried {
            should_emit = true; wde.is_exe_queried = true;
            wde.exe_path_name = get_hwnd_exe_path(wde.hwnd) .and_then (Self::parse_exe_path);
            if wde.exe_path_name .iter() .any (|ep| ep.name.as_str() == "ApplicationFrameHost.exe") { //dbg!(hwnd);
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

        let excl_flag = Some ( ss.render_lists_m.calc_excl_flag (ss,wde) );
        if wde.should_exclude != excl_flag { should_emit = true }
        wde.should_exclude = excl_flag;

        drop(hmap); // clearing out write scope before lenghty calls

        if let Some(wde) = self.hwnd_map.read().unwrap().get(&hwnd) {
            if wde.should_exclude == Some(false) {
                ss.icons_m .process_found_hwnd_exe_path (wde);
                if should_emit { ss.emit_win_dat_entry (wde) }
                return true
        } }
        false
    }


    fn parse_exe_path (exe_path:String) -> Option<ExePathName> {
        let name = exe_path .split('\\') .last() .unwrap_or_default() .to_string();
        if name.is_empty() { None } else { Some (ExePathName { full_path: exe_path, name }) }
    }


    fn post_enum_win_call_cleanup (&'static self) {
        // we want to clean up both map entries and any icon-cache mappings for any hwnds that are no longer present
        let ss = SwitcheState::instance();
        self.cur_win_enum_type .set (EnumWindowsReqType::Light);
        // ^^ this is for next call .. since it needs all pending calls to specify light, its init should be light too
        let cur_hwnds_set = self.hwnds_acc.read().unwrap() .iter() .copied() .collect::<HashSet<Hwnd>>();
        self.hwnds_ordered.read().unwrap() .iter()
            .filter (|hwnd| !cur_hwnds_set.contains(hwnd))
            .for_each (|hwnd| {
                if let Some(wde) = self.hwnd_map .read().unwrap() .get(hwnd) { ss.icons_m.clear_dead_hwnd(wde) };
                self.hwnd_map .write().unwrap() .remove(hwnd);
            } );
        // and we'll swap out the live order-list with the readied accumulator to make the new ordering current
        std::mem::swap (&mut *self.hwnds_ordered.write().unwrap(), &mut *self.hwnds_acc.write().unwrap());
        //debug!("hwnds:{}, hacc:{}", self.hwnds_ordered.read().unwrap().len(), self.hwnds_acc.read().unwrap().len());

        // we'll also check/trigger the top in rendering list for once-only backed-off icon requeries
        let fgnd_hwnd_opt = self.hwnds_ordered.read().unwrap().iter().next().cloned(); // avoiding if-let to reduce lock scope
        if let Some(hwnd) = fgnd_hwnd_opt { self.check_for_once_only_backed_off_icon_requeries (hwnd, ss) }

        ss.emit_render_lists_queued(false);    // we'll queue it as icon upates might tack on more in a bit
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
                0x8004 : EVENT_OBJECT_REORDER
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
            while BOOL(0) != GetMessageW (&mut msg, Hwnd(0).HWND(), 0, 0) { };
        } );
    }



    #[ allow (clippy::missing_safety_doc) ]
    pub unsafe extern "system" fn win_event_hook_cb (
        _id_hook: HWINEVENTHOOK, event: u32, hwnd: HWND,
        id_object: i32, id_child: i32, _id_thread: u32, _event_time: u32
    ) {
        if id_object == 0 && id_child == 0 {
            //let t = std::time::UNIX_EPOCH.elapsed().unwrap().as_millis();
            //tracing::debug!("--> {:16} : hook event: 0x{:X}, hwnd:{:?}, id_object: 0x{:4X}", t, event, hwnd, id_object);
            // todo: prob need to figure out actual logging w debug/run switches .. theres samples incl in the other repo
            let ss = SwitcheState::instance();
            match event {
                //
                EVENT_SYSTEM_FOREGROUND    =>  ss.win_dats_m.proc_win_report__fgnd_hwnd      (hwnd.into(), ss),
                EVENT_SYSTEM_MINIMIZESTART =>  ss.win_dats_m.proc_win_report__minimized      (hwnd.into(), ss),
                EVENT_SYSTEM_MINIMIZEEND   =>  ss.win_dats_m.proc_win_report__minimize_end   (hwnd.into(), ss),
                //
                EVENT_OBJECT_CREATE        =>  ss.win_dats_m.proc_win_report__obj_shown      (hwnd.into(), ss),
                EVENT_OBJECT_DESTROY       =>  ss.win_dats_m.proc_win_report__obj_destroyed  (hwnd.into(), ss),
                EVENT_OBJECT_SHOW          =>  ss.win_dats_m.proc_win_report__obj_shown      (hwnd.into(), ss),
                EVENT_OBJECT_HIDE          =>  ss.win_dats_m.proc_win_report__obj_destroyed  (hwnd.into(), ss),
                EVENT_OBJECT_REORDER       =>  ss.win_dats_m.proc_win_report__obj_reorder    (hwnd.into(), ss),
                EVENT_OBJECT_FOCUS         =>  ss.win_dats_m.proc_win_report__fgnd_hwnd      (hwnd.into(), ss),
                EVENT_OBJECT_NAMECHANGE    =>  ss.win_dats_m.proc_win_report__title_changed  (hwnd.into(), ss),
                EVENT_OBJECT_CLOAKED       =>  ss.win_dats_m.proc_win_report__obj_destroyed  (hwnd.into(), ss),
                EVENT_OBJECT_UNCLOAKED     =>  ss.win_dats_m.proc_win_report__obj_shown      (hwnd.into(), ss),
                //
                _ => { }
            }
        }
    }



    pub fn _stamp (&self) -> u128 {
        SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis()
    }
    # [ allow (dead_code) ]
    fn check_owner_chain_in_render_list (&'static self, hwnd:Hwnd) -> bool {
        if self.check_hwnd_renderable_pre_passed(hwnd) { return true }
        let owner_hwnd = win_apis::get_window_owner(hwnd);
        //debug! ("owner-chain: {:?} -> {:?}", hwnd, owner_hwnd);
        if owner_hwnd.is_null() || owner_hwnd == hwnd { return false }
        self.check_owner_chain_in_render_list (owner_hwnd)
    }
    # [ allow (dead_code) ]
    fn check_parent_chain_in_render_list (&'static self, hwnd:Hwnd) -> bool {
        if self.check_hwnd_renderable_pre_passed(hwnd) { return true }
        let parent_hwnd = win_apis::get_window_parent(hwnd);
        //debug! ("parent-chain: {:?} -> {:?}", hwnd, owner_hwnd);
        if parent_hwnd.is_null() || parent_hwnd == hwnd { return false }
        self.check_parent_chain_in_render_list (parent_hwnd)
    }
    fn check_for_once_only_backed_off_icon_requeries (&'static self, hwnd:Hwnd, ss: &'static SwitcheState) {
        // some freshly created windows like chrome apps seem to take forever to put up real icons instead of just placeholders ..
        // .. to catch those, when some hwnd is seen to be fgnd/show for first time ever, we'll set up backed-off icon requeries for it

        // but first, if its already queried, we return
        if self.hwnd_map.read().unwrap() .get(&hwnd) .filter (|wde| wde.icons_requeried) .is_some() { return }

        // else we mark the flag before setting up requeries
        if let Some(wde) = self.hwnd_map.write().unwrap() .get_mut(&hwnd) {
            if wde.should_exclude == Some(true) { return }
            wde.icons_requeried = true;
        } else { return }

        // and if all checks out, we can set off the once-only backed-off requeries for this renderable hwnd
        info!("## triggering backed-off icon requeries for {:?}", hwnd);
        spawn ( move || {
            for i in 1..15 {
                // quadratic backoff seems reasonable .. 6 backoffs accumulates to x100, 11 to x500, 15 to 1240
                sleep (Duration::from_millis (100 * i*i));   // w 100ms multiplier, 15th backoff happens by around 2 mins
                if let Some(wde) = self.hwnd_map.read().unwrap().get(&hwnd) { ss.icons_m.queue_icon_refresh(wde) };
            }
        } );
    }


    pub fn proc_win_report__title_changed (&'static self, hwnd:Hwnd, ss: &'static SwitcheState) {
        //debug! ("@{:?} title-changed: {:?}", self._stamp(), hwnd);

        let hmap = self.hwnd_map.read().unwrap();   // acquired read lock
        let wdeOpt = hmap.get(&hwnd);

        // if its not even in our map, or its not excl-check passed, we can ignore it
        if wdeOpt .filter (|wde| wde.should_exclude == Some(false)) .is_none() { return }

        // somethings like IDE seem to give many window-level title-changes just from typing, w/o title change .. we'll filter those
        if wdeOpt .filter (|wde| wde.win_text.as_ref() == Some(& win_apis::get_window_text(hwnd))) .is_some() { return }

        // so it looks legit .. we'll fully update this hwnd, and queue up an ordering-only enum-query ..
        // .. but first, we'll mark icon stale so the icon gets refreshed (off-thread) upon reprocessing
        wdeOpt .iter() .for_each (|wde| ss.icons_m.mark_cached_icon_mapping_stale(wde));

        drop(hmap);   // release lock before lengthier calls

        if self.process_discovered_hwnd (hwnd, ss) {    // this processing will update hwnd data, and refresh stale-marked icon
            ss.emit_render_lists_queued (true);
        }
    }


    pub fn proc_win_report__fgnd_hwnd (&'static self, hwnd:Hwnd, ss: &'static SwitcheState) {
        info! ("@{:?} fgnd: {:?}", self._stamp(), hwnd);

        // first, we'll update self-fgnd state if either this is self hwnd, or if its a valid renderable hwnd coming to fgnd
        if ss.check_self_hwnd(hwnd) {
            ss.handle_event__switche_fgnd();
            return
        }
        // first we'll set this hwnds icon to be refreshed upon reprocessing (if it was already in map)
        if let Some(wde) = self.hwnd_map.read().unwrap() .get(&hwnd) {
            ss.icons_m.mark_cached_icon_mapping_stale(wde)
        };
        // now for everything that came to fgnd, we definitely want to reprocess it (so its state, excl, icons can be updated)
        let render_check_passed = self.process_discovered_hwnd(hwnd, ss);

        // and if its renderable, we'll also check/set this for first-timer icon requeries
        if render_check_passed { self.check_for_once_only_backed_off_icon_requeries(hwnd, ss) }

        // now, we'd normally only requery for ordering if this was render-check passed ..
        // .. but windows often reorders the parent window if its child or owned window etc etc comes to fgnd ..
        // .. and checking for owner/parent directly, didnt seem to catch all such cases,
        // .. so we'll just requery light everytime .. (and profiling shows its pretty low cost anyway)
        self .trigger_enum_windows_query_pending (EnumWindowsReqType::Light);

        //tracing::debug! ("fgnd ({:?}) --> {:?}\n{:?}", self.is_fgnd.is_set(),  &self.hwnd_map.read().unwrap().get(&hwnd),  win_apis::win_get_window_frame(hwnd));

        // we'll do the fgnd-lost handling at last .. it might require FE messaging etc
        //if render_check_passed && self.is_fgnd.is_set() { self.handle_event__switche_fgnd_lost() }
        // ^^ gating by render-check-passed would keep switche up upon bringing fgnd child windows etc (e.g. IDE run window)
        if ss.is_fgnd.is_set() { ss.handle_event__switche_fgnd_lost() }
    }


    pub fn proc_win_report__minimize_end (&'static self, hwnd:Hwnd, ss: &'static SwitcheState) {
        info! ("@{:?} minimize-ended: {:?}", self._stamp(), hwnd);
        // ehh, we can just treat this as a fgnd report (other than the printout above for identification)
        self.proc_win_report__fgnd_hwnd (hwnd, ss);
    }


    pub fn proc_win_report__minimized (&'static self, hwnd:Hwnd, _ss: &'static SwitcheState) {
        info! ("@{:?} minimized: {:?}", self._stamp(), hwnd);
        // we only really want to query/update z-order here if this was in our windows list
        if self.check_hwnd_renderable_pre_passed (hwnd) {
            self .trigger_enum_windows_query_pending (EnumWindowsReqType::Light);
        }
    }


    pub fn proc_win_report__obj_shown (&'static self, hwnd:Hwnd, ss: &'static SwitcheState) {
        //debug! ("@{:?} obj-shown: {:?}", self._stamp(), hwnd);

        // windows can get into nothing-in-fgnd state, and in such cases, if the new fgnd is the same as what was fgnd last ..
        // .. then it will not send a new fgnd report .. if this happens to switche, our is_fgnd flags get out of sync ..
        // .. so we'll handle this here directly .. and in fgnd report we'll ignore if our is_fgnd flag is already set
        if ss.check_self_hwnd(hwnd) {
            ss.handle_event__switche_fgnd();
            return
        }
        // before we reprocess, if this was in our list, we'll prime it to have icon refreshed upon processing
        if let Some(wde) = self.hwnd_map.read().unwrap() .get(&hwnd) {
            ss.icons_m.mark_cached_icon_mapping_stale(wde);
        }

        // then do the initial processing, and if that returns renderable-passed, set up ordering-only enum-query
        if self.process_discovered_hwnd (hwnd, ss) {
            self.check_for_once_only_backed_off_icon_requeries (hwnd, ss);
            self .trigger_enum_windows_query_pending (EnumWindowsReqType::Light);
        }
    }


    pub fn proc_win_report__obj_destroyed (&'static self, hwnd:Hwnd, ss: &'static SwitcheState) {
        //debug! ("@{:?} obj-destroyed: {:?}", self._stamp(), hwnd);

        // this is counterpart to special swi handling in obj-shown
        if ss.check_self_hwnd(hwnd) {
            ss.handle_event__switche_fgnd_lost();
            return
        }

        // if we werent even showing this object, we're done, else queue up a enum-trigger
        if self.hwnd_map.read().unwrap() .get(&hwnd) .filter (|wde| wde.should_exclude == Some(false)) .is_some() {
            // its in our maps, so lets process it, but if processing now rejects it, we should remove it from map
            if !self.process_discovered_hwnd (hwnd, ss) { self.hwnd_map.write().unwrap().remove(&hwnd); }
            // and a 'light' enum call should take care of ordering changes (whether it is now passing or not)
            self .trigger_enum_windows_query_pending (EnumWindowsReqType::Light);
        } // else the destoryed hwnd wasnt even in our list .. we can ignore it
    }


    pub fn proc_win_report__obj_reorder (&'static self, _hwnd:Hwnd, _ss: &'static SwitcheState) {
        // todo : prob no use for this .. it seems mostly to be for z-order reordering WITHIN an app's child windows
        //debug! ("@{:?} obj-reorder: {:?}", self._stamp(), hwnd);
        // we'll just trigger a light enum-query to keep ordering in sync
        self .trigger_enum_windows_query_pending (EnumWindowsReqType::Light);
    }


}



