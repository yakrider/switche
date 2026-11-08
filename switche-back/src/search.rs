#![allow (non_snake_case)]

use std::ops::Range;

use crate::switche::WinDatEntry_Pl;



#[derive (Debug, Default, Clone)]
pub struct SearchCheck {
    pub passed               : bool,
    pub frags                : Vec<String>,
    pub exe_matched_frags    : Vec<String>,
    pub y_matched            : bool,
}



/// Port of the frontend's search semantics:
///   - only one of the terms needs to match the exe name
///   - a term can match the shown z-index number (with or without a trailing 'i')
///   - all remaining terms must match the title
pub fn check_search_exe_title (wde:&WinDatEntry_Pl, search_text:&str, render_y:u32) -> SearchCheck {

    let frags : Vec<String> = search_text
        .split (' ')
        .filter (|s| !s.is_empty())
        .map (|s| s.to_lowercase())
        .collect();

    if frags.is_empty() {
        return SearchCheck { passed: true, frags, exe_matched_frags: Vec::new(), y_matched: false }
    }

    let exe_name = wde.exe_path_name.as_ref().map(|p| p.name.as_str()).unwrap_or("");
    let title    = wde.win_text.as_deref().unwrap_or("");
    let exe_l    = exe_name.to_lowercase();
    let title_l  = title.to_lowercase();
    let y_str    = render_y.to_string();
    let y_str_i  = format!("{}i", y_str);

    let mut exe_matched_frags : Vec<String> = Vec::new();
    let mut y_matched = false;
    let mut passed = true;

    for f in &frags {
        let is_exe = exe_l.contains (f.as_str());
        let is_y   = f.as_str() == y_str.as_str() || f.as_str() == y_str_i.as_str();
        if is_exe { exe_matched_frags.push (f.clone()) }
        if is_y   { y_matched = true }
        if !is_exe && !is_y && !title_l.contains (f.as_str()) { passed = false }
    }

    SearchCheck { passed, frags, exe_matched_frags, y_matched }
}



/// Returns merged, sorted byte-ranges of `text` that match any of the (already lowercased) fragments.
pub fn highlight_ranges (text:&str, frags:&[String]) -> Vec <Range<usize>> {

    if frags.is_empty() || text.is_empty() { return Vec::new() }

    let lower = text.to_lowercase();
    // for non-ascii lowercasing changes the byte length, so indices wouldn't line up with the original ..
    // .. we'll just forgo highlighting in those rare cases (matching itself still works via contains)
    if lower.len() != text.len() { return Vec::new() }

    let mut ranges : Vec <Range<usize>> = Vec::new();

    for frag in frags {
        if frag.is_empty() { continue }
        let mut start = 0;
        while start < lower.len() {
            let Some (pos) = lower [start..] .find (frag.as_str()) else { break };
            let s = start + pos;
            let e = s + frag.len();
            ranges.push (s..e);
            start = e;
        }
    }

    if ranges.is_empty() { return ranges }

    ranges.sort_by_key (|r| r.start);
    let mut merged : Vec <Range<usize>> = Vec::with_capacity (ranges.len());
    for r in ranges {
        if let Some (last) = merged.last_mut() {
            if r.start <= last.end {
                if r.end > last.end { last.end = r.end }
                continue
            }
        }
        merged.push (r);
    }
    merged
}



/// Splits `text` into (segment, is_match) pairs given merged highlight ranges.
pub fn split_highlighted <'a> (text:&'a str, ranges:&[Range<usize>]) -> Vec <(&'a str, bool)> {
    let mut out = Vec::with_capacity (ranges.len() * 2 + 1);
    let mut pos = 0;
    for r in ranges {
        if r.start > pos { out.push ((&text [pos..r.start], false)) }
        out.push ((&text [r.start..r.end], true));
        pos = r.end;
    }
    if pos < text.len() { out.push ((&text [pos..], false)) }
    out
}
