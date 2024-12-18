

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::ops::Deref;
use std::string::ToString;
use std::sync::{Arc};
use std::sync::{RwLock};
//use no_deadlocks::RwLock;
use std::thread::{sleep, spawn};
use std::time::{Duration};

use once_cell::sync::OnceCell;
use rand::Rng;
use tracing::warn;


use crate::switche::{Hwnd, IconEntry, SwitcheState, WinDatEntry};



#[derive (Debug, Default, Eq, PartialEq, Hash, Clone)]
struct HwndExePathPair { hwnd:Hwnd, path:String, is_uwp:bool }

#[derive (Debug, Default, Eq, PartialEq, Hash, Copy, Clone)]
struct IconCacheMapping { cache_idx:usize, is_stale:bool }

# [ derive ( ) ]
pub struct _IconsManager {

    // we'll store icons in a vec and just add remove mappings to its indices for associated hwnds
    icons_cache        : RwLock <Vec <String>>,
    // we'll use hashes of icons as map-key for reverse lookup
    icons_idx_map      : RwLock <HashMap <String, usize>>,

    // for hwnds, key needs to incl the exe-path as a hpp pair
    icons_hpp_map      : RwLock <HashMap <HwndExePathPair, IconCacheMapping>>,
    // for exes, icons dont go stale, so directly mapping to icons idx
    icons_exe_map      : RwLock <HashMap <String, usize>>,

