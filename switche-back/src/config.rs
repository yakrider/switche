#![ allow (non_snake_case, non_upper_case_globals) ]

use std::{fs, time};
use std::ops::{Deref, Not};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};
use std::time::SystemTime;

use once_cell::sync::{Lazy, OnceCell};
use tauri::Manager;
use toml_edit::DocumentMut;

use tracing::{info, error, warn};
use tracing::metadata::LevelFilter;
use tracing_appender::non_blocking;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{Layer, Registry, reload};
use tracing_subscriber::fmt::time::LocalTime;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::prelude::*;

use crate::switche::SwitcheState;





# [ derive (Debug, Clone) ]
/// Deferred Executor sets up a deferred action until a (pushable/resettable) deadline has passed
pub struct DeferredExecutor {
    deadline : Arc <Mutex <SystemTime>>,
}

/// Action type to pass in to a DeferredExecutor
pub type Action = Arc < dyn Fn() + Send + Sync + 'static >;




# [ derive (Debug) ]
pub struct _Config {
    pub toml     : RwLock <Option <DocumentMut>>,
    pub default  : DocumentMut,
    pub loglevel : RwLock <Option <Handle <LevelFilter, Registry>>>,
}


# [ derive (Debug, Clone) ]
pub struct Config ( Arc <_Config> );

impl Deref for Config {
    type Target = _Config;
    fn deref (&self) -> &_Config { &self.0 }
}





// first some module level helper functions ..
/// Returns the directory of the currently running executable
fn get_app_dir () -> Option<PathBuf> {
    std::env::current_exe().ok() .and_then (|p| p.parent() .map (|p| p.to_path_buf()))
}

/// Checks whether a path is readonly .. note: this is not a good indicator of whether the path is writeable by a user
fn _is_writeable (path: &Path) -> bool {
    if let Ok(metadata) = fs::metadata(path) {
        metadata.permissions().readonly().not()
        // ^^ doesnt work .. its a direct read-only flag check, orthogonal to user-role based permissions
    } else { false }
}

/// Checks whether a path is writeable by the current user by attempting to open/create a file in write mode
fn is_writeable (path: &Path) -> bool {
    fs::OpenOptions::new().write(true).create(true).truncate(false).open(path).is_ok()
    // note that ^^ this is similar to 'touch' and will create an empty file if it doesnt exist
}




impl Config {

    pub fn instance () -> Config {
        static INSTANCE: OnceCell <Config> = OnceCell::new();
        INSTANCE .get_or_init ( || {
            let conf = Config ( Arc::new ( _Config {
                toml    : RwLock::new (None),
                default : DocumentMut::from_str (include_str!("../../switche.conf.toml")).unwrap(),
                // ^^ our switche.conf.toml is at root of project, the include_str macro will load the contents at compile time
                loglevel : RwLock::new (None),
            } ) );
            conf.load();
            conf
        } ) .clone()
    }

    pub const CONF_FILE_NAME  : &'static str = "switche.conf.toml";
    pub const UNKNOWN_EXE_STR : &'static str = "__unknown__";

    pub const SWITCHE_VERSION : &'static str = env!("CARGO_PKG_VERSION");


    fn get_config_file (&self) -> Option<PathBuf> {  //debug! ("get_config_file called");
        let app_dir_loc = get_app_dir() .map (|p| p.join(Self::CONF_FILE_NAME));
        //println! ("app_dir_loc: {:?}", app_dir_loc);
        //win_apis::write_win_dbg_string (&format!("SWITCHE : app_dir_loc: {:?}", &app_dir_loc));
        if app_dir_loc.as_ref() .is_some_and (|p| is_writeable(p)) {
            return app_dir_loc
        }
        let swi_data_dir = tauri::api::path::local_data_dir() .map (|p| p.join("Switche"));
        if swi_data_dir .as_ref() .is_some_and (|p| !p.exists()) {
            let _ = fs::create_dir (swi_data_dir.as_ref().unwrap());
        }
        let swi_data_dir_loc = swi_data_dir .map (|p| p.join(Self::CONF_FILE_NAME));
        //println! ("swi_data_dir_loc: {:?}", swi_data_dir_loc);
        //win_apis::write_win_dbg_string (&format!("SWITCHE : swi_data_dir_loc: {:?}", &swi_data_dir_loc));

        if swi_data_dir_loc .as_ref() .is_some_and (|p| is_writeable(p)) {
            return swi_data_dir_loc
        }
        None
    }
    pub fn get_log_loc (&self) -> Option<PathBuf> {
        if let Some(conf_path) = self.get_config_file() {
            if let Some(conf_loc) = conf_path.parent() {
                return Some(conf_loc.to_path_buf())
        } }
        None
    }


