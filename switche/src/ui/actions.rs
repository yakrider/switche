use gpui::Context;

use crate::app_window;
use crate::switche::UiRequest;

use super::*;


impl SwitcheApp {
    pub(super) fn take_fresh (&mut self) -> bool {
        let fresh = self.fresh_show;
        self.fresh_show = false;
        fresh
    }
    pub(super) fn handle_invoke (&mut self) {
        if self.take_fresh() { self.reset_focus(); } else { self.focus_next(); }
    }


    // ---------------- actions ----------------

    pub(super) fn do_escape (&mut self) {
        self.fresh_show = true;
        self.scroll_armed = false; self.menu_open = false;
        self.exit_search (false);
        ss().handle_request (UiRequest::Escape);
    }
    pub(super) fn do_quit (&mut self, cx:&mut Context<Self>) {
        app_window::persist_self_window_dimensions_now (ss());
        cx.quit();
    }
    pub(super) fn activate_cur (&mut self) {
        self.fresh_show = true;
        self.scroll_armed = false; self.menu_open = false;
        if let Some (id) = self.cur_elem {
            ss().handle_request (UiRequest::WindowActivate (id.hwnd));
        }
    }
    pub(super) fn close_cur (&mut self) {
        if let Some (id) = self.cur_elem {
            if let Some (next) = self.pick_next_elem (false, false, true) { self.set_cur (Some (next)) }
            ss().handle_request (UiRequest::WindowClose (id.hwnd));
        }
    }
    pub(super) fn peek_cur (&mut self) {
        if let Some (id) = self.cur_elem { ss().handle_request (UiRequest::WindowPeek (id.hwnd)) }
    }
    pub(super) fn handle_refresh (&mut self, cx:&mut Context<Self>) {
        self.blip_armed_indicator (cx);
        ss().handle_request (UiRequest::Refresh);
    }
    pub(super) fn handle_reload (&mut self, cx:&mut Context<Self>) {
        self.blip_armed_indicator (cx);
        self.in_search = false;
        self.scroll_armed = false; self.menu_open = false;
        self.search_text.clear();
        self.prior_search_text.clear();
        self.match_count = 0;
        self.cur_elem = None;
        self.scroll_offset = 0.0;
        self.hover_anchor = None;
        self.hovered_row = None;
        ss().handle_request (UiRequest::Reload);
        cx.notify();
    }
    pub(super) fn toggle_group_mode (&mut self) {
        let enabled = !ss().conf.check_flag__group_mode_enabled();
        ss().handle_request (UiRequest::SetGroupMode (enabled));
        self.rebuild_render_state();
        self.resync_focus_after_rebuild();
    }
    pub(super) fn toggle_auto_hide (&mut self) {
        let enabled = !ss().conf.check_flag__auto_hide_enabled();
        ss().handle_request (UiRequest::SetAutoHide (enabled));
    }
    pub(super) fn do_menu_action (&mut self, action:MenuAction, cx:&mut Context<Self>) {
        self.menu_open = false;
        // menu actions blink the armed indicator as feedback (as in the old frontend)
        self.blip_armed_indicator (cx);
        match action {
            MenuAction::Refresh       => self.handle_refresh (cx),
            MenuAction::Reload        => self.handle_reload (cx),
            MenuAction::GroupToggle   => self.toggle_group_mode(),
            MenuAction::AutoHideToggle=> self.toggle_auto_hide(),
            MenuAction::EditConfig    => { ss().handle_request (UiRequest::EditConfig) }
            MenuAction::ResetConfig   => { ss().handle_request (UiRequest::ResetConfig) }
            MenuAction::Quit          => self.do_quit (cx),
        }
        cx.notify();
    }

}
