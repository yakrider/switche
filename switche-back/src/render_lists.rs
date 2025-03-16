#![ allow (non_camel_case_types) ]
#![ allow (non_snake_case) ]
#![ allow (non_upper_case_globals) ]

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::RwLock;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::{Duration, Instant};
use grouping_by::GroupingBy;
use serde::{Deserialize, Serialize};

use crate::switche::{Hwnd, ExePathName, WinDatEntry, SwitcheState};
use crate::config::Config;





# [ derive (Debug, Eq, PartialEq, Hash, Default, Copy, Clone, Serialize, Deserialize) ]
pub struct RenderListEntry {
    pub(crate) hwnd : Hwnd,
    pub(crate) y    : u32,
}

# [ derive (Debug, Eq, PartialEq, Hash, Default, Clone, Serialize, Deserialize) ]
pub struct RenderList_Pl {
    pub(crate) rl  : Vec <RenderListEntry>,
    pub(crate) grl : Vec <Vec <RenderListEntry>>,
}




# [ derive (Debug, Default, Copy, Clone) ]
pub struct GroupSortingEntry {
    seen_count: u32,
    mean_perc_idx: f32
}

# [ derive (Debug, Default) ]
pub struct RenderReadyListsManager {

    pub render_list      : RwLock <Vec <RenderListEntry>>,
    pub render_hwnds     : RwLock <HashSet <Hwnd>>,

    pub grp_sorting_map  : RwLock <HashMap <String, GroupSortingEntry>>,
    pub grpd_render_list : RwLock <Vec <Vec <RenderListEntry>>>,

    pub mru_hwnd_list    : RwLock <VecDeque <(Hwnd, Instant)>>,
    pub mru_hwnd_set     : RwLock <HashSet <Hwnd>>,

    pub exes_excl_map    : RwLock <HashMap <String, HashSet<String>>>,
    // ^^ map of exe-name to set of titles to exclude .. (read from user configs)

    pub ordering_ref_map : RwLock <HashMap <String, usize>>,
    // ^^ map of exe names to absolute ordering reference .. (read from user configs)
}




# [ derive (Debug, Default) ]
/// SnapListManager is used to grab a snapshot of the current rendering list hwnds and nav through them w/o bringing switche up
pub struct SnapListManager {
    snap_list : RwLock <Vec <Hwnd>>,
    cur_idx   : AtomicIsize,
}





impl RenderReadyListsManager {

    // -- creation --
    // ^^ happens via derived default



    // --- MRU tracking ---
    
    pub fn mru_list__register_fgnd_hwnd (&self, hwnd: Hwnd) {
        let mut mru_list = self.mru_hwnd_list.write().unwrap();
        mru_list .retain (|(h,_)| *h != hwnd);
        mru_list .push_front ((hwnd, Instant::now()));
        self.mru_hwnd_set.write().unwrap().insert(hwnd);
    }

    pub fn mru_list__post_win_enum_kick (&self, ss: &SwitcheState) {
        // first we'll clean up any hwnds no longer reported ..
        // however, for new windows (e.g. win-explorer), it can take a while after the fgnd report for the hwnd to show up in win-enums
        // so we'll need to allow a long enough grace before removing MRU recorded hwnds that dont show up in win-enum z-list

        let Ok(mut mru_list) = self.mru_hwnd_list.write() else { return };
        let Ok(mut mru_set) = self.mru_hwnd_set.write() else { return };

        let z_order_hwnds = ss.win_dats_m.hwnds_ordered.read().unwrap();
        let z_order_set: HashSet<Hwnd> = z_order_hwnds.iter().copied().collect();
        mru_list .retain (|(h,t)| {
            let zap_it = !z_order_set.contains(h) && t.elapsed() > Duration::from_millis(500);
            if zap_it { mru_set.remove(h); }
            !zap_it
        });

        // then we'll also try and catch any sent-to-back hwnd and mirror that in mru-list ..
        // we'll scan from the z-list bottom for the first hwnd that was already in our rendered list
        // and if that hwnd was also the MRU-list top, must have been a send-to-back operation, so we'll kick it off our MRU list !!
        let rl_hwnds = self.render_hwnds.read().unwrap();
        let rl0 = self.render_list .read().unwrap() .first() .map (|rle| rle.hwnd);
        for &z_hwnd in z_order_hwnds.iter().rev() {
            if rl_hwnds.contains(&z_hwnd) {
                if Some(z_hwnd) == rl0 {
                    tracing::debug! ("@{:?} sent-to-back: {:?}", crate::win_dats::_stamp(), z_hwnd.0);
                    mru_list.retain (|(h,_)| *h != z_hwnd);
                    mru_set.remove (&z_hwnd);
                }
                break;
            }
        }
    }


