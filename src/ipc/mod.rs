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
                    let monitor_idx = json.get("monitor")
                        .and_then(|v| v.as_u64())
                        .map(|m| m as usize)
                        .unwrap_or_else(|| {
                            platform.focused_hwnd
                                .and_then(|hwnd| platform.window_for_hwnd(hwnd.0))
                                .and_then(|info| platform.window_monitors.get(&info.id))
                                .copied()
                                .unwrap_or(0)
                        });

                    if monitor_idx >= platform.monitor_workspaces.len() {
                        return serde_json::json!({
                            "success": false,
                            "command": cmd_str,
                            "message": format!("Invalid monitor index: {} ({} monitors)", monitor_idx, platform.monitor_workspaces.len()),
                        });
                    }

                    let ws_count = platform.monitor_workspaces[monitor_idx].grids.len();
                    let target_ws = (ws_num as usize - 1).min(ws_count - 1);

                    if target_ws != platform.monitor_workspaces[monitor_idx].current {
                        platform.switch_workspace(target_ws);
                        return serde_json::json!({
                            "success": true,
                            "command": cmd_str,
                            "message": format!("Switching monitor {} to workspace {} with animation", monitor_idx + 1, ws_num),
                        });
                    } else {
                        return serde_json::json!({
                            "success": false,
                            "command": cmd_str,
                            "message": format!("Monitor {} already on workspace {}", monitor_idx + 1, ws_num),
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

    // Handle set-app-opacity command
    if cmd_str == "set-app-opacity" {
        if let Some(exe) = json.get("exe").and_then(|v| v.as_str()) {
            if let Some(opacity_val) = json.get("opacity").and_then(|v| v.as_f64()) {
                let opacity = opacity_val.clamp(0.0, 1.0) as f32;
                unsafe {
                    let ptr = crate::platform::keyboard::PLATFORM_PTR;
                    if !ptr.is_null() {
                        let platform = &mut *ptr;
                        platform.remember_app_opacity(exe, opacity);
                        // Apply to all matching windows
                        for (hwnd_wrapper, info) in &platform.windows {
                            if info.exe == exe {
                                platform.apply_window_opacity(hwnd_wrapper.0, opacity);
                                if let Some(win_info) = platform.windows.get_mut(&hwnd_wrapper) {
                                    win_info.opacity = Some(opacity);
                                }
                            }
                        }
                        platform.save_per_app_opacity();
                        return serde_json::json!({
                            "success": true,
                            "command": cmd_str,
                            "message": format!("Set opacity {:.0}% for {}", (opacity * 100.0) as i32, exe),
                        });
                    }
                }
            }
            return serde_json::json!({
                "success": false,
                "command": cmd_str,
                "message": "missing 'opacity' field (0.0-1.0)",
            });
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "missing 'exe' field",
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

    // Handle get-window-rules command
    if cmd_str == "get-window-rules" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let mut matching_rules = Vec::new();

                if let Some(hwnd) = platform.focused_hwnd {
                    if let Some(info) = platform.windows.get(&hwnd) {
                        for rule in &platform.config.rules {
                            if rule.match_exe(&info.exe) || rule.match_class(&info.class) || rule.match_title(&info.title) {
                                matching_rules.push(serde_json::json!({
                                    "match": rule.match_,
                                    "float": rule.float,
                                    "workspace": rule.workspace,
                                    "opacity": rule.opacity,
                                    "sticky": rule.sticky,
                                    "max_width": rule.max_width,
                                    "max_height": rule.max_height,
                                    "min_width": rule.min_width,
                                    "min_height": rule.min_height,
                                }));
                            }
                        }
                    }
                }

                return serde_json::json!({
                    "success": true,
                    "command": cmd_str,
                    "data": serde_json::json!({
                        "rules": matching_rules,
                        "count": matching_rules.len(),
                    }),
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "Platform not available",
        });
    }

    // Handle list-workspaces command
    if cmd_str == "list-workspaces" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let monitor_filter = json.get("monitor")
                    .and_then(|v| v.as_u64())
                    .map(|m| m as usize);
                let mut workspaces = Vec::new();
                for (mi, mw) in platform.monitor_workspaces.iter().enumerate() {
                    if let Some(filt) = monitor_filter {
                        if mi != filt { continue; }
                    }
                    for (wi, grid) in mw.grids.iter().enumerate() {
                        let ws_name = platform.workspace_names.get(mi)
                            .and_then(|names| names.get(wi))
                            .map(|s| s.clone())
                            .unwrap_or_else(|| format!("{}", wi + 1));
                        let win_count = grid.windows.len();
                        workspaces.push(serde_json::json!({
                            "monitor": mi,
                            "index": wi,
                            "name": ws_name,
                            "windows": win_count,
                            "active": mi == mw.current_monitor && wi == mw.current,
                        }));
                    }
                }
                let focused_monitor = platform.monitor_workspaces.iter()
                    .position(|mw| mw.current_monitor == mw.current_monitor).unwrap_or(0);
                return serde_json::json!({
                    "success": true,
                    "command": "list-workspaces",
                    "focused_monitor": focused_monitor,
                    "focused_workspace": platform.monitor_workspaces.get(focused_monitor).map(|mw| mw.current).unwrap_or(0),
                    "workspaces": workspaces,
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "Platform not available",
        });
    }

    // Handle get-config command
    if cmd_str == "get-config" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let config_value = serde_json::to_value(&platform.config).unwrap_or_else(|_| serde_json::json!({}));
                return serde_json::json!({
                    "success": true,
                    "command": "get-config",
                    "data": config_value,
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "Platform not available",
        });
    }

    // Handle get-stats command
    if cmd_str == "get-stats" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let total_windows = platform.windows.len();
                let mut tiling_count = 0;
                let mut floating_count = 0;
                for (_, info) in &platform.windows {
                    if info.floating { floating_count += 1; }
                    else { tiling_count += 1; }
                }
                let monitor_count = platform.monitors.len();
                let mut total_workspaces = 0;
                for mw in &platform.monitor_workspaces {
                    total_workspaces += mw.grids.len();
                }
                return serde_json::json!({
                    "success": true,
                    "command": "get-stats",
                    "data": {
                        "total_windows": total_windows,
                        "tiling_windows": tiling_count,
                        "floating_windows": floating_count,
                        "monitors": monitor_count,
                        "total_workspaces": total_workspaces,
                        "focused_hwnd": platform.focused_hwnd.map(|h| h.0),
                        "monocle": platform.monocle,
                        "snap_mode": platform.snap_mode,
                        "overview": platform.overview,
                        "scratchpads": platform.scratchpad.as_ref().map(|s| s.windows.len()).unwrap_or(0),
                        "rules_count": platform.config.rules.len(),
                    },
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "Platform not available",
        });
    }

    // Handle list-monitors command
    if cmd_str == "list-monitors" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let mut monitors = Vec::new();
                for (i, mon) in platform.monitors.iter().enumerate() {
                    let mws = platform.monitor_workspaces.get(i);
                    let current_ws = mws.map(|m| m.current).unwrap_or(0);
                    let ws_count = mws.map(|m| m.grids.len()).unwrap_or(0);
                    monitors.push(serde_json::json!({
                        "index": i,
                        "width": mon.width(),
                        "height": mon.height(),
                        "work_width": mon.work_width(),
                        "work_height": mon.work_height(),
                        "work_left": mon.work_left,
                        "work_top": mon.work_top,
                        "dpi": mon.dpi,
                        "primary": i == 0,
                        "current_workspace": current_ws,
                        "workspace_count": ws_count,
                    }));
                }
                return serde_json::json!({
                    "success": true,
                    "command": "list-monitors",
                    "monitors": monitors,
                    "count": monitors.len(),
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "Platform not available",
        });
    }

    // Handle get-bar-config command
    if cmd_str == "get-bar-config" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let bar_value = serde_json::to_value(&platform.config.bar).unwrap_or_else(|_| serde_json::json!({}));
                let bar_state = platform.bar.as_ref().map(|b| {
                    serde_json::json!({
                        "visible": platform.bar_visible,
                        "height": b.height,
                    })
                }).unwrap_or_else(|| serde_json::json!({
                    "visible": false,
                    "height": 0,
                }));
                return serde_json::json!({
                    "success": true,
                    "command": "get-bar-config",
                    "config": bar_value,
                    "state": bar_state,
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "Platform not available",
        });
    }

    // Handle get-theme command
    if cmd_str == "get-theme" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let theme_name = platform.config.theme.default.clone();
                let current_colors = platform.theme_mgr.as_ref().map(|tm| tm.current_colors()).unwrap_or_else(|| serde_json::json!({}));
                return serde_json::json!({
                    "success": true,
                    "command": "get-theme",
                    "data": {
                        "name": theme_name,
                        "colors": current_colors,
                    },
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "Platform not available",
        });
    }

    // Handle get-window-info command
    if cmd_str == "get-window-info" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                if let Some(hwnd) = platform.focused_hwnd {
                    if let Some(info) = platform.windows.get(&hwnd) {
                        let mon_idx = platform.window_monitors.get(&info.id).copied().unwrap_or(0);
                        let ws_idx = platform.window_workspaces.get(&info.id).copied().unwrap_or(0);
                        let bw = platform.window_border_widths.get(&info.id).copied();
                        let opacity = info.opacity.unwrap_or(1.0);
                        let border_color = format!("0x{:08X}", info.border_color);
                        return serde_json::json!({
                            "success": true,
                            "command": "get-window-info",
                            "data": {
                                "hwnd": hwnd.0,
                                "id": info.id,
                                "title": info.title,
                                "class": info.class,
                                "exe": info.exe,
                                "visible": info.visible,
                                "floating": info.floating,
                                "fullscreen": info.fullscreen,
                                "always_on_top": info.always_on_top,
                                "minimized": info.minimized,
                                "maximized": info.maximized,
                                "sticky": info.sticky,
                                "monitor": mon_idx,
                                "workspace": ws_idx,
                                "border_color": border_color,
                                "border_width": bw.unwrap_or(platform.config.layout.border_width),
                                "opacity": opacity,
                                "z_order": info.z_order,
                            },
                        });
                    }
                }
                return serde_json::json!({
                    "success": false,
                    "command": cmd_str,
                    "message": "No focused window",
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "Platform not available",
        });
    }

    // Handle list-themes command
    if cmd_str == "list-themes" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                if let Some(theme_mgr) = &platform.theme_mgr {
                    let themes: Vec<String> = theme_mgr.theme_names();
                    let current = theme_mgr.current_name().to_string();
                    return serde_json::json!({
                        "success": true,
                        "command": "list-themes",
                        "current": current,
                        "themes": themes,
                        "count": themes.len(),
                    });
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "Platform not available",
        });
    }

    // Handle get-layout-presets command
    if cmd_str == "get-layout-presets" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let presets: Vec<serde_json::Value> = platform.config.layout.layout_presets.iter().map(|p| {
                    serde_json::json!({
                        "name": p.name,
                        "kind": p.kind,
                        "cols": p.cols,
                        "rows": p.rows,
                    })
                }).collect();
                return serde_json::json!({
                    "success": true,
                    "command": "get-layout-presets",
                    "presets": presets,
                    "count": presets.len(),
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "Platform not available",
        });
    }

    // Handle set-workspace-name command
    if cmd_str == "set-workspace-name" {
        if let Some(name_val) = json.get("name").and_then(|v| v.as_str()) {
            if let Some(ws_val) = json.get("workspace").and_then(|v| v.as_u64()) {
                let ws_idx = ws_val as usize;
                unsafe {
                    let ptr = crate::platform::keyboard::PLATFORM_PTR;
                    if !ptr.is_null() {
                        let platform = &mut *ptr;
                        let mon_idx = json.get("monitor")
                            .and_then(|v| v.as_u64())
                            .map(|m| m as usize)
                            .unwrap_or_else(|| {
                                platform.focused_hwnd
                                    .and_then(|hwnd| platform.window_for_hwnd(hwnd.0))
                                    .and_then(|info| platform.window_monitors.get(&info.id))
                                    .copied()
                                    .unwrap_or(0)
                            });
                        if mon_idx < platform.workspace_names.len() {
                            let names = &mut platform.workspace_names[mon_idx];
                            if ws_idx < names.len() {
                                names[ws_idx] = name_val.to_string();
                                platform.refresh_bar_workspaces(mon_idx);
                                return serde_json::json!({
                                    "success": true,
                                    "command": "set-workspace-name",
                                    "message": format!("Renamed monitor {} workspace {} to '{}'", mon_idx + 1, ws_idx + 1, name_val),
                                });
                            }
                        }
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "Usage: {\"command\":\"set-workspace-name\",\"workspace\":1,\"name\":\"Web\",\"monitor\":0}",
        });
    }

    // Handle set-wallpaper-image-monitor command
    if cmd_str == "set-wallpaper-image-monitor" {
        let path = params.get("path").and_then(|v| v.as_str());
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);
        if let Some(p) = path {
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    let monitors = crate::platform::monitor::get_monitors();

                    // Store the wallpaper path for the monitor
                    let target_mon = monitor_idx.unwrap_or(0);
                    if target_mon < monitors.len() {
                        if std::path::Path::new(p).exists() {
                            platform.wallpapers[target_mon] = Some(p.to_string());

                            // Apply the wallpaper
                            match crate::platform::wallpaper::apply_wallpaper_image_monitor(p) {
                                Ok(_) => {
                                    return serde_json::json!({
                                        "success": true,
                                        "monitor": target_mon,
                                        "path": p,
                                    });
                                }
                                Err(e) => {
                                    return serde_json::json!({
                                        "success": false,
                                        "error": format!("Failed to apply wallpaper: {}", e),
                                    });
                                }
                            }
                        } else {
                            return serde_json::json!({
                                "success": false,
                                "error": format!("Wallpaper file not found: {}", p),
                            });
                        }
                    } else {
                        return serde_json::json!({
                            "success": false,
                            "error": format!("monitor {} out of range", target_mon),
                        });
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "missing 'path' parameter",
        });
    }

    // Handle set-wallpaper-monitor command
    if cmd_str == "set-wallpaper-monitor" {
        let monitor_idx = json.get("monitor").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        if let Some(color) = json.get("color").and_then(|v| v.as_str()) {
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    if monitor_idx < platform.monitors.len() {
                        let mon = &platform.monitors[monitor_idx];
                        if let Err(e) = crate::platform::wallpaper::apply_wallpaper_monitor(color, monitor_idx, mon.width(), mon.height()) {
                            return serde_json::json!({
                                "success": false,
                                "command": cmd_str,
                                "message": format!("Wallpaper failed: {}", e),
                            });
                        }
                        return serde_json::json!({
                            "success": true,
                            "command": cmd_str,
                            "message": format!("Wallpaper set for monitor {}", monitor_idx + 1),
                        });
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "Usage: {\"command\":\"set-wallpaper-monitor\",\"color\":\"#1E1E2E\",\"monitor\":0}",
        });
    }

    // Handle set-bar-position command
    if cmd_str == "set-bar-position" {
        if let Some(position) = json.get("position").and_then(|v| v.as_str()) {
            if position == "top" || position == "bottom" {
                unsafe {
                    let ptr = crate::platform::keyboard::PLATFORM_PTR;
                    if !ptr.is_null() {
                        let platform = &mut *ptr;
                        platform.config.bar.position = position.to_string();
                        if let Some(ref bar) = platform.bar {
                            let mon = platform.primary_monitor().copied();
                            if let Some(mon) = mon {
                                let bar_height = bar.height;
                                let bar_width = 9999;
                                if position == "bottom" {
                                    let _ = SetWindowPos(
                                        bar.hwnd,
                                        HWND_TOPMOST,
                                        0,
                                        mon.height() - bar_height,
                                        bar_width,
                                        bar_height,
                                        SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                                    );
                                } else {
                                    let _ = SetWindowPos(
                                        bar.hwnd,
                                        HWND_TOPMOST,
                                        0,
                                        0,
                                        bar_width,
                                        bar_height,
                                        SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                                    );
                                }
                            }
                            let _ = platform.config.save();
                            return serde_json::json!({
                                "success": true,
                                "command": cmd_str,
                                "message": format!("Bar position set to {}", position),
                            });
                        }
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "Usage: {\"command\":\"set-bar-position\",\"position\":\"top\"}",
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

    // Handle create-layout-preset: save current layout as named preset
    if cmd_str == "create-layout-preset" {
        if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
            let kind = json.get("kind").and_then(|v| v.as_str()).unwrap_or("grid");
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &mut *ptr;
                    platform.save_layout_preset(name, kind);
                    return serde_json::json!({
                        "success": true,
                        "command": cmd_str,
                        "message": format!("Layout preset '{}' created ({})", name, kind),
                    });
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "command": cmd_str,
            "message": "missing 'name' field. Use: {\"command\":\"create-layout-preset\",\"name\":\"my-preset\",\"kind\":\"columns\"}",
        });
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

    // Handle save-snap-layout: save current grid as named layout
    if cmd_str == "save-snap-layout" {
        if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &mut *ptr;
                    platform.save_snap_layout(name);
                    return IpcResponse { success: true, message: Some(format!("Snap layout '{}' saved", name)), data: None };
                }
            }
        }
        return IpcResponse { success: false, message: Some("missing 'name' field. Use: {\"command\":\"save-snap-layout\",\"name\":\"my-layout\"}".into()), data: None };
    }

    // Handle list-snap-layouts
    if cmd_str == "list-snap-layouts" {
        let layouts = unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                platform.config.layout.snap_layouts.iter().map(|l| {
                    serde_json::json!({
                        "name": l.name,
                        "widths": l.widths,
                        "heights": l.heights,
                    })
                }).collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        };
        return serde_json::json!({
            "success": true,
            "command": cmd_str,
            "data": {
                "count": layouts.len(),
                "layouts": layouts,
            },
        });
    }

    // Handle session save/restore commands
    if cmd_str == "save-session" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &mut *ptr;
                platform.save_session();
                return IpcResponse { success: true, message: Some("Session saved".into()), data: None };
            }
        }
        return IpcResponse { success: false, message: Some("Platform not available".into()), data: None };
    }

    if cmd_str == "restore-session" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &mut *ptr;
                platform.restore_session();
                return IpcResponse { success: true, message: Some("Session restored".into()), data: None };
            }
        }
        return IpcResponse { success: false, message: Some("Platform not available".into()), data: None };
    }

    // Handle startup integration commands
    if cmd_str == "enable-startup" {
        if let Err(e) = crate::platform::Platform::enable_startup() {
            return IpcResponse { success: false, message: Some(format!("Failed: {}", e)), data: None };
        }
        return IpcResponse { success: true, message: Some("UltraWM will run on login".into()), data: None };
    }

    if cmd_str == "disable-startup" {
        if let Err(e) = crate::platform::Platform::disable_startup() {
            return IpcResponse { success: false, message: Some(format!("Failed: {}", e)), data: None };
        }
        return IpcResponse { success: true, message: Some("UltraWM removed from login".into()), data: None };
    }

    if cmd_str == "startup-status" {
        let enabled = crate::platform::Platform::is_startup_enabled();
        return IpcResponse {
            success: true,
            message: Some(if enabled { "enabled" } else { "disabled" }.into()),
            data: Some(serde_json::json!({ "enabled": enabled })),
        };
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
        "get-app-opacity" => {
            if let Some(exe) = json.get("exe").and_then(|v| v.as_str()) {
                unsafe {
                    let ptr = crate::platform::keyboard::PLATFORM_PTR;
                    if !ptr.is_null() {
                        let platform = &*ptr;
                        let opacity = platform.per_app_opacity.get(exe).copied().unwrap_or(1.0);
                        return serde_json::json!({
                            "success": true,
                            "command": cmd_str,
                            "data": {
                                "exe": exe,
                                "opacity": opacity,
                            },
                        });
                    }
                }
            }
            return serde_json::json!({
                "success": false,
                "command": cmd_str,
                "message": "Platform not available or missing 'exe' field",
            });
        }
        "list-app-opacities" => {
            let opacities = unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    let mut items = Vec::new();
                    for (exe, &opacity) in &platform.per_app_opacity {
                        items.push(serde_json::json!({
                            "exe": exe,
                            "opacity": opacity,
                        }));
                    }
                    items
                } else {
                    Vec::new()
                }
            };
            return serde_json::json!({
                "success": true,
                "command": cmd_str,
                "data": {
                    "count": opacities.len(),
                    "apps": opacities,
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
            let (monitor_idx, workspaces) = unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    let mon_idx = platform.focused_hwnd
                        .and_then(|hwnd| platform.window_for_hwnd(hwnd.0))
                        .and_then(|info| platform.window_monitors.get(&info.id))
                        .copied()
                        .unwrap_or(0);
                    let mws = &platform.monitor_workspaces[mon_idx];
                    let ws_count = mws.grids.len();
                    let per_mon = &platform.config.layout.per_monitor_workspace_names;
                    let ws_names: Vec<String> = if let Some(names) = per_mon.get(mon_idx) {
                        if !names.is_empty() {
                            names.iter().take(ws_count).cloned().collect()
                        } else {
                            let global = &platform.config.layout.workspace_names;
                            if global.is_empty() {
                                (1..=ws_count).map(|i| i.to_string()).collect()
                            } else {
                                global.iter().take(ws_count).cloned().collect()
                            }
                        }
                    } else {
                        let global = &platform.config.layout.workspace_names;
                        if global.is_empty() {
                            (1..=ws_count).map(|i| i.to_string()).collect()
                        } else {
                            global.iter().take(ws_count).cloned().collect()
                        }
                    };
                    let counts = platform.window_count_per_workspace();
                    let mut ws_data = Vec::new();
                    for (i, name) in ws_names.iter().enumerate() {
                        ws_data.push(serde_json::json!({
                            "index": i,
                            "name": name,
                            "window_count": counts.get(i).copied().unwrap_or(0),
                            "active": i == mws.current,
                        }));
                    }
                    let mon_count = platform.monitors.len();
                    (mon_count, serde_json::json!({
                        "monitor_count": mon_count,
                        "workspaces": ws_data,
                        "current": mws.current,
                    }))
                } else {
                    (0, serde_json::json!({
                        "monitor_count": 0,
                        "workspaces": Vec::new(),
                        "current": 0,
                    }))
                }
            };
            return serde_json::json!({
                "success": true,
                "command": cmd_str,
                "data": workspaces,
                "monitors": monitor_idx,
            });
        }
        "diagnose" => {
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    if let Err(e) = platform.diagnose() {
                        return serde_json::json!({
                            "success": false,
                            "command": cmd_str,
                            "message": format!("Diagnostics failed: {}", e),
                        });
                    }
                    return serde_json::json!({
                        "success": true,
                        "command": cmd_str,
                        "message": "Diagnostics printed to log",
                    });
                }
            }
            return serde_json::json!({
                "success": false,
                "command": cmd_str,
                "message": "Platform not available",
            });
        }
        "list-layout-presets" => {
            let presets = unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    platform.config.layout.layout_presets.iter().map(|p| {
                        serde_json::json!({
                            "name": p.name,
                            "kind": p.kind,
                            "cols": p.cols,
                            "rows": p.rows,
                        })
                    }).collect::<Vec<_>>()
                } else {
                    Vec::new()
                }
            };
            return serde_json::json!({
                "success": true,
                "command": cmd_str,
                "data": {
                    "count": presets.len(),
                    "presets": presets,
                },
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
        "search-windows" => {
            let query = json.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let filter_workspace = json.get("workspace").and_then(|v| v.as_u64()).map(|u| u as usize);
            let filter_monitor = json.get("monitor").and_then(|v| v.as_u64()).map(|u| u as usize);
            let filter_floating = json.get("floating").and_then(|v| v.as_bool());
            let filter_minimized = json.get("minimized").and_then(|v| v.as_bool());

            let results = unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    let q = query.to_lowercase();
                    let mut matches = Vec::new();
                    for (hwnd_wrapper, info) in &platform.windows {
                        if !info.visible { continue; }
                        if let Some(ws) = filter_workspace {
                            let win_ws = platform.window_workspaces.get(&info.id).copied().unwrap_or(0);
                            if win_ws != ws { continue; }
                        }
                        if let Some(mon) = filter_monitor {
                            let win_mon = platform.window_monitors.get(&info.id).copied().unwrap_or(0);
                            if win_mon != mon { continue; }
                        }
                        if let Some(f) = filter_floating {
                            if info.floating != f { continue; }
                        }
                        if let Some(m) = filter_minimized {
                            if info.minimized != m { continue; }
                        }
                        if !q.is_empty() && !info.title.to_lowercase().contains(&q) && !info.exe.to_lowercase().contains(&q) {
                            continue;
                        }
                        let ws = platform.window_workspaces.get(&info.id).copied().unwrap_or(0);
                        let mon = platform.window_monitors.get(&info.id).copied().unwrap_or(0);
                        matches.push(serde_json::json!({
                            "hwnd": hwnd_wrapper.0 .0 as usize,
                            "title": info.title,
                            "exe": info.exe,
                            "workspace": ws,
                            "monitor": mon,
                            "floating": info.floating,
                            "minimized": info.minimized,
                            "always_on_top": info.always_on_top,
                            "opacity": info.opacity,
                        }));
                    }
                    matches
                } else {
                    Vec::new()
                }
            };
            return serde_json::json!({
                "success": true,
                "command": cmd_str,
                "data": {
                    "query": query,
                    "count": results.len(),
                    "windows": results,
                },
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
        "focus-monitor" => {
            if let Some(mon) = json.get("monitor").and_then(|v| v.as_u64()) {
                unsafe {
                    let ptr = crate::platform::keyboard::PLATFORM_PTR;
                    if !ptr.is_null() {
                        let platform = &mut *ptr;
                        platform.focus_monitor(mon as usize);
                        return serde_json::json!({
                            "success": true,
                            "command": cmd_str,
                            "message": format!("Focusing monitor {}", mon + 1),
                        });
                    }
                }
            }
            return serde_json::json!({
                "success": false,
                "command": cmd_str,
                "message": "missing 'monitor' field (0-indexed)",
            });
        }
        "move-to-monitor" => {
            if let Some(mon) = json.get("monitor").and_then(|v| v.as_u64()) {
                unsafe {
                    let ptr = crate::platform::keyboard::PLATFORM_PTR;
                    if !ptr.is_null() {
                        let platform = &mut *ptr;
                        platform.move_focused_to_monitor(mon as usize);
                        return serde_json::json!({
                            "success": true,
                            "command": cmd_str,
                            "message": format!("Moving window to monitor {}", mon + 1),
                        });
                    }
                }
            }
            return serde_json::json!({
                "success": false,
                "command": cmd_str,
                "message": "missing 'monitor' field (0-indexed)",
            });
        }
        "focus-next-monitor" => {
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &mut *ptr;
                    platform.focus_next_monitor();
                    return serde_json::json!({
                        "success": true,
                        "command": cmd_str,
                        "message": "Focusing next monitor",
                    });
                }
            }
            return serde_json::json!({
                "success": false,
                "command": cmd_str,
                "message": "Platform not available",
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
        "save-session" => IpcCommand::Single { command: "save-session".into() },
        "restore-session" => IpcCommand::Single { command: "restore-session".into() },
        "screenshot" => IpcCommand::Single { command: "screenshot".into() },
        "window-search" => IpcCommand::Single { command: "window-search".into() },
        "help" => IpcCommand::Single { command: "help".into() },
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
        "toggle-monocle" => IpcCommand::Single { command: "toggle-monocle".into() },
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
        "list-workspaces" => IpcCommand::Single { command: "list-workspaces".into() },
        "get-config" => IpcCommand::Single { command: "get-config".into() },
        "swap-windows" => IpcCommand::Single { command: "swap-windows".into() },
        "toggle-bar" => IpcCommand::Single { command: "toggle-bar".into() },
        "bring-to-front" => IpcCommand::Single { command: "bring-to-front".into() },
        "get-stats" => IpcCommand::Single { command: "get-stats".into() },
        "list-monitors" => IpcCommand::Single { command: "list-monitors".into() },
        "get-bar-config" => IpcCommand::Single { command: "get-bar-config".into() },
        "set-border-color" => IpcCommand::Single { command: "set-border-color".into() },
        "set-border-width" => IpcCommand::Single { command: "set-border-width".into() },
        "get-theme" => IpcCommand::Single { command: "get-theme".into() },
        "reset-layout" => IpcCommand::Single { command: "reset-layout".into() },
        "set-theme" => IpcCommand::Single { command: "set-theme".into() },
        "cycle-gap" => IpcCommand::Single { command: "cycle-gap".into() },
        "toggle-snap" => IpcCommand::Single { command: "toggle-snap".into() },
        "get-window-info" => IpcCommand::Single { command: "get-window-info".into() },
        "cycle-theme" => IpcCommand::Single { command: "cycle-theme".into() },
        "list-themes" => IpcCommand::Single { command: "list-themes".into() },
        "get-layout-presets" => IpcCommand::Single { command: "get-layout-presets".into() },
        "set-workspace-name" => IpcCommand::Single { command: "set-workspace-name".into() },
        "set-wallpaper-monitor" => IpcCommand::Single { command: "set-wallpaper-monitor".into() },
        "set-bar-position" => IpcCommand::Single { command: "set-bar-position".into() },
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

    // Handle set-monitor-bar-height command
    if cmd_str == "set-monitor-bar-height" {
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);
        let height = params.get("height").and_then(|v| v.as_u64()).map(|v| v as u32);
        if let (Some(mon_idx), Some(h)) = (monitor_idx, height) {
            if h >= 20 && h <= 200 {
                unsafe {
                    let ptr = crate::platform::keyboard::PLATFORM_PTR;
                    if !ptr.is_null() {
                        let platform = &*ptr;
                        let monitors = crate::platform::monitor::get_monitors();
                        if mon_idx < monitors.len() {
                            platform.bar_heights.insert(mon_idx, h);
                            return serde_json::json!({
                                "success": true,
                                "monitor": mon_idx,
                                "height": h,
                            });
                        } else {
                            return serde_json::json!({
                                "success": false,
                                "error": format!("monitor {} out of range (0-{})", mon_idx, monitors.len() - 1),
                            });
                        }
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "missing or invalid parameters (monitor, height 20-200)",
        });
    }

    // Handle get-monitor-bar-height command
    if cmd_str == "get-monitor-bar-height" {
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let default_height = platform.config.bar.height;
                if let Some(m_idx) = monitor_idx {
                    let h = platform.bar_heights.get(&m_idx).copied().unwrap_or(default_height);
                    return serde_json::json!({
                        "success": true,
                        "monitor": m_idx,
                        "height": h,
                    });
                } else {
                    let all_heights: Vec<(usize, u32)> = platform.bar_heights.iter().map(|(k, v)| (*k, *v)).collect();
                    return serde_json::json!({
                        "success": true,
                        "default_height": default_height,
                        "overrides": all_heights,
                    });
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "platform not available",
        });
    }

    // Handle set-window-opacity command
    if cmd_str == "set-window-opacity" {
        let window_id = params.get("window_id").and_then(|v| v.as_u64()).map(|v| v as u64);
        let opacity = params.get("opacity").and_then(|v| v.as_f64()).map(|v| v as f32);
        let focused_only = params.get("focused").and_then(|v| v.as_bool()).unwrap_or(false);
        if let Some(op) = opacity {
            if op >= 0.0 && op <= 1.0 {
                unsafe {
                    let ptr = crate::platform::keyboard::PLATFORM_PTR;
                    if !ptr.is_null() {
                        let platform = &mut *ptr;
                        if !focused_only {
                            if let Some(wid) = window_id {
                                let hwnd_opt = platform.windows.iter().find(|(_, i)| i.id == wid).map(|(hw, _)| *hw);
                                if let Some(hwnd_wrapper) = hwnd_opt {
                                    if let Some(info) = platform.windows.get_mut(&hwnd_wrapper) {
                                        info.opacity = Some(op);
                                        platform.apply_window_opacity(hwnd_wrapper.0, op);
                                        return serde_json::json!({
                                            "success": true,
                                            "window_id": wid,
                                            "opacity": op,
                                        });
                                    }
                                }
                            }
                        }
                        // Fall back to focused window
                        if let Some(hwnd_wrapper) = platform.focused_hwnd {
                            if let Some(info) = platform.windows.get_mut(&hwnd_wrapper) {
                                let wid = info.id;
                                info.opacity = Some(op);
                                platform.apply_window_opacity(hwnd_wrapper.0, op);
                                return serde_json::json!({
                                    "success": true,
                                    "window_id": wid,
                                    "opacity": op,
                                    "note": "applied to focused window",
                                });
                            }
                        }
                    }
                }
                return serde_json::json!({
                    "success": false,
                    "error": "no window specified or focused",
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "missing or invalid 'opacity' parameter (0.0-1.0)",
        });
    }

    // Handle get-window-opacity command
    if cmd_str == "get-window-opacity" {
        let window_id = params.get("window_id").and_then(|v| v.as_u64()).map(|v| v as u64);
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                if let Some(wid) = window_id {
                    let hwnd_opt = platform.windows.iter().find(|(_, i)| i.id == wid).map(|(hw, _)| *hw);
                    if let Some(hwnd_wrapper) = hwnd_opt {
                        if let Some(info) = platform.windows.get(&hwnd_wrapper) {
                            return serde_json::json!({
                                "success": true,
                                "window_id": wid,
                                "opacity": info.opacity.unwrap_or(1.0),
                            });
                        }
                    }
                    return serde_json::json!({
                        "success": false,
                        "error": format!("window {} not found", wid),
                    });
                } else if let Some(hwnd_wrapper) = platform.focused_hwnd {
                    if let Some(info) = platform.windows.get(&hwnd_wrapper) {
                        return serde_json::json!({
                            "success": true,
                            "window_id": info.id,
                            "opacity": info.opacity.unwrap_or(1.0),
                        });
                    }
                }
                return serde_json::json!({
                    "success": false,
                    "error": "no window specified or focused",
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "platform not available",
        });
    }

    // Handle set-monitor-bar-enabled command
    if cmd_str == "set-monitor-bar-enabled" {
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);
        let enabled = params.get("enabled").and_then(|v| v.as_bool());
        if let (Some(mon_idx), Some(e)) = (monitor_idx, enabled) {
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    let monitors = crate::platform::monitor::get_monitors();
                    if mon_idx < monitors.len() {
                        platform.bar_enabled_monitors.insert(mon_idx, e);
                        return serde_json::json!({
                            "success": true,
                            "monitor": mon_idx,
                            "enabled": e,
                        });
                    } else {
                        return serde_json::json!({
                            "success": false,
                            "error": format!("monitor {} out of range (0-{})", mon_idx, monitors.len() - 1),
                        });
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "missing or invalid parameters (monitor, enabled)",
        });
    }

    // Handle get-monitor-bar-enabled command
    if cmd_str == "get-monitor-bar-enabled" {
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let default_enabled = platform.config.bar.enabled;
                if let Some(m_idx) = monitor_idx {
                    let e = platform.bar_enabled_monitors.get(&m_idx).copied().unwrap_or(default_enabled);
                    return serde_json::json!({
                        "success": true,
                        "monitor": m_idx,
                        "enabled": e,
                    });
                } else {
                    let all_enabled: Vec<(usize, bool)> = platform.bar_enabled_monitors.iter().map(|(k, v)| (*k, *v)).collect();
                    return serde_json::json!({
                        "success": true,
                        "default_enabled": default_enabled,
                        "overrides": all_enabled,
                    });
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "platform not available",
        });
    }

    // Handle set-monitor-bar-transparency command
    if cmd_str == "set-monitor-bar-transparency" {
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);
        let transparency = params.get("transparency").and_then(|v| v.as_f64()).map(|v| v as f32);
        if let (Some(mon_idx), Some(t)) = (monitor_idx, transparency) {
            if t >= 0.0 && t <= 1.0 {
                unsafe {
                    let ptr = crate::platform::keyboard::PLATFORM_PTR;
                    if !ptr.is_null() {
                        let platform = &*ptr;
                        let monitors = crate::platform::monitor::get_monitors();
                        if mon_idx < monitors.len() {
                            platform.bar_transparencies.insert(mon_idx, t);
                            return serde_json::json!({
                                "success": true,
                                "monitor": mon_idx,
                                "transparency": t,
                            });
                        } else {
                            return serde_json::json!({
                                "success": false,
                                "error": format!("monitor {} out of range (0-{})", mon_idx, monitors.len() - 1),
                            });
                        }
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "missing or invalid parameters (monitor, transparency 0.0-1.0)",
        });
    }

    // Handle get-monitor-bar-transparency command
    if cmd_str == "get-monitor-bar-transparency" {
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let default_transparency = platform.config.bar.transparency;
                if let Some(m_idx) = monitor_idx {
                    let t = platform.bar_transparencies.get(&m_idx).copied().unwrap_or(default_transparency);
                    return serde_json::json!({
                        "success": true,
                        "monitor": m_idx,
                        "transparency": t,
                    });
                } else {
                    let all_transparencies: Vec<(usize, f32)> = platform.bar_transparencies.iter().map(|(k, v)| (*k, *v)).collect();
                    return serde_json::json!({
                        "success": true,
                        "default_transparency": default_transparency,
                        "overrides": all_transparencies,
                    });
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "platform not available",
        });
    }

    // Handle reload-config command
    if cmd_str == "reload-config" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &mut *ptr;
                match platform.config.reload_if_changed() {
                    Ok(Some(new_config)) => {
                        let old = std::mem::replace(&mut platform.config, new_config);
                        let changes = vec!["config reloaded from disk"];
                        let _ = platform.config.save();
                        return serde_json::json!({
                            "success": true,
                            "message": "Config reloaded",
                            "changes": changes,
                        });
                    }
                    Ok(None) => {
                        return serde_json::json!({
                            "success": true,
                            "message": "Config unchanged",
                            "changes": vec![],
                        });
                    }
                    Err(e) => {
                        return serde_json::json!({
                            "success": false,
                            "error": format!("Failed to reload config: {}", e),
                        });
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "platform not available",
        });
    }

    // Handle list-workspaces command
    if cmd_str == "list-workspaces" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let monitors = crate::platform::monitor::get_monitors();
                let mut ws_list: Vec<serde_json::Value> = Vec::new();

                for (mon_idx, mws) in platform.monitor_workspaces.iter().enumerate() {
                    let mon_name = monitors.get(mon_idx).map(|m| m.name.clone()).unwrap_or_else(|| "unknown".into());
                    for (ws_idx, grid) in mws.grids.iter().enumerate() {
                        let mut win_list: Vec<serde_json::Value> = Vec::new();
                        for &wid in grid.windows.iter() {
                            let win_info = platform.windows.iter().find(|(_, i)| i.id == wid);
                            if let Some((hwnd_wrapper, info)) = win_info {
                                win_list.push(serde_json::json!({
                                    "id": wid,
                                    "hwnd": hwnd_wrapper.0,
                                    "title": info.title,
                                    "exe": info.exe,
                                    "floating": info.floating,
                                    "fullscreen": info.fullscreen,
                                    "opacity": info.opacity.unwrap_or(1.0),
                                }));
                            }
                        }
                        ws_list.push(serde_json::json!({
                            "monitor": mon_idx,
                            "monitor_name": mon_name,
                            "workspace": ws_idx,
                            "active": mws.current == ws_idx,
                            "window_count": grid.windows.len(),
                            "windows": win_list,
                        }));
                    }
                }
                return serde_json::json!({
                    "success": true,
                    "workspaces": ws_list,
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "platform not available",
        });
    }

    // Handle move-window-to-workspace command
    if cmd_str == "move-window-to-workspace" {
        let window_id = params.get("window_id").and_then(|v| v.as_u64()).map(|v| v as u64);
        let workspace_idx = params.get("workspace").and_then(|v| v.as_u64()).map(|v| v as usize);
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);
        let focused_only = params.get("focused").and_then(|v| v.as_bool()).unwrap_or(false);

        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &mut *ptr;

                // Determine target window
                let target_wid = if !focused_only {
                    window_id
                } else {
                    None
                };

                if let Some(wid) = target_wid {
                    let hwnd_opt = platform.windows.iter().find(|(_, i)| i.id == wid).map(|(hw, _)| *hw);
                    if let Some(hwnd_wrapper) = hwnd_opt {
                        if let Some(ws_idx) = workspace_idx {
                            let target_mon = monitor_idx.unwrap_or_else(|| {
                                platform.window_monitors.get(&wid).copied().unwrap_or(0)
                            });
                            if target_mon < platform.monitor_workspaces.len() {
                                if ws_idx < platform.monitor_workspaces[target_mon].grids.len() {
                                    platform.window_workspaces.insert(wid, ws_idx);
                                    platform.window_monitors.insert(wid, target_mon);
                                    platform.refresh_bar_workspaces(target_mon);
                                    return serde_json::json!({
                                        "success": true,
                                        "window_id": wid,
                                        "workspace": ws_idx,
                                        "monitor": target_mon,
                                    });
                                }
                            }
                        }
                    }
                }

                // Fall back to focused window
                if focused_only || window_id.is_none() {
                    if let Some(hwnd_wrapper) = platform.focused_hwnd {
                        if let Some(info) = platform.windows.get(&hwnd_wrapper) {
                            let wid = info.id;
                            if let Some(ws_idx) = workspace_idx {
                                let target_mon = monitor_idx.unwrap_or_else(|| {
                                    platform.window_monitors.get(&wid).copied().unwrap_or(0)
                                });
                                if target_mon < platform.monitor_workspaces.len() {
                                    if ws_idx < platform.monitor_workspaces[target_mon].grids.len() {
                                        platform.window_workspaces.insert(wid, ws_idx);
                                        platform.window_monitors.insert(wid, target_mon);
                                        platform.refresh_bar_workspaces(target_mon);
                                        return serde_json::json!({
                                            "success": true,
                                            "window_id": wid,
                                            "workspace": ws_idx,
                                            "monitor": target_mon,
                                            "note": "applied to focused window",
                                        });
                                    }
                                }
                            }
                        }
                    }
                }

                return serde_json::json!({
                    "success": false,
                    "error": "no window specified or focused",
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "platform not available",
        });
    }

    // Handle apply-layout-preset command
    if cmd_str == "apply-layout-preset" {
        let preset_name = params.get("preset").and_then(|v| v.as_str());
        if let Some(name) = preset_name {
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &mut *ptr;
                    let presets = &platform.config.layout.layout_presets;
                    if let Some(preset) = presets.iter().find(|p| p.name == name) {
                        let mut changes = Vec::new();
                        if let Some(g) = preset.gaps {
                            platform.config.layout.gaps = g;
                            changes.push(format!("gaps: {}", g));
                        }
                        if let Some(ip) = preset.inner_padding {
                            platform.config.layout.inner_padding = ip;
                            changes.push(format!("inner_padding: {}", ip));
                        }
                        if let Some(bw) = preset.border_width {
                            platform.config.layout.border_width = bw;
                            changes.push(format!("border_width: {}", bw));
                        }
                        if let Some(cr) = preset.corner_radius {
                            platform.config.layout.corner_radius = cr;
                            changes.push(format!("corner_radius: {}", cr));
                        }
                        let _ = platform.config.save();
                        return serde_json::json!({
                            "success": true,
                            "preset": name,
                            "changes": changes,
                        });
                    } else {
                        return serde_json::json!({
                            "success": false,
                            "error": format!("preset '{}' not found", name),
                        });
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "missing 'preset' parameter",
        });
    }

    // Handle get-session command
    if cmd_str == "get-session" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let session = platform.session.as_ref();
                let has_session = session.is_some();
                let window_count = platform.windows.len();
                let monitor_count = platform.monitors.len();

                let session_data = session.map(|s| {
                    serde_json::json!({
                        "window_count": s.windows.len(),
                        "workspace_count": s.workspace_count,
                        "timestamp": s.timestamp,
                    })
                }).unwrap_or_else(|| serde_json::json!(null));

                return serde_json::json!({
                    "success": true,
                    "has_session": has_session,
                    "current_windows": window_count,
                    "monitor_count": monitor_count,
                    "session": session_data,
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "platform not available",
        });
    }

    // Handle list-monitor-workspaces command
    if cmd_str == "list-monitor-workspaces" {
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let monitors = crate::platform::monitor::get_monitors();

                // If no monitor specified, list all
                if monitor_idx.is_none() {
                    let mut all: Vec<serde_json::Value> = Vec::new();
                    for (idx, mws) in platform.monitor_workspaces.iter().enumerate() {
                        let mon_name = monitors.get(idx).map(|m| m.name.clone()).unwrap_or_else(|| "unknown".into());
                        let mut ws_list: Vec<serde_json::Value> = Vec::new();
                        for (ws_idx, grid) in mws.grids.iter().enumerate() {
                            let win_count = grid.windows.len();
                            let is_active = mws.current == ws_idx;
                            ws_list.push(serde_json::json!({
                                "index": ws_idx,
                                "active": is_active,
                                "window_count": win_count,
                            }));
                        }
                        all.push(serde_json::json!({
                            "monitor": idx,
                            "name": mon_name,
                            "current": mws.current,
                            "workspaces": ws_list,
                        }));
                    }
                    return serde_json::json!({
                        "success": true,
                        "monitors": all,
                    });
                }

                // Specific monitor
                if let Some(m_idx) = monitor_idx {
                    if m_idx < platform.monitor_workspaces.len() {
                        let mws = &platform.monitor_workspaces[m_idx];
                        let mon_name = monitors.get(m_idx).map(|m| m.name.clone()).unwrap_or_else(|| "unknown".into());
                        let mut ws_list: Vec<serde_json::Value> = Vec::new();
                        for (ws_idx, grid) in mws.grids.iter().enumerate() {
                            let win_count = grid.windows.len();
                            let is_active = mws.current == ws_idx;
                            ws_list.push(serde_json::json!({
                                "index": ws_idx,
                                "active": is_active,
                                "window_count": win_count,
                            }));
                        }
                        return serde_json::json!({
                            "success": true,
                            "monitor": m_idx,
                            "name": mon_name,
                            "current": mws.current,
                            "workspaces": ws_list,
                        });
                    } else {
                        return serde_json::json!({
                            "success": false,
                            "error": format!("monitor {} out of range", m_idx),
                        });
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "platform not available",
        });
    }

    // Handle ping command
    if cmd_str == "ping" {
        return serde_json::json!({
            "success": true,
            "command": "ping",
            "message": "pong",
            "version": env!("CARGO_PKG_VERSION"),
        });
    }

    // Handle list-all-windows command
    if cmd_str == "list-all-windows" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let mut windows: Vec<serde_json::Value> = Vec::new();
                for (hwnd_wrapper, info) in platform.windows.iter() {
                    let mon_idx = platform.window_monitors.get(&info.id).copied().unwrap_or(0);
                    let ws_idx = platform.window_workspaces.get(&info.id).copied().unwrap_or(0);
                    let bw = platform.window_border_widths.get(&info.id).copied();
                    let opacity = info.opacity.unwrap_or(1.0);
                    let is_focused = platform.focused_hwnd == Some(*hwnd_wrapper);
                    windows.push(serde_json::json!({
                        "hwnd": hwnd_wrapper.0,
                        "id": info.id,
                        "title": info.title,
                        "class": info.class,
                        "exe": info.exe,
                        "visible": info.visible,
                        "floating": info.floating,
                        "fullscreen": info.fullscreen,
                        "always_on_top": info.always_on_top,
                        "minimized": info.minimized,
                        "sticky": info.sticky,
                        "monitor": mon_idx,
                        "workspace": ws_idx,
                        "border_color": format!("0x{:08X}", info.border_color),
                        "border_width": bw.unwrap_or(platform.config.layout.border_width),
                        "opacity": opacity,
                        "focused": is_focused,
                    }));
                }
                return serde_json::json!({
                    "success": true,
                    "command": "list-all-windows",
                    "count": windows.len(),
                    "windows": windows,
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "platform not available",
        });
    }

    // Handle get-monitor-info command
    if cmd_str == "get-monitor-info" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let monitors = crate::platform::monitor::get_monitors();
                let mut mon_infos: Vec<serde_json::Value> = Vec::new();
                for (idx, mon) in monitors.iter().enumerate() {
                    let ws_count = platform.monitor_workspaces.get(idx).map(|mws| mws.grids.len()).unwrap_or(0);
                    let current_ws = platform.monitor_workspaces.get(idx).map(|mws| mws.current).unwrap_or(0);
                    let window_count = platform.window_monitors.values().filter(|&&m| m == idx).count();
                    mon_infos.push(serde_json::json!({
                        "index": idx,
                        "name": mon.name,
                        "x": mon.left,
                        "y": mon.top,
                        "width": mon.width(),
                        "height": mon.height(),
                        "work_width": mon.work_width(),
                        "work_height": mon.work_height(),
                        "scale_factor": mon.scale_factor,
                        "workspace_count": ws_count,
                        "current_workspace": current_ws,
                        "window_count": window_count,
                        "primary": mon.is_primary,
                    }));
                }
                return serde_json::json!({
                    "success": true,
                    "monitor_count": monitors.len(),
                    "monitors": mon_infos,
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "platform not available",
        });
    }

    // Handle get-monitor-layout command
    if cmd_str == "get-monitor-layout" {
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let layouts = &platform.config.layout.monitor_layouts;

                if let Some(m_idx) = monitor_idx {
                    let layout = layouts.get(m_idx).cloned().unwrap_or(crate::config::MonitorLayout {
                        gaps: None,
                        inner_padding: None,
                        outer_padding: None,
                        border_width: None,
                        corner_radius: None,
                    });
                    return serde_json::json!({
                        "success": true,
                        "monitor": m_idx,
                        "layout": {
                            "gaps": layout.gaps,
                            "inner_padding": layout.inner_padding,
                            "border_width": layout.border_width,
                            "corner_radius": layout.corner_radius,
                        },
                        "effective": {
                            "gaps": layout.gaps.unwrap_or(platform.config.layout.gaps),
                            "inner_padding": layout.inner_padding.unwrap_or(platform.config.layout.inner_padding),
                            "border_width": layout.border_width.unwrap_or(platform.config.layout.border_width),
                            "corner_radius": layout.corner_radius.unwrap_or(platform.config.layout.corner_radius),
                        },
                    });
                } else {
                    let all_layouts: Vec<serde_json::Value> = layouts.iter().enumerate().map(|(idx, layout)| {
                        serde_json::json!({
                            "monitor": idx,
                            "gaps": layout.gaps,
                            "inner_padding": layout.inner_padding,
                            "border_width": layout.border_width,
                            "corner_radius": layout.corner_radius,
                        })
                    }).collect();
                    return serde_json::json!({
                        "success": true,
                        "defaults": {
                            "gaps": platform.config.layout.gaps,
                            "inner_padding": platform.config.layout.inner_padding,
                            "border_width": platform.config.layout.border_width,
                            "corner_radius": platform.config.layout.corner_radius,
                        },
                        "overrides": all_layouts,
                    });
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "platform not available",
        });
    }

    // Handle set-monitor-layout-preset command
    if cmd_str == "set-monitor-layout-preset" {
        let preset_name = params.get("preset").and_then(|v| v.as_str());
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);

        if let (Some(name), Some(mon_idx)) = (preset_name, monitor_idx) {
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &mut *ptr;
                    let presets = &platform.config.layout.layout_presets;
                    if let Some(preset) = presets.iter().find(|p| p.name == name) {
                        let layouts = &mut platform.config.layout.monitor_layouts;
                        if mon_idx >= layouts.len() {
                            layouts.resize(mon_idx + 1, crate::config::MonitorLayout {
                                gaps: None,
                                inner_padding: None,
                                outer_padding: None,
                                border_width: None,
                                corner_radius: None,
                            });
                        }
                        let layout = &mut layouts[mon_idx];
                        layout.gaps = preset.gaps;
                        layout.inner_padding = preset.inner_padding;
                        layout.border_width = preset.border_width;
                        layout.corner_radius = preset.corner_radius;
                        let _ = platform.config.save();
                        return serde_json::json!({
                            "success": true,
                            "monitor": mon_idx,
                            "preset": name,
                            "layout": {
                                "gaps": layout.gaps,
                                "inner_padding": layout.inner_padding,
                                "border_width": layout.border_width,
                                "corner_radius": layout.corner_radius,
                            },
                        });
                    } else {
                        return serde_json::json!({
                            "success": false,
                            "error": format!("preset '{}' not found", name),
                        });
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "missing 'preset' or 'monitor' parameter",
        });
    }

    // Handle set-monitor-layout command
    if cmd_str == "set-monitor-layout" {
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);
        let gaps = params.get("gaps").and_then(|v| v.as_u64()).map(|v| v as u32);
        let inner_padding = params.get("inner_padding").and_then(|v| v.as_u64()).map(|v| v as u32);
        let border_width = params.get("border_width").and_then(|v| v.as_u64()).map(|v| v as u32);
        let corner_radius = params.get("corner_radius").and_then(|v| v.as_u64()).map(|v| v as u32);

        if let Some(mon_idx) = monitor_idx {
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    let layouts = &mut platform.config.layout.monitor_layouts;
                    if mon_idx >= layouts.len() {
                        layouts.resize(mon_idx + 1, crate::config::MonitorLayout {
                            gaps: None,
                            inner_padding: None,
                            outer_padding: None,
                            border_width: None,
                            corner_radius: None,
                        });
                    }
                    let layout = &mut layouts[mon_idx];
                    if let Some(g) = gaps { layout.gaps = Some(g); }
                    if let Some(ip) = inner_padding { layout.inner_padding = Some(ip); }
                    if let Some(bw) = border_width { layout.border_width = Some(bw); }
                    if let Some(cr) = corner_radius { layout.corner_radius = Some(cr); }

                    let _ = platform.config.save();
                    return serde_json::json!({
                        "success": true,
                        "monitor": mon_idx,
                        "layout": {
                            "gaps": layout.gaps,
                            "inner_padding": layout.inner_padding,
                            "border_width": layout.border_width,
                            "corner_radius": layout.corner_radius,
                        },
                    });
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "missing 'monitor' parameter",
        });
    }

    // Handle add-monitor-workspace command
    if cmd_str == "add-monitor-workspace" {
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);
        if let Some(mon_idx) = monitor_idx {
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    if mon_idx < platform.monitor_workspaces.len() {
                        let new_count = platform.monitor_workspaces[mon_idx].grids.len() + 1;
                        platform.monitor_workspaces[mon_idx].grids.push(GridState::new());
                        if platform.config.layout.workspace_count < new_count {
                            platform.config.layout.workspace_count = new_count;
                        }
                        if let Some(ref bar) = platform.bar {
                            let ws_names = platform.workspace_names(mon_idx);
                            let current = platform.monitor_workspaces[mon_idx].current;
                            bar.set_workspaces(ws_names, current, platform.window_count_per_workspace(mon_idx));
                        }
                        return serde_json::json!({
                            "success": true,
                            "monitor": mon_idx,
                            "workspace_count": new_count,
                        });
                    } else {
                        return serde_json::json!({
                            "success": false,
                            "error": format!("monitor {} out of range", mon_idx),
                        });
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "missing 'monitor' parameter",
        });
    }

    // Handle remove-monitor-workspace command
    if cmd_str == "remove-monitor-workspace" {
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);
        if let Some(mon_idx) = monitor_idx {
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    if mon_idx < platform.monitor_workspaces.len() {
                        let mws = &mut platform.monitor_workspaces[mon_idx];
                        if mws.grids.len() > 1 {
                            mws.grids.pop();
                            if mws.current >= mws.grids.len() {
                                mws.current = mws.grids.len() - 1;
                            }
                            if platform.config.layout.workspace_count > mws.grids.len() {
                                platform.config.layout.workspace_count = mws.grids.len();
                            }
                            if let Some(ref bar) = platform.bar {
                                let ws_names = platform.workspace_names(mon_idx);
                                let current = mws.current;
                                bar.set_workspaces(ws_names, current, platform.window_count_per_workspace(mon_idx));
                            }
                            return serde_json::json!({
                                "success": true,
                                "monitor": mon_idx,
                                "workspace_count": mws.grids.len(),
                            });
                        } else {
                            return serde_json::json!({
                                "success": false,
                                "error": "cannot remove last workspace",
                            });
                        }
                    } else {
                        return serde_json::json!({
                            "success": false,
                            "error": format!("monitor {} out of range", mon_idx),
                        });
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "missing 'monitor' parameter",
        });
    }

    // Handle set-monitor-workspace command
    if cmd_str == "set-monitor-workspace" {
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);
        let workspace_idx = params.get("workspace").and_then(|v| v.as_u64()).map(|v| v as usize);
        if let (Some(mon_idx), Some(ws_idx)) = (monitor_idx, workspace_idx) {
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    if mon_idx < platform.monitor_workspaces.len() {
                        if ws_idx < platform.monitor_workspaces[mon_idx].grids.len() {
                            platform.monitor_workspaces[mon_idx].current = ws_idx;
                            let ws_names = platform.workspace_names(mon_idx);
                            if let Some(ref bar) = platform.bar {
                                bar.set_workspaces(ws_names, ws_idx, platform.window_count_per_workspace(mon_idx));
                            }
                            return serde_json::json!({
                                "success": true,
                                "monitor": mon_idx,
                                "workspace": ws_idx,
                            });
                        } else {
                            return serde_json::json!({
                                "success": false,
                                "error": format!("workspace {} out of range", ws_idx),
                            });
                        }
                    } else {
                        return serde_json::json!({
                            "success": false,
                            "error": format!("monitor {} out of range", mon_idx),
                        });
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "missing 'monitor' or 'workspace' parameter",
        });
    }

    // Handle set-monitor-workspace-names command
    if cmd_str == "set-monitor-workspace-names" {
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);
        let names = params.get("names").and_then(|v| v.as_array());

        if let (Some(mon_idx), Some(name_arr)) = (monitor_idx, names) {
            let name_vec: Vec<String> = name_arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
            if !name_vec.is_empty() {
                unsafe {
                    let ptr = crate::platform::keyboard::PLATFORM_PTR;
                    if !ptr.is_null() {
                        let platform = &mut *ptr;
                        if mon_idx < platform.workspace_names.len() {
                            platform.workspace_names[mon_idx] = name_vec;
                            platform.refresh_bar_workspaces(mon_idx);
                            return serde_json::json!({
                                "success": true,
                                "monitor": mon_idx,
                                "names": platform.workspace_names[mon_idx].clone(),
                            });
                        }
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "missing 'monitor' or 'names' parameter (names must be an array of strings)",
        });
    }

    // Handle get-workspace-names command
    if cmd_str == "get-workspace-names" {
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                if let Some(m_idx) = monitor_idx {
                    let names = platform.workspace_names(m_idx);
                    let current = platform.monitor_workspaces[m_idx].current;
                    return serde_json::json!({
                        "success": true,
                        "monitor": m_idx,
                        "names": names,
                        "current": current,
                    });
                } else {
                    let mut all: Vec<serde_json::Value> = Vec::new();
                    for (idx, mws) in platform.monitor_workspaces.iter().enumerate() {
                        all.push(serde_json::json!({
                            "monitor": idx,
                            "names": platform.workspace_names(idx),
                            "current": mws.current,
                        }));
                    }
                    return serde_json::json!({
                        "success": true,
                        "workspaces": all,
                    });
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "platform not available",
        });
    }

    // Handle get-active-monitor command
    if cmd_str == "get-active-monitor" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let monitors = crate::platform::monitor::get_monitors();
                if let Some(hwnd_wrapper) = platform.focused_hwnd {
                    let wid = platform.windows.get(&hwnd_wrapper).map(|i| i.id);
                    if let Some(wid) = wid {
                        if let Some(mon_idx) = platform.window_monitors.get(&wid) {
                            let mon_name = monitors.get(*mon_idx).map(|m| m.name.clone()).unwrap_or_else(|| "unknown".into());
                            return serde_json::json!({
                                "success": true,
                                "monitor": mon_idx,
                                "name": mon_name,
                                "window_id": wid,
                            });
                        }
                    }
                }
                return serde_json::json!({
                    "success": true,
                    "monitor": None,
                    "name": None,
                    "message": "no focused window",
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "platform not available",
        });
    }

    // Handle list-monitor-bars command
    if cmd_str == "list-monitor-bars" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                let monitors = crate::platform::monitor::get_monitors();
                let mut bars: Vec<serde_json::Value> = Vec::new();
                for (idx, mon) in monitors.iter().enumerate() {
                    let height = platform.bar_heights.get(&idx).copied().unwrap_or(platform.config.bar.height);
                    let transparency = platform.bar_transparencies.get(&idx).copied().unwrap_or(platform.config.bar.transparency);
                    let enabled = platform.bar_enabled_monitors.get(&idx).copied().unwrap_or(platform.config.bar.enabled);
                    bars.push(serde_json::json!({
                        "monitor": idx,
                        "name": mon.name,
                        "enabled": enabled,
                        "height": height,
                        "transparency": transparency,
                    }));
                }
                return serde_json::json!({
                    "success": true,
                    "monitor_count": monitors.len(),
                    "bars": bars,
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "platform not available",
        });
    }

    // Handle set-monitor-focus command
    if cmd_str == "set-monitor-focus" {
        let monitor_idx = params.get("monitor").and_then(|v| v.as_u64()).map(|v| v as usize);
        if let Some(mon_idx) = monitor_idx {
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    let monitors = crate::platform::monitor::get_monitors();
                    if mon_idx < monitors.len() {
                        platform.switch_monitor(mon_idx);
                        return serde_json::json!({
                            "success": true,
                            "monitor": mon_idx,
                            "name": monitors[mon_idx].name,
                        });
                    } else {
                        return serde_json::json!({
                            "success": false,
                            "error": format!("monitor {} out of range (0-{})", mon_idx, monitors.len() - 1),
                        });
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "missing or invalid 'monitor' parameter",
        });
    }

    // Handle set-mouse-threshold command
    if cmd_str == "set-mouse-threshold" {
        let threshold = params.get("threshold").and_then(|v| v.as_f64()).map(|v| v as f32);
        if let Some(t) = threshold {
            if t >= 0.0 && t <= 5000.0 {
                unsafe {
                    let ptr = crate::platform::keyboard::PLATFORM_PTR;
                    if !ptr.is_null() {
                        let platform = &*ptr;
                        platform.config.mouse_follow_threshold = t;
                        let _ = platform.save_config();
                        return serde_json::json!({
                            "success": true,
                            "threshold": t,
                        });
                    }
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "missing or invalid 'threshold' parameter (0-5000)",
        });
    }

    // Handle get-mouse-threshold command
    if cmd_str == "get-mouse-threshold" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                return serde_json::json!({
                    "success": true,
                    "threshold": platform.config.mouse_follow_threshold,
                    "focus_follows_mouse": platform.config.layout.focus_follows_mouse,
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "platform not available",
        });
    }

    // Handle set-focus-follows-mouse command
    if cmd_str == "set-focus-follows-mouse" {
        let enabled = params.get("enabled").and_then(|v| v.as_bool());
        if let Some(e) = enabled {
            unsafe {
                let ptr = crate::platform::keyboard::PLATFORM_PTR;
                if !ptr.is_null() {
                    let platform = &*ptr;
                    platform.config.layout.focus_follows_mouse = e;
                    platform.focus_follows_mouse_check();
                    let _ = platform.save_config();
                    return serde_json::json!({
                        "success": true,
                        "focus_follows_mouse": e,
                    });
                }
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "missing or invalid 'enabled' parameter",
        });
    }

    // Handle get-focus-follows-mouse command
    if cmd_str == "get-focus-follows-mouse" {
        unsafe {
            let ptr = crate::platform::keyboard::PLATFORM_PTR;
            if !ptr.is_null() {
                let platform = &*ptr;
                return serde_json::json!({
                    "success": true,
                    "focus_follows_mouse": platform.config.layout.focus_follows_mouse,
                    "threshold": platform.config.mouse_follow_threshold,
                });
            }
        }
        return serde_json::json!({
            "success": false,
            "error": "platform not available",
        });
    }
}
