use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, point, px, rgb, AnyElement, Context, CursorStyle, FontWeight, MouseButton, MouseDownEvent, SharedString, Window};

use super::*;


#[derive (Clone, Copy, Debug, PartialEq, Eq)]
enum HelpItemKind { Title, Para, Section, Bullet }


impl SwitcheApp {
    /// keeps the help panel up briefly after leaving the "?", so the mouse can travel into it
    pub(super) fn schedule_help_hide (&mut self, cx:&mut Context<Self>) {
        if self.help_open { return }
        self.help_hide_gen = self.help_hide_gen.wrapping_add (1);
        let gen = self.help_hide_gen;
        self.help_hide_pending = true;
        let (tx, rx) = async_channel::bounded (1);
        std::thread::spawn ( move || {
            std::thread::sleep (Duration::from_millis (350));
            let _ = tx.send_blocking (());
        } );
        cx.spawn (async move |this, cx| {
            let _ = rx.recv().await;
            let _ = this.update (cx, |this, cx| {
                if this.help_hide_pending && this.help_hide_gen == gen {
                    this.help_hide_pending = false;
                    if !this.help_open {
                        this.help_hover = false;
                        cx.notify();
                    }
                }
            });
        }) .detach();
    }

    fn help_section (&self, text:&str) -> AnyElement {
        div().w_full().min_w(px(0.)).pt(px(6.)).text_size(px(13.)).font_weight(FontWeight::BOLD).child(text.to_string()).into_any_element()
    }
    fn help_para (&self, text:&str) -> AnyElement {
        div().w_full().min_w(px(0.)).child(text.to_string()).into_any_element()
    }
    fn help_bullet (&self, text:&str) -> AnyElement {
        div().w_full().min_w(px(0.)).flex().flex_row().items_start().gap(px(4.))
            .child (div().flex_none().child("•"))
            .child (div().flex_1().min_w(px(0.)).child(text.to_string()))
            .into_any_element()
    }

