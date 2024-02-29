#![ allow (non_upper_case_globals, non_snake_case) ]

use std::ffi::{c_void};
use std::mem::size_of;
use std::ptr;
use std::sync::{Arc, Mutex, RwLock};
use once_cell::sync::Lazy;

use windows::core::{GUID, Interface, PCWSTR, PSTR, PWSTR};
use windows::Win32::Foundation::{BOOL, CloseHandle, HANDLE, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Security::TOKEN_READ;
use windows::Win32::Storage::Packaging::Appx::{GetApplicationUserModelId, GetPackagePathByFullName, GetPackagesByPackageFamily, ParseApplicationUserModelId};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameA};
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, PROPERTYKEY, SHGetPropertyStoreForWindow};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowPlacement, ShowWindow, GetWindowTextW, IsWindowVisible, GetAncestor, GetWindowThreadProcessId, PostMessageA, SetForegroundWindow, ShowWindowAsync, GetWindowLongW, WINDOWPLACEMENT, WM_CLOSE, SW_HIDE, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SW_SHOW, SW_SHOWMINIMIZED, WS_CHILD, GWL_STYLE, GA_ROOTOWNER, WS_EX_TOOLWINDOW, GWL_EXSTYLE, EnumChildWindows};


use crate::*;



pub fn check_window_visible (hwnd:Hwnd) -> bool { unsafe {
    IsWindowVisible (HWND(hwnd)) .as_bool()
} }

pub fn check_window_cloaked (hwnd:Hwnd) -> bool { unsafe {
    let mut cloaked_state: isize = 0;
    let out_ptr = &mut cloaked_state as *mut isize as *mut c_void;
    let _ = DwmGetWindowAttribute (HWND(hwnd), DWMWA_CLOAKED, out_ptr, size_of::<isize>() as u32);
    cloaked_state != 0
} }

pub fn check_if_tool_window (hwnd:Hwnd) -> bool { unsafe {
    GetWindowLongW (HWND(hwnd), GWL_EXSTYLE) & WS_EX_TOOLWINDOW.0 as i32 != 0
} }

pub fn check_window_has_owner (hwnd:Hwnd) -> bool { unsafe {
    GetAncestor (HWND(hwnd), GA_ROOTOWNER) != HWND(hwnd)
} }



pub fn check_window_is_child (hwnd:Hwnd) -> bool { unsafe {
    (GetWindowLongW (HWND(hwnd), GWL_STYLE) & WS_CHILD.0 as i32) != 0
} }

pub fn get_fgnd_window () -> Hwnd { unsafe {
    GetForegroundWindow().0 as Hwnd
} }



pub fn window_activate (hwnd:Hwnd) { unsafe { println!("winapi activate {:?}",hwnd);
    let mut win_state =  WINDOWPLACEMENT::default();
    //ShowWindowAsync (HWND(hwnd), SW_NORMAL);
    // ^^ this will cause minimized/maximized windows to be restored
    GetWindowPlacement (HWND(hwnd), &mut win_state);
    if win_state.showCmd == SW_SHOWMINIMIZED {
        ShowWindowAsync (HWND(hwnd), SW_RESTORE);
    } else {
        ShowWindowAsync (HWND(hwnd), SW_SHOW);
    }
    //keybd_event (0, 0, KEYBD_EVENT_FLAGS::default(), 0);
    SetForegroundWindow (HWND(hwnd));
} }



pub fn window_hide (hwnd:Hwnd) { unsafe { println!("winapi hide {:?}",hwnd);
    ShowWindow (HWND(hwnd), SW_HIDE);
} }
pub fn window_minimize (hwnd:Hwnd) { unsafe {
    ShowWindowAsync (HWND(hwnd), SW_MINIMIZE);
} }
pub fn window_maximize (hwnd:Hwnd) { unsafe {
    ShowWindowAsync (HWND(hwnd), SW_MAXIMIZE);
} }