    pub fn trigger_config_file_edit (&self) {
        if let Some(conf_path) = self.get_config_file() {
            let _ = std::process::Command::new("cmd").arg("/c").arg("start").arg(conf_path).spawn();
        }
    }
    pub fn trigger_config_file_reset (&self) {
        self.toml.write().unwrap() .replace (self.default.clone());
        self.write_back_toml();
    }


    pub fn load (&self) {
        if let Some(conf_path) = self.get_config_file().as_ref() {
            if let Ok(cfg_str) = fs::read_to_string(conf_path) {
                if !cfg_str.trim().is_empty() {
                    if let Ok(toml) = DocumentMut::from_str(&cfg_str) {
                        // successfully read and parsed a writeable non-empty toml, we'll use that
                        self.toml.write().unwrap().replace(toml);
                        return
        }   }   }  }
        // there's no writeable location, or the file was empty, or we failed to read or parse it .. load default and write back
        self.trigger_config_file_reset();
    }

    pub fn reload_log_level (&self) {
        // we'll revisit log-subscriber setup in case there was a switch from disabled to enabled
        // (but note we still want to have the initial setup_log_subscriber called direct from main first to keep around the flush guard)
        let _ = self.setup_log_subscriber();
        let log_level = self.get_log_level();
        warn! ("Setting log-level to {:?}", log_level.into_level());
        self.loglevel.write().unwrap().as_ref() .map (|h| {
            h.modify (|f| *f = log_level)
        } );
    }

    pub fn setup_log_subscriber (&self) -> Result <WorkerGuard, ()> {
        // todo .. ^^ prob use actual errors, though little utility here

        if self.check_flag__logging_enabled().not() ||  self.loglevel.read().unwrap().is_some() {
            return Err(())
        }

        if let Some(log_loc) = self.get_log_loc() {

            let log_appender = RollingFileAppender::builder()
                .rotation(Rotation::DAILY)
                .filename_prefix("switche_log")
                .filename_suffix("log")
                .max_log_files(7)
                .build(log_loc)
                .map_err (|_e| ())?;

            let (nb_log_appender, guard) = non_blocking (log_appender);

            let (level_filter, filter_handle) = reload::Layer::new(self.get_log_level());

            *self.loglevel.write().unwrap() = Some(filter_handle);

            let timer = LocalTime::new ( ::time::format_description::parse (
                "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
            ).unwrap() );

            let subscriber = tracing_subscriber::fmt::Layer::new()
                .with_writer(nb_log_appender)
                .with_timer(timer)
                .with_ansi(false)
                .with_filter(level_filter);

            tracing_subscriber::registry().with(subscriber).init();

            return Ok(guard)
        }
        Err(())
    }


    fn write_back_toml (&self) {
        //debug! ("write_back_toml");
        let conf_path = self.get_config_file();
        if conf_path.is_none() { return }
        let _ = fs::write (
            conf_path.as_ref().unwrap(),
            self.toml.read().unwrap().as_ref() .map (|d| d.to_string()).unwrap_or_default()
        );
    }
    fn write_back_toml_if_changed (&self) {
        let conf_path = self.get_config_file();
        if conf_path.is_none() { return }
        let toml_str = self.toml.read().unwrap().as_ref() .map (|d| d.to_string()) .unwrap_or_default();
        let old_toml_str = fs::read_to_string (conf_path.as_ref().unwrap()) .unwrap_or_default();
        if toml_str != old_toml_str {
            let _ = fs::write (conf_path.as_ref().unwrap(), toml_str);
        }
    }
    pub fn deferred_write_back_toml (&self) {
        static dfr_ex: Lazy<DeferredExecutor> = Lazy::new (DeferredExecutor::default);
        let conf = self.clone();
        let action = Arc::new (move || conf.write_back_toml_if_changed());
        dfr_ex .setup_deferred_action (action, time::Duration::from_millis(300));
    }



