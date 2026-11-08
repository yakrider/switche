use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::*;


impl SwitcheApp {
    /// rate-limits navigation steps (30ms in release builds) so the highlight/caret
    /// .. movement stays visually trackable even under a flood of key-repeat events
    pub(super) fn allow_nav_action (&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since (self.last_nav_stamp) < Duration::from_millis (self.nav_repeat_ms) { return false }
        self.last_nav_stamp = now;
        true
    }




    // ---------------- focus / nav ----------------

    pub(super) fn reset_focus (&mut self) {
        self.menu_open = false;
        self.cur_elem = self.recents_ids.first().copied();
        self.focus_next();
    }
    pub(super) fn reset_search_match_focus (&mut self) {
        let block = if self.group_mode { ElemT::G } else { ElemT::R };
        let id = self.block_ids (block) .first().copied();
        self.set_cur (id);
    }
    pub(super) fn set_cur (&mut self, id:Option<ElemId>) {
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

    pub(super) fn pick_next_elem (&self, reverse:bool, grp_next:bool, wrap_blocks:bool) -> Option <ElemId> {
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

    pub(super) fn focus_next (&mut self)       { let id = self.pick_next_elem (false, false, true); self.set_cur (id) }
    pub(super) fn focus_prev (&mut self)       { let id = self.pick_next_elem (true,  false, true); self.set_cur (id) }
    pub(super) fn focus_group_next (&mut self) { let id = self.pick_next_elem (false, true,  true); self.set_cur (id) }
    pub(super) fn focus_group_prev (&mut self) { let id = self.pick_next_elem (true,  true,  true); self.set_cur (id) }
    pub(super) fn focus_block_next (&mut self) { let id = self.pick_next_elem (false, false, false); self.set_cur (id) }
    pub(super) fn focus_block_prev (&mut self) { let id = self.pick_next_elem (true,  false, false); self.set_cur (id) }

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
    pub(super) fn focus_top (&mut self)        { self.focus_topbtm (true,  false) }
    pub(super) fn focus_bottom (&mut self)     { self.focus_topbtm (false, false) }
    pub(super) fn focus_block_top (&mut self)  { self.focus_topbtm (true,  true)  }
    pub(super) fn focus_block_bottom (&mut self) { self.focus_topbtm (false, true) }

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

    pub(super) fn resync_focus_after_rebuild (&mut self) {
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

}