pub fn window_close (hwnd:Hwnd) { unsafe { println!("winapi close {:?}",hwnd);
    //CloseWindow(HWND(hwnd));
    // note ^^ that the u32 'CloseWindow' cmd actually minimizes it, to close, send it a WM_CLOSE msg
    PostMessageA (HWND(hwnd), WM_CLOSE, WPARAM::default(), LPARAM::default());
} }



pub fn get_window_text (hwnd:Hwnd) -> String { unsafe {
    const MAX_LEN : usize = 512;
    let mut lpstr = [0u16; MAX_LEN];
    let copied_len = GetWindowTextW (HWND(hwnd), &mut lpstr);
    String::from_utf16_lossy (&lpstr[..(copied_len as _)])
    // ^^ todo: see if makes sense to do all string work everywhere with cow instead of cloned strings
} }



pub fn get_hwnd_exe_path (hwnd:Hwnd) -> Option<String> { unsafe {
    let mut pid : u32 = 0;
    let _ = GetWindowThreadProcessId (HWND(hwnd), Some(&mut pid));
    get_pid_exe_path (pid)
} }
fn get_pid_exe_path (pid:u32) -> Option<String> { unsafe {
    const MAX_LEN : usize = 1024;
    let handle = OpenProcess (PROCESS_QUERY_LIMITED_INFORMATION, BOOL::from(false), pid);
    let mut lpstr = [0u8; MAX_LEN];
    let mut lpdwsize = MAX_LEN as u32;
    if handle.is_err() { return None }
    let _ = QueryFullProcessImageNameA ( HANDLE (handle.as_ref().unwrap().0), PROCESS_NAME_WIN32, PSTR::from_raw(lpstr.as_mut_ptr()), &mut lpdwsize );
    handle .iter() .for_each ( |h| { CloseHandle(*h); } );
    PSTR::from_raw (lpstr.as_mut_ptr()) .to_string() .ok()
} }

pub fn get_uwp_hwnd_exe_path (hwnd:Hwnd) -> Option<String> { unsafe {
    let mut frame_host_pid : u32 = 0;
    let _ = GetWindowThreadProcessId (HWND(hwnd), Some(&mut frame_host_pid));  if hwnd==657422 {println!("fh-pid--{:?}",(frame_host_pid));}
    let uwp_pid = get_child_windows (hwnd) .iter() .map (|cwh| {
        let mut pid = 0u32;
        let _ = GetWindowThreadProcessId (HWND(*cwh), Some(&mut pid));  if hwnd==657422 {println!("uwp-pid{:?}",(pid));}
        pid
    } ) .filter (|pid| *pid != frame_host_pid) .collect::<Vec<u32>>();
    uwp_pid .first() .and_then (|&pid| get_pid_exe_path(pid))
} }

pub fn get_aumid_from_hwnd (hwnd:Hwnd) -> Option<String> { unsafe {
    let mut pid : u32 = 0;
    let _ = GetWindowThreadProcessId (HWND(hwnd), Some(&mut pid));  dbg!(pid);
    let handle = OpenProcess (PROCESS_QUERY_LIMITED_INFORMATION, BOOL::from(false), pid); dbg!(&handle);
    if handle.is_err() { return None }
    const AUMID_MAX_LEN : usize = 1024;
    let mut aum_id_buf = [0u16; AUMID_MAX_LEN];
    let mut aum_id_buf_len = AUMID_MAX_LEN as u32;
    let aum_id_res = GetApplicationUserModelId ( handle.to_owned().unwrap(), &mut aum_id_buf_len, PWSTR::from_raw(aum_id_buf.as_mut_ptr())); dbg!(&aum_id_res);
    if aum_id_res.is_err() { return None }
    let _ = dbg!(PWSTR::from_raw(aum_id_buf.as_mut_ptr()).to_string());
    None
} }

