#![ allow (non_upper_case_globals, non_snake_case) ]

use std::ffi::c_void;
use std::mem::size_of;
use std::sync::{Arc, Mutex, RwLock};
use once_cell::sync::Lazy;

use windows::core::{GUID, PCWSTR, PSTR, PWSTR};
use windows::Win32::Foundation::{BOOL, CloseHandle, GetLastError, ERROR_INSUFFICIENT_BUFFER, HANDLE, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
use windows::Win32::Storage::Packaging::Appx::{
    GetApplicationUserModelId, GetPackagePathByFullName, GetPackagesByPackageFamily, ParseApplicationUserModelId
};
use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;
use windows::Win32::System::Threading::{
    GetCurrentProcess, HIGH_PRIORITY_CLASS, OpenProcess, OpenProcessToken, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameA, SetPriorityClass
};
use windows::Win32::System::WindowsProgramming::GetUserNameW;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_SHIFT};
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, PROPERTYKEY, SHGetPropertyStoreForWindow};
use windows::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_SYSTEM_AWARE, SetThreadDpiAwarenessContext};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowPlacement, GetWindowTextW, IsWindowVisible, GetAncestor, GetWindowThreadProcessId, PostMessageA, SetForegroundWindow, ShowWindowAsync, GetWindowLongW, WINDOWPLACEMENT, EnumChildWindows, SystemParametersInfoW, WM_CLOSE, SW_HIDE, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SW_SHOW, SW_SHOWMINIMIZED, WS_CHILD, GWL_STYLE, GA_ROOTOWNER, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, GWL_EXSTYLE, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, MoveWindow, GA_PARENT, GetWindow, GW_OWNER};


use crate::switche::Hwnd;



pub fn check_window_visible (hwnd:Hwnd) -> bool { unsafe {
    IsWindowVisible (HWND(hwnd)) .as_bool()
} }

pub fn check_window_cloaked (hwnd:Hwnd) -> bool { unsafe {
    let mut cloaked_state: isize = 0;
    let out_ptr = &mut cloaked_state as *mut isize as *mut c_void;
    let _ = DwmGetWindowAttribute (HWND(hwnd), DWMWA_CLOAKED, out_ptr, size_of::<isize>() as u32);
    cloaked_state != 0
} }

pub fn check_if_app_window (hwnd:Hwnd) -> bool { unsafe {
    GetWindowLongW (HWND(hwnd), GWL_EXSTYLE) & WS_EX_APPWINDOW.0 as i32 != 0
} }

pub fn check_if_tool_window (hwnd:Hwnd) -> bool { unsafe {
    GetWindowLongW (HWND(hwnd), GWL_EXSTYLE) & WS_EX_TOOLWINDOW.0 as i32 != 0
} }

pub fn get_window_parent (hwnd:Hwnd) -> Hwnd { unsafe {
    GetAncestor (HWND(hwnd), GA_PARENT).0 as Hwnd
} }
pub fn get_window_owner (hwnd:Hwnd) -> Hwnd { unsafe {
    GetWindow (HWND(hwnd), GW_OWNER).0 as Hwnd
} }
pub fn get_window_root_owner (hwnd:Hwnd) -> Hwnd { unsafe {
    println!("owner of {:?} : {:?}",hwnd, GetAncestor (HWND(hwnd), GA_ROOTOWNER).0 as Hwnd);
    GetAncestor (HWND(hwnd), GA_ROOTOWNER).0 as Hwnd
} }
pub fn check_window_has_owner (hwnd:Hwnd) -> bool { unsafe {
    GetAncestor (HWND(hwnd), GA_ROOTOWNER).0 as Hwnd != hwnd
} }

pub fn check_window_is_child (hwnd:Hwnd) -> bool { unsafe {
    (GetWindowLongW (HWND(hwnd), GWL_STYLE) & WS_CHILD.0 as i32) != 0
} }

pub fn get_fgnd_window () -> Hwnd { unsafe {
    GetForegroundWindow().0 as Hwnd
} }



