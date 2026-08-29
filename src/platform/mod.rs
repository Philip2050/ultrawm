use crate::anim::{Spring, SpringValue};
use crate::layout::{Cell, GridState};
use crate::theme::ThemeManager;
use chrono::Local;
use log::{debug, info, warn};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ptr::null_mut;
use std::sync::mpsc;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::{Dwm::*, Gdi::*},
        System::{LibraryLoader::*, Power::*, SystemInformation::*, Threading::*},
        UI::{Accessibility::*, HiDpi::*, Shell::*, WindowsAndMessaging::*},
    },
};

pub use window::WindowInfo;
pub use keyboard::KeyboardHook;
pub use border::BorderOverlay;
pub use bar::AppBar;
pub use launcher::AppLauncher;
pub use notifier::Notifier;
pub use gesture::GestureReceiver;
pub use theme_picker::ThemePicker;
pub use blur::enable_blur;
pub use scratchpad::ScratchpadManager;
pub use window_search::WindowSearch;
pub use help_overlay::HelpOverlay;

mod window;
pub mod keyboard;
pub mod border;
mod bar;
mod notifier;
mod launcher;
mod gesture;
mod theme_picker;
mod blur;
mod scratchpad;
mod window_search;
mod help_overlay;
mod wallpaper;
mod keybinds;

#[derive(Debug, Clone, Copy)]
pub struct HWnd(pub HWND);

#[derive(Debug, Clone, Copy)]
struct WindowAnimState {
    x: SpringValue,
    y: SpringValue,
    w: SpringValue,
    h: SpringValue,
}

impl WindowAnimState {
    fn new(x: f32, y: f32, w: f32, h: f32, stiffness: f32, damping: f32) -> Self {
        let spring = Spring {
            stiffness,
            damping,
            mass: 1.0,
        };
        Self {
            x: SpringValue::new(x).with_spring(spring),
            y: SpringValue::new(y).with_spring(spring),
            w: SpringValue::new(w).with_spring(spring),
            h: SpringValue::new(h).with_spring(spring),
        }
    }

    fn set_target(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.x.set_target(x);
        self.y.set_target(y);
        self.w.set_target(w);
        self.h.set_target(h);
    }

    fn step(&mut self, dt: f32) -> (f32, f32, f32, f32) {
        (
            self.x.step(dt),
            self.y.step(dt),
            self.w.step(dt),
            self.h.step(dt),
        )
    }
}

impl PartialEq for HWnd {
    fn eq(&self, other: &Self) -> bool {
        self.0 .0 == other.0 .0
    }
}
impl Eq for HWnd {}

impl Hash for HWnd {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0 .0.hash(state);
    }
}

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub handle: HMONITOR,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub work_left: i32,
    pub work_top: i32,
    pub work_right: i32,
    pub work_bottom: i32,
    pub dpi: u32,
    pub scale_factor: f32,
}

impl MonitorInfo {
    pub fn width(&self) -> i32 { self.right - self.left }
    pub fn height(&self) -> i32 { self.bottom - self.top }
    pub fn work_width(&self) -> i32 { self.work_right - self.work_left }
    pub fn work_height(&self) -> i32 { self.work_bottom - self.work_top }
    pub fn effective_width(&self) -> i32 { (self.width() as f32 * self.scale_factor) as i32 }
    pub fn effective_height(&self) -> i32 { (self.height() as f32 * self.scale_factor) as i32 }
}

pub struct Platform {
    pub windows: HashMap<HWnd, WindowInfo>,
    pub focused_hwnd: Option<HWnd>,
    pub monitors: Vec<MonitorInfo>,
    pub keyboard_hook: Option<keyboard::KeyboardHook>,
    pub keybinds: keybinds::ParsedKeybinds,
    pub border_overlay: Option<BorderOverlay>,
    pub bar: Option<AppBar>,
    pub bar_visible: bool,
    pub notifier: Option<Notifier>,
    // Each monitor has its own set of workspaces (independent)
    pub monitor_workspaces: Vec<MonitorWorkspaces>,
    pub window_workspaces: HashMap<u64, usize>, // wid -> workspace index (0-3)
    pub window_monitors: HashMap<u64, usize>,   // wid -> monitor index
    pub window_border_widths: HashMap<u64, u32>, // wid -> border width override
    pub anim: HashMap<u64, WindowAnimState>,
    pub swap_flash: HashMap<u64, u32>, // wid -> flash timer (countdown frames)
    pub opacity_anim: HashMap<u64, crate::anim::SpringValue>, // wid -> opacity spring animation
    pub snap_mode: bool,       // true when Win+G snap mode is active
    pub snap_flash: u32,       // snap mode flash timer (frames)
    pub monocle: bool,         // true when monocle layout is active
    last_rounded: HashMap<u64, (i32, i32, i32)>, // wid -> (w, h, radius) last applied
    last_frame_time: std::time::Instant,
    shadow_set: HashMap<u64, bool>, // wid -> whether DWM shadow is enabled
    per_app_opacity: HashMap<String, f32>, // exe -> remembered opacity
    pub config: crate::config::Config,
    pub config_reload_counter: u32,
    pub next_id: u64,
    pub win_event_hook: HWINEVENTHOOK,
    pub overview: bool,
    pub overview_positions: Vec<(i32, i32, i32, i32, HWnd)>,
    pub gesture_receiver: Option<GestureReceiver>,
    pub gesture_pan_start: Option<(i32, i32)>,
    pub gesture_pan_last: Option<(i32, i32)>,
    pub theme_picker: Option<ThemePicker>,
    pub session: Option<crate::session::SessionState>,
    pub scratchpad: Option<ScratchpadManager>,
    idle_inhibit: bool,
    pub theme_mgr: Option<RefCell<ThemeManager>>,
    ws_fade_anim: crate::anim::SpringValue, // workspace switch fade animation
    ws_pending_ws: Option<usize>,
    ws_pending_monitor: Option<usize>,
    ws_animating: bool,
    last_session_save: std::time::Instant,
    /// Windows minimized to system tray (wid -> hwnd)
    tray_windows: HashMap<u64, HWND>,
    /// Hidden window for tray icon notifications
    tray_hwnd: Option<HWND>,
}

pub struct MonitorWorkspaces {
    pub monitor: MonitorInfo,
    pub grids: Vec<GridState>,
    pub current: usize,
}

fn find_empty_cell(grid: &GridState) -> Option<Cell> {
    for dist in 0..100 {
        for dc in (-dist..=dist).rev() {
            for dr in (-dist..=dist).rev() {
                if dr == 0 && dc == 0 { continue; }
                let cell = Cell::new(dr, dc);
                if !grid.cells.contains_key(&cell) && !grid.cell_nodes.contains_key(&cell) {
                    return Some(cell);
                }
            }
        }
    }
    None
}

fn get_startup_dir() -> Option<std::path::PathBuf> {
    use std::env;
    dirs::home_dir().map(|h| h.join("AppData/Roaming/Microsoft/Windows/Start Menu/Programs/Startup"))
}

impl Platform {
    pub fn new() -> anyhow::Result<Self> {
        // Set per-monitor DPI awareness v2 for correct scaling on mixed-DPI setups
        unsafe {
            let _ = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }

        let monitors = enumerate_monitors()?;

        let monitor_workspaces = monitors
            .iter()
            .map(|m| MonitorWorkspaces {
                monitor: m.clone(),
                grids: vec![GridState::new(), GridState::new(), GridState::new(), GridState::new()],
                current: 0,
            })
            .collect();

        Ok(Self {
            windows: HashMap::new(),
            focused_hwnd: None,
            monitors,
            keyboard_hook: None,
            keybinds: keybinds::parse_keybinds(&crate::config::Config::default().keybinds),
            border_overlay: None,
            bar_visible: true,
            bar: None,
            notifier: None,
            monitor_workspaces,
            window_workspaces: HashMap::new(),
            window_monitors: HashMap::new(),
            window_border_widths: HashMap::new(),
            anim: HashMap::new(),
            swap_flash: HashMap::new(),
            opacity_anim: HashMap::new(),
            snap_mode: false,
            snap_flash: 0,
            monocle: false,
            last_rounded: HashMap::new(),
            last_frame_time: std::time::Instant::now(),
            last_session_save: std::time::Instant::now(),
            shadow_set: HashMap::new(),
            per_app_opacity: HashMap::new(),
            config: crate::config::Config::default(),
            config_reload_counter: 0,
            next_id: 1,
            win_event_hook: HWINEVENTHOOK(std::ptr::null_mut()),
            overview: false,
            overview_positions: Vec::new(),
            gesture_receiver: None,
            gesture_pan_start: None,
            gesture_pan_last: None,
            theme_picker: None,
            session: crate::session::SessionState::load().ok().flatten(),
            scratchpad: None,
            idle_inhibit: false,
            theme_mgr: None,
            ws_fade_anim: crate::anim::SpringValue::new(1.0),
            ws_pending_ws: None,
            ws_pending_monitor: None,
            ws_animating: false,
            tray_windows: HashMap::new(),
            tray_hwnd: None,
        })
    }

    /// Get the monitor index for a window, or default to primary monitor
    pub fn monitor_idx_for_hwnd(&self, hwnd: HWND) -> usize {
        if let Some(&idx) = self.window_monitors.values().find(|&&_| true) {
            // Find the specific window
        }
        if let Some(idx) = self.window_for_hwnd(hwnd).and_then(|info| self.window_monitors.get(&info.id)) {
            return *idx;
        }
        // Fall back to monitor containing the window
        if let Some(m) = self.monitor_for_hwnd(hwnd) {
            self.monitors.iter().position(|mon| mon.handle == m.handle).unwrap_or(0)
        } else {
            0
        }
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    pub fn window_for_hwnd(&self, hwnd: HWND) -> Option<&WindowInfo> {
        self.windows.get(&HWnd(hwnd))
    }

    /// Get mutable reference to the current grid for a specific monitor
    pub fn grid_for_monitor(&mut self, monitor_idx: usize) -> &mut GridState {
        let current = self.monitor_workspaces[monitor_idx].current;
        &mut self.monitor_workspaces[monitor_idx].grids[current]
    }

    /// Get current grid for the monitor containing the focused window
    pub fn current_grid(&mut self) -> &mut GridState {
        let monitor_idx = self.focused_hwnd
            .and_then(|hwnd| self.window_for_hwnd(hwnd.0))
            .and_then(|info| self.window_monitors.get(&info.id))
            .copied()
            .unwrap_or(0);
        self.grid_for_monitor(monitor_idx)
    }

    pub fn switch_workspace(&mut self, ws: usize) {
        let monitor_idx = self.focused_hwnd
            .and_then(|hwnd| self.window_for_hwnd(hwnd.0))
            .and_then(|info| self.window_monitors.get(&info.id))
            .copied()
            .unwrap_or(0);

        if ws >= self.monitor_workspaces[monitor_idx].grids.len()
            || ws == self.monitor_workspaces[monitor_idx].current
        {
            return;
        }

        // Start workspace spring animation
        self.ws_fade_anim.set_target(0.0);
        self.ws_fade_anim.current = 1.0;
        self.ws_fade_anim.velocity = -1.5;
        self.ws_pending_ws = Some(ws);
        self.ws_pending_monitor = Some(monitor_idx);
        self.ws_animating = true;

        info!("Monitor {}: switching to workspace {} (fade)", monitor_idx + 1, ws + 1);
    }

    pub fn move_focused_window_to_workspace(&mut self, ws: usize) {
        let Some(hwnd) = self.focused_hwnd else { return };
        let Some(info) = self.windows.get(&hwnd) else { return };
        let wid = info.id;

        let mon_idx = self.window_monitors.get(&wid).copied().unwrap_or(0);
        if ws >= self.monitor_workspaces[mon_idx].grids.len() { return; }

        let current_ws = self.monitor_workspaces[mon_idx].current;

        // Remove from current workspace grid
        for grid in &mut self.monitor_workspaces[mon_idx].grids {
            grid.cells.retain(|_, v| *v != wid);
            grid.window_positions.remove(&wid);
        }

        // Add to new workspace grid at (0, 0)
        let new_grid = &mut self.monitor_workspaces[mon_idx].grids[ws];
        new_grid.cells.insert(Cell::new(0, 0), wid);
        new_grid.window_positions.insert(wid, Cell::new(0, 0));

        // Update workspace assignment
        self.window_workspaces.insert(wid, ws);

        // If we're currently on that workspace, show the window
        if current_ws == ws {
            unsafe { let _ = ShowWindow(info.hwnd, SW_SHOW); }
        } else {
            unsafe { let _ = ShowWindow(info.hwnd, SW_HIDE); }
        }

        info!("Moved window to workspace {} on monitor {}", ws + 1, mon_idx + 1);
        self.save_session();
    }

    pub fn primary_monitor(&self) -> Option<&MonitorInfo> {
        self.monitors.first()
    }

    pub fn monitor_for_hwnd(&self, hwnd: HWND) -> Option<&MonitorInfo> {
        let mut rect = RECT::default();
        unsafe {
            if GetWindowRect(hwnd, &mut rect).is_ok() {
                let cx = (rect.left + rect.right) / 2;
                let cy = (rect.top + rect.bottom) / 2;
                return self.monitors.iter().find(|m| {
                    cx >= m.left && cx < m.right && cy >= m.top && cy < m.bottom
                });
            }
        }
        self.primary_monitor()
    }

    /// Get DPI for a specific monitor handle
    pub fn get_monitor_dpi(&self, hmonitor: HMONITOR) -> u32 {
        for m in &self.monitors {
            if m.handle == hmonitor {
                return m.dpi;
            }
        }
        unsafe { GetDpiForSystem() }
    }

    /// Handle DPI change notification — re-enumerate monitors and re-tile
    pub fn on_dpi_changed(&mut self) {
        info!("DPI changed, re-enumerating monitors");
        if let Ok(new_monitors) = enumerate_monitors() {
            // Preserve window positions relative to new DPI
            let old_scales: Vec<f32> = self.monitors.iter().map(|m| m.scale_factor).collect();
            let new_scales: Vec<f32> = new_monitors.iter().map(|m| m.scale_factor).collect();

            self.monitors = new_monitors;

            // Re-apply window positions scaled for new DPI
            for (&hwnd_wrapper, info) in &mut self.windows {
                if info.floating {
                    if let (Some(fx), Some(fy), Some(fw), Some(fh)) =
                        (info.float_x, info.float_y, info.float_w, info.float_h) {
                        let mon_idx = self.window_monitors.get(&info.id).copied().unwrap_or(0);
                        let old_scale = old_scales.get(mon_idx).copied().unwrap_or(1.0);
                        let new_scale = self.monitors.get(mon_idx).map(|m| m.scale_factor).unwrap_or(1.0);
                        if old_scale > 0.0 {
                            let ratio = new_scale / old_scale;
                            let new_fw = (fw as f32 * ratio) as u32;
                            let new_fh = (fh as f32 * ratio) as u32;
                            let new_fx = (fx as f32 * ratio) as i32;
                            let new_fy = (fy as f32 * ratio) as i32;
                            info.float_w = Some(new_fw);
                            info.float_h = Some(new_fh);
                            info.float_x = Some(new_fx);
                            info.float_y = Some(new_fy);
                        }
                    }
                }
            }
        }
    }

    pub fn current_work_area(&self) -> (i32, i32, i32, i32) {
        let pad = self.config.layout.outer_padding as i32;
        if let Some(hwnd) = self.focused_hwnd {
            if let Some(mon) = self.monitor_for_hwnd(hwnd.0) {
                return (mon.work_left + pad, mon.work_top + pad, mon.work_right - pad, mon.work_bottom - pad);
            }
        }
        if let Some(m) = self.primary_monitor() {
            return (m.work_left + pad, m.work_top + pad, m.work_right - pad, m.work_bottom - pad);
        }
        (pad, pad, 1920 - pad, 1080 - pad)
    }

    pub fn initialize(&mut self, _config: &crate::config::Config) -> anyhow::Result<()> {
        info!("UltraWM initializing...");

        // Load per-app opacity memory
        self.load_per_app_opacity();

        // Enumerate existing top-level windows
        self.enumerate_windows()?;

        // Restore session: match windows by exe and restore workspace/monitor/position
        if self.session.is_some() {
            self.restore_session();
            self.apply_z_order();
        }

        // Focus the first window if any
        if let Some(first) = self.windows.keys().next().copied() {
            self.on_focus_changed(first.0);
        }

        // Install WinEvent hook for focus tracking + new window creation
        unsafe {
            self.win_event_hook = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_OBJECT_CREATE,
                HINSTANCE(null_mut()),
                Some(win_event_proc),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            );
        }

        // Install WinEvent hook for floating window movement (snap to grid/edges)
        unsafe {
            let _ = SetWinEventHook(
                EVENT_OBJECT_LOCATIONCHANGE,
                EVENT_OBJECT_LOCATIONCHANGE,
                HINSTANCE(null_mut()),
                Some(win_event_proc),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            );
        }

        // Install low-level keyboard hook — must pass platform pointer
        unsafe {
            self.keyboard_hook = Some(KeyboardHook::install(self)?);
        }

        // Create border overlay for focus rings (non-fatal if it fails)
        let overlay_w = self.primary_monitor().map(|m| m.width()).unwrap_or(unsafe { GetSystemMetrics(SM_CXSCREEN) });
        let overlay_h = self.primary_monitor().map(|m| m.height()).unwrap_or(unsafe { GetSystemMetrics(SM_CYSCREEN) });
        match BorderOverlay::create(overlay_w, overlay_h) {
            Ok(mut overlay) => {
                overlay.border_width = self.config.layout.border_width as i32;
                overlay.border_radius = self.config.layout.corner_radius as i32;
                self.border_overlay = Some(overlay);
                unsafe {
                    if let Some(ref mut o) = self.border_overlay {
                        border::BORDER_PTR = o as *mut _;
                    }
                }
                info!("Border overlay created (border_width={}, radius={})", self.config.layout.border_width, self.config.layout.corner_radius);
            }
            Err(e) => {
                warn!("Border overlay creation failed: {}", e);
            }
        }

        // Create AppBar top bar
        let bar_width = self.primary_monitor().map(|m| m.width()).unwrap_or(unsafe { GetSystemMetrics(SM_CXSCREEN) });
        let bar_height = 28i32;
        let bar_bg = 0xFF1E1E2E; // catppuccin base
        let bar_fg = 0xFFCDD6F4; // catppuccin text
        let bar_cfg = &self.config.bar;
        if bar_cfg.enabled {
            match AppBar::create(
                bar_width,
                bar_height,
                bar_bg,
                bar_fg,
                bar_cfg.transparency,
                self.config.layout.workspace_count,
                bar_cfg.show_workspaces,
                bar_cfg.show_clock,
                bar_cfg.show_volume,
                bar_cfg.show_battery,
                bar_cfg.show_cpu,
                bar_cfg.show_memory,
            ) {
                Ok(mut bar) => {
                    // Position bar at top or bottom based on config
                    if bar_cfg.position == "bottom" {
                        if let Some(mon) = self.primary_monitor() {
                            let _ = unsafe {
                                SetWindowPos(
                                    bar.hwnd,
                                    HWND_TOPMOST,
                                    0,
                                    mon.height() - bar_height,
                                    bar_width,
                                    bar_height,
                                    SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                                )
                            };
                        }
                    }
                    self.bar = Some(bar);
                    info!("AppBar created (position={})", bar_cfg.position);
                }
                Err(e) => {
                    warn!("AppBar creation failed: {}", e);
                }
            }
        }

        // Create notifier
        let notif_w = 300i32;
        let notif_h = 36i32;
        match Notifier::create(notif_w, notif_h, 0xFF1E1E2E, 0xFFCDD6F4) {
            Ok(notifier) => {
                let primary = self.primary_monitor().map(|m| (m.width(), m.height())).unwrap_or((1920, 1080));
                notifier.set_position(primary.0 - notif_w - 20, primary.1 - notif_h - 20);
                self.notifier = Some(notifier);
                info!("Notifier created");
            }
            Err(e) => {
                warn!("Notifier creation failed: {}", e);
            }
        }

        // Create system tray icon with context menu
        if let Err(e) = self.create_tray_icon() {
            warn!("Tray icon creation failed: {}", e);
        }

        info!(
            "UltraWM initialized — {} windows managed",
            self.windows.len()
        );

        // Create gesture receiver for touchpad support (non-fatal)
        let gesture_w = self.primary_monitor().map(|m| m.width()).unwrap_or(1920);
        let gesture_h = self.primary_monitor().map(|m| m.height()).unwrap_or(1080);
        match GestureReceiver::create(gesture_w, gesture_h) {
            Ok(_) => {
                info!("Gesture receiver created");
            }
            Err(e) => {
                warn!("Gesture receiver creation failed: {}", e);
            }
        }

        // If running as shell, launch Explorer for taskbar + desktop
        if is_running_as_shell() {
            info!("Running as shell — launching Explorer for taskbar/desktop");
            launch_explorer();
        }

        Ok(())
    }