    // we'll need a reverse lookup table from exe paths to check if they were queried
    queried_exe_cache  : RwLock <HashSet <String>>,
    // but for hpps, since cache map key is hwnd+path, we'll store that as a map and compare against it (alt coudld use an hpp set)
    // (that will also ensure only one hpp for a hwnd is in the map as it should be)
    queried_hpp_cache  : RwLock <HashMap <Hwnd, HwndExePathPair>>,

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
                icons_cache        : RwLock::new(Default::default()),
                icons_idx_map      : RwLock::new(Default::default()),
                icons_hpp_map      : RwLock::new(Default::default()),
                icons_exe_map      : RwLock::new(Default::default()),
                queried_exe_cache  : RwLock::new(Default::default()),
                queried_hpp_cache  : RwLock::new(Default::default()),
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
        let ico_path = {
            if wde.is_uwp_app != Some(true) { wde.exe_path_name.as_ref() .map (|p| &p.full_path) }
            else { wde.uwp_icon_path.as_ref() }
        };
        ico_path .map (|p| HwndExePathPair { hwnd:wde.hwnd, path:p.to_string(), is_uwp: wde.is_uwp_app == Some(true) } )
    }

    pub fn get_cached_icon_idx (&self, wde:&WinDatEntry) -> Option<usize> {
        // note that to allow marking stale, we always return from hwnd map, even for those where the cache idx is from exe-icon cache
        Self::make_hwnd_path_pair (wde) .and_then ( |hpp| {
            self.icons_hpp_map.read().unwrap() .get(&hpp) .map (|icm| icm.cache_idx) .filter (|idx| *idx != 0)
        } )
    }

    pub fn remove_cached_icon_mapping (&self, wde:&WinDatEntry) {
        Self::make_hwnd_path_pair (wde) .into_iter() .for_each ( |hpp| {
            self.icons_hpp_map .write().unwrap() .remove(&hpp);
            self.queried_hpp_cache .write().unwrap() .remove(&hpp.hwnd);
        } );
    }


    pub fn mark_cached_icon_mapping_stale (&self, wde:&WinDatEntry) {
        if let Some(hpp) = Self::make_hwnd_path_pair(wde) {
            self.icons_hpp_map.write().unwrap() .get_mut(&hpp) .iter_mut().for_each (|icm| { icm.is_stale = true });
        }
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
        //debug!("hwnd icon query: {:?} : {:?}", &hpp.hwnd, &hpp.path.clone().split(r"\").last());
        if hpp.is_uwp { return }
        // ^^ UWP apps sometimes seem to hang at hwnd icon query winapi calls, but we shouldnt get here anyway as they have separate handling now
        let (icmgr, hppc) = (self.clone(), hpp.clone());
        spawn ( move || unsafe {
            if let Some(ico_str) = icon_extraction::extract_hwnd_icon (hppc.hwnd.HWND()) {
                icmgr.icon_string_callback (&hppc, ico_str, true);
            } else {
                icmgr.queue_exe_icon_query (&hppc);
            }
        } );
    }

    fn queue_exe_icon_query (&self, hpp:&HwndExePathPair) {
        let icmgr = self.clone();
        let hppc = hpp.clone();
        spawn ( move || unsafe {
            let delay = rand::thread_rng().gen_range(10..80);
            sleep (Duration::from_millis(delay));   // randomized so multiple fallbacks on same exe dont race together
            let was_past_queried = icmgr.queried_exe_cache.read().unwrap().contains(&hppc.path);
            if !was_past_queried {
                icmgr.queried_exe_cache .write().unwrap() .insert (hppc.path.clone());
                let ico_str = if hppc.is_uwp {
                    // note that for uwm, the hpp has already been populated with icon path (instead of exe path)
                    icon_extraction::extract_png_icon (hppc.path.as_str())
                } else {
                    icon_extraction::extract_exe_path_icon (&hppc.path)
                };
                let ico_str = ico_str .or_else (|| icon_extraction::get_default_icon());
                // note ^^ this is the win default 'exe' icon, not our empty icon
                ico_str .into_iter() .for_each (|s| icmgr.icon_string_callback (&hppc, s, false));
            } else {
                // if multiple hwnds for same app get queued, because of random delay and cache checking the second one might end up here
                // so hopefully we have a cached icon and we can populate its icm with it
                let cache_idx_opt = icmgr.icons_exe_map .read().unwrap() .get(&hppc.path) .copied();
                if let Some(cache_idx) = cache_idx_opt {
                    icmgr.handle_icon_cache_mapping_update (cache_idx, false, false, &hppc);
                } else { warn!("WARNING: exe icon lookup says prior-queried but nothing in cache!")
                    // means we tried the exe query and failed .. nothing to do as thats prob not gonna change
                }
        } } );
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
            drop(icm); drop(iidm);
            if prior_cache_idx.is_none() {
                ss .emit_icon_entry ( &IconEntry { ico_id: cache_idx, ico_str: icon_str } );
            }
            //debug!("ico-cb (exe?:{:?}) (idx:{:?}) : {:?}", !is_from_hwnd, cache_idx, hpp.path.clone().split(r"\").last());
            // now we can send it for hwnd/path icon-idx mappings
            self.handle_icon_cache_mapping_update (cache_idx, is_from_hwnd, prior_cache_idx.is_none(), hpp);
        } else {
            warn! ("WARNING: got empty icon-string callback for hwnd: {:?} {:?}", hpp.hwnd, &hpp.path.clone().split('\\').last());
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
            ss.hwnd_map .read().unwrap() .get(&hpp.hwnd) .iter().for_each (|wde| ss.emit_win_dat_entry (wde));
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


    pub fn emit_all_icon_entries (&self) {
        let ss = SwitcheState::instance();
        self.icons_cache .read().unwrap() .iter() .enumerate() .for_each (|(iid, ico)| {
            ss .emit_icon_entry ( &IconEntry { ico_id: iid, ico_str: ico.clone() } );
        } );
        // this should only happen on reload etc, fine time to dump out icons and mappings for examination too
        //self.icons_hpp_map .read().unwrap() .iter().for_each(|(hpp,icm)| debug!("{:?}, {:?}", icm, hpp));
        //self.icons_exe_map  .read().unwrap() .iter().for_each(|(exe,idx)| debug!("{:?}, {:?}", idx, exe));
    }


    pub fn process_found_hwnd_exe_path (&self, wde:&WinDatEntry) {
        if wde.should_exclude == Some(true) { return }
        if let Some(hpp) = Self::make_hwnd_path_pair(wde) {
            let was_past_queried = self.queried_hpp_cache.read().unwrap().get(&wde.hwnd).filter(|h| *h == &hpp).is_some();
            let do_refresh = Some(true) != self.icons_hpp_map.read().unwrap().get(&hpp).map(|icm| !icm.is_stale && icm.cache_idx > 0);
            if !was_past_queried || do_refresh {
                self.queried_hpp_cache .write().unwrap() .insert(wde.hwnd, hpp.clone());
                if wde.is_uwp_app != Some(true) {
                    self.queue_hwnd_icon_query (&hpp);
                } else {
                    self.queue_exe_icon_query (&hpp)
                }
        } };
    }


}





