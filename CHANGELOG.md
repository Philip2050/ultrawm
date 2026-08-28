# Changelog

All notable changes to UltraWM will be documented in this file.

## [6.2.0] - 2026-08-29 — Focus Flash Animation
### Added
- **Focus flash**: border overlay briefly pulses when focus changes to a window
- `trigger_focus_flash(wid)` method triggers 15-frame flash animation
- Flash rendered in `tile_all_windows()` using existing `swap_flash` mechanism
- Flash blends white into accent color for focused, inactive color for unfocused
- Focus flash integrated into `on_focus_changed()` callback

### Changed
- Flash timer: 15 frames (~250ms at 60fps)
- White-to-accent blend during flash creates smooth visual transition

## [6.1.0] - 2026-08-29 — Per-Monitor Layout Settings
### Added
- **monitor_layouts** config: per-monitor layout overrides (gaps, padding, border, corner_radius)
- `MonitorLayout` struct with optional fields: gaps, inner_padding, outer_padding, border_width, corner_radius
- `Platform::monitor_layout()` returns per-monitor layout config
- `Platform::effective_gap()` resolves gap with per-monitor override
- `Platform::effective_border_width()` resolves border width with per-monitor override
- `Platform::effective_corner_radius()` resolves corner radius with per-monitor override

### Example config
```toml
[[layout.monitor_layouts]]
gaps = 10
border_width = 2

[[layout.monitor_layouts]]
gaps = 5
border_width = 1
```

## [6.0.0] - 2026-08-29 — Dynamic Workspace Count
### Added
- **set-workspace-count \<N>** IPC command: changes number of workspaces at runtime (1-10)
- `Platform::set_workspace_count()` adds or removes workspaces dynamically
- `Platform::find_empty_cell()` finds next available grid cell for window placement
- When reducing workspaces: windows moved to current workspace, grids truncated
- When increasing workspaces: new empty GridState created per monitor
- Bar updated with new workspace names after count change
- Config `workspace_count` updated to reflect new count

### Example IPC usage
```
echo {"command":"set-workspace-count 6"} | \\\\.\\pipe\\ultrawm-ipc
echo {"command":"set-workspace-count 2"} | \\\\.\\pipe\\ultrawm-ipc
```

## [5.9.0] - 2026-08-29 — Scratchpad IPC Commands
### Added
- **add-scratchpad \<name>** IPC command: adds focused window to scratchpad with given name
- **remove-scratchpad \<name>** IPC command: removes scratchpad entry by name
- `ScratchpadManager::find_by_name()` method for name-based lookup
- `Platform::add_scratchpad()` adds focused window to scratchpad manager
- `Platform::remove_scratchpad()` removes scratchpad entry by name
- **scratchpad** IPC command: toggles all scratchpad windows visibility

### Changed
- Scratchpad toggle uses `SCRATCHPAD_PTR` static pointer for global access
- Windows in scratchpad are shown/hidden via `ShowWindow(SW_SHOW/SW_HIDE)`

## [5.8.0] - 2026-08-29 — Named Workspaces
### Added
- **workspace_names** config field: list of custom names for workspaces (falls back to numbers)
- `Platform::workspace_names()` helper returns names or numeric fallback
- Bar renders workspace names instead of numbers when configured
- IPC `get-workspaces` returns configured names from LayoutConfig
- Empty workspace_names array defaults to "1", "2", "3", ... numbering

### Example config
```toml
[layout]
workspace_count = 4
workspace_names = ["web", "code", "chat", "media"]
```

## [5.7.0] - 2026-08-29 — Per-App Bar Color
### Added
- Bar title color adapts to focused app's identity via exe name hash
- `exe_hash_color()` function: deterministic color from app exe name using DefaultHasher
- Each app gets a unique bar title color for visual differentiation
- Falls back to theme accent color when no window is focused
- Bar title updated with both text and color on every focus change

### Changed
- `BarState::title_color` field set dynamically based on focused window's exe
- `DefaultHasher` import added to platform module

## [5.6.0] - 2026-08-29 — Enhanced Bar Title Rendering
### Added
- Title color rendering in status bar (configurable per-focus via `set_title_color`)
- `DT_END_ELLIPSIS` flag for long title truncation in bar
- Title rendering uses configurable `title_color` instead of fixed `fg_color`
- `set_title_color()` method on AppBar for per-window accent coloring
- `title_color` field in `BarState` struct
- Title right edge respects clock/volume/battery space allocation

### Changed
- Title text rendered with `title_color` instead of `fg_color` for visual distinction
- Long window titles now truncate with "..." instead of overflowing

