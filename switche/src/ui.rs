#![allow (non_snake_case, non_camel_case_types, dead_code)]

mod help;
mod ribbon;
mod keys;
mod list;
mod nav;
mod actions;
mod search_edit;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{
    div, px, rgb, AnyElement, AsyncApp, Context,
    FocusHandle, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Render,
    ScrollDelta, ScrollHandle, ScrollWheelEvent, TextRun, Window,
};


use crate::app_window;
use crate::switche::{ EngineNotice, ConfigsView, Hwnd, RowView, SwitcheState, UiEvent };

use crate::search::check_search_exe_title;



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
const SEARCH_SZ   : f32 = 18.0;
const EXE_W       : f32 = 120.0;
const Y_W         : f32 = 14.0;
const ICO_W       : f32 = 22.0;

#[derive (Clone, Copy, Debug)]
enum MenuAction { Refresh, Reload, GroupToggle, AutoHideToggle, EditConfig, ResetConfig, Quit }

#[derive (Clone, Copy, Debug, PartialEq, Eq)]
enum TooltipKind { AltTab, RbtnWheel, AutoOrder, Elev, Armed, Drag, Counts, Warn, Help }


const MIN_SCROLL_SPACING : Duration = Duration::from_millis (15);
// suppress hover-driven selection for a short while after invocations, keys, wheel and clicks
// .. so the programmatically/keyboard-selected row isn't stolen by a stationary pointer
const HOVER_LOCK : Duration = Duration::from_millis (200);



#[derive (Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Block { R, G }

#[derive (Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct ElemId { hwnd: Hwnd, block: Block }

#[derive (Debug, Clone, Copy)]
struct OrderedElem { nav_idx: usize, yg: i32 }

#[derive (Debug, Clone, Copy, PartialEq, Eq)]
enum Grp { NG, LH, GH, GT }

#[derive (Debug, Clone, Copy, PartialEq, Eq)]
enum ElemT { R, G, GR }

#[derive (Debug, Clone)]
struct Row {
    id        : ElemId,
    grp       : Grp,
    elem_t    : ElemT,
    dimmed    : bool,
    matched   : bool,
    view      : RowView,
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





fn ss () -> &'static SwitcheState { SwitcheState::instance() }

pub struct SwitcheApp {
    pub focus_handle : FocusHandle,

    recents : Vec <RowView>,
    grouped : Vec <Vec <RowView>>,
    configs : ConfigsView,

    // armed whenever the window goes away; the next invocation consumes it and starts
    // .. with a fresh selection instead of advancing (pure view state)
    fresh_show   : bool,
    scroll_armed : bool,
    menu_open    : bool,

    in_search         : bool,
    search_text       : String,
    search_caret      : usize,
    search_anchor     : Option <usize>,
    search_scroll_x   : f32,
    prior_search_text : String,
    match_count       : usize,

    cur_elem  : Option <ElemId>,
    tray      : Option <crate::tray::TrayState>,

    tooltip       : Option <TooltipKind>,
    help_hover    : bool,
    help_open     : bool,
    help_hide_gen : u64,
    help_hide_pending: bool,
    
    recents_map : HashMap <ElemId, OrderedElem>,
    grouped_map : HashMap <ElemId, OrderedElem>,
    recents_ids : Vec <ElemId>,
    grouped_ids : Vec <ElemId>,
    group_heads : Vec <ElemId>,

    display        : Vec <DispItem>,
    display_height : f32,
    layout_dirty   : bool,
    scroll_offset  : f32,

    last_viewport_h   : f32,
    last_action_stamp : Instant,
    last_nav_stamp    : Instant,

    hover_lock_stamp : Instant,
    hover_anchor     : Option <ElemId>,
    hovered_row      : Option <ElemId>,

    armed_blip     : bool,
    armed_blip_gen : u64,
    nav_repeat_ms  : u64,

    help_text_w : Option <f32>,
    help_scroll : ScrollHandle,
    help_drag   : Option <(f32, f32)>,   // (cursor y, scroll offset) while dragging the help thumb