    fn check_flag (&self, flag_name:&str) -> bool {
        self.toml.read().unwrap().as_ref()
            .and_then (|t| t.get(flag_name))
            .and_then (|t| t.as_bool())
            .unwrap_or (self.default.get(flag_name).unwrap().as_bool().unwrap())
    }
    # [ allow (dead_code) ]
    fn set_flag (&self, flag_name:&str, flag_val:bool) {
        if let Some(toml) = self.toml.write().unwrap().as_mut() {
            toml [flag_name] = toml_edit::value (flag_val);
            self.deferred_write_back_toml();
        }
    }
    # [ allow (dead_code) ]
    fn toggle_flag (&self, flag_name:&str) -> bool {
        // WARNING: this fn is not synchronized, it is NOT suitable for unguarded high-freq or multi-threaded use
        let flag_val = self.check_flag(flag_name);
        self.set_flag(flag_name, flag_val.not());
        flag_val.not()
    }


    fn get_number (&self, key:&str) -> u32 {
        self.toml.read().unwrap().as_ref()
            .and_then (|t| t.get(key))
            .and_then (|t| t.as_integer().map(|n| n as u32))
            .unwrap_or (self.default.get(key).unwrap().as_integer().unwrap() as u32)
    }

    fn get_string (&self, key:&str) -> String {
        self.toml.read().unwrap().as_ref()
            .and_then (|t| t.get(key))
            .and_then (|t| t.as_str()) .map (|s| s.to_string())
            .unwrap_or (self.default.get(key).unwrap().as_str().unwrap().to_string())
    }

    fn get_string_array (&self, key:&str) -> Vec<String> {
        self.toml.read().unwrap() .as_ref()
            .and_then (|t| t.get(key))
            .and_then (|t| t.as_array())
            .map (|t| t.iter() .filter_map (|v| v.as_str().map(|s| s.to_string())) .collect())
            .unwrap_or ( self.default.get(key).unwrap().as_array().unwrap().iter() .map (|v| v.as_str().unwrap().to_string()) .collect() )
    }



    // all the config flags we can check
    pub fn check_flag__auto_hide_enabled          (&self) -> bool { self.check_flag ( "auto_hide_enabled"              ) }
    pub fn check_flag__group_mode_enabled         (&self) -> bool { self.check_flag ( "group_mode_enabled"             ) }
    pub fn check_flag__alt_tab_enabled            (&self) -> bool { self.check_flag ( "alt_tab_enabled"                ) }
    pub fn check_flag__rbtn_scroll_enabled        (&self) -> bool { self.check_flag ( "mouse_right_btn_scroll_enabled" ) }
    pub fn check_flag__restore_window_dimensions  (&self) -> bool { self.check_flag ( "restore_window_dimensions"      ) }
    pub fn check_flag__auto_order_window_groups   (&self) -> bool { self.check_flag ( "auto_order_window_groups"       ) }
    pub fn check_flag__logging_enabled            (&self) -> bool { self.check_flag ( "logging_enabled"                ) }


    pub fn get_log_level (&self) -> LevelFilter {
        if !self.check_flag__logging_enabled() {
            return LevelFilter::OFF;
        }
        match self.get_string("logging_level").as_str() {
            "TRACE" => LevelFilter::TRACE,
            "DEBUG" => LevelFilter::DEBUG,
            //"INFO"  => LevelFilter::INFO,
            "WARN"  => LevelFilter::WARN,
            "ERROR" => LevelFilter::ERROR,
            "OFF"   => LevelFilter::OFF,
            _       => LevelFilter::INFO,
        }
    }


    // only a few flags are settable via code/UI .. (the rest can be changed directly in config file)
    //pub fn set_flag__auto_hide_enabled  (&self, auto_hide:bool) { self.set_flag ( "auto_hide_enabled",  auto_hide ) }
    //pub fn set_flag__group_mode_enabled (&self, grp_mode:bool ) { self.set_flag ( "group_mode_enabled", grp_mode  ) }
    // ^^ disabled since we only want to use the deferred update functionality for these


    pub fn deferred_update_conf__auto_hide_toggle (&self) -> Option<bool> {
        if let Some(toml) = self.toml.write().unwrap().as_mut() { // serves as re-entrancy guard too
            //let new_state = self.toggle_flag("auto_hide_enabled");
            // ^^ cant do this as rust read/write guards (even in the same thread) are not recursion capable and will panic
            if let Some(old_state) = toml["auto_hide_enabled"].as_bool() {
                toml["auto_hide_enabled"] = toml_edit::value (old_state.not());
                self.deferred_write_back_toml();
                return Some(old_state.not())
        }  }
        None
    }