## [5.5.0] - 2026-08-29 — Snap Layouts
### Added
- **snap-layout \<cols>x\<rows>** IPC command: rearranges all tiled windows into a flat grid
- `GridState::snap_layout()` method in layout engine
- Creates cols×rows cells and distributes visible windows in Z-order
- Supports patterns like `2x2` (4 quadrants), `3x1` (3 columns), `1x3` (3 rows), `2x1`, `1x2`
- Extra windows beyond the grid cell count are placed sequentially (wrapping)
- `Platform::snap_layout()` collects visible, non-floating windows and calls grid method
- Floating windows are excluded from snap layout arrangement

### Example IPC usage
```
echo {"command":"snap-layout 2x2"} | \\\\.\\pipe\\ultrawm-ipc
echo {"command":"snap-layout 3x1"} | \\\\.\\pipe\\ultrawm-ipc
```

## [5.4.0] - 2026-08-29 — All-Window Title Rendering in Border Overlay
### Added
- Window titles rendered for ALL managed windows, not just focused
- Focused window titles: accent color background with white text
- Unfocused window titles: dimmed text on semi-transparent dark background (0x40000000)
- Floating window titles: blue dimmed text matching floating border color
- Title text rendered after border drawing in `BorderOverlay::update()`
- `get_window_title()` called for every tiled window in `tile_all_windows()`
- `get_window_title()` called for every overview window in `tile_overview()`

### Changed
- Title rendering moved out of `if focused && !floating` block to apply to all windows
- Background brush color for titles: `color_rgb & 0xFF333333` for focused, `0x40000000` for unfocused

## [5.3.0] - 2026-08-29 — Geometry Constraint Enforcement
### Added
- **clamp-focused** IPC command: clamps focused floating window to its min/max size constraints
- Geometry constraints enforced in `edge_tile_window()` — max_width, max_height, min_width, min_height
- Geometry constraints enforced during border overlay mouse resize — respects per-window rules
- `clamp_focused_window()` method in Platform for runtime constraint enforcement
- Min/max checks in `edge_tile_window()`: `w = w.max(min_w).min(max_w)` pattern
- Min/max checks in border.rs resize handler via PLATFORM_PTR lookup

### Changed
- Rules-based min_width/min_height/max_width/max_height now enforced during all resize operations

## [5.2.0] - 2026-08-29 — Edge Tiling
### Added
- Edge tiling: drag windows to screen edges for maximize and half-screen snap
- Corner zones (20px) for quarter-screen snap (top-left, top-right, bottom-left, bottom-right)
- Edge modes: maximize (top edge), left half, right half, bottom half
- 8 mode codes sent via `WM_EDGE_TILE` custom message from border overlay
- `edge_tile_window()` method in Platform with monitor-aware positioning
- `find_monitor_for_window()` helper to detect which monitor a window is on
- WM_EDGE_TILE handler in message loop dispatches edge tile actions

### Fixed
- `find_monitor_for_window()` uses MonitorInfo.left/top/right/bottom fields (not x/y/width/height)

## [5.1.0] - 2026-08-29 — Managed Windows IPC Query with Full State
### Added
- **get-managed-windows** IPC command: returns full state for all UltraWM-managed windows
- Includes window id, hwnd, title, exe, workspace, floating, sticky, maximized, minimized, always_on_top, opacity, visible
- Uses PLATFORM_PTR for direct access to Platform.windows map
- Complements existing `get-windows` (which uses EnumWindows for all top-level windows)

## [5.0.0] - 2026-08-29 — Overlay Notification System with IPC and Event Notifications
### Added
- **Overlay notifications**: toast-style overlay in bottom-right corner with fade in/out
- **Workspace switch notifications**: shows current workspace number on switch
- **IPC `notify <message>` command**: trigger custom notifications externally
- 3-second display duration with 500ms fade out
- 200ms fade in animation on show
- Uses layered window (UpdateLayeredWindow) with per-frame alpha updates
- `Notifier` module with show/tick/position lifecycle

## [4.9.0] - 2026-08-29 — Idle Inhibit: Prevent Screen Lock and Sleep
### Added
- **idle-inhibit IPC command**: prevents screen lock/sleep via SetThreadExecutionState
- **idle-noinhibit IPC command**: re-enables screen lock/sleep
- Auto-inhibit when entering fullscreen mode, auto-release when exiting
- Uses ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED flags
- Uses Win32::System::Power for execution state management

## [4.8.0] - 2026-08-29 — Per-Monitor DPI Scaling with Independent Scale Factors
### Added
- **scale_factor** field on MonitorInfo calculated from effective DPI (dpi/96.0)
- **effective_width()** and **effective_height()** methods for DPI-aware pixel dimensions
- Per-monitor DPI queried via GetDpiForMonitor with MDT_EFFECTIVE_DPI
- DPI scaling applied consistently across window placement, border overlay, and bar positioning
- Fallback scale_factor: 1.0 for single-monitor fallback

