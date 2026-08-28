# Changelog

All notable changes to UltraWM will be documented in this file.

## [0.5.0] - 2026-08-28 — Visual Polish (DWM Shadows + Bar Transparency + Rounded Corners + Blur)

### Added
- **DWM drop shadows on tiled windows**: `DwmSetWindowAttribute` with `DWMWA_NCRENDERING_POLICY = DWMNCRP_ENABLED`, tracked per-window via `shadow_set`
- **Bar transparency**: `WS_EX_LAYERED` + `SetLayeredWindowAttributes` with configurable alpha (default 85% via `bar.transparency`)
- **Rounded corners on tiled windows**: `corner_radius` config option (default: 8px), applied via `SetWindowRgn` with `CreateRoundRectRgn`, tracked via `last_rounded` HashMap
- **Focused window glow**: 3-pass border rendering (outer glow 25% +4px, mid-glow 50% +2px, solid border) via `color_dim()` helper
- **Rounded workspace indicators**: bar buttons use `RoundRect` with configurable corner radius
- **Acrylic blur with accent color**: `enable_blur` now uses `ACCENT_ENABLE_ACRYLICBLURBEHIND` with theme accent color + 0xCC alpha for visible tinted blur

### Changed
- Border overlay: 3-pass focused rendering vs 1-pass unfocused
- Bar workspace indicators: rounded pill-shaped active indicators with inverted text
- Animation timestep: capped at 1/30s for spring stability
- Blur: acrylic blur with accent color tint instead of generic blur

## [0.5.1] - 2026-08-28 — Window Rules Enhancements

### Added
- **Window opacity rule**: `opacity` field in rules (0.0-1.0), applied via `SetLayeredWindowAttributes` with `LWA_ALPHA`
- **Sticky window rule**: `sticky` field in rules, windows stay visible across all workspaces
- **Max size constraints**: `max_width` and `max_height` fields in rules for size limits
- `WindowInfo.opacity` and `WindowInfo.sticky` fields for per-window state tracking
- Opacity applied per-window in tile_all_windows after blur

## [0.5.2] - 2026-08-28 — Battery & Volume Indicators

### Added
- **Battery indicator** in bar: shows percentage with red text when below 20%
- **Volume indicator** in bar: shows volume percentage
- `AppBar::set_battery()` and `AppBar::set_volume()` methods
- `get_battery_level()` using `GetSystemPowerStatus` from `Win32::System::Power`
- Added `Win32_System_Power` feature to Cargo.toml
- Battery updates every ~10 seconds in event loop

## [0.5.3] - 2026-08-28 — Expanded Built-in Themes

### Added
- **7 new built-in themes**: dracula, monokai, solarized-dark, ayu-dark, one-dark, material-ocean, catppuccin-latte (light theme)
- Total of 12 built-in themes (5 dark + 1 light)
- Updated `list_themes()` and `ThemeManager::load()` with all new themes### Fixed
- Animation timestep: capped at 1/30s to prevent spring instability

## [0.4.2] - 2026-08-28 — Swap Flash Animation

### Added
- **Swap flash animation**: when windows swap positions via `Win+Shift+arrows`, both windows flash white for ~20 frames (~333ms) with smooth fade-out, giving clear visual feedback of the swap
- `Platform.swap_flash`: HashMap tracking flash animation timers per window
- `blend_color()`: color interpolation helper for flash-to-accent/inactive transitions
- Swap flash decay in event loop (frame-by-frame countdown)

### Fixed
- Extensive borrow-checker refactoring in `platform/mod.rs` to support gesture code alongside existing features

## [0.4.1] - 2026-08-28 — Touchpad Gesture Support

### Added
- **Touchpad gesture support**: transparent overlay receives WM_GESTURE messages
- Pan gesture (single-finger swipe): focus movement; (two-finger drag): camera panning
- Pinch zoom gesture: resize focused window width
- Two-finger tap gesture: toggle fullscreen
- Dead window cleanup: tile_all_windows removes closed windows automatically
- Fixed fullscreen toggle with proper saved/restored window rects

## [0.4.0] - 2026-08-28 — Window Tab Stacking

### Added
- **Window tab stacking**: group windows in the same cell with tab switching
- `TabGroup` struct: holds list of WindowIds + active index
- `CellNode::Tab` variant: cells can now be Leaf, Tab, or Split
- `GridState::tab_cell()`: tab two windows together
- `GridState::untab_cell()`: untab back to single leaf
- `GridState::cycle_tab()`: switch between tabs
- `Platform::tab_focused()`, `untab_focused()`, `cycle_tab()` methods
- Keyboard shortcuts: `Win+Alt+T` (tab), `Win+Alt+Shift+T` (untab)
- Only active tab window is positioned; inactive tabs are hidden
- All rect/layout methods updated to handle Tab nodes

