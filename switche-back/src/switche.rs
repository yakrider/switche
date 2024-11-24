#![ allow (non_camel_case_types) ]
#![ allow (non_snake_case) ]
#![ allow (non_upper_case_globals) ]

use std::collections::{HashMap, HashSet};
use std::ops::{Deref, Not};
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
use tauri::{AppHandle, Emitter, Listener, Wry};
use tracing::{info, warn, error};

use windows::Win32::Foundation::{BOOL, HINSTANCE, HWND, LPARAM};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, EVENT_OBJECT_CLOAKED, EVENT_OBJECT_CREATE, EVENT_OBJECT_DESTROY, EVENT_OBJECT_FOCUS,
    EVENT_OBJECT_HIDE, EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_SHOW, EVENT_OBJECT_UNCLOAKED, EVENT_SYSTEM_FOREGROUND,
    EVENT_OBJECT_REORDER, EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND, GetMessageW, MSG
};

use crate::{win_apis, icons};
use crate::input_proc::InputProcessor;
use crate::icons::IconsManager;
use crate::config::Config;



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
    backend_req__app_invoke,
    backend_req__scroll_down,
    backend_req__scroll_up,
    backend_req__scroll_end,
    backend_req__scroll_end_disarm,
    backend_req__switche_escape,
    backend_req__switche_reload,
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
    switche_version         : &'static str,
    is_elevated             : bool,
    alt_tab_enabled         : bool,
    rbtn_whl_enabled        : bool,
    auto_hide_enabled       : bool,
    group_mode_enabled      : bool,
    n_grp_mode_top_recents  : u32,
    n_grp_mode_last_recents : u32,
    grp_ordering_is_auto    : bool,
}
impl Configs_Pl {
    pub fn assemble (ss:&SwitcheState) -> Configs_Pl { Configs_Pl {
        switche_version          : Config::SWITCHE_VERSION,
        is_elevated              : win_apis::check_cur_proc_elevated().unwrap_or(false),
        alt_tab_enabled          : ss.conf.check_flag__alt_tab_enabled(),
        rbtn_whl_enabled         : ss.conf.check_flag__rbtn_scroll_enabled(),
        auto_hide_enabled        : ss.conf.check_flag__auto_hide_enabled(),
        group_mode_enabled       : ss.conf.check_flag__group_mode_enabled(),
        n_grp_mode_top_recents   : ss.conf.get_n_grp_mode_top_recents(),
        n_grp_mode_last_recents  : ss.conf.get_n_grp_mode_last_recents(),
        grp_ordering_is_auto     : ss.conf.check_flag__auto_order_window_groups(),
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
    pub fn is_set   (&self) -> bool { self.0 .load (Ordering::SeqCst) }
    pub fn is_clear (&self) -> bool { self.0 .load (Ordering::SeqCst) .not() }
    pub fn toggle   (&self) -> bool { ! self.0 .fetch_xor (true, Ordering::SeqCst) }
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


# [ derive ( ) ]
pub struct _SwitcheState {

    // note: should always use hwnds_ordered as that only flips fully formed (unlike hwnds_acc which might be getting slowly rebuilt)
    pub hwnd_map      : RwLock <HashMap <Hwnd, WinDatEntry>>,
    pub hwnds_ordered : RwLock <Vec <Hwnd>>,
    pub hwnds_acc     : RwLock <Vec <Hwnd>>,

    pub cur_call_id       : AtomicIsize,
    pub cur_win_enum_type : EnumWindowsReqType_A,

    pub is_dismissed  : Flag,
    pub is_fgnd       : Flag,

    pub in_alt_tab        : Flag,
    pub was_alt_preloaded : Flag,

    pub is_mouse_right_down       : Flag,
    pub in_right_btn_scroll_state : Flag,

    pub render_lists_m : RenderReadyListsManager,
    pub snap_list_m    : SnapListManager,
    pub i_proc         : InputProcessor,
    pub icons_m        : IconsManager,
    pub conf           : Config,

    pub app_handle : RwLock < Option <AppHandle<Wry>>>,
    pub self_hwnd  : AtomicIsize,

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

# [ derive (Debug, Default) ]
pub struct RenderReadyListsManager {
    grp_sorting_map  : RwLock <HashMap <String, GroupSortingEntry>>,
    render_list      : RwLock <Vec <RenderListEntry>>,
    grpd_render_list : RwLock <Vec <Vec <RenderListEntry>>>,
    exes_excl_map    : RwLock <HashMap <String, HashSet<String>>>,   // Map of exe-name to set of titles
    ordering_ref_map : RwLock <HashMap <String, usize>>,
}

# [ derive (Debug, Default) ]
/// SnapListManager is used to grab a snapshot of the current rendering list hwnds and nav through them w/o bringing switche up
pub struct SnapListManager {
    snap_list : RwLock <Vec <Hwnd>>,
    cur_idx   : AtomicIsize,
}




impl SwitcheState {

    pub fn instance () -> SwitcheState {
        static INSTANCE: OnceCell <SwitcheState> = OnceCell::new();
        INSTANCE .get_or_init ( || {
            let ss = SwitcheState ( Arc::new ( _SwitcheState {
                hwnd_map       : RwLock::new (HashMap::new()),
                hwnds_ordered  : RwLock::new (Vec::default()),
                hwnds_acc      : RwLock::new (Vec::default()),

                cur_call_id       : AtomicIsize::default(),
                cur_win_enum_type : EnumWindowsReqType_A::default(),

                is_dismissed   : Flag::default(),
                is_fgnd        : Flag::default(),

                in_alt_tab        : Flag::default(),
                was_alt_preloaded : Flag::default(),

                is_mouse_right_down       : Flag::default(),
                in_right_btn_scroll_state : Flag::default(),

                render_lists_m : RenderReadyListsManager::default(),
                snap_list_m    : SnapListManager::default(),
                i_proc         : InputProcessor::instance(),
                icons_m        : IconsManager::instance(),
                conf           : Config::instance(),

                app_handle     : RwLock::new (None),
                self_hwnd      : AtomicIsize::default(),
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
        self.self_hwnd.store (hwnd.0, Ordering::Relaxed)
    }
    pub fn get_self_hwnd (&self) -> Hwnd {
        Hwnd (self.self_hwnd.load(Ordering::Relaxed))
    }
    pub fn check_self_hwnd (&self, hwnd:Hwnd) -> bool {
        hwnd == self.get_self_hwnd()
    }




    /*****  win-api windows-enumeration setup and processing  ******/


    fn trigger_enum_windows_query_pending (&self, req_type: EnumWindowsReqType) {
        static trigger_pending : Lazy<Flag> = Lazy::new (|| {Flag::default()});
        // we'll first update the enum type whether we're ready to trigger or not
        if req_type == EnumWindowsReqType::Full { self.cur_win_enum_type .set (EnumWindowsReqType::Full); }
        // ^^ default is light, and if any pending call wants full, we set it to full
        // and if we're not already pending, we'll set it up
        if !trigger_pending.is_set() {
            trigger_pending.set();
            let ss = self.clone();
            let tp = trigger_pending.clone();
            // we'll set up a delay to reduce thrashing from bunched up trains of events that the OS often sends
            let delay = if self.cur_win_enum_type.is_light() {20} else {100};
            spawn ( move || {
                sleep (Duration::from_millis(delay));
                tp.clear();
                ss .trigger_enum_windows_query_immdt (ss.cur_win_enum_type.get());
            } );
        }
    }
    fn trigger_enum_windows_query_immdt (&self, qt: EnumWindowsReqType) {
        info!("***** starting new enum-windows query! **** (query type: {:?})",qt);
        self.cur_win_enum_type.set(qt);    // we might be getting called directly, so gotta cache it up for that
        let call_id_old = self.cur_call_id.fetch_add (1, Ordering::Relaxed);
        *self.hwnds_acc.write().unwrap() = Vec::new();
        // enum windows is blocking, and returns a bool which is false if it fails or either of its callback calls returns false
        // so we'll spawn out this call, and there, we'll wait till its done then trigger cleanup and rendering etc
        let ss = self.clone();
        spawn ( move || unsafe {
            //let t = Instant::now();
            let res = EnumWindows ( Some(Self::enum_windows_streamed_callback), LPARAM (call_id_old + 1) );
            //let dur = Instant::now().duration_since(t).as_millis();
            //debug! ("enum-windows query completed in {dur} ms, with success result: {:?}", res); // --> 'light' ones now finish < 1ms
            if res.is_err() { return }    // the call could have been superceded by a newer request
            ss.post_enum_win_call_cleanup();
        } );
    }

    #[ allow (clippy::missing_safety_doc) ]
    pub unsafe extern "system" fn enum_windows_streamed_callback (hwnd:HWND, call_id:LPARAM) -> BOOL {
        let ss = SwitcheState::instance();
        let latest_call_id = ss.cur_call_id .load (Ordering::Relaxed);
        if call_id.0 > latest_call_id {
            warn! ("WARNING: got win-api callback w higher call_id than last triggered .. will restart enum-call! !");
            ss.trigger_enum_windows_query_immdt (ss.cur_win_enum_type.get());
            return BOOL (false as i32)
        };
        if call_id.0 < latest_call_id {
            // if we're still getting callbacks with stale call_id, signal that call to stop
            warn! ("WARNING: got callbacks @cur-call-id {} from stale cb-id: {} .. ending it!!", latest_call_id, call_id.0);
            return BOOL (false as i32)
        };
        let passed = {
            if ss.cur_win_enum_type.is_light() { ss.check_hwnd_renderable_pre_passed(hwnd.into()) }
            else { ss.process_discovered_hwnd(hwnd.into()) }
        };
        if passed { ss.hwnds_acc .write().unwrap() .push (hwnd.into()) }
        BOOL (true as i32)
    }

    fn check_hwnd_renderable_pre_passed (&self, hwnd:Hwnd) -> bool {
        if let Some(wde) = self.hwnd_map.read().unwrap().get(&hwnd) {
            if wde.should_exclude == Some(false) {
                return true
        }  }
        false
    }

    fn process_discovered_hwnd (&self, hwnd:Hwnd) -> bool {
        use win_apis::*;

        if self.check_self_hwnd (hwnd) { return false }

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

        let excl_flag = Some ( self.render_lists_m.calc_excl_flag (self,wde) );
        if wde.should_exclude != excl_flag { should_emit = true }
        wde.should_exclude = excl_flag;

        drop(hmap); // clearing out write scope before lenghty calls

        if let Some(wde) = self.hwnd_map.read().unwrap().get(&hwnd) {
            if wde.should_exclude == Some(false) {
                self.icons_m .process_found_hwnd_exe_path (wde);
                if should_emit { self.emit_win_dat_entry (wde) }
                return true
        } }
        false
    }


    fn parse_exe_path (exe_path:String) -> Option<ExePathName> {
        let name = exe_path .split('\\') .last() .unwrap_or_default() .to_string();
        if name.is_empty() { None } else { Some (ExePathName { full_path: exe_path, name }) }
    }


    fn post_enum_win_call_cleanup (&self) {
        // we want to clean up both map entries and any icon-cache mappings for any hwnds that are no longer present
        self.cur_win_enum_type .set (EnumWindowsReqType::Light);
        // ^^ this is for next call .. since it needs all pending calls to specify light, its init should be light too
        let cur_hwnds_set = self.hwnds_acc.read().unwrap() .iter() .copied() .collect::<HashSet<Hwnd>>();
        self.hwnds_ordered.read().unwrap() .iter()
            .filter (|hwnd| !cur_hwnds_set.contains(hwnd))
            .for_each (|hwnd| {
                if let Some(wde) = self.hwnd_map .read().unwrap() .get(hwnd) { self.icons_m.clear_dead_hwnd(wde) };
                self.hwnd_map .write().unwrap() .remove(hwnd);
            } );
        // and we'll swap out the live order-list with the readied accumulator to make the new ordering current
        std::mem::swap (&mut *self.hwnds_ordered.write().unwrap(), &mut *self.hwnds_acc.write().unwrap());
        //debug!("hwnds:{}, hacc:{}", self.hwnds_ordered.read().unwrap().len(), self.hwnds_acc.read().unwrap().len());

        // we'll also check/trigger the top in rendering list for once-only backed-off icon requeries
        let fgnd_hwnd_opt = self.hwnds_ordered.read().unwrap().iter().next().cloned(); // avoiding if-let to reduce lock scope
        if let Some(hwnd) = fgnd_hwnd_opt { self.check_for_once_only_backed_off_icon_requeries(hwnd) }

        self.emit_render_lists_queued(false);    // we'll queue it as icon upates might tack on more in a bit
    }






    /*****  some support functions  ******/


    fn handle_event__switche_fgnd (&self) {
        //debug! ("switche self-fgnd report .. refreshing window-list-top icon");
        if self.is_fgnd.is_set() { return }

        // we'll trigger an immdt query if its only just coming to fgnd .. (no time to queue, and its only about ~1ms)
        self .trigger_enum_windows_query_immdt (EnumWindowsReqType::Light);

        self.is_dismissed.clear(); self.is_fgnd.set();
        self.emit_backend_notice (Backend_Notice::switche_event__in_fgnd);
        // the idea below is that to keep icons mostly updated, we do icon-refresh for a window when it comes to fgnd ..
        // however, when switche is brought to fgnd, recent changes in the topmost window might not be updated yet .. so we'll trigger that here
        if let Some(rle) = self .render_lists_m.render_list.read().unwrap() .first() .copied() {
            if let Some(wde) = self .hwnd_map .read().unwrap() .get(&rle.hwnd) { self.icons_m.queue_icon_refresh(wde) };
            // note that this ^^ will extend read scope into icon-refresh and its children, but it avoids having to clone wde
        };
    }
    fn handle_event__switche_fgnd_lost (&self) {
        if self.is_fgnd.is_clear() { return }

        self.is_fgnd.clear();
        if self.conf.check_flag__auto_hide_enabled() {
            //self.handle_req__switche_escape();
            // ^^ instead of immediately hiding switche window, we'll come back after a delay and ensure its still not-fgnd before hiding it
            // .. this reduces spurious auto-hide events from transient fgnd stealers
            // .. (e.g. google-play-games service, which seems to steal fgnd (for ~100ms) soon after unlocking pc post win-L lock etc)
            let ss = self.clone();
            spawn (move || {
                sleep (Duration::from_millis(150));
                if !ss.is_fgnd.is_set() { ss.handle_req__switche_escape() }
            } );
        } else {
            //self.app_handle .read().unwrap() .iter() .for_each ( |ah| {
            //    ah .windows() .get("main") .map (|w| w.set_always_on_top (false))
            //} );
            // ^^ disabled because removing always-on-top here seems to be too late for the window coming to fgnd ..
            // .. in theory we could then try to re-bring the 'fgnd' to fgnd, but eitherway, the topmost enablement here has minimal utility
            // .. esp since, w auto-hide-enabled all those issues are avoided anyway (by always keeping always-on-top)
        }
        self.emit_backend_notice (Backend_Notice::switche_event__fgnd_lost);
    }

    fn activate_matching_window (&self, exe:Option<&str>, title:Option<&str>, partial:bool) {

        let hwnd_map = self.hwnd_map.read().unwrap();

        let match_fn = |base:&str, cand:&str| if !partial {base == cand} else {base.contains(cand)};

        let top2 : Vec<Hwnd> = self.render_lists_m.render_list.read().unwrap()
            .iter() .filter_map ( |rle| hwnd_map .get (&rle.hwnd))
            .filter ( |wde|
                ( exe.is_none()   || exe .filter (|&p| wde.exe_path_name.as_ref().filter(|_p| _p.name.as_str() == p).is_some()).is_some() ) &&
                ( title.is_none() || title .filter (|&t| wde.win_text.as_ref().filter(|_t| match_fn(_t.as_str(),t)).is_some()).is_some() )
            ) .take(2) .map (|wde| wde.hwnd) .collect::<Vec<_>>();

        if top2.is_empty() { return }

        // if we found the hwnd, if its not already active, activate it
        let rle_top = self.render_lists_m.render_list.read().unwrap() .first() .map(|rle| rle.hwnd);
        if top2.first() != rle_top.as_ref() {
            self.handle_req__window_activate (*top2.first().unwrap());
            return
        }
        // if the top match was already active, if there was a second match, activate that
        if top2.get(1) != rle_top.as_ref() {
            if let Some(&hwnd) = top2 .get(1) {
                self.handle_req__window_activate(hwnd);
                return
        }  }
        // so we found a match, its already at top, and there are no other matches .. so toggle to window behind it
        self.handle_req__z_idx_window_activate(1);
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
            match event {
                //
                EVENT_SYSTEM_FOREGROUND    =>  SwitcheState::instance().proc_win_report__fgnd_hwnd     (hwnd.into()),
                EVENT_SYSTEM_MINIMIZESTART =>  SwitcheState::instance().proc_win_report__minimized     (hwnd.into()),
                EVENT_SYSTEM_MINIMIZEEND   =>  SwitcheState::instance().proc_win_report__minimize_end  (hwnd.into()),
                //
                EVENT_OBJECT_CREATE       =>  SwitcheState::instance().proc_win_report__obj_shown      (hwnd.into()),
                EVENT_OBJECT_DESTROY      =>  SwitcheState::instance().proc_win_report__obj_destroyed  (hwnd.into()),
                EVENT_OBJECT_SHOW         =>  SwitcheState::instance().proc_win_report__obj_shown      (hwnd.into()),
                EVENT_OBJECT_HIDE         =>  SwitcheState::instance().proc_win_report__obj_destroyed  (hwnd.into()),
                EVENT_OBJECT_REORDER      =>  SwitcheState::instance().proc_win_report__obj_reorder    (hwnd.into()),
                EVENT_OBJECT_FOCUS        =>  SwitcheState::instance().proc_win_report__fgnd_hwnd      (hwnd.into()),
                EVENT_OBJECT_NAMECHANGE   =>  SwitcheState::instance().proc_win_report__title_changed  (hwnd.into()),
                EVENT_OBJECT_CLOAKED      =>  SwitcheState::instance().proc_win_report__obj_destroyed  (hwnd.into()),
                EVENT_OBJECT_UNCLOAKED    =>  SwitcheState::instance().proc_win_report__obj_shown      (hwnd.into()),
                //
                _ => { }
            }
        }
    }

    // some supoort fns
    pub fn _stamp (&self) -> u128 {
        SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis()
    }
    # [ allow (dead_code) ]
    fn check_owner_chain_in_render_list (&self, hwnd:Hwnd) -> bool {
        if self.check_hwnd_renderable_pre_passed(hwnd) { return true }
        let owner_hwnd = win_apis::get_window_owner(hwnd);
        //debug! ("owner-chain: {:?} -> {:?}", hwnd, owner_hwnd);
        if owner_hwnd.is_null() || owner_hwnd == hwnd { return false }
        self.check_owner_chain_in_render_list (owner_hwnd)
    }
    # [ allow (dead_code) ]
    fn check_parent_chain_in_render_list (&self, hwnd:Hwnd) -> bool {
        if self.check_hwnd_renderable_pre_passed(hwnd) { return true }
        let parent_hwnd = win_apis::get_window_parent(hwnd);
        //debug! ("parent-chain: {:?} -> {:?}", hwnd, owner_hwnd);
        if parent_hwnd.is_null() || parent_hwnd == hwnd { return false }
        self.check_parent_chain_in_render_list (parent_hwnd)
    }
    fn check_for_once_only_backed_off_icon_requeries(&self, hwnd:Hwnd) {
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
        let ss = self.clone();
        spawn ( move || {
            for i in 1..15 {
                // quadratic backoff seems reasonable .. 6 backoffs accumulates to x100, 11 to x500, 15 to 1240
                sleep (Duration::from_millis (100 * i*i));   // w 100ms multiplier, 15th backoff happens by around 2 mins
                if let Some(wde) = ss.hwnd_map.read().unwrap().get(&hwnd) { ss.icons_m.queue_icon_refresh(wde) };
            }
        } );
    }

    pub fn proc_win_report__title_changed (&self, hwnd:Hwnd) {
        //debug! ("@{:?} title-changed: {:?}", self._stamp(), hwnd);

        let hmap = self.hwnd_map.read().unwrap();   // acquired read lock
        let wdeOpt = hmap.get(&hwnd);

        // if its not even in our map, or its not excl-check passed, we can ignore it
        if wdeOpt .filter (|wde| wde.should_exclude == Some(false)) .is_none() { return }

        // somethings like IDE seem to give many window-level title-changes just from typing, w/o title change .. we'll filter those
        if wdeOpt .filter (|wde| wde.win_text.as_ref() == Some(& win_apis::get_window_text(hwnd))) .is_some() { return }

        // so it looks legit .. we'll fully update this hwnd, and queue up an ordering-only enum-query ..
        // .. but first, we'll mark icon stale so the icon gets refreshed (off-thread) upon reprocessing
        wdeOpt .iter() .for_each (|wde| self.icons_m.mark_cached_icon_mapping_stale(wde));

        drop(hmap);   // release lock before lengthier calls

        if self.process_discovered_hwnd (hwnd) {    // this processing will update hwnd data, and refresh stale-marked icon
            self.emit_render_lists_queued (true);
        }
    }


    pub fn proc_win_report__fgnd_hwnd (&self, hwnd:Hwnd) {
        info! ("@{:?} fgnd: {:?}", self._stamp(), hwnd);

        // first, we'll update self-fgnd state if either this is self hwnd, or if its a valid renderable hwnd coming to fgnd
        if self.check_self_hwnd(hwnd) {
            self.handle_event__switche_fgnd();
            return
        }
        // first we'll set this hwnds icon to be refreshed upon reprocessing (if it was already in map)
        if let Some(wde) = self.hwnd_map.read().unwrap() .get(&hwnd) {
            self.icons_m.mark_cached_icon_mapping_stale(wde)
        };
        // now for everything that came to fgnd, we definitely want to reprocess it (so its state, excl, icons can be updated)
        let render_check_passed = self.process_discovered_hwnd(hwnd);

        // and if its renderable, we'll also check/set this for first-timer icon requeries
        if render_check_passed { self.check_for_once_only_backed_off_icon_requeries(hwnd) }

        // now, we'd normally only requery for ordering if this was render-check passed ..
        // .. but windows often reorders the parent window if its child or owned window etc etc comes to fgnd ..
        // .. and checking for owner/parent directly, didnt seem to catch all such cases,
        // .. so we'll just requery light everytime .. (and profiling shows its pretty low cost anyway)
        self .trigger_enum_windows_query_pending (EnumWindowsReqType::Light);

        //tracing::debug! ("fgnd ({:?}) --> {:?}\n{:?}", self.is_fgnd.is_set(),  &self.hwnd_map.read().unwrap().get(&hwnd),  win_apis::win_get_window_frame(hwnd));

        // we'll do the fgnd-lost handling at last .. it might require FE messaging etc
        //if render_check_passed && self.is_fgnd.is_set() { self.handle_event__switche_fgnd_lost() }
        // ^^ gating by render-check-passed would keep switche up upon bringing fgnd child windows etc (e.g. IDE run window)
        if self.is_fgnd.is_set() { self.handle_event__switche_fgnd_lost() }
    }

    pub fn proc_win_report__minimize_end (&self, hwnd:Hwnd) {
        info! ("@{:?} minimize-ended: {:?}", self._stamp(), hwnd);
        // ehh, we can just treat this as a fgnd report (other than the printout above for identification)
        self.proc_win_report__fgnd_hwnd(hwnd);
    }

    pub fn proc_win_report__minimized (&self, hwnd:Hwnd) {
        info! ("@{:?} minimized: {:?}", self._stamp(), hwnd);
        // we only really want to query/update z-order here if this was in our windows list
        if self.check_hwnd_renderable_pre_passed (hwnd) {
            self .trigger_enum_windows_query_pending (EnumWindowsReqType::Light);
        }
    }

    pub fn proc_win_report__obj_shown (&self, hwnd:Hwnd) {
        //debug! ("@{:?} obj-shown: {:?}", self._stamp(), hwnd);

        // windows can get into nothing-in-fgnd state, and in such cases, if the new fgnd is the same as what was fgnd last ..
        // .. then it will not send a new fgnd report .. if this happens to switche, our is_fgnd flags get out of sync ..
        // .. so we'll handle this here directly .. and in fgnd report we'll ignore if our is_fgnd flag is already set
        if self.check_self_hwnd(hwnd) {
            self.handle_event__switche_fgnd();
            return
        }
        // before we reprocess, if this was in our list, we'll prime it to have icon refreshed upon processing
        if let Some(wde) = self.hwnd_map.read().unwrap() .get(&hwnd) {
            self.icons_m.mark_cached_icon_mapping_stale(wde);
        }

        // then do the initial processing, and if that returns renderable-passed, set up ordering-only enum-query
        if self.process_discovered_hwnd(hwnd) {
            self.check_for_once_only_backed_off_icon_requeries(hwnd);
            self .trigger_enum_windows_query_pending (EnumWindowsReqType::Light);
        }
    }

    pub fn proc_win_report__obj_destroyed (&self, hwnd:Hwnd) {
        //debug! ("@{:?} obj-destroyed: {:?}", self._stamp(), hwnd);

        // this is counterpart to special swi handling in obj-shown
        if self.check_self_hwnd(hwnd) {
            self.handle_event__switche_fgnd_lost();
            return
        }

        // if we werent even showing this object, we're done, else queue up a enum-trigger
        if self.hwnd_map.read().unwrap() .get(&hwnd) .filter (|wde| wde.should_exclude == Some(false)) .is_some() {
            // its in our maps, so lets process it, but if processing now rejects it, we should remove it from map
            if !self.process_discovered_hwnd (hwnd) { self.hwnd_map.write().unwrap().remove(&hwnd); }
            // and a 'light' enum call should take care of ordering changes (whether it is now passing or not)
            self .trigger_enum_windows_query_pending (EnumWindowsReqType::Light);
        } // else the destoryed hwnd wasnt even in our list .. we can ignore it
    }

    pub fn proc_win_report__obj_reorder (&self, _hwnd:Hwnd) {
        // todo : prob no use for this .. it seems mostly to be for z-order reordering WITHIN an app's child windows
        //debug! ("@{:?} obj-reorder: {:?}", self._stamp(), hwnd);
        // we'll just trigger a light enum-query to keep ordering in sync
        self .trigger_enum_windows_query_pending (EnumWindowsReqType::Light);
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
        let self_hwnd = self.get_self_hwnd();
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
        let ss = self.clone();
        spawn ( move || {
            win_apis::window_close(hwnd);
            // in case it doesnt close (e.g. it presents save dialog etg), we'll recehck and attempt to bring it to fgnd
            let ss = ss.clone();
            spawn ( move || {
                sleep (Duration::from_millis(300));
                if ss.check_hwnd_renderable_pre_passed(hwnd) { ss.handle_req__window_activate(hwnd) }
            } );
        } );
    }

    fn self_window_activate (&self) {
        self.is_dismissed.clear();
        /*
        self.app_handle .read().unwrap() .iter() .for_each (|ah| {
            ah .webview_windows() .get("main") .iter() .for_each (|w| {
                let (_, _) = ( w.show(), w.set_focus() );
                //let (_, _, _) = ( w.show(), w.set_focus(), w.set_always_on_top(true) );
                // ^^ disabling setting always-on-top since it doesnt play too well when auto-hide is disabled ..
                // (instead, we will always set always-on-top when auto-hide is enabled)
            } );
        } );
        // ^^ had to revert back to using our own self-hide upon Tauri-2.0 migration ..
        //      looks like whatever theyre' doing now, causes a hook fgnd report for switche hwnd right-after self-hide
        //      needless to say, that screws up our logic, and so we'd rather just do the hiding ourselves
        // - and looks like when doing our own hide, tauri cant unhide it either .. so we do the unhide oursevles too
        */
        let self_hwnd = self.get_self_hwnd();
        spawn ( move || {
            win_apis::window_activate (self_hwnd);
        } );
    }
    fn self_window_hide (&self) {
        self.is_dismissed.set();
        /*
        self.app_handle .read().unwrap() .iter() .for_each ( |ah| {
            ah .webview_windows() .get("main") .map (|w| w.hide() );
        } );
        // refer to notes above .. had to revert back to using our own hide again post tauri-2.0 migration
        */
        let self_hwnd = self.get_self_hwnd();
        spawn ( move || {
            win_apis::window_hide (self_hwnd);
        } );
    }

    fn handle_req__z_idx_window_activate (&self, z:usize) {
        let hwnd = self.render_lists_m.render_list.read().unwrap() .get(z) .map (|e| e.hwnd);
        spawn ( move || hwnd.map(win_apis::window_activate) );
    }


    fn handle_req__switche_dismiss (&self) {
        // this is called after some window-activation
        self.self_window_hide();
        self .trigger_enum_windows_query_pending (EnumWindowsReqType::Light);
        // ^^ we'll do a light query so we'll have the latest ordering when we come back
    }
    fn handle_req__switche_escape (&self) {
        // this is called specifically upon escape from switche
        self.self_window_hide();
        //self.handle_req__nth_recent_window_activate(0);
        // ^^ should we reactivate last active window before we dismiss .. nah .. should rather maintain 'least-surprise'
        self .trigger_enum_windows_query_pending (EnumWindowsReqType::Full);
        // ^^ this is also a good time to do a full query to keep things in sync if any weird events etc have fallen through the cracks
    }
    fn handle_req__switche_quit (&self) {
        if let Some(ah) = self.app_handle.read().unwrap().as_ref() { ah.exit(0); }
    }
    fn handle_req__self_auto_resize (&self) {
        crate::tauri::auto_setup_self_window (self);
    }
    fn handle_req__toggle_auto_hide (&self) {
        self.conf.deferred_update_conf__auto_hide_toggle();
        self.emit_configs();
        crate::tauri::sync_self_always_on_top(self);
    }

    fn handle_req__debug_print (&self) { }

    pub(crate) fn handle_req__enum_query_preload (&self) {
        // ^^ triggers for instance on alt-press as advance notice for enum-query preload anticipating an alt-tab
        self.trigger_enum_windows_query_pending(EnumWindowsReqType::Light)
    }

    pub(crate) fn handle_req__data_load(&self) {
        // ^ this triggers on reload .. we'll use that to refresh our hooks, configs etc too
        self.conf.load();
        self.conf.reload_log_level();
        crate::tauri::setup_self_window(self);
        self.render_lists_m .reload_exes_excl_set (self.conf.get_exe_exclusions_list());
        self.render_lists_m .reload_ordering_ref_map (self.conf.get_exe_manual_ordering_seq());
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
        self .trigger_enum_windows_query_pending (EnumWindowsReqType::Full);
        // ^^ this will also trigger a renderlist push once the call is done
    }
    fn handle_req__refresh (&self) {
        self.icons_m.mark_all_cached_icon_mappings_stale();
        self .trigger_enum_windows_query_pending (EnumWindowsReqType::Full)
    }



    pub fn handle_frontend_request (&self, r:&FrontendRequest) {
        info! ("received {:?}", r);

        match r.req.as_str() {
            "fe_req_window_activate"      => { r.hwnd .iter() .for_each (|&h| self.handle_req__window_activate (Hwnd (h as isize)) ); }
            "fe_req_window_peek"          => { r.hwnd .iter() .for_each (|&h| self.handle_req__window_peek     (Hwnd (h as isize)) ); }
            "fe_req_window_minimize"      => { r.hwnd .iter() .for_each (|&h| self.handle_req__window_minimize (Hwnd (h as isize)) ); }
            "fe_req_window_maximize"      => { r.hwnd .iter() .for_each (|&h| self.handle_req__window_maximize (Hwnd (h as isize)) ); }
            "fe_req_window_close"         => { r.hwnd .iter() .for_each (|&h| self.handle_req__window_close    (Hwnd (h as isize)) ); }

            "fe_req_data_load"            => { self.handle_req__data_load()        }
            "fe_req_refresh"              => { self.handle_req__refresh()          }
            "fe_req_switche_escape"       => { self.handle_req__switche_escape()   }
            "fe_req_switche_quit"         => { self.handle_req__switche_quit()     }
            "fe_req_self_auto_resize"     => { self.handle_req__self_auto_resize() }

            "fe_req_grp_mode_enable"      => { self.conf.deferred_update_conf__grp_mode (true)  }
            "fe_req_grp_mode_disable"     => { self.conf.deferred_update_conf__grp_mode (false) }
            "fe_req_auto_hide_toggle"     => { self.handle_req__toggle_auto_hide() }

            "fe_req_edit_config"          => { self.conf.trigger_config_file_edit() }
            "fe_req_reset_config"         => { self.conf.trigger_config_file_reset(); self.handle_req__data_load() }

            "fe_req_debug_print"          => { self.handle_req__debug_print() }

            _ => { warn! ("unrecognized frontend cmd: {}", r.req) }
        }
    }

    pub fn setup_front_end_listener (&self, ah:&AppHandle<Wry>) {
        let ss = self.clone();
        let _ = ah .listen_any ( "frontend_request", move |event| {
            //debug!("got event with raw payload {:?}", &event.payload());
            if let Ok(req) = serde_json::from_str::<FrontendRequest>(event.payload()) {
                ss.handle_frontend_request(&req)
            }
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
        if self.is_dismissed.is_set() || self.is_fgnd.is_clear() || self.get_self_hwnd() != win_apis::get_fgnd_window() {
            //self .trigger_enum_windows_query_immdt (EnumWindowsReqType::Light);
            // ^^ will happen on fgnd anyway, and is fast enough to not notice difference between those two
            self.self_window_activate();
            //self.handle_event__switche_fgnd();
            // ^^ this should trigger from win-event fgnd report anyway .. and calling this triggers icon refresh, so we'll let it happen then
        }
    }
    pub fn proc_hot_key__invoke (&self) {
        // we'll ensure the app window is up, then let the frontend deal w it
        self.checked_self_activate();
        self.emit_backend_notice(Backend_Notice::backend_req__app_invoke)
    }
    pub fn proc_hot_key__scroll_down (&self) {
        self.checked_self_activate();
        self.emit_backend_notice (Backend_Notice::backend_req__scroll_down);
    }
    pub fn proc_hot_key__scroll_up (&self) {
        self.checked_self_activate();
        self.emit_backend_notice (Backend_Notice::backend_req__scroll_up);
    }
    pub fn proc_hot_key__scroll_end (&self) {
        if self.is_dismissed.is_clear() && self.is_fgnd.is_set() {
            self.emit_backend_notice (Backend_Notice::backend_req__scroll_end)
        }
    }
    pub fn proc_hot_key__scroll_end_disarm (&self) {
        if self.is_dismissed.is_clear() && self.is_fgnd.is_set() {
            self.emit_backend_notice (Backend_Notice::backend_req__scroll_end_disarm)
        }
    }
    pub fn proc_hot_key__switche_escape (&self) {
        self.handle_req__switche_escape();
        self.emit_backend_notice (Backend_Notice::backend_req__switche_escape)
    }

    pub fn proc_hot_key__switch_z_idx (&self, z:usize) {
        self .trigger_enum_windows_query_immdt (EnumWindowsReqType::Light);
        let ss = self.clone();
        spawn ( move || {
            sleep (Duration::from_millis(10));
            ss .handle_req__z_idx_window_activate (z);
        } );
    }
    pub fn proc_hot_key__switch_app (&self, exe:Option<&str>, title:Option<&str>, partial:bool) {
        self.activate_matching_window (exe, title, partial)
    }

    pub fn proc_hot_key__snap_list_refresh (&self) {
        //debug!("snaplist refresh");
        self .trigger_enum_windows_query_immdt (EnumWindowsReqType::Light);
        let ss = self.clone();
        spawn ( move || {
            sleep (Duration::from_millis(10));
            ss.snap_list_m.capture(&ss.render_lists_m);
        } );
    }
    pub fn proc_hot_key__snap_list_switch <SLFN> (&self, slfn:SLFN)
        where SLFN : Fn (&SnapListManager) -> Option<Hwnd>
    {
        if let Some(hwnd) = slfn (&self.snap_list_m) {
            win_apis::window_activate(hwnd)
        };
    }
    pub fn proc_menu_req__switche_reload (&self) {
        self.emit_backend_notice (Backend_Notice::backend_req__switche_reload)
    }








    /*****    emitting backend messages   ******/


    pub fn emit_win_dat_entry (&self, wde:&WinDatEntry) {
        //debug! ("emitting **{notice}** win_dat_entry for: {:?}", wde.hwnd);
        let pl = WinDatEntry_Pl {
                hwnd: wde.hwnd,
                win_text: wde.win_text.clone(),
                exe_path_name: wde.exe_path_name.clone(),
                icon_cache_idx: self.icons_m.get_cached_icon_idx(wde).unwrap_or(0) as u32,
        };
        //debug! ("** wde-update for hwnd {:8} : ico-idx: {:?}, {:?}", pl.hwnd, pl.icon_cache_idx, pl.exe_path_name.as_ref().map(|p|p.name.clone()));
        self.app_handle.read().unwrap() .iter().for_each ( |ah| {
            serde_json::to_string(&pl) .map ( |pl| {
                ah.emit::<String> (Backend_Event::updated_win_dat_entry.str(), pl )
            } ) .err() .iter() .for_each (|err| error!{"win_dat emit failed: {:?}", err});
        } );
    }

    pub fn emit_icon_entry (&self, ie:&IconEntry) {
        //debug! ("** icon-update for icon-id: {:?}", ie.ico_id);
        self.app_handle .read().unwrap() .iter() .for_each ( |ah| {
            serde_json::to_string(ie) .map (|pl| {
                ah.emit::<String> ( Backend_Event::updated_icon_entry.str(), pl )
            } ) .err() .iter().for_each (|err| error!("icon-entry emit failed: {:?}", err));
        } );
    }

    pub fn emit_configs (&self) {
        info! ("** emitting .. configs-update");
        self.app_handle .read().unwrap() .iter() .for_each ( |ah| {
            serde_json::to_string (&Configs_Pl::assemble(self)) .map (|pl| {
                ah.emit::<String> ( Backend_Event::updated_configs.str(), pl )
            } ) .err() .iter() .for_each (|err| error!("configs emit failed: {:?}", err));
        } );
    }


    pub fn emit_backend_notice (&self, notice: Backend_Notice) {
        let pl = BackendNotice_Pl { msg: notice.str().to_string() };
        self.app_handle.read().unwrap() .iter() .for_each ( |ah| {
            serde_json::to_string(&pl) .map ( |pl| {
                info!("sending backend notice: {}", &pl);
                ah.emit::<String> ( Backend_Event::backend_notice.str(), pl ) .map_err (|_| "emit failure")
            } ) .err() .iter().for_each (|err| error!("render-list emit failed: {}", err));
        } )
    }


    fn emit_render_lists (&self) {
        let rlp = RenderList_Pl {
            rl  : self.render_lists_m.render_list.read().unwrap().clone(),
            grl : self.render_lists_m.grpd_render_list.read().unwrap().clone()
        };
        //debug!("emitting renderlist ({:?}): {:?}", rlp.rl.len(), serde_json::to_string(&rlp).unwrap());
        self.app_handle.read().unwrap().iter().for_each ( |ah| {
            serde_json::to_string(&rlp).map(|pl| {
                ah.emit::<String>(Backend_Event::updated_render_list.str(), pl) .map_err (|_| "emit failure")
            }) .err() .iter().for_each (|err| error!("rl emit failed: {}", err));
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
        //debug!("rl;{:?}",self.render_lists_m.render_list.read().unwrap());

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

    // -- creation --
    // ^^ happens via derived default


    // --- data reloads ---

    pub fn reload_exes_excl_set (&self, exes:Vec<String>) {
        // for any specified exe, we'll store either we have empty set, or a set of values ..
        // empty set will mean we'll filter out anything that matches exe .. non-empty set will mean we'll have to match title to one of those
        let mut ees = self.exes_excl_map.write().unwrap();
        ees.clear();
        exes.into_iter() .for_each (|e| {
            if let Some ((exe, title)) = e.split_once(';') {
                if let Some(prior) = ees.get_mut(exe) {
                    prior.insert (title.to_string().to_lowercase());
                } else {
                    ees .insert (exe.to_string(), HashSet::from ([title.to_string().to_lowercase()]));
                }
            } else {
                ees.insert (e, HashSet::default());
            }
        } )
    }
    pub fn reload_ordering_ref_map (&self, exes:Vec<String>) {
        let mut orm = self.ordering_ref_map.write().unwrap();
        orm.clear();
        exes .into_iter() .enumerate() .for_each (|(i,s)| { orm.insert(s,i); });
        if !orm.contains_key(Config::UNKNOWN_EXE_STR) {
            let next_idx = orm.len();
            orm.insert(Config::UNKNOWN_EXE_STR.to_string(), next_idx);
        }
    }



    // --- rendering exclusions ---

    pub fn calc_excl_flag (&self, ss:&SwitcheState, wde:&WinDatEntry) -> bool {
        //wde.is_vis == Some(false) ||  wde.is_uncloaked == Some(false) ||    // already covered during enum-filtering
        // ss.check_self_hwnd(wde.hwnd) || wde.win_text.is_none() ||
        // ^^ no good reason to exclude legit windows w/o titles
        ss.check_self_hwnd(wde.hwnd) ||
            wde.exe_path_name.as_ref() .filter (|p| !p.full_path.is_empty()) .is_none() ||
            wde.exe_path_name.as_ref() .filter (|p| !self.exes_excl_check (&p.name, wde)) .is_none()
    }
    pub fn runtime_should_excl_check (&self, ss:&SwitcheState, wde:&WinDatEntry) -> bool {
        wde.should_exclude .unwrap_or_else ( move || self.calc_excl_flag(ss,wde) )
    }
    fn exes_excl_check (&self, estr:&str, wde:&WinDatEntry) -> bool {
        self.exes_excl_map.read().unwrap().get(estr) .is_some_and ( |ot|
            ot.is_empty() || wde.win_text.as_ref().is_some_and (|t| ot.contains(&t.to_lowercase()))
        )
    }




    // --- groups auto-ordering registry ----

    fn update_entry (&self, exe_path:&str, ge:GroupSortingEntry, perc_idx:f32) {
        let mean_perc_idx = (ge.mean_perc_idx * ge.seen_count as f32 + perc_idx) / (ge.seen_count + 1) as f32;
        self.grp_sorting_map.write().unwrap() .insert (
            exe_path.to_owned(), GroupSortingEntry { seen_count: ge.seen_count+1, mean_perc_idx }
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

    fn recalc_render_ready_lists (&self, ss:&SwitcheState) -> (Vec<RenderListEntry>, Vec<Vec<RenderListEntry>>) {

        struct RenderListEntryInfo<'a> { exe_path_name: Option <&'a ExePathName>, ico_idx: i32, rle: RenderListEntry }

        let is_dismissed = ss.is_dismissed.check();     // local copy to avoid guarded accesses in a loop
        let hwnds = ss.hwnds_ordered.read().unwrap();
        let hwnd_map = ss.hwnd_map.read().unwrap();
        let filt_wdes = hwnds .iter() .flat_map (|h| hwnd_map.get(h))
            .filter (|&wde| !self.runtime_should_excl_check(ss,wde)) .collect::<Vec<_>>();

        // we'll gather all the info to sort the renderlist entries and their groups
        let filt_rle_info = filt_wdes .iter() .enumerate() .map ( |(i,wde)| {
            // we'll also register these while we're creating the RLE-infos
            wde.exe_path_name .as_ref() .map (|p| &p.full_path) .iter() .for_each ( |fp| {
                self.register_entry (fp, i, filt_wdes.len() as u32, is_dismissed)
            } );
            RenderListEntryInfo {
                exe_path_name : wde.exe_path_name.as_ref(),
                ico_idx       : ss.icons_m.get_cached_icon_idx(wde).map (|i| i as i32) .unwrap_or(-1),
                rle           : RenderListEntry { hwnd: wde.hwnd, y: 1+i as u32 }
                // ^^ note that renderlist entries are 1 based corresponding to how we want them shown in the UI
                // (and we calc and send from here instead of just wde vecs as grpd-render-list still should show recents-ordered/sorted idxs)
            }
        } ) .collect::<Vec<_>>();

        // for recents, we simply order by the natural z-order as returned by the enum call
        let filt_rl = filt_rle_info .iter() .map (|e| e.rle) .collect::<Vec<_>>();

        let mut grpd_render_list_builder = filt_rle_info .iter() .grouping_by (|e| e.exe_path_name) .into_values() .collect::<Vec<_>>();

        // within each group, we want to keep the z-order, but first sort by icon-count and icon-idx (mostly for things like chrome apps)
        let ico_freqs = filt_rle_info .iter() .fold ( HashMap::new(), |mut m, e| { *m.entry(e.ico_idx).or_insert(0) -= 1; m } );
        // ^^ we use negative counts so that we can sort by descending freq count

        use std::cmp::Ordering;
        grpd_render_list_builder .iter_mut() .for_each (|es| es.sort_unstable_by ( |a,b| {
            match ico_freqs.get(&a.ico_idx) .cmp (&ico_freqs.get(&b.ico_idx)) {  // by ico-freq
                Ordering::Equal => match a.ico_idx .cmp (&b.ico_idx) {  // by ico-idx
                    Ordering::Equal => a.rle.y .cmp (&b.rle.y),         // by z-index
                    ord => ord
                },
                ord => ord
            }
        } ) );

        // as for the groups themselves, we want to order by perc_idx if auto-order, else as listed in configs .. breaking ties by exe name and ico-idx
        // .. so we'll set up the appropirate comparison extractor for those two cases
        fn ext_cmp_auto (rrlm:&RenderReadyListsManager, po:&Option<&ExePathName> ) -> Option<f32> {
            let grm = rrlm.grp_sorting_map.read().unwrap();
            po .and_then (|p| grm .get(&p.full_path) .map (|gse| gse.mean_perc_idx) )
        }
        fn ext_cmp_manual (rrlm:&RenderReadyListsManager, po:&Option<&ExePathName> ) -> Option<f32> {
            let orm = rrlm.ordering_ref_map.read().unwrap();
            po .and_then (|p| orm .get(&p.name) .or (orm.get(Config::UNKNOWN_EXE_STR)) .map (|v| *v as f32) )
        }
        let cmp_ext = if ss.conf.check_flag__auto_order_window_groups() { ext_cmp_auto } else { ext_cmp_manual };

        grpd_render_list_builder .sort_unstable_by ( |ga, gb| {
            // we'll break ties w exe path so there's no instability in ui ordering when two groups have equal perc_idx
            let (g_epn_a, g_epn_b) = (ga.first().and_then(|e| e.exe_path_name), gb.first().and_then(|e| e.exe_path_name));
            match  cmp_ext (self, &g_epn_a) .partial_cmp ( &cmp_ext (self, &g_epn_b) ) .unwrap() {
                // note ^^ that unwrap is ok because it would fail only for NaN
                Ordering::Equal => {
                    let g_exe_a = ga.first() .and_then (|e| e.exe_path_name);
                    let g_exe_b = gb.first() .and_then (|e| e.exe_path_name);
                    match g_exe_a .map (|epn| &epn.name) .cmp (&g_exe_b.map(|epn| &epn.name)) {
                        Ordering::Equal => g_exe_a .map (|epn| &epn.full_path) .cmp (&g_exe_b.map(|epn| &epn.full_path)),
                        ord => ord
                    }
                },
                ord => ord
            }
        } );
        let grpd_render_list = grpd_render_list_builder .into_iter() .map ( |es| {
            es .into_iter() .map (|e| e.rle) .collect::<Vec<_>>()
        } ) .collect::<Vec<Vec<_>>>();
        //debug!("render-ready-list recalc -- rl:{:?}", filt_rl.len());
        ( filt_rl, grpd_render_list )
    }

    fn update_render_ready_lists (&self, ss:&SwitcheState) {
        let (rl, grl) = self.recalc_render_ready_lists (ss);
        *self.render_list.write().unwrap() = rl;
        *self.grpd_render_list.write().unwrap() = grl;
    }


}










impl SnapListManager {

    pub fn capture (&self, rrlm: &RenderReadyListsManager) {
        *self.snap_list .write().unwrap() = rrlm.render_list.read().unwrap().iter() .map (|rle| rle.hwnd) .collect::<Vec<Hwnd>>();
        self.cur_idx.store (0, Ordering::Relaxed);
    }

    pub fn next_hwnd (&self) -> Option<Hwnd> {
        let sl = self.snap_list.read().unwrap();
        if sl.is_empty() { return None }
        let idx = (self.cur_idx.fetch_add(1, Ordering::Relaxed) + 1) .rem_euclid (sl.len() as isize);
        sl .get (idx as usize) .copied()
    }
    pub fn prev_hwnd (&self) -> Option<Hwnd> {
        let sl = self.snap_list.read().unwrap();
        if sl.is_empty() { return None }
        let idx = (self.cur_idx.fetch_sub(1, Ordering::Relaxed) - 1) .rem_euclid (sl.len() as isize);
        sl .get (idx as usize) .copied()
    }
    pub fn top_hwnd (&self) -> Option<Hwnd> {
        let sl = self.snap_list.read().unwrap();
        if sl.is_empty() { return None }
        self.cur_idx.store (0, Ordering::Relaxed);
        sl .first() .copied()
    }
    pub fn bottom_hwnd (&self) -> Option<Hwnd> {
        let sl = self.snap_list.read().unwrap();
        if sl.is_empty() { return None }
        let idx = self.snap_list.read().unwrap().len() - 1;
        self.cur_idx.store (idx as isize, Ordering::Relaxed);
        sl .get (idx) .copied()
    }

}