    pub fn get_n_grp_mode_top_recents (&self) -> u32 {
        let ngmtr = self.get_number("number_of_top_recents_in_grouped_mode");
        if ngmtr > 2 { ngmtr } else { 2 }
        // ^^ we enforce a min of 2 as that is necessary to make the basic switch-to-next work (and for scroll across grp logic etc)
    }
    pub fn get_n_grp_mode_last_recents (&self) -> u32 {
        self.get_number("number_of_last_recents_in_grouped_mode")
    }
    pub fn deferred_update_conf__grp_mode (&self, grp_mode:bool) {
        if let Some(toml) = self.toml.write().unwrap().as_mut() {   // serves as re-entrancy guard too
            toml["group_mode_enabled"] = toml_edit::value (grp_mode);
            self.deferred_write_back_toml();
        }
    }



    pub fn read_conf__window_dimensions (&self) -> Option<(i32, i32, i32, i32)> {
        if let Some(toml) = self.toml.read().unwrap().as_ref() {
            if let ( Some(x), Some(y), Some(w), Some(h) ) = (
                toml .get("window_dimensions") .and_then (|t| t.get("location")) .and_then (|t| t.get("x"))  .and_then (|v| v.as_integer()),
                toml .get("window_dimensions") .and_then (|t| t.get("location")) .and_then (|t| t.get("y"))  .and_then (|v| v.as_integer()),
                toml .get("window_dimensions") .and_then (|t| t.get("size")) .and_then (|t| t.get("width"))  .and_then (|v| v.as_integer()),
                toml .get("window_dimensions") .and_then (|t| t.get("size")) .and_then (|t| t.get("height")) .and_then (|v| v.as_integer()),
            ) {
                return Some ( (x as i32, y as i32, w as i32, h as i32) )
        } }
        None
    }
    pub fn update_conf__switche_window (&self, ss:&SwitcheState) {
        info! ("update_conf__switche_window");
        let (conf, ss) = (self.clone(), ss.clone());
        std::thread::spawn ( move || {
            if let Some(ah) = ss.app_handle.read().unwrap().as_ref() {
                if let Some(w) = ah.get_window("main") {
                    if let (Some(p), Some(s)) = (w.outer_position().ok(), w.outer_size().ok()) {
                        // want to confirm its not minimized before we update its dimensions in configs
                        // and despite tauri minimize check (below), we still sometimes get the minimized loc (-32000), so we'll filter those
                        if (w.is_minimized().ok() == Some(true)) || (p.x as i64 == -32000 && p.y as i64 == -32000) {
                            // and we're getting minimized despite setting 'minimizable' as false in tauri.conf.json .. so we'll at least restore it here
                            w.unminimize().ok();
                            // plus, minimize makes window tiny, and that seems to move off ribbon etc .. so we'll also reload the page
                            ss.proc_menu_req__switche_reload();
                            return
                        }
                        let mut toml_guard = conf.toml.write().unwrap();
                        if let Some(toml) = toml_guard.as_mut() {
                            toml ["window_dimensions"] ["location"] ["x"]  = toml_edit::value (p.x as i64);
                            toml ["window_dimensions"] ["location"] ["y"]  = toml_edit::value (p.y as i64);
                            toml ["window_dimensions"] ["size"] ["width"]  = toml_edit::value (s.width  as i64);
                            toml ["window_dimensions"] ["size"] ["height"] = toml_edit::value (s.height as i64);
                            drop(toml_guard);
                            conf.write_back_toml_if_changed();
                        } else {
                            error!("update_conf__switche_window: failed to get window position, size, or toml doc");
                        }
            }  }  }
        } );
    }
    pub fn deferred_update_conf__switche_window (&self, ss:&SwitcheState) {
        static dfr_ex: Lazy<DeferredExecutor> = Lazy::new (DeferredExecutor::default);
        if !self.check_flag__restore_window_dimensions() { return }
        let (conf, ss) = (self.clone(), ss.clone());
        let action = Arc::new ( move || conf.update_conf__switche_window(&ss) );
        dfr_ex .setup_deferred_action (action, time::Duration::from_millis(1000));
    }



    pub fn get_exe_exclusions_list               (&self)  -> Vec<String> { self.get_string_array ("exe_exclusions_list") }
    pub fn get_exe_manual_ordering_seq           (&self)  -> Vec<String> { self.get_string_array ("exe_manual_ordering_seq") }

