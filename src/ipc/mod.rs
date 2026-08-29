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

    // Handle switch-workspace command with structured input
    if cmd_str == "switch-workspace" {
        if let Some(ws_num) = json.get("workspace").and_then(|v| v.as_u64()) {
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &mut *ptr;
                    let monitor_idx = platform.focused_hwnd
                        .and_then(|hwnd| platform.window_for_hwnd(hwnd.0))
                        .and_then(|info| platform.window_monitors.get(&info.id))
                        .copied()
                        .unwrap_or(0);

                    let ws_count = platform.monitor_workspaces[monitor_idx].grids.len();
                    let target_ws = (ws_num as usize - 1).min(ws_count - 1);

                    if target_ws != platform.monitor_workspaces[monitor_idx].current {
                        platform.switch_workspace(target_ws);
                        return serde_json::json!({
                            "success": true,
                            "command": cmd_str,
                            "message": format!("Switching to workspace {} with animation", ws_num),
                        });
                    } else {
                        return serde_json::json!({
                            "success": false,
                            "command": cmd_str,
                            "message": format!("Already on workspace {}", ws_num),
                        });
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "Invalid workspace number",
        });
    }

    // Handle notify command with structured input
    if cmd_str == "notify" {
        if let Some(message) = json.get("message").and_then(|v| v.as_str()) {
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &mut *ptr;
                    if let Some(ref notifier) = platform.notifier {
                        notifier.show(message);
                        return serde_json::json!({
                            "success": true,
                            "command": cmd_str,
                            "message": "Notification shown",
                        });
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "Notifier not available or missing message",
        });
    }

    // Handle screenshot command with structured input
    if cmd_str == "screenshot" {
        let hwnd_opt = json.get("hwnd").and_then(|v| v.as_u64()).map(|h| HWND(h as *mut _));
        let output = json.get("output").and_then(|v| v.as_str()).unwrap_or("ultrawm-screenshot.png");
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                match platform.take_screenshot(hwnd_opt, output) {
                    Ok(_) => {
                        return serde_json::json!({
                            "success": true,
                            "command": cmd_str,
                            "message": format!("Screenshot saved to {}", output),
                        });
                    }
                    Err(e) => {
                        return serde_json::json!({
                            "success": false,
                            "command": cmd_str,
                            "message": format!("Screenshot failed: {}", e),
                        });
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "Platform not available",
        });
    }

    // Handle add-rule command with structured input
    if cmd_str == "add-rule" {
        if let Some(match_str) = json.get("match").and_then(|v| v.as_str()) {
            let mut rule_json = serde_json::json!({
                "command": "add-rule",
                "match": match_str,
            });
            if let Some(float_val) = json.get("float") {
                rule_json["float"] = float_val.clone();
            }
            if let Some(workspace_val) = json.get("workspace") {
                rule_json["workspace"] = workspace_val.clone();
            }
            if let Some(opacity_val) = json.get("opacity") {
                rule_json["opacity"] = opacity_val.clone();
            }
            if let Some(sticky_val) = json.get("sticky") {
                rule_json["sticky"] = sticky_val.clone();
            }
            if let Some(max_width_val) = json.get("max_width") {
                rule_json["max_width"] = max_width_val.clone();
            }
            if let Some(max_height_val) = json.get("max_height") {
                rule_json["max_height"] = max_height_val.clone();
            }
            if let Some(min_width_val) = json.get("min_width") {
                rule_json["min_width"] = min_width_val.clone();
            }
            if let Some(min_height_val) = json.get("min_height") {
                rule_json["min_height"] = min_height_val.clone();
            }
            if let Some(width_val) = json.get("width") {
                rule_json["width"] = width_val.clone();
            }
            if let Some(height_val) = json.get("height") {
                rule_json["height"] = height_val.clone();
            }
            if let Some(float_x_val) = json.get("float_x") {
                rule_json["float_x"] = float_x_val.clone();
            }
            if let Some(float_y_val) = json.get("float_y") {
                rule_json["float_y"] = float_y_val.clone();
            }
            if let Some(float_w_val) = json.get("float_w") {
                rule_json["float_w"] = float_w_val.clone();
            }
            if let Some(float_h_val) = json.get("float_h") {
                rule_json["float_h"] = float_h_val.clone();
            }
            let rule_str = serde_json::to_string(&rule_json).unwrap_or_default();
            let _ = tx.send(crate::ipc::IpcCommand::Single { command: rule_str });
            return IpcResponse { success: true, message: Some("rule added".into()), data: None };
        }
        return IpcResponse { success: false, message: Some("missing 'match' field".into()), data: None };
    }

    // Handle import-rules: batch import rules from JSON array
    if cmd_str == "import-rules" {
        if let Some(rules_array) = json.get("rules").and_then(|v| v.as_array()) {
            let import_json = serde_json::json!({
                "command": "import-rules",
                "rules": rules_array,
            });
            let import_str = serde_json::to_string(&import_json).unwrap_or_default();
            let _ = tx.send(crate::ipc::IpcCommand::Single { command: import_str });
            return IpcResponse { success: true, message: Some(format!("importing {} rules", rules_array.len())), data: None };
        }
        return IpcResponse { success: false, message: Some("missing 'rules' array field".into()), data: None };
    }

    // Handle layout-preset with name parameter
    if cmd_str == "layout-preset" {
        if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
            let _ = tx.send(crate::ipc::IpcCommand::Single { command: format!("layout-preset:{}", name) });
            return IpcResponse { success: true, message: Some(format!("applying preset '{}'", name)), data: None };
        }
        return IpcResponse { success: false, message: Some("missing 'name' field. Use: {\"command\":\"layout-preset\",\"name\":\"my-preset\"}".into()), data: None };
    }

    // Handle snap-custom with name parameter
    if cmd_str == "snap-custom" {
        if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
            let _ = tx.send(crate::ipc::IpcCommand::Single { command: format!("snap-custom:{}", name) });
            return IpcResponse { success: true, message: Some(format!("applying custom layout '{}'", name)), data: None };
        }
        return IpcResponse { success: false, message: Some("missing 'name' field. Use: {\"command\":\"snap-custom\",\"name\":\"my-layout\"}".into()), data: None };
    }

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
            let (monitor_count, workspace_info, managed_count, theme_name) = unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    let mc = platform.monitors.len();
                    let mut ws_info = Vec::new();
                    for (i, mws) in platform.monitor_workspaces.iter().enumerate() {
                        if i >= mc { break; }
                        ws_info.push(serde_json::json!({
                            "monitor": i,
                            "current": mws.current,
                            "count": mws.grids.len(),
                        }));
                    }
                    let mc2 = platform.windows.len();
                    let tn = platform.theme_mgr
                        .as_ref()
                        .map(|m| m.borrow().current_theme().name.clone())
                        .unwrap_or_default();
                    (mc, ws_info, mc2, tn)
                } else {
                    (0usize, Vec::new(), 0usize, String::new())
                }
            };
            let state = serde_json::json!({
                "status": "running",
                "version": env!("CARGO_PKG_VERSION"),
                "monitors": monitor_count,
                "managed_windows": managed_count,
                "theme": theme_name,
                "workspaces": workspace_info,
            });
            return serde_json::json!({
                "success": true,
                "command": cmd_str,
                "data": state,
            });
        }
        "get-dpi" => {
            let dpi_info = unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    let mut monitors = Vec::new();
                    for (i, m) in platform.monitors.iter().enumerate() {
                        monitors.push(serde_json::json!({
                            "index": i,
                            "dpi": m.dpi,
                            "scale_factor": m.scale_factor,
                            "width": m.width(),
                            "height": m.height(),
                            "left": m.left,
                            "top": m.top,
                            "right": m.right,
                            "bottom": m.bottom,
                        }));
                    }
                    monitors
                } else {
                    Vec::new()
                }
            };
            return serde_json::json!({
                "success": true,
                "command": cmd_str,
                "data": {
                    "monitors": dpi_info,
                },
            });
        }
        "screenshot" => {
            let result = capture_screenshot();
            return serde_json::json!({
                "success": result.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
                "command": cmd_str,
                "data": result,
            });
        }
        "get-config" => {
            let config_data = unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    let monitor_count: usize = platform.monitors.len();
                    let window_count: usize = platform.window_count();
                    serde_json::json!({
                        "layout": {
                            "gaps": platform.config.layout.gaps,
                            "inner_padding": platform.config.layout.inner_padding,
                            "outer_padding": platform.config.layout.outer_padding,
                            "border_width": platform.config.layout.border_width,
                            "corner_radius": platform.config.layout.corner_radius,
                            "rounded_corners": platform.config.layout.rounded_corners,
                            "dwm_shadows": platform.config.layout.dwm_shadows,
                            "window_opacity": platform.config.layout.window_opacity,
                            "spring_stiffness": platform.config.layout.spring_stiffness,
                            "spring_damping": platform.config.layout.spring_damping,
                            "workspace_count": platform.config.layout.workspace_count,
                            "default_float_width": platform.config.layout.default_float_width,
                            "default_float_height": platform.config.layout.default_float_height,
                            "center_focused": platform.config.layout.center_focused,
                            "focus_follows_mouse": platform.config.layout.focus_follows_mouse,
                            "snap_grid_size": platform.config.layout.snap_grid_size,
                            "snap_edge_distance": platform.config.layout.snap_edge_distance,
                        },
                        "bar": {
                            "enabled": platform.config.bar.enabled,
                            "height": platform.config.bar.height,
                            "position": platform.config.bar.position,
                            "transparency": platform.config.bar.transparency,
                            "show_workspaces": platform.config.bar.show_workspaces,
                            "show_clock": platform.config.bar.show_clock,
                            "show_volume": platform.config.bar.show_volume,
                            "show_battery": platform.config.bar.show_battery,
                        },
                        "theme": {
                            "default": platform.config.theme.default,
                        },
                        "launcher": {
                            "enabled": platform.config.launcher.enabled,
                            "fuzzy_search": platform.config.launcher.fuzzy_search,
                            "show_recent": platform.config.launcher.show_recent,
                        },
                        "monitors": {
                            "count": monitor_count,
                            "dpi": platform.monitors.iter().map(|m| m.dpi).collect::<Vec<_>>(),
                            "scale_factors": platform.monitors.iter().map(|m| m.scale_factor).collect::<Vec<_>>(),
                        },
                        "windows": window_count,
                    })
                } else {
                    serde_json::json!({ "error": "platform not available" })
                }
            };
            return serde_json::json!({
                "success": true,
                "command": cmd_str,
                "data": config_data,
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
                    let ws_names = &platform.config.layout.workspace_names;
                    let names: Vec<String> = if ws_names.is_empty() {
                        (1..=ws_count).map(|i| i.to_string()).collect()
                    } else {
                        ws_names.iter().take(ws_count).cloned().collect()
                    };
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
        "get-managed-windows" => {
            let mut managed = Vec::new();
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    for (hwnd_wrapper, info) in &platform.windows {
                        let ws = platform.window_workspaces.get(&info.id).copied().unwrap_or(0);
                        managed.push(serde_json::json!({
                            "id": info.id,
                            "hwnd": hwnd_wrapper.0 .0 as usize,
                            "title": info.title,
                            "exe": info.exe,
                            "workspace": ws,
                            "floating": info.floating,
                            "sticky": info.sticky,
                            "maximized": info.maximized,
                            "minimized": info.minimized,
                            "always_on_top": info.always_on_top,
                            "opacity": info.opacity,
                            "visible": info.visible,
                        }));
                    }
                }
            }
            return serde_json::json!({
                "success": true,
                "command": cmd_str,
                "data": serde_json::Value::Array(managed),
            });
        }
        "list-rules" => {
            let rules = unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    let rules: Vec<serde_json::Value> = platform.config.rules.iter().map(|r| {
                        serde_json::json!({
                            "match": r.match_,
                            "float": r.float,
                            "workspace": r.workspace,
                            "width": r.width,
                            "height": r.height,
                            "max_width": r.max_width,
                            "max_height": r.max_height,
                            "min_width": r.min_width,
                            "min_height": r.min_height,
                            "opacity": r.opacity,
                            "sticky": r.sticky,
                        })
                    }).collect();
                    rules
                } else {
                    Vec::new()
                }
            };
            return serde_json::json!({
                "success": true,
                "command": cmd_str,
                "data": serde_json::Value::Array(rules),
            });
        }
        "export-rules" => {
            let rules = unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    let rules: Vec<serde_json::Value> = platform.config.rules.iter().map(|r| {
                        serde_json::json!({
                            "match": r.match_,
                            "float": r.float,
                            "workspace": r.workspace,
                            "width": r.width,
                            "height": r.height,
                            "max_width": r.max_width,
                            "max_height": r.max_height,
                            "min_width": r.min_width,
                            "min_height": r.min_height,
                            "float_x": r.float_x,
                            "float_y": r.float_y,
                            "float_w": r.float_w,
                            "float_h": r.float_h,
                            "opacity": r.opacity,
                            "sticky": r.sticky,
                        })
                    }).collect();
                    rules
                } else {
                    Vec::new()
                }
            };
            return serde_json::json!({
                "success": true,
                "command": cmd_str,
                "data": serde_json::Value::Array(rules),
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
        "maximize" => IpcCommand::Single { command: "maximize".into() },
        "always-on-top" => IpcCommand::Single { command: "always-on-top".into() },
        "grow-gap" => IpcCommand::Single { command: "grow-gap".into() },
        "shrink-gap" => IpcCommand::Single { command: "shrink-gap".into() },
        "unfloat-all" => IpcCommand::Single { command: "unfloat-all".into() },
        "list-rules" => IpcCommand::Single { command: "list-rules".into() },
        "export-rules" => IpcCommand::Single { command: "export-rules".into() },
        "import-rules" => IpcCommand::Single { command: "import-rules".into() },
        "reload-config" => IpcCommand::Single { command: "reload-config".into() },
        "set-wallpaper" => IpcCommand::Single { command: "set-wallpaper".into() },
        "set-wallpaper-image" => IpcCommand::Single { command: "set-wallpaper-image".into() },
        "set-gap" => IpcCommand::Single { command: "set-gap".into() },
        "set-corner-radius" => IpcCommand::Single { command: "set-corner-radius".into() },
        "set-border-width" => IpcCommand::Single { command: "set-border-width".into() },
        "idle-inhibit" => IpcCommand::Single { command: "idle-inhibit".into() },
        "idle-noinhibit" => IpcCommand::Single { command: "idle-noinhibit".into() },
        "notify" => IpcCommand::Single { command: "notify".into() },
        "get-managed-windows" => IpcCommand::Single { command: "get-managed-windows".into() },
        "clamp-focused" => IpcCommand::Single { command: "clamp-focused".into() },
        "minimize-to-tray" => IpcCommand::Single { command: "minimize-to-tray".into() },
        "restore-from-tray" => IpcCommand::Single { command: "restore-from-tray".into() },
        "restore-all-tray" => IpcCommand::Single { command: "restore-all-tray".into() },
        "screenshot" => IpcCommand::Single { command: "screenshot".into() },
        "window-search" => IpcCommand::Single { command: "window-search".into() },
        "snap-layout" => IpcCommand::Single { command: "snap-layout".into() },
        "snap-custom" => {
            return serde_json::json!({
                "success": false,
                "command": cmd_str,
                "error": "snap-custom requires a name. Use: {\"command\":\"snap-custom\",\"name\":\"my-layout\"}",
            });
        }
        "layout-columns" => IpcCommand::Single { command: "layout-columns".into() },
        "layout-rows" => IpcCommand::Single { command: "layout-rows".into() },
        "layout-master" => IpcCommand::Single { command: "layout-master".into() },
        "layout-fibonacci" => IpcCommand::Single { command: "layout-fibonacci".into() },
        "layout-preset" => {
            // layout-preset requires a name parameter; handled via channel with name appended
            return serde_json::json!({
                "success": false,
                "command": cmd_str,
                "error": "layout-preset requires a name parameter. Use: {\"command\":\"layout-preset\",\"name\":\"my-preset\"}",
            });
        }
        "set-workspace-count" => IpcCommand::Single { command: "set-workspace-count".into() },
        "set-window-opacity" => IpcCommand::Single { command: "set-window-opacity".into() },
        "increase-opacity" => IpcCommand::Single { command: "increase-opacity".into() },
        "decrease-opacity" => IpcCommand::Single { command: "decrease-opacity".into() },
        "add-scratchpad" => IpcCommand::Single { command: "add-scratchpad".into() },
        "remove-scratchpad" => IpcCommand::Single { command: "remove-scratchpad".into() },
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

fn capture_screenshot() -> serde_json::Value {
    unsafe {
        use windows::Win32::{
            Foundation::*,
            Graphics::Gdi::*,
            UI::WindowsAndMessaging::*,
        };

        let ptr = crate::platform::keyboard::PLATFORM_PTR;
        if ptr.is_null() {
            return serde_json::json!({
                "success": false,
                "message": "platform not available",
                "path": null,
            });
        }

        let platform = &*ptr;
        let hwnd = match platform.focused_hwnd {
            Some(h) => h.0,
            None => {
                return serde_json::json!({
                    "success": false,
                    "message": "no focused window",
                    "path": null,
                });
            }
        };

        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return serde_json::json!({
                "success": false,
                "message": "failed to get window rect",
                "path": null,
            });
        }

        let w = (rect.right - rect.left).max(1);
        let h = (rect.bottom - rect.top).max(1);

        let hdc_screen = GetDC(HWND(std::ptr::null_mut()));
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbm = CreateCompatibleBitmap(hdc_screen, w, h);
        let old_bmp = SelectObject(hdc_mem, hbm);

        // Capture from screen at window position
        let _ = BitBlt(hdc_mem, 0, 0, w, h, hdc_screen, rect.left, rect.top, SRCCOPY);

        // Extract pixel data
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0u32,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default(); 1],
        };

        let mut pixels = vec![0u8; (w * h * 4) as usize];
        let _ = GetDIBits(hdc_mem, hbm, 0, h as u32, Some(pixels.as_mut_ptr() as *mut _), &mut bmi, DIB_RGB_COLORS);

        // Convert BGRA to RGBA
        for px in pixels.chunks_exact_mut(4) {
            px.swap(0, 2);
        }

        // Save PNG
        let screenshots_dir = match dirs::picture_dir() {
            Some(d) => d.join("UltraWM"),
            None => std::path::PathBuf::from(".").join("UltraWM"),
        };
        let _ = std::fs::create_dir_all(&screenshots_dir);

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let path = screenshots_dir.join(format!("screenshot_{}.png", timestamp));

        if let Ok(file) = std::fs::File::create(&path) {
            let mut encoder = png::Encoder::new(file, w as u32, h as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            if let Ok(mut writer) = encoder.write_header() {
                let _ = writer.write_image_data(&pixels);
            }
        }

        // Cleanup
        SelectObject(hdc_mem, old_bmp);
        DeleteObject(hbm);
        DeleteDC(hdc_mem);
        ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);

        let path_str = path.to_string_lossy().to_string();
        serde_json::json!({
            "success": true,
            "message": format!("screenshot saved: {}", path_str),
            "path": path_str,
        })
    }
}