    bounds_subscription : Option <gpui::Subscription>,
}



impl SwitcheApp {

    pub fn new (cx:&mut Context<Self>) -> Self {

        let focus_handle = cx.focus_handle();
        let now = Instant::now();
        // release renders comfortably at 15ms .. debug gets glitchy when up/dn keys held
        let nav_repeat_ms = if cfg!(debug_assertions) { 60 } else { 15 };
        tracing::info! ("nav-repeat-interval: {}ms", nav_repeat_ms);

        let mut app = SwitcheApp {
            focus_handle,

            recents : Vec::new(),
            grouped : Vec::new(),
            configs : ConfigsView::default(),

            fresh_show   : true,
            scroll_armed : false,
            menu_open    : false,

            in_search         : false,
            search_text       : String::new(),
            search_caret      : 0,
            search_anchor     : None,
            search_scroll_x   : 0.0,
            prior_search_text : String::new(),
            match_count       : 0,

            cur_elem  : None,
            tray      : None,

            tooltip    : None,
            help_hover : false,
            help_open  : false,
            help_hide_gen : 0,
            help_hide_pending : false,
            
            recents_map : HashMap::new(),
            grouped_map : HashMap::new(),
            recents_ids : Vec::new(),
            grouped_ids : Vec::new(),
            group_heads : Vec::new(),

            display        : Vec::new(),
            display_height : 0.0,
            layout_dirty   : false,
            scroll_offset  : 0.0,

            last_viewport_h   : 0.0,
            last_action_stamp : now,
            last_nav_stamp    : now - Duration::from_secs (1),

            hover_lock_stamp : now - Duration::from_secs (1),
            hover_anchor     : None,
            hovered_row      : None,

            armed_blip     : false,
            armed_blip_gen : 0,
            nav_repeat_ms,

            help_text_w : None,
            help_scroll : ScrollHandle::new(),
            help_drag   : None,

            bounds_subscription : None,
        };

        app.rebuild_render_state();
        app
    }


    /// Creates the tray icon/menu. Must run on the main thread, after the window exists.
    pub fn setup_tray (&mut self, cx:&mut Context<Self>) {
        if self.tray.is_some() { return }
        match crate::tray::setup_tray (ss()) {
            Some (tray) => {
                self.tray = Some (tray);
                self.refresh_autostart_flags (cx);
            }
            None => tracing::error! ("tray setup failed"),
        }
    }

    /// planif/task-scheduler access can be slow and change thread mode, so query flags off-thread
    /// .. and apply them to the (main-thread-only) menu items in a foreground task
    fn refresh_autostart_flags (&mut self, cx:&mut Context<Self>) {
        let (tx, rx) = async_channel::bounded (1);
        std::thread::spawn ( move || {
            let is_elev = crate::win_apis::check_cur_proc_elevated().unwrap_or (false);
            let el_en   = crate::autostart::check_status_auto_start_elev_task();
            let nel_en  = crate::autostart::check_status_auto_start_task();
            let _ = tx.send_blocking ((is_elev, el_en, nel_en));
        } );
        cx.spawn (async move |this, cx| {
            if let Ok ((is_elev, el_en, nel_en)) = rx.recv().await {
                let _ = this.update (cx, |this, _cx| {
                    if let Some (tray) = this.tray.as_ref() {
                        crate::tray::apply_auto_start_flags (tray, is_elev, el_en, nel_en);
                    }
                });
            }
        }) .detach();
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
            UiEvent::RefreshAutostartFlags => {
                self.refresh_autostart_flags (cx);
            }
            UiEvent::Redraw => {
                cx.notify();
            }
            UiEvent::Quit => {
                app_window::persist_self_window_dimensions_now (ss());
                cx.quit();
            }
            UiEvent::Notice (n) => self.handle_notice (n, cx),
            UiEvent::Model (mv) => {
                if mv.recents == self.recents && mv.grouped == self.grouped { return }
                // ^^ if the snapshot is unchanged, skip the rebuild
                tracing::debug! ("ui: model ({} rows / {} groups)", mv.recents.len(), mv.grouped.len());
                self.recents = mv.recents;
                self.grouped = mv.grouped;
                self.layout_dirty = true;
                cx.notify();
            }
            UiEvent::Configs (conf) => {
                self.configs = conf;
                self.layout_dirty = true;
                cx.notify();
            }
        }
    }