    fn help_items (&self) -> Vec <(HelpItemKind, String)> {
        let v = self.configs.switche_version;
        let mut items : Vec <(HelpItemKind, String)> = Vec::new();
        items.push ((HelpItemKind::Title, format! ("Switche - A Searchable Task Switcher   (v{v})")));
        for s in [
            "Switche is designed to be a fast, ergonomic, and search-first task switcher.",
            "Configs are stored in a switche.conf.toml which can be opened via the menu.",
            "The default config file has reasonable settings and explanations for each config.",
            "Some other key features are explained below:",
        ] { items.push ((HelpItemKind::Para, s.to_string())) }

        items.push ((HelpItemKind::Section, "Search".into()));
        for s in [
            "Starting to type will auto-activate search mode and filtering",
            "Partial words can be entered separated by spaces, and will be matched separately",
            "Search matching operates on the exe filename, window title, and the z-index shown",
            "Escape exits search mode, Ctrl-Space activates selected item during search mode",
            "Other general navigation hotkeys as listed below work in search mode as well",
        ] { items.push ((HelpItemKind::Bullet, s.to_string())) }

        items.push ((HelpItemKind::Section, "Invocation options".into()));
        for s in [
            "Hotkeys to invoke Switche can be changed in configs, default and recommended is F1",
            "Alternately, can use Alt-Tab combo, or use mouse-wheel while holding mouse-right-btn",
            "If so, releasing the Alt key or the right-button will activate the selected window",
            "The Release-Armed indicator lights up when that is the case.",
            "To disarm the state, press S, L, or Space key, or click empty space in the window",
        ] { items.push ((HelpItemKind::Bullet, s.to_string())) }

        items.push ((HelpItemKind::Section, "General Hotkeys".into()));
        for s in [
            "In the menu dropdown: Ctrl+R: Refresh, F5: Reload, Ctrl+G: Group Mode, Alt+F4: Quit",
            "Note that Refresh re-queries the windows list, and updates icons.",
            "Reload will additionally reload the UI, reset kbd/mouse hooks, and reload configs.",
            "Enter, Space, Mouse-Click : Activate the window currently selected in switche list",
            "Esc : Escape out of search-mode if active, or else from switche window",
            "Tab, Arrow keys, PgUp, PgDown : Navigate the list of windows",
            "Alt + [ I, J, K, M, U, Comma ] : Navigate the list of windows",
            "Ctrl+W, mouse middle-click : Close the selected window",
            "Ctrl+P : Peek at the selected window for a few seconds",
            "Other hotkeys for direct switching to specific-applications can be setup in configs.",
        ] { items.push ((HelpItemKind::Bullet, s.to_string())) }

        items.push ((HelpItemKind::Section, "Windows Listings".into()));
        for s in [
            "Recents section shows a subset of windows ordered by most recently in foreground",
            "The number shown here is configurable, but the minimum is 2 for quick switching",
            "The intended focus is on the Grouped section where windows are organized by exe",
            "The ordering of the grouped section can be set to auto or specified in configs",
            "Auto-ordering is based on a running tally of the average z-index of each group",
            "Windows can be specified in configs by exe and/or title to be excluded from the lists.",
        ] { items.push ((HelpItemKind::Bullet, s.to_string())) }

        items.push ((HelpItemKind::Section, "Special Considerations".into()));
        for s in [
            "When not elevated/as-admin, Switche can not close/switch-to elevated app windows.",
            "Running elevated is recommended. The Elevation indicator serves as visual reminder.",
            "Switche system tray menu can setup auto-start at login (as elevated or normal user).",
            "The option for Auto-Start-as-Admin is disabled when not running elevated/as-admin.",
            "Clearing prior configs in Windows Task Scheduler can help with auto-start issues.",
            "Other apps using lower level kbd hooks can prevent Switche from receiving Alt-Tab.",
            "Mutiple Switche instances running simultaneously can lead to unexpected behavior.",
        ] { items.push ((HelpItemKind::Bullet, s.to_string())) }

        items.push ((HelpItemKind::Para, "Repository:  https://github.com/yakrider/switche".into()));
        items.push ((HelpItemKind::Para, "Copyright @ 2024: yakrider@gmail.com ... Distributed under MIT license.".into()));
        items
    }

    fn measure_help_line (window:&Window, text:&str, size:f32, bold:bool) -> f32 {
        if text.is_empty() { return 0.0 }
        let base = window.text_style();
        let mut run = base.clone().to_run (text.len());
        run.font = gpui::font ("Segoe UI");
        if bold { run.font.weight = FontWeight::BOLD }
        let shaped = window.text_system().shape_line (text.to_string().into(), px(size), &[run], None);
        f32::from (shaped.width)
    }

    fn help_panel_width (&mut self, window:&Window, max_w:f32) -> f32 {
        let text_w = match self.help_text_w {
            Some (w) => w,
            None => {
                let mut widest = 0.0f32;
                for (kind, text) in self.help_items() {
                    let w = match kind {
                        HelpItemKind::Title   => Self::measure_help_line (window, &text, 14.0, true),
                        HelpItemKind::Section => Self::measure_help_line (window, &text, 13.0, true),
                        HelpItemKind::Para    => Self::measure_help_line (window, &text, 12.0, false),
                        HelpItemKind::Bullet  => Self::measure_help_line (window, &text, 12.0, false)
                            + Self::measure_help_line (window, "•", 12.0, false) + 4.0,
                    };
                    widest = widest.max (w);
                }
                // implausible measurement (e.g. first frame before the text system is ready): retry later
                if widest < 200.0 { return max_w }
                self.help_text_w = Some (widest);
                widest
            }
        };
        // add padding (8 + 10), scrollbar room (4), and a little slack (10)
        (text_w + 32.0).min (max_w).max (max_w.min (480.0))
    }

