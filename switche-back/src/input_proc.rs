#![ allow (non_snake_case, clippy::missing_safety_doc) ]

use std::sync::Arc;
use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
use std::os::raw::c_int;
use std::ops::Deref;
use std::time;
use std::thread::{sleep, spawn};

use once_cell::sync::OnceCell;

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM, LRESULT, POINT, BOOL, GetLastError};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL, CallNextHookEx, HHOOK, KBDLLHOOKSTRUCT, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_KEYUP, WH_MOUSE_LL, WINDOWS_HOOK_ID, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_MOUSEWHEEL, MSLLHOOKSTRUCT, SetCursorPos, GetCursorPos, WM_USER, PostThreadMessageW};
use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, MOUSE_EVENT_FLAGS, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, SendInput, VIRTUAL_KEY, VK_LMENU, VK_MENU, VK_RMENU, VK_SPACE, VK_TAB};

use crate::*;
use crate::switche::SwitcheState;




// extra-info identifier that we inject in sent kbd events (alt-release) so we can watch and ignore it when it comes back
//const INJECTED_IDENTIFIER_EXTRA_INFO: usize = 0xFFC3D44F;     // ahk/krusty stamp
const SWITCHE_INJECTED_IDENTIFIER_EXTRA_INFO: usize = 0x5317C7EE;      // switche's own stamp

const KILL_MSG : u32 = WM_USER + 1;

fn hi_word(l: u32) -> u16 { ((l >> 16) & 0xffff) as u16 }


# [ derive (Debug) ]
pub struct _InputProcessor {
    kbd_hook     : AtomicIsize,
    mouse_hook   : AtomicIsize,
    iproc_thread : AtomicU32,
}

# [ derive (Debug, Clone) ]
pub struct InputProcessor ( Arc <_InputProcessor> );

impl Deref for InputProcessor {
    type Target = _InputProcessor;
    fn deref (&self) -> &_InputProcessor { &self.0 }
}

impl InputProcessor {

    pub fn instance () -> InputProcessor {
        static INSTANCE: OnceCell <InputProcessor> = OnceCell::new();
        INSTANCE .get_or_init ( || {
            InputProcessor ( Arc::new ( _InputProcessor {
                kbd_hook     : AtomicIsize::default(),
                mouse_hook   : AtomicIsize::default(),
                iproc_thread : AtomicU32::default(),
            } ) )
        } ) .clone()
    }


    fn store_iproc_thread (&self) { unsafe {
        self.iproc_thread.store (GetCurrentThreadId(), Ordering::SeqCst);
    } }
    fn kill_iproc_thread (&self) {
        if self.iproc_thread.load(Ordering::SeqCst) != 0 { unsafe {
            let _ = PostThreadMessageW ( self.iproc_thread.load(Ordering::SeqCst), KILL_MSG, WPARAM::default(), LPARAM::default() );
            self.iproc_thread.store (0, Ordering::SeqCst);
        } }
    }

    fn set_kbd_hook   (&self)  { set_hook ( WH_KEYBOARD_LL,  &self.kbd_hook,   kbd_hook_cb   ) }
    fn set_mouse_hook (&self)  { set_hook ( WH_MOUSE_LL,     &self.mouse_hook, mouse_hook_cb ) }

    pub fn unset_kbd_hook   (&self) -> bool  { unset_hook (&self.kbd_hook  ) }
    pub fn unset_mouse_hook (&self) -> bool  { unset_hook (&self.mouse_hook) }

    pub fn re_set_hooks (&self) {
        // first we'll unhook any prior hooks and signal prior input-processing thread to terminate
        self.unset_kbd_hook();
        self.unset_mouse_hook();
        self.kill_iproc_thread();

        // then we can re-set hooks and start the input-proc thread
        self.begin_input_processing();
    }


    /// Starts listening for bound input events.
    pub fn begin_input_processing (&self) {

        spawn ( || unsafe {

            let ss = SwitcheState::instance();
            let mut some_hook_set = false;

            if ss.conf.check_flag__alt_tab_enabled() {
                ss.i_proc.set_kbd_hook(); some_hook_set = true;
            }
            if ss.conf.check_flag__rbtn_scroll_enabled() {
                ss.i_proc.set_mouse_hook(); some_hook_set = true;
            }
            if !some_hook_set {
                println! ("WARNING: no hooks set .. no input-processing started !!");
                return
            }

            // we'll store this thread's id in case we need to unhook the hooks later and want to terminate this thread
            ss.i_proc.store_iproc_thread();

            // before starting to listen to events, lets set this thread dpi-aware
            win_apis::win_set_thread_dpi_aware();

            // also, we might as well set the whole process higher priority, as we dont want lag in basic input processing
            let _ = win_apis::win_set_cur_process_priority_high();

            // win32 sends hook events to a thread with a 'message loop', but we dont create any windows,
            //  so we wont get any actual messages, so we can just leave a forever waiting GetMessage instead of setting up a msg-loop
            // .. basically while its waiting, the thread is awakened simply to call kbd hook (for an actual msg, itd awaken give the msg)
            let mut msg: MSG = MSG::default();
            while BOOL(0) != GetMessageW (&mut msg, HWND(0), 0, 0) {
                if msg.message == KILL_MSG {
                    println! ("received kill-msg in input-processing thread .. terminating thread ..");
                    break
                }
            }
        } );
    }

}