pub fn window_activate (hwnd:Hwnd) { unsafe { println!("winapi activate {:?}",hwnd);
    //ShowWindowAsync (HWND(hwnd), SW_NORMAL);
    // ^^ this will cause minimized/maximized windows to be restored
    let mut win_state =  WINDOWPLACEMENT::default();
    let _ = GetWindowPlacement (HWND(hwnd), &mut win_state);
    if win_state.showCmd == SW_SHOWMINIMIZED.0 as u32 {
        //ShowWindow (HWND(hwnd), SW_RESTORE);
        ShowWindowAsync (HWND(hwnd), SW_RESTORE);
    } else {
        //ShowWindow (HWND(hwnd), SW_SHOW);
        ShowWindowAsync (HWND(hwnd), SW_SHOW);
    }
    //keybd_event (0, 0, KEYBD_EVENT_FLAGS::default(), 0);
    SetForegroundWindow (HWND(hwnd));
    //std::thread::spawn ( move || SetForegroundWindow (HWND(hwnd)) );
    // its a lil flaky, so we'll try the another call too, (plus the little delay from spawn should also help)
    //std::thread::spawn (move || SwitchToThisWindow (HWND(hwnd), BOOL::from(true)) );
    // ^^ appears basically the same as above calls, doesnt help when have issues, else isnt necessary
} }


pub fn window_hide (hwnd:Hwnd) { unsafe { println!("winapi hide {:?}",hwnd);
    //ShowWindow (HWND(hwnd), SW_HIDE);
    // ^^ since this calls from our thread, this can apparently be unable to remove kbd focus from the window being hidden!!
    ShowWindowAsync (HWND(hwnd), SW_HIDE);
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
    let _ = PostMessageA (HWND(hwnd), WM_CLOSE, WPARAM::default(), LPARAM::default());
} }



pub fn get_window_text (hwnd:Hwnd) -> String { unsafe {
    const MAX_LEN : usize = 512;
    let mut lpstr = [0u16; MAX_LEN];
    let copied_len = GetWindowTextW (HWND(hwnd), &mut lpstr);
    String::from_utf16_lossy (&lpstr[..(copied_len as _)])
    // ^^ todo: see if makes sense to do all string work everywhere with cow instead of cloned strings
} }

pub fn win_get_work_area () -> RECT { unsafe {
    let mut rect = RECT::default();
    let _ = SystemParametersInfoW (SPI_GETWORKAREA, 0, Some (&mut rect as *mut RECT as *mut c_void), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS::default());
    rect
} }

pub fn win_get_window_frame (hwnd:Hwnd) -> RECT { unsafe {
    // note that rect includes padding of the 'drop shadow' around the frame
    let mut rect = RECT::default();
    let _ = DwmGetWindowAttribute (HWND(hwnd), DWMWA_EXTENDED_FRAME_BOUNDS, &mut rect as *mut RECT as *mut c_void, size_of::<RECT>() as u32);
    rect
} }

pub fn win_move_to (hwnd:Hwnd, x:i32, y:i32, width:i32, height:i32) { unsafe {
    let _ = MoveWindow (HWND(hwnd), x, y, width, height, true);
    // ^^ the bool param at end flags whether to repaint or not
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
    handle .iter() .for_each ( |h| { let _ = CloseHandle(*h); } );
    PSTR::from_raw (lpstr.as_mut_ptr()) .to_string() .ok()
} }

