#![allow (non_snake_case, non_camel_case_types, dead_code)]

pub mod search;

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{
    div, px, rgb, rgba, AnyElement, AsyncApp, ClickEvent, Context, CursorStyle, FocusHandle,
    FontWeight, ImageSource, KeyDownEvent, MouseButton, MouseDownEvent, Render,
    RenderImage, ScrollDelta, ScrollWheelEvent, SharedString, StyledText, Window,
};

use crate::app_window;
use crate::render_lists::RenderListEntry;
use crate::switche::{
    Backend_Notice, Configs_Pl, Hwnd, SwitcheState, UiEvent, UiRequest, WinDatEntry_Pl,
};

use self::search::{check_search_exe_title, highlight_ranges, split_highlighted};



// ---- colors from the old css ----
const C_BG              : u32 = 0x161616;
const C_RIBBON          : u32 = 0xbd9d2c;
const C_PRIMARY         : u32 = 0xe0d180;
const C_SECONDARY       : u32 = 0xa8c77d;
const C_EXE             : u32 = 0xbbc18f;
const C_SEARCH          : u32 = 0x7aff00;
const C_SEARCH_EDGE     : u32 = 0x7aff00b3;
const C_CUR_TEXT        : u32 = 0xe0f3d9;
const C_CUR_BG          : u32 = 0x49317c;
const C_Y               : u32 = 0x5c9d4d;
const C_Y_CUR           : u32 = 0x92ca7b;
const C_Y_ELEV          : u32 = 0xb9737b;
const C_Y_CUR_ELEV      : u32 = 0xd9a4cd;
const C_MENU_HOVER      : u32 = 0x404040;
const C_TOOLTIP         : u32 = 0xd0c0a0;
const C_TOOLTIP_BG      : u32 = 0x404040;
const C_ALT_TAB         : u32 = 0xbd9d2c;
const C_RBTN_WHL        : u32 = 0xbd9d2c;
const C_ELEV            : u32 = 0x00b000;
const C_ARMED           : u32 = 0xff8080;
const C_WARN            : u32 = 0xd050f0;
const C_DIM             : u32 = 0x808080;
const C_HELP            : u32 = 0x60d060;
const C_SCROLLBAR_BG    : u32 = 0x554400;
const C_SCROLL_THUMB    : u32 = 0x997700;

// ---- layout ----
const ROOT_PAD    : f32 = 4.0;
const BORDER      : f32 = 1.0;
const RIBBON_H    : f32 = 26.0;
const ROW_H       : f32 = 19.0;
const HEADER_H    : f32 = 26.0;   // includes the 8px section top-padding
const SPACER_H    : f32 = 10.0;
const GAP_H       : f32 = 4.0;
const TEXT_SZ     : f32 = 13.0;
const SMALL_SZ    : f32 = 11.0;
const EXE_W       : f32 = 120.0;
const Y_W         : f32 = 14.0;
const ICO_W       : f32 = 22.0;

const MIN_SCROLL_SPACING : Duration = Duration::from_millis (15);



#[derive (Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Block { R, G }

#[derive (Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ElemId { hwnd: Hwnd, block: Block }

#[derive (Debug, Clone, Copy)]
struct OrderedElem { nav_idx: usize, render_y: u32, yg: i32 }

#[derive (Debug, Clone, Copy, PartialEq, Eq)]
enum Grp { NG, LH, GH, GT }

#[derive (Debug, Clone, Copy, PartialEq, Eq)]
enum ElemT { R, G, GR }

#[derive (Debug, Clone)]
struct Row {
    id        : ElemId,
    render_y  : u32,
    grp       : Grp,
    elem_t    : ElemT,
    dimmed    : bool,
    matched   : bool,
}

#[derive (Debug, Clone)]
enum DispItem {
    Header (ElemT),
    Row (Row),
    Spacer,
    Gap,
}
impl DispItem {
    fn height (&self) -> f32 {
        match self {
            DispItem::Header (_) => HEADER_H,
            DispItem::Row (_)    => ROW_H,
            DispItem::Spacer     => SPACER_H,
            DispItem::Gap        => GAP_H,
        }
    }
}

#[derive (Clone, Copy, Debug)]
enum MenuAction { Refresh, Reload, GroupToggle, AutoHideToggle, EditConfig, ResetConfig, Quit }



fn ss () -> &'static SwitcheState { SwitcheState::instance() }

fn decode_icon (ico_str:&str) -> Option <Arc <RenderImage>> {
    let b64 = ico_str.strip_prefix ("data:image/png;base64,")?;
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode (b64) .ok()?;
    let mut data = image::load_from_memory (&bytes) .ok()? .to_rgba8();
    // gpui's image buffers are BGRA (same conversion it does for its own png loading)
    for pixel in data.chunks_exact_mut (4) { pixel.swap (0, 2) }
    let frame = image::Frame::new (data);
    Some (Arc::new (RenderImage::new (smallvec::smallvec![frame])))
}



pub struct SwitcheApp {
    pub focus_handle : FocusHandle,

    hwnd_map             : HashMap <Hwnd, WinDatEntry_Pl>,
    icons                : HashMap <u32, Arc <RenderImage>>,
    render_list          : Vec <RenderListEntry>,
    grouped_render_list  : Vec <Vec <RenderListEntry>>,
    configs              : Configs_Pl,

    group_mode   : bool,
    auto_hide    : bool,
    is_dismissed : bool,
    is_fgnd      : bool,
    scroll_armed : bool,

    in_search          : bool,
    search_text        : String,
    prior_search_text  : String,
    match_count        : usize,

    menu_open    : bool,
    cur_elem     : Option <ElemId>,

    recents_map  : HashMap <ElemId, OrderedElem>,
    grouped_map  : HashMap <ElemId, OrderedElem>,
    recents_ids  : Vec <ElemId>,
    grouped_ids  : Vec <ElemId>,
    group_heads  : Vec <ElemId>,

    display        : Vec <DispItem>,
    display_height : f32,
    scroll_offset  : f32,
    last_viewport_h: f32,
    last_action_stamp : Instant,
}



impl SwitcheApp {