    pub fn run_event_loop(&mut self, theme_mgr: &mut ThemeManager, ipc_rx: Option<mpsc::Receiver<crate::ipc::IpcCommand>>) -> anyhow::Result<()> {
        info!("UltraWM running — Win+Arrows to navigate");

        let mut msg = MSG::default();
        loop {
            // Process IPC commands (non-blocking)
            if let Some(ref rx) = ipc_rx {
                while let Ok(cmd) = rx.try_recv() {
                    self.handle_ipc_command(cmd, theme_mgr);
                }
            }

            unsafe {
                if GetMessageW(&mut msg, HWND(null_mut()), 0, 0).0 > 0 {
                    // Check for new window creation via WM_CREATE
                    if msg.message == WM_CREATE {
                        let hwnd = msg.hwnd;
                        self.manage_window(hwnd);
                    }

                    // Handle overview click-to-focus
                    if msg.message == border::WM_OVERVIEW_CLICK {
                        let clicked_hwnd = HWND(msg.wParam.0 as *mut _);
                        unsafe {
                            let _ = SetForegroundWindow(clicked_hwnd);
                        }
                        self.overview = false;
                        self.overview_positions.clear();
                    }

                    // Handle window swap from border overlay drag
                    if msg.message == border::WM_SWAP_WINDOWS {
                        let src_hwnd = HWND(msg.wParam.0 as *mut _);
                        let tgt_hwnd = HWND(msg.lParam.0 as *mut _);
                        self.swap_windows(src_hwnd, tgt_hwnd);
                    }

                    // Handle window drag-move from border overlay
                    if msg.message == border::WM_DRAG_MOVE {
                        let src_hwnd = HWND(msg.wParam.0 as *mut _);
                        let tgt_hwnd = HWND(msg.lParam.0 as *mut _);
                        self.drag_move_window(src_hwnd, tgt_hwnd);
                    }

                    // Handle edge tiling from border overlay drag
                    if msg.message == border::WM_EDGE_TILE {
                        let src_hwnd = HWND(msg.wParam.0 as *mut _);
                        let mode_code = msg.lParam.0 as i32;
                        self.edge_tile_window(src_hwnd, mode_code);
                    }

                    // Handle bar workspace indicator click
                    const WM_BAR_WORKSPACE_CLICK: u32 = WM_APP + 0x100;
                    if msg.message == WM_BAR_WORKSPACE_CLICK {
                        let ws_idx = msg.wParam.0 as usize;
                        self.switch_workspace(ws_idx);
                    }

                    // Handle DPI change notification
                    if msg.message == WM_DPICHANGED {
                        self.on_dpi_changed();
                    }

                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                } else {
                    self.save_session();
                    break;
                }
            }

            // Periodic tiling refresh — spring-animated
            let theme = theme_mgr.current_theme();
            let accent = hex_to_rgb(&theme.accent);
            let inactive = hex_to_rgb(&theme.inactive);
            self.tile_all_windows(accent, inactive);

            // Tick notifier
            self.tick_notifier();

            // Workspace switch spring animation
            if self.ws_animating {
                let fade = self.ws_fade_anim.step(1.0 / 60.0).clamp(0.0, 1.0);

                // Apply fade alpha to overlay
                if let Some(ref overlay) = self.border_overlay {
                    let alpha = (fade * 255.0) as u8;
                    overlay.set_alpha(alpha);
                }

                // Check if we've crossed the midpoint (fade out complete)
                if self.ws_fade_anim.current <= 0.05 && self.ws_fade_anim.velocity < 0.0 {
                    // Execute pending workspace switch at the dark point
                    if let (Some(ws), Some(mon)) = (self.ws_pending_ws, self.ws_pending_monitor) {
                        if ws < self.monitor_workspaces[mon].grids.len() {
                            let old_ws = self.monitor_workspaces[mon].current;
                            // Hide non-sticky windows on old workspace
                            for (_, info) in &self.windows {
                                if let Some(wm) = self.window_monitors.get(&info.id) {
                                    if *wm == mon && !info.sticky {
                                        if let Some(ws_id) = self.window_workspaces.get(&info.id) {
                                            if *ws_id == old_ws {
                                                unsafe { let _ = ShowWindow(info.hwnd, SW_HIDE); }
                                            }
                                        }
                                    }
                                }
                            }
                            self.monitor_workspaces[mon].current = ws;
                            self.save_session();
                            // Update bar with workspace names
                            if let Some(ref bar) = self.bar {
                                let names = self.workspace_names(mon);
                                let counts = self.window_count_per_workspace(mon);
                                bar.set_workspaces(names, ws, counts);
                            }
                            // Show non-sticky windows on new workspace
                            for (_, info) in &self.windows {
                                if let Some(wm) = self.window_monitors.get(&info.id) {
                                    if *wm == mon && !info.sticky {
                                        if let Some(ws_id) = self.window_workspaces.get(&info.id) {
                                            if *ws_id == ws {
                                                unsafe { let _ = ShowWindow(info.hwnd, SW_SHOW); }
                                            }
                                        }
                                    }
                                }
                            }
                            info!("Monitor {}: switched to workspace {}", mon + 1, ws + 1);
                            self.notify(&format!("Workspace {}", ws + 1));
                        }
                        self.ws_pending_ws = None;
                        self.ws_pending_monitor = None;
                    }
                    // Now animate back to visible
                    self.ws_fade_anim.set_target(1.0);
                    self.ws_fade_anim.velocity = 2.0;
                }

                // Check if animation is fully settled
                if self.ws_fade_anim.is_settled() && self.ws_fade_anim.target == 1.0 {
                    self.ws_animating = false;
                    self.ws_fade_anim.current = 1.0;
                    if let Some(ref overlay) = self.border_overlay {
                        overlay.set_alpha(255);
                    }
                }
            }

            // Update bar system info (clock, CPU, memory)
            self.update_bar_system_info();

            // Hot-reload config every ~1 second
            self.config_reload_counter += 1;
            if self.config_reload_counter >= 60 {
                self.config_reload_counter = 0;
                if let Ok(Some(new_config)) = self.config.reload_if_changed() {
                    let old = self.config.clone();
                    self.config = new_config;

                    // Re-parse keybinds from updated config
                    self.keybinds = keybinds::parse_keybinds(&self.config.keybinds);

                    // Flash bar to indicate config reloaded
                    if let Some(ref bar) = self.bar {
                        bar.trigger_reload_flash();
                    }

                    // Compute and apply config diff
                    let mut changes = Vec::new();
                    let layout = &old.layout;
                    let new_layout = &self.config.layout;

                    if layout.gaps != new_layout.gaps {
                        changes.push(format!("gaps: {} → {}", layout.gaps, new_layout.gaps));
                    }
                    if layout.inner_padding != new_layout.inner_padding {
                        changes.push(format!("inner_padding: {} → {}", layout.inner_padding, new_layout.inner_padding));
                    }
                    if layout.peek_x != new_layout.peek_x {
                        changes.push(format!("peek_x: {} → {}", layout.peek_x, new_layout.peek_x));
                    }
                    if layout.peek_y != new_layout.peek_y {
                        changes.push(format!("peek_y: {} → {}", layout.peek_y, new_layout.peek_y));
                    }
                    if layout.border_width != new_layout.border_width {
                        changes.push(format!("border_width: {} → {}", layout.border_width, new_layout.border_width));
                    }
                    if layout.corner_radius != new_layout.corner_radius {
                        changes.push(format!("corner_radius: {} → {}", layout.corner_radius, new_layout.corner_radius));
                    }
                    if layout.window_opacity != new_layout.window_opacity {
                        changes.push(format!("window_opacity: {:.2} → {:.2}", layout.window_opacity, new_layout.window_opacity));
                    }
                    if layout.workspace_count != new_layout.workspace_count {
                        changes.push(format!("workspace_count: {} → {}", layout.workspace_count, new_layout.workspace_count));
                    }
                    if layout.default_float_width != new_layout.default_float_width {
                        changes.push(format!("default_float_width: {} → {}", layout.default_float_width, new_layout.default_float_width));
                    }
                    if layout.default_float_height != new_layout.default_float_height {
                        changes.push(format!("default_float_height: {} → {}", layout.default_float_height, new_layout.default_float_height));
                    }
                    if layout.snap_grid_size != new_layout.snap_grid_size {
                        changes.push(format!("snap_grid_size: {} → {}", layout.snap_grid_size, new_layout.snap_grid_size));
                    }
                    if layout.snap_edge_distance != new_layout.snap_edge_distance {
                        changes.push(format!("snap_edge_distance: {} → {}", layout.snap_edge_distance, new_layout.snap_edge_distance));
                    }
                    if layout.rounded_corners != new_layout.rounded_corners {
                        changes.push(format!("rounded_corners: {} → {}", layout.rounded_corners, new_layout.rounded_corners));
                    }
                    if layout.dwm_shadows != new_layout.dwm_shadows {
                        changes.push(format!("dwm_shadows: {} → {}", layout.dwm_shadows, new_layout.dwm_shadows));
                    }
                    if layout.center_focused != new_layout.center_focused {
                        changes.push(format!("center_focused: {} → {}", layout.center_focused, new_layout.center_focused));
                    }
                    if layout.focus_follows_mouse != new_layout.focus_follows_mouse {
                        changes.push(format!("focus_follows_mouse: {} → {}", layout.focus_follows_mouse, new_layout.focus_follows_mouse));
                    }
                    if layout.auto_split != new_layout.auto_split {
                        changes.push(format!("auto_split: {} → {}", layout.auto_split, new_layout.auto_split));
                    }
                    if layout.default_split_dir != new_layout.default_split_dir {
                        changes.push(format!("default_split_dir: {} → {}", layout.default_split_dir, new_layout.default_split_dir));
                    }
                    if layout.resize_step_px != new_layout.resize_step_px {
                        changes.push(format!("resize_step_px: {} → {}", layout.resize_step_px, new_layout.resize_step_px));
                    }
                    if layout.session_auto_save_interval != new_layout.session_auto_save_interval {
                        changes.push(format!("session_auto_save_interval: {} → {}", layout.session_auto_save_interval, new_layout.session_auto_save_interval));
                    }
                    if layout.spring_stiffness != new_layout.spring_stiffness {
                        changes.push(format!("spring_stiffness: {:.1} → {:.1}", layout.spring_stiffness, new_layout.spring_stiffness));
                    }
                    if layout.spring_damping != new_layout.spring_damping {
                        changes.push(format!("spring_damping: {:.1} → {:.1}", layout.spring_damping, new_layout.spring_damping));
                    }

                    // Apply layout changes to grid
                    if layout.gaps != new_layout.gaps
                        || layout.inner_padding != new_layout.inner_padding
                        || layout.peek_x != new_layout.peek_x
                        || layout.peek_y != new_layout.peek_y
                    {
                        let grid = self.current_grid();
                        grid.apply_layout_config(new_layout.gaps, new_layout.inner_padding, new_layout.peek_x, new_layout.peek_y);
                    }

                    // Update border overlay
                    if let Some(ref mut overlay) = self.border_overlay {
                        overlay.border_width = new_layout.border_width as i32;
                        overlay.border_radius = new_layout.corner_radius as i32;
                    }

                    // Re-tile to apply all layout changes
                    self.tile_all_windows(0xFF7F7F7F, 0xFF454545);

                    if !changes.is_empty() {
                        let change_str = changes.join("; ");
                        info!("Config reloaded ({} changes): {}", changes.len(), change_str);
                        if let Some(ref notifier) = self.notifier {
                            let _ = notifier.show(&format!("Config reloaded: {}", changes.len()));
                        }
                    } else {
                        debug!("Config reloaded: no changes detected");
                    }
                }
            }

            // Auto-save session at configured interval
            let auto_save_secs = self.config.layout.session_auto_save_interval;
            if auto_save_secs > 0 && self.last_session_save.elapsed().as_secs() >= auto_save_secs as u64 {
                self.save_session();
                self.last_session_save = std::time::Instant::now();
            }

            // Update bar with focused window title, color, and clock
            if let Some(ref bar) = self.bar {
                let (title, title_color) = self
                    .focused_hwnd
                    .and_then(|hwnd| self.windows.get(&hwnd))
                    .map(|info| {
                        let color = exe_hash_color(&info.exe);
                        (info.title.clone(), color)
                    })
                    .unwrap_or_else(|| {
                        let accent = hex_to_rgb(&theme_mgr.current_theme().accent);
                        (String::new(), accent)
                    });
                bar.set_title(&title);
                bar.set_title_color(title_color);

                // Update snap mode indicator
                bar.set_snap_mode(self.snap_mode);
                bar.set_monocle_mode(self.monocle);

                // Update network status
                bar.set_network(crate::platform::bar::is_network_online());

                // Update clock every second
                let now = chrono::Local::now();
                let clock = now.format("%H:%M").to_string();
                bar.set_clock(&clock);

                // Update battery every 10 seconds
                if self.config_reload_counter % 600 == 0 {
                    let bat = crate::platform::bar::get_battery_level();
                    bar.set_battery(bat);
                }

                // Update volume every 5 seconds
                if self.config_reload_counter % 300 == 0 {
                    let vol = crate::platform::bar::get_volume_level();
                    bar.set_volume(vol);
                }
            }

            // Focus-follows-mouse check (every frame if enabled)
            self.focus_follows_mouse_check();

            // Decay swap flash timers
            self.swap_flash.retain(|_, timer| {
                *timer = timer.saturating_sub(1);
                *timer > 0
            });

            // Decay snap flash timer
            if self.snap_flash > 0 {
                self.snap_flash = self.snap_flash.saturating_sub(1);
                if self.snap_flash == 0 && !self.snap_mode {
                    // clean
                }
            }

            // Vsync-aligned wait (LeopardWM-proven DwmFlush)
            unsafe {
                let _ = DwmFlush();
            }
        }
        Ok(())
    }

    pub fn enumerate_windows(&mut self) -> anyhow::Result<()> {
        unsafe {
            let _ = EnumWindows(
                Some(enum_windows_proc),
                LPARAM(self as *mut Self as isize),
            );
        }
        Ok(())
    }

