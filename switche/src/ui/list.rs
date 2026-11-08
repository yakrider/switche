use std::ops::Range;

use gpui::prelude::*;
use gpui::{div, px, rgb, rgba, AnyElement, Context, CursorStyle, FontWeight, ImageSource, MouseButton, MouseDownEvent, MouseMoveEvent, SharedString, StyledText, Window};

use crate::search::{check_search_exe_title, highlight_ranges, split_highlighted};

use super::*;


impl SwitcheApp {
    pub(super) fn render_header (&self, elem_t:ElemT) -> AnyElement {
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

    pub(super) fn render_row (&self, row:&Row, window:&Window, cx:&mut Context<Self>) -> AnyElement {
        let Some (wde) = self.hwnd_map.get (&row.id.hwnd) else {
            return div().h(px(ROW_H)).into_any_element()
        };

        let id = row.id;
        // we'll maintain matching highlights in the dimmed recents block during search for better usability
        let selected = self.cur_elem == Some (id)
            || (self.in_search && self.group_mode
                && row.elem_t == ElemT::GR
                && self.cur_elem.is_some_and (|c| c.hwnd == id.hwnd));

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
                if this.menu_open || this.help_visible() { return }
                this.cur_elem = Some (id);
                this.activate_cur();
                cx.notify();
            }))
            .on_mouse_down (MouseButton::Middle, cx.listener (move |this, _ev:&MouseDownEvent, _window, cx| {
                if this.menu_open || this.help_visible() { return }
                this.cur_elem = Some (id);
                this.close_cur();
                cx.notify();
            }))
            .id (SharedString::from (format!("row-{}-{:?}", id.hwnd.0, id.block)))
            .on_hover (cx.listener (move |this, hovered:&bool, _window, cx| {
                if this.menu_open || this.help_visible() { return }
                if *hovered { this.hover_row_entered (id, cx) }
                else { this.hover_row_left (id) }
            }))
            .on_mouse_move (cx.listener (move |this, _ev:&MouseMoveEvent, _window, cx| {
                if this.menu_open || this.help_visible() { return }
                this.hover_row_entered (id, cx);
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
            .when (row.dimmed, |d| d.opacity(0.75))
            .into_any_element()
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