    // --- data reloads ---

    pub fn reload_exes_excl_set (&self, exes:Vec<String>) {
        // for any specified exe, we'll store either we have empty set, or a set of values ..
        // empty set will mean we'll filter out anything that matches exe .. non-empty set will mean we'll have to match title to one of those
        let mut ees = self.exes_excl_map.write().unwrap();
        ees.clear();
        exes.into_iter() .for_each (|e| {
            if let Some ((exe, title)) = e.split_once(';') {
                let title = title.to_lowercase();
                let title = if title == "`" { "".into() } else { title };
                if let Some(prior) = ees.get_mut(exe) {
                    prior.insert (title);
                } else {
                    ees .insert (exe.to_string(), HashSet::from ([title]));
                }
            } else {
                ees.insert (e, HashSet::default());
            }
        } )
    }
    pub fn reload_ordering_ref_map (&self, exes:Vec<String>) {
        let mut orm = self.ordering_ref_map.write().unwrap();
        orm.clear();
        exes .into_iter() .enumerate() .for_each (|(i,s)| { orm.insert(s,i); });
        if !orm.contains_key(Config::UNKNOWN_EXE_STR) {
            let next_idx = orm.len();
            orm.insert(Config::UNKNOWN_EXE_STR.to_string(), next_idx);
        }
    }



    // --- rendering exclusions ---

    pub fn calc_excl_flag (&self, ss:&SwitcheState, wde:&WinDatEntry) -> bool {
        //wde.is_vis == Some(false) ||  wde.is_uncloaked == Some(false) ||    // already covered during enum-filtering
        // ss.check_self_hwnd(wde.hwnd) || wde.win_text.is_none() ||
        // ^^ no good reason to exclude legit windows w/o titles
        ss.check_self_hwnd(wde.hwnd) ||
            wde.exe_path_name.as_ref() .filter (|p| !p.full_path.is_empty()) .is_none() ||
            wde.exe_path_name.as_ref() .filter (|p| !self.exes_excl_check (&p.name, wde)) .is_none()
    }
    pub fn runtime_should_excl_check (&self, ss:&SwitcheState, wde:&WinDatEntry) -> bool {
        wde.should_exclude .unwrap_or_else ( move || self.calc_excl_flag(ss,wde) )
    }
    fn exes_excl_check (&self, exe:&str, wde:&WinDatEntry) -> bool {
        self.exes_excl_map.read().unwrap().get(exe) .is_some_and ( |ot|
            // we exclude if exe matches and no title was specified in config, or if title matches too
            ot.is_empty() ||
                ( wde.win_text.as_ref().is_none()  &&  ot.contains("") ) ||
                wde.win_text.as_ref().is_some_and (|t| ot.contains(&t.to_lowercase()))
        )
    }



    // --- groups auto-ordering registry ----

    pub(crate) fn update_entry (&self, exe_path:&str, ge:GroupSortingEntry, perc_idx:f32) {
        let mean_perc_idx = (ge.mean_perc_idx * ge.seen_count as f32 + perc_idx) / (ge.seen_count + 1) as f32;
        self.grp_sorting_map.write().unwrap() .insert (
            exe_path.to_owned(), GroupSortingEntry { seen_count: ge.seen_count+1, mean_perc_idx }
        );
    }

    pub(crate) fn register_entry (&self, exe_path:&String, idx:usize, list_size:u32, do_update:bool) {
        let perc_idx : f32 = (idx as f32) / list_size as f32;
        if !self.grp_sorting_map.read().unwrap().contains_key(exe_path) {
            self.update_entry (exe_path, GroupSortingEntry::default(), perc_idx);
        } else if do_update {
            let ge = self.grp_sorting_map.read().unwrap() .get(exe_path) .copied(); // split to end read scope
            ge .iter() .for_each (|ge| self.update_entry (exe_path, *ge, perc_idx))
        }
    }