    pub fn new (cx:&mut Context<Self>) -> Self {

        let focus_handle = cx.focus_handle();

        let mut app = SwitcheApp {
            focus_handle,

            hwnd_map             : HashMap::new(),
            icons                : HashMap::new(),
            render_list          : Vec::new(),
            grouped_render_list  : Vec::new(),
            configs              : Configs_Pl::default(),

            group_mode   : true,
            auto_hide    : true,
            is_dismissed : false,
            is_fgnd      : false,
            scroll_armed : false,

            in_search         : false,
            search_text       : String::new(),
            prior_search_text : String::new(),
            match_count       : 0,

            menu_open : false,
            cur_elem  : None,

            recents_map : HashMap::new(),
            grouped_map : HashMap::new(),
            recents_ids : Vec::new(),
            grouped_ids : Vec::new(),
            group_heads : Vec::new(),

            display         : Vec::new(),
            display_height  : 0.0,
            scroll_offset   : 0.0,
            last_viewport_h : 0.0,
            last_action_stamp : Instant::now(),
        };

        app.rebuild_render_state();
        app
    }


    /// Starts draining engine events. Note that this must be called after the window
    /// .. has been fully created, as the foreground task can otherwise be polled while
    /// .. GPUI still holds the app borrow (which panics with a RefCell double borrow)
    pub fn start_event_pump (&mut self, cx:&mut Context<Self>, rx:async_channel::Receiver <UiEvent>) {
        cx.spawn (async move |this, cx:&mut AsyncApp| {
            while let Ok (ev) = rx.recv().await {
                if this.update (cx, |this, cx| this.handle_event (ev, cx)) .is_err() { break }
            }
        }) .detach();
    }



    // ---------------- event handling ----------------

    fn handle_event (&mut self, ev:UiEvent, cx:&mut Context<Self>) {
        match ev {
            UiEvent::Quit => {
                app_window::persist_self_window_dimensions (ss());
                cx.quit();
            }
            UiEvent::Notice (n) => self.handle_notice (n, cx),
            UiEvent::RenderLists (pl) => {
                tracing::debug! ("ui: render list ({} rows / {} groups)", pl.rl.len(), pl.grl.len());
                self.render_list = pl.rl;
                self.grouped_render_list = pl.grl;
                self.rebuild_render_state();
                self.resync_focus_after_rebuild();
                cx.notify();
            }
            UiEvent::WinDatEntry (pl) => {
                tracing::trace! ("ui: win-dat-entry {:?}", pl.hwnd);
                self.hwnd_map.insert (pl.hwnd, pl);
                self.rebuild_render_state();
                self.resync_focus_after_rebuild();
                cx.notify();
            }
            UiEvent::IconEntry (ie) => {
                if let Some (img) = decode_icon (&ie.ico_str) {
                    tracing::trace! ("ui: icon {}", ie.ico_id);
                    self.icons.insert (ie.ico_id as u32, img);
                    cx.notify();
                }
            }
            UiEvent::Configs (conf) => {
                self.configs = conf;
                self.group_mode = self.configs.group_mode_enabled;
                self.auto_hide  = self.configs.auto_hide_enabled;
                self.rebuild_render_state();
                self.resync_focus_after_rebuild();
                cx.notify();
            }
        }
    }


    fn handle_notice (&mut self, n:Backend_Notice, cx:&mut Context<Self>) {
        use Backend_Notice::*;
        match n {
            backend_req__app_invoke => {
                self.handle_invoke();
            }
            backend_req__scroll_down => {
                if !self.verify_spacing() { return }
                self.scroll_armed = true;
                self.handle_invoke();
            }
            backend_req__scroll_up => {
                if !self.verify_spacing() { return }
                self.scroll_armed = true;
                if self.is_dismissed || !self.is_fgnd {
                    self.is_dismissed = false; self.is_fgnd = true;
                    self.reset_focus();
                    self.focus_prev();
                } else {
                    self.focus_prev();
                }
            }
            backend_req__scroll_end => {
                if !self.is_dismissed && self.is_fgnd && self.scroll_armed { self.activate_cur() }
            }
            backend_req__scroll_end_disarm => {
                if !self.is_dismissed && self.is_fgnd && self.scroll_armed { self.scroll_armed = false }
            }
            backend_req__switche_escape => {
                self.scroll_armed = false;
                self.is_dismissed = true;
                self.menu_open = false;
                self.exit_search (false);
            }
            backend_req__switche_reload => {
                self.handle_reload (cx);
            }
            switche_event__in_fgnd => {
                self.is_fgnd = true;
                self.is_dismissed = false;
            }
            switche_event__fgnd_lost => {
                self.is_fgnd = false;
                self.scroll_armed = false;
                if self.auto_hide { self.is_dismissed = true }
                self.exit_search (false);
            }
        }
        cx.notify();
    }


    fn handle_invoke (&mut self) {
        if self.is_dismissed || !self.is_fgnd {
            self.is_dismissed = false;
            self.is_fgnd = true;
            self.reset_focus();
        } else {
            self.focus_next();
        }
    }


    fn verify_spacing (&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since (self.last_action_stamp) < MIN_SCROLL_SPACING { return false }
        self.last_action_stamp = now;
        true
    }



    // ---------------- key handling ----------------

