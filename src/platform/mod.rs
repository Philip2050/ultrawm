use crate::anim::{Spring, SpringValue};
use crate::layout::GridState;
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
mod keyboard;
mod border;
mod bar;
mod launcher;
mod gesture;
mod theme_picker;
mod blur;
mod scratchpad;

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
    fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        let spring = Spring {
            stiffness: 220.0,
            damping: 24.0,
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
}

impl MonitorInfo {
    pub fn width(&self) -> i32 { self.right - self.left }
    pub fn height(&self) -> i32 { self.bottom - self.top }
    pub fn work_width(&self) -> i32 { self.work_right - self.work_left }
    pub fn work_height(&self) -> i32 { self.work_bottom - self.work_top }
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
    pub config: crate::config::Config,
    pub config_reload_counter: u32,
    pub next_id: u64,
    pub win_event_hook: HWINEVENTHOOK,
    pub overview: bool,
    pub gesture_receiver: Option<GestureReceiver>,
    pub theme_picker: Option<ThemePicker>,
    pub session: Option<crate::session::SessionState>,
    pub scratchpad: Option<ScratchpadManager>,
    pub theme_mgr: Option<RefCell<ThemeManager>>,
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
            config: crate::config::Config::default(),
            config_reload_counter: 0,
            next_id: 1,
            win_event_hook: HWINEVENTHOOK(std::ptr::null_mut()),
            overview: false,
            gesture_receiver: None,
            theme_picker: None,
            session: crate::session::SessionState::load().ok().flatten(),
            scratchpad: None,
            theme_mgr: None,
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

    fn window_for_hwnd(&self, hwnd: HWND) -> Option<&WindowInfo> {
        self.windows.get(&HWnd(hwnd))
    }

    /// Get mutable reference to the current grid for a specific monitor
    pub fn grid_for_monitor(&mut self, monitor_idx: usize) -> &mut GridState {
        &mut self.monitor_workspaces[monitor_idx].grids[self.monitor_workspaces[monitor_idx].current]
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

        let old_ws = self.monitor_workspaces[monitor_idx].current;

        // Hide windows on old workspace for this monitor
        for (&wid, info) in &self.windows {
            if let Some(wm) = self.window_monitors.get(&wid) {
                if *wm == monitor_idx {
                    if let Some(ws_id) = self.window_workspaces.get(&wid) {
                        if *ws_id == old_ws {
                            unsafe {
                                let _ = ShowWindow(info.hwnd, SW_HIDE);
                            }
                        }
                    }
                }
            }
        }

        self.monitor_workspaces[monitor_idx].current = ws;

        // Update bar to show active workspace
        if let Some(ref bar) = self.bar {
            bar.set_workspaces(
                (0..self.monitor_workspaces[monitor_idx].grids.len()).map(|i| (i + 1).to_string()).collect(),
                ws,
            );
        }

        // Show windows on new workspace for this monitor
        for (&wid, info) in &self.windows {
            if let Some(wm) = self.window_monitors.get(&wid) {
                if *wm == monitor_idx {
                    if let Some(ws_id) = self.window_workspaces.get(&wid) {
                        if *ws_id == ws {
                            unsafe {
                                let _ = ShowWindow(info.hwnd, SW_SHOW);
                            }
                        }
                    }
                }
            }
        }

        info!("Monitor {}: switched to workspace {}", monitor_idx + 1, ws + 1);
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
        if let Some(hwnd) = self.focused_hwnd {
            if let Some(mon) = self.monitor_for_hwnd(hwnd.0) {
                return (mon.work_left, mon.work_top, mon.work_right, mon.work_bottom);
            }
        }
        if let Some(m) = self.primary_monitor() {
            return (m.work_left, m.work_top, m.work_right, m.work_bottom);
        }
        (0, 0, 1920, 1080)
    }