    pub(crate) fn clear_grouping (&self) {
        self.grp_sorting_map.write().unwrap().clear()
    }



    // --- render lists calculations ---

    pub(crate) fn recalc_render_ready_lists (&self, ss:&SwitcheState) -> (Vec<RenderListEntry>, Vec<Vec<RenderListEntry>>) {

        struct RenderListEntryInfo<'a> { exe_path_name: Option <&'a ExePathName>, ico_idx: i32, rle: RenderListEntry }

        let is_dismissed = ss.is_dismissed.check();     // local copy to avoid guarded accesses in a loop
        let hwnd_map = ss.win_dats_m.hwnd_map.read().unwrap();
        
        // we want to preserve mru ordering, then append any window not in mru list by z-order
        let mru_list = self.mru_hwnd_list.read().unwrap();
        let mru_set = self.mru_hwnd_set.read().unwrap();
        let z_order_hwnds = ss.win_dats_m.hwnds_ordered.read().unwrap();
        
        let mut rl = Vec::with_capacity(z_order_hwnds.len());
        mru_list .iter() .for_each (|(h,_)| rl.push(*h));
        z_order_hwnds .iter().for_each (|h| if !mru_set.contains(h) { rl.push(*h); });

        // Filter the windows based on exclusion criteria
        let filt_wdes = rl.iter()
            .flat_map (|h| hwnd_map.get(h))
            .filter (|&wde| !self.runtime_should_excl_check(ss, wde))
            .collect::<Vec<_>>();

        // we'll gather all the info to sort the renderlist entries and their groups
        let filt_rle_info = filt_wdes .iter() .enumerate() .map ( |(i,wde)| {
            // we'll also register these while we're creating the RLE-infos
            wde.exe_path_name .as_ref() .map (|p| &p.full_path) .iter() .for_each ( |fp| {
                self.register_entry (fp, i, filt_wdes.len() as u32, is_dismissed)
            } );
            RenderListEntryInfo {
                exe_path_name : wde.exe_path_name.as_ref(),
                ico_idx       : ss.icons_m.get_cached_icon_idx(wde).map (|i| i as i32) .unwrap_or(-1),
                rle           : RenderListEntry { hwnd: wde.hwnd, y: 1+i as u32 }
                // ^^ note that renderlist entries are 1 based corresponding to how we want them shown in the UI
                // (and we calc and send from here instead of just wde vecs as grpd-render-list still should show recents-ordered/sorted idxs)
            }
        } ) .collect::<Vec<_>>();

        // for recents, we use the MRU order as reflected in the filt_rle_info
        let filt_rl = filt_rle_info .iter() .map (|e| e.rle) .collect::<Vec<_>>();

        let mut grpd_render_list_builder = filt_rle_info .iter() .grouping_by (|e| e.exe_path_name) .into_values() .collect::<Vec<_>>();

        // within each group, we want to keep the MRU order, but first sort by icon-count and icon-idx (mostly for things like chrome apps)
        let ico_freqs = filt_rle_info .iter() .fold ( HashMap::new(), |mut m, e| { *m.entry(e.ico_idx).or_insert(0) -= 1; m } );
        // ^^ we use negative counts so that we can sort by descending freq count

        use std::cmp::Ordering;
        grpd_render_list_builder .iter_mut() .for_each (|es| es.sort_unstable_by ( |a,b| {
            match ico_freqs.get(&a.ico_idx) .cmp (&ico_freqs.get(&b.ico_idx)) {  // by ico-freq
                Ordering::Equal => match a.ico_idx .cmp (&b.ico_idx) {  // by ico-idx
                    Ordering::Equal => a.rle.y .cmp (&b.rle.y),         // by z-index
                    ord => ord
                },
                ord => ord
            }
        } ) );