    /// applies any coalesced layout rebuild before navigation needs the row maps.
    /// .. this matters because the window is hidden between invocations, and no render
    /// .. (and therefore no deferred rebuild) may happen while it is hidden
    pub(super) fn flush_layout (&mut self) {
        if self.layout_dirty {
            self.rebuild_render_state();
            self.resync_focus_after_rebuild();
        }
    }


    fn handle_notice (&mut self, n:EngineNotice, cx:&mut Context<Self>) {
        use EngineNotice::*;
        self.flush_layout();
        self.lock_hover();
        match n {
            Invoke => {
                self.handle_invoke();
            }
            ScrollDown => {
                if !self.verify_spacing() { return }
                self.scroll_armed = true;
                self.handle_invoke();
            }
            ScrollUp => {
                if !self.verify_spacing() { return }
                self.scroll_armed = true;
                if self.take_fresh() { self.reset_focus(); }
                self.focus_prev();
            }
            ScrollEnd => {
                // engine only sends this while shown (and it has already checked that)
                if self.scroll_armed { self.activate_cur() }
            }
            ScrollEndDisarm => {
                if self.scroll_armed { self.scroll_armed = false }
            }
            Escape => {
                self.fresh_show = true;
                self.scroll_armed = false; self.menu_open = false;
                self.hover_anchor = None;
                self.hovered_row = None;
                self.exit_search (false);
            }
            Reload => {
                self.handle_reload (cx);
            }
            ForegroundGained => {
                // engine owns is_fgnd/is_dismissed; the notice prelude re-syncs layout
                // .. and holds off hover-driven selection while the window settles
            }
            ForegroundLost => {
                self.fresh_show = true;
                self.scroll_armed = false;
                self.hover_anchor = None;
                self.hovered_row = None;
                self.exit_search (false);
            }
        }
        cx.notify();
    }




    fn verify_spacing (&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since (self.last_action_stamp) < MIN_SCROLL_SPACING { return false }
        self.last_action_stamp = now;
        true
    }



    fn recompute_display_height (&mut self) {
        self.display_height = self.display.iter().map (|i| i.height()).sum();
        let max = (self.display_height - self.last_viewport_h.max (0.0)) .max (0.0);
        self.scroll_offset = self.scroll_offset.clamp (0.0, max);
    }



    // ---------------- building lists ----------------

    fn get_capped_recents (&self, group_mode:bool) -> Vec <(RowView, Grp)> {
        let capped : Vec <RowView> = if group_mode {
            let nfirst = self.configs.n_grp_mode_top_recents as usize;
            let nlast  = self.configs.n_grp_mode_last_recents as usize;
            let ngap = self.recents.len() as isize - nfirst as isize - nlast as isize;
            let mut v = self.recents.iter().take (nfirst).cloned().collect::<Vec<_>>();
            v.extend (self.recents.iter().skip (nfirst).skip (ngap.max (0) as usize).cloned());
            v
        } else {
            self.recents.clone()
        };

        let mut last_y = capped.first().map (|r| r.y).unwrap_or (0);
        capped.into_iter().map (|rle| {
            let grp = if rle.y > last_y + 1 { Grp::LH } else { Grp::NG };
            last_y = rle.y;
            (rle, grp)
        }) .collect()
    }