fn set_hook (
    hook_id: WINDOWS_HOOK_ID,
    hhook: &AtomicIsize,
    hook_proc: unsafe extern "system" fn (c_int, WPARAM, LPARAM) -> LRESULT,
) { unsafe {
    SetWindowsHookExW (hook_id, Some(hook_proc), HINSTANCE(0), 0) .iter() .for_each ( |hh|
        hhook.store (hh.0, Ordering::SeqCst)
    );
} }



fn unset_hook (hhook: &AtomicIsize) -> bool {
    if hhook.load (Ordering::SeqCst) != HHOOK::default().0 {
        if unsafe { UnhookWindowsHookEx ( HHOOK (hhook.load(Ordering::SeqCst)) ) .is_ok() } {
            hhook.store (HHOOK::default().0, Ordering::SeqCst);
            println!("unhooking attempt .. succeeded!");
            return true
        }
        // we'll print the error both to console, and windows debug-out (which can be checked via dbgview at runtime)
        let err = format! ("SWITCHE : unhooking attempt failed .. error code : {:?} !!", unsafe { GetLastError().0 } );
        eprintln!("{}", &err);
        win_apis::write_win_dbg_string (&err)
    } else {
        println!("unhooking attempt .. no prior hook found !!");
    }
    false
}







fn is_alt_key (vk_code:u32) -> bool {
    vk_code == VK_MENU.0 as u32 || vk_code == VK_LMENU.0 as u32 || vk_code == VK_RMENU.0 as u32
}


fn _print_kbd_event (wp:&WPARAM, kbs:&KBDLLHOOKSTRUCT) {
    println!("w_param: {:X}, vk_code: {:?}, scanCode: {:#06X}, flags: {:#018b}, time: {}, dwExtraInfo: {:X}",
             wp.0, kbs.vkCode, kbs.scanCode, kbs.flags.0, kbs.time, kbs.dwExtraInfo);
}