    fn handle_key_down (&mut self, ev:&KeyDownEvent, _window:&mut Window, cx:&mut Context<Self>) {
        let ks    = &ev.keystroke;
        let key   = ks.key.as_str();
        let ctrl  = ks.modifiers.control;
        let alt   = ks.modifiers.alt;
        let shift = ks.modifiers.shift;

        if matches! (key, "shift" | "control" | "alt" | "platform" | "function" | "capslock" | "numlock") { return }

        let mut handled = true;

        if key == "enter" {
            self.activate_cur();
        } else if key == "escape" {
            if self.in_search { self.exit_search (false) } else { self.do_escape() }
        } else if key == "f5" {
            self.handle_reload (cx);
        } else if key == "f3" {
            if self.in_search { self.focus_next() } else { self.reactivate_last_search() }
        } else if key == "f1" || key == "f15" {
            self.focus_next();
        } else if key == "f2" {
            self.focus_prev();
        } else if key == "f16" {
            self.scroll_armed = true;
            if shift { self.focus_prev() } else { self.focus_next() }
        } else if key == "f17" {
            self.scroll_armed = true;
            if shift { self.focus_next() } else { self.focus_prev() }
        } else if key == "tab" {
            if shift { self.focus_prev() } else { self.focus_next() }
        } else if key == "up" {
            if shift { self.focus_prev() } else { self.focus_block_prev() }
        } else if key == "down" {
            if shift { self.focus_next() } else { self.focus_block_next() }
        } else if key == "pageup" {
            if shift { self.focus_top() } else { self.focus_block_top() }
        } else if key == "pagedown" {
            if shift { self.focus_bottom() } else { self.focus_block_bottom() }
        } else if key == "left" && (!self.in_search || self.scroll_armed || alt) {
            self.focus_group_prev();
        } else if key == "right" && (!self.in_search || self.scroll_armed || alt) {
            self.focus_group_next();
        } else if key == "space" {
            if self.scroll_armed { self.scroll_armed = false }
            else if alt { self.activate_cur() }
            else if !self.in_search { self.activate_cur() }
            else if ctrl { self.activate_cur() }
            else { self.insert_search_text (" ") }
        } else if alt && key == "f4" {
            self.do_quit (cx);
        } else if alt && ctrl && key == "a" {
            self.toggle_auto_hide();
        } else if alt || self.scroll_armed {
            match key {
                "i" => if shift { self.focus_prev() }       else { self.focus_block_prev() },
                "," => if shift { self.focus_next() }       else { self.focus_block_next() },
                "u" => if shift { self.focus_top() }        else { self.focus_block_top() },
                "m" => if shift { self.focus_bottom() }     else { self.focus_block_bottom() },
                "j" => self.focus_group_prev(),
                "k" => self.focus_group_next(),
                "r" => self.scroll_armed = false,
                "s" | "l" => self.scroll_armed = false,
                _ => handled = false,
            }
        } else if ctrl {
            match key {
                "r" | "f" => self.handle_refresh(),
                "g" => self.toggle_group_mode(),
                "w" => self.close_cur(),
                "p" => self.peek_cur(),
                _ => handled = false,
            }
        } else if key == "backspace" {
            if self.in_search {
                self.search_text.pop();
                if self.search_text.trim().is_empty() { self.exit_search (true) }
                else { self.on_search_text_changed() }
            } else { handled = false }
        } else if let Some (ch) = ks.key_char.as_ref().filter (|c| !c.is_empty()) {
            self.insert_search_text (ch);
        } else {
            handled = false;
        }

        if handled { cx.notify() }
    }


    fn insert_search_text (&mut self, s:&str) {
        self.in_search = true;
        self.search_text.push_str (s);
        self.prior_search_text = self.search_text.trim().to_string();
        self.rebuild_render_state();
        self.reset_search_match_focus();
    }
    fn on_search_text_changed (&mut self) {
        self.prior_search_text = self.search_text.trim().to_string();
        self.rebuild_render_state();
        self.reset_search_match_focus();
    }
    fn reactivate_last_search (&mut self) {
        self.in_search = true;
        self.search_text = self.prior_search_text.clone();
        self.rebuild_render_state();
        self.reset_search_match_focus();
    }
    fn exit_search (&mut self, clear_prior:bool) {
        let was_searching = self.in_search || !self.search_text.is_empty();
        self.in_search = false;
        self.search_text.clear();
        self.match_count = 0;
        if clear_prior { self.prior_search_text.clear() }
        if was_searching {
            self.rebuild_render_state();
            self.resync_focus_after_rebuild();
        }
    }



    // ---------------- actions ----------------

    fn do_escape (&mut self) {
        self.scroll_armed = false;
        self.is_dismissed = true;
        self.menu_open = false;
        self.exit_search (false);
        ss().handle_ui_request (UiRequest::SwitcheEscape);
    }
    fn do_quit (&mut self, cx:&mut Context<Self>) {
        app_window::persist_self_window_dimensions (ss());
        cx.quit();
    }
    fn activate_cur (&mut self) {
        self.scroll_armed = false;
        self.is_dismissed = true;
        self.menu_open = false;
        if let Some (id) = self.cur_elem {
            ss().handle_ui_request (UiRequest::WindowActivate (id.hwnd));
        }
    }
    fn close_cur (&mut self) {
        if let Some (id) = self.cur_elem {
            if let Some (next) = self.pick_next_elem (false, false, true) { self.set_cur (Some (next)) }
            ss().handle_ui_request (UiRequest::WindowClose (id.hwnd));
        }
    }
    fn peek_cur (&mut self) {
        if let Some (id) = self.cur_elem { ss().handle_ui_request (UiRequest::WindowPeek (id.hwnd)) }
    }
    fn handle_refresh (&mut self) {
        ss().handle_ui_request (UiRequest::Refresh);
    }
    fn handle_reload (&mut self, cx:&mut Context<Self>) {
        self.in_search = false;
        self.search_text.clear();
        self.prior_search_text.clear();
        self.match_count = 0;
        self.menu_open = false;
        self.cur_elem = None;
        self.scroll_offset = 0.0;
        ss().handle_ui_request (UiRequest::DataLoad);
        cx.notify();
    }
    fn toggle_group_mode (&mut self) {
        self.group_mode = !self.group_mode;
        ss().handle_ui_request (if self.group_mode { UiRequest::GrpModeEnable } else { UiRequest::GrpModeDisable });
        self.rebuild_render_state();
        self.resync_focus_after_rebuild();
    }
    fn toggle_auto_hide (&mut self) {
        self.auto_hide = !self.auto_hide;
        ss().handle_ui_request (if self.auto_hide { UiRequest::AutoHideEnable } else { UiRequest::AutoHideDisable });
    }
    fn do_menu_action (&mut self, action:MenuAction, cx:&mut Context<Self>) {
        self.menu_open = false;
        match action {
            MenuAction::Refresh       => self.handle_refresh(),
            MenuAction::Reload        => self.handle_reload (cx),
            MenuAction::GroupToggle   => self.toggle_group_mode(),
            MenuAction::AutoHideToggle=> self.toggle_auto_hide(),
            MenuAction::EditConfig    => { ss().handle_ui_request (UiRequest::EditConfig) }
            MenuAction::ResetConfig   => { ss().handle_ui_request (UiRequest::ResetConfig) }
            MenuAction::Quit          => self.do_quit (cx),
        }
        cx.notify();
    }