pub(crate) mod uwp_processing {
    use std::path::{Path, PathBuf};
    use tracing::error;

    pub struct ManifestParse { pub(crate) ico:PathBuf, pub(crate) exe:PathBuf }

    pub fn get_uwp_manifest_parse (exe_path:&String) -> Option<ManifestParse> {
        fn append_manifest_path (p:&Path) -> PathBuf { p.join("AppxManifest.xml") }
        let exe_path = Path::new (exe_path);
        let mut manifest = exe_path.parent().map(append_manifest_path);
        if manifest.as_ref().filter(|mf| mf.exists()).is_none() {
            manifest = Some (append_manifest_path(exe_path));
        }
        let mut buf = Vec::new();
        let mut ico_path : Option<String> = None;
        let mut app_path : Option<String> = None;
        use quick_xml::events::Event::*;
        manifest.as_ref() .filter (|mf| mf.exists()) .iter() .for_each (|mf| {
            quick_xml::reader::Reader::from_file(mf) .ok() .iter_mut() .for_each ( |reader| {
                loop {
                    match reader.read_event_into(&mut buf) {
                        Err (e) => { error!("UWP Manifest parse error at position {}: {:?}", reader.buffer_position(), e); break; },
                        Ok (Eof) => break,
                        Ok (Start (ref e)) => {
                            reader.decoder() .decode (e.name().as_ref()) .ok() .iter() .for_each (|tag| {
                            if tag == "Application"  {
                                app_path = e.attributes() .filter (|a| a.as_ref().ok() .filter (|a| a.key.as_ref() == "Executable".as_bytes()) .is_some())
                                    .flatten() .next() .map (|a| String::from_utf8_lossy(&a.value).into_owned());
                            } else if tag == "uap:VisualElements" || tag == "VisualElements" {
                                ico_path = e.attributes() .filter (|a| a.as_ref().ok() .filter (|a| a.key.as_ref() == "Square44x44Logo".as_bytes()) .is_some())
                                    .flatten() .next() .map (|a| String::from_utf8_lossy(&a.value).into_owned());
                            }
                        } ) },
                        _ => (),
                    }
                    if ico_path.is_some() && app_path.is_some() { break }
            } } )
        } );
        fn get_checked_file_path (file:&Path) -> Option<PathBuf> {
            let stem = file.file_stem()? .to_str()?;
            file.parent()? .read_dir().ok()? .flat_map (|e| Some(e.ok()?.path())) .find (|p|
                p.file_name() .filter (|f| f.to_string_lossy().contains(stem)) .is_some()
            )
        }
        fn get_full_path (rel_path:&String, manifest:&Path) -> Option<PathBuf> {
            manifest.parent() .map (|mpp| mpp.join(rel_path))
        }
        if let Some(mf) = manifest.as_ref() {
        if let (Some(ico), Some(exe)) = (ico_path, app_path) { //dbg!((&ico, &exe));
        if let (Some(ico), Some(exe)) = ( get_full_path(&ico,mf), get_full_path(&exe,mf) ) {
        if let (Some(ico), Some(exe)) = ( get_checked_file_path(&ico), Some(exe) ) { //dbg! ((&ico,&exe));
            return Some ( ManifestParse {ico, exe} );
        } } } }
        None
    }


}