/// Keyboard lower-level-hook processor
pub unsafe extern "system" fn kbd_hook_cb (code:c_int, w_param:WPARAM, l_param:LPARAM) -> LRESULT {

    let return_call  = || CallNextHookEx(HHOOK(0), code, w_param, l_param);
    let return_block = || LRESULT(1);    // returning with non-zero code signals OS to block further processing on the input event

    if code < 0 { return return_call() }

    let kbs = *(l_param.0 as *const KBDLLHOOKSTRUCT);

    if kbs.dwExtraInfo == SWITCHE_INJECTED_IDENTIFIER_EXTRA_INFO {
        //println! ("ignoring swi injected event");
        return return_call()
    }

    //println! ("vk: {:?}, ev: {:?}, inj: {:?}", kbs.vkCode, w_param.0, kbs.dwExtraInfo);

    if w_param.0 as u32 == WM_SYSKEYDOWN && is_alt_key(kbs.vkCode) {  //println! ("alt-press");
        let ss = SwitcheState::instance();
        if ss.was_alt_preloaded.is_clear() {
            ss.was_alt_preloaded.set();
            ss.handle_req__enum_query_preload();    // (this just queues it up for off-thread processing)
        }
        return return_call()
    }
    else if w_param.0 as u32 == WM_KEYUP  && is_alt_key(kbs.vkCode) {
        // for actual alt-release, we'll spawn to queue events at bottom of msg queue, and have an alt release sent out
        // (note thread spawning to give OS time to process between our execution chunks)
        let ss = SwitcheState::instance();
        ss.was_alt_preloaded.clear();
        if ss.in_alt_tab.is_set() {
            //send_dummy_key_release();
            send_alt_release();
            // ^^ note that this alt-release here seems to be CRITICAL in covering corner cases like wanting to do alt-tab while drag-drop ..
            // .. esp while there's something else like krusty behind us in the hook chain (and therefore gets the last input?) etc
            // .. Not even a dummy-key release seems to work, has to be an alt press or rel .. presumably win32 has special case for alt-tab?
            spawn ( move || {
                sleep(time::Duration::from_millis(50));   // we want this delay to be no less than the sleep-delay in tab-press processing
                ss.proc_hot_key__scroll_end();
                ss.in_alt_tab.clear();
            } );
        }
        return return_call()
        // ^^ note that we cant block it even if we send out a replacement because it could be left/right/virt whatever
    }
    else if w_param.0 as u32 == WM_SYSKEYDOWN  && kbs.vkCode == VK_TAB.0 as u32 {  //println! ("alt-tab");
        // when we get an actual alt-tab, we'll block the tab from going out to avoid conflict w native alt-tab
        // further, to remain in windows graces since it only lets us call fgnd if we handled last input etc, we'll send ourselves an input
        // the spawning in thread is VERY important, as that gives OS time to process msgs before we try to bring switche fgnd
        //send_dummy_key_release();
        spawn ( move || {
            let ss = SwitcheState::instance();
            ss.in_alt_tab.set();    // set this flag up first in case the alt-release comes during our sleep delay below
            sleep(time::Duration::from_millis(50));
            // ^^ this allows krusty to get the tab-up out as well, so we dont get interspersed (and thereby lose our last-input status?)
            // (plus it avoids possible races w krusty hooks, or win report handling etc that can interefere w bringing us fgnd)
            send_dummy_key_release();
            if win_apis::check_shift_active() {
                ss.proc_hot_key__scroll_up();
            } else {
                ss.proc_hot_key__scroll_down();
            }
        } );
        return return_block()
    }
    else if w_param.0 as u32 == WM_SYSKEYUP  && kbs.vkCode == VK_TAB.0 as u32 {
        // for tab release (w alt), we simply block it to keep balance, but its not that big a deal either way
        return return_block()
    }
    else if w_param.0 as u32 == WM_SYSKEYDOWN && kbs.vkCode == VK_SPACE.0 as u32 {
        // we want to use alt-space to disarm alt-tab etc when swi fgnd
        // .. we need this here as OS captures alt-space and the browser/webapp doesnt ever see it
        let ss = SwitcheState::instance();
        if ss.is_fgnd.is_set() {
            spawn ( move || ss.proc_hot_key__scroll_end_disarm() );
            return return_block()
        }
        return return_call()
    }
    return_call()
}


pub fn send_alt_press()   { send_key_event (VK_MENU, false) }
pub fn send_alt_release() { send_key_event (VK_MENU, true) }
pub fn send_dummy_key_release() { send_key_event (VIRTUAL_KEY(0x9A), true) }

fn send_key_event (virt_key:VIRTUAL_KEY, is_key_up:bool) {
    let no_flag = KEYBD_EVENT_FLAGS::default();
    let keyup_flag = if is_key_up { KEYEVENTF_KEYUP } else { no_flag };
    let (scan_code, sc_flag, ext_key_flag) = (0, no_flag, no_flag);

    let inputs = [ INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: virt_key,
                wScan: scan_code,
                dwFlags: ext_key_flag | sc_flag | keyup_flag,
                time: 0,
                dwExtraInfo: SWITCHE_INJECTED_IDENTIFIER_EXTRA_INFO,
        } }
    } ];
    unsafe { SendInput (&inputs, core::mem::size_of::<INPUT>() as c_int) };
}










/// mouse lower-level-hook processor
pub unsafe extern "system"
fn mouse_hook_cb (code: c_int, w_param: WPARAM, l_param: LPARAM) -> LRESULT {

    let return_call = || { CallNextHookEx(HHOOK(0), code, w_param, l_param) };

    if code < 0 { return return_call() }      // ms-docs says we MUST do this, so ig k fine

    let mhs = *(l_param.0 as *const MSLLHOOKSTRUCT);

    if mhs.dwExtraInfo == SWITCHE_INJECTED_IDENTIFIER_EXTRA_INFO {
        // if we injected it ourselves, we should just bail (and call the next guy down the line)
        return return_call()
    }
    //println!("{:#?}", mh_struct);

    let ss = SwitcheState::instance();

    if w_param.0 as u32 == WM_RBUTTONDOWN {
        ss.is_mouse_right_down.set();
        ss.handle_req__enum_query_preload();
    }
    else if w_param.0 as u32 == WM_RBUTTONUP {
        if ss.is_mouse_right_down.is_set() {
            ss.is_mouse_right_down.clear();
            if ss.in_right_btn_scroll_state.is_set() {
                ss.in_right_btn_scroll_state.clear();
                spawn (move || {
                    send_dummy_key_release();
                    // ^^ this makes sure switche gets the last input-event before it tries to change fgnd (to satisfy win32 rules)
                    ss.proc_hot_key__scroll_end();
                } );
                return LRESULT(1);
                // ^^ we've already done a release when we started the scroll state, so we gotta block this one
            } // else passes through
        }
    }
    else if w_param.0 as u32 == WM_MOUSEWHEEL {
        let delta = hi_word(mhs.mouseData) as i16 as i32;
        if ss.is_mouse_right_down.is_set() {
            if ss.in_right_btn_scroll_state.is_clear() || ss.is_fgnd.is_clear() {
                ss.in_right_btn_scroll_state.set();
                spawn ( move || {
                    send_dummy_key_release();   // this again helps bring switche to fgnd by letting it have last input-event
                    right_btn_dn_scroll_action(delta, ss);
                    send_mouse_rbtn_release_masked();
                    // ^^ we want the release to happen after switche has been brought to fgnd (hence after scroll action)
                } );
            } else if ss.is_fgnd.is_set() { // ie. swi already invoked and in fgnd
                right_btn_dn_scroll_action(delta, ss);
            }
            return LRESULT(1);
            // ^^ we'll block all wheel events while in right-btn-scroll state
        } else if ss.in_alt_tab.is_set() && ss.is_fgnd.is_set() {
            right_btn_dn_scroll_action(delta, ss);
            return LRESULT(1);
        }
    }

    return_call()
    // ^^ any other case than explicitly cut short above, we'll let it pass through
}

