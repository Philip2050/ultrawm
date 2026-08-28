use crate::anim::{Spring, SpringValue};
use crate::layout::{Cell, GridState};
use crate::theme::ThemeManager;
use log::{debug, info, warn};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::ptr::null_mut;
use std::sync::mpsc;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::{Dwm::*, Gdi::*},
        UI::{Accessibility::*, HiDpi::*, Shell::*, WindowsAndMessaging::*},
    },
};

pub use window::WindowInfo;
pub use keyboard::KeyboardHook;
pub use border::BorderOverlay;
pub use bar::AppBar;
pub use launcher::AppLauncher;
pub use gesture::GestureReceiver;
pub use theme_picker::ThemePicker;
pub use blur::enable_blur;
pub use scratchpad::ScratchpadManager;

mod window;
pub mod keyboard;
pub mod border;
mod bar;
mod launcher;
mod gesture;
mod theme_picker;
mod blur;
mod scratchpad;
mod wallpaper;

#[derive(Debug, Clone, Copy)]
struct HWnd(HWND);

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
    pub border_overlay: Option<BorderOverlay>,
    pub bar: Option<AppBar>,
    // Each monitor has its own set of workspaces (independent)
    pub monitor_workspaces: Vec<MonitorWorkspaces>,
    pub window_workspaces: HashMap<u64, usize>, // wid -> workspace index (0-3)
    pub window_monitors: HashMap<u64, usize>,   // wid -> monitor index
    pub anim: HashMap<u64, WindowAnimState>,
    pub swap_flash: HashMap<u64, u32>, // wid -> flash timer (countdown frames)
    last_rounded: HashMap<u64, (i32, i32, i32)>, // wid -> (w, h, radius) last applied
    last_frame_time: std::time::Instant,
    shadow_set: HashMap<u64, bool>, // wid -> whether DWM shadow is enabled
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
    pub theme_mgr: Option<RefCell<ThemeManager>>,
    ws_fade: f32,           // workspace switch fade (1.0=visible, 0.0=invisible)
    ws_fade_out: bool,      // true=fading out, false=fading in
    ws_pending_ws: Option<usize>,
    ws_pending_monitor: Option<usize>,
}

pub struct MonitorWorkspaces {
    pub monitor: MonitorInfo,
    pub grids: Vec<GridState>,
    pub current: usize,
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
            border_overlay: None,
            bar: None,
            monitor_workspaces,
            window_workspaces: HashMap::new(),
            window_monitors: HashMap::new(),
            anim: HashMap::new(),
            swap_flash: HashMap::new(),
            last_rounded: HashMap::new(),
            last_frame_time: std::time::Instant::now(),
            shadow_set: HashMap::new(),
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
            theme_mgr: None,
            ws_fade: 1.0,
            ws_fade_out: false,
            ws_pending_ws: None,
            ws_pending_monitor: None,
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

