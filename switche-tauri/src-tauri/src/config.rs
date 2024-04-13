#![ allow (non_snake_case, non_upper_case_globals) ]

use std::{fs, time};
use std::ops::{Deref, Not};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};
use std::time::SystemTime;

use once_cell::sync::{Lazy, OnceCell};
use tauri::{AppHandle, Manager, Wry};
use toml_edit::DocumentMut;





# [ derive (Debug, Clone) ]
/// Deferred Executor sets up a deferred action until a (pushable/resettable) deadline has passed
pub struct DeferredExecutor {
    deadline : Arc <Mutex <SystemTime>>,
}

/// Action type to pass in to a DeferredExecutor
pub type Action = Arc < dyn Fn() + Send + Sync + 'static >;



impl DeferredExecutor {

    pub fn new () -> Self {
        Self { deadline: Arc::new (Mutex::new (SystemTime::UNIX_EPOCH)) }
    }
    pub fn reset (&self) -> Self {
        *self.deadline.lock().unwrap() = SystemTime::UNIX_EPOCH;
        self.clone()
    }
    pub fn set_deferral_dur (&self, dur:time::Duration) -> Self {
        SystemTime::now() .checked_add(dur) .map (|t| *self.deadline.lock().unwrap() = t);
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
            //println! ("deadline reset or pushed forward, nothing to do")
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




# [ derive (Debug) ]
pub struct _Config {
    pub toml    : RwLock <Option <DocumentMut>>,
    pub default : DocumentMut,
}


# [ derive (Debug, Clone) ]
pub struct Config ( Arc <_Config> );

impl Deref for Config {
    type Target = _Config;
    fn deref (&self) -> &_Config { &self.0 }
}


const CONF_FILE_NAME : &str = "switche.conf.toml";


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
    fs::OpenOptions::new().write(true).create(true).open(path).is_ok()
    // note that ^^ this is similar to 'touch' and will create an empty file if it doesnt exist
}




impl Config {

    pub fn instance () -> Config {
        static INSTANCE: OnceCell <Config> = OnceCell::new();
        INSTANCE .get_or_init ( || {
            let conf = Config ( Arc::new ( _Config {
                toml    : RwLock::new (None),
                default : DocumentMut::from_str (include_str!("switche.conf.toml")).unwrap(),
            } ) );
            conf.load();
            conf
        } ) .clone()
    }


    fn get_config_file (&self) -> Option<PathBuf> {  //println! ("get_config_file");
        let app_dir_loc = get_app_dir() .map (|p| p.join(CONF_FILE_NAME));
        //println! ("app_dir_loc: {:?}", app_dir_loc);
        //win_apis::write_win_dbg_string (&format!("SWITCHE : app_dir_loc: {:?}", &app_dir_loc));
        if app_dir_loc.as_ref() .is_some_and (|p| is_writeable(p)) {
            return app_dir_loc
        }
        let swi_data_dir = tauri::api::path::local_data_dir() .map (|p| p.join("Switche"));
        if swi_data_dir .as_ref() .is_some_and (|p| !p.exists()) {
            let _ = fs::create_dir (swi_data_dir.as_ref().unwrap());
        }
        let swi_data_dir_loc = swi_data_dir .map (|p| p.join(CONF_FILE_NAME));
        //println! ("swi_data_dir_loc: {:?}", swi_data_dir_loc);
        //win_apis::write_win_dbg_string (&format!("SWITCHE : swi_data_dir_loc: {:?}", &swi_data_dir_loc));

        if swi_data_dir_loc .as_ref() .is_some_and (|p| is_writeable(p)) {
            return swi_data_dir_loc
        }
        return None
    }

    pub fn trigger_config_file_edit (&self) {
        if let Some(conf_path) = self.get_config_file() {
            let _ = std::process::Command::new("cmd").arg("/c").arg("start").arg(conf_path).spawn();
        }
    }


