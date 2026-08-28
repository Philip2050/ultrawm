use std::sync::mpsc;
use std::thread;
use std::io::{Read, Write};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Storage::FileSystem::*,
        System::Pipes::*,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcCommand {
    NextTheme,
    PrevTheme,
    FocusNext,
    FocusPrev,
    FocusLeft,
    FocusRight,
    FocusUp,
    FocusDown,
    PanLeft,
    PanRight,
    PanUp,
    PanDown,
    GrowWidth,
    ShrinkWidth,
    GrowHeight,
    ShrinkHeight,
    Close,
    Float,
    Unfloat,
    ToggleLauncher,
    Quit,
}

pub fn start_ipc_server(tx: mpsc::Sender<IpcCommand>) -> anyhow::Result<thread::JoinHandle<()>> {
    let handle = thread::spawn(move || {
        let pipe_name = r"\\.\pipe\ultrawm-ipc";
        loop {
            unsafe {
                use windows::Win32::Foundation::*;
                use windows::Win32::Storage::FileSystem::*;

                let pipe = CreateNamedPipeW(
                    PCWSTR(pipe_name.encode_utf16().chain(Some(0)).collect::<Vec<u16>>().as_ptr()),
                    PIPE_ACCESS_INBOUND,
                    PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                    1,
                    4096,
                    4096,
                    0,
                    None,
                );

                if pipe.is_invalid() {
                    log::error!("Failed to create named pipe: {:?}", std::io::Error::last_os_error());
                    thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }

                let _ = ConnectNamedPipe(pipe, None);

                let mut buf = [0u8; 4096];
                let mut bytes_read = 0u32;

                let result = ReadFile(
                    pipe,
                    Some(&mut buf),
                    Some(&mut bytes_read),
                    None,
                );

                if result.is_ok() && bytes_read > 0 {
                    let cmd = String::from_utf8_lossy(&buf[..bytes_read as usize]).trim().to_string();
                    log::debug!("IPC received: {}", cmd);
                    let command = match cmd.as_str() {
                        "next-theme" => IpcCommand::NextTheme,
                        "prev-theme" => IpcCommand::PrevTheme,
                        "focus-next" => IpcCommand::FocusNext,
                        "focus-prev" => IpcCommand::FocusPrev,
                        "focus-left" => IpcCommand::FocusLeft,
                        "focus-right" => IpcCommand::FocusRight,
                        "focus-up" => IpcCommand::FocusUp,
                        "focus-down" => IpcCommand::FocusDown,
                        "pan-left" => IpcCommand::PanLeft,
                        "pan-right" => IpcCommand::PanRight,
                        "pan-up" => IpcCommand::PanUp,
                        "pan-down" => IpcCommand::PanDown,
                        "grow-width" => IpcCommand::GrowWidth,
                        "shrink-width" => IpcCommand::ShrinkWidth,
                        "grow-height" => IpcCommand::GrowHeight,
                        "shrink-height" => IpcCommand::ShrinkHeight,
                        "close" => IpcCommand::Close,
                        "float" => IpcCommand::Float,
                        "unfloat" => IpcCommand::Unfloat,
                        "launcher" => IpcCommand::ToggleLauncher,
                        "quit" => IpcCommand::Quit,
                        _ => continue,
                    };
                    let _ = tx.send(command);
                }

                let _ = CloseHandle(pipe);
            }
        }
    });

    Ok(handle)
}