    fn rebuild_render_state (&mut self) {
        self.layout_dirty = false;
        self.recents_map.clear();
        self.grouped_map.clear();
        self.recents_ids.clear();
        self.grouped_ids.clear();
        self.group_heads.clear();

        let group_mode = ss().conf.check_flag__group_mode_enabled();

        let mut recents_rows : Vec <Row> = Vec::new();
        let mut grouped_rows : Vec <Row> = Vec::new();

        if self.in_search {
            self.build_search_rows (group_mode, &mut recents_rows, &mut grouped_rows);
        } else {
            self.build_normal_rows (group_mode, &mut recents_rows, &mut grouped_rows);
        }

        self.display.clear();
        if self.in_search {
            if group_mode {
                self.push_section (ElemT::GR, recents_rows);
                self.push_section (ElemT::G,  grouped_rows);
            } else {
                self.push_section (ElemT::R, recents_rows);
            }
        } else if group_mode {
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

    fn build_normal_rows (&mut self, group_mode:bool, recents_rows:&mut Vec <Row>, grouped_rows:&mut Vec <Row>) {
        let capped = self.get_capped_recents (group_mode);
        for (view, grp) in capped {
            let id = ElemId { hwnd: view.hwnd, block: Block::R };
            let elem_t = if group_mode { ElemT::GR } else { ElemT::R };
            let nav_idx = self.recents_ids.len();
            self.recents_map.insert (id, OrderedElem { nav_idx, yg: -1 });
            self.recents_ids.push (id);
            recents_rows.push (Row { id, grp, elem_t, dimmed: false, matched: false, view });
        }

        if group_mode {
            let mut nav_i = 0;
            for (gi, ll) in self.grouped.iter().enumerate() {
                let mut first = true;
                for view in ll {
                    let id = ElemId { hwnd: view.hwnd, block: Block::G };
                    let grp = if first { Grp::GH } else { Grp::GT };
                    if first { self.group_heads.push (id) }
                    first = false;
                    self.grouped_map.insert (id, OrderedElem { nav_idx: nav_i, yg: gi as i32 });
                    self.grouped_ids.push (id);
                    grouped_rows.push (Row { id, grp, elem_t: ElemT::G, dimmed: false, matched: false, view: view.clone() });
                    nav_i += 1;
                }
            }
        }
    }

    fn build_search_rows (&mut self, group_mode:bool, recents_rows:&mut Vec <Row>, grouped_rows:&mut Vec <Row>) {
        let capped = self.get_capped_recents (group_mode);
        let mut match_count = 0;

        if !group_mode {
            let mut nav_i = 0;
            for (view, grp) in capped {
                let check = check_search_exe_title (&view, &self.search_text);
                let id = ElemId { hwnd: view.hwnd, block: Block::R };
                let nav_idx = if check.passed { let n = nav_i; nav_i += 1; n } else { usize::MAX };
                self.recents_map.insert (id, OrderedElem { nav_idx, yg: -1 });
                if check.passed { self.recents_ids.push (id); match_count += 1 }
                recents_rows.push (Row { id, grp, elem_t: ElemT::R, dimmed: false, matched: check.passed, view });
            }
        } else {
            for (gi, ll) in self.grouped.iter().enumerate() {
                let mut first = true;
                for view in ll {
                    let check = check_search_exe_title (view, &self.search_text);
                    let id = ElemId { hwnd: view.hwnd, block: Block::G };
                    let grp = if first { Grp::GH } else { Grp::GT };
                    let nav_idx = if check.passed { let n = self.grouped_ids.len(); n } else { usize::MAX };
                    if check.passed { self.grouped_ids.push (id); match_count += 1 }
                    first = false;
                    self.grouped_map.insert (id, OrderedElem { nav_idx, yg: gi as i32 });
                    grouped_rows.push (Row { id, grp, elem_t: ElemT::G, dimmed: false, matched: check.passed, view: view.clone() });
                }
            }
            for (i,(view,grp)) in capped.into_iter().enumerate() {
                let check = check_search_exe_title (&view, &self.search_text);
                let id = ElemId { hwnd: view.hwnd, block: Block::R };
                self.recents_map.insert (id, OrderedElem { nav_idx: i, yg: -1 });
                self.recents_ids.push (id);
                recents_rows.push (Row { id, grp, elem_t: ElemT::GR, dimmed: true, matched: check.passed, view });
            }
        }

        self.match_count = match_count;
    }



    // ---------------- wheel ----------------

    fn handle_wheel (&mut self, ev:&ScrollWheelEvent, cx:&mut Context<Self>) {
        if self.menu_open || self.help_visible() { return }
        self.flush_layout();
        let (dy, dx) = match ev.delta {
            ScrollDelta::Pixels (p) => (f32::from(p.y), f32::from(p.x)),
            ScrollDelta::Lines  (p) => (p.y, p.x),
        };
        if dy == 0.0 && dx == 0.0 { return }
        if !self.verify_spacing() { return }
        self.lock_hover();

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
}



    // ---------------- rendering ----------------





impl Render for SwitcheApp {
    fn render (&mut self, window:&mut Window, cx:&mut Context<Self>) -> impl IntoElement {

        // persist manually moved/resized dimensions (deferred/coalesced)
        if self.bounds_subscription.is_none() {
            self.bounds_subscription = Some (cx.observe_window_bounds (window, |_this, _window, _cx| {
                if ss().is_fgnd.is_set() {
                    crate::app_window::persist_self_window_dimensions (ss());
                }
            }));
        }

        self.update_search_scroll (window);

        let win_h = f32::from(window.bounds().size.height);
        self.last_viewport_h = (win_h - 2.0 * (ROOT_PAD + BORDER) - RIBBON_H).max (0.0);

        // any events since the last frame collapse into a single rebuild here
        self.flush_layout();
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
            // drag tracking for the help panel's scrollbar thumb
            .on_mouse_move (cx.listener (|this, ev:&MouseMoveEvent, _window, cx| {
                if this.help_drag.is_some() {
                    if ev.pressed_button == Some (MouseButton::Left) {
                        this.help_drag_move (f32::from (ev.position.y));
                    } else {
                        // missed the mouse-up (e.g. released outside the window)
                        this.help_drag = None;
                    }
                    cx.notify();
                }
            }))
            .on_mouse_up (MouseButton::Left, cx.listener (|this, _ev:&MouseUpEvent, _window, cx| {
                if this.help_drag.take().is_some() { cx.notify(); }
            }))
            .on_mouse_down (MouseButton::Left, cx.listener (|this, _ev:&MouseDownEvent, _window, cx| {
                this.lock_hover();
                // clicking anywhere disarms the release-activation state (old frontend behavior)
                let was_armed = this.scroll_armed;
                this.scroll_armed = false;
                if this.menu_open || this.help_open {
                    this.menu_open = false;
                    this.help_open = false;
                    this.help_hover = false;
                    this.help_hide_pending = false;
                    this.help_hide_gen = this.help_hide_gen.wrapping_add (1);
                    this.tooltip = None;
                    cx.notify()
                } else if was_armed {
                    cx.notify();
                }
            }))
            .child (self.render_ribbon (cx, &*window))
            .child (list_area);

        if let Some (tip) = self.render_tooltip() {
            root = root.child (tip);
        }
        if self.help_visible() {
            root = root.child (self.render_help_panel (&*window, cx));
        }

        if self.menu_open {
            // block the list from mouse-hover/wheel while the dropdown is open, then paint the
            // .. dropdown above this catch layer (so its items still work)
            root = root.child (
                div()
                    .absolute().top(px(ROOT_PAD + RIBBON_H)).left(px(0.)).right(px(0.)).bottom(px(0.))
                    .on_mouse_down (MouseButton::Left, cx.listener (|this, _ev:&MouseDownEvent, _window, cx| {
                        cx.stop_propagation();
                        this.menu_open = false;
                        cx.notify();
                    }))
                    .on_mouse_down (MouseButton::Middle, cx.listener (|_this, _ev:&MouseDownEvent, _window, cx| { cx.stop_propagation() }))
                    .on_mouse_down (MouseButton::Right,  cx.listener (|_this, _ev:&MouseDownEvent, _window, cx| { cx.stop_propagation() }))
                    .on_scroll_wheel (cx.listener (|_this, _ev:&ScrollWheelEvent, _window, cx| { cx.stop_propagation() }))
            );
            root = root.child (self.render_dropdown (cx));
        }
        root
    }
}