    // ---------------- focus / nav ----------------

    fn reset_focus (&mut self) {
        self.menu_open = false;
        self.cur_elem = self.recents_ids.first().copied();
        self.focus_next();
    }
    fn reset_search_match_focus (&mut self) {
        let block = if self.group_mode { ElemT::G } else { ElemT::R };
        let id = self.block_ids (block) .first().copied();
        self.set_cur (id);
    }
    fn set_cur (&mut self, id:Option<ElemId>) {
        self.cur_elem = id;
        self.ensure_visible();
    }

    fn block_ids (&self, t:ElemT) -> &Vec <ElemId> {
        if t == ElemT::G { &self.grouped_ids } else { &self.recents_ids }
    }
    fn block_map (&self, t:ElemT) -> &HashMap <ElemId, OrderedElem> {
        if t == ElemT::G { &self.grouped_map } else { &self.recents_map }
    }

    fn pick_in (&self, oe:OrderedElem, cur_block:ElemT, wrap_block:ElemT, reverse:bool) -> Option <ElemId> {
        let cur_ids  = self.block_ids (cur_block);
        let cur_map  = self.block_map (cur_block);
        let wrap_ids = self.block_ids (wrap_block);
        let wrap_map = self.block_map (wrap_block);

        let next_idx = if reverse { oe.nav_idx.checked_sub (1) } else { Some (oe.nav_idx + 1) };
        let next = next_idx .and_then (|i| cur_ids.get(i).copied()) .filter (|id| cur_map.contains_key (id));

        next .or_else (|| {
            let id = if reverse { wrap_ids.last().copied() } else { wrap_ids.first().copied() };
            id.filter (|id| wrap_map.contains_key (id))
        })
    }

    fn pick_next_elem (&self, reverse:bool, grp_next:bool, wrap_blocks:bool) -> Option <ElemId> {
        let cur = self.cur_elem?;
        let (cur_in_grouped, oe) = if let Some (oe) = self.recents_map.get (&cur) { (false, *oe) }
            else if let Some (oe) = self.grouped_map.get (&cur) { (true, *oe) }
            else { return None };

        if !self.group_mode {
            return self.pick_in (oe, ElemT::R, ElemT::R, reverse)
        }
        if self.in_search {
            return self.pick_in (oe, ElemT::G, ElemT::G, reverse)
        }
        if !grp_next {
            return if wrap_blocks {
                if !cur_in_grouped { self.pick_in (oe, ElemT::R, ElemT::G, reverse) }
                else { self.pick_in (oe, ElemT::G, ElemT::R, reverse) }
            } else if !cur_in_grouped {
                self.pick_in (oe, ElemT::R, ElemT::R, reverse)
            } else {
                self.pick_in (oe, ElemT::G, ElemT::G, reverse)
            }
        }

        if !cur_in_grouped {
            if self.recents_ids.first() == Some (&cur) {
                let id = if reverse { self.group_heads.last() } else { self.group_heads.first() };
                return id.copied().filter (|id| self.grouped_map.contains_key (id))
            } else if reverse {
                return self.recents_ids.first().copied().filter (|id| self.recents_map.contains_key (id))
            } else {
                return self.group_heads.first().copied().filter (|id| self.grouped_map.contains_key (id))
            }
        }

        let head_idx = oe.yg.max (0) as usize;
        let at_grp_head = self.group_heads.get (head_idx) == Some (&cur);
        if at_grp_head || !reverse {
            if reverse {
                if oe.yg > 0 {
                    if let Some (id) = self.group_heads.get ((oe.yg - 1) as usize) {
                        return Some (*id)
                    }
                }
            } else if let Some (id) = self.group_heads.get ((oe.yg + 1) as usize) {
                return Some (*id)
            }
            return self.recents_ids.first().copied().filter (|id| self.recents_map.contains_key (id))
        }
        self.group_heads.get (head_idx).copied()
    }

    fn focus_next (&mut self)       { let id = self.pick_next_elem (false, false, true); self.set_cur (id) }
    fn focus_prev (&mut self)       { let id = self.pick_next_elem (true,  false, true); self.set_cur (id) }
    fn focus_group_next (&mut self) { let id = self.pick_next_elem (false, true,  true); self.set_cur (id) }
    fn focus_group_prev (&mut self) { let id = self.pick_next_elem (true,  true,  true); self.set_cur (id) }
    fn focus_block_next (&mut self) { let id = self.pick_next_elem (false, false, false); self.set_cur (id) }
    fn focus_block_prev (&mut self) { let id = self.pick_next_elem (true,  false, false); self.set_cur (id) }

    fn focus_topbtm (&mut self, to_top:bool, within_block:bool) {
        let cur_in_grouped = self.cur_elem.map (|id| self.grouped_map.contains_key (&id)) .unwrap_or (false);
        let use_grouped = self.group_mode && (
            self.in_search || (within_block && cur_in_grouped) || (!within_block && !to_top)
        );
        let id = if use_grouped {
            if to_top { self.grouped_ids.first().copied() } else { self.grouped_ids.last().copied() }
        } else {
            if to_top { self.recents_ids.first().copied() } else { self.recents_ids.last().copied() }
        };
        self.set_cur (id);
    }
    fn focus_top (&mut self)        { self.focus_topbtm (true,  false) }
    fn focus_bottom (&mut self)     { self.focus_topbtm (false, false) }
    fn focus_block_top (&mut self)  { self.focus_topbtm (true,  true)  }
    fn focus_block_bottom (&mut self) { self.focus_topbtm (false, true) }

    fn sync_elem (&self, cur:Option<ElemId>, block:ElemT, wrap:ElemT) -> Option <ElemId> {
        let hwnd = cur.map (|id| id.hwnd);
        let ids  = self.block_ids (block);
        let map  = self.block_map (block);
        if let Some (hwnd) = hwnd {
            let found = ids.iter().copied().find (|id|
                id.hwnd == hwnd && map.get (id) .map (|oe| oe.nav_idx != usize::MAX) .unwrap_or (false)
            );
            if found.is_some() { return found }
        }
        let wids = self.block_ids (wrap);
        let wmap = self.block_map (wrap);
        wids.first().copied() .filter (|id| wmap.get (id) .map (|oe| oe.nav_idx != usize::MAX) .unwrap_or (false))
    }

