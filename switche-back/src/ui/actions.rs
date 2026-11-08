use gpui::Context;

use crate::app_window;
use crate::switche::UiRequest;

use super::*;


impl SwitcheApp {
    pub(super) fn handle_invoke (&mut self) {
        if self.is_dismissed || !self.is_fgnd {
            self.is_dismissed = false;
            self.is_fgnd = true;
            self.reset_focus();
        } else {
            self.focus_next();
        }
    }


    // ---------------- actions ----------------

    pub(super) fn do_escape (&mut self) {
        self.scroll_armed = false;
        self.is_dismissed = true;
        self.menu_open = false;
        self.exit_search (false);
        ss().handle_ui_request (UiRequest::SwitcheEscape);
    }
    pub(super) fn do_quit (&mut self, cx:&mut Context<Self>) {
        app_window::persist_self_window_dimensions_now (ss());
        cx.quit();
    }
    pub(super) fn activate_cur (&mut self) {
        self.scroll_armed = false;
        self.is_dismissed = true;
        self.menu_open = false;
        if let Some (id) = self.cur_elem {
            ss().handle_ui_request (UiRequest::WindowActivate (id.hwnd));
        }
    }
    pub(super) fn close_cur (&mut self) {
        if let Some (id) = self.cur_elem {
            if let Some (next) = self.pick_next_elem (false, false, true) { self.set_cur (Some (next)) }
            ss().handle_ui_request (UiRequest::WindowClose (id.hwnd));
        }
    }
    pub(super) fn peek_cur (&mut self) {
        if let Some (id) = self.cur_elem { ss().handle_ui_request (UiRequest::WindowPeek (id.hwnd)) }
    }
    pub(super) fn handle_refresh (&mut self) {
        ss().handle_ui_request (UiRequest::Refresh);
    }
    pub(super) fn handle_reload (&mut self, cx:&mut Context<Self>) {
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
    pub(super) fn toggle_group_mode (&mut self) {
        self.group_mode = !self.group_mode;
        ss().handle_ui_request (if self.group_mode { UiRequest::GrpModeEnable } else { UiRequest::GrpModeDisable });
        self.rebuild_render_state();
        self.resync_focus_after_rebuild();
    }
    pub(super) fn toggle_auto_hide (&mut self) {
        self.auto_hide = !self.auto_hide;
        ss().handle_ui_request (if self.auto_hide { UiRequest::AutoHideEnable } else { UiRequest::AutoHideDisable });
    }
    pub(super) fn do_menu_action (&mut self, action:MenuAction, cx:&mut Context<Self>) {
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

}