pub fn get_uwp_hwnd_exe_path (hwnd:Hwnd) -> Option<String> { unsafe {
    let mut frame_host_pid : u32 = 0;
    let _ = GetWindowThreadProcessId (HWND(hwnd), Some(&mut frame_host_pid));
    let uwp_pid = get_child_windows (hwnd) .iter() .map (|cwh| {
        let mut pid = 0u32;
        let _ = GetWindowThreadProcessId (HWND(*cwh), Some(&mut pid));
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

    let result = SHGetPropertyStoreForWindow(HWND(hwnd));
    if result.is_err() { return None }
    let store : IPropertyStore = result.unwrap();
    //dbg!(&store);

    let PKEY_AppUserModel_ID = PROPERTYKEY {fmtid: GUID::from("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3"), pid:5 };
    let _res = store.GetCount();
    let _res = store.GetValue ( &PKEY_AppUserModel_ID as *const _ );

    let aumid = _res.unwrap().as_raw().Anonymous.Anonymous.Anonymous.pwszVal;


    let mut pkg_fam_name_len = 0u32;
    let mut pkg_rel_app_name_len = 0u32;
    let _res = ParseApplicationUserModelId (PCWSTR(aumid.cast_const()), &mut pkg_fam_name_len, PWSTR::null(), &mut pkg_rel_app_name_len, PWSTR::null());
    //dbg! ((res, pkg_fam_name_len, pkg_rel_app_name_len));

    let mut pkg_fam_name     : Vec<u16> = Vec::with_capacity (pkg_fam_name_len as _);
    let mut pkg_rel_app_name : Vec<u16> = Vec::with_capacity (pkg_rel_app_name_len as _);
    let _res = ParseApplicationUserModelId ( PCWSTR(aumid.cast_const()),
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

    let package = PWSTR::from_raw (buf.as_mut_ptr());
    // todo ^^ note that we'll pick up the first of possibly multiple packages in this family

    let mut pkg_path_len = 0u32;
    let _res = GetPackagePathByFullName (PCWSTR(package.as_ptr()), &mut pkg_path_len, PWSTR::null() );
    //dbg!((res, pkg_path_len));
    let mut pkg_path : Vec<u16> = Vec::with_capacity(pkg_path_len as _);
    let _res = GetPackagePathByFullName (PCWSTR(package.as_ptr()), &mut pkg_path_len, PWSTR::from_raw(pkg_path.as_mut_ptr()));
    //dbg! (res);
    PWSTR::from_raw(pkg_path.as_mut_ptr()).to_string().ok()

} }



pub fn check_cur_proc_elevated () -> Option<bool> {
    match check_proc_elevated ( unsafe { GetCurrentProcess()} ) {
        Ok (res) => Some(res),
        Err (e) => {
            println!("Error checking process elevation : {:?}", e);
            None
    }  }
}
pub fn check_proc_elevated (h_proc:HANDLE) -> windows::core::Result<bool> { unsafe {
    let mut h_token = HANDLE::default();
    OpenProcessToken (h_proc, TOKEN_QUERY, &mut h_token)?;
    let mut token_info : TOKEN_ELEVATION = TOKEN_ELEVATION::default();
    let mut token_info_len = size_of::<TOKEN_ELEVATION>() as u32;
    GetTokenInformation (h_token, TokenElevation, Some(&mut token_info as *mut _ as *mut _), token_info_len, &mut token_info_len)?;
    Ok (token_info.TokenIsElevated != 0 )
} }


pub fn get_cur_user_name () -> Option<String> {
    match _get_cur_user_name() {
        Ok (res) => Some(res),
        Err (e) => {
            println!("Error getting current user name : {:?}", e);
            None
    }  }
}
pub fn _get_cur_user_name () -> Result<String, Box<dyn std::error::Error>> { unsafe {
    // we'll put some default size enough for most cases, but if name too long, we'll allocate and requery
    let mut name_len = 512;
    let mut buf = vec![0u16; name_len as usize];
    if GetUserNameW  (PWSTR::from_raw(buf.as_mut_ptr()), &mut name_len) .is_ok() {
        let name = PWSTR::from_raw (buf.as_mut_ptr()) .to_string()?;
        return Ok(name)
    }
    if GetLastError() != ERROR_INSUFFICIENT_BUFFER {
        return Err (Box::new(windows::core::Error::from_win32()))
    }
    // buffer wasnt large enough, we'll resize and try again
    if name_len > 8192 {
        return Err ("User name too long".into())
    }
    buf.resize (name_len as usize, 0);
    GetUserNameW  (PWSTR::from_raw(buf.as_mut_ptr()), &mut name_len)?;
    let name = PWSTR::from_raw(buf.as_mut_ptr()).to_string()?;
    Ok (name)
} }



pub fn win_set_cur_process_priority_high() -> bool { unsafe {
    SetPriorityClass (GetCurrentProcess(), HIGH_PRIORITY_CLASS) .is_ok()
} }

pub fn win_set_thread_dpi_aware() { unsafe {
    SetThreadDpiAwarenessContext (DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
} }

pub fn write_win_dbg_string (msg:&str) { unsafe {
    let msg_wide : Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
    OutputDebugStringW (PCWSTR(msg_wide.as_ptr()));
} }

pub fn check_shift_active () -> bool { unsafe {
    GetKeyState (VK_SHIFT.0 as i32) & 0x80 != 0
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

#[ allow (clippy::missing_safety_doc) ]
pub unsafe extern "system" fn enum_child_windows_cb (hwnd:HWND, _:LPARAM) -> BOOL {
    child_windows.write().unwrap().push(hwnd.0);
    BOOL (true as i32)
}