    fn resync_focus_after_rebuild (&mut self) {
        if !(self.is_fgnd && !self.is_dismissed) {
            self.reset_focus();
            return
        }
        let cur = self.cur_elem;
        let synced = if !self.group_mode {
            self.sync_elem (cur, ElemT::R, ElemT::R)
        } else if self.in_search {
            self.sync_elem (cur, ElemT::G, ElemT::G)
        } else {
            cur.filter (|id| self.recents_map.contains_key (id))
                .or_else (|| self.sync_elem (cur, ElemT::G, ElemT::R))
        };
        self.cur_elem = synced;
        self.ensure_visible();
    }

    fn ensure_visible (&mut self) {
        let Some (cur) = self.cur_elem else { return };
        let mut top = 0.0;
        let mut target = None;
        for item in &self.display {
            let h = item.height();
            if let DispItem::Row (r) = item {
                if r.id == cur { target = Some ((top, top + h)); break }
            }
            top += h;
        }
        let Some ((row_top, row_btm)) = target else { return };
        let vh = self.last_viewport_h;
        if vh <= 0.0 { return }
        let thresh = 8.0;
        if row_top < self.scroll_offset + thresh {
            self.scroll_offset = (row_top - thresh).max (0.0);
        } else if row_btm > self.scroll_offset + vh - thresh {
            self.scroll_offset = row_btm - vh + thresh;
        }
        let max = (self.display_height - vh).max (0.0);
        self.scroll_offset = self.scroll_offset.clamp (0.0, max);
    }

    fn recompute_display_height (&mut self) {
        self.display_height = self.display.iter().map (|i| i.height()).sum();
        let max = (self.display_height - self.last_viewport_h.max (0.0)) .max (0.0);
        self.scroll_offset = self.scroll_offset.clamp (0.0, max);
    }



    // ---------------- building lists ----------------

    fn get_capped_recents (&self) -> Vec <(RenderListEntry, Grp)> {
        let capped : Vec <RenderListEntry> = if self.group_mode {
            let nfirst = self.configs.n_grp_mode_top_recents as usize;
            let nlast  = self.configs.n_grp_mode_last_recents as usize;
            let ngap = self.render_list.len() as isize - nfirst as isize - nlast as isize;
            let mut v = self.render_list.iter().take (nfirst).cloned().collect::<Vec<_>>();
            v.extend (self.render_list.iter().skip (nfirst).skip (ngap.max (0) as usize).cloned());
            v
        } else {
            self.render_list.clone()
        };

        let mut last_y = capped.first().map (|r| r.y).unwrap_or (0);
        capped.into_iter().map (|rle| {
            let grp = if rle.y > last_y + 1 { Grp::LH } else { Grp::NG };
            last_y = rle.y;
            (rle, grp)
        }) .collect()
    }

    fn rebuild_render_state (&mut self) {
        self.recents_map.clear();
        self.grouped_map.clear();
        self.recents_ids.clear();
        self.grouped_ids.clear();
        self.group_heads.clear();

        let mut recents_rows : Vec <Row> = Vec::new();
        let mut grouped_rows : Vec <Row> = Vec::new();

        if self.in_search {
            self.build_search_rows (&mut recents_rows, &mut grouped_rows);
        } else {
            self.build_normal_rows (&mut recents_rows, &mut grouped_rows);
        }

        self.display.clear();
        if self.in_search {
            if self.group_mode {
                self.push_section (ElemT::GR, recents_rows);
                self.push_section (ElemT::G,  grouped_rows);
            } else {
                self.push_section (ElemT::R, recents_rows);
            }
        } else if self.group_mode {
            self.push_section (ElemT::GR, recents_rows);
            self.push_section (ElemT::G,  grouped_rows);
        } else {
            self.push_section (ElemT::R, recents_rows);
        }
        self.recompute_display_height();
    }

    fn push_section (&mut self, elem_t:ElemT, rows:Vec <Row>) {
        self.display.push (DispItem::Header (elem_t));
        let mut spacer_inserted = false;
        for r in rows {
            // the first row of the capped-recents' last-recents block is tagged LH
            // .. and the old ui separated that block from the top-recents with a small gap
            if !spacer_inserted && r.grp == Grp::LH {
                self.display.push (DispItem::Spacer);
                spacer_inserted = true;
            }
            self.display.push (DispItem::Row (r))
        }
        self.display.push (DispItem::Gap);
    }

    fn build_normal_rows (&mut self, recents_rows:&mut Vec <Row>, grouped_rows:&mut Vec <Row>) {
        let capped = self.get_capped_recents();
        for (i, (rle, grp)) in capped.iter().enumerate() {
            if !self.hwnd_map.contains_key (&rle.hwnd) { continue }
            let id = ElemId { hwnd: rle.hwnd, block: Block::R };
            let elem_t = if self.group_mode { ElemT::GR } else { ElemT::R };
            self.recents_map.insert (id, OrderedElem { nav_idx: i, render_y: rle.y, yg: -1 });
            self.recents_ids.push (id);
            recents_rows.push (Row { id, render_y: rle.y, grp: *grp, elem_t, dimmed: false, matched: false });
        }

        if self.group_mode {
            let mut nav_i = 0;
            for (gi, ll) in self.grouped_render_list.iter().enumerate() {
                let mut first = true;
                for rle in ll {
                    if !self.hwnd_map.contains_key (&rle.hwnd) { continue }
                    let id = ElemId { hwnd: rle.hwnd, block: Block::G };
                    let grp = if first { Grp::GH } else { Grp::GT };
                    if first { self.group_heads.push (id) }
                    first = false;
                    self.grouped_map.insert (id, OrderedElem { nav_idx: nav_i, render_y: rle.y, yg: gi as i32 });
                    self.grouped_ids.push (id);
                    grouped_rows.push (Row { id, render_y: rle.y, grp, elem_t: ElemT::G, dimmed: false, matched: false });
                    nav_i += 1;
                }
            }
        }
    }

