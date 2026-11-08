use gpui::{ClipboardItem, Context};

use super::*;


impl SwitcheApp {
    // ---------------- search editing ----------------

    pub(super) fn search_selection (&self) -> Option <(usize, usize)> {
        let a = self.search_anchor?;
        let b = self.search_caret;
        if a == b { None } else { Some ((a.min (b), a.max (b))) }
    }
    fn delete_search_selection (&mut self) -> bool {
        if let Some ((a, b)) = self.search_selection() {
            self.search_text.replace_range (a..b, "");
            self.search_caret = a;
            self.search_anchor = None;
            true
        } else { false }
    }
    pub(super) fn insert_search_text (&mut self, s:&str) {
        self.in_search = true;
        // search is a single-line field; normalize any pasted line breaks
        let s : String = s.chars().map (|c| if c == '\n' || c == '\r' { ' ' } else { c }) .collect();
        self.delete_search_selection();
        let caret = self.search_caret.min (self.search_text.len());
        self.search_text.insert_str (caret, &s);
        self.search_caret = caret + s.len();
        self.search_anchor = None;
        self.prior_search_text = self.search_text.trim().to_string();
        self.rebuild_render_state();
        self.reset_search_match_focus();
    }
    fn on_search_text_changed (&mut self) {
        self.search_caret = self.search_caret.min (self.search_text.len());
        self.prior_search_text = self.search_text.trim().to_string();
        self.rebuild_render_state();
        self.reset_search_match_focus();
    }
    pub(super) fn move_search_caret (&mut self, key:&str, shift:bool) {
        let len = self.search_text.len();
        if !shift {
            if let Some ((a, b)) = self.search_selection() {
                if key == "left"  { self.search_caret = a; self.search_anchor = None; return }
                if key == "right" { self.search_caret = b; self.search_anchor = None; return }
            }
        }
        let old = self.search_caret.min (len);
        let mut caret = old;
        match key {
            "left" => if caret > 0 {
                caret -= self.search_text [..caret] .chars() .next_back() .map (|c| c.len_utf8()) .unwrap_or (1)
            },
            "right" => if caret < len {
                caret += self.search_text [caret..] .chars() .next() .map (|c| c.len_utf8()) .unwrap_or (1)
            },
            "home" => caret = 0,
            "end"  => caret = len,
            _ => { }
        }
        if shift {
            if self.search_anchor.is_none() { self.search_anchor = Some (old) }
        } else {
            self.search_anchor = None;
        }
        self.search_caret = caret;
    }
    pub(super) fn backspace_search (&mut self) {
        if !self.delete_search_selection() {
            if self.search_caret == 0 { return }
            let caret = self.search_caret.min (self.search_text.len());
            let prev = self.search_text [..caret] .chars() .next_back() .map (|c| caret - c.len_utf8()) .unwrap_or (0);
            self.search_text.replace_range (prev..caret, "");
            self.search_caret = prev;
        }
        if self.search_text.trim().is_empty() { self.exit_search (true) }
        else { self.on_search_text_changed() }
    }
    pub(super) fn delete_search_forward (&mut self) {
        if !self.delete_search_selection() {
            let len = self.search_text.len();
            if self.search_caret >= len { return }
            let next = self.search_caret + self.search_text [self.search_caret..] .chars() .next() .map (|c| c.len_utf8()) .unwrap_or (1);
            self.search_text.replace_range (self.search_caret..next, "");
        }
        if self.search_text.trim().is_empty() { self.exit_search (true) }
        else { self.on_search_text_changed() }
    }
    pub(super) fn select_all_search (&mut self) {
        self.in_search = true;
        self.search_anchor = Some (0);
        self.search_caret = self.search_text.len();
    }
    pub(super) fn copy_search_selection (&mut self, cx:&mut Context<Self>) {
        if let Some ((a, b)) = self.search_selection() {
            let selected = self.search_text [a..b] .to_string();
            cx.write_to_clipboard (ClipboardItem::new_string (selected));
        }
    }
    pub(super) fn cut_search_selection (&mut self, cx:&mut Context<Self>) {
        if self.search_selection().is_some() {
            self.copy_search_selection (cx);
            self.delete_search_selection();
            if self.search_text.trim().is_empty() { self.exit_search (true) }
            else { self.on_search_text_changed() }
        }
    }
    pub(super) fn reactivate_last_search (&mut self) {
        self.in_search = true;
        self.search_text = self.prior_search_text.clone();
        self.search_caret = self.search_text.len();
        self.search_anchor = None;
        self.rebuild_render_state();
        self.reset_search_match_focus();
    }
    pub(super) fn exit_search (&mut self, clear_prior:bool) {
        let was_searching = self.in_search || !self.search_text.is_empty();
        self.in_search = false;
        self.search_text.clear();
        self.search_caret = 0;
        self.search_anchor = None;
        self.match_count = 0;
        if clear_prior { self.prior_search_text.clear() }
        if was_searching {
            self.rebuild_render_state();
            self.resync_focus_after_rebuild();
        }
    }

}