    pub fn focus_follows_mouse_check(&mut self) {
        if !self.config.layout.focus_follows_mouse {
            return;
        }
        unsafe {
            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);
            let hwnd = WindowFromPoint(pt);
            if hwnd.is_invalid() {
                return;
            }
            let hwnd_wrapper = HWnd(hwnd);
            if self.focused_hwnd == Some(hwnd_wrapper) {
                return;
            }
            if let Some(info) = self.windows.get(&hwnd_wrapper) {
                if info.id > 0 {
                    let wid = info.id;
                    let _ = SetForegroundWindow(hwnd);
                    self.focused_hwnd = Some(hwnd_wrapper);
                    let grid = self.current_grid();
                    grid.focus_window(wid);
                }
            }
        }
    }

    pub fn tile_all_windows(&mut self, accent_rgb: u32, inactive_rgb: u32) {
        let mut border_rects: Vec<(i32, i32, i32, i32, u32, bool, bool, Option<String>, u32)> = Vec::new();
        let mut tile_rects: Vec<(i32, i32, i32, i32, HWND)> = Vec::new();

        // Overview mode: compact grid of all windows
        if self.overview {
            self.tile_overview(accent_rgb, inactive_rgb, &mut border_rects);
            if let Some(ref mut overlay) = self.border_overlay {
                overlay.update(&border_rects);
            }
            return;
        }

        // Monocle mode: focused window takes full viewport
        if self.monocle {
            if let Some(hwnd_wrapper) = self.focused_hwnd {
                if let Some(info) = self.windows.get(&hwnd_wrapper) {
                    let wid = info.id;
                    let mon_idx = self.window_monitors.get(&wid).copied().unwrap_or(0);
                    let mws = &self.monitor_workspaces[mon_idx];
                    let monitor = &mws.monitor;
                    let (vw, vh) = (monitor.work_width(), monitor.work_height());

                    // Hide all non-focused windows
                    for (&hwnd, win_info) in &self.windows {
                        if win_info.id != wid && !win_info.minimized {
                            unsafe {
                                let _ = ShowWindow(hwnd.0, SW_HIDE);
                            }
                        }
                    }

                    // Show and position the focused window
                    unsafe {
                        let _ = ShowWindow(hwnd_wrapper.0, SW_SHOW);
                        let _ = SetWindowPos(
                            hwnd_wrapper.0,
                            HWND_TOP,
                            monitor.work_left,
                            monitor.work_top,
                            vw,
                            vh,
                            SWP_SHOWWINDOW | SWP_FRAMECHANGED,
                        );
                    }

                    // Draw full-viewport border
                    let is_focused = true;
                    let color = accent_rgb;
                    let title = info.title.clone();
                    border_rects.push((
                        monitor.work_left,
                        monitor.work_top,
                        vw,
                        vh,
                        color,
                        is_focused,
                        info.floating,
                        Some(title),
                        self.window_border_widths.get(&info.id).copied().unwrap_or(self.config.layout.border_width),
                    ));

                    if let Some(ref mut overlay) = self.border_overlay {
                        overlay.update(&border_rects);
                    }
                    return;
                }
            }
            return;
        }

        // Pre-compute monitor assignments to avoid borrow conflicts
        // Clone monitor handles for position-based assignment
        let mon_handles: Vec<_> = self.monitors.iter().map(|m| m.handle).collect();

        // Build a temporary map of wid -> mon_idx for this pass
        let mut window_mon_map: HashMap<u64, usize> = HashMap::new();
        for (&hwnd_wrapper, info) in &self.windows {
            if let Some(&wm) = self.window_monitors.get(&info.id) {
                window_mon_map.insert(info.id, wm);
            } else {
                let mut pt = POINT::default();
                unsafe {
                    let _ = GetCursorPos(&mut pt);
                    let mon = MonitorFromPoint(pt, MONITOR_DEFAULTTONULL);
                    if let Some(m_idx) = mon_handles.iter().position(|&h| h == mon) {
                        window_mon_map.insert(info.id, m_idx);
                    } else {
                        window_mon_map.insert(info.id, 0);
                    }
                }
            }
        }

        // Remove dead windows
        let mut dead_wids = Vec::new();
        for (&hwnd_wrapper, info) in &self.windows {
            if !is_window_alive(hwnd_wrapper.0) {
                dead_wids.push(info.id);
            }
        }
        for wid in dead_wids {
            self.remove_window_by_id(wid);
        }

        // Get focused hwnd for border coloring
        let focused_hwnd = self.focused_hwnd;

        // Tile windows per-monitor
        for (mon_idx, mws) in self.monitor_workspaces.iter_mut().enumerate() {
            let (vw, vh) = (mws.monitor.work_width(), mws.monitor.work_height());
            let grid = &mut mws.grids[mws.current];

            for (&hwnd_wrapper, info) in &self.windows {
                // Only tile windows belonging to this monitor
                if let Some(&wm) = window_mon_map.get(&info.id) {
                    if wm != mon_idx {
                        continue;
                    }
                } else {
                    continue;
                }

                // Skip floating windows
                if info.floating {
                    continue;
                }

                // Skip floating, cloaked, hidden, or minimized windows
                if !info.visible || info.minimized {
                    continue;
                }

                let wid = info.id;
                if let Some((x, y, w, h)) = Self::position_for_hwnd(wid, grid, vw, vh, mws.monitor.work_left, mws.monitor.work_top) {
                    let target_x = x as f32;
                    let target_y = y as f32;
                    let target_w = w as f32;
                    let target_h = h as f32;

                    let anim = self.anim.entry(wid).or_insert_with(|| {
                        // Try to get current window position for smooth animation
                        let (start_x, start_y, start_w, start_h) = if let Ok(rect) = get_window_rect(info.hwnd) {
                            (rect.0 as f32, rect.1 as f32, rect.2 as f32, rect.3 as f32)
                        } else {
                            (target_x, target_y, target_w, target_h)
                        };
                        WindowAnimState::new(
                            start_x, start_y, start_w, start_h,
                            self.config.layout.spring_stiffness,
                            self.config.layout.spring_damping,
                        )
                    });
                    anim.set_target(target_x, target_y, target_w, target_h);
                    let dt: f32 = (1.0f32 / 60.0).min(1.0f32 / 30.0);
                    let (ax, ay, aw, ah) = anim.step(dt);

                    let pad = self.config.layout.inner_padding as i32;
                    let px = ax as i32 + pad;
                    let py = ay as i32 + pad;
                    let mut pw = (aw as i32 - pad * 2).max(pad * 2);
                    let mut ph = (ah as i32 - pad * 2).max(pad * 2);

                    // Enforce max size constraints from rules
                    if let Some(mw) = info.max_width {
                        pw = pw.min(mw as i32);
                    }
                    if let Some(mh) = info.max_height {
                        ph = ph.min(mh as i32);
                    }
                    // Enforce min size constraints from rules
                    if let Some(mw) = info.min_width {
                        pw = pw.max(mw as i32);
                    }
                    if let Some(mh) = info.min_height {
                        ph = ph.max(mh as i32);
                    }

                    unsafe {
                        let _ = SetWindowPos(
                            hwnd_wrapper.0,
                            HWND(null_mut()),
                            px,
                            py,
                            pw,
                            ph,
                            SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                        );
                    }

                    // Collect rect for mouse resize detection
                    tile_rects.push((px, py, pw, ph, hwnd_wrapper.0));

                    // Enable DWM shadow once per window
                    if !self.shadow_set.get(&wid).copied().unwrap_or(false) {
                        enable_dwm_shadow(hwnd_wrapper.0);
                        self.shadow_set.insert(wid, true);
                    }

                    // Apply rounded corners when size changes
                    let pad = self.config.layout.inner_padding as i32;
                    let radius = (self.config.layout.corner_radius as i32).saturating_sub(pad);
                    let cur_w = if pad == 0 { aw as i32 } else { pw };
                    let cur_h = if pad == 0 { ah as i32 } else { ph };
                    let prev = self.last_rounded.get(&wid);
                    if prev.map_or(true, |&(pw, ph, pr)| pw != cur_w || ph != cur_h || pr != radius) {
                        let rx = if pad == 0 { ax as i32 } else { px };
                        let ry = if pad == 0 { ay as i32 } else { py };
                        let rw = if pad == 0 { aw as i32 } else { pw };
                        let rh = if pad == 0 { ah as i32 } else { ph };
                        apply_rounded_corners(hwnd_wrapper.0, rx, ry, rw, rh, radius);
                        self.last_rounded.insert(wid, (cur_w, cur_h, radius));
                    }

                    let is_focused = focused_hwnd == Some(hwnd_wrapper);
                    let flash = self.swap_flash.get(&wid).copied().unwrap_or(0);
                    let base_color = if is_focused { info.border_color } else { dim_color(info.border_color, 0.5) };
                    let color = if flash > 0 {
                        // Flash white during swap animation, fading with timer
                        let alpha = flash.min(35) as f32 / 35.0;
                        blend_color(0xFFFFFFFF, base_color, alpha * 0.8 + 0.2)
                    } else if self.snap_mode && is_focused && self.snap_flash > 0 {
                        // Cyan flash for snap mode
                        let alpha = self.snap_flash.min(30) as f32 / 30.0;
                        blend_color(0xFF00FFFF, base_color, alpha * 0.7 + 0.3)
                    } else {
                        base_color
                    };
                    let title = get_window_title(hwnd_wrapper.0);
                    let bw = self.window_border_widths.get(&info.id).copied().unwrap_or(self.config.layout.border_width);
                    border_rects.push((ax as i32, ay as i32, aw as i32, ah as i32, color, is_focused, info.floating, title, bw as i32));

                    // Apply DWM blur to windows
                    let _ = enable_blur(hwnd_wrapper.0, accent_rgb);

                    // Apply per-window opacity (with animation support)
                    let final_op = if let Some(spring) = self.opacity_anim.get_mut(&info.id) {
                        spring.step(1.0 / 60.0).clamp(0.0, 1.0)
                    } else {
                        info.opacity.unwrap_or(1.0)
                    };
                    unsafe {
                        let _ = SetLayeredWindowAttributes(hwnd_wrapper.0, COLORREF(0), (final_op * 255.0) as u8, LWA_ALPHA);
                    }
                }
            }
        }

        // Update border overlay
        if let Some(ref mut overlay) = self.border_overlay {
            overlay.tile_rects = tile_rects;
            overlay.update(&border_rects);
        }

        // Clean up settled opacity animations
        self.opacity_anim.retain(|_, spring| !spring.is_settled());
    }

    fn tile_overview(
        &mut self,
        accent_rgb: u32,
        inactive_rgb: u32,
        border_rects: &mut Vec<(i32, i32, i32, i32, u32, bool, bool, Option<String>, u32)>,
    ) {
        let (wl, wt, wr, wb) = self.current_work_area();
        let vw = wr - wl;
        let vh = wb - wt;

        // Collect visible, non-floating windows
        let mut windows: Vec<(u64, HWND, WindowInfo)> = Vec::new();
        for (&hwnd_wrapper, info) in &self.windows {
            if info.floating || !info.visible || !is_window_alive(hwnd_wrapper.0) {
                continue;
            }
            windows.push((info.id, hwnd_wrapper.0, info.clone()));
        }

        if windows.is_empty() {
            self.overview_positions.clear();
            return;
        }

        // Compact grid: 200x150 per window, up to 4 per row
        let cols = (vw / 220).max(1).min(windows.len() as i32);
        let rows = (windows.len() as f32 / cols as f32).ceil() as i32;
        let cell_w = (vw / cols).min(300);
        let cell_h = (vh / rows).min(200);
        let gap = 8;

        self.overview_positions.clear();
        for (i, (wid, hwnd, _info)) in windows.iter().enumerate() {
            let row = (i / cols as usize) as i32;
            let col = (i % cols as usize) as i32;
            let x = wl + col * (cell_w + gap) + (vw - cols * (cell_w + gap)) / 2;
            let y = wt + row * (cell_h + gap) + (vh - rows * (cell_h + gap)) / 2;

            let pad = self.config.layout.inner_padding as i32;
            let px = x + pad;
            let py = y + pad;
            let pw = (cell_w - pad * 2).max(pad * 2);
            let ph = (cell_h - pad * 2).max(pad * 2);

            unsafe {
                let _ = SetWindowPos(
                    *hwnd,
                    HWND(null_mut()),
                    px,
                    py,
                    pw,
                    ph,
                    SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
            }

            let is_focused = self.focused_hwnd == Some(HWnd(*hwnd));
            let color = if is_focused { accent_rgb } else { inactive_rgb };
            let title = get_window_title(*hwnd);
            let hwnd_wrapper = HWnd(*hwnd);
            let floating = self.windows.get(&hwnd_wrapper).map(|i| i.floating).unwrap_or(false);
            let bw = self.window_border_widths.get(&wid).copied().unwrap_or(self.config.layout.border_width);
            border_rects.push((x, y, cell_w, cell_h, color, is_focused, floating, title, bw as i32));
            self.overview_positions.push((x, y, cell_w, cell_h, hwnd_wrapper));
        }
    }

    fn position_for_hwnd(
        wid: u64,
        grid: &GridState,
        vw: i32,
        vh: i32,
        work_left: i32,
        work_top: i32,
    ) -> Option<(i32, i32, u32, u32)> {
        let cell = grid.window_positions.get(&wid)?;
        let (x, y, w, h) = grid.cell_rect(*cell, vw, vh);
        Some((x + work_left, y + work_top, w, h))
    }

    pub fn manage_window(&mut self, hwnd: HWND) {
        // Skip if already managed
        if self.windows.contains_key(&HWnd(hwnd)) {
            return;
        }

        if let Ok(info) = WindowInfo::from_hwnd(hwnd) {
            if info.should_tile() {
                let wid = self.next_id;
                self.next_id += 1;
                let hwnd_wrapper = HWnd(hwnd);
                let title = info.title.clone();
                let exe = info.exe.clone();
                let mut win_info = info;
                win_info.id = wid;
                win_info.border_color = exe_hash_color(&win_info.exe);

                // Apply per-app rules before inserting
                let rule_info = win_info.clone();
                self.apply_rules(&mut win_info, &rule_info);

                // Determine which monitor this window belongs to
                let mon_idx = if let Some(mon) = self.monitor_for_hwnd(hwnd) {
                    self.monitors.iter().position(|m| m.handle == mon.handle).unwrap_or(0)
                } else {
                    0
                };
                self.window_monitors.insert(wid, mon_idx);

                // Assign workspace (rule overrides, else current workspace on that monitor)
                let default_ws = self.monitor_workspaces[mon_idx].current;
                let ws = self.window_workspaces.get(&wid).copied().unwrap_or(default_ws);
                self.window_workspaces.insert(wid, ws);

                self.windows.insert(hwnd_wrapper, win_info);

                let grid = &mut self.monitor_workspaces[mon_idx].grids[ws];
                {
                    // Auto-split: if enabled, split focused window instead of placing on grid
                    if self.config.layout.auto_split {
                        if let Some(focused_wid) = grid.focused_window {
                            if focused_wid != wid {
                                let dir = if self.config.layout.default_split_dir == "horizontal" {
                                    crate::layout::SplitDir::Horizontal
                                } else {
                                    crate::layout::SplitDir::Vertical
                                };
                                let _ = grid.split_cell(focused_wid, dir);
                                grid.focus_window(wid);
                            }
                        }
                    }
                    // Restore state from session if available (search all monitors/workspaces)
                    let mut session_cell: Option<crate::layout::Cell> = None;
                    let mut session_floating = false;
                    if let Some(ref session) = self.session {
                        for mstate in &session.monitors {
                            for gs in &mstate.grids {
                                for sw in &gs.windows {
                                    if sw.exe == exe {
                                        session_cell = Some(sw.cell);
                                        session_floating = sw.floating;
                                        // Restore window properties
                                        if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                                            info.floating = sw.floating;
                                            info.opacity = sw.opacity;
                                            info.sticky = sw.sticky;
                                            info.maximized = sw.maximized;
                                            info.always_on_top = sw.always_on_top;
                                            info.z_order = sw.z_order;
                                            if sw.float_x.is_some() { info.float_x = sw.float_x; }
                                            if sw.float_y.is_some() { info.float_y = sw.float_y; }
                                            if sw.float_w.is_some() { info.float_w = sw.float_w.map(|v| v as u32); }
                                            if sw.float_h.is_some() { info.float_h = sw.float_h.map(|v| v as u32); }
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    grid.place_window(wid);

                    // Apply session cell position if found
                    if let Some(cell) = session_cell {
                        grid.window_positions.insert(wid, cell);
                        grid.cells.insert(cell, wid);
                    }

                    // If session says floating, remove from grid and position manually
                    if session_floating {
                        grid.remove_window(wid);
                        if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                            unsafe {
                                let _ = SetWindowPos(
                                    info.hwnd,
                                    if info.always_on_top { HWND_TOPMOST } else { HWND_NOTOPMOST },
                                    info.saved_x, info.saved_y, info.saved_w, info.saved_h,
                                    SWP_FRAMECHANGED,
                                );
                            }
                        }
                    }

                    // Enforce rule-based floating: position window and remove from grid
                    if let Some(info) = self.windows.get(&hwnd_wrapper) {
                        if info.floating && grid.window_positions.contains_key(&wid) {
                            grid.remove_window(wid);
                            unsafe {
                                let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL);
                                if !mon.is_invalid() {
                                    let mut mi = MONITORINFO {
                                        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                                        ..Default::default()
                                    };
                                    if GetMonitorInfoW(mon, &mut mi).as_bool() {
                                        let fw = info.float_w.unwrap_or(self.config.layout.default_float_width).min((mi.rcWork.right - mi.rcWork.left) as u32);
                                        let fh = info.float_h.unwrap_or(self.config.layout.default_float_height).min((mi.rcWork.bottom - mi.rcWork.top) as u32);
                                        let fx = info.float_x.unwrap_or_else(|| mi.rcWork.left + ((mi.rcWork.right - mi.rcWork.left) - fw as i32) / 2);
                                        let fy = info.float_y.unwrap_or_else(|| mi.rcWork.top + ((mi.rcWork.bottom - mi.rcWork.top) - fh as i32) / 2);

                                        if let Some(mut_info) = self.windows.get_mut(&hwnd_wrapper) {
                                            mut_info.saved_x = fx;
                                            mut_info.saved_y = fy;
                                            mut_info.saved_w = fw as i32;
                                            mut_info.saved_h = fh as i32;
                                        }

                                        let _ = SetWindowPos(
                                            hwnd,
                                            HWND_TOPMOST,
                                            fx, fy, fw as i32, fh as i32,
                                            SWP_FRAMECHANGED,
                                        );
                                    }
                                }
                            }
                            info!("Window {} floated by rule", wid);
                        }
                    }

                    grid.focus_window(wid);
                }

                // Apply visual effects
                let cfg = &self.config.layout;
                if cfg.rounded_corners && cfg.corner_radius > 0 {
                    self.apply_rounded_corners(hwnd, cfg.corner_radius);
                }
                if cfg.dwm_shadows {
                    self.apply_dwm_shadow(hwnd);
                }
                if cfg.window_opacity < 1.0 {
                    self.apply_window_opacity(hwnd, cfg.window_opacity);
                }
                // Apply per-app opacity if remembered
                if !info.opacity.is_some_and(|o| o < 1.0) {
                    self.apply_per_app_opacity(hwnd, &info.exe);
                }

                info!("Managed window: {} (id={})", title, wid);
                self.tile_all_windows(0xFF7F7F7F, 0xFF454545);
            }
        }
    }

    /// Split the focused window's cell horizontally or vertically
    pub fn split_focused(&mut self, horizontal: bool) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get(&hwnd_wrapper) {
                let wid = info.id;
                if wid > 0 {
                    let dir = if horizontal { crate::layout::SplitDir::Horizontal } else { crate::layout::SplitDir::Vertical };
                    let grid = self.current_grid();
                    if grid.split_cell(wid, dir) {
                        info!("Split cell {} {:?}", wid, dir);
                        self.tile_all_windows(0xFF7F7F7F, 0xFF454545);
                    }
                }
            }
        }
    }

    /// Unsplit the focused window's cell (merge children)
    pub fn unsplit_focused(&mut self) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get(&hwnd_wrapper) {
                let wid = info.id;
                if wid > 0 {
                    let grid = self.current_grid();
                    if grid.unsplit_cell(wid) {
                        info!("Unsplit cell for window {}", wid);
                        self.tile_all_windows(0xFF7F7F7F, 0xFF454545);
                    }
                }
            }
        }
    }

    /// Adjust the split ratio for the focused window
    pub fn adjust_split(&mut self, grow: bool) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get(&hwnd_wrapper) {
                let wid = info.id;
                if wid > 0 {
                    let grid = self.current_grid();
                    grid.adjust_split_ratio(wid, grow);
                    self.tile_all_windows(0xFF7F7F7F, 0xFF454545);
                }
            }
        }
    }

    /// Tab the focused window with another window in the same cell
    pub fn tab_focused(&mut self) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get(&hwnd_wrapper) {
                let wid = info.id;
                if wid > 0 {
                    let grid = self.current_grid();
                    // Find another window in the same cell
                    if let Some(&cell) = grid.window_positions.get(&wid) {
                        if let Some(&other_wid) = grid.cells.get(&cell) {
                            if other_wid != wid {
                                if grid.tab_cell(wid, other_wid) {
                                    info!("Tabbed {} with {}", wid, other_wid);
                                    self.tile_all_windows(0xFF7F7F7F, 0xFF454545);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Untab the focused window's cell
    pub fn untab_focused(&mut self) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get(&hwnd_wrapper) {
                let wid = info.id;
                if wid > 0 {
                    let grid = self.current_grid();
                    if grid.untab_cell(wid) {
                        info!("Untabbed cell for window {}", wid);
                        self.tile_all_windows(0xFF7F7F7F, 0xFF454545);
                    }
                }
            }
        }
    }

    /// Cycle through tabs in the focused cell
    pub fn cycle_tab(&mut self, forward: bool) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get(&hwnd_wrapper) {
                let wid = info.id;
                if wid > 0 {
                    let grid = self.current_grid();
                    if grid.cycle_tab(wid, forward) {
                        self.tile_all_windows(0xFF7F7F7F, 0xFF454545);
                    }
                }
            }
        }
    }

    pub fn on_focus_changed(&mut self, hwnd: HWND) {
        let old_mon = self.focused_hwnd
            .and_then(|hw| self.window_for_hwnd(hw.0))
            .and_then(|info| self.window_monitors.get(&info.id))
            .copied();

        self.focused_hwnd = Some(HWnd(hwnd));

        // Bring floating windows to top of z-order when focused
        if let Some(info) = self.windows.get(&HWnd(hwnd)) {
            if info.floating {
                unsafe {
                    let _ = SetWindowPos(hwnd, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                }
            }
        }

        let wid = self.windows.get(&HWnd(hwnd)).map(|i| i.id).unwrap_or(0);
        if wid > 0 {
            // Monocle mode: show focused window, hide others
            if self.monocle {
                if let Some(old_hwnd) = self.focused_hwnd {
                    if old_hwnd != HWnd(hwnd) {
                        // Hide old focused window
                        unsafe {
                            let _ = ShowWindow(old_hwnd.0, SW_HIDE);
                        }
                    }
                }
                // Show and position new focused window
                if let Some(info) = self.windows.get(&HWnd(hwnd)) {
                    let mon_idx = self.window_monitors.get(&wid).copied().unwrap_or(0);
                    let mws = &self.monitor_workspaces[mon_idx];
                    let monitor = &mws.monitor;
                    let (vw, vh) = (monitor.work_width(), monitor.work_height());
                    unsafe {
                        let _ = ShowWindow(hwnd, SW_SHOW);
                        let _ = SetWindowPos(
                            hwnd,
                            HWND_TOP,
                            monitor.work_left,
                            monitor.work_top,
                            vw,
                            vh,
                            SWP_SHOWWINDOW | SWP_FRAMECHANGED,
                        );
                    }
                }
            }

            // Animate opacity: dim old focused, brighten new focused
            let old_focused = self.focused_hwnd;
            if let Some(old_hwnd) = old_focused {
                if old_hwnd != HWnd(hwnd) {
                    if let Some(old_info) = self.windows.get(&old_hwnd) {
                        let old_wid = old_info.id;
                        let base_opacity = old_info.opacity.unwrap_or(1.0);
                        let target = (base_opacity * 0.7).max(0.3);
                        self.opacity_anim.entry(old_wid).or_insert_with(|| {
                            crate::anim::SpringValue::new(base_opacity).with_spring(crate::anim::Spring {
                                stiffness: 200.0,
                                damping: 25.0,
                                mass: 1.0,
                            })
                        }).set_target(target);
                    }
                }
            }
            let new_info = self.windows.get(&HWnd(hwnd));
            if let Some(info) = new_info {
                let base_opacity = info.opacity.unwrap_or(1.0);
                self.opacity_anim.entry(wid).or_insert_with(|| {
                    crate::anim::SpringValue::new(base_opacity).with_spring(crate::anim::Spring {
                        stiffness: 200.0,
                        damping: 25.0,
                        mass: 1.0,
                    })
                }).set_target(1.0);
            }

            self.trigger_focus_flash(wid);
            let grid = self.current_grid();
            grid.focus_window(wid);
        }

        // Update bar workspace indicators if focus moved to a different monitor
        let new_mon = self.windows.get(&HWnd(hwnd))
            .and_then(|info| self.window_monitors.get(&info.id))
            .copied();
        if old_mon != new_mon {
            if let (Some(mon_idx), Some(ref bar)) = (new_mon, self.bar.as_ref()) {
                let names = self.workspace_names(mon_idx);
                let current = self.monitor_workspaces[mon_idx].current;
                let counts = self.window_count_per_workspace(mon_idx);
                bar.set_workspaces(names, current, counts);
            }
        }
    }

    /// Remove a window by its internal ID (for dead window cleanup)
    pub fn remove_window_by_id(&mut self, wid: u64) {
        let (was_focused, info_clone, hwnd_wrapper) = {
            let hw = self.windows.iter().find(|(_, i)| i.id == wid).map(|(hw, _)| *hw);
            let wf = self.focused_hwnd == hw;
            let info = self.windows.values().find(|i| i.id == wid).cloned();
            (wf, info, hw)
        };

        if let Some(info) = info_clone {
            let hwnd = info.hwnd;
            self.windows.remove(&HWnd(hwnd));

            let grid = self.current_grid();
            grid.remove_window(info.id);

            let next_hwnd = if was_focused {
                find_nearest_window(&grid, info.id)
                    .and_then(|nwid| self.windows.iter().find(|(_, wi)| wi.id == nwid).map(|(hw, _)| *hw))
            } else {
                None
            };

            if let Some(hw) = next_hwnd {
                unsafe {
                    let _ = SetForegroundWindow(hw.0);
                }
                self.focused_hwnd = Some(hw);
            } else if was_focused {
                self.focused_hwnd = None;
            }

            self.window_workspaces.remove(&info.id);
            self.anim.remove(&info.id);
            debug!("Removed dead window: {} (id={})", info.title, info.id);
        }
    }

    pub fn remove_window(&mut self, hwnd: HWND) {
        let hwnd_wrapper = HWnd(hwnd);
        let was_focused = self.focused_hwnd == Some(hwnd_wrapper);

        if let Some(info) = self.windows.remove(&hwnd_wrapper) {
            let grid = self.current_grid();
            grid.remove_window(info.id);

            // Auto-focus neighbor when focused window is closed
            if was_focused {
                let next_hwnd = find_nearest_window(&grid, info.id)
                    .and_then(|nwid| self.windows.iter().find(|(_, wi)| wi.id == nwid).map(|(hw, _)| *hw));

                if let Some(hw) = next_hwnd {
                    unsafe {
                        let _ = SetForegroundWindow(hw.0);
                    }
                    self.focused_hwnd = Some(hw);
                    debug!("Swallowed window, focused next");
                } else {
                    self.focused_hwnd = None;
                }
            }

            self.window_workspaces.remove(&info.id);
            self.anim.remove(&info.id);
            debug!("Removed window: {} (id={})", info.title, info.id);
        }
    }

    pub fn move_focus(&mut self, dr: i32, dc: i32) {
        let grid = self.current_grid();
        let prev_focused = grid.focused_window;
        if let Some(wid) = grid.move_focus(dr, dc) {
            // Check if a swap happened (focused window changed cell)
            if let Some(prev) = prev_focused {
                if prev != wid {
                    if let Some(prev_cell) = grid.window_positions.get(&prev) {
                        if let Some(&swapped_wid) = grid.cells.get(prev_cell) {
                            if swapped_wid != prev {
                                // Flash both windows involved in swap
                                self.swap_flash.insert(prev, 20);
                                self.swap_flash.insert(swapped_wid, 20);
                            }
                        }
                    }
                }
            }

            for (&hwnd_wrapper, info) in &self.windows {
                if info.id == wid {
                    unsafe {
                        let _ = SetForegroundWindow(hwnd_wrapper.0);
                    }
                    self.focused_hwnd = Some(hwnd_wrapper);
                    debug!("Focus moved to window {} ({})", wid, info.title);
                    break;
                }
            }
        }
    }

    pub fn pan_camera(&mut self, dr: i32, dc: i32) {
        let grid = self.current_grid();
        grid.pan(dr, dc);
        debug!("Camera panned to ({}, {})", grid.camera.row, grid.camera.col);
    }

    pub fn resize_width(&mut self, grow: bool) {
        let step = self.config.layout.resize_step_px;
        let grid = self.current_grid();
        if let Some(wid) = grid.focused_window {
            if step > 0 {
                // Pixel-based resize
                let delta = if grow { step as i32 } else { -(step as i32) };
                self.adjust_window_size(wid, delta, 0);
            } else {
                if grow {
                    grid.grow_width(wid);
                } else {
                    grid.shrink_width(wid);
                }
            }
            self.show_resize_size(wid);
            debug!("Width adjusted for window {}", wid);
        }
    }

    pub fn resize_height(&mut self, grow: bool) {
        let step = self.config.layout.resize_step_px;
        let grid = self.current_grid();
        if let Some(wid) = grid.focused_window {
            if step > 0 {
                // Pixel-based resize
                let delta = if grow { step as i32 } else { -(step as i32) };
                self.adjust_window_size(wid, 0, delta);
            } else {
                if grow {
                    grid.grow_height(wid);
                } else {
                    grid.shrink_height(wid);
                }
            }
            self.show_resize_size(wid);
            debug!("Height adjusted for window {}", wid);
        }
    }

    fn adjust_window_size(&mut self, wid: u64, dw: i32, dh: i32) {
        if let Some(info) = self.windows.values().find(|i| i.id == wid) {
            unsafe {
                let mut rect = RECT::default();
                if GetWindowRect(info.hwnd, &mut rect).is_ok() {
                    let new_w = (rect.right - rect.left + dw).max(100);
                    let new_h = (rect.bottom - rect.top + dh).max(100);
                    let _ = SetWindowPos(
                        info.hwnd,
                        HWND(null_mut()),
                        rect.left,
                        rect.top,
                        new_w,
                        new_h,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
            }
        }
    }

    fn show_resize_size(&mut self, wid: u64) {
        if let Some(info) = self.windows.values().find(|i| i.id == wid) {
            unsafe {
                let mut rect = RECT::default();
                if GetWindowRect(info.hwnd, &mut rect).is_ok() {
                    let w = rect.right - rect.left;
                    let h = rect.bottom - rect.top;
                    let size_text = format!("{} x {}", w, h);
                    if let Some(ref bar) = self.bar {
                        bar.show_resize_size(size_text);
                    }
                }
            }
        }
    }

    pub fn move_window(&mut self, dr: i32, dc: i32) {
        let grid = self.current_grid();
        if let Some(wid) = grid.focused_window {
            grid.move_window(wid, dr, dc);
            self.tile_all_windows(0xFF7F7F7F, 0xFF454545);
        }
    }

    pub fn trigger_focus_flash(&mut self, wid: u64) {
        self.swap_flash.insert(wid, 15);
    }

    pub fn swap_windows(&mut self, src_hwnd: HWND, tgt_hwnd: HWND) {
        let src_wrapper = HWnd(src_hwnd);
        let tgt_wrapper = HWnd(tgt_hwnd);

        let src_id = self.windows.get(&src_wrapper).map(|i| i.id);
        let tgt_id = self.windows.get(&tgt_wrapper).map(|i| i.id);

        if let (Some(src_id), Some(tgt_id)) = (src_id, tgt_id) {
            let grid = self.current_grid();
            let src_cell = grid.window_positions.get(&src_id).copied();
            let tgt_cell = grid.window_positions.get(&tgt_id).copied();

            if let (Some(src_cell), Some(tgt_cell)) = (src_cell, tgt_cell) {
                grid.window_positions.insert(src_id, tgt_cell);
                grid.window_positions.insert(tgt_id, src_cell);
                grid.cells.insert(src_cell, tgt_id);
                grid.cells.insert(tgt_cell, src_id);
                self.swap_flash.insert(src_id, 35);
                self.swap_flash.insert(tgt_id, 35);
                self.notify("Swapped");
                info!("Swapped windows: {} <-> {}", src_id, tgt_id);
            }
        }
    }

    pub fn drag_move_window(&mut self, src_hwnd: HWND, tgt_hwnd: HWND) {
        let src_wrapper = HWnd(src_hwnd);
        let tgt_wrapper = HWnd(tgt_hwnd);

        let src_id = self.windows.get(&src_wrapper).map(|i| i.id);
        let tgt_id = self.windows.get(&tgt_wrapper).map(|i| i.id);

        if let (Some(src_id), Some(tgt_id)) = (src_id, tgt_id) {
            let grid = self.current_grid();
            let src_cell = grid.window_positions.get(&src_id).copied();
            let tgt_cell = grid.window_positions.get(&tgt_id).copied();

            if let (Some(src_cell), Some(tgt_cell)) = (src_cell, tgt_cell) {
                // Remove source from its cell, put target in source's cell
                grid.window_positions.insert(src_id, tgt_cell);
                grid.cells.remove(&src_cell);
                grid.cells.insert(tgt_cell, src_id);

                // Place target in source's cell
                grid.window_positions.insert(tgt_id, src_cell);
                grid.cells.remove(&tgt_cell);
                grid.cells.insert(src_cell, tgt_id);

                grid.focus_window(src_id);
                self.swap_flash.insert(src_id, 35);
                self.swap_flash.insert(tgt_id, 35);
                info!("Drag-moved window {} to cell of {}", src_id, tgt_id);
            }
        }
    }

    pub fn edge_tile_window(&mut self, src_hwnd: HWND, mode_code: i32) {
        let src_wrapper = HWnd(src_hwnd);
        let win_id = match self.windows.get(&src_wrapper) {
            Some(i) => i.id,
            None => return,
        };

        // Find the monitor this window is on
        let mon_idx = self.find_monitor_for_window(src_hwnd).unwrap_or(0);
        let mon = &self.monitors[mon_idx];

        let info = match self.windows.get_mut(&src_wrapper) {
            Some(i) => i,
            None => return,
        };

        let (mut x, mut y, mut w, mut h);

        match mode_code {
            0 => {
                // Maximize
                x = mon.left;
                y = mon.top;
                w = mon.width();
                h = mon.height();
                info.maximized = true;
                info.floating = false;
                info.float_x = None;
                info.float_y = None;
                info.float_w = None;
                info.float_h = None;
            }
            5 => {
                // Left half
                x = mon.left;
                y = mon.top;
                w = mon.width() / 2;
                h = mon.height();
            }
            6 => {
                // Right half
                x = mon.left + mon.width() / 2;
                y = mon.top;
                w = mon.width() / 2;
                h = mon.height();
            }
            1 => {
                // Top-left quarter
                x = mon.left;
                y = mon.top;
                w = mon.width() / 2;
                h = mon.height() / 2;
            }
            2 => {
                // Top-right quarter
                x = mon.left + mon.width() / 2;
                y = mon.top;
                w = mon.width() / 2;
                h = mon.height() / 2;
            }
            3 => {
                // Bottom-left quarter
                x = mon.left;
                y = mon.top + mon.height() / 2;
                w = mon.width() / 2;
                h = mon.height() / 2;
            }
            4 => {
                // Bottom-right quarter
                x = mon.left + mon.width() / 2;
                y = mon.top + mon.height() / 2;
                w = mon.width() / 2;
                h = mon.height() / 2;
            }
            7 => {
                // Bottom half
                x = mon.left;
                y = mon.top + mon.height() / 2;
                w = mon.width();
                h = mon.height() / 2;
            }
            _ => return,
        }

        // Enforce min/max constraints from rules
        if let Some(min_w) = info.min_width { if w < min_w as i32 { w = min_w as i32; } }
        if let Some(min_h) = info.min_height { if h < min_h as i32 { h = min_h as i32; } }
        if let Some(max_w) = info.max_width { if w > max_w as i32 { w = max_w as i32; } }
        if let Some(max_h) = info.max_height { if h > max_h as i32 { h = max_h as i32; } }

        // Switch to floating mode with the tiled position
        info.floating = true;
        info.float_x = Some(x);
        info.float_y = Some(y);
        info.float_w = Some(w.max(0) as u32);
        info.float_h = Some(h.max(0) as u32);

        unsafe {
            let _ = SetWindowPos(
                src_hwnd,
                HWND_TOP,
                x, y, w, h,
                SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }

        info!("Edge tiled window {} with mode {} to ({},{}) {}x{}",
              win_id, mode_code, x, y, w, h);
    }

    pub fn toggle_snap_mode(&mut self) {
        self.snap_mode = !self.snap_mode;
        if self.snap_mode {
            self.snap_flash = 60; // Flash for ~1 second
            info!("Snap mode ON — use arrows/1-9 to snap, Esc to exit");
        } else {
            self.snap_flash = 0;
            info!("Snap mode OFF");
        }
    }

    pub fn exit_snap_mode(&mut self) {
        self.snap_mode = false;
        self.snap_flash = 0;
    }

    pub fn snap_window(&mut self, pos: i32) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            self.edge_tile_window(hwnd_wrapper.0, pos);
            self.snap_flash = 15; // Brief flash on snap
        }
    }

    /// Snap a rectangle position/size to the grid
    fn snap_to_grid(&self, x: i32, y: i32, w: i32, h: i32) -> (i32, i32, i32, i32) {
        let grid = self.config.layout.snap_grid_size as i32;
        let sx = (x / grid) * grid;
        let sy = (y / grid) * grid;
        let sw = ((w + grid - 1) / grid) * grid;
        let sh = ((h + grid - 1) / grid) * grid;
        (sx, sy, sw.max(grid), sh.max(grid))
    }

    /// Snap a floating window to other windows' edges and the grid
    pub fn snap_floating_window(&mut self, hwnd: HWND) {
        let wrapper = HWnd(hwnd);
        let info = match self.windows.get(&wrapper) {
            Some(i) if i.floating => i,
            _ => return,
        };

        let mon_idx = self.find_monitor_for_window(hwnd).unwrap_or(0);
        let mon = &self.monitors[mon_idx];
        let mw = mon.work_width();
        let mh = mon.work_height();
        let mleft = mon.work_left;
        let mtop = mon.work_top;

        unsafe {
            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).is_err() {
                return;
            }

            let mut x = rect.left;
            let mut y = rect.top;
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;

            // Snap to grid
            let (gx, gy, gw, gh) = self.snap_to_grid(x, y, w, h);

            let edge_dist = self.config.layout.snap_edge_distance as i32;
            let mut snapped_x = gx;
            let mut snapped_y = gy;

            // --- Screen edge snapping ---
            // Left edge
            if (gx - mleft).abs() <= edge_dist {
                snapped_x = mleft;
            }
            // Right edge
            if ((mleft + mw) - (gx + gw)).abs() <= edge_dist {
                snapped_x = mleft + mw - gw;
            }
            // Top edge
            if (gy - mtop).abs() <= edge_dist {
                snapped_y = mtop;
            }
            // Bottom edge
            if ((mtop + mh) - (gy + gh)).abs() <= edge_dist {
                snapped_y = mtop + mh - gh;
            }

            // --- Corner snapping (half/maximize) ---
            // Left half: snap left edge + half width
            if (gx - mleft).abs() <= edge_dist && (gw - mw / 2).abs() <= edge_dist {
                snapped_x = mleft;
                snapped_y = mtop;
                let snapped_w = mw / 2;
                let snapped_h = mh;
                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOP,
                    snapped_x, snapped_y, snapped_w, snapped_h,
                    SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOACTIVATE,
                );
                if let Some(info) = self.windows.get_mut(&wrapper) {
                    info.float_x = Some(snapped_x);
                    info.float_y = Some(snapped_y);
                    info.float_w = Some(snapped_w as u32);
                    info.float_h = Some(snapped_h as u32);
                }
                return;
            }
            // Right half
            if ((mleft + mw) - (gx + gw)).abs() <= edge_dist && (gw - mw / 2).abs() <= edge_dist {
                snapped_x = mleft + mw / 2;
                snapped_y = mtop;
                let snapped_w = mw / 2;
                let snapped_h = mh;
                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOP,
                    snapped_x, snapped_y, snapped_w, snapped_h,
                    SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOACTIVATE,
                );
                if let Some(info) = self.windows.get_mut(&wrapper) {
                    info.float_x = Some(snapped_x);
                    info.float_y = Some(snapped_y);
                    info.float_w = Some(snapped_w as u32);
                    info.float_h = Some(snapped_h as u32);
                }
                return;
            }
            // Maximize (snap to all 4 edges)
            if (gx - mleft).abs() <= edge_dist
                && (gy - mtop).abs() <= edge_dist
                && ((mleft + mw) - (gx + gw)).abs() <= edge_dist
                && ((mtop + mh) - (gy + gh)).abs() <= edge_dist
            {
                snapped_x = mleft;
                snapped_y = mtop;
                let snapped_w = mw;
                let snapped_h = mh;
                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOP,
                    snapped_x, snapped_y, snapped_w, snapped_h,
                    SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOACTIVATE,
                );
                if let Some(info) = self.windows.get_mut(&wrapper) {
                    info.float_x = Some(snapped_x);
                    info.float_y = Some(snapped_y);
                    info.float_w = Some(snapped_w as u32);
                    info.float_h = Some(snapped_h as u32);
                }
                return;
            }

            // --- Edge snapping to other windows ---
            for (other_hwnd, other_info) in &self.windows {
                if other_info.floating && *other_hwnd != wrapper {
                    let mut orect = RECT::default();
                    if GetWindowRect(other_info.hwnd, &mut orect).is_ok() {
                        let ox1 = orect.left;
                        let ox2 = orect.right;
                        let oy1 = orect.top;
                        let oy2 = orect.bottom;

                        if (gx + gw - ox1).abs() <= edge_dist && gx + gw >= ox1 && gx <= ox2 {
                            snapped_x = ox1 - gw;
                        }
                        if (ox2 - gx).abs() <= edge_dist && ox2 >= gx && ox2 <= gx + gw {
                            snapped_x = ox2;
                        }
                        if (gy + gh - oy1).abs() <= edge_dist && gy + gh >= oy1 && gy <= oy2 {
                            snapped_y = oy1 - gh;
                        }
                        if (oy2 - gy).abs() <= edge_dist && oy2 >= gy && oy2 <= gy + gh {
                            snapped_y = oy2;
                        }
                    }
                }
            }

            // Clamp to monitor bounds
            snapped_x = snapped_x.clamp(mleft, mleft + mw - gw);
            snapped_y = snapped_y.clamp(mtop, mtop + mh - gh);

            // Apply if position changed
            if snapped_x != x || snapped_y != y {
                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOP,
                    snapped_x, snapped_y, gw, gh,
                    SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOACTIVATE,
                );
                // Update stored float position
                if let Some(info) = self.windows.get_mut(&wrapper) {
                    info.float_x = Some(snapped_x);
                    info.float_y = Some(snapped_y);
                    info.float_w = Some(gw as u32);
                    info.float_h = Some(gh as u32);
                }
            }
        }
    }

    pub fn close_focused(&mut self) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            unsafe {
                let _ = PostMessageW(hwnd_wrapper.0, WM_CLOSE, WPARAM(0), LPARAM(0));
            }
            debug!("Close sent to focused window");
        }
    }

    pub fn toggle_bar_visibility(&mut self) {
        self.bar_visible = !self.bar_visible;
        if let Some(ref bar) = self.bar {
            unsafe {
                let _ = ShowWindow(bar.hwnd, if self.bar_visible { SW_SHOW } else { SW_HIDE });
            }
        }
        info!("Bar visibility: {}", if self.bar_visible { "shown" } else { "hidden" });
    }

    pub fn toggle_fullscreen(&mut self) {
        let mut became_fullscreen = false;
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                info.fullscreen = !info.fullscreen;
                became_fullscreen = info.fullscreen;
                unsafe {
                    if info.fullscreen {
                        let mut rect = RECT::default();
                        let _ = GetWindowRect(info.hwnd, &mut rect);
                        info.saved_x = rect.left;
                        info.saved_y = rect.top;
                        info.saved_w = rect.right - rect.left;
                        info.saved_h = rect.bottom - rect.top;

                        let mon = MonitorFromWindow(info.hwnd, MONITOR_DEFAULTTONULL);
                        if !mon.is_invalid() {
                            let mut mi = MONITORINFO {
                                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                                ..Default::default()
                            };
                            if GetMonitorInfoW(mon, &mut mi).as_bool() {
                                let _ = SetWindowPos(
                                    info.hwnd,
                                    HWND_TOPMOST,
                                    mi.rcWork.left,
                                    mi.rcWork.top,
                                    mi.rcWork.right - mi.rcWork.left,
                                    mi.rcWork.bottom - mi.rcWork.top,
                                    SWP_FRAMECHANGED,
                                );
                            }
                        }
                    } else {
                        let _ = SetWindowPos(
                            info.hwnd,
                            HWND_NOTOPMOST,
                            info.saved_x,
                            info.saved_y,
                            info.saved_w,
                            info.saved_h,
                            SWP_FRAMECHANGED,
                        );
                    }
                }
                info!("Fullscreen toggled: {} (id={})", info.title, info.id);
            }
        }
        if became_fullscreen {
            self.enable_idle_inhibit();
        } else if self.focused_hwnd.is_some() {
            self.disable_idle_inhibit();
        }
    }

    fn find_monitor_for_window(&self, hwnd: HWND) -> Option<usize> {
        unsafe {
            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).is_ok() {
                let wx = (rect.left + rect.right) / 2;
                let wy = (rect.top + rect.bottom) / 2;
                for (i, mon) in self.monitors.iter().enumerate() {
                    if wx >= mon.left && wx < mon.right
                        && wy >= mon.top && wy < mon.bottom
                    {
                        return Some(i);
                    }
                }
            }
        }
        Some(0)
    }

    /// Move focused window to a different monitor
    pub fn move_focused_to_monitor(&mut self, target_mon: usize) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                if target_mon >= self.monitors.len() { return; }
                let mon = &self.monitors[target_mon];
                let wid = info.id;

                // Update monitor assignment
                self.window_monitors.insert(wid, target_mon);

                // Move window to target monitor
                unsafe {
                    let _ = SetWindowPos(
                        hwnd_wrapper.0,
                        HWND_TOP,
                        mon.work_left + 50,
                        mon.work_top + 50,
                        info.saved_w.max(400),
                        info.saved_h.max(300),
                        SWP_SHOWWINDOW | SWP_FRAMECHANGED,
                    );
                }

                // Re-tile
                self.tile_all_windows(0xFF7F7F7F, 0xFF454545);
                info!("Moved window to monitor {}", target_mon + 1);
            }
        }
    }

    /// Focus the next monitor (cycle focus across monitors)
    pub fn focus_next_monitor(&mut self) {
        if self.monitors.len() <= 1 { return; }
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get(&hwnd_wrapper) {
                let current_mon = self.window_monitors.get(&info.id).copied().unwrap_or(0);
                let next_mon = (current_mon + 1) % self.monitors.len();
                self.focus_monitor(next_mon);
            }
        }
    }

    /// Focus a specific monitor by switching to its current workspace
    pub fn focus_monitor(&mut self, mon_idx: usize) {
        if mon_idx >= self.monitor_workspaces.len() { return; }
        let target_ws = self.monitor_workspaces[mon_idx].current;
        // Find a window on that monitor to focus
        for (hwnd_wrapper, info) in &self.windows {
            if let Some(wmon) = self.window_monitors.get(&info.id) {
                if *wmon == mon_idx && !info.minimized && info.visible {
                    self.on_focus_changed(hwnd_wrapper.0);
                    return;
                }
            }
        }
    }

    pub fn clamp_focused_window(&mut self) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                let (mut x, mut y, mut w, mut h) = match info.float_x.zip(info.float_y).zip(info.float_w).zip(info.float_h) {
                    Some((((x, y), w), h)) => (x as i32, y as i32, w as i32, h as i32),
                    None => return,
                };

                if let Some(min_w) = info.min_width { w = w.max(min_w as i32); }
                if let Some(min_h) = info.min_height { h = h.max(min_h as i32); }
                if let Some(max_w) = info.max_width { w = w.min(max_w as i32); }
                if let Some(max_h) = info.max_height { h = h.min(max_h as i32); }

                info.float_w = Some(w.max(0) as u32);
                info.float_h = Some(h.max(0) as u32);

                unsafe {
                    let _ = SetWindowPos(hwnd_wrapper.0, HWND(null_mut()), x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED);
                }
            }
        }
    }

    pub fn window_count_per_workspace(&self, mon_idx: usize) -> Vec<usize> {
        let count = self.monitor_workspaces[mon_idx].grids.len();
        let mut counts = vec![0usize; count];
        for (&wid, &ws) in &self.window_workspaces {
            if ws < count {
                if let Some(&wm) = self.window_monitors.get(&wid) {
                    if wm == mon_idx {
                        counts[ws] += 1;
                    }
                }
            }
        }
        counts
    }

    pub fn workspace_names(&self, mon_idx: usize) -> Vec<String> {
        let count = self.monitor_workspaces[mon_idx].grids.len();
        let per_mon = &self.config.layout.per_monitor_workspace_names;
        if let Some(names) = per_mon.get(mon_idx) {
            if !names.is_empty() {
                return names.iter().take(count).cloned().collect();
            }
        }
        let global = &self.config.layout.workspace_names;
        if global.is_empty() {
            (0..count).map(|i| (i + 1).to_string()).collect()
        } else {
            global.iter().take(count).cloned().collect()
        }
    }

    pub fn monitor_layout(&self, mon_idx: usize) -> crate::config::MonitorLayout {
        let layouts = &self.config.layout.monitor_layouts;
        layouts.get(mon_idx).cloned().unwrap_or(crate::config::MonitorLayout {
            gaps: None,
            inner_padding: None,
            outer_padding: None,
            border_width: None,
            corner_radius: None,
        })
    }

    pub fn effective_gap(&self, mon_idx: usize) -> u32 {
        self.monitor_layout(mon_idx).gaps.unwrap_or(self.config.layout.gaps)
    }

    pub fn effective_border_width(&self, mon_idx: usize) -> u32 {
        self.monitor_layout(mon_idx).border_width.unwrap_or(self.config.layout.border_width)
    }

    pub fn effective_corner_radius(&self, mon_idx: usize) -> u32 {
        self.monitor_layout(mon_idx).corner_radius.unwrap_or(self.config.layout.corner_radius)
    }

    pub fn set_workspace_count(&mut self, count: usize) {
        if count < 1 || count > 10 { return; }

        for mws in &mut self.monitor_workspaces {
            let old_count = mws.grids.len();
            if count > old_count {
                // Add new workspaces
                for _ in old_count..count {
                    mws.grids.push(GridState::new());
                }
            } else if count < old_count {
                // Remove extra workspaces — move windows to current workspace
                let current = mws.current;
                let mut to_move: Vec<(Cell, u64)> = Vec::new();
                for wid in count..old_count {
                    let grid = &mws.grids[wid];
                    for (&cell, &win_id) in &grid.cells {
                        to_move.push((cell, win_id));
                    }
                }
                for (cell, win_id) in to_move {
                    if let Some(new_cell) = find_empty_cell(&mws.grids[current]) {
                        mws.grids[current].cells.insert(new_cell, win_id);
                        mws.grids[current].window_positions.insert(win_id, new_cell);
                        self.window_workspaces.insert(win_id, current);
                    }
                }
                mws.grids.truncate(count);
                if mws.current >= count {
                    mws.current = count - 1;
                }
            }
        }

        self.config.layout.workspace_count = count;
        info!("Workspace count set to {}", count);

        // Update bar
        if let Some(ref bar) = self.bar {
            let mon_idx = self.focused_hwnd
                .and_then(|hwnd| self.window_for_hwnd(hwnd.0))
                .and_then(|info| self.window_monitors.get(&info.id))
                .copied()
                .unwrap_or(0);
            let names = self.workspace_names(mon_idx);
            let counts = self.window_count_per_workspace(mon_idx);
            bar.set_workspaces(names, self.monitor_workspaces[mon_idx].current, counts);
        }
    }

    /// Add a new empty workspace to all monitors
    pub fn add_workspace(&mut self) {
        let new_count = self.config.layout.workspace_count + 1;
        if new_count > 10 {
            self.notify("Max 10 workspaces");
            return;
        }
        self.set_workspace_count(new_count);

        // Switch to the new workspace on the current monitor
        let mon_idx = self.focused_hwnd
            .and_then(|hwnd| self.window_for_hwnd(hwnd.0))
            .and_then(|info| self.window_monitors.get(&info.id))
            .copied()
            .unwrap_or(0);
        let new_ws = new_count - 1;
        self.monitor_workspaces[mon_idx].current = new_ws;
        self.switch_workspace(new_ws);
        self.notify(&format!("Workspace {} added", new_count));
        info!("Workspace count increased to {}", new_count);
    }

    /// Remove the current workspace if it's empty, or merge windows to adjacent workspace
    pub fn remove_workspace(&mut self) {
        let count = self.config.layout.workspace_count;
        if count <= 1 {
            self.notify("Cannot remove last workspace");
            return;
        }

        let mon_idx = self.focused_hwnd
            .and_then(|hwnd| self.window_for_hwnd(hwnd.0))
            .and_then(|info| self.window_monitors.get(&info.id))
            .copied()
            .unwrap_or(0);
        let current = self.monitor_workspaces[mon_idx].current;

        // Check if current workspace has windows
        let grid = &self.monitor_workspaces[mon_idx].grids[current];
        if !grid.cells.is_empty() {
            self.notify("Close all windows first");
            return;
        }

        // Remove the workspace from all monitors
        self.set_workspace_count(count - 1);

        // Switch to the closest available workspace
        let new_current = current.min(count - 2);
        self.monitor_workspaces[mon_idx].current = new_current;
        self.switch_workspace(new_current);
        self.notify(&format!("Workspace removed, now {} workspaces", count - 1));
        info!("Workspace count decreased to {}", count - 1);
    }

    pub fn snap_layout(&mut self, cols: usize, rows: usize) {
        // Collect visible window IDs before getting mutable grid access
        let mut visible_wids: Vec<(u64, usize)> = Vec::new();
        for (&hwnd_wrapper, info) in &self.windows {
            if info.floating || !info.visible || info.minimized {
                continue;
            }
            let z = info.z_order;
            visible_wids.push((info.id, z));
        }

        visible_wids.sort_by_key(|&(_, z)| z);

        let wids: Vec<u64> = visible_wids.into_iter().map(|(wid, _)| wid).collect();

        let grid = self.current_grid();
        grid.layout_mode = crate::layout::LayoutMode::Grid;
        grid.snap_layout(&wids, cols, rows);
        info!("Snap layout: {}x{} ({} windows)", cols, rows, wids.len());
    }

    fn collect_visible_wids(&self) -> Vec<u64> {
        let mut visible_wids: Vec<(u64, usize)> = Vec::new();
        for info in self.windows.values() {
            if info.floating || !info.visible || info.minimized {
                continue;
            }
            visible_wids.push((info.id, info.z_order));
        }
        visible_wids.sort_by_key(|&(_, z)| z);
        visible_wids.into_iter().map(|(wid, _)| wid).collect()
    }

    pub fn layout_preset_columns(&mut self, n: usize) {
        if n == 0 { return; }
        let wids = self.collect_visible_wids();
        if wids.is_empty() { return; }
        let grid = self.current_grid();
        grid.layout_mode = crate::layout::LayoutMode::Grid;
        grid.snap_layout(&wids, n, 1);
        self.save_session();
        info!("Layout preset: columns ({})", n);
    }

    pub fn layout_preset_rows(&mut self, n: usize) {
        if n == 0 { return; }
        let wids = self.collect_visible_wids();
        if wids.is_empty() { return; }
        let grid = self.current_grid();
        grid.layout_mode = crate::layout::LayoutMode::Grid;
        grid.snap_layout(&wids, 1, n);
        self.save_session();
        info!("Layout preset: rows ({})", n);
    }

    pub fn layout_preset_master(&mut self) {
        let wids = self.collect_visible_wids();
        if wids.is_empty() { return; }
        let grid = self.current_grid();
        grid.layout_mode = crate::layout::LayoutMode::Master;
        grid.cells.clear();
        grid.window_positions.clear();
        grid.cell_nodes.clear();
        grid.focused_window = None;

        if wids.len() == 1 {
            let wid = wids[0];
            grid.cells.insert(crate::layout::Cell::new(0, 0), wid);
            grid.window_positions.insert(wid, crate::layout::Cell::new(0, 0));
            grid.cell_nodes.insert(crate::layout::Cell::new(0, 0), crate::layout::CellNode::Leaf(wid));
            grid.focused_window = Some(wid);
        } else {
            // First window: master (column 0)
            let master_wid = wids[0];
            grid.cells.insert(crate::layout::Cell::new(0, 0), master_wid);
            grid.window_positions.insert(master_wid, crate::layout::Cell::new(0, 0));
            grid.cell_nodes.insert(crate::layout::Cell::new(0, 0), crate::layout::CellNode::Leaf(master_wid));

            // Rest: stack (column 1, rows 0, 1, 2, ...)
            for (i, &wid) in wids.iter().skip(1).enumerate() {
                let cell = crate::layout::Cell::new(i as i32, 1);
                grid.cells.insert(cell, wid);
                grid.window_positions.insert(wid, cell);
                grid.cell_nodes.insert(cell, crate::layout::CellNode::Leaf(wid));
            }
            grid.focused_window = Some(master_wid);
        }
        self.save_session();
        info!("Layout preset: master ({} windows)", wids.len());
    }

    pub fn layout_preset_fibonacci(&mut self) {
        let wids = self.collect_visible_wids();
        if wids.is_empty() { return; }
        let n = wids.len();
        let grid = self.current_grid();
        grid.layout_mode = crate::layout::LayoutMode::Grid;
        // Fibonacci: 1, 1, 2, 3, 5, 8, ... columns
        let fib: Vec<usize> = (0..10).scan((0u64, 1u64), |state, _| {
            let next = state.0 + state.1;
            state.0 = state.1;
            state.1 = next;
            Some(next as usize)
        }).collect();
        let cols = fib.iter().take_while(|&&f| f < n).last().copied().unwrap_or(1).max(1).min(n);
        grid.snap_layout(&wids, cols, 1);
        self.save_session();
        info!("Layout preset: fibonacci ({} cols)", cols);
    }

    pub fn apply_layout_preset(&mut self, name: &str) {
        let (kind, cols, rows) = match self.config.layout.layout_presets.iter().find(|p| p.name == name) {
            Some(p) => (p.kind.clone(), p.cols, p.rows),
            None => {
                warn!("Layout preset not found: {}", name);
                return;
            }
        };
        let wids = self.collect_visible_wids();
        if wids.is_empty() { return; }
        let grid = self.current_grid();

        match kind.as_str() {
            "grid" => {
                let cols = cols.unwrap_or(2).max(1) as usize;
                grid.layout_mode = crate::layout::LayoutMode::Grid;
                grid.snap_layout(&wids, cols, 1);
                info!("Layout preset '{}': grid {}x1", name, cols);
            }
            "columns" => {
                let n = cols.unwrap_or(2).max(1) as usize;
                grid.layout_mode = crate::layout::LayoutMode::Grid;
                grid.snap_layout(&wids, n, 1);
                info!("Layout preset '{}': columns ({})", name, n);
            }
            "rows" => {
                let n = rows.unwrap_or(2).max(1) as usize;
                grid.layout_mode = crate::layout::LayoutMode::Grid;
                grid.snap_layout(&wids, 1, n);
                info!("Layout preset '{}': rows ({})", name, n);
            }
            "master" => {
                grid.layout_mode = crate::layout::LayoutMode::Master;
                grid.cells.clear();
                grid.window_positions.clear();
                grid.cell_nodes.clear();
                grid.focused_window = None;
                if wids.len() == 1 {
                    let wid = wids[0];
                    grid.cells.insert(crate::layout::Cell::new(0, 0), wid);
                    grid.window_positions.insert(wid, crate::layout::Cell::new(0, 0));
                    grid.cell_nodes.insert(crate::layout::Cell::new(0, 0), crate::layout::CellNode::Leaf(wid));
                    grid.focused_window = Some(wid);
                } else {
                    let master_wid = wids[0];
                    grid.cells.insert(crate::layout::Cell::new(0, 0), master_wid);
                    grid.window_positions.insert(master_wid, crate::layout::Cell::new(0, 0));
                    grid.cell_nodes.insert(crate::layout::Cell::new(0, 0), crate::layout::CellNode::Leaf(master_wid));
                    for (i, &wid) in wids.iter().skip(1).enumerate() {
                        let cell = crate::layout::Cell::new(i as i32, 1);
                        grid.cells.insert(cell, wid);
                        grid.window_positions.insert(wid, cell);
                        grid.cell_nodes.insert(cell, crate::layout::CellNode::Leaf(wid));
                    }
                    grid.focused_window = Some(master_wid);
                }
                info!("Layout preset '{}': master ({} windows)", name, wids.len());
            }
            "fibonacci" => {
                let n = wids.len();
                grid.layout_mode = crate::layout::LayoutMode::Grid;
                let fib: Vec<usize> = (0..10).scan((0u64, 1u64), |state, _| {
                    let next = state.0 + state.1;
                    state.0 = state.1;
                    state.1 = next;
                    Some(next as usize)
                }).collect();
                let cols = fib.iter().take_while(|&f| *f < n).last().copied().unwrap_or(1).max(1).min(n);
                grid.snap_layout(&wids, cols, 1);
                info!("Layout preset '{}': fibonacci ({} cols)", name, cols);
            }
            "fullscreen" => {
                grid.layout_mode = crate::layout::LayoutMode::Grid;
                if wids.len() == 1 {
                    grid.snap_layout(&wids, 1, 1);
                }
                info!("Layout preset '{}': fullscreen", name);
            }
            _ => {
                warn!("Unknown layout preset kind: {}", kind);
            }
        }
        self.save_session();
    }

    /// Save current layout as a named preset
    pub fn save_layout_preset(&mut self, name: &str, kind: &str) {
        use crate::config::LayoutPreset;

        let grid = self.current_grid();
        let wids = self.collect_visible_wids();
        let n = wids.len().max(1);

        let preset = LayoutPresest {
            name: name.to_string(),
            kind: kind.to_string(),
            cols: if kind == "grid" || kind == "columns" { Some(n as u32) } else { None },
            rows: if kind == "rows" { Some(n as u32) } else { None },
        };

        // Remove existing preset with same name
        self.config.layout.layout_presets.retain(|p| p.name != name);
        self.config.layout.layout_presets.push(preset);

        // Save config to file
        if let Err(e) = self.config.save() {
            warn!("Failed to save config with new layout preset: {}", e);
        } else {
            info!("Layout preset '{}' saved ({})", name, kind);
        }
    }

    pub fn apply_custom_layout(&mut self, name: &str) {
        let (widths, heights) = match self.config.layout.snap_layouts.iter().find(|l| l.name == name) {
            Some(l) => (l.widths.clone(), l.heights.clone()),
            None => {
                warn!("Custom layout not found: {}", name);
                return;
            }
        };
        let wids = self.collect_visible_wids();
        if wids.is_empty() { return; }
        let grid = self.current_grid();
        grid.layout_mode = crate::layout::LayoutMode::Grid;
        grid.snap_layout_custom(&wids, &widths, &heights);
        self.save_session();
        info!("Custom layout '{}': {} cols x {} rows ({} windows)", name, widths.len().max(1), heights.len().max(1), wids.len());
    }

    /// Save current grid's custom widths/heights as a named snap layout
    pub fn save_snap_layout(&mut self, name: &str) {
        use crate::config::SnapLayout;

        let grid = self.current_grid();
        let widths = if grid.custom_widths.is_empty() {
            vec![800]  // default single column
        } else {
            grid.custom_widths.clone()
        };
        let heights = if grid.custom_heights.is_empty() {
            vec![600]  // default single row
        } else {
            grid.custom_heights.clone()
        };

        let layout = SnapLayout {
            name: name.to_string(),
            widths,
            heights,
        };

        // Remove existing with same name
        self.config.layout.snap_layouts.retain(|l| l.name != name);
        self.config.layout.snap_layouts.push(layout);

        if let Err(e) = self.config.save() {
            warn!("Failed to save snap layout '{}': {}", name, e);
        } else {
            info!("Snap layout '{}' saved ({}x{})", name, grid.custom_widths.len().max(1), grid.custom_heights.len().max(1));
        }
    }

    pub fn minimize_to_tray(&mut self) {
        let hwnd = match self.focused_hwnd {
            Some(h) => h.0,
            None => return,
        };
        let wid = match self.windows.get(&HWnd(hwnd)).map(|i| i.id) {
            Some(id) => id,
            None => return,
        };
        if self.tray_windows.contains_key(&wid) { return; }

        unsafe {
            // Hide the window
            let _ = ShowWindow(hwnd, SW_HIDE);

            // Create tray icon if not exists
            self.ensure_tray_icon();

            // Add to tray
            let title = self.windows.get(&HWnd(hwnd)).map(|i| i.title.clone()).unwrap_or_default();
            let tip_str: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();

            let mut nid = NOTIFYICONDATAW::default();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = self.tray_hwnd.unwrap_or(HWND(std::ptr::null_mut()));
            nid.uID = wid as u32;
            nid.uFlags = NIF_ICON | NIF_TIP | NIF_MESSAGE;
            nid.uCallbackMessage = WM_APP + 0x200;
            let copy_len = tip_str.len().min(nid.szTip.len());
            nid.szTip[..copy_len].copy_from_slice(&tip_str[..copy_len]);

            let _ = Shell_NotifyIconW(NIM_ADD, &nid);
            self.tray_windows.insert(wid, hwnd);
            info!("Minimized to tray: {} (wid={})", title, wid);
        }
    }

    pub fn restore_from_tray(&mut self, wid: u64) {
        if let Some(&hwnd) = self.tray_windows.get(&wid) {
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOW);
                let _ = SetForegroundWindow(hwnd);

                // Remove tray icon
                let mut nid = NOTIFYICONDATAW::default();
                nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
                nid.hWnd = self.tray_hwnd.unwrap_or(HWND(std::ptr::null_mut()));
                nid.uID = wid as u32;
                nid.uFlags = 0;
                let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            }
            self.tray_windows.remove(&wid);
            info!("Restored from tray: wid={}", wid);
        }
    }

    pub fn restore_all_tray(&mut self) {
        let wids: Vec<u64> = self.tray_windows.keys().copied().collect();
        for wid in wids {
            self.restore_from_tray(wid);
        }
    }

    /// Create persistent UltraWM system tray icon with context menu
    pub fn create_tray_icon(&mut self) -> anyhow::Result<()> {
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::Win32::UI::Shell::*;

        self.ensure_tray_icon();
        let hwnd = match self.tray_hwnd {
            Some(h) => h,
            None => return Err(anyhow::anyhow!("No tray window")),
        };

        unsafe {
            // Load tray icon (use system default or load from resource)
            let icon_handle = LoadImageW(
                HINSTANCE(GetModuleHandleW(None).unwrap_or_default().0),
                w!("ULTRAWM_ICON"),
                IMAGE_ICON,
                16, 16,
                LR_DEFAULTCOLOR,
            );

            let h_icon = match icon_handle {
                Ok(h) => HICON(h.0),
                Err(_) => {
                    // Fallback: use system application icon
                    let hinst = HINSTANCE(GetModuleHandleW(None).unwrap_or_default().0);
                    HICON(LoadIconW(hinst, w!("IDI_APPLICATION")).0)
                }
            };

            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 100,
                uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_GUID,
                uCallbackMessage: WM_APP + 0x200,
                hIcon: h_icon,
                szTip: [0; 128],
                dwState: 0,
                dwStateMask: 0,
                szInfo: [0; 256],
                Anonymous: Default::default(),
                szInfoTitle: [0; 64],
                dwInfoFlags: 0,
                guidItem: Default::default(),
                hBalloonIcon: HICON(null_mut()),
            };

            let tip = "UltraWM — tiling window manager";
            let tip_w: Vec<u16> = tip.encode_utf16().chain(Some(0)).collect();
            nid.szTip[..tip_w.len().min(128)].copy_from_slice(&tip_w[..tip_w.len().min(128)]);

            Shell_NotifyIconW(NIM_ADD, &nid);
        }

        info!("UltraWM tray icon created");
        Ok(())
    }

    fn ensure_tray_icon(&mut self) {
        if self.tray_hwnd.is_some() { return; }

        unsafe {
            // Create a message-only window for tray notifications
            let hinstance = HINSTANCE(GetModuleHandleW(None).unwrap_or_default().0);
            let class = WNDCLASSW {
                lpfnWndProc: Some(tray_wnd_proc),
                hInstance: hinstance,
                lpszClassName: w!("UltraWMTray"),
                hbrBackground: HBRUSH(GetStockObject(HOLLOW_BRUSH).0),
                ..Default::default()
            };
            RegisterClassW(&class);

            if let Ok(hwnd) = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("UltraWMTray"),
                w!("UltraWM Tray"),
                WS_POPUP,
                0, 0, 0, 0,
                HWND_MESSAGE,
                None, hinstance, None,
            ) {
                self.tray_hwnd = Some(hwnd);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
        }
    }

    pub fn toggle_shade(&mut self) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                info.shaded = !info.shaded;
                unsafe {
                    if info.shaded {
                        let mut rect = RECT::default();
                        let _ = GetWindowRect(info.hwnd, &mut rect);
                        info.saved_w = rect.right - rect.left;
                        info.saved_h = rect.bottom - rect.top;
                        let _ = SetWindowPos(
                            info.hwnd,
                            HWND(null_mut()),
                            rect.left, rect.top,
                            info.saved_w, 30,
                            SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                        );
                    } else {
                        let _ = SetWindowPos(
                            info.hwnd,
                            HWND(null_mut()),
                            info.saved_x, info.saved_y,
                            info.saved_w, info.saved_h,
                            SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                        );
                    }
                }
                info!("Shade toggled: {} (id={}, shaded={})", info.title, info.id, info.shaded);
            }
        }
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                info.opacity = Some(opacity.clamp(0.0, 1.0));
                unsafe {
                    let _ = SetLayeredWindowAttributes(info.hwnd, COLORREF(0), (opacity.clamp(0.0, 1.0) * 255.0) as u8, LWA_ALPHA);
                }
                info!("Opacity set: {} (id={})", opacity, info.id);
            }
        }
    }

    pub fn adjust_opacity(&mut self, delta: f32) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                let current = info.opacity.unwrap_or(1.0);
                let new_opacity = (current + delta).clamp(0.0, 1.0);
                info.opacity = Some(new_opacity);
                self.apply_window_opacity(hwnd_wrapper.0, new_opacity);
                self.remember_app_opacity(&info.exe, new_opacity);
                info!("Opacity adjusted: {} -> {} (id={})", current, new_opacity, info.id);
            }
        }
    }

    /// Apply rounded corners to a window using hardware-accelerated window region
    pub fn apply_rounded_corners(&self, hwnd: HWND, radius: u32) {
        if radius == 0 {
            return;
        }
        unsafe {
            let mut rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut rect);
            let w = (rect.right - rect.left).max(1) as i32;
            let h = (rect.bottom - rect.top).max(1) as i32;
            let r = radius.min((w / 2).max(1) as u32).min((h / 2).max(1) as u32) as i32;
            let rgn = CreateRoundRectRgn(0, 0, w, h, r, r);
            if !rgn.is_invalid() {
                let _ = SetWindowRgn(hwnd, rgn, false);
            }
        }
    }

    /// Enable DWM drop shadow for a window
    pub fn apply_dwm_shadow(&self, hwnd: HWND) {
        unsafe {
            let mut policy = DWMNCRP_ENABLED;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_NCRENDERING_POLICY,
                &mut policy as *mut _ as *const _,
                std::mem::size_of::<DWMNCRENDERINGPOLICY>() as u32,
            );
        }
    }

    /// Apply default window opacity
    pub fn apply_window_opacity(&self, hwnd: HWND, opacity: f32) {
        if opacity < 1.0 {
            unsafe {
                let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                let _ = SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED.0 as i32);
                let alpha = (opacity.clamp(0.0, 1.0) * 255.0) as u8;
                let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA);
            }
        }
    }

    /// Load per-app opacity memory from file
    pub fn load_per_app_opacity(&mut self) {
        let path = self.per_app_opacity_path();
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(map) = serde_json::from_str::<HashMap<String, f32>>(&contents) {
                self.per_app_opacity = map;
                debug!("Loaded {} per-app opacity entries", self.per_app_opacity.len());
            }
        }
    }

    /// Save per-app opacity memory to file
    pub fn save_per_app_opacity(&self) {
        let path = self.per_app_opacity_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.per_app_opacity) {
            let _ = std::fs::write(&path, json);
        }
    }

    /// Get or set per-app opacity for an exe, applying it to the window
    pub fn apply_per_app_opacity(&mut self, hwnd: HWND, exe: &str) {
        if let Some(&opacity) = self.per_app_opacity.get(exe) {
            if opacity < 1.0 {
                self.apply_window_opacity(hwnd, opacity);
                if let Some(info) = self.windows.values_mut().find(|i| i.hwnd == hwnd) {
                    info.opacity = Some(opacity);
                }
            }
        }
    }

    /// Remember opacity for an app (exe name)
    pub fn remember_app_opacity(&mut self, exe: &str, opacity: f32) {
        if opacity < 1.0 {
            self.per_app_opacity.insert(exe.to_string(), opacity);
        } else {
            self.per_app_opacity.remove(exe);
        }
    }

    fn per_app_opacity_path(&self) -> std::path::PathBuf {
        let base = std::env::var("XDG_CONFIG_HOME").ok()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")));
        base.join(".config/ultrawm/per_app_opacity.json")
    }

    /// Check if UltraWM is set to run on startup
    pub fn is_startup_enabled() -> bool {
        use std::env::current_exe;
        if let Ok(startup_dir) = get_startup_dir() {
            let shortcut_path = startup_dir.join("UltraWM.lnk");
            return shortcut_path.exists();
        }
        false
    }

    /// Enable startup: create a shortcut in the Windows Startup folder
    pub fn enable_startup() -> anyhow::Result<()> {
        use std::env::current_exe;
        use std::process::Command;

        let startup_dir = get_startup_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find Startup folder"))?;
        std::fs::create_dir_all(&startup_dir)?;

        let shortcut_path = startup_dir.join("UltraWM.lnk");
        let exe_path = current_exe()?;
        let exe_str = exe_path.to_string_lossy().to_string();

        // Use PowerShell to create shortcut
        let ps_script = format!(
            r#"$WshShell = New-Object -ComObject WScript.Shell; $Shortcut = $WshShell.CreateShortcut("{}"); $Shortcut.TargetPath = "{}"; $Shortcut.Arguments = "--start"; $Shortcut.WorkingDirectory = "{}"; $Shortcut.WindowStyle = 7; $Shortcut.Save()"#,
            shortcut_path.display(),
            exe_str,
            exe_path.parent().unwrap_or(std::path::Path::new(".")).display()
        );

        let output = Command::new("powershell")
            .args(["-Command", &ps_script])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to create startup shortcut: {:?}", String::from_utf8_lossy(&output.stderr)));
        }

        info!("UltraWM added to startup: {}", shortcut_path.display());
        Ok(())
    }

    /// Disable startup: remove shortcut from the Windows Startup folder
    pub fn disable_startup() -> anyhow::Result<()> {
        if let Some(startup_dir) = get_startup_dir() {
            let shortcut_path = startup_dir.join("UltraWM.lnk");
            if shortcut_path.exists() {
                std::fs::remove_file(&shortcut_path)?;
                info!("UltraWM removed from startup: {}", shortcut_path.display());
            }
        }
        Ok(())
    }

    /// Enable idle inhibit — prevent screen lock/sleep
    pub fn enable_idle_inhibit(&mut self) {
        if !self.idle_inhibit {
            self.idle_inhibit = true;
            unsafe {
                let _ = SetThreadExecutionState(
                    ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED,
                );
            }
            info!("Idle inhibit enabled");
        }
    }

    /// Disable idle inhibit — allow screen lock/sleep
    pub fn disable_idle_inhibit(&mut self) {
        if self.idle_inhibit {
            self.idle_inhibit = false;
            unsafe {
                let _ = SetThreadExecutionState(ES_CONTINUOUS);
            }
            info!("Idle inhibit disabled");
        }
    }

    /// Toggle idle inhibit
    pub fn toggle_idle_inhibit(&mut self) {
        if self.idle_inhibit {
            self.disable_idle_inhibit();
        } else {
            self.enable_idle_inhibit();
        }
    }

    pub fn notify(&self, message: &str) {
        if let Some(ref notifier) = self.notifier {
            notifier.show(message);
        }
    }

    pub fn tick_notifier(&self) {
        if let Some(ref notifier) = self.notifier {
            let _ = notifier.tick();
        }
    }

    pub fn toggle_floating(&mut self) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            let was_floating = self.windows.get(&hwnd_wrapper).map(|i| i.floating).unwrap_or(false);
            let wid = self.windows.get(&hwnd_wrapper).map(|i| i.id);
            let hwnd = self.windows.get(&hwnd_wrapper).map(|i| i.hwnd);

            if let (Some(wid), Some(hwnd)) = (wid, hwnd) {
                // Flip floating flag
                if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                    info.floating = !was_floating;
                }

                if !was_floating {
                    // Center window on monitor at rule-specified or default size
                    unsafe {
                        let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL);
                        if !mon.is_invalid() {
                            let mut mi = MONITORINFO {
                                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                                ..Default::default()
                            };
                            if GetMonitorInfoW(mon, &mut mi).as_bool() {
                                let (fx, fy, fw, fh);
                                if let Some(info) = self.windows.get(&hwnd_wrapper) {
                                    fw = info.float_w.unwrap_or(self.config.layout.default_float_width).min((mi.rcWork.right - mi.rcWork.left) as u32);
                                    fh = info.float_h.unwrap_or(self.config.layout.default_float_height).min((mi.rcWork.bottom - mi.rcWork.top) as u32);
                                    fx = info.float_x.unwrap_or_else(|| mi.rcWork.left + ((mi.rcWork.right - mi.rcWork.left) - fw as i32) / 2);
                                    fy = info.float_y.unwrap_or_else(|| mi.rcWork.top + ((mi.rcWork.bottom - mi.rcWork.top) - fh as i32) / 2);
                                } else {
                                    fw = self.config.layout.default_float_width.min((mi.rcWork.right - mi.rcWork.left) as u32);
                                    fh = self.config.layout.default_float_height.min((mi.rcWork.bottom - mi.rcWork.top) as u32);
                                    fx = mi.rcWork.left + ((mi.rcWork.right - mi.rcWork.left) - fw as i32) / 2;
                                    fy = mi.rcWork.top + ((mi.rcWork.bottom - mi.rcWork.top) - fh as i32) / 2;
                                }
                                if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                                    info.saved_x = fx;
                                    info.saved_y = fy;
                                    info.saved_w = fw as i32;
                                    info.saved_h = fh as i32;
                                }
                                let _ = SetWindowPos(
                                    hwnd,
                                    HWND_TOPMOST,
                                    fx, fy, fw as i32, fh as i32,
                                    SWP_FRAMECHANGED,
                                );
                            }
                        }
                    }
                    let grid = self.current_grid();
                    grid.remove_window(wid);
                    info!("Floating window (id={})", wid);
                } else {
                    let grid = self.current_grid();
                    grid.place_window(wid);
                    grid.focus_window(wid);
                    info!("Unfloating window (id={})", wid);
                }
                self.tile_all_windows(0xFF7F7F7F, 0xFF454545);
                self.save_session();
            }
        }
    }

    pub fn toggle_sticky(&mut self) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                info.sticky = !info.sticky;
                info!("Sticky toggled: {} (id={}, sticky={})", info.title, info.id, info.sticky);
            }
        }
    }

    pub fn toggle_maximize(&mut self) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                info.maximized = !info.maximized;
                unsafe {
                    if info.maximized {
                        let mon = MonitorFromWindow(info.hwnd, MONITOR_DEFAULTTONULL);
                        if !mon.is_invalid() {
                            let mut mi = MONITORINFO {
                                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                                ..Default::default()
                            };
                            if GetMonitorInfoW(mon, &mut mi).as_bool() {
                                let _ = SetWindowPos(
                                    info.hwnd,
                                    HWND_NOTOPMOST,
                                    mi.rcWork.left,
                                    mi.rcWork.top,
                                    mi.rcWork.right - mi.rcWork.left,
                                    mi.rcWork.bottom - mi.rcWork.top,
                                    SWP_FRAMECHANGED,
                                );
                            }
                        }
                    } else {
                        let _ = SetWindowPos(
                            info.hwnd,
                            HWND_NOTOPMOST,
                            info.saved_x,
                            info.saved_y,
                            info.saved_w,
                            info.saved_h,
                            SWP_FRAMECHANGED,
                        );
                    }
                }
                info!("Maximize toggled: {} (id={}, maximized={})", info.title, info.id, info.maximized);
            }
        }
    }

    pub fn toggle_always_on_top(&mut self) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                info.always_on_top = !info.always_on_top;
                unsafe {
                    let _ = SetWindowPos(
                        info.hwnd,
                        if info.always_on_top { HWND_TOPMOST } else { HWND_NOTOPMOST },
                        info.saved_x,
                        info.saved_y,
                        info.saved_w,
                        info.saved_h,
                        SWP_NOMOVE | SWP_NOSIZE,
                    );
                }
                info!("Always-on-top toggled: {} (id={}, always_on_top={})", info.title, info.id, info.always_on_top);
            }
        }
    }

    pub fn adjust_gap(&mut self, delta: i32) {
        let new_gap = (self.config.layout.gaps as i32 + delta).max(0).min(100) as u32;
        self.config.layout.gaps = new_gap;
        info!("Gap adjusted: {}px", new_gap);
        let _ = self.config.save();
    }

    pub fn apply_rule_from_json(&mut self, json: serde_json::Value) {
        let match_str = json.get("match").and_then(|v| v.as_str()).unwrap_or("");
        if match_str.is_empty() {
            warn!("add-rule: missing 'match' field");
            return;
        }
        let float = json.get("float").and_then(|v| v.as_bool());
        let workspace = json.get("workspace").and_then(|v| v.as_u64()).map(|v| v as usize);
        let opacity = json.get("opacity").and_then(|v| v.as_f64()).map(|v| v as f32);
        let sticky = json.get("sticky").and_then(|v| v.as_bool());
        let max_width = json.get("max_width").and_then(|v| v.as_u64()).map(|v| v as u32);
        let max_height = json.get("max_height").and_then(|v| v.as_u64()).map(|v| v as u32);
        let min_width = json.get("min_width").and_then(|v| v.as_u64()).map(|v| v as u32);
        let min_height = json.get("min_height").and_then(|v| v.as_u64()).map(|v| v as u32);
        let float_x = json.get("float_x").and_then(|v| v.as_i64()).map(|v| v as i32);
        let float_y = json.get("float_y").and_then(|v| v.as_i64()).map(|v| v as i32);
        let float_w = json.get("float_w").and_then(|v| v.as_u64()).map(|v| v as u32);
        let float_h = json.get("float_h").and_then(|v| v.as_u64()).map(|v| v as u32);

        let rule = crate::config::WindowRule {
            match_: match_str.to_string(),
            float,
            workspace,
            width: None,
            height: None,
            max_width,
            max_height,
            min_width,
            min_height,
            float_x,
            float_y,
            float_w,
            float_h,
            opacity,
            sticky,
        };

        self.config.rules.push(rule);
        info!("Added rule: match='{}', float={:?}, float_pos={:?}x{:?}, float_size={:?}x{:?}", match_str, float, float_x, float_y, float_w, float_h);
    }

    pub fn import_rules_from_json(&mut self, rules: serde_json::Value) -> anyhow::Result<()> {
        let array = rules.as_array().ok_or_else(|| anyhow::anyhow!("expected JSON array of rules"))?;
        let mut count = 0;
        for rule_json in array {
            let match_str = rule_json.get("match").and_then(|v| v.as_str()).unwrap_or("");
            if match_str.is_empty() {
                warn!("import-rules: skipping rule with empty 'match' field");
                continue;
            }
            self.apply_rule_from_json(rule_json.clone());
            count += 1;
        }
        let _ = self.config.save();
        info!("Imported {} rules from JSON", count);
        Ok(())
    }

    pub fn unfloat_all(&mut self) {
        let count = self.windows.values().filter(|i| i.floating).count();
        for info in self.windows.values_mut() {
            if info.floating {
                info.floating = false;
                unsafe {
                    let _ = SetWindowPos(
                        info.hwnd,
                        HWND_NOTOPMOST,
                        info.saved_x,
                        info.saved_y,
                        info.saved_w,
                        info.saved_h,
                        SWP_FRAMECHANGED,
                    );
                }
            }
        }
        info!("Unfloated {} windows", count);
    }

    pub fn minimize_focused(&mut self) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                if !info.minimized {
                    unsafe { let _ = ShowWindow(info.hwnd, SW_MINIMIZE); }
                    info.minimized = true;
                    info!("Minimized: {} (id={})", info.title, info.id);
                }
            }
        }
    }

    pub fn restore_minimized(&mut self) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                if info.minimized {
                    unsafe { let _ = ShowWindow(info.hwnd, SW_RESTORE); }
                    info.minimized = false;
                    info!("Restored: {} (id={})", info.title, info.id);
                }
            }
        }
    }

    pub fn handle_ipc_command(&mut self, cmd: crate::ipc::IpcCommand, theme_mgr: &mut crate::theme::ThemeManager) {
        match cmd {
            crate::ipc::IpcCommand::Single { command } => {
                if command.starts_with("workspace-") {
                    if let Some(num) = command.strip_prefix("workspace-").and_then(|s| s.parse::<usize>().ok()) {
                        if num > 0 && num <= 4 {
                            self.switch_workspace(num - 1);
                        }
                    }
                    return;
                }
                if command.starts_with("move-window-to-workspace ") {
                    if let Some(num) = command.strip_prefix("move-window-to-workspace ").and_then(|s| s.parse::<usize>().ok()) {
                        self.move_focused_window_to_workspace(num);
                    }
                    return;
                }
                if command == "theme-next" {
                    self.cycle_theme(true);
                    return;
                }
                if command == "theme-prev" {
                    self.cycle_theme(false);
                    return;
                }
                if command == "sticky" {
                    self.toggle_sticky();
                    return;
                }
                if command == "minimize" {
                    self.minimize_focused();
                    return;
                }
                if command == "restore" {
                    self.restore_minimized();
                    return;
                }
                if command == "minimize-to-tray" {
                    self.minimize_to_tray();
                    return;
                }
                if command == "restore-from-tray" {
                    if let Some(focused) = self.focused_hwnd {
                        let wid = self.windows.get(&focused).map(|i| i.id);
                        if let Some(w) = wid {
                            self.restore_from_tray(w);
                        }
                    }
                    return;
                }
                if command == "restore-all-tray" {
                    self.restore_all_tray();
                    return;
                }
                if command == "maximize" {
                    self.toggle_maximize();
                    return;
                }
                if command == "always-on-top" {
                    self.toggle_always_on_top();
                    return;
                }
                if command.starts_with("add-rule:") {
                    if let Ok(rule_json) = serde_json::from_str::<serde_json::Value>(&command["add-rule:".len()..]) {
                        self.apply_rule_from_json(rule_json);
                    }
                    return;
                }
                if command == "grow-gap" {
                    self.adjust_gap(2);
                    return;
                }
                if command == "shrink-gap" {
                    self.adjust_gap(-2);
                    return;
                }
                if command == "unfloat-all" {
                    self.unfloat_all();
                    return;
                }
                if command == "reload-config" {
                    match crate::config::Config::load() {
                        Ok(new_config) => {
                            self.config = new_config;
                            info!("Config reloaded via IPC");
                        }
                        Err(e) => {
                            warn!("Config reload failed: {}", e);
                        }
                    }
                    return;
                }
                if command == "save-config" {
                    if let Err(e) = self.config.save() {
                        warn!("Config save failed: {}", e);
                    } else {
                        info!("Config saved via IPC");
                    }
                    return;
                }
                if command.starts_with("set-opacity ") {
                    if let Some(val) = command.strip_prefix("set-opacity ") {
                        if let Ok(op) = val.parse::<f32>() {
                            self.set_opacity(op);
                        }
                    }
                    return;
                }
                if command.starts_with("set-wallpaper ") {
                    if let Some(color) = command.strip_prefix("set-wallpaper ") {
                        let primary = self.primary_monitor().map(|m| (m.width(), m.height())).unwrap_or((1920, 1080));
                        if let Err(e) = crate::platform::wallpaper::apply_wallpaper(color, primary.0, primary.1) {
                            warn!("Wallpaper error: {}", e);
                        }
                    }
                    return;
                }
                if command.starts_with("set-wallpaper-image ") {
                    if let Some(path) = command.strip_prefix("set-wallpaper-image ") {
                        if let Err(e) = crate::platform::wallpaper::apply_wallpaper_image(path) {
                            warn!("Wallpaper image error: {}", e);
                        }
                    }
                    return;
                }
                if command.starts_with("set-gap ") {
                    if let Some(val) = command.strip_prefix("set-gap ") {
                        if let Ok(gap) = val.parse::<u32>() {
                            self.config.layout.gaps = gap;
                            info!("Gap set to {}", gap);
                            let _ = self.config.save();
                        }
                    }
                    return;
                }
                if command.starts_with("set-corner-radius ") {
                    if let Some(val) = command.strip_prefix("set-corner-radius ") {
                        if let Ok(r) = val.parse::<u32>() {
                            self.config.layout.corner_radius = r;
                            if let Some(overlay) = self.border_overlay.as_mut() {
                                overlay.border_radius = r as i32;
                            }
                            info!("Corner radius set to {}", r);
                            let _ = self.config.save();
                        }
                    }
                    return;
                }
                if command.starts_with("set-border-width ") {
                    if let Some(val) = command.strip_prefix("set-border-width ") {
                        if let Ok(bw) = val.parse::<u32>() {
                            self.config.layout.border_width = bw;
                            if let Some(overlay) = self.border_overlay.as_mut() {
                                overlay.border_width = bw as i32;
                            }
                            info!("Border width set to {}", bw);
                            let _ = self.config.save();
                        }
                    }
                    return;
                }
                if command == "idle-inhibit" {
                    self.enable_idle_inhibit();
                    return;
                }
                if command == "idle-noinhibit" {
                    self.disable_idle_inhibit();
                    return;
                }
                if command.starts_with("notify ") {
                    if let Some(msg) = command.strip_prefix("notify ") {
                        self.notify(msg);
                    }
                    return;
                }
                if command == "clamp-focused" {
                    self.clamp_focused_window();
                    return;
                }
                if command.starts_with("set-workspace-count ") {
                    if let Some(val) = command.strip_prefix("set-workspace-count ") {
                        if let Ok(count) = val.parse::<usize>() {
                            self.set_workspace_count(count);
                        }
                    }
                    return;
                }
                if command.starts_with("snap-layout ") {
                    if let Some(args) = command.strip_prefix("snap-layout ") {
                        let parts: Vec<&str> = args.split('x').collect();
                        if parts.len() == 2 {
                            if let (Ok(cols), Ok(rows)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                                self.snap_layout(cols, rows);
                            }
                        }
                    }
                    return;
                }
                if command.starts_with("snap-custom:") {
                    if let Some(name) = command.strip_prefix("snap-custom:") {
                        self.apply_custom_layout(name);
                    }
                    return;
                }
                if command.starts_with("layout-columns ") {
                    if let Some(n_str) = command.strip_prefix("layout-columns ") {
                        if let Ok(n) = n_str.parse::<usize>() {
                            self.layout_preset_columns(n.max(1));
                        }
                    }
                    return;
                }
                if command.starts_with("layout-rows ") {
                    if let Some(n_str) = command.strip_prefix("layout-rows ") {
                        if let Ok(n) = n_str.parse::<usize>() {
                            self.layout_preset_rows(n.max(1));
                        }
                    }
                    return;
                }
                if command == "layout-master" {
                    self.layout_preset_master();
                    return;
                }
                if command == "layout-fibonacci" {
                    self.layout_preset_fibonacci();
                    return;
                }
                if command == "toggle-monocle" {
                    self.toggle_monocle();
                    return;
                }
                if command.starts_with("layout-preset:") {
                    if let Some(name) = command.strip_prefix("layout-preset:") {
                        self.apply_layout_preset(name);
                    }
                    return;
                }
                if command.starts_with("set-window-opacity ") {
                    if let Some(val_str) = command.strip_prefix("set-window-opacity ") {
                        if let Ok(val) = val_str.parse::<f32>() {
                            self.set_opacity(val.clamp(0.0, 1.0));
                        }
                    }
                    return;
                }
                if command == "increase-opacity" {
                    self.adjust_opacity(0.05);
                    return;
                }
                if command == "decrease-opacity" {
                    self.adjust_opacity(-0.05);
                    return;
                }
                if command.starts_with("add-scratchpad ") {
                    if let Some(name) = command.strip_prefix("add-scratchpad ") {
                        self.add_scratchpad(name);
                    }
                    return;
                }
                if command.starts_with("remove-scratchpad ") {
                    if let Some(name) = command.strip_prefix("remove-scratchpad ") {
                        self.remove_scratchpad(name);
                    }
                    return;
                }
                if command == "swap-windows" {
                    if let Some(src) = self.focused_hwnd {
                        let grid = self.current_grid();
                        let src_id = self.windows.get(&src).map(|i| i.id);
                        if let Some(src_id) = src_id {
                            if let Some(&tgt_id) = grid.windows.get(1) {
                                if let Some(tgt_hwnd) = self.windows.iter().find(|(_, i)| i.id == tgt_id).map(|(hw, _)| *hw) {
                                    self.swap_windows(src, tgt_hwnd);
                                }
                            }
                        }
                    }
                    return;
                }
                if command == "toggle-bar" {
                    self.toggle_bar_visibility();
                    return;
                }
                if command == "bring-to-front" {
                    if let Some(hwnd) = self.focused_hwnd {
                        if let Some(info) = self.windows.get(&hwnd) {
                            if info.floating {
                                unsafe {
                                    let _ = SetWindowPos(hwnd.0, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                                }
                            }
                        }
                    }
                    return;
                }
                if command.starts_with("set-border-color ") {
                    if let Some(color_str) = command.strip_prefix("set-border-color ") {
                        if let Ok(color) = u32::from_str_radix(color_str.trim_start_matches("0x"), 16) {
                            if let Some(hwnd) = self.focused_hwnd {
                                if let Some(info) = self.windows.get_mut(&hwnd) {
                                    info.border_color = color;
                                }
                            }
                        }
                    }
                    return;
                }
                if command.starts_with("set-border-width ") {
                    if let Some(width_str) = command.strip_prefix("set-border-width ") {
                        if let Ok(width) = width_str.parse::<u32>() {
                            if let Some(hwnd) = self.focused_hwnd {
                                if let Some(info) = self.windows.get(&hwnd) {
                                    if width > 0 {
                                        self.window_border_widths.insert(info.id, width);
                                    } else {
                                        self.window_border_widths.remove(&info.id);
                                    }
                                }
                            }
                        }
                    }
                    return;
                }
                match command.as_str() {
                    "next-theme" => { let _ = theme_mgr.next_theme(); }
                    "prev-theme" => { let _ = theme_mgr.prev_theme(); }
                    "focus-next" => { self.move_focus(0, 1); }
                    "focus-prev" => { self.move_focus(0, -1); }
                    "focus-left" => { self.move_focus(0, -1); }
                    "focus-right" => { self.move_focus(0, 1); }
                    "focus-up" => { self.move_focus(-1, 0); }
                    "focus-down" => { self.move_focus(1, 0); }
                    "pan-left" => { self.pan_camera(0, -1); }
                    "pan-right" => { self.pan_camera(0, 1); }
                    "pan-up" => { self.pan_camera(-1, 0); }
                    "pan-down" => { self.pan_camera(1, 0); }
                    "grow-width" => { self.resize_width(true); }
                    "shrink-width" => { self.resize_width(false); }
                    "grow-height" => { self.resize_height(true); }
                    "shrink-height" => { self.resize_height(false); }
                    "close" => { self.close_focused(); }
                    "float" => { self.toggle_floating(); }
                    "unfloat" => { self.toggle_floating(); }
                    "launcher" => { self.toggle_launcher(); }
                    "overview" => { self.toggle_overview(); }
                    "scratchpad" => { self.toggle_scratchpad(); }
                    "fullscreen" => { self.toggle_fullscreen(); }
                    "split-horizontal" => { self.split_focused(true); }
                    "split-vertical" => { self.split_focused(false); }
                    "unsplit" => { self.unsplit_focused(); }
                    "tab" => { self.cycle_tab(true); }
                    "untab" => { self.untab_focused(); }
                    "quit" => {
                        log::info!("IPC: quit requested");
                        self.save_session();
                        unsafe { let _ = PostQuitMessage(0); }
                    }
                    _ => {}
                }
            }
            crate::ipc::IpcCommand::Batch { .. } => {
                // Batch commands handled in IPC thread directly
            }
        }
    }

    pub fn apply_rules(&mut self, win_info: &mut WindowInfo, raw_info: &WindowInfo) {
        // Sort rules by priority (lower first, higher overrides)
        let mut sorted_rules: Vec<_> = self.config.rules.iter().collect();
        sorted_rules.sort_by_key(|r| r.priority());

        for rule in sorted_rules {
            if !rule.matches_any(&raw_info.exe, &raw_info.class, &raw_info.title) {
                continue;
            }

            if let Some(float) = rule.float {
                win_info.floating = float;
            }
            if let Some(ws) = rule.workspace {
                if ws < self.config.layout.workspace_count {
                    self.window_workspaces.insert(win_info.id, ws);
                }
            }
            if let Some(mon) = rule.monitor {
                if mon < self.monitors.len() {
                    self.window_monitors.insert(win_info.id, mon);
                }
            }
            if let Some(op) = rule.opacity {
                if op >= 0.0 && op <= 1.0 {
                    win_info.opacity = Some(op);
                    // Animate to the target opacity using spring physics
                    self.opacity_anim.entry(win_info.id).or_insert_with(|| {
                        crate::anim::SpringValue::new(op).with_spring(crate::anim::Spring {
                            stiffness: 200.0,
                            damping: 25.0,
                            mass: 1.0,
                        })
                    }).set_target(op);
                }
            }
            if let Some(sticky) = rule.sticky {
                win_info.sticky = sticky;
            }
            if let Some(aot) = rule.always_on_top {
                win_info.always_on_top = aot;
            }
            if let Some(fs) = rule.fullscreen {
                win_info.fullscreen = fs;
            }
            if let Some(bc) = rule.border_color {
                win_info.border_color = bc;
            }
            if let Some(bw) = rule.border_width {
                if bw > 0 {
                    self.window_border_widths.insert(win_info.id, bw);
                }
            }
            if let Some(cr) = rule.corner_radius {
                // Apply per-app corner radius via DWM window corner preference
                self.apply_rounded_corners(win_info.hwnd, cr);
            }
            if let Some(mw) = rule.max_width {
                if mw > 0 {
                    win_info.max_width = Some(mw);
                }
            }
            if let Some(mh) = rule.max_height {
                if mh > 0 {
                    win_info.max_height = Some(mh);
                }
            }
            if let Some(mw) = rule.min_width {
                if mw > 0 {
                    win_info.min_width = Some(mw);
                }
            }
            if let Some(mh) = rule.min_height {
                if mh > 0 {
                    win_info.min_height = Some(mh);
                }
            }
            if let Some(w) = rule.width {
                if w > 0 {
                    win_info.float_w = Some(w);
                }
            }
            if let Some(h) = rule.height {
                if h > 0 {
                    win_info.float_h = Some(h);
                }
            }
            if let Some(fx) = rule.float_x {
                win_info.float_x = Some(fx);
            }
            if let Some(fy) = rule.float_y {
                win_info.float_y = Some(fy);
            }
            if let Some(fw) = rule.float_w {
                win_info.float_w = Some(fw);
            }
            if let Some(fh) = rule.float_h {
                win_info.float_h = Some(fh);
            }
        }
    }

    pub fn apply_z_order(&mut self) {
        let mut sorted: Vec<_> = self.windows.iter().collect();
        sorted.sort_by_key(|(_, info)| info.z_order);
        for (hwnd_wrapper, _) in sorted {
            unsafe {
                let _ = SetWindowPos(hwnd_wrapper.0, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            }
        }
    }

    pub fn restore_session(&mut self) {
        let session = match &self.session {
            Some(s) => s,
            None => return,
        };

        if session.monitors.is_empty() {
            return;
        }

        // Build exe -> list of (hwnd, info) for matching
        let mut exe_map: HashMap<String, Vec<(HWnd, &WindowInfo)>> = HashMap::new();
        for (&hwnd_wrapper, info) in &self.windows {
            exe_map.entry(info.exe.clone()).or_default().push((hwnd_wrapper, info));
        }

        // Track which running windows have been matched to avoid double-assignment
        let mut matched: std::collections::HashSet<usize> = std::collections::HashSet::new();

        let mut restore_plan: Vec<(HWnd, usize, usize, bool, Option<i32>, Option<i32>, Option<i32>, Option<i32>)> = Vec::new();

        for (mon_idx, mon_state) in session.monitors.iter().enumerate() {
            for grid in &mon_state.grids {
                for sw in &grid.windows {
                    if let Some(candidates) = exe_map.get(&sw.exe) {
                        // Find first unmatched candidate
                        for &(hwnd_wrapper, info) in candidates {
                            if matched.insert(hwnd_wrapper.0 .0 as usize) {
                                let fx = sw.float_x.or(sw.x).or(info.float_x).or(Some(info.saved_x));
                                let fy = sw.float_y.or(sw.y).or(info.float_y).or(Some(info.saved_y));
                                let fw = sw.float_w.or(sw.w).or(info.float_w.map(|w| w as i32)).or(Some(info.saved_w as i32));
                                let fh = sw.float_h.or(sw.h).or(info.float_h.map(|h| h as i32)).or(Some(info.saved_h as i32));
                                restore_plan.push((
                                    hwnd_wrapper,
                                    sw.monitor.min(mon_idx.max(sw.monitor).min(self.monitors.len().saturating_sub(1))),
                                    sw.workspace,
                                    sw.floating,
                                    fx,
                                    fy,
                                    fw,
                                    fh,
                                ));
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Apply restore plan
        for (hwnd_wrapper, mon_idx, ws, floating, fx, fy, fw, fh) in restore_plan {
            let wid = if let Some(info) = self.windows.get(&hwnd_wrapper) {
                info.id
            } else {
                continue;
            };

            // Assign to correct monitor workspace
            if mon_idx < self.monitor_workspaces.len() {
                self.window_monitors.insert(wid, mon_idx);
            }

            // Assign to correct workspace
            let target_ws = ws.min(self.monitor_workspaces.get(mon_idx).map(|m| m.grids.len().saturating_sub(1)).unwrap_or(0));
            self.window_workspaces.insert(wid, target_ws);

            // Restore floating state
            if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                info.floating = floating;
                if let Some(x) = fx { info.float_x = Some(x); info.saved_x = x; }
                if let Some(y) = fy { info.float_y = Some(y); info.saved_y = y; }
                if let Some(w) = fw { info.float_w = Some(w as u32); info.saved_w = w; }
                if let Some(h) = fh { info.float_h = Some(h as u32); info.saved_h = h; }
                info.maximized = false;
            }

            // Switch to correct workspace and position the window
            if mon_idx < self.monitor_workspaces.len() {
                let mws = &mut self.monitor_workspaces[mon_idx];
                let prev_ws = mws.current;
                mws.current = target_ws;

                unsafe {
                    let _ = ShowWindow(hwnd_wrapper.0, SW_SHOW);
                    if floating {
                        let px = fx.unwrap_or(100);
                        let py = fy.unwrap_or(100);
                        let pw = fw.unwrap_or(800).max(100) as i32;
                        let ph = fh.unwrap_or(600).max(100) as i32;
                        let _ = SetWindowPos(hwnd_wrapper.0, HWND_TOP, px, py, pw, ph, SWP_SHOWWINDOW);
                    }
                }

                mws.current = prev_ws;
            }
        }

        info!("Session restored: {} windows repositioned", restore_plan.len());
    }

    pub fn save_session(&mut self) {
        // Build Z-order map: HWND raw ptr -> position (0 = topmost)
        let mut z_order_map: HashMap<usize, usize> = HashMap::new();
        unsafe {
            if let Ok(mut hwnd) = GetTopWindow(HWND(null_mut())) {
                let mut z = 0usize;
                while !hwnd.is_invalid() {
                    z_order_map.insert(hwnd.0 as usize, z);
                    z += 1;
                    if let Ok(next) = GetWindow(hwnd, GW_HWNDNEXT) {
                        hwnd = next;
                    } else {
                        break;
                    }
                }
            }
        }

        let mut monitor_states: Vec<crate::session::MonitorSessionState> = Vec::new();

        for mws in &self.monitor_workspaces {
            let mut grid_states: Vec<crate::session::GridSessionState> = Vec::new();

            for grid in &mws.grids {
                let mut ws_windows: Vec<crate::session::SessionWindowState> = Vec::new();

                // Save tiled windows from this grid
                for (&wid, &cell) in &grid.window_positions {
                    if let Some(info) = self.windows.values().find(|i| i.id == wid) {
                        let hwnd_key = HWnd(info.hwnd);
                        let z_order = z_order_map.get(&(hwnd_key.0 .0 as usize)).copied().unwrap_or(usize::MAX);
                        // Get actual window position/size if available
                        let (wx, wy, ww, wh) = if info.floating {
                            (info.float_x, info.float_y, info.float_w, info.float_h)
                        } else {
                            // For tiled windows, save the current rect
                            if let Ok(rect) = get_window_rect(info.hwnd) {
                                (Some(rect.0), Some(rect.1), Some(rect.2), Some(rect.3))
                            } else {
                                (None, None, None, None)
                            }
                        };

                        let monitor_idx = self.window_monitors.get(&wid).copied().unwrap_or(0);

                        ws_windows.push(crate::session::SessionWindowState {
                            exe: info.exe.clone(),
                            cell,
                            floating: info.floating,
                            workspace: mws.current,
                            monitor: monitor_idx,
                            opacity: info.opacity,
                            sticky: info.sticky,
                            maximized: info.maximized,
                            always_on_top: info.always_on_top,
                            z_order,
                            x: wx,
                            y: wy,
                            w: ww,
                            h: wh,
                            float_x: info.float_x,
                            float_y: info.float_y,
                            float_w: info.float_w.map(|w| w as i32),
                            float_h: info.float_h.map(|h| h as i32),
                        });
                    }
                }

                // Save floating windows belonging to this monitor that aren't in any grid
                for info in self.windows.values() {
                    if info.floating {
                        let is_in_grid = self.monitor_workspaces.iter()
                            .any(|m| m.grids.iter().any(|g| g.window_positions.contains_key(&info.id)));
                        if !is_in_grid {
                            if let Some(&mon_idx) = self.window_monitors.get(&info.id) {
                                if mon_idx == self.monitor_workspaces.iter().position(|m| m.monitor.handle == mws.monitor.handle).unwrap_or(usize::MAX) {
                                    let hwnd_key = HWnd(info.hwnd);
                                    let z_order = z_order_map.get(&(hwnd_key.0 .0 as usize)).copied().unwrap_or(usize::MAX);
                                    let (fx, fy, fw, fh) = if let Ok(rect) = get_window_rect(info.hwnd) {
                                        (Some(rect.0), Some(rect.1), Some(rect.2), Some(rect.3))
                                    } else {
                                        (info.float_x, info.float_y, info.float_w, info.float_h)
                                    };

                                    let monitor_idx = self.window_monitors.get(&info.id).copied().unwrap_or(0);

                                    ws_windows.push(crate::session::SessionWindowState {
                                        exe: info.exe.clone(),
                                        cell: crate::layout::Cell::new(0, 0),
                                        floating: true,
                                        workspace: mws.current,
                                        monitor: monitor_idx,
                                        opacity: info.opacity,
                                        sticky: info.sticky,
                                        maximized: info.maximized,
                                        always_on_top: info.always_on_top,
                                        z_order,
                                        x: fx,
                                        y: fy,
                                        w: fw,
                                        h: fh,
                                        float_x: info.float_x,
                                        float_y: info.float_y,
                                        float_w: info.float_w.map(|w| w as i32),
                                        float_h: info.float_h.map(|h| h as i32),
                                    });
                                }
                            }
                        }
                    }
                }

                grid_states.push(crate::session::GridSessionState {
                    windows: ws_windows,
                    camera: grid.camera,
                    focused: grid.focused_window,
                });
            }

            monitor_states.push(crate::session::MonitorSessionState {
                grids: grid_states,
                current: mws.current,
            });
        }

        let state = crate::session::SessionState {
            version: 2,
            monitors: monitor_states,
        };

        if let Err(e) = state.save() {
            warn!("Failed to save session: {}", e);
        }
        self.save_per_app_opacity();
    }

    pub fn toggle_scratchpad(&mut self) {
        if self.scratchpad.is_none() {
            if let Ok(()) = ScratchpadManager::create() {
                ScratchpadManager::create().ok();
            }
        }
        if let Some(ref mut sp) = self.scratchpad {
            sp.toggle();
        }
    }

    pub fn add_scratchpad(&mut self, name: &str) {
        if self.scratchpad.is_none() {
            if let Ok(()) = ScratchpadManager::create() {
                ScratchpadManager::create().ok();
            }
        }
        if let Some(ref mut sp) = self.scratchpad {
            // Find the focused window and add it to scratchpad
            if let Some(hwnd_wrapper) = self.focused_hwnd {
                sp.add(hwnd_wrapper.0, name.to_string());
                info!("Added scratchpad: {} (hwnd={:?})", name, hwnd_wrapper.0);
            }
        }
    }

    pub fn remove_scratchpad(&mut self, name: &str) {
        if let Some(ref mut sp) = self.scratchpad {
            if let Some(win) = sp.find_by_name(name) {
                sp.remove(win.hwnd);
                info!("Removed scratchpad: {}", name);
            }
        }
    }

    pub fn cycle_theme(&mut self, forward: bool) {
        if let Some(ref mgr) = self.theme_mgr {
            let mut m = mgr.borrow_mut();
            if forward {
                let _ = m.next_theme();
            } else {
                let _ = m.prev_theme();
            }
            let primary = self.primary_monitor().map(|m| (m.width(), m.height())).unwrap_or((1920, 1080));
            let bg = m.current_theme().background.clone();
            let accent = m.current_theme().accent.clone();
            let _ = crate::platform::wallpaper::apply_theme_wallpaper(&bg, &accent, primary.0, primary.1);
        }
    }

    pub fn toggle_theme_picker(&mut self) {
        unsafe {
            if !theme_picker::THEME_PICKER_PTR.is_null() {
                drop(Box::from_raw(theme_picker::THEME_PICKER_PTR));
                theme_picker::THEME_PICKER_PTR = std::ptr::null_mut();
            } else {
                let themes = match ThemeManager::list_themes() {
                    Ok(t) => t,
                    Err(_) => return,
                };
                let current = self.theme_mgr.as_ref().map(|m| m.borrow().theme_count()).unwrap_or(0);
                let _ = ThemePicker::create(themes, current.saturating_sub(1));
            }
        }
    }

    pub fn apply_theme_by_idx(&mut self, idx: usize) {
        if let Some(ref mgr) = self.theme_mgr {
            let mut m = mgr.borrow_mut();
            let _ = m.apply_idx(idx);
            let primary = self.primary_monitor().map(|m| (m.width(), m.height())).unwrap_or((1920, 1080));
            let bg = m.current_theme().background.clone();
            let accent = m.current_theme().accent.clone();
            let _ = wallpaper::apply_theme_wallpaper(&bg, &accent, primary.0, primary.1);
        }
    }

    pub fn next_theme(&mut self) {
        if let Some(ref mgr) = self.theme_mgr {
            let mut m = mgr.borrow_mut();
            let _ = m.next_theme();
            let primary = self.primary_monitor().map(|m| (m.width(), m.height())).unwrap_or((1920, 1080));
            let bg = m.current_theme().background.clone();
            let accent = m.current_theme().accent.clone();
            let _ = wallpaper::apply_theme_wallpaper(&bg, &accent, primary.0, primary.1);
        }
    }

    pub fn toggle_overview(&mut self) {
        self.overview = !self.overview;
        if self.overview {
            if let Some(ref overlay) = self.border_overlay {
                overlay.set_transparent(false);
            }
            log::info!("Overview mode ON — {} windows visible", self.windows.len());
        } else {
            if let Some(ref overlay) = self.border_overlay {
                overlay.set_transparent(true);
            }
            self.overview_positions.clear();
            log::info!("Overview mode OFF");
        }
    }

    pub fn toggle_monocle(&mut self) {
        self.monocle = !self.monocle;
        if self.monocle {
            log::info!("Monocle mode ON");
        } else {
            log::info!("Monocle mode OFF");
        }
    }

    pub fn toggle_launcher(&mut self) {
        unsafe {
            if !launcher::LAUNCHER_PTR.is_null() {
                // Dismiss existing launcher
                drop(Box::from_raw(launcher::LAUNCHER_PTR));
                launcher::LAUNCHER_PTR = std::ptr::null_mut();
            } else if let Err(e) = AppLauncher::create() {
                warn!("Launcher creation failed: {}", e);
            }
        }
    }

    pub fn show_window_search(&mut self) {
        unsafe {
            if !window_search::SEARCH_PTR.is_null() {
                // Dismiss existing search
                drop(Box::from_raw(window_search::SEARCH_PTR));
                window_search::SEARCH_PTR = std::ptr::null_mut();
            } else if let Ok(_) = WindowSearch::create() {
                if let Some(ref mut search) = window_search::SEARCH_PTR.as_mut() {
                    search.populate(self);
                }
            }
        }
    }

    pub fn toggle_help(&mut self) {
        unsafe {
            if !help_overlay::HELP_PTR.is_null() {
                // Dismiss existing help
                drop(Box::from_raw(help_overlay::HELP_PTR));
                help_overlay::HELP_PTR = std::ptr::null_mut();
            } else if let Err(e) = HelpOverlay::create() {
                warn!("Help overlay creation failed: {}", e);
            }
        }
    }

    pub fn quit(&mut self) {
        self.save_session();
        self.save_per_app_opacity();
        unsafe {
            let _ = PostQuitMessage(0);
        }
    }

    pub fn diagnose(&self) -> anyhow::Result<()> {
        println!("=== UltraWM Diagnostics ===");

        // DWM status
        println!("DWM:");
        unsafe {
            if let Ok(enabled) = DwmIsCompositionEnabled() {
                println!("  Composition: {}", if enabled.as_bool() { "enabled" } else { "disabled" });
            } else {
                println!("  Composition: unknown");
            }
        }

        // DPI awareness
        println!("DPI:");
        if let Some(m) = self.monitors.first() {
            println!("  Primary monitor DPI: {}", m.dpi);
        }

        // Shell replacement
        println!("Shell:");
        if let Ok(key) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon")
        {
            let shell: String = key.get_value("Shell").unwrap_or_default();
            println!("  Current shell: {}", shell);
            let is_ultrawm = shell.contains("ultrawm");
            println!("  UltraWM as shell: {}", if is_ultrawm { "yes" } else { "no" });
        }

        // Startup
        println!("Startup: {}", if Platform::is_startup_enabled() { "enabled" } else { "disabled" });

        // Config
        println!("Config:");
        let config_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".config/ultrawm/config.toml");
        println!("  Path: {}", config_path.display());
        println!("  Exists: {}", config_path.exists());
        if let Some(modified) = &self.config.last_modified {
            println!("  Last modified: {:?}", modified);
        }
        println!("  Corner radius: {}", self.config.layout.corner_radius);
        println!("  Gaps: {}", self.config.layout.gaps);
        println!("  Border width: {}", self.config.layout.border_width);
        println!("  Inner padding: {}", self.config.layout.inner_padding);

        // Themes
        println!("Themes:");
        let themes = ThemeManager::list_themes().unwrap_or_default();
        println!("  Available: {} themes", themes.len());
        for t in &themes {
            println!("    - {}", t);
        }

        // Monitors
        println!("Monitors: {}", self.monitors.len());
        for (i, m) in self.monitors.iter().enumerate() {
            println!(
                "  Monitor {}: {}x{} at ({},{}) work={}x{} DPI={}",
                i, m.width(), m.height(), m.left, m.top, m.work_width(), m.work_height(), m.dpi
            );
        }

        // Workspaces
        println!("Workspaces:");
        for (mi, mws) in self.monitor_workspaces.iter().enumerate() {
            println!("  Monitor {}: workspace {} ({} grids)", mi, mws.current + 1, mws.grids.len());
            let grid = &mws.grids[mws.current];
            println!("    Windows: {}", grid.window_positions.len());
            println!("    Camera: ({}, {})", grid.camera.row, grid.camera.col);
            for (&wid, &cell) in &grid.window_positions {
                println!("    window {}: cell ({}, {})", wid, cell.row, cell.col);
            }
        }

        // Windows
        println!("Managed windows: {}", self.windows.len());
        for (hwnd_wrapper, info) in &self.windows {
            println!(
                "  {:?}: {} (class={}, exe={}, id={}, float={})",
                hwnd_wrapper.0, info.title, info.class, info.exe, info.id, info.floating
            );
        }

        // Bar
        println!("Bar: {}", if self.bar.is_some() { "enabled" } else { "disabled" });

        // Border overlay
        println!("Border overlay: {}", if self.border_overlay.is_some() { "enabled" } else { "disabled" });

        // Keyboard hook
        println!("Keyboard hook: {}", if self.keyboard_hook.is_some() { "enabled" } else { "disabled" });

        // Session
        println!("Session: {}", if self.session.is_some() { "saved" } else { "none" });

        // Per-app opacity memory
        println!("Per-app opacity: {} entries", self.per_app_opacity.len());
        for (exe, opacity) in &self.per_app_opacity {
            println!("  {}: {:.0}%", exe, opacity * 100.0);
        }

        // System info
        println!("CPU: {}%", self.get_cpu_usage());
        println!("Monocle mode: {}", if self.monocle { "ON" } else { "OFF" });

        Ok(())
    }

    /// Read CPU usage percentage using GetSystemTimes
    pub fn get_cpu_usage(&self) -> u32 {
        unsafe {
            let mut idle_time = FILETIME::default();
            let mut kernel_time = FILETIME::default();
            let mut user_time = FILETIME::default();

            if GetSystemTimes(&mut idle_time, &mut kernel_time, &mut user_time).is_ok() {
                let idle = (idle_time.dwHighDateTime as u64) << 32 | idle_time.dwLowDateTime as u64;
                let kernel = (kernel_time.dwHighDateTime as u64) << 32 | kernel_time.dwLowDateTime as u64;
                let user = (user_time.dwHighDateTime as u64) << 32 | user_time.dwLowDateTime as u64;

                let total = kernel + user;
                if total > 0 {
                    let idle_percent = (idle * 100) / total;
                    return (100 - idle_percent) as u32;
                }
            }
        }
        0
    }

    /// Read memory usage percentage using GlobalMemoryStatusEx
    pub fn get_memory_usage(&self) -> u32 {
        unsafe {
            let mut info = MEMORYSTATUSEX::default();
            info.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(&mut info).is_ok() {
                let total = info.ullTotalPhys;
                let avail = info.ullAvailPhys;
                if total > 0 {
                    let used = total - avail;
                    return ((used * 100) / total) as u32;
                }
            }
        }
        0
    }

    /// Update bar with system info (CPU, memory, clock) at reduced frequency
    pub fn update_bar_system_info(&mut self) {
        if let Some(ref bar) = self.bar {
            // Update clock every frame
            let now = Local::now();
            let clock_str = now.format("%H:%M").to_string();
            bar.set_clock(&clock_str);

            // Update CPU and memory every 60 frames (~1 second)
            if self.config_reload_counter % 60 == 0 {
                if self.config.bar.show_cpu {
                    let cpu = self.get_cpu_usage();
                    bar.set_cpu(cpu);
                }
                if self.config.bar.show_memory {
                    let mem = self.get_memory_usage();
                    bar.set_memory(mem);
                }
            }
        }
    }

    /// Take a screenshot of the specified window or full screen and save to PNG
    pub fn take_screenshot(&self, hwnd: Option<HWND>, output_path: &str) -> anyhow::Result<()> {
        unsafe {
            let (src_hwnd, width, height, x, y) = if let Some(hwnd) = hwnd {
                // Capture specific window
                let mut rect = RECT::default();
                GetWindowRect(hwnd, &mut rect)?;
                let w = (rect.right - rect.left).max(1);
                let h = (rect.bottom - rect.top).max(1);
                (hwnd, w, h, rect.left, rect.top)
            } else {
                // Capture full virtual screen
                let w = GetSystemMetrics(SM_CXSCREEN).max(1);
                let h = GetSystemMetrics(SM_CYSCREEN).max(1);
                (HWND(GetDesktopWindow().0), w, h, 0, 0)
            };

            let hdc_src = GetDC(src_hwnd);
            let hdc_mem = CreateCompatibleDC(hdc_src);
            let bitmap = CreateCompatibleBitmap(hdc_src, width, height);
            let old_bitmap = SelectObject(hdc_mem, bitmap);

            // Copy screen to bitmap
            BitBlt(hdc_mem, 0, 0, width, height, hdc_src, x, y, SRCCOPY | CAPTUREBLT);

            // Get bitmap bits
            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                bmiColors: [Default::default(); 1],
            };

            let mut buffer = vec![0u8; (width * height * 4) as usize];
            if GetDIBits(hdc_mem, bitmap, 0, height, Some(buffer.as_mut_ptr() as *mut _), &mut bmi, DIB_RGB_COLORS) == 0 {
                return Err(anyhow::anyhow!("GetDIBits failed"));
            }

            // Convert BGRA to RGBA
            for pixel in buffer.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }

            // Save as PNG
            let mut png_file = std::fs::File::create(output_path)?;
            let mut encoder = png::Encoder::new(&mut png_file, width as u32, height as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header()?;
            writer.write_image_data(&buffer)?;
            writer.finish()?;

            // Cleanup
            SelectObject(hdc_mem, old_bitmap);
            DeleteObject(bitmap);
            DeleteDC(hdc_mem);
            ReleaseDC(src_hwnd, hdc_src);

            info!("Screenshot saved to {}", output_path);
            Ok(())
        }
    }
}

// ============ Win32 Helpers ============

fn enumerate_monitors() -> anyhow::Result<Vec<MonitorInfo>> {
    let mut monitors = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(enum_monitors_proc),
            LPARAM(&mut monitors as *mut _ as isize),
        );
    }
    if monitors.is_empty() {
        // Fallback: single monitor from system metrics
        let w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        let (wl, wt, wr, wb) = get_work_area()?;
        monitors.push(MonitorInfo {
            handle: HMONITOR(std::ptr::null_mut()),
            left: 0,
            top: 0,
            right: w,
            bottom: h,
            work_left: wl,
            work_top: wt,
            work_right: wr,
            work_bottom: wb,
            dpi: 96,
            scale_factor: 1.0,
        });
    }
    Ok(monitors)
}

unsafe extern "system" fn enum_monitors_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let monitors = &mut *(data.0 as *mut Vec<MonitorInfo>);
    let mut mi = MONITORINFO::default();
    mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    if GetMonitorInfoW(hmonitor, &mut mi).as_bool() {
        let mut dpi_x = 96u32;
        let mut dpi_y = 96u32;
        let _ = GetDpiForMonitor(hmonitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        let scale = dpi_x as f32 / 96.0;
        monitors.push(MonitorInfo {
            handle: hmonitor,
            left: mi.rcMonitor.left,
            top: mi.rcMonitor.top,
            right: mi.rcMonitor.right,
            bottom: mi.rcMonitor.bottom,
            work_left: mi.rcWork.left,
            work_top: mi.rcWork.top,
            work_right: mi.rcWork.right,
            work_bottom: mi.rcWork.bottom,
            dpi: dpi_x,
            scale_factor: scale,
        });
    }
    TRUE
}

fn is_running_as_shell() -> bool {
    unsafe {
        let hwnd_shell = FindWindowW(w!("Shell_TrayWnd"), None).ok();
        hwnd_shell.map(|h| !h.is_invalid()).unwrap_or(false)
    }
}

fn launch_explorer() {
    use std::process::Command;
    let _ = Command::new("explorer.exe").spawn();
}

fn get_work_area() -> anyhow::Result<(i32, i32, i32, i32)> {
    unsafe {
        let mut rect = RECT::default();
        if SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut rect as *mut _ as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_ok()
        {
            Ok((rect.left, rect.top, rect.right, rect.bottom))
        } else {
            Ok((0, 0, 1920, 1080))
        }
    }
}

fn is_window_alive(hwnd: HWND) -> bool {
    unsafe { IsWindow(hwnd).as_bool() }
}

fn find_nearest_window(
    grid: &GridState,
    removed_id: u64,
) -> Option<u64> {
    let removed_cell = grid.window_positions.get(&removed_id)?;

    let mut best: Option<(u64, i32)> = None;
    for (&wid, &cell) in &grid.window_positions {
        if wid == removed_id {
            continue;
        }
        let dist = (cell.row - removed_cell.row).abs() + (cell.col - removed_cell.col).abs();
        match best {
            None => best = Some((wid, dist)),
            Some((_, d)) if dist < d => best = Some((wid, dist)),
            _ => {}
        }
    }

    best.map(|(wid, _)| wid)
}

fn hex_to_rgb(hex: &str) -> u32 {
    let s = hex.trim_start_matches('#');
    if s.len() == 6 {
        let r = u32::from_str_radix(&s[0..2], 16).unwrap_or(0);
        let g = u32::from_str_radix(&s[2..4], 16).unwrap_or(0);
        let b = u32::from_str_radix(&s[4..6], 16).unwrap_or(0);
        (b << 16) | (g << 8) | r
    } else {
        0
    }
}

fn exe_hash_color(exe: &str) -> u32 {
    use std::hash::{DefaultHasher, Hasher};
    let mut hasher = DefaultHasher::new();
    hasher.write(exe.as_bytes());
    let h = hasher.finish();
    let r = ((h >> 0) & 0xFF) as u32;
    let g = ((h >> 8) & 0xFF) as u32;
    let b = ((h >> 16) & 0xFF) as u32;
    (b << 16) | ((g & 0xFF) << 8) | (r & 0xFF)
}

fn apply_rounded_corners(hwnd: HWND, x: i32, y: i32, w: i32, h: i32, radius: i32) {
    if w <= radius * 2 || h <= radius * 2 {
        return;
    }
    unsafe {
        let region = CreateRoundRectRgn(
            x, y, x + w, y + h,
            radius * 2, radius * 2,
        );
        if !region.is_invalid() {
            let _ = SetWindowRgn(hwnd, region, FALSE);
        }
    }
}

fn blend_color(flash: u32, base: u32, t: f32) -> u32 {
    let fr = (flash & 0xFF) as f32;
    let fg = ((flash >> 8) & 0xFF) as f32;
    let fb = ((flash >> 16) & 0xFF) as f32;
    let br = (base & 0xFF) as f32;
    let bg = ((base >> 8) & 0xFF) as f32;
    let bb = ((base >> 16) & 0xFF) as f32;
    let r = (fr * t + br * (1.0 - t)) as u32;
    let g = (fg * t + bg * (1.0 - t)) as u32;
    let b = (fb * t + bb * (1.0 - t)) as u32;
    (b << 16) | ((g & 0xFF) << 8) | (r & 0xFF)
}

fn dim_color(color: u32, factor: f32) -> u32 {
    let r = ((color & 0xFF) as f32 * factor) as u32;
    let g = (((color >> 8) & 0xFF) as f32 * factor) as u32;
    let b = (((color >> 16) & 0xFF) as f32 * factor) as u32;
    (b << 16) | ((g & 0xFF) << 8) | (r & 0xFF)
}

fn get_window_rect(hwnd: HWND) -> anyhow::Result<(i32, i32, i32, i32)> {
    unsafe {
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            Ok((rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top))
        } else {
            Err(anyhow::anyhow!("Failed to get window rect"))
        }
    }
}

fn enable_dwm_shadow(hwnd: HWND) {
    unsafe {
        let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, WS_CAPTION.0 as isize);
        let mut policy = DWMNCRP_ENABLED;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_NCRENDERING_POLICY,
            &mut policy as *mut _ as *const _,
            std::mem::size_of::<DWMNCRENDERINGPOLICY>() as u32,
        );
        let disabled = 0u32;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &disabled as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

fn get_window_title(hwnd: HWND) -> Option<String> {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len == 0 {
            return None;
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        let _ = GetWindowTextW(hwnd, &mut buf);
        let title = String::from_utf16_lossy(&buf[..len as usize]);
        if title.is_empty() { None } else { Some(title) }
    }
}

// ============ Win32 Callbacks ============

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let platform = &mut *(lparam.0 as *mut Platform);

    if !IsWindowVisible(hwnd).as_bool() {
        return TRUE;
    }

    let mut class_name = [0u16; 256];
    let len = GetClassNameW(hwnd, &mut class_name);
    if len > 0 {
        let class = String::from_utf16_lossy(&class_name[..len as usize]);
        if class == "Shell_TrayWnd"
            || class == "Shell_SecondaryTrayWnd"
            || class.contains("UltraWM")
        {
            return TRUE;
        }
    }

    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
    if (ex_style & WS_EX_TOOLWINDOW.0 as i32) != 0 {
        return TRUE;
    }

    if let Ok(info) = WindowInfo::from_hwnd(hwnd) {
        if info.should_tile() {
            let wid = platform.next_id;
            platform.next_id += 1;
            let hwnd_wrapper = HWnd(hwnd);
            let mut win_info = info;
            win_info.id = wid;
            platform.windows.insert(hwnd_wrapper, win_info);

            // Assign to monitor and place in that monitor's current workspace
            let mon_idx = if let Some(mon) = platform.monitor_for_hwnd(hwnd) {
                platform.monitors.iter().position(|m| m.handle == mon.handle).unwrap_or(0)
            } else {
                0
            };
            platform.window_monitors.insert(wid, mon_idx);
            let ws_idx = platform.monitor_workspaces[mon_idx].current;
            platform.window_workspaces.insert(wid, ws_idx);
            platform.monitor_workspaces[mon_idx].grids[ws_idx].place_window(wid);
        }
    }

    TRUE
}