    fn build_search_rows (&mut self, recents_rows:&mut Vec <Row>, grouped_rows:&mut Vec <Row>) {
        let capped = self.get_capped_recents();
        let mut match_count = 0;

        if !self.group_mode {
            let mut nav_i = 0;
            for (_i,(rle,grp)) in capped.iter().enumerate() {
                let Some (wde) = self.hwnd_map.get (&rle.hwnd) else { continue };
                let check = check_search_exe_title (wde, &self.search_text, rle.y);
                let id = ElemId { hwnd: rle.hwnd, block: Block::R };
                let nav_idx = if check.passed { let n = nav_i; nav_i += 1; n } else { usize::MAX };
                self.recents_map.insert (id, OrderedElem { nav_idx, render_y: rle.y, yg: -1 });
                if check.passed { self.recents_ids.push (id); match_count += 1 }
                recents_rows.push (Row { id, render_y: rle.y, grp: *grp, elem_t: ElemT::R, dimmed: false, matched: check.passed });
            }
        } else {
            for (gi, ll) in self.grouped_render_list.iter().enumerate() {
                let mut first = true;
                for rle in ll {
                    let Some (wde) = self.hwnd_map.get (&rle.hwnd) else { continue };
                    let check = check_search_exe_title (wde, &self.search_text, rle.y);
                    let id = ElemId { hwnd: rle.hwnd, block: Block::G };
                    let grp = if first { Grp::GH } else { Grp::GT };
                    let nav_idx = if check.passed { let n = self.grouped_ids.len(); n } else { usize::MAX };
                    if check.passed { self.grouped_ids.push (id); match_count += 1 }
                    first = false;
                    self.grouped_map.insert (id, OrderedElem { nav_idx, render_y: rle.y, yg: gi as i32 });
                    grouped_rows.push (Row { id, render_y: rle.y, grp, elem_t: ElemT::G, dimmed: false, matched: check.passed });
                }
            }
            for (i,(rle,grp)) in capped.iter().enumerate() {
                let Some (wde) = self.hwnd_map.get (&rle.hwnd) else { continue };
                let check = check_search_exe_title (wde, &self.search_text, rle.y);
                let id = ElemId { hwnd: rle.hwnd, block: Block::R };
                self.recents_map.insert (id, OrderedElem { nav_idx: i, render_y: rle.y, yg: -1 });
                self.recents_ids.push (id);
                recents_rows.push (Row { id, render_y: rle.y, grp: *grp, elem_t: ElemT::GR, dimmed: true, matched: check.passed });
            }
        }

        self.match_count = match_count;
    }



    // ---------------- wheel ----------------

    fn handle_wheel (&mut self, ev:&ScrollWheelEvent, cx:&mut Context<Self>) {
        let (dy, dx) = match ev.delta {
            ScrollDelta::Pixels (p) => (f32::from(p.y), f32::from(p.x)),
            ScrollDelta::Lines  (p) => (p.y, p.x),
        };
        if dy == 0.0 && dx == 0.0 { return }
        if !self.verify_spacing() { return }

        let up = if ev.modifiers.shift && dx != 0.0 { dx > 0.0 } else { dy > 0.0 };
        // note: gpui reports wheel-up as positive y, while the old web deltaY was positive for wheel-down

        if ev.modifiers.control {
            if up { self.focus_group_prev() } else { self.focus_group_next() }
        } else if ev.modifiers.shift {
            if up { self.focus_block_prev() } else { self.focus_block_next() }
        } else if up {
            self.focus_prev()
        } else {
            self.focus_next()
        }
        cx.notify();
    }



    // ---------------- rendering ----------------

    fn render_header (&self, elem_t:ElemT) -> AnyElement {
        let (text, color) = match elem_t {
            ElemT::G  => ("Grouped:", C_PRIMARY),
            ElemT::GR => ("Recents:", C_SECONDARY),
            ElemT::R  => ("Recents:", C_PRIMARY),
        };
        div()
            .h(px(HEADER_H)).pt(px(8.)).pl(px(4.))
            .text_size(px(TEXT_SZ)).text_color(rgb(color))
            .child(text.to_string())
            .into_any_element()
    }

