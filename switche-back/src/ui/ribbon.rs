use gpui::prelude::*;
use gpui::{div, px, rgb, rgba, AnyElement, ClickEvent, Context, CursorStyle, FontWeight, MouseButton, MouseDownEvent, SharedString, StyledText, Window};

use crate::switche::UiRequest;

use super::*;




impl SwitcheApp {
    fn render_menu_link (&self, cx:&mut Context<Self>) -> AnyElement {
        div()
            .w(px(36.)).h(px(RIBBON_H))
            .flex().flex_col().justify_center().items_center().gap(px(3.))
            .cursor(CursorStyle::PointingHand)
            .on_mouse_down (MouseButton::Left, cx.listener (|this, _ev:&MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                this.menu_open = !this.menu_open;
                if this.menu_open {
                    this.help_open = false;
                    this.help_hover = false;
                    this.help_hide_pending = false;
                    this.help_hide_gen = this.help_hide_gen.wrapping_add (1);
                    this.tooltip = None;
                }
                cx.notify();
            }))
            .child (div().w(px(16.)).h(px(2.)).bg(rgb(C_RIBBON)))
            .child (div().w(px(16.)).h(px(2.)).bg(rgb(C_RIBBON)))
            .child (div().w(px(16.)).h(px(2.)).bg(rgb(C_RIBBON)))
            .into_any_element()
    }

    fn indicator (&self, symbol:&str, on:bool, on_color:u32, tip:TooltipKind, cx:&mut Context<Self>) -> AnyElement {
        let color = if on { on_color } else { C_DIM };
        div()
            .w(px(32.)).flex_none().flex().justify_center()
            .id (SharedString::from (format!("ind-{:?}", tip)))
            .on_hover (cx.listener (move |this, hovered:&bool, _window, cx| {
                if *hovered {
                    if tip != TooltipKind::Help && !this.menu_open { this.tooltip = Some (tip) }
                    if tip == TooltipKind::Help {
                        this.help_hover = true;
                        this.help_hide_pending = false;
                        this.help_hide_gen = this.help_hide_gen.wrapping_add (1);
                    }
                } else {
                    if this.tooltip == Some (tip) { this.tooltip = None }
                    if tip == TooltipKind::Help { this.schedule_help_hide (cx) }
                }
                cx.notify()
            }))
            .on_mouse_down (MouseButton::Left, cx.listener (move |this, _ev:&MouseDownEvent, _window, cx| {
                if tip == TooltipKind::Help {
                    cx.stop_propagation();
                    let now_open = !this.help_open;
                    this.help_open = now_open;
                    if !now_open {
                        this.help_hover = false;
                        this.help_hide_pending = false;
                        this.help_hide_gen = this.help_hide_gen.wrapping_add (1);
                    }
                    this.menu_open = false;
                    cx.notify()
                }
            }))
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
            .on_mouse_down (MouseButton::Left, cx.listener (|_this, _ev:&MouseDownEvent, _window, _cx| {
                ss().handle_ui_request (UiRequest::WindowDragStart)
            }))
            .id (SharedString::from ("drag-indicator"))
            .on_hover (cx.listener (|this, hovered:&bool, _window, cx| {
                if *hovered { this.tooltip = Some (TooltipKind::Drag) }
                else if this.tooltip == Some (TooltipKind::Drag) { this.tooltip = None }
                cx.notify()
            }))
            .on_click (cx.listener (|_this, ev:&ClickEvent, _window, cx| {
                if ev.click_count() == 2 {
                    ss().handle_ui_request (UiRequest::SelfAutoResize);
                    // the window is moved by win32; make sure gpui repaints the newly exposed area
                    cx.notify();
                }
            }))
            .child("✥")
            .into_any_element()
    }

    /// returns (x-offset, line height) for the caret, measured from the shaped prefix
    fn search_view_width (&self, window:&Window) -> f32 {
        // approx width of the search text viewport after the ribbon's fixed children
        (f32::from (window.bounds().size.width) - 410.0).max(80.0)
    }

    fn search_text_width (&self, window:&Window) -> f32 {
        let text = &self.search_text;
        if text.is_empty() { return 0.0 }
        let base = window.text_style();
        let mut run = base.clone().to_run (text.len());
        run.font = gpui::font ("Segoe UI");
        let shaped = window.text_system().shape_line (text.clone().into(), px(SEARCH_SZ), &[run], None);
        f32::from (shaped.width)
    }

    /// horizontally scrolls the search text so the caret stays visible, like an input field
    pub(super) fn update_search_scroll (&mut self, window:&Window) {
        if !self.in_search { self.search_scroll_x = 0.0; return }
        let view_w = self.search_view_width (window);
        let text_w = self.search_text_width (window);
        let (caret_x, _) = self.search_caret_metrics (window);
        let margin = 12.0;
        let max_scroll = (text_w + 6.0 - view_w).max (0.0);
        if caret_x < self.search_scroll_x + margin {
            self.search_scroll_x = (caret_x - margin).max (0.0);
        } else if caret_x > self.search_scroll_x + view_w - margin {
            self.search_scroll_x = caret_x - view_w + margin;
        }
        self.search_scroll_x = self.search_scroll_x.clamp (0.0, max_scroll);
    }

    fn search_caret_metrics (&self, window:&Window) -> (f32, f32) {
        let text = &self.search_text;
        let caret = self.search_caret.min (text.len());
        let prefix = &text [..caret];
        let base = window.text_style();
        // shape a space when the prefix is empty, just to get the font's line metrics
        let (measure, run_len) = if prefix.is_empty() {
            (" ".to_string(), 1)
        } else {
            (prefix.to_string(), prefix.len())
        };
        let mut run = base.clone().to_run (run_len);
        run.font = gpui::font ("Segoe UI");
        let shaped = window.text_system().shape_line (
            measure.into(), px(SEARCH_SZ), &[run], None
        );
        let width  = if prefix.is_empty() { 0.0 } else { f32::from (shaped.width) };
        let height = (f32::from (shaped.ascent) + f32::from (shaped.descent)).max (SEARCH_SZ);
        (width, height)
    }

    fn render_search (&self, window:&Window) -> AnyElement {
        let count = if self.match_count > 0 { self.match_count.to_string() } else { String::new() };
        let text = &self.search_text;
        let sel = self.search_selection();

        let text_el = if let Some ((a, b)) = sel {
            let base = window.text_style();
            let mut runs : Vec <TextRun> = Vec::new();
            let mut push = |len:usize, selected:bool| {
                if len == 0 { return }
                let mut run = base.clone().to_run (len);
                run.font = gpui::font ("Segoe UI");
                if selected {
                    run.color = rgb(C_BG).into();
                    run.background_color = Some (rgb(C_SEARCH).into());
                } else {
                    run.color = rgb(C_SEARCH).into();
                }
                runs.push (run);
            };
            push (a, false);
            push (b - a, true);
            push (text.len() - b, false);
            div().text_size(px(SEARCH_SZ)).child (StyledText::new (text.clone()) .with_runs (runs)) .into_any_element()
        } else {
            div().text_size(px(SEARCH_SZ)).text_color(rgb(C_SEARCH)).child(text.clone()).into_any_element()
        };

        let (caret_x, caret_h) = if self.in_search { self.search_caret_metrics (window) } else { (0.0, SEARCH_SZ) };
        let mut text_row = div().relative().left(px(-self.search_scroll_x))
            .flex().flex_row().items_center().child (text_el);
        if self.in_search {
            text_row = text_row.child (
                div().absolute().left(px(caret_x)).top(px(0.)).w(px(2.)).h(px(caret_h)).bg(rgb(C_CUR_TEXT))
            );
        }

        div()
            .flex_1().flex().flex_row().items_center().pl(px(12.))
            .child (
                div().flex_1().min_w(px(0.)).overflow_hidden().child (text_row)
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

    pub(super) fn render_dropdown (&self, cx:&mut Context<Self>) -> AnyElement {
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

    fn tooltip_lines (&self, kind:TooltipKind) -> Vec <String> {
        match kind {
            TooltipKind::AltTab => vec![format! ("Alt-Tab Replacement : {}",
                if self.configs.alt_tab_enabled { "Enabled" } else { "Disabled" })],
            TooltipKind::RbtnWheel => vec![format! ("Mouse Right Btn with Wheel Activation : {}",
                if self.configs.rbtn_whl_enabled { "Enabled" } else { "Disabled" })],
            TooltipKind::AutoOrder => vec![format! ("Groups Ordering is : {}",
                if self.configs.grp_ordering_is_auto { "Auto" } else { "From-Configs (Not Auto)" })],
            TooltipKind::Elev => vec![format! ("Running Elevated : {}",
                if self.configs.is_elevated { "Yes" } else { "No" })],
            TooltipKind::Armed => vec![format! ("Key Release Activation : {}",
                if self.scroll_armed { "Active" } else { "Inactive" })],
            TooltipKind::Drag => vec!["Click and hold here to drag.".to_string(), "Double-click to Auto-Size".to_string()],
            TooltipKind::Counts => vec!["Listed Windows Counts".to_string()],
            TooltipKind::Warn => vec!["No Warnings or Errors".to_string()],
            TooltipKind::Help => Vec::new(),
        }
    }

    fn tooltip_left (&self, kind:TooltipKind) -> Option <f32> {
        Some (match kind {
            TooltipKind::AltTab    => 66.0,
            TooltipKind::RbtnWheel => 98.0,
            TooltipKind::AutoOrder => 130.0,
            TooltipKind::Elev      => 162.0,
            TooltipKind::Armed     => 194.0,
            TooltipKind::Drag      => 226.0,
            TooltipKind::Counts    => 258.0,
            TooltipKind::Warn | TooltipKind::Help => return None,
        })
    }

    pub(super) fn render_tooltip (&self) -> Option <AnyElement> {
        if self.menu_open { return None }
        let kind = self.tooltip?;
        if kind == TooltipKind::Help { return None }
        let mut tip = div().absolute().top(px(ROOT_PAD + RIBBON_H));
        match self.tooltip_left (kind) {
            Some (x) => tip = tip.left(px(12. + x)),
            None      => tip = tip.right(px(10.)),
        }
        let mut content = div()
            .w(px(340.)).bg(rgb(C_TOOLTIP_BG)).shadow_lg()
            .px(px(12.)).py(px(8.))
            .text_size(px(12.)).text_color(rgb(C_TOOLTIP))
            .flex().flex_col().gap(px(2.));
        for line in self.tooltip_lines (kind) { content = content.child (line) }
        Some (tip.child (content) .into_any_element())
    }

    pub(super) fn help_visible (&self) -> bool {
        (self.help_open || self.help_hover) && !self.menu_open
    }


    pub(super) fn render_ribbon (&self, cx:&mut Context<Self>, window:&Window) -> AnyElement {
        let mut ribbon = div()
            .relative().w_full().flex().flex_row().items_center().h(px(RIBBON_H))
            .text_color(rgb(C_RIBBON));

        ribbon = ribbon
            .child (self.render_menu_link (cx))
            .child (self.indicator ("A", self.configs.alt_tab_enabled, C_ALT_TAB, TooltipKind::AltTab, cx))
            .child (self.indicator ("M", self.configs.rbtn_whl_enabled, C_RBTN_WHL, TooltipKind::RbtnWheel, cx))
            .child (self.indicator ("⇅", self.configs.grp_ordering_is_auto, C_RIBBON, TooltipKind::AutoOrder, cx))
            .child (self.indicator ("⚿", self.configs.is_elevated, C_ELEV, TooltipKind::Elev, cx))
            .child (self.indicator ("R", self.scroll_armed, C_ARMED, TooltipKind::Armed, cx))
            .child (self.render_drag_indicator (cx))
            .child (
                div().w(px(40.)).flex_none().text_center().text_color(rgb(C_RIBBON))
                    .id (SharedString::from ("ind-counts"))
                    .on_hover (cx.listener (|this, hovered:&bool, _window, cx| {
                        if *hovered { this.tooltip = Some (TooltipKind::Counts) }
                        else if this.tooltip == Some (TooltipKind::Counts) { this.tooltip = None }
                        cx.notify()
                    }))
                    .child(format!("({})", self.render_list.len()))
            )
            .child (self.render_search (window))
            .child (self.indicator ("!", false, C_WARN, TooltipKind::Warn, cx))
            .child (self.indicator ("?", false, C_HELP, TooltipKind::Help, cx));

        ribbon.into_any_element()
    }
}