pub fn get_package_path_from_hwnd (hwnd:Hwnd) -> Option<String> { unsafe {

    let mut store = ptr::null_mut();
    let result = SHGetPropertyStoreForWindow(HWND(hwnd), &IPropertyStore::IID, &mut store);
    //result.expect("SHGetPropertyStoreForWindow failed");
    if result.ok().is_none() { return None }
    if store.is_null() { return None }
    //let store : IPropertyStore = unsafe { core::mem::transmute(store) };

    // (alternatively, a bit more explicitly cautious about types:)
    let store = core::mem::transmute::<*mut c_void, IPropertyStore>(store);

    //dbg!(&store);

    let PKEY_AppUserModel_ID = PROPERTYKEY {fmtid: GUID::from("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3"), pid:5 };
    //let PKEY_AppUserModel_ID = &IPropertyStore::
    let _res = store.GetCount();
    //dbg!(res);
    let _res = store.GetValue ( &PKEY_AppUserModel_ID as *const _ );
    //dbg!(&res.is_err());
    //dbg!(&res.unwrap().Anonymous.Anonymous.Anonymous.pwszVal.to_string());
    //dbg!(&res.clone().unwrap().Anonymous.Anonymous.Anonymous.pwszVal.to_string());

    let aumid = _res.unwrap().Anonymous.Anonymous.Anonymous.pwszVal;


    let mut pkg_fam_name_len = 0u32;
    let mut pkg_rel_app_name_len = 0u32;
    let _res = ParseApplicationUserModelId (PCWSTR(aumid.as_ptr()), &mut pkg_fam_name_len, PWSTR::null(), &mut pkg_rel_app_name_len, PWSTR::null());
    //dbg! ((res, pkg_fam_name_len, pkg_rel_app_name_len));

    let mut pkg_fam_name     : Vec<u16> = Vec::with_capacity (pkg_fam_name_len as _);
    let mut pkg_rel_app_name : Vec<u16> = Vec::with_capacity (pkg_rel_app_name_len as _);
    let _res = ParseApplicationUserModelId ( PCWSTR(aumid.as_ptr()),
                                            &mut pkg_fam_name_len,     PWSTR::from_raw (pkg_fam_name.as_mut_ptr()),
                                            &mut pkg_rel_app_name_len, PWSTR::from_raw (pkg_rel_app_name.as_mut_ptr() ),
    );
    //dbg! ((&res, PWSTR::from_raw (pkg_fam_name.as_mut_ptr()).to_string(), PWSTR::from_raw (pkg_rel_app_name.as_mut_ptr()).to_string()));

    let mut num_pkg_full_names = 0u32;
    let mut buf_len = 0u32;
    let _res = GetPackagesByPackageFamily (PCWSTR(pkg_fam_name.as_ptr()), &mut num_pkg_full_names, None, &mut buf_len, PWSTR::null());
    //dbg! ((&_res, num_pkg_full_names, buf_len));

    let mut pkg_full_names : Vec<*const PWSTR> = Vec::with_capacity (num_pkg_full_names as _);
    let mut buf : Vec<u16> = Vec::with_capacity (buf_len as _);
    let _res = GetPackagesByPackageFamily ( PCWSTR(pkg_fam_name.as_ptr()),
                                           &mut num_pkg_full_names,  Some(&mut PWSTR::from_raw(&mut pkg_full_names as *mut _ as _)),
                                           &mut buf_len,  PWSTR::from_raw(buf.as_mut_ptr()) );
    //dbg!(&_res);
    //let _res = GetPackagesByPackageFamily ( PCWSTR(pkg_fam_name.as_ptr()),
    //                                       &mut num_pkg_full_names,  Some(&mut PWSTR::from_raw(&mut pkg_full_names as *mut _ as _)),
    //                                       ptr::null_mut(),  PWSTR::null() );
    //dbg!(&_res);
    //let _res = GetPackagesByPackageFamily ( PCWSTR(pkg_fam_name.as_ptr()),
    //                                       ptr::null_mut(),  None,
    //                                       &mut buf_len,  PWSTR::from_raw(buf.as_mut_ptr()) );
    //dbg!(&_res);
    //let _res = GetPackagesByPackageFamily ( PCWSTR(pkg_fam_name.as_ptr()),
    //                                       &mut num_pkg_full_names,  None,
    //                                       &mut buf_len,  PWSTR::from_raw(buf.as_mut_ptr()) );
    //dbg!(&_res);
    //dbg! ( PWSTR::from_raw (pkg_full_names.as_mut_ptr()) .to_string() );
    //dbg! ( pkg_full_names.get(0) .map(|p| p.as_ref()).flatten().map(|p|p.to_hstring()) );
    //let mut full_names_vec = std::slice::from_raw_parts (pkg_full_names.as_mut_ptr() as *const PWSTR, num_pkg_full_names as _);
    //dbg! (full_names_vec.as_ref().len());
    //dbg! (full_names_vec.as_ref().clone());
    //dbg! (&buf);
    //dbg! ( PWSTR::from_raw (buf.as_mut_ptr()) .to_string() );
    //if full_names_vec.as_ref().is_empty() { return None }

    //dbg! ( pkg_full_names.get(0).map(|p| (*p).read().to_string()) );
    //dbg! ( *pkg_full_names )
    //full_names_vec .iter() .for_each (|p| println! ("{:?}", p) );

    //dbg! ( PWSTR::from_raw (full_names_vec.as_ptr() as *mut _) .to_string() );
    //dbg! ( PWSTR::from_raw (full_names_vec[0].as_ptr()) .to_string() );
    //full_names_vec .iter() .for_each (|p| println! ("{:?}", PWSTR::from_raw (p.as_ptr()) .to_string()) );
    //full_names_vec .iter() .for_each (|p| println! ("{:?}", p.to_string()) );

    let package = PWSTR::from_raw (buf.as_mut_ptr()); //.to_string();
    // todo ^^ note that we'll pick up the first of possibly multiple packages in this family

    let mut pkg_path_len = 0u32;
    let _res = GetPackagePathByFullName (PCWSTR(package.as_ptr()), &mut pkg_path_len, PWSTR::null() );
    //dbg!((res, pkg_path_len));
    let mut pkg_path : Vec<u16> = Vec::with_capacity(pkg_path_len as _);
    let _res = GetPackagePathByFullName (PCWSTR(package.as_ptr()), &mut pkg_path_len, PWSTR::from_raw(pkg_path.as_mut_ptr()));
    //dbg! (res);
    //dbg! (PWSTR::from_raw(pkg_path.as_mut_ptr()).to_hstring());

    PWSTR::from_raw(pkg_path.as_mut_ptr()).to_string().ok()


} }