    fn render_row (&self, row:&Row, window:&Window, cx:&mut Context<Self>) -> AnyElement {
        let Some (wde) = self.hwnd_map.get (&row.id.hwnd) else {
            return div().h(px(ROW_H)).into_any_element()
        };

        let id = row.id;
        let selected = self.cur_elem == Some (id);

        let check = if self.in_search {
            Some (check_search_exe_title (wde, &self.search_text, row.render_y))
        } else { None };

        let title = wde.win_text.clone().unwrap_or_else (|| "-- no title --".to_string());
        let exe   = wde.exe_path_name.as_ref().map (|p| p.name.clone()).unwrap_or_else (|| "exe..".to_string());
        let y_str = format!("{:>2}", row.render_y);

        let title_color = if selected { C_CUR_TEXT }
            else if row.elem_t == ElemT::GR { C_SECONDARY } else { C_PRIMARY };
        let exe_color = if selected { C_CUR_TEXT }
            else if row.grp == Grp::GT { C_DIM } else { C_EXE };
        let y_color = if selected {
            if wde.is_elevated { C_Y_CUR_ELEV } else { C_Y_CUR }
        } else if wde.is_elevated { C_Y_ELEV } else { C_Y };

        let title_el = match &check {
            Some (c) if c.passed => {
                let ranges = highlight_ranges (&title, &c.frags);
                highlighted_text (window, &title, &ranges, title_color, C_SEARCH, TEXT_SZ, true)
            }
            _ => div().text_size(px(TEXT_SZ)).text_color(rgb(title_color)).child(title).into_any_element(),
        };
        let exe_el = match &check {
            Some (c) if c.passed && !c.exe_matched_frags.is_empty() => {
                let ranges = highlight_ranges (&exe, &c.exe_matched_frags);
                highlighted_text (window, &exe, &ranges, exe_color, C_SEARCH, TEXT_SZ, false)
            }
            _ => div().text_size(px(TEXT_SZ)).text_color(rgb(exe_color)).child(exe).into_any_element(),
        };
        let y_el = if check.as_ref().map (|c| c.passed && c.y_matched).unwrap_or (false) {
            div().flex_none().text_size(px(SMALL_SZ)).font_weight(FontWeight::BOLD).text_color(rgb(C_SEARCH)).child(y_str).into_any_element()
        } else {
            div().flex_none().text_size(px(SMALL_SZ)).font_weight(FontWeight::BOLD).text_color(rgb(y_color)).child(y_str).into_any_element()
        };

        let ico_el = match self.icons.get (&wde.icon_cache_idx) {
            Some (img) => gpui::img (ImageSource::Render (img.clone())) .h(px(16.)) .into_any_element(),
            None => div().h(px(16.)).into_any_element(),
        };

        let mut row_div = div()
            .flex().flex_row().items_center().h(px(ROW_H))
            .cursor(CursorStyle::ContextualMenu);
        if selected { row_div = row_div.bg(rgb(C_CUR_BG)) }

        let mut ico_holder = div().w(px(ICO_W)).flex_none().h(px(16.))
            .flex().items_center().child(ico_el);
        if check.as_ref().map (|c| c.passed).unwrap_or (false) {
            ico_holder = ico_holder.border_r_2().border_color(rgba(C_SEARCH_EDGE));
        }

        row_div
            .on_mouse_down (MouseButton::Left, cx.listener (move |this, _ev:&MouseDownEvent, _window, cx| {
                this.cur_elem = Some (id);
                this.activate_cur();
                cx.notify();
            }))
            .on_mouse_down (MouseButton::Middle, cx.listener (move |this, _ev:&MouseDownEvent, _window, cx| {
                this.cur_elem = Some (id);
                this.close_cur();
                cx.notify();
            }))
            .id (SharedString::from (format!("row-{}-{:?}", id.hwnd.0, id.block)))
            .on_hover (cx.listener (move |this, hovered:&bool, _window, cx| {
                if *hovered && this.cur_elem != Some (id) {
                    this.set_cur (Some (id));
                    cx.notify();
                }
            }))
            .child (
                div().w(px(EXE_W)).flex_none().flex().flex_row().justify_end()
                    .overflow_hidden().whitespace_nowrap()
                    .child(exe_el)
            )
            .child (div().w(px(8.)).flex_none())
            .child (
                div().w(px(Y_W)).flex_none().flex().flex_row().justify_end()
                    .child(y_el)
            )
            .child (div().w(px(8.)).flex_none())
            .child (ico_holder)
            .child (div().w(px(4.)).flex_none())
            .child (
                div().flex_1().min_w(px(0.)).overflow_hidden().whitespace_nowrap()
                    .child(title_el)
            )
            .when (row.dimmed, |d| d.opacity(0.8))
            .into_any_element()
    }

    fn render_menu_link (&self, cx:&mut Context<Self>) -> AnyElement {
        div()
            .w(px(36.)).h(px(RIBBON_H))
            .flex().flex_col().justify_center().items_center().gap(px(3.))
            .cursor(CursorStyle::PointingHand)
            .on_mouse_down (MouseButton::Left, cx.listener (|this, _ev:&MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                this.menu_open = !this.menu_open;
                cx.notify();
            }))
            .child (div().w(px(16.)).h(px(2.)).bg(rgb(C_RIBBON)))
            .child (div().w(px(16.)).h(px(2.)).bg(rgb(C_RIBBON)))
            .child (div().w(px(16.)).h(px(2.)).bg(rgb(C_RIBBON)))
            .into_any_element()
    }

    fn indicator (&self, symbol:&str, on:bool, on_color:u32) -> AnyElement {
        let color = if on { on_color } else { C_DIM };
        div()
            .w(px(32.)).flex_none().flex().justify_center()
            .child (
                div()
                    .w(px(14.)).h(px(12.))
                    .border_1().border_color(rgb(color))
                    .text_size(px(SMALL_SZ)).font_weight(FontWeight::BOLD).text_color(rgb(color))
                    .flex().items_center().justify_center()
                    .child(symbol.to_string())
            )
            .into_any_element()
    }

    fn render_drag_indicator (&self, cx:&mut Context<Self>) -> AnyElement {
        div()
            .w(px(32.)).flex_none().flex().justify_center()
            .text_size(px(14.)).text_color(rgb(C_RIBBON))
            .cursor(CursorStyle::OpenHand)
            .on_mouse_down (MouseButton::Left, |_ev:&MouseDownEvent, window, _cx| {
                window.start_window_move();
            })
            .id (SharedString::from ("drag-indicator"))
            .on_click (cx.listener (|_this, ev:&ClickEvent, _window, _cx| {
                if ev.click_count() == 2 { ss().handle_ui_request (UiRequest::SelfAutoResize) }
            }))
            .child("✥")
            .into_any_element()
    }

    fn render_search (&self) -> AnyElement {
        let count = if self.match_count > 0 { self.match_count.to_string() } else { String::new() };
        div()
            .flex_1().flex().flex_row().items_center().pl(px(12.))
            .child (
                div().flex_1().text_size(px(17.)).text_color(rgb(C_SEARCH))
                    .child(self.search_text.clone())
            )
            .child (
                div().flex_none().pr(px(4.)).text_size(px(SMALL_SZ)).text_color(rgba(C_SEARCH_EDGE))
                    .child(count)
            )
            .into_any_element()
    }

    fn menu_item (&self, label:&str, hotkey:&str, action:MenuAction, cx:&mut Context<Self>) -> AnyElement {
        div()
            .flex().flex_row().items_center()
            .px(px(12.)).py(px(5.))
            .text_size(px(12.)).text_color(rgb(C_RIBBON))
            .cursor(CursorStyle::PointingHand)
            .hover (|s| s.bg(rgb(C_MENU_HOVER)))
            .on_mouse_down (MouseButton::Left, cx.listener (move |this, _ev:&MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                this.do_menu_action (action, cx);
            }))
            .child (div().flex_1().child(label.to_string()))
            .child (div().text_size(px(10.)).text_color(rgba(0xbd9d2ce0)).child(hotkey.to_string()))
            .into_any_element()
    }

    fn render_dropdown (&self, cx:&mut Context<Self>) -> AnyElement {
        div()
            // rendered as a root-level absolute child so it paints above the list area
            .absolute().top(px(ROOT_PAD + RIBBON_H)).left(px(12.))
            .w(px(150.)).pb(px(10.))
            .bg(rgb(C_BG)).shadow_lg()
            .flex().flex_col()
            .on_mouse_down (MouseButton::Left, cx.listener (|_this, _ev:&MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }))
            .child (self.menu_item ("Refresh"      , "Ctrl+R"    , MenuAction::Refresh,        cx))
            .child (self.menu_item ("Reload"       , "F5"        , MenuAction::Reload,         cx))
            .child (self.menu_item ("Group Mode"   , "Ctrl+G"    , MenuAction::GroupToggle,    cx))
            .child (self.menu_item ("Auto Hide"    , "Ctrl+Alt+A", MenuAction::AutoHideToggle, cx))
            .child (self.menu_item ("Edit Config"  , ""          , MenuAction::EditConfig,     cx))
            .child (self.menu_item ("Reset Config" , ""          , MenuAction::ResetConfig,    cx))
            .child (self.menu_item ("Quit"         , "Alt+F4"    , MenuAction::Quit,           cx))
            .into_any_element()
    }

    fn render_ribbon (&self, cx:&mut Context<Self>) -> AnyElement {
        let mut ribbon = div()
            .relative().w_full().flex().flex_row().items_center().h(px(RIBBON_H))
            .text_color(rgb(C_RIBBON));

        ribbon = ribbon
            .child (self.render_menu_link (cx))
            .child (self.indicator ("A", self.configs.alt_tab_enabled, C_ALT_TAB))
            .child (self.indicator ("M", self.configs.rbtn_whl_enabled, C_RBTN_WHL))
            .child (self.indicator ("⇅", self.configs.grp_ordering_is_auto, C_RIBBON))
            .child (self.indicator ("⚿", self.configs.is_elevated, C_ELEV))
            .child (self.indicator ("R", self.scroll_armed, C_ARMED))
            .child (self.render_drag_indicator (cx))
            .child (
                div().w(px(40.)).flex_none().text_center().text_color(rgb(C_RIBBON))
                    .child(format!("({})", self.render_list.len()))
            )
            .child (self.render_search())
            .child (self.indicator ("!", false, C_WARN))
            .child (self.indicator ("?", false, C_HELP));

        ribbon.into_any_element()
    }
}