    pub fn initialize(&mut self, _config: &crate::config::Config) -> anyhow::Result<()> {
        info!("UltraWM initializing...");

        // Enumerate existing top-level windows
        self.enumerate_windows()?;

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
            Ok(overlay) => {
                self.border_overlay = Some(overlay);
                info!("Border overlay created");
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
        match AppBar::create(bar_width, bar_height, bar_bg, bar_fg) {
            Ok(bar) => {
                self.bar = Some(bar);
                info!("AppBar created");
                if let Some(ref bar) = self.bar {
                    bar.set_workspaces(
                        (0..self.monitor_workspaces[0].grids.len()).map(|i| (i + 1).to_string()).collect(),
                        0,
                    );
                }
            }
            Err(e) => {
                warn!("AppBar creation failed: {}", e);
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
                        let grid = self.current_grid();
                        {
                            grid.apply_layout_config(
                                self.config.layout.gaps,
                                self.config.layout.peek_x,
                                self.config.layout.peek_y,
                            );
                        }
                        info!(
                            "Config reloaded: gaps={}, peek_x={}, peek_y={}",
                            self.config.layout.gaps,
                            self.config.layout.peek_x,
                            self.config.layout.peek_y
                        );
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
            }

            // Focus-follows-mouse check (every frame if enabled)
            self.focus_follows_mouse_check();

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
                    let _ = SetForegroundWindow(hwnd);
                    self.focused_hwnd = Some(hwnd_wrapper);
                    let grid = self.current_grid();
                    grid.focus_window(info.id);
                }
            }
        }
    }

    pub fn tile_all_windows(&mut self, accent_rgb: u32, inactive_rgb: u32) {
        let mut border_rects: Vec<(i32, i32, i32, i32, u32, bool)> = Vec::new();

        // Overview mode: compact grid of all windows
        if self.overview {
            self.tile_overview(accent_rgb, inactive_rgb, &mut border_rects);
            if let Some(ref mut overlay) = self.border_overlay {
                overlay.update(&border_rects);
            }
            return;
        }

        // Tile windows per-monitor (each monitor has its own workspace)
        for (mon_idx, mws) in self.monitor_workspaces.iter_mut().enumerate() {
            let (vw, vh) = (mws.monitor.work_width(), mws.monitor.work_height());
            let grid = &mut mws.grids[mws.current];

            for (&hwnd_wrapper, info) in &self.windows {
                // Only tile windows belonging to this monitor
                if let Some(&wm) = self.window_monitors.get(&info.id) {
                    if wm != mon_idx {
                        continue;
                    }
                } else {
                    // Assign window to monitor based on position if not yet assigned
                    if let Some(mon) = self.monitor_for_hwnd(hwnd_wrapper.0) {
                        let assigned = self.monitors.iter().position(|m| m.handle == mon.handle).unwrap_or(0);
                        if assigned != mon_idx {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                // Skip floating windows
                if info.floating {
                    continue;
                }

                // Skip cloaked/hidden windows
                if !info.visible {
                    continue;
                }

                // Check if window still exists
                if !is_window_alive(hwnd_wrapper.0) {
                    continue;
                }

                if let Some((x, y, w, h)) = self.position_for_hwnd(hwnd_wrapper.0, grid, vw, vh) {
                    let wid = info.id;
                    let target_x = x as f32;
                    let target_y = y as f32;
                    let target_w = w as f32;
                    let target_h = h as f32;

                    let anim = self.anim.entry(wid).or_insert_with(|| {
                        WindowAnimState::new(target_x, target_y, target_w, target_h)
                    });
                    anim.set_target(target_x, target_y, target_w, target_h);
                    let (ax, ay, aw, ah) = anim.step(1.0 / 60.0);

                    unsafe {
                        let _ = SetWindowPos(
                            hwnd_wrapper.0,
                            HWND(null_mut()),
                            ax as i32,
                            ay as i32,
                            aw as i32,
                            ah as i32,
                            SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                        );
                    }

                    let is_focused = self.focused_hwnd == Some(hwnd_wrapper);
                    let color = if is_focused {
                        accent_rgb
                    } else {
                        inactive_rgb
                    };
                    border_rects.push((ax as i32, ay as i32, aw as i32, ah as i32, color, is_focused));

                    // Apply DWM blur to windows
                    let _ = enable_blur(hwnd_wrapper.0);
                }
            }
        }

        // Update border overlay
        if let Some(ref mut overlay) = self.border_overlay {
            overlay.update(&border_rects);
        }
    }

    fn tile_overview(
        &mut self,
        accent_rgb: u32,
        inactive_rgb: u32,
        border_rects: &mut Vec<(i32, i32, i32, i32, u32, bool)>,
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
            return;
        }

        // Compact grid: 200x150 per window, up to 4 per row
        let cols = (vw / 220).max(1).min(windows.len() as i32);
        let rows = (windows.len() as f32 / cols as f32).ceil() as i32;
        let cell_w = (vw / cols).min(300);
        let cell_h = (vh / rows).min(200);
        let gap = 8;

        for (i, (wid, hwnd, _info)) in windows.iter().enumerate() {
            let row = (i / cols as usize) as i32;
            let col = (i % cols as usize) as i32;
            let x = wl + col * (cell_w + gap) + (vw - cols * (cell_w + gap)) / 2;
            let y = wt + row * (cell_h + gap) + (vh - rows * (cell_h + gap)) / 2;

            unsafe {
                let _ = SetWindowPos(
                    *hwnd,
                    HWND(null_mut()),
                    x,
                    y,
                    cell_w,
                    cell_h,
                    SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
            }

            let is_focused = self.focused_hwnd == Some(HWnd(*hwnd));
            let color = if is_focused { accent_rgb } else { inactive_rgb };
            border_rects.push((x, y, cell_w, cell_h, color, is_focused));
        }
    }

    fn position_for_hwnd(
        &self,
        hwnd: HWND,
        grid: &GridState,
        vw: i32,
        vh: i32,
    ) -> Option<(i32, i32, u32, u32)> {
        let wid = self.windows.get(&HWnd(hwnd))?.id;
        let cell = grid.window_positions.get(&wid)?;
        let (x, y, w, h) = grid.cell_rect(*cell, vw, vh);
        let (wl, wt, _, _) = self.current_work_area();
        Some((x + wl, y + wt, w, h))
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
                    grid.place_window(wid);

                    // Restore position from session if available
                    if let Some(ref session) = self.session {
                        if let Some(cell) = session.window_positions.get(&exe) {
                            grid.window_positions.insert(wid, *cell);
                            grid.cells.insert(*cell, wid);
                        }
                    }

                    grid.focus_window(wid);
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
                if info.id > 0 {
                    let dir = if horizontal { crate::layout::SplitDir::Horizontal } else { crate::layout::SplitDir::Vertical };
                    let grid = self.current_grid();
                    if grid.split_cell(info.id, dir) {
                        info!("Split cell {} {:?}", info.id, dir);
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
                if info.id > 0 {
                    let grid = self.current_grid();
                    if grid.unsplit_cell(info.id) {
                        info!("Unsplit cell for window {}", info.id);
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
                if info.id > 0 {
                    let grid = self.current_grid();
                    grid.adjust_split_ratio(info.id, grow);
                    self.tile_all_windows(0xFF7F7F7F, 0xFF454545);
                }
            }
        }
    }

    pub fn on_focus_changed(&mut self, hwnd: HWND) {
        self.focused_hwnd = Some(HWnd(hwnd));

        {
            let grid = self.current_grid();
            if let Some(info) = self.windows.get(&HWnd(hwnd)) {
                if info.id > 0 {
                    grid.focus_window(info.id);
                }
            }
        }
    }

    pub fn remove_window(&mut self, hwnd: HWND) {
        let hwnd_wrapper = HWnd(hwnd);
        let was_focused = self.focused_hwnd == Some(hwnd_wrapper);

        if let Some(info) = self.windows.remove(&hwnd_wrapper) {
            let grid = self.current_grid();
            grid.remove_window(info.id);
            self.window_workspaces.remove(&info.id);
            self.anim.remove(&info.id);

            // Auto-focus neighbor when focused window is closed
            if was_focused {
                if let Some(next_wid) = find_nearest_window(&self.windows, &grid, info.id) {
                    for (&hw, wi) in &self.windows {
                        if wi.id == next_wid {
                            unsafe {
                                let _ = SetForegroundWindow(hw.0);
                            }
                            self.focused_hwnd = Some(hw);
                            debug!("Swallowed window, focused next: {}", wi.title);
                            break;
                        }
                    }
                } else {
                    self.focused_hwnd = None;
                }
            }
            debug!("Removed window: {} (id={})", info.title, info.id);
        }
    }

    pub fn move_focus(&mut self, dr: i32, dc: i32) {
        let grid = self.current_grid();
        if let Some(wid) = grid.move_focus(dr, dc) {
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
                        let _ = SetWindowPos(
                            info.hwnd,
                            HWND_TOPMOST,
                            0, 0,
                            info.monitor_width.unwrap_or(1920),
                            info.monitor_height.unwrap_or(1080),
                            SWP_FRAMECHANGED.0,
                        );
                    } else {
                        let _ = SetWindowPos(
                            info.hwnd,
                            HWND_NOTOPMOST,
                            info.x, info.y,
                            info.width, info.height,
                            SWP_FRAMECHANGED.0,
                        );
                    }
                }
                info!("Fullscreen toggled: {} (id={})", info.title, info.id);
            }
        }
    }

    pub fn toggle_floating(&mut self) {
        if let Some(hwnd_wrapper) = self.focused_hwnd {
            if let Some(info) = self.windows.get_mut(&hwnd_wrapper) {
                info.floating = !info.floating;
                if info.floating {
                    let grid = self.current_grid();
                    {
                        grid.remove_window(info.id);
                    }
                    info!("Floating window: {} (id={})", info.title, info.id);
                } else {
                    let grid = self.current_grid();
                    {
                        grid.place_window(info.id);
                        grid.focus_window(info.id);
                    }
                    info!("Unfloating window: {} (id={})", info.title, info.id);
                }
                self.tile_all_windows(0xFF7F7F7F, 0xFF454545);
            }
        }
    }

    pub fn handle_ipc_command(&mut self, cmd: crate::ipc::IpcCommand, theme_mgr: &mut crate::theme::ThemeManager) {
        match cmd {
            crate::ipc::IpcCommand::NextTheme => {
                let _ = theme_mgr.next_theme();
            }
            crate::ipc::IpcCommand::PrevTheme => {
                let _ = theme_mgr.prev_theme();
            }
            crate::ipc::IpcCommand::FocusNext => {
                self.move_focus(0, 1);
            }
            crate::ipc::IpcCommand::FocusPrev => {
                self.move_focus(0, -1);
            }
            crate::ipc::IpcCommand::FocusLeft => {
                self.move_focus(0, -1);
            }
            crate::ipc::IpcCommand::FocusRight => {
                self.move_focus(0, 1);
            }
            crate::ipc::IpcCommand::FocusUp => {
                self.move_focus(-1, 0);
            }
            crate::ipc::IpcCommand::FocusDown => {
                self.move_focus(1, 0);
            }
            crate::ipc::IpcCommand::PanLeft => {
                self.pan_camera(0, -1);
            }
            crate::ipc::IpcCommand::PanRight => {
                self.pan_camera(0, 1);
            }
            crate::ipc::IpcCommand::PanUp => {
                self.pan_camera(-1, 0);
            }
            crate::ipc::IpcCommand::PanDown => {
                self.pan_camera(1, 0);
            }
            crate::ipc::IpcCommand::GrowWidth => {
                self.resize_width(true);
            }
            crate::ipc::IpcCommand::ShrinkWidth => {
                self.resize_width(false);
            }
            crate::ipc::IpcCommand::GrowHeight => {
                self.resize_height(true);
            }
            crate::ipc::IpcCommand::ShrinkHeight => {
                self.resize_height(false);
            }
            crate::ipc::IpcCommand::Close => {
                self.close_focused();
            }
            crate::ipc::IpcCommand::Float => {
                self.toggle_floating();
            }
            crate::ipc::IpcCommand::Unfloat => {
                self.toggle_floating();
            }
            crate::ipc::IpcCommand::ToggleLauncher => {
                self.toggle_launcher();
            }
            crate::ipc::IpcCommand::ToggleOverview => {
                self.toggle_overview();
            }
            crate::ipc::IpcCommand::ToggleScratchpad => {
                self.toggle_scratchpad();
            }
            crate::ipc::IpcCommand::ToggleFullscreen => {
                self.toggle_fullscreen();
            }
            crate::ipc::IpcCommand::SplitHorizontal => {
                self.split_focused(true);
            }
            crate::ipc::IpcCommand::SplitVertical => {
                self.split_focused(false);
            }
            crate::ipc::IpcCommand::Unsplit => {
                self.unsplit_focused();
            }
            crate::ipc::IpcCommand::GetState |
            crate::ipc::IpcCommand::ListThemes |
            crate::ipc::IpcCommand::GetWindows => {
                // Query commands handled by IPC thread directly
            }
            crate::ipc::IpcCommand::Quit => {
                log::info!("IPC: quit requested");
                unsafe {
                    let _ = PostQuitMessage(0);
                }
            }
        }
    }

    pub fn apply_rules(&self, win_info: &mut WindowInfo, raw_info: &WindowInfo) {
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
        }
    }

    pub fn save_session(&mut self) {
        let grid = &self.monitor_workspaces[0].grids[self.monitor_workspaces[0].current];
        let mut positions = BTreeMap::new();
        for (&hwnd_wrapper, info) in &self.windows {
            if let Some(cell) = grid.window_positions.get(&info.id) {
                positions.insert(info.exe.clone(), *cell);
            }
        }

        let state = crate::session::SessionState {
            window_positions: positions,
            camera: grid.camera,
        };

        if let Err(e) = state.save() {
            warn!("Failed to save session: {}", e);
        }
    }

    pub fn toggle_scratchpad(&mut self) {
        if self.scratchpad.is_none() {
            if let Ok(()) = ScratchpadManager::create() {
                self.scratchpad = Some(ScratchpadManager::create().unwrap_or_default());
            }
        }
        if let Some(ref mut sp) = self.scratchpad {
            sp.toggle();
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
                let current = self.theme_mgr.as_ref().map(|m| m.borrow().current).unwrap_or(0);
                let _ = ThemePicker::create(themes, current);
            }
        }
    }

    pub fn apply_theme_by_idx(&mut self, idx: usize) {
        if let Some(ref mgr) = self.theme_mgr {
            let mut m = mgr.borrow_mut();
            let _ = m.apply_idx(idx);
        }
    }

    pub fn toggle_overview(&mut self) {
        self.overview = !self.overview;
        if self.overview {
            log::info!("Overview mode ON — {} windows visible", self.windows.len());
        } else {
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
        println!("Monitors: {}", self.monitors.len());
        for (i, m) in self.monitors.iter().enumerate() {
            println!(
                "  Monitor {}: {}x{} at ({},{}) work={}x{}",
                i,
                m.width(),
                m.height(),
                m.left,
                m.top,
                m.work_width(),
                m.work_height()
            );
        }
        println!("Managed windows: {}", self.windows.len());
        for (hwnd_wrapper, info) in &self.windows {
            println!(
                "  {:?}: {} (class={}, exe={}, id={})",
                hwnd_wrapper.0, info.title, info.class, info.exe, info.id
            );
        }
        for (mi, mws) in self.monitor_workspaces.iter().enumerate() {
            let grid = &mws.grids[mws.current];
            println!("Monitor {} workspace {}: {} windows", mi, mws.current + 1, grid.window_positions.len());
            for (&wid, &cell) in &grid.window_positions {
                println!("  window {}: cell ({}, {})", wid, cell.row, cell.col);
            }
            println!(
                "Camera: ({}, {})",
                grid.camera.row, grid.camera.col
            );
        }
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
    windows: &HashMap<HWnd, WindowInfo>,
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
            platform.window_workspaces.insert(wid, platform.monitor_workspaces[mon_idx].current);
            platform.monitor_workspaces[mon_idx].grids[platform.monitor_workspaces[mon_idx].current].place_window(wid);
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
