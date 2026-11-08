use gpui::{Context, KeyDownEvent, Window};

use super::*;


impl SwitcheApp {
    pub(super) fn handle_key_down (&mut self, ev:&KeyDownEvent, _window:&mut Window, cx:&mut Context<Self>) {
        let ks    = &ev.keystroke;
        let key   = ks.key.as_str();
        let ctrl  = ks.modifiers.control;
        let alt   = ks.modifiers.alt;
        let shift = ks.modifiers.shift;

        self.flush_layout();
        self.lock_hover();

        if matches! (key, "shift" | "control" | "alt" | "platform" | "function" | "capslock" | "numlock") { return }

        let left_right_is_nav = (key == "left" || key == "right") && (!self.in_search || self.scroll_armed || alt);
        // pageup/pagedown are discrete block jumps, so they're exempt from the repeat throttle
        let is_nav_key = matches!(key, "up" | "down" | "tab" | "f1" | "f2" | "f15" | "f16" | "f17")
            || left_right_is_nav
            || ((alt || self.scroll_armed) && matches!(key, "i" | "j" | "k" | "m" | "u" | ","));
        let nav_allowed = !is_nav_key || self.allow_nav_action();
        if !nav_allowed { return }

        let mut handled = true;

        match key {
            "enter" => self.activate_cur(),

            "escape" => {
                if self.menu_open {
                    self.menu_open = false;
                } else if self.help_open || self.help_hover {
                    self.help_open = false;
                    self.help_hover = false;
                    self.help_hide_pending = false;
                    self.help_hide_gen = self.help_hide_gen.wrapping_add (1);
                } else if self.in_search {
                    self.exit_search (false)
                } else {
                    self.do_escape()
                }
            }

            "f5" => self.handle_reload (cx),
            "f3" => {
                if self.in_search { self.focus_next() } else { self.reactivate_last_search() }
            }
            "f1" | "f15" => self.focus_next(),
            "f2"         => self.focus_prev(),
            "f16"        => { self.scroll_armed = true; if shift { self.focus_prev() } else { self.focus_next() } }
            "f17"        => { self.scroll_armed = true; if shift { self.focus_next() } else { self.focus_prev() } }

            "tab"  => { if shift { self.focus_prev() } else { self.focus_next() } }
            "up"   => { if shift { self.focus_prev() } else { self.focus_block_prev() } }
            "down" => { if shift { self.focus_next() } else { self.focus_block_next() } }

            "pageup" => {
                // plain = current block top; shift = whole-list top
                if shift { self.focus_top() } else { self.focus_block_top() }
            }
            "pagedown" => {
                if shift { self.focus_bottom() } else { self.focus_block_bottom() }
            }

            "left"  if left_right_is_nav => self.focus_group_prev(),
            "right" if left_right_is_nav => self.focus_group_next(),

            "left" | "right" | "home" | "end" if self.in_search => {
                // shift-selection must consume every event (external select-all workflows),
                // .. but plain caret movement is throttled for visible feedback
                if shift || self.allow_nav_action() {
                    self.move_search_caret (key, shift);
                }
            }
            "delete" if self.in_search => self.delete_search_forward(),

            "space" => {
                if self.scroll_armed { self.scroll_armed = false }
                else if alt { self.activate_cur() }
                else if !self.in_search { self.activate_cur() }
                else if ctrl { self.activate_cur() }
                else { self.insert_search_text (" ") }
            }

            "f4" if alt => self.do_quit (cx),
            "a"  if alt && ctrl => self.toggle_auto_hide(),

            "i" | "," | "u" | "m" | "j" | "k" | "r" | "s" | "l" if alt || self.scroll_armed => {
                match key {
                    "i" => if shift { self.focus_prev() }   else { self.focus_block_prev() },
                    "," => if shift { self.focus_next() }   else { self.focus_block_next() },
                    "u" => if shift { self.focus_top() }    else { self.focus_block_top() },
                    "m" => if shift { self.focus_bottom() } else { self.focus_block_bottom() },
                    "j" => self.focus_group_prev(),
                    "k" => self.focus_group_next(),
                    "r" | "s" | "l" => self.scroll_armed = false,
                    _ => unreachable!(),
                }
            }
            _ if alt || self.scroll_armed => handled = false,

            "r" | "f" if ctrl => self.handle_refresh (cx),
            "g" if ctrl => self.toggle_group_mode(),
            "w" if ctrl => self.close_cur(),
            "p" if ctrl => self.peek_cur(),
            "a" if ctrl => self.select_all_search(),
            "c" if ctrl => self.copy_search_selection (cx),
            "x" if ctrl => self.cut_search_selection (cx),
            "v" if ctrl => {
                if let Some (text) = cx.read_from_clipboard().and_then (|c| c.text()) {
                    self.insert_search_text (&text);
                }
            }

            "backspace" => if self.in_search { self.backspace_search() } else { handled = false },

            _ if !alt && !ctrl && !self.scroll_armed => {
                if let Some (ch) = ks.key_char.as_ref().filter (|c| !c.is_empty()) {
                    self.insert_search_text (ch);
                } else {
                    handled = false;
                }
            }
            _ => handled = false,
        }

        if handled { cx.notify() }
    }

}