fn highlighted_text (
    window:&Window, text:&str, ranges:&[Range<usize>], normal:u32, matched:u32, size:f32, truncate:bool
) -> AnyElement {
    if text.is_empty() {
        return div().text_size(px(size)).into_any_element()
    }
    // note: we lay this out as a single StyledText with runs (rather than a flex row of spans),
    // .. so kerning/shaping is preserved across segment boundaries and the text doesn't widen
    let base = window.text_style();
    let mut runs = Vec::new();
    for (seg, is_match) in split_highlighted (text, ranges) {
        if seg.is_empty() { continue }
        let mut run = base.clone().to_run (seg.len());
        run.font = gpui::font("Segoe UI");
        run.color = rgb(if is_match { matched } else { normal }).into();
        runs.push (run);
    }
    if runs.is_empty() {
        return div().text_size(px(size)).text_color(rgb(normal)).child(text.to_string()).into_any_element()
    }
    let styled = StyledText::new (text.to_string()) .with_runs (runs);
    let wrapper = div().text_size(px(size));
    if truncate { wrapper.truncate().child(styled).into_any_element() }
    else { wrapper.child(styled).into_any_element() }
}



impl Render for SwitcheApp {
    fn render (&mut self, window:&mut Window, cx:&mut Context<Self>) -> impl IntoElement {

        let win_h = f32::from(window.bounds().size.height);
        self.last_viewport_h = (win_h - 2.0 * (ROOT_PAD + BORDER) - RIBBON_H).max (0.0);
        self.recompute_display_height();

        let mut items : Vec <AnyElement> = Vec::with_capacity (self.display.len());
        for item in &self.display {
            match item {
                DispItem::Header (t) => items.push (self.render_header (*t)),
                DispItem::Row (r)    => items.push (self.render_row (r, &*window, cx)),
                DispItem::Spacer     => items.push (div().h(px(SPACER_H)).into_any_element()),
                DispItem::Gap        => items.push (div().h(px(GAP_H)).into_any_element()),
            }
        }

        let vh = self.last_viewport_h;
        let total = self.display_height;
        let thumb = if total > vh && vh > 0.0 {
            let thumb_h = (vh / total * vh).max(12.0);
            let max_off = (total - vh).max(1.0);
            let thumb_top = (self.scroll_offset / max_off) * (vh - thumb_h);
            Some ((thumb_top, thumb_h))
        } else { None };

        let mut list_area = div()
            .flex_1().relative().overflow_hidden()
            .on_scroll_wheel (cx.listener (|this, ev:&ScrollWheelEvent, _window, cx| this.handle_wheel (ev, cx)))
            .child (
                div()
                    .absolute().top(px(-self.scroll_offset)).left(px(0.)).right(px(0.))
                    .flex().flex_col()
                    .children(items)
            );

        if let Some ((top, h)) = thumb {
            list_area = list_area.child (
                div()
                    .absolute().right(px(0.)).top(px(top)).w(px(4.)).h(px(h))
                    .bg(rgb(C_SCROLL_THUMB)).rounded_sm()
            );
        }

        let mut root = div()
            .size_full().flex().flex_col()
            .bg(rgb(C_BG))
            .border_1().border_color(rgb(C_RIBBON)).rounded_lg()
            .p(px(ROOT_PAD)).px(px(12.))
            .font_family("Segoe UI").text_size(px(TEXT_SZ))
            .track_focus(&self.focus_handle)
            .capture_key_down (cx.listener (|this, ev:&KeyDownEvent, window, cx| this.handle_key_down (ev, window, cx)))
            .on_mouse_down (MouseButton::Left, cx.listener (|this, _ev:&MouseDownEvent, _window, cx| {
                if this.menu_open { this.menu_open = false; cx.notify() }
            }))
            .child (self.render_ribbon (cx))
            .child (list_area);

        if self.menu_open {
            root = root.child (self.render_dropdown (cx));
        }
        root
    }
}