pub fn check_cur_proc_elevated() -> Option<bool> { unsafe {
    check_proc_elevated (GetCurrentProcess())
} }
pub fn check_proc_elevated (h_proc:HANDLE) -> Option<bool> { unsafe {
    let mut h_token = HANDLE::default();
    let open_success = OpenProcessToken (h_proc, TOKEN_READ, &mut h_token);
    if !open_success.as_bool() { return None }

    None
} }


// we'll use a static rwlocked vec to store child-windows from callbacks, and a mutex to ensure only one child-windows call is active
static child_windows_lock : Lazy <Arc <Mutex <()>>> = Lazy::new (|| Arc::new ( Mutex::new(())));
static child_windows : Lazy <Arc <RwLock <Vec <Hwnd>>>> = Lazy::new (|| Arc::new ( RwLock::new (vec!()) ) );

pub fn get_child_windows (hwnd:Hwnd) -> Vec<isize> { unsafe {
    let lock = child_windows_lock.lock().unwrap();
    *child_windows.write().unwrap() = vec!();
    EnumChildWindows ( HWND(hwnd), Some(enum_child_windows_cb), LPARAM::default() );
    let cws = child_windows.read().unwrap().clone();
    drop(lock);
    cws
} }
pub unsafe extern "system" fn enum_child_windows_cb (hwnd:HWND, _:LPARAM) -> BOOL {
    child_windows.write().unwrap().push(hwnd.0);
    BOOL (true as i32)
}





