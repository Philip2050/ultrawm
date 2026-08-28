use std::sync::mpsc;
use std::thread;
use std::io::Read;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Storage::FileSystem::*,
        System::Pipes::*,
        UI::WindowsAndMessaging::*,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IpcCommand {
    Single { command: String },
    Batch { commands: Vec<String> },
}

impl PartialEq for IpcCommand {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Single { command: a }, Self::Single { command: b }) => a == b,
            (Self::Batch { commands: a }, Self::Batch { commands: b }) => a == b,
            _ => false,
        }
    }
}
impl Eq for IpcCommand {}

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
                let _ = WriteFile(pipe, Some(resp_bytes), None, None);

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
    let cmd = IpcCommand::Single { command: input.into() };
    let _ = tx.send(cmd);
    IpcResponse { success: true, message: Some("ok".into()), data: None }
}

fn handle_json_command(json: serde_json::Value, tx: &mpsc::Sender<IpcCommand>) -> IpcResponse {
    // Handle batch commands
    if let Some(commands) = json.get("commands").and_then(|v| v.as_array()) {
        let mut results = Vec::new();
        for cmd_val in commands {
            let cmd_str = cmd_val.as_str().unwrap_or("");
            let result = process_single_command(cmd_str, tx);
            results.push(result);
        }
        return IpcResponse {
            success: true,
            message: Some(format!("batch: {} commands", results.len())),
            data: Some(Value::Array(results)),
        };
    }

    let cmd_str = json.get("command").and_then(|v| v.as_str()).unwrap_or("");
    let result = process_single_command(cmd_str, tx);
    if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
        IpcResponse { success: true, message: Some("ok".into()), data: None }
    } else {
        let err_msg = result.get("error").and_then(|v| v.as_str()).unwrap_or("unknown error").to_string();
        IpcResponse { success: false, message: Some(err_msg), data: None }
    }
}

fn process_single_command(cmd_str: &str, tx: &mpsc::Sender<IpcCommand>) -> serde_json::Value {
    // Handle query commands directly in IPC thread
    match cmd_str {
        "list-themes" => {
            let themes = match crate::theme::ThemeManager::list_themes() {
                Ok(t) => t,
                Err(_) => vec![],
            };
            return serde_json::json!({
                "success": true,
                "command": cmd_str,
                "data": themes,
            });
        }
        "get-state" => {
            let state = serde_json::json!({
                "status": "running",
                "version": env!("CARGO_PKG_VERSION"),
            });
            return serde_json::json!({
                "success": true,
                "command": cmd_str,
                "data": state,
            });
        }
        "get-windows" => {
            let mut windows = Vec::new();
            unsafe {
                let _ = EnumWindows(Some(enum_windows_proc), LPARAM(&mut windows as *mut Vec<_> as isize));
            }
            return serde_json::json!({
                "success": true,
                "command": cmd_str,
                "data": serde_json::Value::Array(
                    windows.into_iter().map(|(hwnd, title): (HWND, String)| {
                        serde_json::json!({
                            "hwnd": hwnd.0 as usize,
                            "title": title,
                        })
                    }).collect(),
                ),
            });
        }
        "get-workspaces" => {
            let (count, names) = unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    let ws_count = platform.monitor_workspaces[0].grids.len();
                    let names: Vec<String> = (1..=ws_count).map(|i| i.to_string()).collect();
                    (ws_count, names)
                } else {
                    (4, vec!["1".into(), "2".into(), "3".into(), "4".into()])
                }
            };
            return serde_json::json!({
                "success": true,
                "command": cmd_str,
                "data": serde_json::json!({
                    "count": count,
                    "names": names,
                }),
            });
        }
        _ => {}
    }

    let cmd = match cmd_str {
        "next-theme" => IpcCommand::Single { command: "next-theme".into() },
        "prev-theme" => IpcCommand::Single { command: "prev-theme".into() },
        "focus-next" => IpcCommand::Single { command: "focus-next".into() },
        "focus-prev" => IpcCommand::Single { command: "focus-prev".into() },
        "focus-left" => IpcCommand::Single { command: "focus-left".into() },
        "focus-right" => IpcCommand::Single { command: "focus-right".into() },
        "focus-up" => IpcCommand::Single { command: "focus-up".into() },
        "focus-down" => IpcCommand::Single { command: "focus-down".into() },
        "pan-left" => IpcCommand::Single { command: "pan-left".into() },
        "pan-right" => IpcCommand::Single { command: "pan-right".into() },
        "pan-up" => IpcCommand::Single { command: "pan-up".into() },
        "pan-down" => IpcCommand::Single { command: "pan-down".into() },
        "grow-width" => IpcCommand::Single { command: "grow-width".into() },
        "shrink-width" => IpcCommand::Single { command: "shrink-width".into() },
        "grow-height" => IpcCommand::Single { command: "grow-height".into() },
        "shrink-height" => IpcCommand::Single { command: "shrink-height".into() },
        "close" => IpcCommand::Single { command: "close".into() },
        "float" => IpcCommand::Single { command: "float".into() },
        "unfloat" => IpcCommand::Single { command: "unfloat".into() },
        "split-horizontal" => IpcCommand::Single { command: "split-horizontal".into() },
        "split-vertical" => IpcCommand::Single { command: "split-vertical".into() },
        "unsplit" => IpcCommand::Single { command: "unsplit".into() },
        "launcher" => IpcCommand::Single { command: "launcher".into() },
        "overview" => IpcCommand::Single { command: "overview".into() },
        "scratchpad" => IpcCommand::Single { command: "scratchpad".into() },
        "fullscreen" => IpcCommand::Single { command: "fullscreen".into() },
        "tab" => IpcCommand::Single { command: "tab".into() },
        "untab" => IpcCommand::Single { command: "untab".into() },
        "sticky" => IpcCommand::Single { command: "sticky".into() },
        "minimize" => IpcCommand::Single { command: "minimize".into() },
        "restore" => IpcCommand::Single { command: "restore".into() },
        "quit" => IpcCommand::Single { command: "quit".into() },
        _ => {
            return serde_json::json!({
                "success": false,
                "command": cmd_str,
                "error": format!("unknown command: {}", cmd_str),
            });
        }
    };

    let _ = tx.send(cmd);
    serde_json::json!({
        "success": true,
        "command": cmd_str,
    })
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if !IsWindowVisible(hwnd).as_bool() {
        return TRUE;
    }

    let windows = &mut *(lparam.0 as *mut Vec<(HWND, String)>);
    let len = GetWindowTextLengthW(hwnd);
    if len > 0 {
        let mut buf = vec![0u16; (len + 1) as usize];
        let _ = GetWindowTextW(hwnd, &mut buf);
        let title = String::from_utf16_lossy(&buf[..len as usize]);
        windows.push((hwnd, title));
    }

    TRUE
}