    /// moves the help text while its scrollbar thumb is being dragged
    pub(super) fn help_drag_move (&mut self, cursor_y:f32) {
        let Some ((y0, off0)) = self.help_drag else { return };
        let view_h = f32::from (self.help_scroll.bounds().size.height);
        let max = f32::from (self.help_scroll.max_offset().height);
        let thumb_h = (view_h * view_h / (view_h + max)).clamp (12.0, view_h);
        let track_h = view_h - thumb_h;
        if max <= 0.5 || track_h <= 0.0 { return }
        let off = self.help_scroll.offset();
        let new_y = (off0 - (cursor_y - y0) / track_h * max).clamp (-max, 0.0);
        self.help_scroll.set_offset (point (off.x, px (new_y)));
    }

    pub(super) fn render_help_panel (&mut self, window:&Window, cx:&mut Context<Self>) -> AnyElement {
        let top = ROOT_PAD + RIBBON_H;
        let panel_h = (f32::from (window.bounds().size.height) - top - 12.0).max(0.0);
        let panel_max_w = (f32::from (window.bounds().size.width) - 12.0).max(320.0);
        let panel_w = self.help_panel_width (window, panel_max_w);

        let mut content = div().id (SharedString::from ("help-scroll")).size_full()
            .overflow_y_scroll().scrollbar_width(px(4.))
            .track_scroll (&self.help_scroll)
            .p(px(8.)).pr(px(10.)).flex().flex_col().gap(px(4.))
            .text_size(px(12.)).text_color(rgb(C_TOOLTIP));

        for (kind, text) in self.help_items() {
            content = content.child (match kind {
                HelpItemKind::Title => div().w_full().min_w(px(0.)).text_size(px(14.)).font_weight(FontWeight::BOLD)
                    .child(text).into_any_element(),
                HelpItemKind::Para    => self.help_para (&text),
                HelpItemKind::Section => self.help_section (&text),
                HelpItemKind::Bullet  => self.help_bullet (&text),
            });
        }

        // gpui reserves space for a scrollbar but doesn't draw one, so we draw a thumb
        // .. and make it draggable by feeding the drag delta back into the scroll handle
        let max_scroll = f32::from (self.help_scroll.max_offset().height);
        let thumb = (max_scroll > 0.5).then (|| {
            let thumb_h = (panel_h * panel_h / (panel_h + max_scroll)).clamp (12.0, panel_h);
            let frac = (-f32::from (self.help_scroll.offset().y) / max_scroll).clamp (0.0, 1.0);
            (frac * (panel_h - thumb_h), thumb_h)
        });

        let mut panel = div()
            .absolute().top(px(top)).right(px(0.))
            .w(px(panel_w)).h(px(panel_h)).overflow_hidden()
            .bg(rgb(C_TOOLTIP_BG)).shadow_lg()
            .id (SharedString::from ("help-panel"))
            .on_hover (cx.listener (|this, hovered:&bool, _window, cx| {
                if *hovered {
                    this.help_hover = true;
                    this.help_hide_pending = false;
                    this.help_hide_gen = this.help_hide_gen.wrapping_add (1);
                } else {
                    this.schedule_help_hide (cx);
                }
                cx.notify()
            }))
            .on_mouse_down (MouseButton::Left, |_ev:&MouseDownEvent, _window, cx| { cx.stop_propagation() })
            .child (content);
        if let Some ((thumb_top, thumb_h)) = thumb {
            panel = panel.child (
                // 8px grab zone at the right edge, with the 4px bar painted inside it
                div().absolute().right(px(0.)).top(px(thumb_top)).w(px(8.)).h(px(thumb_h))
                    .flex().justify_end()
                    .cursor(CursorStyle::ResizeUpDown)
                    .on_mouse_down (MouseButton::Left, cx.listener (|this, ev:&MouseDownEvent, _window, cx| {
                        cx.stop_propagation();
                        this.help_drag = Some ((f32::from (ev.position.y), f32::from (this.help_scroll.offset().y)));
                        cx.notify()
                    }))
                    .child (
                        div().w(px(4.)).h_full().bg(rgb(C_SCROLL_THUMB)).rounded_sm()
                    )
            );
        }
        panel.into_any_element()
    }
}
