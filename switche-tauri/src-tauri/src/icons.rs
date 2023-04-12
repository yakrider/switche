

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::ops::Deref;
use std::string::ToString;
use std::sync::{Arc};
use std::sync::{RwLock};
//use no_deadlocks::RwLock;
use std::thread::{sleep, spawn};
use std::time::{Duration};
use std::mem;

// todo recheck whether on no-deadlocks

use once_cell::sync::OnceCell;
use rand::Rng;
use windows::Win32::Foundation::HWND;



use crate::*;



#[derive (Debug, Default, Eq, PartialEq, Hash, Clone)]
struct HwndExePathPair { hwnd:Hwnd, path:String }
// todo ^^ prob could make this hold just the hash of path all throughout this module

#[derive (Debug, Default, Eq, PartialEq, Hash, Copy, Clone)]
struct IconCacheMapping { cache_idx:usize, is_stale:bool }

# [ derive ( ) ]
pub struct _IconsManager {

    // we'll store icons in a vec and just add remove mappings to its indices for associated hwnds
    icons_cache        : Arc <RwLock <Vec <String>>>,
    // we'll use hashes of icons as map-key for reverse lookup
    icons_idx_map      : Arc <RwLock <HashMap <String, usize>>>,

    // for hwnds, key needs to incl the exe-path as a hpp pair
    icons_hpp_map      : Arc <RwLock <HashMap <HwndExePathPair, IconCacheMapping>>>,
    // for exes, icons dont go stale, so directly mapping to icons idx
    icons_exe_map      : Arc <RwLock <HashMap <String, usize>>>,

    // we'll need a reverse lookup table from exe paths to check if they were queried
    queried_exe_cache  : Arc <RwLock <HashSet <String>>>,
    // but for hpps, since cache map key is hwnd+path, we'll store that as a map and compare against it (alt coudld use an hpp set)
    // (that will also ensure only one hpp for a hwnd is in the map as it should be)
    queried_hpp_cache  : Arc <RwLock <HashMap <Hwnd, HwndExePathPair>>>,

}

# [ derive (Clone) ]
pub struct IconsManager ( Arc <_IconsManager> );

impl Deref for IconsManager {
    type Target = _IconsManager;
    fn deref(&self) -> &_IconsManager { &self.0 }
}



impl IconsManager {

    pub fn instance () -> IconsManager {
        static INSTANCE: OnceCell <IconsManager> = OnceCell::new();
        INSTANCE .get_or_init ( || {
            //let icons_mgr = IconsManager ( Arc::new ( _IconsManager::default() ) );
            // ^^ when trying no-deadlock, cant do that as it doesnt impl default
            let icons_mgr = IconsManager ( Arc::new ( _IconsManager {
                icons_cache        : Arc::new(RwLock::new(Default::default())),
                icons_idx_map      : Arc::new(RwLock::new(Default::default())),
                icons_hpp_map      : Arc::new(RwLock::new(Default::default())),
                icons_exe_map      : Arc::new(RwLock::new(Default::default())),
                queried_exe_cache  : Arc::new(RwLock::new(Default::default())),
                queried_hpp_cache  : Arc::new(RwLock::new(Default::default())),
            } ) );
            let empty_icon : String = {
                "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAAklEQVR4AewaftIAAAAPSURBVGMYBaNgFIwCKAAABBAAAY7F3VUAAAAASUVORK5CYII="
            } .to_string();
            icons_mgr.icons_idx_map .write().unwrap() .insert (empty_icon.clone(), 0);
            icons_mgr.icons_cache .write().unwrap() .push (empty_icon);
            icons_mgr
        } ) .clone()
    }

    fn make_hwnd_path_pair (wde:&WinDatEntry) -> Option<HwndExePathPair> {
        wde.icon_override_loc.clone() .or_else ( || {
            wde.exe_path_name.as_ref() .map (|p| p.full_path.clone())
        } ) .map (|p| HwndExePathPair { hwnd:wde.hwnd, path:p } )
    }

    pub fn get_cached_icon_idx (&self, wde:&WinDatEntry) -> Option<usize> {
        // note that to allow marking stale, we always return from hwnd map, even for those where the cache idx is from exe-icon cache
        Self::make_hwnd_path_pair (wde) .and_then ( |hpp| {
            self.icons_hpp_map.read().unwrap() .get(&hpp) .map (|icm| icm.cache_idx)
                .filter (|idx| *idx != 0) .or_else (|| None)
        } )
    }

    pub fn remove_cached_icon_mapping (&self, wde:&WinDatEntry) {
        Self::make_hwnd_path_pair (wde) .into_iter() .for_each ( |hpp| {
            self.icons_hpp_map .write().unwrap() .remove(&hpp);
            self.queried_hpp_cache .write().unwrap() .remove(&hpp.hwnd);
        } );
    }