    pub fn load (&self) {
        if let Some(conf_path) = self.get_config_file().as_ref() {
            if let Some(cfg_str) = fs::read_to_string(conf_path).ok() {
                if !cfg_str.trim().is_empty() {
                    if let Some(toml) = DocumentMut::from_str(&cfg_str).ok() {
                        // successfully read and parsed a writeable non-empty toml, we'll use that
                        self.toml.write().unwrap().replace(toml);
                        return
            }   }   }
            // a writeable conf file existed, but it was empty, or we failed to read or parse it .. we'll load default and write back
            self.toml.write().unwrap() .replace (self.default.clone());
            self.write_back_toml();
            return
        }
        // there's no writable conf location, we'll just load default
        self.toml.write().unwrap() .replace (self.default.clone());
    }


    fn write_back_toml (&self) {   //println! ("write_back_toml");
        let conf_path = self.get_config_file();
        if conf_path.is_none() { return }
        let _ = fs::write (
            &conf_path.as_ref().unwrap(),
            self.toml.read().unwrap().as_ref() .map (|d| d.to_string()).unwrap_or_default()
        );
    }
    fn write_back_toml_if_changed (&self) {
        let conf_path = self.get_config_file();
        if conf_path.is_none() { return }
        let toml_str = self.toml.read().unwrap().as_ref() .map (|d| d.to_string()) .unwrap_or_default();
        let old_toml_str = fs::read_to_string (&conf_path.as_ref().unwrap()) .unwrap_or_default();
        if toml_str != old_toml_str {
            let _ = fs::write (&conf_path.as_ref().unwrap(), toml_str);
        }
    }
    pub fn deferred_write_back_toml (&self) {
        static dfr_ex: Lazy<DeferredExecutor> = Lazy::new (|| DeferredExecutor::new() );
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
    fn set_flag (&self, flag_name:&str, flag_val:bool) {
        if let Some(toml) = self.toml.write().unwrap().as_mut() {
            toml [flag_name] = toml_edit::value (flag_val);
            self.deferred_write_back_toml();
        }
    }
    fn toggle_flag (&self, flag_name:&str) -> bool {
        // WARNING: this fn is not synchronized, it is NOT suitable for high-frequency or multi-threaded use
        let flag_val = self.check_flag(flag_name);
        self.set_flag(flag_name, flag_val.not());
        flag_val.not()
    }


    // all the config flags we can check
    pub fn check_flag__auto_hide_enabled          (&self) -> bool { self.check_flag ( "auto_hide_enabled"              ) }
    pub fn check_flag__group_mode_enabled         (&self) -> bool { self.check_flag ( "group_mode_enabled"             ) }
    pub fn check_flag__alt_tab_enabled            (&self) -> bool { self.check_flag ( "alt_tab_enabled"                ) }
    pub fn check_flag__rbtn_scroll_enabled        (&self) -> bool { self.check_flag ( "mouse_right_btn_scroll_enabled" ) }
    pub fn check_flag__start_as_admin             (&self) -> bool { self.check_flag ( "start_as_admin"                 ) }
    pub fn check_flag__restore_window_dimensions  (&self) -> bool { self.check_flag ( "restore_window_dimensions"      ) }


    // only a few flags are settable via code/UI .. (the rest can be changed directly in config file)
    pub fn set_flag__auto_hide_enabled  (&self, auto_hide:bool) { self.set_flag ( "auto_hide_enabled",  auto_hide ) }
    //pub fn set_flag__group_mode_enabled (&self, grp_mode:bool ) { self.set_flag ( "group_mode_enabled", grp_mode  ) }
    // ^^ disabled since we only want to use the deferred update functionality for group-mode toggles


    // for some few flags, toggling semantics makes sense (e.g from ui w/o having to know cur state)
    pub fn toggle_flag__auto_hide_enabled (&self) -> bool { self.toggle_flag("auto_hide_enabled") }


    fn get_number (&self, key:&str) -> u32 {
        self.toml.read().unwrap().as_ref()
            .and_then (|t| t.get(key))
            .and_then (|t| t.as_integer().map(|n| n as u32))
            .unwrap_or (self.default.get(key).unwrap().as_integer().unwrap() as u32)
    }

    pub fn get_n_grp_mode_top_recents (&self) -> u32 {
        self.get_number("number_of_top_recents_in_grouped_mode")
    }
    pub fn deferred_update_conf__grp_mode (&self, grp_mode:bool) {
        if let Some(toml) = self.toml.write().unwrap().as_mut() {
            toml["group_mode_is_default"] = toml_edit::value (grp_mode);
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
        return None
    }
    pub fn update_conf__switche_window (&self, ah:&AppHandle<Wry>) { println! ("update_conf__switche_window");
        let (ah, conf) = (ah.clone(), self.clone());
        std::thread::spawn ( move || {
            ah.get_window("main").as_ref() .iter() .for_each (|w| {
                let mut toml_guard = conf.toml.write().unwrap();
                if let (Some(p), Some(s), Some(toml)) = (w.outer_position().ok(), w.outer_size().ok(), toml_guard.as_mut()) {
                    toml ["window_dimensions"] ["location"] ["x"]  = toml_edit::value (p.x as i64);
                    toml ["window_dimensions"] ["location"] ["y"]  = toml_edit::value (p.y as i64);
                    toml ["window_dimensions"] ["size"] ["width"]  = toml_edit::value (s.width  as i64);
                    toml ["window_dimensions"] ["size"] ["height"] = toml_edit::value (s.height as i64);
                    drop(toml_guard);
                    conf.write_back_toml_if_changed();
                } else {
                    eprintln!("update_conf__switche_window: failed to get window position, size, or toml doc");
                }
            } );
        } );
    }
    pub fn deferred_update_conf__switche_window (&self, ah:&AppHandle<Wry>) {
        static dfr_ex: Lazy<DeferredExecutor> = Lazy::new (|| DeferredExecutor::new() );
        if !self.check_flag__restore_window_dimensions() { return }
        let (conf, ah) = (self.clone(), ah.clone());
        let action = Arc::new ( move || conf.update_conf__switche_window(&ah) );
        dfr_ex .setup_deferred_action (action, time::Duration::from_millis(1000));
    }



    fn get_string_array (&self, key:&str) -> Vec<String> {
        self.toml.read().unwrap().as_ref()
            .and_then (|t| t.get(key))
            .and_then (|t| t.as_array())
            .map (|t| t.iter() .map (|v| v.as_str().map(|s| s.to_string())) .flatten() .collect())
            .unwrap_or ( self.default.get(key).unwrap().as_array().unwrap().iter() .map (|v| v.as_str().unwrap().to_string()) .collect() )
    }

    pub fn get_switche_invocation_hotkeys        (&self)  -> Vec<String> { self.get_string_array ("switche_invocation_hotkeys") }
    pub fn get_direct_last_window_switch_hotkeys (&self)  -> Vec<String> { self.get_string_array ("last_window_direct_switch_hotkeys") }
    pub fn get_exe_exclusions_list               (&self)  -> Vec<String> { self.get_string_array ("exe_exclusions_list") }


    pub fn get_direct_app_switch_hotkeys (&self) -> Vec < (String, Option<String>, Option<String>) > {
        self.toml.read().unwrap().as_ref()
            .and_then (|t| t.get("flex_hotkey"))
            .and_then (|t| t.as_array_of_tables())
            .and_then (|arr| arr.iter() .map (|e| {
                let hotkey = e.get("hotkey") .and_then (|v| v.as_str() .map (|v| v.to_string()) );
                let exe    = e.get("exe")    .and_then (|v| v.as_str() .map (|v| v.to_string()) );
                let title  = e.get("title")  .and_then (|v| v.as_str() .map (|v| v.to_string()) );
                hotkey .map (|hk| Some ( (hk, exe, title) )) .unwrap_or (None)
            } ) .collect() )
            .unwrap_or ( vec![] )
    }



}



