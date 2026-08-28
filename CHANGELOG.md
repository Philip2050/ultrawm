# Changelog

All notable changes to UltraWM will be documented in this file.

## [2.8.0] - 2026-08-28 — IPC add-rule Command
### Added
- **`add-rule` IPC command**: add window rules at runtime without editing config.toml
- Supports `match`, `float`, `workspace`, `opacity`, `sticky` fields
- IPC usage: `echo '{"command":"add-rule","match":"class:Notepad","float":true}' > \\.\pipe\ultrawm-ipc`
- Rules applied immediately to matching windows on next tile pass

## [2.7.0] - 2026-08-28 — Dynamic Gap Adjustment
### Added
- **`adjust_gap(delta)`**: dynamically grow/shrink window gaps at runtime
- `Win+,` (comma): shrink gaps by 2px (minimum 0)
- `Win+.` (period): grow gaps by 2px (maximum 100px)
- Gap changes apply immediately on next tiling pass
- **IPC commands**: `grow-gap` and `shrink-gap`

## [2.6.0] - 2026-08-28 — Always-on-Top Toggle
### Added
- **`toggle_always_on_top()`**: pin focused window above all others with `Win+O`
- `WindowInfo.always_on_top` field tracks per-window state
- Uses `SetWindowPos` with `HWND_TOPMOST` / `HWND_NOTOPMOST`
- **IPC command**: `always-on-top` (`echo '{"command":"always-on-top"}' > \\.\pipe\ultrawm-ipc`)

## [2.5.0] - 2026-08-28 — IPC get-config Command
### Added
- **`get-config` IPC command**: returns runtime config as JSON (layout, bar, theme, launcher settings)
- Returns monitor count and window count alongside config values
- Useful for external status bars and configuration monitoring
- IPC: `echo '{"command":"get-config"}' > \\.\pipe\ultrawm-ipc`

## [2.4.0] - 2026-08-28 — Overview Mode Click-to-Focus
### Added
- **Click-to-focus in overview mode**: clicking any window in overview focuses it and exits overview
- `WS_EX_TRANSPARENT` automatically toggled when entering/exiting overview mode
- `BorderOverlay.overview_positions` stores window rects for hit-testing
- `WM_OVERVIEW_CLICK` custom message for wndproc-to-main-thread communication
- `BorderOverlay.set_transparent()` to toggle click-through behavior

