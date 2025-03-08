#![ allow (non_camel_case_types, non_snake_case, non_upper_case_globals) ]

use std::ops::Not;
use std::sync::Arc;
//use no_deadlocks::RwLock;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::thread::{sleep, spawn};
use std::time::Duration;

use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr};
use tauri::{AppHandle, Emitter, Listener, Wry};
use tracing::{info, warn, error};

use crate::{win_apis, pipe_proc};
use crate::input_proc::InputProcessor;
use crate::icons::IconsManager;
use crate::config::Config;
use crate::render_lists::*;
use crate::win_dats::*;


// for ergonomics, we'll re-export some inner structs publicly out to everyone
pub use crate::win_dats::{Hwnd, ExePathName, WinDatEntry};






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




/// The WinDatEntry_Pl is the subset of WinDatEntry that we send to the front-end
# [ derive (Debug, Clone, Serialize, Deserialize) ]
pub struct WinDatEntry_Pl {
    pub hwnd           : Hwnd,
    pub win_text       : Option<String>,
    pub exe_path_name  : Option<ExePathName>,
    pub icon_cache_idx : u32,
}




/// The IconEntry_Pl is the icon-specific payload we send out to the front-end
# [ derive (Debug, Default, Eq, PartialEq, Hash, Clone, Serialize, Deserialize) ]
pub struct IconEntry_Pl {
    pub ico_id  : usize,
    pub ico_str : String,
}




/// Configs-payload contains the subset of configs that we send out to the front-end
# [ derive (Debug, Eq, PartialEq, Hash, Default, Clone, Serialize, Deserialize) ]
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





# [ derive ( ) ]
pub struct SwitcheState {

    pub conf    : &'static Config,
    pub i_proc  : &'static InputProcessor,
    pub icons_m : &'static IconsManager,

    pub win_dats_m     : &'static WinDatsManager,
    pub render_lists_m : RenderReadyListsManager,
    pub snap_list_m    : SnapListManager,

    pub is_dismissed : Flag,
    pub is_fgnd      : Flag,

    pub in_alt_tab        : Flag,
    pub was_alt_preloaded : Flag,

    pub is_mouse_right_down       : Flag,
    pub in_right_btn_scroll_state : Flag,

    pub app_handle : RwLock < Option <AppHandle<Wry>>>,
    pub self_hwnd  : AtomicIsize,

}




impl SwitcheState {

    pub fn instance () -> &'static SwitcheState {
        static INSTANCE: OnceCell <SwitcheState> = OnceCell::new();
        INSTANCE .get_or_init ( || {
            let ss = SwitcheState {
                conf           : Config::instance(),
                i_proc         : InputProcessor::instance(),
                icons_m        : IconsManager::instance(),

                win_dats_m     : WinDatsManager::instance(),
                render_lists_m : RenderReadyListsManager::default(),
                snap_list_m    : SnapListManager::default(),

                is_dismissed   : Flag::default(),
                is_fgnd        : Flag::default(),

                in_alt_tab        : Flag::default(),
                was_alt_preloaded : Flag::default(),

                is_mouse_right_down       : Flag::default(),
                in_right_btn_scroll_state : Flag::default(),

                app_handle     : RwLock::new (None),
                self_hwnd      : AtomicIsize::default(),
            };
            // lets do some init for the new instance
            pipe_proc::start_pipe_processor();
            ss.win_dats_m.setup_win_event_hooks();
            //ss.i_proc.begin_input_processing(&ss);
            // ^^ instead, we do this everytime on reload (which front-end requests on first load too)
            ss
        } )
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





    /*****  some support functions  ******/

    pub(crate) fn handle_event__switche_fgnd (&'static self) {
        //debug! ("switche self-fgnd report .. refreshing window-list-top icon");
        if self.is_fgnd.is_set() { return }

        // we'll trigger an immdt query if its only just coming to fgnd .. (no time to queue, and its only about ~1ms)
        self .win_dats_m.trigger_enum_windows_query_immdt (EnumWindowsReqType::Light);

        self.is_dismissed.clear(); self.is_fgnd.set();
        self.emit_backend_notice (Backend_Notice::switche_event__in_fgnd);
        // the idea below is that to keep icons mostly updated, we do icon-refresh for a window when it comes to fgnd ..
        // however, when switche is brought to fgnd, recent changes in the topmost window might not be updated yet .. so we'll trigger that here
        if let Some(rle) = self .render_lists_m.render_list.read().unwrap() .first() .copied() {
            if let Some(wde) = self .win_dats_m.hwnd_map .read().unwrap() .get(&rle.hwnd) { self.icons_m.queue_icon_refresh(wde) };
            // note that this ^^ will extend read scope into icon-refresh and its children, but it avoids having to clone wde
        };
    }

