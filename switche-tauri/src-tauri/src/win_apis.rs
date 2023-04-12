

use std::ffi::{c_void};
use std::mem::size_of;

use windows::core::PSTR;
use windows::Win32::Foundation::{BOOL, CloseHandle, HANDLE, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameA};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowPlacement, ShowWindow, GetWindowTextA, GetWindowTextW,
    IsWindowVisible, GetWindowThreadProcessId, PostMessageA, SetForegroundWindow, ShowWindowAsync,
    WM_CLOSE, SW_HIDE, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SW_SHOW, SW_SHOWMINIMIZED, WINDOWPLACEMENT,
};


use crate::*;



pub fn check_window_visible (hwnd:Hwnd) -> bool { unsafe {
    IsWindowVisible (HWND(hwnd)) .as_bool()
} }
pub fn get_fgnd_window () -> Option<Hwnd> { unsafe {
    Some ( GetForegroundWindow().0 as Hwnd )
} }

pub fn check_window_cloaked (hwnd:Hwnd) -> bool { unsafe {
    let mut cloaked_state: isize = 0;
    let out_ptr = &mut cloaked_state as *mut isize as *mut c_void;
    let _ = DwmGetWindowAttribute (HWND(hwnd), DWMWA_CLOAKED, out_ptr, size_of::<isize>() as u32);
    cloaked_state != 0
} }

pub fn __get_window_text (hwnd:Hwnd) -> String { unsafe {
    // todo : this doesnt seem to be working for unicode?
    const MAX_LEN : usize = 1024;
    let mut lpstr = [0u8; MAX_LEN];
    //let out_ptr = PSTR::from_raw (lpstr.as_mut_ptr());
    let copied_len = GetWindowTextA (HWND(hwnd), &mut lpstr);
    if copied_len > 0 && copied_len < MAX_LEN as i32 {
        //CStr::from_ptr (lpstr.as_ptr() as *const _).to_str().unwrap_or_default().to_string()
        //PSTR::from_raw (lpstr.as_mut_ptr()) .to_string() .unwrap_or_default()
        //std::str::from_utf8(&lpstr).unwrap_or("").to_string()
        String::from_utf8_lossy(PSTR::from_raw(lpstr.as_mut_ptr()).as_bytes().into()).to_string()
        // ^^ todo: see if makes sense to do all string work everywhere with cow instead of cloned strings
    } else { "".into() }
} }
pub fn get_window_text (hwnd:Hwnd) -> String { unsafe {
    const MAX_LEN : usize = 512;
    let mut lpstr = [0u16; MAX_LEN];
    let copied_len = GetWindowTextW (HWND(hwnd), &mut lpstr);
    String::from_utf16_lossy (&lpstr[..(copied_len as _)])
    // ^^ todo: see if makes sense to do all string work everywhere with cow instead of cloned strings
} }



pub fn get_exe_path_name (hwnd:Hwnd) -> Option<String> { unsafe {
    const MAX_LEN : usize = 1024;
    let mut pid : u32 = 0;
    let _ = GetWindowThreadProcessId (HWND(hwnd), Some(&mut pid));
    let handle = OpenProcess (PROCESS_QUERY_LIMITED_INFORMATION, BOOL::from(false), pid);
    let mut lpstr = [0u8; MAX_LEN];
    let mut lpdwsize = MAX_LEN as u32;
    if handle.is_err() { return None }
    let _ = QueryFullProcessImageNameA ( HANDLE (handle.as_ref().unwrap().0), PROCESS_NAME_WIN32, PSTR::from_raw(lpstr.as_mut_ptr()), &mut lpdwsize );
    handle .iter() .for_each ( |h| { CloseHandle(*h); } );
    PSTR::from_raw (lpstr.as_mut_ptr()) .to_string() .ok()
} }








pub fn window_activate (hwnd:Hwnd) { unsafe { println!("winapi activate {:?}",hwnd);
    let mut win_state =  WINDOWPLACEMENT::default();
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
    ShowWindow (HWND(hwnd), SW_MINIMIZE);
} }
pub fn window_maximize (hwnd:Hwnd) { unsafe {
    ShowWindow (HWND(hwnd), SW_MAXIMIZE);
} }

pub fn window_close (hwnd:Hwnd) { unsafe { println!("winapi close {:?}",hwnd);
    //CloseWindow(HWND(hwnd));
    // note ^^ that the u32 'CloseWindow' cmd actually minimizes it, to close, send it a WM_CLOSE msg
    PostMessageA (HWND(hwnd), WM_CLOSE, WPARAM::default(), LPARAM::default());
} }