### Changed
- Overview mode overlay captures mouse input (clicks no longer pass through)
- Clicking empty space in overview does nothing (doesn't exit)

## [2.3.0] - 2026-08-28 — Window Maximize Toggle
### Added
- **`toggle_maximize()`**: maximize focused window to fill monitor work area with `Win+Shift+F`
- Maximize respects monitor's `rcWork` area (taskbar-aware)
- `maximize` IPC command (`echo '{"command":"maximize"}' > \\.\pipe\ultrawm-ipc`)
- Uses `WindowInfo.maximized` field for state tracking

### Changed
- `Win+F`: fullscreen toggle (topmost, covers taskbar)
- `Win+Shift+F`: maximize toggle (fills work area, taskbar visible)

## [2.2.0] - 2026-08-28 — Window Minimize/Restore
### Added
- **Minimize focused window**: `Win+M` minimizes the focused window, hiding it from the tiling layout
- **Restore focused window**: `Win+Shift+M` restores the previously minimized window
- **Minimized tracking**: `WindowInfo.minimized` field tracks per-window minimize state
- **IPC commands**: `minimize` and `restore` via JSON IPC (`echo '{"command":"minimize"}' > \\.\pipe\ultrawm-ipc`)
- Minimized windows excluded from tiling layout automatically

### Changed
- `tile_all_windows` skips minimized windows alongside floating and hidden windows

## [2.1.1] - 2026-08-28 — Configurable Workspace Count
### Added
- **`workspace_count` config option**: set 2-10 workspaces per monitor (default: 4) in `layout.workspace_count`
- **Dynamic workspace keybindings**: `Win+0-9` switches to workspaces 1-10, `Win+Shift+0-9` moves windows
- `0` key maps to the last workspace (e.g., 10 if workspace_count=10)
- Workspace switching and move commands respect the configured count
- `get-workspaces` IPC command returns actual workspace count and names from config

### Changed
- Removed hardcoded workspace limit of 4 in `move_focused_window_to_workspace`
- Bar workspace indicators generated dynamically based on actual grid count

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
- Updated `list_themes()` and `ThemeManager::load()` with all new themes

### Changed
- Animation timestep: capped at 1/30s to prevent spring instability

## [0.5.4] - 2026-08-28 — IPC Batch Commands + New Tab/Untab Commands

### Added
- **IPC batch commands**: `{"commands": ["focus-left", "grow-width", "split-vertical"]}` executes multiple commands atomically in one pipe write
- **IPC tab/untab commands**: `tab` and `untab` commands via JSON IPC
- `IpcCommand::Single { command }` and `IpcCommand::Batch { commands }` enum variants for cleaner IPC protocol
- `process_single_command()` helper for unified command processing in IPC thread

## [0.5.5] - 2026-08-28 — Launcher Transparency + Docs Polish

### Changed
- App launcher now uses `WS_EX_LAYERED` + `SetLayeredWindowAttributes` with 240/255 alpha (94% opacity)
- Launcher window has subtle transparency matching the bar aesthetic

## [2.0.0] - 2026-08-28 — Sticky Windows + Configurable Spring Animation
### Added
- **Sticky windows**: remain visible across all workspace switches
- `Win+Y` keyboard shortcut to toggle sticky on focused window
- Sticky windows excluded from hide/show during workspace transition
- **Configurable spring animation**: `spring_stiffness` and `spring_damping` in LayoutConfig
- Default values (180.0, 20.0) maintain existing animation feel

## [1.9.0] - 2026-08-28 — Configurable Spring Animation Parameters
### Added
- `spring_stiffness` and `spring_damping` config options
- WindowAnimState::new takes stiffness/damping from config
- Users can tune animation feel: lower=softer, higher=snappier

## [1.8.0] - 2026-08-28 — Floating Window Visual Indicator
### Added
- **Floating windows render with dashed blue border** (PS_DASH style)
- "FLOATING" label displayed at top of floating windows
- Floating windows excluded from 3-pass focused glow rendering
- Easily distinguish floating from tiled windows

## [1.7.0] - 2026-08-28 — Theme-prev + Workspace Move Shortcuts
### Added
- `Win+Shift+T`: cycle to previous theme
- `Win+1/2/3/4`: switch to workspace 1/2/3/4
- `Win+Shift+1/2/3/4`: move focused window to workspace 1/2/3/4
- `theme_prev` keybinding config option

## [1.6.0] - 2026-08-28 — Theme Cycle IPC Commands
### Added
- `theme-next` and `theme-prev` IPC commands for cycling themes
- `cycle_theme(forward)` method on Platform
- `next-theme`/`prev-theme` IPC commands retained for backward compat

## [1.5.0] - 2026-08-28 — IPC Get-Workspaces Command
### Added
- `get-workspaces` IPC command returns workspace count and names
- Useful for external status bars and workspace indicators

## [1.4.0] - 2026-08-28 — IPC Get-Windows Command
### Added
- `get-windows` enumerates all visible top-level windows via EnumWindows
- Returns JSON array with hwnd and title for each window
- Useful for external tools that need managed window info

## [1.3.0] - 2026-08-28 — Title Background in Border Overlay
### Changed
- Window title now has a filled background rect (accent color) for better readability
- White text on colored background for high contrast
- Text width measured dynamically for proper background sizing

## [1.2.0] - 2026-08-28 — Configurable Border Width
### Added
- **`border_width` config option**: adjustable border thickness (default: 2px)
- BorderOverlay reads border_width from LayoutConfig on initialization
- Config hot-reload updates border width and corner radius

## [1.1.0] - 2026-08-28 — IPC Workspace Commands
### Added
- **`workspace-1` through `workspace-4`** IPC commands for workspace switching
- **`move-window-to-workspace N`** IPC command to relocate focused window
- Window removed from old workspace grid and placed at (0,0) in new grid
- Window visibility managed based on current workspace state

## [1.0.0] - 2026-08-28 — Workspace Switch Fade Animation
### Added
- **Smooth workspace switch animation**: overlay fades out, workspace swaps, overlay fades in
- ~10 frame transition at 60fps for snappy feel
- `ws_fade` state tracks current overlay opacity (0.0-1.0)
- Fade direction tracked with `ws_fade_out` flag

## [0.9.0] - 2026-08-28 — Window Title Rendering in Border Overlay
### Added
- **Window title in border**: focused window title rendered in top border area
- Uses GDI `TextOutW` with Segoe UI 12px font
- Title uses accent color matching the focused window border
- `get_window_title` helper retrieves title via `GetWindowTextW`

## [0.8.0] - 2026-08-28 — Outer Padding for Tiled Windows
### Added
- **`outer_padding` config option**: margin between tiled layout and screen edges (default: 0)
- Applied via `current_work_area()` which shrinks the usable area
- Works across all monitors

## [0.7.0] - 2026-08-28 — Inner Window Padding
### Added
- **`inner_padding` config option**: margin inside window borders (default: 4px)
- Windows inset by padding on all sides for visual breathing room
- Corner radius automatically reduced by padding amount
- Applied in both tiled and overview modes
- Config hot-reload includes inner_padding

## [0.6.0] - 2026-08-28 — Wallpaper Engine
### Added
- **Wallpaper engine**: generates 1920x1080 gradient BMP from theme background color
- Applies wallpaper via `SystemParametersInfoW(SPI_SETDESKWALLPAPER)`
- Automatically updates on theme switch
- BMP format: 24-bit BGR, bottom-up, with row padding

## [0.5.6] - 2026-08-28 — Doctor Diagnostics Overhaul

### Added
- **Expanded doctor output**: DWM composition status, DPI awareness, shell replacement status, config file info, theme list, bar/overlay/hook/session status
- `DwmIsCompositionEnabled()` call to check DWM status
- Shell replacement check via `HKCU\...\Winlogon\Shell` registry value
- Config path existence and last-modified check
- Theme availability listing with all built-in theme names
- Dynamic version output using `CARGO_PKG_VERSION` instead of hardcoded string## [0.4.2] - 2026-08-28 — Swap Flash Animation

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