## [0.3.0] - 2026-08-28 — Bidirectional Tiling & JSON IPC

### Added
- **Bidirectional tiling**: split cells horizontally or vertically with adjustable ratios
- `SplitDir` enum (Horizontal/Vertical) and `CellNode` enum (Leaf/Split) in layout engine
- `GridState::split_cell()`: splits a cell, creating primary/secondary children
- `GridState::unsplit_cell()`: merges children back into a single leaf
- `GridState::adjust_split_ratio()`: resize split with 0.1 step increments
- `GridState::all_window_rects()`: computes positions for all windows including split cells
- `Platform::split_focused()`, `unsplit_focused()`, `adjust_split()` methods
- Keyboard shortcuts: `Win+Alt+H/V` (split), `Win+Alt+U` (unsplit)
- **JSON IPC protocol**: named pipe now supports JSON commands with JSON responses
- `IpcCommand` now tagged with serde for kebab-case serialization
- `IpcResponse` struct with success/message/data fields
- Query commands: `get-state`, `list-themes`, `get-windows`
- New action commands: `split-horizontal`, `split-vertical`, `unsplit`, `overview`, `scratchpad`, `fullscreen`

### Changed
- IPC backward compatible: still accepts plain text commands
- Layout engine: `cell_nodes` BTreeMap tracks split trees per cell
- `place_window()` now creates `CellNode::Leaf` entries

## [0.2.0] - 2026-08-28 — Multi-Monitor Workspaces

### Added
- **Multi-monitor workspaces**: Each monitor now has its own independent set of 4 workspaces
- `MonitorWorkspaces` struct: per-monitor monitor info + grids + current workspace index
- `window_monitors` HashMap: tracks which monitor each window belongs to
- `monitor_for_hwnd()`: determines monitor from window position
- Per-monitor tiling: `tile_all_windows()` now iterates monitors and tiles only their windows

### Changed
- `grids: Vec<GridState>` → `monitor_workspaces: Vec<MonitorWorkspaces>`
- `switch_workspace()` now switches on the monitor containing the focused window
- Bar shows workspace indicators for the active monitor
- `manage_window()` assigns windows to correct monitor and workspace

## [0.1.1] - 2026-08-28 — Feature Wiring & Polish

### Added
- Theme picker UI wired up (`Win+G`): opens listbox, Enter applies theme
- Fullscreen toggle (`Win+F`): uses SetWindowPos with HWND_TOPMOST
- `Win+Shift+Arrows`: move focused window to adjacent cell with collision swapping
- `GridState::move_window()`: moves window to new cell, swaps if occupied
- Focus-follows-mouse (`focus_follows_mouse: bool` in config): polls cursor position
- Startup rules extended: `WindowRule` now supports `workspace` field
- Window close cleanup: `remove_window()` now also removes from `window_workspaces` and `anim`
- `ThemeManager::apply_idx()`: apply theme by index for picker

### Fixed
- Win+1/2/3/4 workspace switching keyboard shortcuts
- Bar updates active workspace indicator when switching
- Bar initialization sets initial workspace state
- All remaining `self.grid` references updated to `current_grid()` pattern

## [0.1.0] - 2026-08-28 — Initial MVP

### Added
- 2D infinite lattice layout engine with camera panning
- Multi-window tiling with collision-swapping focus
- Spring animation system (stiffness=220, damping=24, mass=1)
- 5 built-in themes: catppuccin-mocha, tokyo-night, gruvbox-dark, nord, rose-pine
- JSON theme support with live wallpaper/accent/dark-mode/cursor switching
- Low-level keyboard hook (`WH_KEYBOARD_LL`) for Win+key combos
- WinEvent hooks for focus tracking and window destroy detection
- TOML config with hot-reload (checks file modification time)
- IPC named pipe (`\\.\pipe\ultrawm-ipc`) with 20+ commands
- Per-monitor DPI awareness (v2)
- Session save/restore (JSON)
- DWM blur via `SetWindowCompositionAttribute`
- Shell replacement (`ultrawm --install/--uninstall`)
- App launcher with search filtering
- Theme picker UI
- Scratchpad windows
- Per-app rules (float, size, workspace)
- Bar with workspace indicators, title, and live clock
- Border overlay using `UpdateLayeredWindow`
- Overview mode (`Win+W`)
- `#![windows_subsystem = "windows"]` for no console