    fn window_for_hwnd(&self, hwnd: HWND) -> Option<&WindowInfo> {
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

        // Start workspace fade animation
        self.ws_fade = 1.0;
        self.ws_fade_out = true;
        self.ws_pending_ws = Some(ws);
        self.ws_pending_monitor = Some(monitor_idx);

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

        // Enumerate existing top-level windows
        self.enumerate_windows()?;

        // Restore Z-order from session
        if self.session.is_some() {
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
            ) {
                Ok(bar) => {
                    self.bar = Some(bar);
                    info!("AppBar created");
                }
                Err(e) => {
                    warn!("AppBar creation failed: {}", e);
                }
            }
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

                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                } else {
                    break;
                }
            }

            // Periodic tiling refresh — spring-animated
            let theme = theme_mgr.current_theme();
            let accent = hex_to_rgb(&theme.accent);
            let inactive = hex_to_rgb(&theme.inactive);
            self.tile_all_windows(accent, inactive);

            // Workspace switch fade animation
            if self.ws_fade < 1.0 {
                if self.ws_fade_out {
                    self.ws_fade -= 0.15;
                    if self.ws_fade <= 0.0 {
                        self.ws_fade = 0.0;
                        self.ws_fade_out = false;
                        // Execute pending workspace switch
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
                                // Update bar
                                if let Some(ref bar) = self.bar {
                                    bar.set_workspaces(
                                        (0..self.monitor_workspaces[mon].grids.len()).map(|i| (i + 1).to_string()).collect(),
                                        ws,
                                    );
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
                            }
                            self.ws_pending_ws = None;
                            self.ws_pending_monitor = None;
                        }
                    }
                } else {
                    self.ws_fade += 0.15;
                    if self.ws_fade >= 1.0 {
                        self.ws_fade = 1.0;
                    }
                }
                // Apply fade alpha to overlay
                if let Some(ref overlay) = self.border_overlay {
                    let alpha = (self.ws_fade * 255.0) as u8;
                    overlay.set_alpha(alpha);
                }
            }

            // Hot-reload config every ~1 second
            self.config_reload_counter += 1;
            if self.config_reload_counter >= 60 {
                self.config_reload_counter = 0;
                if let Ok(Some(new_config)) = self.config.reload_if_changed() {
                    let old_gaps = self.config.layout.gaps;
                    let old_peek_x = self.config.layout.peek_x;
                    let old_peek_y = self.config.layout.peek_y;
                    self.config = new_config;

                    // Apply layout changes to grid if they changed
                    if self.config.layout.gaps != old_gaps
                        || self.config.layout.peek_x != old_peek_x
                        || self.config.layout.peek_y != old_peek_y
                    {
                        let gaps = self.config.layout.gaps;
                        let inner_padding = self.config.layout.inner_padding;
                        let peek_x = self.config.layout.peek_x;
                        let peek_y = self.config.layout.peek_y;
                        let grid = self.current_grid();
                        {
                            grid.apply_layout_config(gaps, inner_padding, peek_x, peek_y);
                        }
                        info!(
                            "Config reloaded: gaps={}, inner_padding={}, peek_x={}, peek_y={}",
                            self.config.layout.gaps,
                            self.config.layout.inner_padding,
                            self.config.layout.peek_x,
                            self.config.layout.peek_y
                        );
                    }
                    // Update border width and radius from config
                    if let Some(ref mut overlay) = self.border_overlay {
                        overlay.border_width = self.config.layout.border_width as i32;
                        overlay.border_radius = self.config.layout.corner_radius as i32;
                    }
                }
            }

            // Save session every ~5 seconds
            self.save_session();

            // Update bar with focused window title and clock
            if let Some(ref bar) = self.bar {
                let title = self
                    .focused_hwnd
                    .and_then(|hwnd| self.windows.get(&hwnd))
                    .map(|info| info.title.clone())
                    .unwrap_or_default();
                bar.set_title(&title);

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
        let mut border_rects: Vec<(i32, i32, i32, i32, u32, bool, bool, Option<String>)> = Vec::new();
        let mut tile_rects: Vec<(i32, i32, i32, i32, HWND)> = Vec::new();

        // Overview mode: compact grid of all windows
        if self.overview {
            self.tile_overview(accent_rgb, inactive_rgb, &mut border_rects);
            if let Some(ref mut overlay) = self.border_overlay {
                overlay.update(&border_rects);
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
                        WindowAnimState::new(
                            target_x, target_y, target_w, target_h,
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
                    let color = if flash > 0 {
                        // Flash white during swap animation, fading with timer
                        let alpha = flash.min(20) as f32 / 20.0;
                        blend_color(0xFFFFFFFF, if is_focused { accent_rgb } else { inactive_rgb }, alpha)
                    } else if is_focused {
                        accent_rgb
                    } else {
                        inactive_rgb
                    };
                    let title = if is_focused {
                        get_window_title(hwnd_wrapper.0)
                    } else {
                        None
                    };
                    border_rects.push((ax as i32, ay as i32, aw as i32, ah as i32, color, is_focused, info.floating, title));

                    // Apply DWM blur to windows
                    let _ = enable_blur(hwnd_wrapper.0, accent_rgb);

                    // Apply per-window opacity
                    if let Some(op) = info.opacity {
                        unsafe {
                            let _ = SetLayeredWindowAttributes(hwnd_wrapper.0, COLORREF(0), (op * 255.0) as u8, LWA_ALPHA);
                        }
                    }
                }
            }
        }

        // Update border overlay
        if let Some(ref mut overlay) = self.border_overlay {
            overlay.tile_rects = tile_rects;
            overlay.update(&border_rects);
        }
    }

    fn tile_overview(
        &mut self,
        accent_rgb: u32,
        inactive_rgb: u32,
        border_rects: &mut Vec<(i32, i32, i32, i32, u32, bool, bool, Option<String>)>,
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
            let title = if is_focused { get_window_title(*hwnd) } else { None };
            let hwnd_wrapper = HWnd(*hwnd);
            let floating = self.windows.get(&hwnd_wrapper).map(|i| i.floating).unwrap_or(false);
            border_rects.push((x, y, cell_w, cell_h, color, is_focused, floating, title));
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
                    grid.place_window(wid);

                    // Restore state from session if available
                    if let Some(ref session) = self.session {
                        for sw in &session.windows {
                            if sw.exe == exe {
                                grid.window_positions.insert(wid, sw.cell);
                                grid.cells.insert(sw.cell, wid);
                                // Restore window properties
                                if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                                    info.floating = sw.floating;
                                    info.opacity = sw.opacity;
                                    info.sticky = sw.sticky;
                                    info.maximized = sw.maximized;
                                    info.always_on_top = sw.always_on_top;
                                    info.z_order = sw.z_order;
                                    if sw.floating {
                                        // Float: remove from grid, position manually
                                        grid.remove_window(wid);
                                        unsafe {
                                            let _ = SetWindowPos(
                                                info.hwnd,
                                                if sw.always_on_top { HWND_TOPMOST } else { HWND_NOTOPMOST },
                                                info.saved_x, info.saved_y, info.saved_w, info.saved_h,
                                                SWP_FRAMECHANGED,
                                            );
                                        }
                                    }
                                }
                                break;
                            }
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
        self.focused_hwnd = Some(HWnd(hwnd));

        let wid = self.windows.get(&HWnd(hwnd)).map(|i| i.id).unwrap_or(0);
        let grid = self.current_grid();
        if wid > 0 {
            grid.focus_window(wid);
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
        let grid = self.current_grid();
        if let Some(wid) = grid.focused_window {
            if grow {
                grid.grow_width(wid);
            } else {
                grid.shrink_width(wid);
            }
            debug!("Width adjusted for window {}", wid);
        }
    }

    pub fn resize_height(&mut self, grow: bool) {
        let grid = self.current_grid();
        if let Some(wid) = grid.focused_window {
            if grow {
                grid.grow_height(wid);
            } else {
                grid.shrink_height(wid);
            }
            debug!("Height adjusted for window {}", wid);
        }
    }

    pub fn move_window(&mut self, dr: i32, dc: i32) {
        let grid = self.current_grid();
        if let Some(wid) = grid.focused_window {
            grid.move_window(wid, dr, dc);
            self.tile_all_windows(0xFF7F7F7F, 0xFF454545);
        }
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
                self.swap_flash.insert(src_id, 20);
                self.swap_flash.insert(tgt_id, 20);
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
                self.swap_flash.insert(src_id, 20);
                self.swap_flash.insert(tgt_id, 20);
                info!("Drag-moved window {} to cell of {}", src_id, tgt_id);
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

    pub fn toggle_fullscreen(&mut self) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                info.fullscreen = !info.fullscreen;
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
    }

    fn apply_rule_from_json(&mut self, json: serde_json::Value) {
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
        for rule in &self.config.rules {
            let matches = raw_info.title.contains(&rule.match_)
                || raw_info.class.contains(&rule.match_)
                || raw_info.exe.contains(&rule.match_);
            if !matches {
                continue;
            }

            if let Some(float) = rule.float {
                win_info.floating = float;
            }
            if let Some(ws) = rule.workspace {
                if ws < 4 {
                    self.window_workspaces.insert(win_info.id, ws);
                }
            }
            if let Some(op) = rule.opacity {
                if op >= 0.0 && op <= 1.0 {
                    win_info.opacity = Some(op);
                }
            }
            if let Some(sticky) = rule.sticky {
                win_info.sticky = sticky;
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

    pub fn save_session(&mut self) {
        let grid = &self.monitor_workspaces[0].grids[self.monitor_workspaces[0].current];

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

        let mut windows = Vec::new();
        for (&hwnd_wrapper, info) in &self.windows {
            if let Some(cell) = grid.window_positions.get(&info.id) {
                let z_order = z_order_map.get(&(hwnd_wrapper.0.0 as usize)).copied().unwrap_or(usize::MAX);
                windows.push(crate::session::SessionWindowState {
                    exe: info.exe.clone(),
                    cell: *cell,
                    floating: info.floating,
                    workspace: self.monitor_workspaces[0].current,
                    opacity: info.opacity,
                    sticky: info.sticky,
                    maximized: info.maximized,
                    always_on_top: info.always_on_top,
                    z_order,
                });
            }
        }

        let state = crate::session::SessionState {
            windows,
            camera: grid.camera,
        };

        if let Err(e) = state.save() {
            warn!("Failed to save session: {}", e);
        }
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

        Ok(())
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
            }
        }
        EVENT_OBJECT_SHOW => {
            if !hwnd.is_invalid() && is_window_alive(hwnd) {
                platform.manage_window(hwnd);
            }
        }
        _ => {}
    }
}