    pub fn get_switche_invocation_hotkeys        (&self)  -> Vec<String> { self.get_string_array ("switche_invocation_hotkeys") }

    pub fn get_direct_last_window_switch_hotkeys (&self)  -> Vec<String> { self.get_string_array ("last_window_direct_switch_hotkeys") }
    pub fn get_second_last_window_switch_hotkeys (&self)  -> Vec<String> { self.get_string_array ("second_last_window_direct_switch_hotkeys") }
    pub fn get_third_last_window_switch_hotkeys  (&self)  -> Vec<String> { self.get_string_array ("third_last_window_direct_switch_hotkeys") }

    pub fn get_windows_list_snapshot_hotkeys     (&self)  -> Vec<String> { self.get_string_array ("windows_list_snapshot_hotkeys") }
    pub fn get_snap_list_switch_next_hotkeys     (&self)  -> Vec<String> { self.get_string_array ("snap_list_switch_next_hotkeys") }
    pub fn get_snap_list_switch_prev_hotkeys     (&self)  -> Vec<String> { self.get_string_array ("snap_list_switch_prev_hotkeys") }


    pub fn get_direct_app_switch_hotkeys (&self) -> Vec < (String, Option<String>, Option<String>, bool) > {
        self.toml.read().unwrap().as_ref()
            .and_then (|t| t.get("flex_hotkey"))
            .and_then (|t| t.as_array_of_tables())
            .and_then (|arr| arr.iter() .map (|e| {
                let hotkey = e.get("hotkey") .and_then (|v| v.as_str() .map (|v| v.to_string()) );
                let exe    = e.get("exe")    .and_then (|v| v.as_str() .map (|v| v.to_string()) );
                let title  = e.get("title")  .and_then (|v| v.as_str() .map (|v| v.to_string()) );
                let partial = e.get("allow_partial_match") .and_then (|v| v.as_bool()) .unwrap_or(false);
                hotkey .map (|hk| Some ( (hk, exe, title, partial) )) .unwrap_or (None)
            } ) .collect() )
            .unwrap_or_default()
    }



}




impl Default for DeferredExecutor {
    fn default () -> Self { Self::new() }
}
impl DeferredExecutor {

    pub fn new () -> Self {
        Self { deadline: Arc::new (Mutex::new (SystemTime::UNIX_EPOCH)) }
    }
    pub fn reset (&self) -> Self {
        *self.deadline.lock().unwrap() = SystemTime::UNIX_EPOCH;
        self.clone()
    }
    pub fn set_deferral_dur (&self, dur:time::Duration) -> Self {
        if let Some(t) = SystemTime::now().checked_add(dur) {
            *self.deadline.lock().unwrap() = t
        }
        self.clone()
    }
    pub fn add_deferral_dur (&self, dur:time::Duration) -> Self {
        let _ = self.deadline.lock().unwrap().checked_add(dur);
        self.clone()
    }
    pub fn is_reset (&self) -> bool {
        *self.deadline.lock().unwrap() == SystemTime::UNIX_EPOCH
    }
    pub fn is_due (&self) -> bool {
        !self.is_reset() && *self.deadline.lock().unwrap() < SystemTime::now()
    }

    fn check_defered_action (&self, action:Action) {
        // if nobody already completed the action (and reset the time), and its time, we'll trigger it
        // (else, someone else moved the deadline and is waiting for it, so we can exit)
        if self.is_due() {
            let mut deadline = self.deadline.lock().unwrap();
            action();
            *deadline = SystemTime::UNIX_EPOCH;
        } else {
            //debug! ("deadline reset or pushed forward, nothing to do")
        }
    }

    /// Sets up a deferred action which will only trigger after a deferrable deadline has passed.<br>
    /// Any subsequent calls to setup deferred action on this deadline will postpone this deadline (if pending).<br>
    /// When the deadline triggers, (only) the last set action will be executed.<br>
    /// Resetting a pending deadline effectively cancels any pending actions
    pub fn setup_deferred_action (&self, action:Action, delay:time::Duration) {
        // we simply set up a deadline and a thread to wake up and check
        let dfr_ex = self.clone();
        dfr_ex.set_deferral_dur(delay);
        std::thread::spawn ( move || {
            std::thread::sleep (delay);
            dfr_ex.check_defered_action(action);
        } );
    }

}
