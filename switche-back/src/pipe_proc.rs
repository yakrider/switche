

use std::thread;
use serde::{Deserialize};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{PIPE_ACCESS_INBOUND, ReadFile};
use windows::Win32::System::Pipes::{CreateNamedPipeW, ConnectNamedPipe, DisconnectNamedPipe, PIPE_TYPE_MESSAGE, PIPE_READMODE_MESSAGE, PIPE_WAIT, PIPE_REJECT_REMOTE_CLIENTS};

use crate::switche::SwitcheState;


const PIPE_NAME: &str = r"\\.\pipe\switche_krusty_cmd_pipe";


#[derive(Debug, Deserialize)]
pub enum PipeCommand {
    Invoke,
    ScrollDown,
    ScrollUp,
    ScrollEnd,
    ScrollEndDisarm,

    SnapListRefresh,
    SnapListSwitchNext,
    SnapListSwitchPrev,
    SnapListSwitchTop,
    SnapListSwitchBottom,

    SwitchNextNonMinimized,
    SwitchZIndex(usize),
    SwitchApp {
        exes: Vec<String>,
        title: Option<String>,
        partial: bool,
    },
}


pub fn handle_pipe_cmd (cmd: PipeCommand) {
    use PipeCommand::*;
    let ss = SwitcheState::instance();

    match cmd {
        Invoke            =>  ss.proc_hot_key__invoke(),
        ScrollDown        =>  ss.proc_hot_key__scroll_down(),
        ScrollUp          =>  ss.proc_hot_key__scroll_up(),
        ScrollEnd         =>  ss.proc_hot_key__scroll_end(),
        ScrollEndDisarm   =>  ss.proc_hot_key__scroll_end_disarm(),
        // ^^ note that backend scroll-up/dn always arm scroll-end activation

        SnapListRefresh       =>  ss.proc_hot_key__snap_list_refresh(),
        SnapListSwitchNext    =>  ss.proc_hot_key__snap_list_switch (|sl| sl.next_hwnd()   ),
        SnapListSwitchPrev    =>  ss.proc_hot_key__snap_list_switch (|sl| sl.prev_hwnd()   ),
        SnapListSwitchTop     =>  ss.proc_hot_key__snap_list_switch (|sl| sl.top_hwnd()    ),
        SnapListSwitchBottom  =>  ss.proc_hot_key__snap_list_switch (|sl| sl.bottom_hwnd() ),

        SwitchNextNonMinimized  =>  ss.proc_hot_key__switch_next_non_minimized(),
        SwitchZIndex (z)        =>  ss.proc_hot_key__switch_z_idx(z),

        SwitchApp { exes, title, partial } => {
            ss.proc_hot_key__switch_app ( &exes, title.as_deref(), partial );
        }
    }
}



pub fn start_pipe_processor() {

    thread::spawn (move || {

        let pipe_name_w : Vec<u16> = PIPE_NAME.encode_utf16() .chain (std::iter::once(0)) .collect();
        let pipe_name_w = PCWSTR (pipe_name_w.as_ptr());

        loop { unsafe {

            // first we create our instance of named pipe
            let pipe = CreateNamedPipeW (
                pipe_name_w,
                PIPE_ACCESS_INBOUND,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                16,      // Max instances
                0,       // Out buffer size
                4096,    // In buffer size
                0,       // Default timeout
                None,
            );
            if pipe == INVALID_HANDLE_VALUE {
                continue;
            }
            // then we'll wait for some client to actually connect to it
            let res = ConnectNamedPipe (pipe, None);
            if res.is_err() {
                continue;
            }
            // and if so, we'll read and process it
            // (we'll spawn out the actual cmd processing so we can immediately go back to waiting state again)
            let mut buffer = [0u8; 4096];
            let mut bytes_read = 0;
            if ReadFile ( pipe, Some (&mut buffer), Some (&mut bytes_read), None ) .is_ok() {
                if let Ok(cmd) = serde_json::from_slice::<PipeCommand> ( & buffer [..bytes_read as usize] ) {
                    thread::spawn ( move || {
                        tracing::info!("received pipe-cmd: {:?}", &cmd);
                        handle_pipe_cmd (cmd)
                    } );
                }
            }
            let _ = DisconnectNamedPipe (pipe);
            let _ = CloseHandle (pipe);
        } }
    } );
}