fn right_btn_dn_scroll_action (delta:i32, ss:SwitcheState) {
    if delta > 0 { ss.proc_hot_key__scroll_up() }
    else         { ss.proc_hot_key__scroll_down() }
}




fn get_cursor_pos() -> POINT {
    unsafe {
        let mut point = POINT::default();
        let _ = GetCursorPos (&mut point);
        point
    }
}
fn cursor_move_abs (x: i32, y: i32) { unsafe { 
    let _ = SetCursorPos (x, y);
} }



fn send_mouse_rbtn_release() {
    send_mouse_input (MOUSEEVENTF_RIGHTUP, 0, 0, 0);
}

#[allow(dead_code)]
fn send_mouse_rbtn_release_at(x:i32, y:i32) {
    send_mouse_input (MOUSEEVENTF_RIGHTUP | MOUSEEVENTF_ABSOLUTE, 0, x, y);
}
fn send_mouse_rbtn_release_masked() {
    // when we have to release the rbtn, to minimize triggering the context menu, we'll release it inside switche-window
    //send_mouse_right_btn_release_at (0xFFFF, 0xFFFF);
    // ^^ ugh this doesnt seem to actually do that .. so instead we'll manually move there, release, then restore

    // and looks like for such a manual-move strategy to work, there HAS to be a delay before we move the pointer back
    // (also, this works for almost all applications EXCEPT for windows-explorer .. MS ofc has to be special .. meh)

    win_apis::win_set_thread_dpi_aware();

    let cursor_orig_loc = get_cursor_pos();
    //println! ("cursor-orig-loc: {:?}", cursor_orig_loc);

    //let cursor_mask_loc = POINT { x: 0xFFFF, y: 0xFFFF };
    // ^^ screen edge avoids the clicked-loc context menu .. but still produces a tiny desktop context menu

    // so we'll move to the bottom-right corner of the switche window instead .. that should prevent context menu from MOST apps
    let self_rect = win_apis::win_get_window_frame (SwitcheState::instance().get_self_hwnd());
    let cursor_mask_loc = POINT { x: self_rect.right - 10, y: self_rect.bottom - 10 };
    //println! ("cursor-mask-loc: {:?}", cursor_mask_loc);

    // we'll add a delay before release, to avoid focus stealing from switche intended window etc
    cursor_move_abs (cursor_mask_loc.x, cursor_mask_loc.y);
    sleep(time::Duration::from_millis(20));
    send_mouse_rbtn_release();
    // we'll add some delay to actually have the release processed while pointer is still away
    sleep(time::Duration::from_millis(40));
    // note that reducing this delay or even removing it will mostly work (as the event handling can happen before spawned thread comes up)..
    // .. however, for times when there's load etc and the event handling is also pushed out, we'll want at least some delay
    cursor_move_abs (cursor_orig_loc.x, cursor_orig_loc.y);
}


/// Send simulated mouse events to OS for injection into events-stream
fn send_mouse_input (flags: MOUSE_EVENT_FLAGS, data: u32, dx: i32, dy: i32) {
    let inputs = [ INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi : MOUSEINPUT {
                dx,
                dy,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: SWITCHE_INJECTED_IDENTIFIER_EXTRA_INFO
        } }
    } ];
    unsafe { SendInput (&inputs, std::mem::size_of::<INPUT>() as c_int) };
}
