use std::sync::mpsc;
use std::thread;
use std::io::{Read, Write};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Storage::FileSystem::*,
        System::Pipes::*,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "kebab-case")]
pub enum IpcCommand {
    // Theme
    NextTheme,
    PrevTheme,
    // Focus
    FocusNext,
    FocusPrev,
    FocusLeft,
    FocusRight,
    FocusUp,
    FocusDown,
    // Pan
    PanLeft,
    PanRight,
    PanUp,
    PanDown,
    // Resize
    GrowWidth,
    ShrinkWidth,
    GrowHeight,
    ShrinkHeight,
    // Actions
    Close,
    Float,
    Unfloat,
    SplitHorizontal,
    SplitVertical,
    Unsplit,
    ToggleLauncher,
    ToggleOverview,
    ToggleScratchpad,
    ToggleFullscreen,
    Quit,
    // Queries (with response)
    GetState,
    ListThemes,
    GetWindows,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
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
                    PIPE_ACCESS_DUPLEX,
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

                let response = if result.is_ok() && bytes_read > 0 {
                    let input = String::from_utf8_lossy(&buf[..bytes_read as usize]).trim().to_string();
                    log::debug!("IPC received: {}", input);
                    handle_ipc_message(&input, &tx)
                } else {
                    IpcResponse { success: false, message: Some("failed to read".into()), data: None }
                };

                // Write response back
                let resp_str = serde_json::to_string(&response).unwrap_or_default();
                let resp_bytes = resp_str.as_bytes();
                let _ = WriteFile(pipe, resp_bytes, None);

                let _ = CloseHandle(pipe);
            }
        }
    });

    Ok(handle)
}

fn handle_ipc_message(input: &str, tx: &mpsc::Sender<IpcCommand>) -> IpcResponse {
    // Try JSON first
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(input) {
        return handle_json_command(json, tx);
    }

    // Fallback to plain text command (backward compat)
    let cmd = match input {
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
        _ => return IpcResponse { success: false, message: Some(format!("unknown command: {}", input)), data: None },
    };

    let _ = tx.send(cmd);
    IpcResponse { success: true, message: Some("ok".into()), data: None }
}

fn handle_json_command(json: serde_json::Value, tx: &mpsc::Sender<IpcCommand>) -> IpcResponse {
    let cmd_str = json.get("command").and_then(|v| v.as_str()).unwrap_or("");

    // Handle query commands directly in IPC thread
    match cmd_str {
        "list-themes" => {
            let themes = match crate::theme::ThemeManager::list_themes() {
                Ok(t) => t,
                Err(_) => vec![],
            };
            return IpcResponse {
                success: true,
                message: Some(format!("{} themes", themes.len())),
                data: Some(Value::Array(themes.into_iter().map(|t| Value::String(t)).collect())),
            };
        }
        "get-state" => {
            let state = serde_json::json!({
                "status": "running",
                "version": env!("CARGO_PKG_VERSION"),
            });
            return IpcResponse { success: true, message: Some("running".into()), data: Some(state) };
        }
        _ => {}
    }

    let cmd = match cmd_str {
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
        "split-horizontal" => IpcCommand::SplitHorizontal,
        "split-vertical" => IpcCommand::SplitVertical,
        "unsplit" => IpcCommand::Unsplit,
        "launcher" => IpcCommand::ToggleLauncher,
        "overview" => IpcCommand::ToggleOverview,
        "scratchpad" => IpcCommand::ToggleScratchpad,
        "fullscreen" => IpcCommand::ToggleFullscreen,
        "quit" => IpcCommand::Quit,
        _ => return IpcResponse { success: false, message: Some(format!("unknown command: {}", cmd_str)), data: None },
    };

    let _ = tx.send(cmd);
    IpcResponse { success: true, message: Some("ok".into()), data: None }
}