    pub(crate) fn handle_event__switche_fgnd_lost (&'static self) {
        if self.is_fgnd.is_clear() { return }
        self.is_fgnd.clear();
        //if self.conf.check_flag__auto_hide_enabled() { self.handle_req__switche_escape() }
        // ^^ instead of immediately hiding switche window, we'll come back after a delay and ensure its still not-fgnd before hiding it
        // .. this reduces spurious auto-hide events from transient fgnd stealers
        // .. (e.g. google-play-games service, which seems to steal fgnd (for ~100ms) soon after unlocking pc post win-L lock etc)
        // .. (or the krusty quick-bar, which however, is verry transient .. usually within 50ms)
        spawn (move || {
            sleep (Duration::from_millis(150));
            if self.is_fgnd.is_clear() {
                if self.conf.check_flag__auto_hide_enabled() {
                    self.handle_req__switche_escape();
                }
                self.emit_backend_notice (Backend_Notice::switche_event__fgnd_lost);
                // ^^ will reset selection to 2nd from top in frontend
            }
        } );
    }

    pub(crate) fn activate_matching_window (&'static self, exe:Option<&str>, title:Option<&str>, partial:bool) {

        let hwnd_map = self.win_dats_m.hwnd_map.read().unwrap();

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








    /*****   Front-End Requests handling  ******/

    // note that in prior incarnations, these were somewhat unreliable and we needed repeated spaced out attempts ..
    // .. however, in this impl, so far things seem to work pretty consistently and without need for delays ..

    fn handle_req__window_activate (&'static self, hwnd:Hwnd) {
        // this call is only for non-self windows .. we'll want to dimiss ourselves
        spawn ( move || {
            win_apis::window_activate(hwnd);
            self.handle_req__switche_dismiss();
        } );
    }

    fn handle_req__window_peek (&'static self, hwnd:Hwnd) {
        let self_hwnd = self.get_self_hwnd();
        spawn ( move || {
            win_apis::window_activate(hwnd);
            // after 'showing'some window for a bit, we'll bring back ourselves
            // the preview duration for this could prob be made configurable
            sleep (Duration::from_millis(1000));
            win_apis::window_activate(self_hwnd);
        } );
    }

    fn handle_req__window_minimize (&'static self, hwnd:Hwnd) {
        spawn ( move || { win_apis::window_minimize(hwnd) } );
    }

    fn handle_req__window_maximize (&'static self, hwnd:Hwnd) {
        spawn ( move || { win_apis::window_maximize(hwnd) } );
    }

    fn handle_req__window_close (&'static self, hwnd:Hwnd) {
        spawn ( move || {
            win_apis::window_close(hwnd);
            // in case it doesnt close (e.g. it presents save dialog etg), we'll recehck and attempt to bring it to fgnd
            spawn ( move || {
                sleep (Duration::from_millis(300));
                if self.win_dats_m.check_hwnd_renderable_pre_passed(hwnd) { self.handle_req__window_activate(hwnd) }
            } );
        } );
    }

    fn self_window_activate (&'static self) {
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

    fn self_window_hide (&'static self) {
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

    fn handle_req__z_idx_window_activate (&'static self, z:usize) {
        let hwnd = self.render_lists_m.render_list.read().unwrap() .get(z) .map (|e| e.hwnd);
        spawn ( move || hwnd.map(win_apis::window_activate) );
    }

    fn handle_req__next_non_minimized_window_activate (&'static self) {
        let hwnd = self.render_lists_m.render_list.read().unwrap() .iter() .map (|e| e.hwnd)
            .enumerate() .find (|&(i,h)| i!=0 && !win_apis::check_window_minimized(h)) .map (|(_,h)| h);
        spawn ( move || hwnd.map(win_apis::window_activate) );
    }


    fn handle_req__switche_dismiss (&'static self) {
        // this is called after some window-activation
        self.self_window_hide();
        self .win_dats_m.trigger_enum_windows_query_pending (EnumWindowsReqType::Light);
        // ^^ we'll do a light query so we'll have the latest ordering when we come back
    }

    fn handle_req__switche_escape (&'static self) {
        // this is called specifically upon escape from switche
        self.self_window_hide();
        //self.handle_req__nth_recent_window_activate(0);
        // ^^ should we reactivate last active window before we dismiss .. nah .. should rather maintain 'least-surprise'
        self .win_dats_m.trigger_enum_windows_query_pending (EnumWindowsReqType::Full);
        // ^^ this is also a good time to do a full query to keep things in sync if any weird events etc have fallen through the cracks
    }

    fn handle_req__switche_quit (&'static self) {
        if let Some(ah) = self.app_handle.read().unwrap().as_ref() { ah.exit(0); }
    }

    fn handle_req__self_auto_resize (&'static self) {
        crate::tauri::auto_setup_self_window (self);
    }

    fn handle_req__toggle_auto_hide (&'static self) {
        self.conf.deferred_update_conf__auto_hide_toggle();
        self.emit_configs();
        crate::tauri::sync_self_always_on_top(self);
    }

    fn handle_req__debug_print (&'static self) { }


    pub(crate) fn handle_req__enum_query_preload (&'static self) {
        // ^^ triggers for instance on alt-press as advance notice for enum-query preload anticipating an alt-tab
        self.win_dats_m.trigger_enum_windows_query_pending(EnumWindowsReqType::Light)
    }

    pub(crate) fn handle_req__data_load (&'static self) {
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
            self.win_dats_m.hwnd_map.read().unwrap() .get (&rle.hwnd) .iter() .for_each ( |wde| {
                self.emit_win_dat_entry (wde)
        }) });
        self.emit_render_lists_queued(true);
        // then we'll trigger a refresh too
        self.render_lists_m.clear_grouping();
        self.icons_m.mark_all_cached_icon_mappings_stale();
        self .win_dats_m.trigger_enum_windows_query_pending (EnumWindowsReqType::Full);
        // ^^ this will also trigger a renderlist push once the call is done
    }
    fn handle_req__refresh (&'static self) {
        self.icons_m.mark_all_cached_icon_mappings_stale();
        self .win_dats_m.trigger_enum_windows_query_pending (EnumWindowsReqType::Full)
    }



    pub fn handle_frontend_request (&'static self, r:&FrontendRequest) {
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

    pub fn setup_front_end_listener (&'static self, ah:&AppHandle<Wry>) {
        let _ = ah .listen_any ( "frontend_request", move |event| {
            //debug!("got event with raw payload {:?}", &event.payload());
            if let Ok(req) = serde_json::from_str::<FrontendRequest>(event.payload()) {
                self.handle_frontend_request(&req)
            }
        } );
    }








    /*****  tauri registered hotkeys and app event handling   ******/

    pub fn proc_app_window_event__focus (&'static self) {
        //if self.is_dismissed.is_set() || self.is_fgnd.is_clear() { self.handle_event__switche_fgnd() }
        // ^^ first off this is unnecessary given our win-event fgnd notice ..
        //  .. plus it will race or get out-of-sync w the actual fgnd notice w/o eqv focus_lost handling, which too would be pointless
    }

    pub fn proc_app_window_event__focus_lost (&'static self) {
        // nothing really .. this doesnt even count as dismissed (which triggers list-cur-elem reset etc)
    }

    pub fn checked_self_activate (&'static self) {
        if self.is_dismissed.is_set() || self.is_fgnd.is_clear() || self.get_self_hwnd() != win_apis::get_fgnd_window() {
            //self .trigger_enum_windows_query_immdt (EnumWindowsReqType::Light);
            // ^^ will happen on fgnd anyway, and is fast enough to not notice difference between those two
            self.self_window_activate();
            //self.handle_event__switche_fgnd();
            // ^^ this should trigger from win-event fgnd report anyway .. and calling this triggers icon refresh, so we'll let it happen then
        }
    }

    pub fn proc_hot_key__invoke (&'static self) {
        // we'll ensure the app window is up, then let the frontend deal w it
        self.checked_self_activate();
        self.emit_backend_notice(Backend_Notice::backend_req__app_invoke)
    }

    pub fn proc_hot_key__scroll_down (&'static self) {
        self.checked_self_activate();
        self.emit_backend_notice (Backend_Notice::backend_req__scroll_down);
    }

    pub fn proc_hot_key__scroll_up (&'static self) {
        self.checked_self_activate();
        self.emit_backend_notice (Backend_Notice::backend_req__scroll_up);
    }

    pub fn proc_hot_key__scroll_end (&'static self) {
        if self.is_dismissed.is_clear() && self.is_fgnd.is_set() {
            self.emit_backend_notice (Backend_Notice::backend_req__scroll_end)
        }
    }

    pub fn proc_hot_key__scroll_end_disarm (&'static self) {
        if self.is_dismissed.is_clear() && self.is_fgnd.is_set() {
            self.emit_backend_notice (Backend_Notice::backend_req__scroll_end_disarm)
        }
    }

    pub fn proc_hot_key__switche_escape (&'static self) {
        self.handle_req__switche_escape();
        self.emit_backend_notice (Backend_Notice::backend_req__switche_escape)
    }

    pub fn proc_hot_key__switch_next_non_minimized (&'static self) {
        self .win_dats_m.trigger_enum_windows_query_immdt (EnumWindowsReqType::Light);
        spawn ( move || {
            sleep (Duration::from_millis(10));
            self .handle_req__next_non_minimized_window_activate();
        } );
    }

    pub fn proc_hot_key__switch_z_idx (&'static self, z:usize) {
        self .win_dats_m.trigger_enum_windows_query_immdt (EnumWindowsReqType::Light);
        spawn ( move || {
            sleep (Duration::from_millis(10));
            self .handle_req__z_idx_window_activate (z);
        } );
    }

    pub fn proc_hot_key__switch_app (&'static self, exe:Option<&str>, title:Option<&str>, partial:bool) {
        self.activate_matching_window (exe, title, partial)
    }

    pub fn proc_hot_key__snap_list_refresh (&'static self) {
        //debug!("snaplist refresh");
        self .win_dats_m.trigger_enum_windows_query_immdt (EnumWindowsReqType::Light);
        spawn ( move || {
            sleep (Duration::from_millis(10));
            self.snap_list_m.capture(&self.render_lists_m);
        } );
    }

    pub fn proc_hot_key__snap_list_switch <SLFN> (&'static self, slfn:SLFN)
        where SLFN : Fn (&SnapListManager) -> Option<Hwnd>
    {
        if let Some(hwnd) = slfn (&self.snap_list_m) {
            win_apis::window_activate(hwnd)
        };
    }

    pub fn proc_menu_req__switche_reload (&'static self) {
        self.emit_backend_notice (Backend_Notice::backend_req__switche_reload)
    }








    /*****    emitting backend messages   ******/

    pub fn emit_win_dat_entry (&'static self, wde:&WinDatEntry) {
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

    pub fn emit_icon_entry (&'static self, ie:&IconEntry_Pl) {
        //debug! ("** icon-update for icon-id: {:?}", ie.ico_id);
        self.app_handle .read().unwrap() .iter() .for_each ( |ah| {
            serde_json::to_string(ie) .map (|pl| {
                ah.emit::<String> ( Backend_Event::updated_icon_entry.str(), pl )
            } ) .err() .iter().for_each (|err| error!("icon-entry emit failed: {:?}", err));
        } );
    }

    pub fn emit_configs (&'static self) {
        info! ("** emitting .. configs-update");
        self.app_handle .read().unwrap() .iter() .for_each ( |ah| {
            serde_json::to_string (&Configs_Pl::assemble(self)) .map (|pl| {
                ah.emit::<String> ( Backend_Event::updated_configs.str(), pl )
            } ) .err() .iter() .for_each (|err| error!("configs emit failed: {:?}", err));
        } );
    }


    pub fn emit_backend_notice (&'static self, notice: Backend_Notice) {
        let pl = BackendNotice_Pl { msg: notice.str().to_string() };
        self.app_handle.read().unwrap() .iter() .for_each ( |ah| {
            serde_json::to_string(&pl) .map ( |pl| {
                info!("sending backend notice: {}", &pl);
                ah.emit::<String> ( Backend_Event::backend_notice.str(), pl ) .map_err (|_| "emit failure")
            } ) .err() .iter().for_each (|err| error!("render-list emit failed: {}", err));
        } )
    }


    fn emit_render_lists (&'static self) {
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

    pub fn emit_render_lists_immdt (&'static self, force:bool) {
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

    pub fn emit_render_lists_queued (&'static self, force:bool) {
        static do_forced      : Lazy<Flag> = Lazy::new (|| {Flag::default()});
        static render_pending : Lazy<Flag> = Lazy::new (|| {Flag::default()});
        if force { do_forced.set() }
        // note that we dont want to keep pushing this out while there are updates ..
        // (we just want to bunch up some but keep pushing when/if there's a stream of updates)
        if render_pending.is_clear() {
            render_pending.set();
            spawn ( move || {
                sleep (Duration::from_millis(100));
                render_pending.clear();
                self.emit_render_lists_immdt(do_forced.check());
                do_forced.clear();
            } );
        }
    }



}