## [4.7.0] - 2026-08-29 — Configurable Layout Algorithm: Auto-split and Default Split Direction
### Added
- **default_split_dir**: "vertical" (default) or "horizontal" — controls auto-split direction
- **auto_split**: when true, new windows automatically split the focused window's cell
- Auto-split places new window in the split cell and focuses it
- Supports both vertical (stack) and horizontal (side-by-side) auto-splitting

## [4.6.0] - 2026-08-29 — IPC Command Expansion: set-gap, set-corner-radius, set-border-width
### Added
- **set-gap \<value\>**: dynamically adjust gaps between tiled windows at runtime
- **set-corner-radius \<value\>**: dynamically adjust window corner radius at runtime
- **set-border-width \<value\>**: dynamically adjust border width at runtime
- All three commands update both the config and the live border overlay rendering
- IPC commands documented and available for external automation

## [4.5.0] - 2026-08-29 — Mouse Drag-to-Move for Tiled Windows
### Added
- **Drag-to-move tiled windows**: click and drag any tiled window to reposition it within the grid
- 5px drag threshold distinguishes drag from click (click = swap, drag = move)
- Green ghost rectangle shows target cell during drag
- Hand cursor during drag operation
- `drag_start`, `drag_active`, `drag_ghost` fields on BorderOverlay track drag state
- `drag_move_window()` method moves source window to target cell, shifting grid accordingly
- `WM_DRAG_MOVE` custom message (WM_USER + 0x102) for border-to-main-thread communication

## [4.4.0] - 2026-08-29 — Wallpaper Support: Theme-based Gradient, Image Wallpaper, IPC Commands
### Added
- **Theme wallpaper**: diagonal gradient from background to accent color, applied on theme switch
- **Image wallpaper**: `set-wallpaper-image <path>` IPC command to set any image as wallpaper
- **Color wallpaper**: `set-wallpaper <hex_color>` IPC command generates gradient wallpaper
- Wallpaper resolution matches primary monitor dimensions
- `apply_theme_wallpaper()` generates diagonal gradient from theme colors
- `apply_wallpaper()` now accepts width/height parameters for resolution matching
- `generate_accent_wallpaper()` creates smooth background-to-accent gradient BMP

## [4.3.0] - 2026-08-29 — Status Bar Improvements: Conditional Display, Volume Polling, Rounded Bar
### Added
- **Conditional bar elements**: `show_workspaces`, `show_clock`, `show_volume`, `show_battery` in BarConfig
- **Volume polling**: `get_volume_level()` via `waveOutGetVolume` (WAVE_MAPPER), updated every 5 seconds
- **Rounded bar corners**: `CreateRoundRectRgn` + `SetWindowRgn` applied to bar window
- `set_workspace_count()` method updates bar workspace indicators when workspace_count changes
- Battery indicator turns red below 20%

## [4.2.0] - 2026-08-29 — Window Visual Effects: Rounded Corners, DWM Shadows, Opacity
### Added
- **Rounded corners**: hardware-accelerated via `SetWindowRgn` + `CreateRoundRectRgn`
- **DWM drop shadows**: `DwmSetWindowAttribute` with `DWMWA_NCRENDERING_POLICY = DWMNCRP_ENABLED`
- **Window opacity**: `SetLayeredWindowAttributes` with `LWA_ALPHA` for per-window transparency
- Configurable via `rounded_corners`, `dwm_shadows`, `window_opacity`, `corner_radius` in LayoutConfig
- Visual effects applied automatically to each managed window via `apply_rounded_corners()`, `apply_dwm_shadow()`, `apply_window_opacity()`
- IPC `get-config` exposes new visual effect fields

## [4.1.0] - 2026-08-29 — Session Z-order Save/Restore
### Added
- **Z-order save/restore**: session saves window stacking order (Z-order) using GetTopWindow/GetWindow enumeration
- Windows restored to their previous Z-order position on startup
- z_order field on WindowInfo tracks per-window stacking position
- apply_z_order() sorts windows by saved Z-order and re-applies with SetWindowPos

## [4.0.0] - 2026-08-29 — Rule-based Float Position and Size
### Added
- **Float position and size in rules**: `float_x`, `float_y`, `float_w`, `float_h` fields
- Windows floated by rules now appear at the specified position and size
- Falls back to config `default_float_width`/`default_float_height` when not specified
- IPC `add-rule` accepts `float_x`, `float_y`, `float_w`, `float_h` parameters