    pub fn mark_cached_icon_mapping_stale (&self, wde:&WinDatEntry) {
        Self::make_hwnd_path_pair (wde) .map ( |hpp| {
            self.icons_hpp_map.write().unwrap() .get_mut(&hpp) .map (|icm| { icm.is_stale = true });
        } );
    }
    pub fn mark_all_cached_icon_mappings_stale (&self) {
        self.icons_hpp_map.write().unwrap() .iter_mut() .for_each (|(_,icm)| { icm.is_stale = true })
    }

    pub fn queue_icon_refresh (&self, wde:&WinDatEntry) {
        self.mark_cached_icon_mapping_stale(wde);
        self.process_found_hwnd_exe_path(wde);
    }

    pub fn clear_dead_hwnd (&self, wde:&WinDatEntry) {
        self.remove_cached_icon_mapping(wde)
    }

    fn queue_hwnd_icon_query (&self, hpp:&HwndExePathPair) {
        //println!("hwnd icon query: {:?} : {:?}", &hpp.hwnd, &hpp.path.clone().split(r"\").last());
        let icmgr = self.clone();
        let hppc = hpp.clone();
        spawn ( move || unsafe {
            let ico_str = icon_extractor::extract_hwnd_icon (HWND(hppc.hwnd));
            ico_str .map (|s| icmgr.icon_string_callback (&hppc, s, true));
        } );
        // todo : prob should find a way of imposing timeout w/o spawning hanging threads willy nilly (or at least reap these threads)?
        let icmgr = self.clone();
        let hppc = hpp.clone();
        spawn ( move || {
            sleep (Duration::from_millis(100));
            icmgr .unheard_hwnd_icon_callback_fallback_check(&hppc);
        } );
    }

    fn queue_exe_icon_query (&self, hpp:&HwndExePathPair) {
        let icmgr = self.clone();
        let hppc = hpp.clone();
        spawn ( move || { unsafe {
            let delay = rand::thread_rng().gen_range(10..80);
            sleep (Duration::from_millis(delay));   // randomized so multiple fallbacks on same exe dont race together
            let was_past_queried = icmgr.queried_exe_cache.read().unwrap().contains(&hppc.path);
            if !was_past_queried {
                icmgr.queried_exe_cache .write().unwrap() .insert (hppc.path.clone());
                let ico_str = icon_extractor::extract_exe_path_icon (hppc.path.clone());
                ico_str .map (|s| icmgr.icon_string_callback (&hppc, s, false));
            } else {
                // so we prob came here after failing hwnd query, but the exe was already queried, and hopefully has a cached icon ..
                // .. so lets get the icm for the hwnd updated w that (.. thats where the flag to mark stale live)
                let cache_idx_opt = icmgr.icons_exe_map .read().unwrap() .get(&hppc.path) .copied();
                if let Some(cache_idx) = cache_idx_opt {
                    icmgr.handle_icon_cache_mapping_update (cache_idx, false, false, &hppc);
                } else { println!("WARNING: exe icon lookup was queried but nothing in cache!")
                    // means we tried the exe query and failed .. nothing to do as thats prob not gonna change
                }
        } } } );
    }