unsafe extern "system" fn win_event_proc(
    _hwin_event_hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    let platform = match keyboard::PLATFORM_PTR.as_mut() {
        Some(p) => p,
        None => return,
    };

    match event {
        EVENT_SYSTEM_FOREGROUND => {
            if !hwnd.is_invalid() && is_window_alive(hwnd) {
                platform.on_focus_changed(hwnd);
            }
        }
        EVENT_OBJECT_DESTROY => {
            if !hwnd.is_invalid() {
                platform.remove_window(hwnd);
                platform.save_session();
            }
        }
        EVENT_OBJECT_SHOW => {
            if !hwnd.is_invalid() && is_window_alive(hwnd) {
                platform.manage_window(hwnd);
            }
        }
        EVENT_OBJECT_LOCATIONCHANGE => {
            if !hwnd.is_invalid() && is_window_alive(hwnd) {
                platform.snap_floating_window(hwnd);
            }
        }
        _ => {}
    }
}

unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::UI::Shell::*;

    const WM_APP_TRAY: u32 = WM_APP + 0x200;
    const WM_APP_TRAYMENU: u32 = WM_APP + 0x201;
    match msg {
        WM_APP_TRAY => {
            // Tray icon callback
            match lparam.0 as u32 {
                WM_LBUTTONUP => {
                    // Left click: show notification with status
                    let _ = PostMessageW(hwnd, WM_APP_TRAYMENU, WPARAM(0), LPARAM(0));
                }
                WM_RBUTTONUP => {
                    // Right click: show context menu
                    let _ = PostMessageW(hwnd, WM_APP_TRAYMENU, WPARAM(1), LPARAM(0));
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_APP_TRAYMENU => {
            let platform = match keyboard::PLATFORM_PTR.as_mut() {
                Some(p) => p,
                None => return LRESULT(0),
            };

            let hmenu = CreatePopupMenu();
            if hmenu.is_invalid() {
                return LRESULT(0);
            }

            let version: String = env!("CARGO_PKG_VERSION").to_string();
            let _ = AppendMenuW(hmenu, MF_STRING | MF_GRAYED, 1001, w!("UltraWM"));
            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, HMENU(null_mut()));
            let _ = AppendMenuW(hmenu, MF_STRING, 1002, w!("Toggle Monocle"));
            let _ = AppendMenuW(hmenu, MF_STRING, 1003, w!("Save Session"));
            let _ = AppendMenuW(hmenu, MF_STRING, 1004, w!("Next Theme"));
            let _ = AppendMenuW(hmenu, MF_STRING, 1005, w!("Show Help"));
            let _ = AppendMenuW(hmenu, MF_STRING, 1006, w!("Window Search"));
            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, HMENU(null_mut()));
            let _ = AppendMenuW(hmenu, MF_STRING, 1007, w!("Quit UltraWM"));

            let mut pt = POINT { x: 0, y: 0 };
            GetCursorPos(&mut pt);
            SetForegroundWindow(hwnd);
            let _ = TrackPopupMenu(hmenu, TPM_LEFTALIGN | TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, std::ptr::null());

            // Handle menu selection
            let cmd = match wparam.0 {
                1002 => { platform.toggle_monocle(); None }
                1003 => { platform.save_session(); None }
                1004 => { platform.cycle_theme(true); None }
                1005 => { platform.toggle_help(); None }
                1006 => { platform.window_search(); None }
                1007 => { platform.quit(); None }
                _ => None,
            };

            let _ = DestroyMenu(hmenu);
            LRESULT(0)
        }
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Platform;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