pub(crate) mod icon_extraction {
    use std::io::{Cursor};
    use std::mem;
    use std::ops::Not;
    use std::path::Path;

    use image::{ImageOutputFormat, RgbaImage};
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;

    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
    use windows::Win32::Graphics::Gdi::{BITMAP, DeleteObject, GetBitmapBits, GetObjectW, HGDIOBJ};
    use windows::Win32::UI::Shell::{ExtractAssociatedIconA};
    use windows::Win32::UI::WindowsAndMessaging::{
        DestroyIcon, GCL_HICON, GCL_HICONSM, GetClassLongPtrW, GetIconInfo, HICON, ICON_BIG, ICON_SMALL, ICON_SMALL2,
        ICONINFO, IDI_APPLICATION, LoadIconW, SendMessageA, WM_GETICON
    };

    pub unsafe fn extract_exe_path_icon (exe_path:&String) -> Option<String> {
        //debug!("exe-ico-ext-- {:?}", exe_path.split(r"\").last());
        let mut path_arr = [0u8; 128];
        path_arr[.. exe_path.len()] .copy_from_slice (exe_path.as_bytes());
        let mut icon_idx = 0u16;
        let hicon = ExtractAssociatedIconA (HINSTANCE::default(), &mut path_arr, &mut icon_idx);
        hicon_to_base64_str(hicon)
    }

    pub unsafe fn extract_hwnd_icon (hwnd:HWND) -> Option<String> {
        let mut hicon = HICON::default();
        // first we'll ask the window directly for the latest icon it has
        if hicon.is_invalid() { hicon = HICON ( SendMessageA ( hwnd, WM_GETICON, WPARAM (ICON_BIG    as usize), LPARAM(0) ) .0 as *mut _ ) }
        if hicon.is_invalid() { hicon = HICON ( SendMessageA ( hwnd, WM_GETICON, WPARAM (ICON_SMALL  as usize), LPARAM(0) ) .0 as *mut _ ) }
        if hicon.is_invalid() { hicon = HICON ( SendMessageA ( hwnd, WM_GETICON, WPARAM (ICON_SMALL2 as usize), LPARAM(0) ) .0 as *mut _ ) }
        // else we'll try and get the icon for its registered window class
        if hicon.is_invalid() { hicon = HICON ( GetClassLongPtrW ( hwnd, GCL_HICON ) as *mut _   ) }
        if hicon.is_invalid() { hicon = HICON ( GetClassLongPtrW ( hwnd, GCL_HICONSM ) as *mut _  ) }

        hicon_to_base64_str(hicon)
    }
    pub unsafe fn get_default_icon () -> Option<String> {
        // IDI_APPLICATION pulls the default system 'application' icon as fallback
        let hicon = LoadIconW (HINSTANCE::default(), IDI_APPLICATION) .unwrap_or_default();
        hicon_to_base64_str(hicon)
    }
    pub unsafe fn extract_png_icon (png_path:&str) -> Option<String> {
        png_file_to_base64_str (Path::new(png_path))
    }
    unsafe fn png_file_to_base64_str (png_path:&Path) -> Option<String> {
        image::open (png_path) .map (|img| {
            let mut buf = Cursor::new(vec![0u8]);
            img .write_to (&mut buf, ImageOutputFormat::Png) .expect("png write error");
            Some ( format! ("data:image/png;base64,{}",  STANDARD.encode(buf.into_inner()) ) )
        } ) .ok() .flatten()
    }