    fn icon_string_callback (&self, hpp:&HwndExePathPair, icon_str:String, is_from_hwnd:bool) {
        if !icon_str.is_empty() {
            let ss = SwitcheState::instance();
            // we'll acquire write locks on cache and idx so there's no race w multiple cbs hitting here
            let mut icm = self.icons_cache.write().unwrap();
            let mut iidm = self.icons_idx_map.write().unwrap();
            let prior_cache_idx = iidm.get(&icon_str).copied();
            let cache_idx = prior_cache_idx .unwrap_or_else ( || {
                let new_cache_idx = icm.len();
                iidm.insert(icon_str.clone(), new_cache_idx);
                icm.push(icon_str.clone());
                new_cache_idx
            } );
            // lets release the guards before we call anywhere outside to emit updates
            mem::drop(icm); mem::drop(iidm);
            if prior_cache_idx.is_none() {
                ss .emit_icon_entry ( &IconEntry { ico_id: cache_idx, ico_str: icon_str } );
            }
            //println!("ico-cb (exe?:{:?}) (idx:{:?}) : {:?}", !is_from_hwnd, cache_idx, hpp.path.clone().split(r"\").last());
            // now we can send it for hwnd/path icon-idx mappings
            self.handle_icon_cache_mapping_update (cache_idx, is_from_hwnd, prior_cache_idx.is_none(), hpp);
        } else {
            println! ("WARNING: got empty icon-string callback for hwnd: {:?} {:?}", hpp.hwnd, &hpp.path.clone().split(r"\").last());
            if is_from_hwnd { self.queue_exe_icon_query (hpp) }
        }
    }
    fn handle_icon_cache_mapping_update (&self, cache_idx:usize, is_from_hwnd:bool, is_new_icon:bool, hpp:&HwndExePathPair) {
        let ss = SwitcheState::instance();
        // we'll update mappings, and use the return val to see if the mapping is new or changed
        let is_mapping_update = if is_from_hwnd {
            let prior = self.icons_hpp_map.write().unwrap() .insert (hpp.clone(), IconCacheMapping { cache_idx, is_stale:false});
            Some(cache_idx) != prior.map (|icm| icm.cache_idx)
        } else {
            let exe_prior  = self.icons_exe_map .write().unwrap() .insert (hpp.path.clone(), cache_idx);
            let hwnd_prior = self.icons_hpp_map .write().unwrap() .insert (hpp.clone(), IconCacheMapping { cache_idx, is_stale:false});
            Some(cache_idx) != exe_prior || Some(cache_idx) != hwnd_prior.map (|icm| icm.cache_idx)
        };
        if is_new_icon || is_mapping_update {
            // send out updates for the specific wde, then queue up to send a renderlist
            ss.hwnd_map .read().unwrap() .get(&hpp.hwnd) .map (|wde| ss.emit_win_dat_entry (wde));
            ss.emit_render_lists_queued(false);
        }
    }
    /* debug printout helper fns:
    pub fn stamp(&self) -> u128 { SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis() }
    fn hpp_split (&self, hpp:&HwndExePathPair) -> String { hpp.path.clone().split(r"\").last().unwrap_or("").into() }
    fn format_icp (&self, hpp:&HwndExePathPair) -> String { if let Some(icp) = self.icons_hwnd_map.read().unwrap().get(hpp) {
        let exe = hpp.path.clone().split(r"\").last().unwrap_or("").to_string();
        format! ("hwnd:{:?}, exe:{:?}, iid:{:?}, stale:{:?}", hpp.hwnd, exe, icp.cache_idx, icp.is_stale)
    } else {"hpp-icp-- None".into()} }
    // */

    fn unheard_hwnd_icon_callback_fallback_check (&self, hpp:&HwndExePathPair) { //println!("unheard-check-- {:?}", self.hpp_split(hpp));
        if self.check_icon_needs_query(hpp) {
            println! ("INFO: Did not hear callback for {:?} {:?} within check period, falling back to exe icon query", hpp.hwnd, &hpp.path.clone().split(r"\").last());
            self.queue_exe_icon_query(hpp)
        }
    }
    fn check_icon_needs_query (&self, hpp:&HwndExePathPair) -> bool {
        self.icons_hpp_map .read().unwrap() .get(hpp) .filter(|icm| icm.cache_idx != 0 && !icm.is_stale) .is_none()
    }

    pub fn emit_all_icon_entries (&self) {
        let ss = SwitcheState::instance();
        self.icons_cache .read().unwrap() .iter() .enumerate() .for_each (|(iid, ico)| {
            ss .emit_icon_entry ( &IconEntry { ico_id: iid, ico_str: ico.clone() } );
        } );
        // this should only happen on reload etc, fine time to dump out icons and mappings for examination too
        self.icons_hpp_map .read().unwrap() .iter().for_each(|(hpp,icm)| println!("{:?}, {:?}", icm, hpp));
        self.icons_exe_map  .read().unwrap() .iter().for_each(|(exe,idx)| println!("{:?}, {:?}", idx, exe));
    }


    pub fn process_found_hwnd_exe_path (&self, wde:&WinDatEntry) {
        if wde.should_exclude == Some(true) { return }
        Self::make_hwnd_path_pair (wde) .map ( |hpp| {
            let was_past_queried = self.queried_hpp_cache.read().unwrap().get(&wde.hwnd).filter(|h| *h == &hpp).is_some();
            let do_refresh = Some(true) != self.icons_hpp_map.read().unwrap().get(&hpp).map(|icm| !icm.is_stale && icm.cache_idx > 0);
            if !was_past_queried || do_refresh {
                self.queried_hpp_cache .write().unwrap() .insert(wde.hwnd, hpp.clone());
                if wde.icon_override_loc.is_none() {
                    self.queue_hwnd_icon_query (&hpp);
                } else {
                    self.queue_exe_icon_query (&hpp)
                }
        } } );
    }


}






pub struct IconPathOverridesManager ;

impl IconPathOverridesManager {

    pub fn instance() -> IconPathOverridesManager {
        // todo update this later
        IconPathOverridesManager
    }

    pub fn get_icon_override_path (_wde:&WinDatEntry) -> Option<String> {
        // todo .. or remove if we dont need this anymore
        None
    }

}






pub mod icon_extractor {

    use std::io::{Cursor};
    use std::mem;

    use image::{ImageOutputFormat, RgbaImage};
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    //use base64::write::{EncoderStringWriter, EncoderWriter};

    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        DestroyIcon, GCL_HICON, GCL_HICONSM, GetClassLongPtrA, GetIconInfo, HICON,
        ICON_BIG, ICON_SMALL, ICON_SMALL2, ICONINFO, SendMessageA, WM_GETICON
    };
    use windows::Win32::Graphics::Gdi::{BITMAP, DeleteObject, GetBitmapBits, GetObjectW, HGDIOBJ};
    use windows::Win32::UI::Shell::{ExtractAssociatedIconA};


    pub unsafe fn extract_exe_path_icon (exe_path:String) -> Option<String> {
        //println!("exe-ico-ext-- {:?}", exe_path.split(r"\").last());
        let mut path_arr = [0u8; 128];
        path_arr[.. exe_path.len()] .copy_from_slice (exe_path.as_bytes());
        let mut icon_idx = 0u16;
        let hicon = ExtractAssociatedIconA (HINSTANCE(0), &mut path_arr, &mut icon_idx);
        hicon_to_base64_str(hicon)
    }

    pub unsafe fn extract_hwnd_icon (hwnd:HWND) -> Option<String> {
        let mut hicon = HICON::default();
        if hicon.is_invalid() { hicon = HICON ( SendMessageA ( hwnd, WM_GETICON, WPARAM (ICON_SMALL2 as usize), LPARAM(0) ) .0 ) }
        if hicon.is_invalid() { hicon = HICON ( SendMessageA ( hwnd, WM_GETICON, WPARAM (ICON_SMALL  as usize), LPARAM(0) ) .0 ) }
        if hicon.is_invalid() { hicon = HICON ( SendMessageA ( hwnd, WM_GETICON, WPARAM (ICON_BIG    as usize), LPARAM(0) ) .0 ) }
        if hicon.is_invalid() { hicon = HICON ( GetClassLongPtrA ( hwnd, GCL_HICONSM ) as isize ) }
        if hicon.is_invalid() { hicon = HICON ( GetClassLongPtrA ( hwnd, GCL_HICON   ) as isize ) }
        hicon_to_base64_str(hicon)
    }



    unsafe fn hicon_to_base64_str (hicon: HICON) -> Option<String> {

        if hicon.is_invalid() { return None }

        // first gotta get the details on the icon
        let mut info = ICONINFO::default();
        let res = GetIconInfo (hicon, &mut info as *mut _);
        if res.ok().is_err() { return None }

        // then we'll get the actual bitmap (ignoring separate mask field as thats typically in rgba already)
        let mut bmp = BITMAP::default();
        let _ = GetObjectW (
            HGDIOBJ (info.hbmColor.0),
            mem::size_of::<BITMAP>() as i32,
            Some ( &mut bmp as *mut BITMAP as _),
        );

        // then convert the bitmap structure into a regular vec/arr
        let buf_size = bmp.bmWidth * bmp.bmHeight * 4;
        let mut buf = vec![0u8; buf_size as usize];
        let _ = GetBitmapBits (info.hbmColor, buf_size, buf.as_mut_ptr() as _);

        // requesting info has the system allocate the bitmap and mask, should release that memory
        let _ = DeleteObject (info.hbmColor);
        let _ = DeleteObject (info.hbmMask);
        let _ = DestroyIcon  (hicon);

        // then rearrange the bitmap into rgba channels for the image representation
        for chunk in buf.chunks_exact_mut(4) {
            let [b, _, r, _] = chunk else { unreachable!() };
            mem::swap(b, r);
        }
        let img = RgbaImage::from_vec (bmp.bmWidth as u32, bmp.bmHeight as u32, buf).unwrap();

        // and finally, we can convert that to base64 string and return that
        //
        //let mut enc = EncoderStringWriter::new (&STANDARD);  // base64 encoder to directly write png encode img data into
        ////enc.write_all(&*buf).unwrap();
        //enc.into_inner()
        //
        //let mut enc = EncoderWriter::new (&mut buf, &STANDARD);
        //img .write_to (&mut enc, ImageOutputFormat::Png) .expect("hicon to png error");
        //String::from_utf8_lossy(enc.into_inner()).into()
        //
        // ^^ base64 encoders no longer seem to be writable directly from image (since image now requires a seekable Cursor)
        // ..  so we'll have to allocate and write the final output into yet another buffer .. oh well
        let mut buf = Cursor::new(vec![0u8]);
        img .write_to (&mut buf, ImageOutputFormat::Png) .expect("hicon to png error");
        Some ( format! ("data:image/png;base64,{}",  STANDARD.encode(&buf.into_inner()) ) )
    }


}