## [3.9.0] - 2026-08-29 — Hand Cursor on Tiled Windows
### Added
- **Hand cursor on tiled windows**: hovering over a tiled window shows a hand cursor (move indicator)
- Resize cursors (sizeWE, sizeNS, sizeAll) take priority near edges
- Arrow cursor on empty space
- Clear visual feedback for clickable/movable windows

## [3.8.0] - 2026-08-29 — Window Shade/Roll-up
### Added
- **Shade/roll-up toggle**: `Win+Shift+S` rolls the focused window to just its title bar (~30px)
- Toggle again restores the original window size
- `shaded` field tracks per-window shade state
- Shaded windows excluded from tiling layout automatically

## [3.7.0] - 2026-08-29 — Click-to-Swap Window Move
### Added
- **Click-to-swap window move**: click a tiled window to select it, click another to swap positions
- Visual feedback: swap flash animation (white fade) on both windows
- Works on border overlay — no keyboard shortcut needed
- Cancel move by clicking empty space

## [3.6.0] - 2026-08-29 — Dynamic Opacity Control via IPC
### Added
- **`set-opacity` IPC command**: dynamically set focused window opacity at runtime
- Opacity value 0.0 (transparent) to 1.0 (opaque), clamped automatically
- Example: `echo '{"command":"set-opacity","value":0.7}' > \\.\pipe\ultrawm-ipc`
- Plain text: `echo 'set-opacity 0.7' > \\.\pipe\ultrawm-ipc`

## [3.5.0] - 2026-08-29 — Min Width/Height Window Constraints
### Added
- **`min_width` and `min_height` rule fields**: enforce minimum tiled window dimensions per-rule
- Pairs with existing `max_width`/`max_height` for full size control
- Rules applied at tiling time — windows never shrink below configured min size
- IPC `add-rule` now accepts `min_width` and `min_height` parameters

## [3.4.0] - 2026-08-29 — Configurable Floating Window Size
### Added
- **`default_float_width` and `default_float_height`** config options: set the default size for floated windows
- Default: 800x600, capped to monitor work area
- Centered on monitor when toggling float
- Example in config.toml: `default_float_width = 1024` and `default_float_height = 768`

## [3.3.0] - 2026-08-29 — Mouse-based Window Resize
### Added
- **Resize tiled windows by dragging edges**: hover near a window edge (6px zone) to see resize cursor
- Supports all four edges and corners (horizontal, vertical, and diagonal resize)
- Click and drag to resize — window dimensions update in real-time
- Minimum window size enforced at 100px

## [3.2.0] - 2026-08-29 — Enhanced Session Save/Restore
### Added
- **Session saves per-window state**: floating, workspace, opacity, sticky, maximized, always-on-top
- Session restore re-applies all properties when windows are re-managed on startup
- Windows return to their exact previous state after restart

## [3.1.0] - 2026-08-29 — Float Window Center on Monitor
### Added
- **Floating windows center on monitor**: toggling float centers the window at 50% of monitor work area
- Window positioned at monitor center with half-width/half-height dimensions
- Unfloat restores window to tiling layout with saved position

## [3.0.0] - 2026-08-29 — Window Size Constraints
### Added
- **`max_width` and `max_height` rule fields**: constrain tiled window dimensions per-rule
- Rules applied at tiling time — windows never exceed configured max size
- IPC `add-rule` now accepts `max_width` and `max_height` parameters
- Example: `echo '{"command":"add-rule","match":"class:Notepad","max_width":800,"max_height":600}' > \\.\pipe\ultrawm-ipc`

## [2.12.0] - 2026-08-28 — IPC reload-config Command
### Added
- **`reload-config` IPC command**: reload config.toml at runtime
- Applies new settings immediately (gaps, padding, borders, themes, rules)
- IPC: `echo '{"command":"reload-config"}' > \\.\pipe\ultrawm-ipc`

## [2.11.0] - 2026-08-28 — IPC list-rules Command
### Added
- **`list-rules` IPC command**: list all active window rules as JSON array
- Each rule shows: match pattern, float, workspace, opacity, sticky
- IPC: `echo '{"command":"list-rules"}' > \\.\pipe\ultrawm-ipc`

## [2.10.0] - 2026-08-28 — Enhanced get-state IPC
### Added
- **`get-state` IPC returns monitor count**: `{"status":"running","version":"x.y.z","monitors":N}`
- Useful for external tools to detect multi-monitor setups

## [2.9.0] - 2026-08-28 — unfloat-all Command
### Added
- **`unfloat-all` IPC command**: unfloat all floating windows at once
- Returns all windows to tiling layout
- Restores saved positions from before floating
- IPC: `echo '{"command":"unfloat-all"}' > \\.\pipe\ultrawm-ipc`

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