    unsafe fn hicon_to_base64_str (hicon: HICON) -> Option<String> {

        if hicon.is_invalid() { return None }

        // first gotta get the details on the icon
        let mut info = ICONINFO::default();
        let res = GetIconInfo (hicon, &mut info as *mut _);
        if res.is_err() { return None }

        // then we'll get the actual bitmap (and later its mask if necessary)
        let mut bmp = BITMAP::default();
        let _ = GetObjectW ( HGDIOBJ (info.hbmColor.0), mem::size_of::<BITMAP>() as i32, Some ( &mut bmp as *mut BITMAP as _) );
        let buf_size = bmp.bmWidth * bmp.bmHeight * 4;
        let mut buf = vec![0u8; buf_size as usize];
        let _ = GetBitmapBits (info.hbmColor, buf_size, buf.as_mut_ptr() as _);

        //let title =  crate::switche::SwitcheState::instance().hwnd_map.read().unwrap().get(&_hwnd.0).and_then(|wde| wde.win_text.to_owned()).unwrap_or_default();
        //debug! ("hwnd:{} \t: w:{},h:{}, {}bpp .. {}", _hwnd.0, bmp.bmWidth, bmp.bmHeight, bmp.bmBitsPixel, title);

        // we'll check to see if all alpha values are 0, in which case we'll fallback to looking at the mask
        let mut alpha_invalid = true;
        for n in 0 .. buf.len()/4 {
            if buf [n*4 + 3] != 0 { alpha_invalid = false; break }
        }

        // lets do the same for the mask (if the alpha channel was invalid)
        let use_mask = alpha_invalid && info.hbmMask.is_invalid().not();
        let buf_size_mask = bmp.bmWidth * bmp.bmHeight / 8;
        let mut mask = BITMAP::default();
        let mut buf_mask = vec![0u8; buf_size_mask as usize];
        if use_mask {
            let _ = GetObjectW ( HGDIOBJ (info.hbmMask.0), mem::size_of::<BITMAP>() as i32, Some ( &mut mask as *mut BITMAP as _) );
            let _ = GetBitmapBits (info.hbmMask, buf_size_mask, buf_mask.as_mut_ptr() as _);
        }

        // requesting info has the system allocate the bitmap and mask, should release that memory
        let _ = DeleteObject (info.hbmColor);
        let _ = DeleteObject (info.hbmMask);
        let _ = DestroyIcon  (hicon);

        // then rearrange the bitmap into rgba channels for the image representation
        for (i, chunk) in buf.chunks_exact_mut(4).enumerate() {
            let [b, _g, r, a] = chunk else { unreachable!() };
            mem::swap(b, r);
            //if alpha_invalid { *a = 0xFF }
            // ^^ this alone mostly works w/o loading mask, but ofc the transparency would be lost
            if use_mask {
                let mask_byte = buf_mask.get(i/8).unwrap_or(&0);
                let mask_bit = 1 << (i % 8);
                *a = if mask_byte & mask_bit != 0 { *a } else { 0xFF };
            }
        }
        let img = RgbaImage::from_vec (bmp.bmWidth as u32, bmp.bmHeight as u32, buf).unwrap();

        // and finally, we can convert that to base64 string and return that
        let mut buf = Cursor::new(vec![0u8]);
        img .write_to (&mut buf, ImageOutputFormat::Png) .expect("hicon to png error");
        Some ( format! ("data:image/png;base64,{}",  STANDARD.encode(buf.into_inner()) ) )
    }


}

#[cfg(test)]
mod test {
    #[test]
    fn calculator_manifest_test () { unsafe {
        let calc_loc = r"C:\Program Files\WindowsApps\Microsoft.WindowsCalculator_11.2210.0.0_x64__8wekyb3d8bbwe\CalculatorApp.exe".to_string();
        let mfp = crate::icons::uwp_processing::get_uwp_manifest_parse (&calc_loc);
        assert! (mfp.is_some());
        if let Some(ico_path) = mfp.as_ref() .map(|mfp| mfp.ico.to_string_lossy()) {
            assert! (crate::icons::icon_extraction::extract_png_icon (ico_path.as_ref()).is_some());
        }
    } }
    #[test]
    fn dbgview_ico_test () { unsafe {
        let dbgview_loc = r"C:\yakdat\downloads\ins-bin\SysinternalsSuite\dbgview64.exe".to_string();
        assert! (crate::icons::icon_extraction::extract_exe_path_icon (&dbgview_loc).is_some());
    } }
}