        // as for the groups themselves, we want to order by perc_idx if auto-order, else as listed in configs .. breaking ties by exe name and ico-idx
        // .. so we'll set up the appropirate comparison extractor for those two cases
        fn ext_cmp_auto (rrlm:&RenderReadyListsManager, po:&Option<&ExePathName> ) -> Option<f32> {
            let grm = rrlm.grp_sorting_map.read().unwrap();
            po .and_then (|p| grm .get(&p.full_path) .map (|gse| gse.mean_perc_idx) )
        }
        fn ext_cmp_manual (rrlm:&RenderReadyListsManager, po:&Option<&ExePathName> ) -> Option<f32> {
            let orm = rrlm.ordering_ref_map.read().unwrap();
            po .and_then (|p| orm .get(&p.name) .or (orm.get(Config::UNKNOWN_EXE_STR)) .map (|v| *v as f32) )
        }
        let cmp_ext = if ss.conf.check_flag__auto_order_window_groups() { ext_cmp_auto } else { ext_cmp_manual };

        grpd_render_list_builder .sort_unstable_by ( |ga, gb| {
            // we'll break ties w exe path so there's no instability in ui ordering when two groups have equal perc_idx
            let (g_epn_a, g_epn_b) = (ga.first().and_then(|e| e.exe_path_name), gb.first().and_then(|e| e.exe_path_name));
            match  cmp_ext (self, &g_epn_a) .partial_cmp ( &cmp_ext (self, &g_epn_b) ) .unwrap() {
                // note ^^ that unwrap is ok because it would fail only for NaN
                Ordering::Equal => {
                    let g_exe_a = ga.first() .and_then (|e| e.exe_path_name);
                    let g_exe_b = gb.first() .and_then (|e| e.exe_path_name);
                    match g_exe_a .map (|epn| &epn.name) .cmp (&g_exe_b.map(|epn| &epn.name)) {
                        Ordering::Equal => g_exe_a .map (|epn| &epn.full_path) .cmp (&g_exe_b.map(|epn| &epn.full_path)),
                        ord => ord
                    }
                },
                ord => ord
            }
        } );
        let grpd_render_list = grpd_render_list_builder .into_iter() .map ( |es| {
            es .into_iter() .map (|e| e.rle) .collect::<Vec<_>>()
        } ) .collect::<Vec<Vec<_>>>();
        //debug!("render-ready-list recalc -- rl:{:?}", filt_rl.len());
        ( filt_rl, grpd_render_list )
    }

    pub(crate) fn update_render_ready_lists (&self, ss:&SwitcheState) {
        let (rl, grl) = self.recalc_render_ready_lists (ss);
        *self.render_hwnds.write().unwrap() = HashSet::from_iter(rl.iter().map(|rle| &rle.hwnd).cloned());
        *self.render_list.write().unwrap() = rl;
        *self.grpd_render_list.write().unwrap() = grl;
    }


}







impl SnapListManager {

    pub fn capture (&self, rrlm: &RenderReadyListsManager) {
        *self.snap_list .write().unwrap() = rrlm.render_list.read().unwrap().iter() .map (|rle| rle.hwnd) .collect::<Vec<Hwnd>>();
        self.cur_idx.store (0, Ordering::Relaxed);
    }

    pub fn next_hwnd (&self) -> Option<Hwnd> {
        let sl = self.snap_list.read().unwrap();
        if sl.is_empty() { return None }
        let idx = (self.cur_idx.fetch_add(1, Ordering::Relaxed) + 1) .rem_euclid (sl.len() as isize);
        sl .get (idx as usize) .copied()
    }
    pub fn prev_hwnd (&self) -> Option<Hwnd> {
        let sl = self.snap_list.read().unwrap();
        if sl.is_empty() { return None }
        let idx = (self.cur_idx.fetch_sub(1, Ordering::Relaxed) - 1) .rem_euclid (sl.len() as isize);
        sl .get (idx as usize) .copied()
    }
    pub fn top_hwnd (&self) -> Option<Hwnd> {
        let sl = self.snap_list.read().unwrap();
        if sl.is_empty() { return None }
        self.cur_idx.store (0, Ordering::Relaxed);
        sl .first() .copied()
    }
    pub fn bottom_hwnd (&self) -> Option<Hwnd> {
        let sl = self.snap_list.read().unwrap();
        if sl.is_empty() { return None }
        let idx = self.snap_list.read().unwrap().len() - 1;
        self.cur_idx.store (idx as isize, Ordering::Relaxed);
        sl .get (idx) .copied()
    }

}

