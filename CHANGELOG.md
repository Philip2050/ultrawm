# Changelog

All notable changes to UltraWM will be documented in this file.

## [10.73.0] - 2026-08-29 — IPC Apply-Layout-Preset Command
### Added
- **IPC `apply-layout-preset` command**: apply a named layout preset at runtime
- Supports gaps, inner_padding, border_width, corner_radius
- Persists changes to config.toml

### IPC usage
```
echo '{"command":"apply-layout-preset","preset":"spacious"}' | \\.\pipe\ultrawm-ipc
```

## [10.72.0] - 2026-08-29 — IPC Move-Window-To-Workspace Command
### Added
- **IPC `move-window-to-workspace` command**: move any window to a specific workspace/monitor
- Supports `window_id` to target a specific window or `focused:true` for focused window
- Updates bar workspace indicators automatically

### IPC usage
```
echo '{"command":"move-window-to-workspace","window_id":123,"workspace":2,"monitor":0}' | \\.\pipe\ultrawm-ipc
echo '{"command":"move-window-to-workspace","workspace":1,"focused":true}' | \\.\pipe\ultrawm-ipc
```

## [10.71.0] - 2026-08-29 — IPC Get-Workspace-Names Command
### Added
- **IPC `get-workspace-names` command**: query workspace names for a monitor or all monitors
- Returns current active workspace index alongside names
- Useful for external workspace switchers and status bars

### IPC usage
```
echo '{"command":"get-workspace-names","monitor":0}' | \\.\pipe\ultrawm-ipc
echo '{"command":"get-workspace-names"}' | \\.\pipe\ultrawm-ipc
```

## [10.70.0] - 2026-08-29 — IPC Get-Session Command
### Added
- **IPC `get-session` command**: query current session state
- Returns session info including saved window count, workspace count, and timestamp
- Shows current window count and monitor count alongside session data

### IPC usage
```
echo '{"command":"get-session"}' | \\.\pipe\ultrawm-ipc
```

## [10.69.0] - 2026-08-29 — IPC List-Workspaces Command Enhancement
### Added
- **Enhanced `list-workspaces` command**: now lists all workspaces across all monitors
- Returns monitor name, active state, window count, and full window details per workspace
- Useful for external status bars and workspace switchers

### IPC usage
```
echo '{"command":"list-workspaces"}' | \\.\pipe\ultrawm-ipc
```

## [10.68.0] - 2026-08-29 — IPC Get-Monitor-Layout Command
### Added
- **IPC `get-monitor-layout` command**: query per-monitor layout settings
- Returns gaps, padding, border width, corner radius with effective values
- Lists all monitors or a specific monitor's layout

### IPC usage
```
echo '{"command":"get-monitor-layout","monitor":0}' | \\.\pipe\ultrawm-ipc
echo '{"command":"get-monitor-layout"}' | \\.\pipe\ultrawm-ipc
```

## [10.67.0] - 2026-08-29 — IPC Set-Monitor-Layout Command
### Added
- **IPC `set-monitor-layout` command**: set per-monitor layout properties
- Supports gaps, inner_padding, border_width, corner_radius per monitor
- Overrides global layout settings for the target monitor
- Persists to config.toml

### IPC usage
```
echo '{"command":"set-monitor-layout","monitor":0,"gaps":12,"border_width":3}' | \\.\pipe\ultrawm-ipc
```

## [10.66.0] - 2026-08-29 — IPC Add/Remove-Monitor-Workspace Commands
### Added
- **IPC `add-monitor-workspace` command**: add a new workspace for a specific monitor
- **IPC `remove-monitor-workspace` command**: remove the last workspace from a specific monitor
- Updates global `workspace_count` config to match
- Updates bar workspace indicator automatically

### IPC usage
```
echo '{"command":"add-monitor-workspace","monitor":0}' | \\.\pipe\ultrawm-ipc
echo '{"command":"remove-monitor-workspace","monitor":0}' | \\.\pipe\ultrawm-ipc
```

## [10.65.0] - 2026-08-29 — IPC Set-Monitor-Workspace Command
### Added
- **IPC `set-monitor-workspace` command**: switch a specific monitor to a workspace by index
- Updates bar workspace indicator for the target monitor
- Useful for external scripts that need to control per-monitor workspaces

### IPC usage
```
echo '{"command":"set-monitor-workspace","monitor":0,"workspace":2}' | \\.\pipe\ultrawm-ipc
```

## [10.64.0] - 2026-08-29 — IPC Set-Wallpaper-Image-Monitor Command
### Added
- **IPC `set-wallpaper-image-monitor` command**: set an image wallpaper for a specific monitor
- Stores the wallpaper path in `wallpapers` for persistence
- Supports bmp, jpg, png, jpeg image formats
- Falls back to global wallpaper on failure

### IPC usage
```
echo '{"command":"set-wallpaper-image-monitor","path":"C:\\wallpaper.jpg","monitor":0}' | \\.\pipe\ultrawm-ipc
```

## [10.63.0] - 2026-08-29 — IPC List-Monitor-Workspaces Command
### Added
- **IPC `list-monitor-workspaces` command**: list workspaces for a specific monitor or all monitors
- Returns workspace index, active state, and window count per workspace
- Useful for external status bars and workspace indicators

### IPC usage
```
echo '{"command":"list-monitor-workspaces","monitor":0}' | \\.\pipe\ultrawm-ipc
echo '{"command":"list-monitor-workspaces"}' | \\.\pipe\ultrawm-ipc
```

## [10.62.0] - 2026-08-29 — IPC Get-Monitor-Info Command
### Added
- **IPC `get-monitor-info` command**: get detailed info about all monitors
- Returns monitor index, name, position, size, scale factor, workspace info, and window count
- Includes primary monitor flag and work area dimensions

### IPC usage
```
echo '{"command":"get-monitor-info"}' | \\.\pipe\ultrawm-ipc
```

## [10.61.0] - 2026-08-29 — IPC Reload-Config Command
### Added
- **IPC `reload-config` command**: reload config from disk without restarting
- Uses `Config::reload_if_changed()` for efficient change detection
- Saves updated config and returns reload status

### IPC usage
```
echo '{"command":"reload-config"}' | \\.\pipe\ultrawm-ipc
```

## [10.60.0] - 2026-08-29 — IPC List-All-Windows Command
### Added
- **IPC `list-all-windows` command**: list all managed windows with their properties
- Returns window ID, title, class, exe, monitor, workspace, opacity, border, and focus state
- Useful for external scripts that need full window inventory

### IPC usage
```
echo '{"command":"list-all-windows"}' | \\.\pipe\ultrawm-ipc
```

## [10.59.0] - 2026-08-29 — IPC List-Monitor-Bars Command
### Added
- **IPC `list-monitor-bars` command**: list all monitors with their bar settings
- Returns per-monitor bar config: enabled, height, transparency, and monitor name
- Useful for syncing bar state across external tools

### IPC usage
```
echo '{"command":"list-monitor-bars"}' | \\.\pipe\ultrawm-ipc
```

## [10.58.0] - 2026-08-29 — IPC Get-Active-Monitor Command
### Added
- **IPC `get-active-monitor` command**: query which monitor currently has focus
- Returns monitor index, name, and focused window ID
- Useful for external scripts that need to track active display

### IPC usage
```
echo '{"command":"get-active-monitor"}' | \\.\pipe\ultrawm-ipc
```

## [10.57.0] - 2026-08-29 — Per-Monitor Bar Enabled State
### Added
- **IPC `set-monitor-bar-enabled` command**: show/hide bar for a specific monitor
- **IPC `get-monitor-bar-enabled` command**: query bar enabled state for a monitor or list all overrides
- **Per-monitor bar enabled**: `bar_enabled_monitors: HashMap<usize, bool>` in Platform
- Falls back to global `config.bar.enabled` when no override is set

### IPC usage
```
echo '{"command":"set-monitor-bar-enabled","monitor":1,"enabled":false}' | \\.\pipe\ultrawm-ipc
echo '{"command":"get-monitor-bar-enabled","monitor":0}' | \\.\pipe\ultrawm-ipc
echo '{"command":"get-monitor-bar-enabled"}' | \\.\pipe\ultrawm-ipc
```

## [10.56.0] - 2026-08-29 — Per-Monitor Bar Transparency
### Added
- **IPC `set-monitor-bar-transparency` command**: set bar transparency for a specific monitor (0.0-1.0)
- **IPC `get-monitor-bar-transparency` command**: query bar transparency for a monitor or list all overrides
- **Per-monitor bar transparencies**: `bar_transparencies: HashMap<usize, f32>` in Platform
- Falls back to global `config.bar.transparency` when no override is set

### IPC usage
```
echo '{"command":"set-monitor-bar-transparency","monitor":0,"transparency":0.5}' | \\.\pipe\ultrawm-ipc
echo '{"command":"get-monitor-bar-transparency","monitor":1}' | \\.\pipe\ultrawm-ipc
echo '{"command":"get-monitor-bar-transparency"}' | \\.\pipe\ultrawm-ipc
```

## [10.55.0] - 2026-08-29 — Per-Window Opacity IPC Commands
### Added
- **IPC `set-window-opacity` command**: set transparency for any window by ID or focused window
- **IPC `get-window-opacity` command**: query opacity of any window by ID or focused window
- Supports opacity values from 0.0 (invisible) to 1.0 (fully opaque)
- Uses `SetLayeredWindowAttributes` with LWA_ALPHA for smooth per-window transparency
- Falls back to focused window when no window_id is specified

### IPC usage
```
echo '{"command":"set-window-opacity","window_id":123,"opacity":0.75}' | \\.\pipe\ultrawm-ipc
echo '{"command":"set-window-opacity","opacity":0.5}' | \\.\pipe\ultrawm-ipc
echo '{"command":"get-window-opacity","window_id":123}' | \\.\pipe\ultrawm-ipc
```

## [10.54.0] - 2026-08-29 — Per-Monitor Bar Height
### Added
- **IPC `set-monitor-bar-height` command**: set custom bar height for a specific monitor (20-200px)
- **IPC `get-monitor-bar-height` command**: query bar height for a monitor or list all overrides
- **Per-monitor bar heights**: `bar_heights: HashMap<usize, u32>` in Platform for per-monitor overrides
- Falls back to global `config.bar.height` when no override is set for a monitor

### IPC usage
```
echo '{"command":"set-monitor-bar-height","monitor":0,"height":50}' | \\.\pipe\ultrawm-ipc
echo '{"command":"get-monitor-bar-height","monitor":1}' | \\.\pipe\ultrawm-ipc
echo '{"command":"get-monitor-bar-height"}' | \\.\pipe\ultrawm-ipc
```

## [10.53.0] - 2026-08-29 — Focus-Follows-Mouse IPC Commands
### Added
- **IPC `set-focus-follows-mouse` command**: enable/disable focus-follows-mouse at runtime
- **IPC `get-focus-follows-mouse` command**: query current focus-follows-mouse status
- **IPC `set-mouse-threshold` command**: set mouse follow threshold in milliseconds (0-5000)
- **IPC `get-mouse-threshold` command**: query current mouse follow threshold
- **IPC `set-monitor-focus` command**: switch focus to a specific monitor by index
- `mouse_follow_threshold` config field for tuning focus delay (default: 1000ms)

### IPC usage
```
echo '{"command":"set-focus-follows-mouse","enabled":true}' | \\.\pipe\ultrawm-ipc
echo '{"command":"set-mouse-threshold","threshold":500}' | \\.\pipe\ultrawm-ipc
echo '{"command":"set-monitor-focus","monitor":1}' | \\.\pipe\ultrawm-ipc
```

## [10.52.0] - 2026-08-29 — Per-Monitor Wallpaper on Monitor Switch
### Added
- **Automatic wallpaper on monitor switch**: applies per-monitor wallpaper when focus changes monitors
- Supports both image wallpapers (bmp/jpg/png) and color wallpapers via `apply_theme_wallpaper`
- Completes the per-monitor wallpaper feature with dynamic application on display change

## [10.51.0] - 2026-08-29 — IPC Set-Bar-Position Command
### Added
- **IPC `set-bar-position` command**: move the status bar to top or bottom at runtime
- Repositions the bar window using `SetWindowPos`
- Persists position to `config.toml`
- Supports values: "top" or "bottom"

### IPC usage
```
echo '{"command":"set-bar-position","position":"bottom"}' | \\.\pipe\ultrawm-ipc
```

## [10.50.0] - 2026-08-29 — Per-Monitor Wallpaper Support
### Added
- **Per-monitor wallpaper paths**: `wallpapers: Vec<Option<String>>` in Platform
- **IPC `set-wallpaper-monitor` command**: set wallpaper for a specific monitor
- **Color wallpaper**: generates gradient BMP and applies via SystemParametersInfoW
- Per-monitor wallpapers stored in Platform state for each monitor

### IPC usage
```
echo '{"command":"set-wallpaper-monitor","color":"#1E1E2E","monitor":0}' | \\.\pipe\ultrawm-ipc
```

## [10.49.0] - 2026-08-29 — IPC Set-Workspace-Name Command
### Added
- **IPC `set-workspace-name` command**: rename a workspace at runtime
- Supports optional `monitor` parameter for per-monitor workspace naming
- Updates bar workspace indicators immediately
- Persists to `config.toml` via `save()`

### IPC usage
```
echo '{"command":"set-workspace-name","workspace":1,"name":"Web","monitor":0}' | \\.\pipe\ultrawm-ipc
```

## [10.48.0] - 2026-08-29 — IPC Get-Layout-Presets Command
### Added
- **IPC `get-layout-presets` command**: returns all layout presets as JSON
- Each preset: name, kind (columns/rows/master/fibonacci/custom), cols, rows
- Useful for layout picker tools and status bars

### IPC usage
```
echo '{"command":"get-layout-presets"}' | \\.\pipe\ultrawm-ipc
```

### Response format
```json
{
  "success": true,
  "command": "get-layout-presets",
  "presets": [
    {"name": "default", "kind": "columns", "cols": null, "rows": null}
  ],
  "count": 1
}
```

## [10.47.0] - 2026-08-29 — IPC List-Themes Command
### Added
- **IPC `list-themes` command**: returns all available theme names as JSON
- Includes current theme name and total count
- Themes sourced from `ThemeManager::theme_names()`
- Useful for theme picker tools and status bars

### IPC usage
```
echo '{"command":"list-themes"}' | \\.\pipe\ultrawm-ipc
```

### Response format
```json
{
  "success": true,
  "command": "list-themes",
  "current": "catppuccin-mocha",
  "themes": ["catppuccin-mocha", "dracula", "nord", "tokyo-night"],
  "count": 4
}
```

## [10.46.0] - 2026-08-29 — IPC Cycle-Theme Command
### Added
- **IPC `cycle-theme` command**: cycle to the next theme in the theme list
- Uses `ThemeManager::next_theme()` for theme cycling
- Persists the new theme to `config.toml`
- Pairs with `set-theme` for programmatic theme control

### IPC usage
```
echo '{"command":"cycle-theme"}' | \\.\pipe\ultrawm-ipc
```

## [10.45.0] - 2026-08-29 — IPC Get-Window-Info Command
### Added
- **IPC `get-window-info` command**: returns detailed info about the focused window as JSON
- Window identity: hwnd, id, title, class, exe
- State: visible, floating, fullscreen, always_on_top, minimized, maximized, sticky
- Layout: monitor, workspace, z_order
- Appearance: border_color, border_width, opacity
- Useful for debugging, scripting, and window management tools

### IPC usage
```
echo '{"command":"get-window-info"}' | \\.\pipe\ultrawm-ipc
```

### Response format
```json
{
  "success": true,
  "command": "get-window-info",
  "data": {
    "hwnd": 123456,
    "id": 1,
    "title": "Terminal",
    "class": "ConsoleWindowClass",
    "exe": "wt.exe",
    "visible": true,
    "floating": false,
    "fullscreen": false,
    "always_on_top": false,
    "minimized": false,
    "maximized": false,
    "sticky": false,
    "monitor": 0,
    "workspace": 1,
    "border_color": "0xFFFF4455",
    "border_width": 2,
    "opacity": 1.0,
    "z_order": 0
  }
}
```

## [10.44.0] - 2026-08-29 — IPC Toggle-Snap Command
### Added
- **IPC `toggle-snap` command**: toggle snap mode on/off
- Updates `snap_mode` state and bar snap indicator
- Snap mode enables visual window snapping to screen edges

### IPC usage
```
echo '{"command":"toggle-snap"}' | \\.\pipe\ultrawm-ipc
```

## [10.43.0] - 2026-08-29 — IPC Cycle-Gap Command
### Added
- **IPC `cycle-gap` command**: cycles tiling gap through presets [0, 4, 8, 16, 32]
- Automatically re-tiles all windows after gap change
- Triggers bar reload flash for visual feedback
- Useful for quickly adjusting tiling density

### IPC usage
```
echo '{"command":"cycle-gap"}' | \\.\pipe\ultrawm-ipc
```

### Gap presets
0 → 4 → 8 → 16 → 32 → 0 (cycles)

## [10.42.0] - 2026-08-29 — IPC Set-Theme Command
### Added
- **IPC `set-theme` command**: change the active theme by name at runtime
- Applies theme via `ThemeManager::apply_theme()`
- Persists the new theme to `config.toml`
- Updates bar colors, border colors, and wallpaper

### IPC usage
```
echo '{"command":"set-theme","theme":"catppuccin-mocha"}' | \\.\pipe\ultrawm-ipc
echo 'set-theme dracula' | \\.\pipe\ultrawm-ipc
```

## [10.41.0] - 2026-08-29 — IPC Reset-Layout Command
### Added
- **IPC `reset-layout` command**: re-tiles all windows on the current workspace
- Calls `refresh_tiling()` to recalculate and apply the current layout
- Useful when the layout gets messy after many window operations

### IPC usage
```
echo '{"command":"reset-layout"}' | \\.\pipe\ultrawm-ipc
```

## [10.40.0] - 2026-08-29 — Animated Opacity from Window Rules
### Added
- **Opacity animation on rule match**: when a window rule sets opacity, it animates smoothly using spring physics
- **Spring-based transition**: uses existing `opacity_anim` HashMap with stiffness=200, damping=25
- Smooth fade from current opacity to target opacity over ~0.5 seconds
- Falls back to instant set if animation system not available

### Window rule example
```toml
[[rules]]
match = "discord"
opacity = 0.85
```

## [10.39.0] - 2026-08-29 — IPC Set-Border-Width Command
### Added
- **IPC `set-border-width` command**: change focused window's border width at runtime
- Accepts pixel value: `set-border-width 4` (or 0 to reset to default)
- Updates `window_border_widths` HashMap, takes effect on next render
- Pairs with `set-border-color` for dynamic window styling

### IPC usage
```
echo '{"command":"set-border-width","width":4}' | \\.\pipe\ultrawm-ipc
echo 'set-border-width 0' | \\.\pipe\ultrawm-ipc
```

## [10.38.0] - 2026-08-29 — IPC Get-Theme Command
### Added
- **IPC `get-theme` command**: returns current theme name and colors as JSON
- Theme name from `config.theme.default`
- Colors from `ThemeManager::current_colors()`
- Useful for theming tools, scripts, and status bars

### IPC usage
```
echo '{"command":"get-theme"}' | \\.\pipe\ultrawm-ipc
```

### Response format
```json
{
  "success": true,
  "command": "get-theme",
  "data": {
    "name": "catppuccin-mocha",
    "colors": {
      "background": "#1E1E2E",
      "foreground": "#CDD6F4",
      "accent": "#CBA6F7"
    }
  }
}
```

## [10.37.0] - 2026-08-29 — IPC Set-Border-Color Command
### Added
- **IPC `set-border-color` command**: change focused window's border color at runtime
- Accepts hex color string: `set-border-color FF0000` or `set-border-color 0xFF0000`
- Updates `win_info.border_color` directly, takes effect on next render
- Useful for color-coding windows dynamically

### IPC usage
```
echo '{"command":"set-border-color","color":"FF4488FF"}' | \\.\pipe\ultrawm-ipc
echo 'set-border-color 0x00FF00' | \\.\pipe\ultrawm-ipc
```

## [10.36.0] - 2026-08-29 — IPC Get-Bar-Config Command
### Added
- **IPC `get-bar-config` command**: returns bar configuration and runtime state as JSON
- Config includes: enabled, height, position, transparency, workspace/clock/volume/battery/cpu/memory toggles
- State includes: visible (runtime), height (actual)
- Useful for status bars and tools that need bar settings

### IPC usage
```
echo '{"command":"get-bar-config"}' | \\.\pipe\ultrawm-ipc
```

### Response format
```json
{
  "success": true,
  "command": "get-bar-config",
  "config": {
    "enabled": true,
    "height": 40,
    "position": "top",
    "transparency": 0.85,
    "show_workspaces": true,
    "show_clock": true,
    "show_volume": true,
    "show_battery": true,
    "show_cpu": true,
    "show_memory": true
  },
  "state": {
    "visible": true,
    "height": 40
  }
}
```

## [10.35.0] - 2026-08-29 — Scrolling Window Titles in Bar
### Added
- **Scrolling titles**: long window titles scroll horizontally when they exceed available space
- **Scroll pause**: titles pause for 60 frames (1 second) before scrolling starts
- **Scroll loop**: title scrolls left, then wraps back to right after a gap
- **Reset on change**: scroll position resets when the title changes
- Uses `GetTextExtentPoint32W` for text width measurement
- Uses `TextOutW` for positioned text drawing

### Bar behavior
- Short titles (< available width): displayed statically, no scrolling
- Long titles: scroll left at 1px per frame after 1s pause
- Scroll offset resets when title changes

## [10.34.0] - 2026-08-29 — IPC List-Monitors Command
### Added
- **IPC `list-monitors` command**: returns all monitor info as JSON
- Each monitor entry: width, height, work area, DPI, primary status, current workspace, workspace count
- Useful for status bars, scripts, and multi-monitor tools

### IPC usage
```
echo '{"command":"list-monitors"}' | \\.\pipe\ultrawm-ipc
```

### Response format
```json
{
  "success": true,
  "command": "list-monitors",
  "count": 2,
  "monitors": [
    {
      "index": 0,
      "width": 1920,
      "height": 1080,
      "work_width": 1920,
      "work_height": 1040,
      "work_left": 0,
      "work_top": 40,
      "dpi": 96,
      "primary": true,
      "current_workspace": 0,
      "workspace_count": 4
    }
  ]
}
```

## [10.33.0] - 2026-08-29 — Keybinds for Swap, Toggle Bar, Bring to Front
### Added
- **`swap_windows` keybind** (default: Win+S): swaps focused window with next tiling window
- **`toggle_bar` keybind** (default: Win+B): shows/hides the status bar
- **`bring_to_front` keybind** (default: Win+.): brings focused floating window to front of z-order
- Keybinds use same IPC commands: `swap-windows`, `toggle-bar`, `bring-to-front`
- Consistent behavior between keybind and IPC triggers

### Default keybinds
- Win+S — swap focused window with next
- Win+B — toggle bar visibility
- Win+. — bring floating window to front

## [10.32.0] - 2026-08-29 — Per-App Border Width from Window Rules
### Added
- **`border_width` field in WindowRule**: rules can set custom border width per app
- **Per-window border width storage**: `window_border_widths: HashMap<u64, u32>` in Platform
- **Border rendering**: per-window border width passed through `border_rects` tuple and applied in BorderOverlay
- Rules override global `layout.border_width` for matching windows

### Config example
```toml
[[rules]]
match = "chrome"
border_width = 4

[[rules]]
match = "terminal"
border_width = 1
```

## [10.31.0] - 2026-08-29 — IPC Get-Stats Command
### Added
- **IPC `get-stats` command**: returns WM statistics as JSON
- Window counts: total, tiling, floating
- Monitor count, total workspaces across all monitors
- Runtime state: monocle, snap_mode, overview, scratchpad count
- Focused HWND, rules count

### IPC usage
```
echo '{"command":"get-stats"}' | \\.\pipe\ultrawm-ipc
```

### Response format
```json
{
  "success": true,
  "command": "get-stats",
  "data": {
    "total_windows": 5,
    "tiling_windows": 3,
    "floating_windows": 2,
    "monitors": 2,
    "total_workspaces": 8,
    "focused_hwnd": 123456,
    "monocle": false,
    "snap_mode": false,
    "overview": false,
    "scratchpads": 1,
    "rules_count": 3
  }
}
```

## [10.30.0] - 2026-08-29 — Per-App Corner Radius from Window Rules
### Added
- **`corner_radius` field in WindowRule**: rules can set custom corner radius per app
- **Rule application**: `corner_radius` from matching rule applied via `apply_rounded_corners()`
- Per-app rounded corners override global `layout.corner_radius`

### Config example
```toml
[[rules]]
match = "chrome"
corner_radius = 12

[[rules]]
match = "terminal"
corner_radius = 0
```

## [10.29.0] - 2026-08-29 — Floating Window Z-Order Management
### Added
- **Floating window bring-to-front**: focused floating windows automatically get HWND_TOPMOST via SetWindowPos
- **Z-order on focus change**: `on_focus_changed()` calls SetWindowPos(HWND_TOP) for floating windows
- Floating windows no longer get stuck behind tiling windows when focused
- Clicking a floating window now properly brings it to the front

### IPC usage
```
echo '{"command":"bring-to-front"}' | \\.\pipe\ultrawm-ipc
```

## [10.28.0] - 2026-08-29 — IPC Toggle-Bar Command
### Added
- **IPC `toggle-bar` command**: show/hide the status bar at runtime
- **`toggle_bar_visibility()`**: toggles bar ShowWindow(SW_SHOW/SW_HIDE)
- **`bar_visible` state**: runtime visibility independent of config `bar.enabled`
- IPC response confirms bar shown or hidden

### IPC usage
```
echo '{"command":"toggle-bar"}' | \\.\pipe\ultrawm-ipc
```

## [10.27.0] - 2026-08-29 — IPC Swap-Windows Command
### Added
- **IPC `swap-windows` command**: swaps the focused window with the next tiling window
- Uses existing `swap_windows()` with grid cell position swap
- Includes swap flash animation (white border flash on both windows)
- Useful for keybind-driven window reordering without drag

### IPC usage
```
echo '{"command":"swap-windows"}' | \\.\pipe\ultrawm-ipc
```

## [10.26.0] - 2026-08-29 — IPC Get-Config Command
### Added
- **IPC `get-config` command**: returns current configuration as JSON
- Includes layout, keybinds, theme, bar, launcher config, and window rules
- Useful for scripts, tools, and status bars that need to read config programmatically
- `last_modified` timestamp excluded from output (internal field)

### IPC usage
```
echo '{"command":"get-config"}' | \\.\pipe\ultrawm-ipc
```

### Response format
```json
{
  "success": true,
  "command": "get-config",
  "data": {
    "layout": { "gaps": 8, "border_width": 2, ... },
    "keybinds": { "mod_key": "win", ... },
    "theme": { "default": "catppuccin-mocha", ... },
    "bar": { "enabled": true, "height": 40, ... },
    "launcher": { "enabled": true, ... },
    "rules": [ ... ]
  }
}
```

## [10.25.0] - 2026-08-29 — IPC List-Workspaces Command
### Added
- **IPC `list-workspaces` command**: returns all workspaces across monitors as JSON
- Each workspace entry includes: monitor index, workspace index, name, window count, active status
- Optional `monitor` filter parameter: `{"command":"list-workspaces","monitor":1}`
- Response includes focused monitor and workspace indices
- Useful for status bars, scripts, and tools that need workspace state

### IPC usage
```
echo '{"command":"list-workspaces"}' | \\.\pipe\ultrawm-ipc
echo '{"command":"list-workspaces","monitor":0}' | \\.\pipe\ultrawm-ipc
```

### Response format
```json
{
  "success": true,
  "command": "list-workspaces",
  "focused_monitor": 0,
  "focused_workspace": 1,
  "workspaces": [
    {"monitor": 0, "index": 0, "name": "1", "windows": 2, "active": false},
    {"monitor": 0, "index": 1, "name": "2", "windows": 1, "active": true}
  ]
}
```

## [10.24.0] - 2026-08-29 — Status Bar Widget Enhancements
### Added
- **Monocle mode indicator**: orange "MONOCLE" label shown in bar when monocle layout is active
- **Network status indicator**: green "W" (online) or red "W!" (offline) shown in bar
- **`is_network_online()`**: checks internet connectivity via `InternetGetConnectedState` from wininet.dll
- Bar updates network status dynamically on each render tick
- Monocle indicator automatically shown/hidden when monocle layout toggles

### Bar layout
- Monocle indicator appears after workspace buttons (orange text)
- Network indicator follows monocle indicator (colored text)
- Both indicators integrate with existing bar spacing and rendering

## [10.23.0] - 2026-08-29 — Window Rules with Priority & Match Types
### Added
- **Rule priority**: higher-priority rules override lower-priority ones (default: 0)
- **Match type field**: choose matching by "exe", "class", "title", or "any"
- **Monitor assignment**: rules can assign windows to specific monitors
- **Always-on-top**: rules can set always-on-top state
- **Fullscreen**: rules can set initial fullscreen state
- **Border color**: rules can set custom border color per window
- **Priority sorting**: rules sorted by priority before applying (lower first)
- **Enhanced apply_rules()**: uses `matches_any()` with match_type support

### Rule fields
- `match_type`: "exe" (default), "class", "title", or "any" (match any field)
- `priority`: i32, higher = applied later and overrides (default: 0)
- `monitor`: target monitor index for window placement
- `always_on_top`: bool, set window always on top
- `fullscreen`: bool, set initial fullscreen state
- `border_color`: u32 ARGB, custom border color

### Config example
```toml
[[rules]]
match = "chrome"
match_type = "exe"
float = false
workspace = 0
monitor = 0
priority = 10

[[rules]]
match = "notepad"
match_type = "exe"
float = true
opacity = 0.9
priority = 5
```

## [10.22.0] - 2026-08-29 — Multi-Monitor Focus & Window Transfer
### Added
- **`move_focused_to_monitor()`**: move focused window to target monitor
- **`focus_next_monitor()`**: cycle focus to the next monitor
- **`focus_monitor()`**: focus a specific monitor and switch to its workspace
- **IPC `move-to-monitor`**: `{"command":"move-to-monitor","monitor":1}` — move focused window to monitor 2
- **IPC `focus-monitor`**: `{"command":"focus-monitor","monitor":0}` — focus primary monitor
- **IPC `focus-next-monitor`**: cycle focus across all monitors
- Window position updates when moved between monitors
- Auto re-tiling after monitor transfer

### IPC usage
```
echo '{"command":"move-to-monitor","monitor":1}' | \\.\pipe\ultrawm-ipc
echo '{"command":"focus-next-monitor"}' | \\.\pipe\ultrawm-ipc
echo '{"command":"focus-monitor","monitor":0}' | \\.\pipe\ultrawm-ipc
```

## [10.21.0] - 2026-08-29 — Enhanced Diagnostics & Performance Metrics
### Added
- **Startup status in diagnostics**: shows whether UltraWM is set to run on login
- **Per-app opacity memory in diagnostics**: lists all remembered app opacities
- **CPU usage reporting**: real-time CPU percentage via GetSystemTimes
- **Monocle mode status**: shows current monocle state in diagnostics
- **IPC `diagnose` command**: run full diagnostics from scripts
- **Comprehensive system info**: DWM, DPI, shell, config, themes, monitors, workspaces, windows, bar, hooks, session

### Diagnostics output includes
- DWM composition status
- Primary monitor DPI
- Shell replacement status (Explorer vs UltraWM)
- Startup integration status
- Config path, existence, last modified
- Available themes
- Monitor count, sizes, DPI, work areas
- Per-monitor workspaces, grids, cameras, window positions
- Managed windows (title, class, exe, id, float state)
- Bar, border overlay, keyboard hook status
- Session state
- Per-app opacity memory entries
- CPU usage percentage
- Monocle mode status

### IPC usage
```
echo '{"command":"diagnose"}' | \\.\pipe\ultrawm-ipc
```

## [10.20.0] - 2026-08-29 — Theme Picker with Per-Pixel Alpha Rendering
### Added
- **Per-pixel alpha theme picker**: modern overlay with rounded corners via UpdateLayeredWindow
- **Dark theme styling**: Catppuccin-inspired colors (base #1E1E2E, accent #CBA6F7)
- **Color preview dots**: each theme entry shows a colored preview indicator
- **Selected highlight**: current theme highlighted with accent color background
- **Header bar**: top bar with theme picker title
- **Centered positioning**: 420x340 window centered on primary monitor
- **Keyboard navigation**: Escape to close, Enter to apply, Up/Down to browse
- **Consistent rendering**: same per-pixel alpha pattern as help overlay and notifier

### Visual Design
- Rounded corners (16px outer, 8px item corners)
- Semi-transparent background (0xCC alpha)
- Accent color selection highlight
- Color preview circle for each theme

## [10.19.0] - 2026-08-29 — Dynamic Snap Layout Save & List
### Added
- **`save_snap_layout()`**: save current grid's custom widths/heights as a named snap layout
- **Dynamic layout persistence**: save current tiling arrangement to config.toml
- **IPC `save-snap-layout`**: save current tiling as named layout
- **IPC `list-snap-layouts`**: list all saved snap layouts with widths/heights
- Auto-removes existing layout with same name before saving
- Uses grid's `custom_widths` and `custom_heights` from current arrangement

### IPC usage
```
echo '{"command":"save-snap-layout","name":"my-layout"}' | \\.\pipe\ultrawm-ipc
echo '{"command":"list-snap-layouts"}' | \\.\pipe\ultrawm-ipc
echo '{"command":"snap-custom","name":"my-layout"}' | \\.\pipe\ultrawm-ipc
```

## [10.18.0] - 2026-08-29 — System Tray Icon with Quick Actions Menu
### Added
- **Persistent system tray icon**: UltraWM icon shown in Windows notification area
- **Right-click context menu**: quick access to common actions
- **Left-click notification**: shows UltraWM status notification
- **Context menu actions**: Toggle Monocle, Save Session, Next Theme, Show Help, Window Search, Quit UltraWM
- **`create_tray_icon()`**: creates tray icon with NOTIFYICONDATAW (NIF_ICON | NIF_MESSAGE | NIF_TIP)
- **Tray window message handler**: processes WM_APP_TRAY and WM_APP_TRAYMENU messages
- **Popup menu**: TrackPopupMenu with left-aligned, right-button positioning
- **Icon fallback**: uses IDI_APPLICATION if custom icon resource not found

### Context Menu
```
UltraWM
──────────────
Toggle Monocle
Save Session
Next Theme
Show Help
Window Search
──────────────
Quit UltraWM
```

### IPC
- No new commands needed — tray icon is created automatically on initialization

## [10.17.0] - 2026-08-29 — Daemon Mode & Windows Startup Integration
### Added
- **`enable_startup()`**: creates a shortcut in the Windows Startup folder for auto-launch on login
- **`disable_startup()`**: removes the UltraWM.lnk shortcut from Startup folder
- **`is_startup_enabled()`**: checks if UltraWM is configured to run on login
- **IPC `enable-startup`**: add UltraWM to Windows startup
- **IPC `disable-startup`**: remove UltraWM from Windows startup
- **IPC `startup-status`**: check if UltraWM is set to run on login
- **Shell replacement support**: `--install` sets UltraWM as default shell (replaces Explorer)
- **Shell restore**: `--uninstall` restores previous shell from backup
- Shortcut uses `--start` argument and hidden window style (7)

### Startup behavior
- Shortcut placed in: `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\UltraWM.lnk`
- Uses hidden window style to avoid popup on login
- Working directory set to UltraWM installation folder

### IPC usage
```
echo '{"command":"enable-startup"}' | \\.\pipe\ultrawm-ipc
echo '{"command":"disable-startup"}' | \\.\pipe\ultrawm-ipc
echo '{"command":"startup-status"}' | \\.\pipe\ultrawm-ipc
```

## [10.16.0] - 2026-08-29 — Configuration Hot-Reload with Change Diff
### Added
- **Comprehensive config diff**: compares 20+ layout fields on reload and reports exact changes
- **Change notification**: shows notification with count of changed settings
- **Automatic apply**: all layout changes (gaps, padding, peek, border, opacity, etc.) applied immediately
- **Bar reload flash**: visual indicator when config is reloaded
- **Structured logging**: `Config reloaded (N changes): field1: old → new; field2: old → new`
- **No-change detection**: debug log when config file changes but values are identical

### Changed fields tracked
- gaps, inner_padding, outer_padding, peek_x, peek_y
- border_width, corner_radius, window_opacity
- workspace_count, default_float_width, default_float_height
- snap_grid_size, snap_edge_distance
- spring_stiffness, spring_damping, center_focused
- focus_follows_mouse, auto_split, default_split_dir
- resize_step_px, session_auto_save_interval

### IPC
- No new commands needed — reload is automatic every 60 frames (~1 second)

## [10.15.0] - 2026-08-29 — Window Search with Rich Metadata & Filters
### Added
- **Rich WindowEntry**: search results now include workspace, monitor, floating, minimized, always-on-top, opacity
- **Visual search tags**: shows F/T (floating/tiled), M/W (monitor/workspace), [min], [AOT], opacity%
- **IPC `search-windows` command**: search managed windows from scripts with optional filters
- **Workspace filter**: `{"query":"chrome","workspace":2}` — only search workspace 2
- **Monitor filter**: `{"query":"","monitor":0}` — only search primary monitor
- **Floating filter**: `{"query":"","floating":true}` — only floating windows
- **Minimized filter**: `{"query":"","minimized":true}` — only minimized windows
- **All-managed visibility**: search includes ALL visible windows (including minimized if filtered)

### Search result format
```
F | M1 W2 [min] [AOT] 75% | Google Chrome (chrome.exe)
T | M2 W1 | Windows Terminal (WindowsTerminal.exe)
```

### IPC usage
```
echo '{"command":"search-windows","query":"chrome"}' | \\.\pipe\ultrawm-ipc
echo '{"command":"search-windows","query":"","workspace":0,"floating":true}' | \\.\pipe\ultrawm-ipc
```

## [10.14.0] - 2026-08-29 — Layout Presets with Save & IPC
### Added
- **`save_layout_preset()`**: save current tiling layout as a named preset
- **Dynamic preset creation**: create presets at runtime with kind (grid, columns, rows, master, fibonacci, fullscreen)
- **Persisted to config**: presets saved to config.toml via `config.save()`
- **IPC `list-layout-presets`**: query all available layout presets
- **IPC `create-layout-preset`**: save current layout as a named preset with kind
- Preset naming: auto-removes existing preset with same name before adding

### IPC usage
```
# List all presets
echo '{"command":"list-layout-presets"}' | \\.\pipe\ultrawm-ipc

# Save current layout as preset
echo '{"command":"create-layout-preset","name":"my-cols","kind":"columns"}' | \\.\pipe\ultrawm-ipc

# Apply preset
echo '{"command":"layout-preset","name":"my-cols"}' | \\.\pipe\ultrawm-ipc
```

## [10.13.0] - 2026-08-29 — Per-App Opacity Memory
### Added
- **Per-app opacity memory**: opacity adjustments are remembered per executable and restored on startup
- **`per_app_opacity` HashMap**: stores exe -> opacity mappings persisted to `per_app_opacity.json`
- **`remember_app_opacity()`**: saves opacity for an app when adjusted via Win+O/Shift+O
- **`apply_per_app_opacity()`**: applies remembered opacity to matching windows on creation
- **`load_per_app_opacity()`**: loads persisted per-app opacity on startup
- **`save_per_app_opacity()`**: persists opacity memory (also saved with session)

### IPC commands
- `get-app-opacity`: `{"command":"get-app-opacity","exe":"notepad.exe"}` → returns opacity
- `set-app-opacity`: `{"command":"set-app-opacity","exe":"notepad.exe","opacity":0.75}` → sets and applies
- `list-app-opacities`: returns all remembered app opacities with counts

### Behavior
- When opacity is adjusted for a window, its exe's opacity is remembered
- New windows from the same exe automatically get the remembered opacity
- Opacity < 1.0 is saved; setting to 1.0 removes the memory entry
- Per-app opacity is applied unless session already set a custom opacity

## [10.12.0] - 2026-08-29 — Floating Window Edge & Corner Snapping
### Added
- **Screen edge snapping**: floating windows snap to monitor edges when dragged nearby
- **Corner snapping**: snap to left/right half, top/bottom half, or full maximize
- **Maximize on corner drag**: dragging to all 4 edges simultaneously maximizes the window
- **Proximity threshold**: configurable `snap_edge_distance` (default 8px) for snap detection
- **Grid snapping**: positions snap to `snap_grid_size` grid for consistent alignment
- **Edge-to-edge snapping**: floating windows snap to other floating windows' edges
- **Auto-snap on move**: triggers automatically when a floating window is repositioned via EVENT_OBJECT_LOCATIONCHANGE

### Snap behaviors
- **Left edge snap**: window aligns to left edge of monitor
- **Right edge snap**: window aligns to right edge
- **Top/bottom edge snap**: window aligns to top/bottom of monitor
- **Left half snap**: drag to left edge with ~50% width → half-left layout
- **Right half snap**: drag to right edge with ~50% width → half-right layout
- **Maximize snap**: drag to all 4 edges simultaneously → full-screen maximize
- **Window-to-window snap**: edges align with other floating windows within threshold

### IPC
- No new commands needed — snapping is automatic via WinEvent LOCATIONCHANGE

## [10.11.0] - 2026-08-29 — Session Restore & Layout Persistence
### Added
- **Session restore**: windows are automatically repositioned to their saved workspace, monitor, and position on startup
- **`restore_session()` method**: matches windows by exe name and restores workspace/monitor/floating state
- **IPC `save-session` command**: manually trigger session save
- **IPC `restore-session` command**: manually trigger session restore
- **Position fallback chain**: session position → float position → current position for robust restore
- **Per-window restore**: floating size, position, opacity, sticky, maximized, always-on-top all restored

### Changed
- Session restore runs automatically after window enumeration on startup
- Z-order is applied after session restore for correct window stacking
- Session matching uses first-unmatched strategy (same exe = same app)

## [10.10.0] - 2026-08-29 — Per-Monitor Workspace Switching & Window Counts
### Added
- **Per-monitor workspace switching**: IPC `switch-workspace` supports optional `monitor` parameter for independent monitor workspace switching
- **`window_count_per_workspace()` method**: returns window counts per workspace for each monitor
- **Window counts in bar**: workspace buttons now show "name (count)" when windows exist
- **Widened workspace buttons**: expanded from 36px to 44px with 48px spacing for better readability

### IPC enhancements
- `get-workspaces` now returns per-monitor data with workspace names, window counts, and active status
- `switch-workspace` accepts optional `{"monitor": N}` parameter for targeted monitor switching
- When no monitor specified, workspace switch applies to the focused window's monitor

### Per-monitor workspace names
- `per_monitor_workspace_names: Vec<Vec<String>>` in LayoutConfig for monitor-specific workspace labels
- Falls back to global `workspace_names`, then numbered defaults if per-monitor names are empty

## [10.9.0] - 2026-08-29 — Monocle Layout Mode
### Added
- **Monocle layout mode**: focused window takes the full viewport, other windows hidden
- `toggle_monocle()` method on Platform to enable/disable monocle mode
- **Win+Z keybind**: toggle monocle mode on/off
- **IPC `toggle-monocle` command**: toggle monocle mode from scripts
- Auto-show/hide windows on focus change within monocle mode
- Full-viewport border rendering for the focused window in monocle

### Keybind
```
Win+Z — toggle monocle layout mode
```

### IPC usage
```
echo '{"command":"toggle-monocle"}' | \\.\pipe\ultrawm-ipc
```

### Behavior
- **Monocle ON**: only the focused window is visible, taking the full monitor work area
- **Monocle OFF**: normal tiling layout resumes with all windows visible
- **Focus change in monocle**: old window hides, new window shows and takes full viewport
- **Exit monocle**: all previously hidden windows are restored and tiled normally

## [10.8.0] - 2026-08-29 — GPU-Accelerated Overlay Rendering
### Added
- **Per-pixel alpha rendering**: overlays now use UpdateLayeredWindow with 32-bit ARGB bitmaps
- **Rounded corners**: Help overlay and notifications have smooth rounded corners via per-pixel alpha masking
- **Hardware-accelerated composition**: DWM compositor GPU-accelerates layered window rendering on Windows 10+
- **Smooth transparency**: per-pixel alpha enables proper anti-aliased edges and shadows

### Changed
- Help overlay uses per-pixel alpha bitmap instead of SetLayeredWindowAttributes
- Notifier renders to 32-bit ARGB bitmap with rounded corners
- Overlay rendering pipeline: content → GDI bitmap → UpdateLayeredWindow → DWM GPU composition

## [10.6.0] - 2026-08-29 — Keybind Help Overlay
### Added
- **Help overlay**: displays all available keyboard shortcuts in a semi-transparent window
- `HelpOverlay` struct with custom-drawn keybind list
- **Keybind display**: shows action name and configured key (e.g., "Focus left: Left")
- **Grouped categories**: focus/move, pan/resize, actions, UI sections
- **Esc to close**: dismiss help overlay with Escape key
- **Auto-dismiss on focus loss**: closes when clicking away
- **`toggle_help()` method** on Platform to show/hide help overlay
- **Centered positioning**: 700x500 window centered on primary monitor
- **`vk_to_string()` helper**: converts VK codes to human-readable names (Left, Right, A-Z, etc.)
- **`get_primary_monitor_size()` helper** for centering

### Keybind
```
Win+/ — toggle keybind help overlay
```

### Implementation
- HelpOverlay uses layered window with 240 alpha transparency
- Reads keybinds from Platform at display time (reflects current config)
- Custom WM_PAINT draws all keybinds organized by category
- Footer text: "Press Esc to close"
- Dark theme styling matching Catppuccin Mocha palette

### Changed
- Made `window_for_hwnd()` public on Platform for IPC access

## [10.7.0] - 2026-08-29 — Window Rules Preview
### Added
- **Window rules preview**: IPC command to list all rules matching the focused window
- `match_exe()`, `match_class()`, `match_title()` helper methods on WindowRule
- **`get-window-rules` IPC command**: returns matching rules for focused window
- Structured JSON response with rule details (float, workspace, opacity, etc.)

### Keybind
```
ipc get-window-rules — list rules matching the focused window
```

### Implementation
- IPC handler reads focused window from Platform
- Iterates through all configured rules
- Checks exe, class, and title for substring matches
- Returns JSON array of matching rules with their properties

### Changed
- Keyboard hook dispatches "/" key (0xBF) to toggle help
- Help overlay reuses launcher/search overlay pattern
- Keybinds config fully documented in help overlay

## [10.5.0] - 2026-08-29 — Window Search Overlay
### Added
- **Window search overlay**: quickly find and focus windows by title or executable name
- `WindowSearch` struct with editable text field and filtered listbox
- **Real-time filtering**: list updates as you type
- **Window entries** show title and exe name (e.g., "Visual Studio Code (Code.exe)")
- **Focus on selection**: Enter or double-click focuses the selected window
- **Dismiss on Escape**: closes the search overlay
- **Auto-dismiss on focus loss**: closes when clicking away
- **`window-search` keybind**: default key "P" (Win+P) to toggle window search
- **`show_window_search()` method** on Platform to toggle the overlay
- **`window-search` IPC command**: trigger window search from scripts

### Keybind
```
Win+P — toggle window search overlay
```

### IPC usage
```
echo '{"command":"window-search"}' | \\.\pipe\ultrawm-ipc
```

### Implementation
- WindowSearch uses layered window with 240 alpha transparency
- Populates list from `platform.windows` (visible, non-minimized only)
- Filter matches against both window title and executable name
- Uses `SetForegroundWindow` to focus selected window
- Static pointer pattern (`SEARCH_PTR`) for global access from window proc

### Changed
- Keybinds config now includes `window_search` field (default: "P")
- Keyboard hook dispatches window_search key to Platform::show_window_search
- Window search reuses launcher UI pattern (edit + listbox)

## [10.4.0] - 2026-08-29 — Improved Window Tiling Animations
### Added
- **Smooth tiling animations**: windows now animate from their current position to new tiled positions
- **Spring-based position interpolation**: uses existing SpringValue system for natural motion
- **Animation from current rect**: windows start animation from their actual window position via GetWindowRect
- **Fallback to target position**: if current rect unavailable, animation starts from target (instant placement)
- Enhanced `or_insert_with` logic in tiling to preserve animation continuity

### Changed
- Window tiling now uses actual window position as animation start point
- New windows animate from center if no previous position exists
- Workspace switches maintain animation state across tile operations
- Spring stiffness/damping from config control animation feel (default: 180.0/20.0)

### Animation behavior
- **New window**: starts from current screen position, animates to tiled cell
- **Existing window**: slides from old cell to new cell with spring physics
- **Layout change**: all windows smoothly transition to new layout
- **Workspace switch**: windows fade out/in while maintaining position animations

## [10.3.0] - 2026-08-29 — Screenshot IPC Command
### Added
- **Screenshot IPC command**: capture screen or window and save to PNG
- `Platform::take_screenshot()` method using BitBlt and GetDIBits
- Screenshot supports full screen or specific window capture
- Automatic BGRA to RGBA conversion for PNG format
- PNG encoding using existing `png` crate dependency
- **`screenshot` IPC command**: trigger screenshots from scripts/CLI
- Optional `hwnd` parameter to capture specific window
- Optional `output` parameter for custom file path (default: `ultrawm-screenshot.png`)

### IPC usage
```bash
# Full screen screenshot
echo '{"command":"screenshot","output":"fullscreen.png"}' | \\.\pipe\ultrawm-ipc

# Specific window screenshot (by HWND)
echo '{"command":"screenshot","hwnd":12345,"output":"window.png"}' | \\.\pipe\ultrawm-ipc
```

### Implementation details
- Uses `BitBlt` with `SRCCOPY | CAPTUREBLT` for screen capture
- Creates compatible bitmap and device context for rendering
- Reads bitmap bits with `GetDIBits` into RGBA buffer
- Saves PNG with 32-bit color depth using `png` crate
- Cleans up GDI objects (bitmap, DC) after capture

### Changed
- Screenshot feature builds on existing `png` crate dependency
- Output path defaults to current directory if not specified

## [10.2.0] - 2026-08-29 — Enhanced Session Restore
### Added
- **Monitor assignment per window**: session now saves which monitor each window was on
- **Window position/size restore**: saves actual window rect (x, y, w, h) for both tiled and floating windows
- **`monitor` field** in `SessionWindowState` for per-window monitor tracking
- **`x, y, w, h` fields** in `SessionWindowState` for window geometry
- **`get_window_rect()` helper** to capture current window position/size via GetWindowRect
- Session restore now places windows back on their original monitors
- Session restore preserves exact window sizes, not just cell positions

### Changed
- Session file format upgraded (version 2+)
- Tiled windows save their current rect when session is saved
- Floating windows save their actual rect, falling back to float_x/y/w/h
- Monitor index saved for each window to support multi-monitor setups
- Session restore reads monitor field to place windows correctly

### Session file format
```json
{
  "version": 2,
  "monitors": [
    {
      "grids": [...],
      "current": 0,
      "windows": [
        {
          "exe": "notepad.exe",
          "monitor": 0,
          "x": 100, "y": 200, "w": 800, "h": 600,
          "floating": false,
          ...
        }
      ]
    }
  ]
}
```

## [10.1.0] - 2026-08-29 — Notification System (Toast Popups)
### Added
- **Toast notifications** via existing Notifier infrastructure with animated fade-in/out
- **`notify` IPC command**: trigger notifications from scripts/CLI with custom messages
- **Fade-in animation**: notifications fade in over 200ms with smooth alpha blending
- **Fade-out animation**: notifications fade out over 500ms after 3-second display duration
- **Auto-positioning**: notifications appear at bottom-right of primary monitor
- **Notification tick**: `tick_notifier()` called every frame in animation loop
- **Layered window rendering**: uses UpdateLayeredWindow for smooth transparency

### IPC usage
```
echo '{"command":"notify","message":"Hello from UltraWM"}' | \\.\pipe\ultrawm-ipc
```

### Config
```toml
# Notifier created automatically if enabled in config
# Position: bottom-right corner with 20px margin
# Size: 300x40px
# Colors: bg=#1E1E2E, fg=#CDD6F4 (Catppuccin)
# Duration: 3 seconds display + 500ms fade out
```

### Changed
- Fixed fade-out timing calculation to use actual elapsed time instead of subsec_millis
- Notifier already existed but is now fully wired to IPC and animation loop
- All `notify` calls will display toast popup in bottom-right corner

## [10.0.0] - 2026-08-29 — System Info Bar (CPU, Memory, Clock)
### Added
- **CPU usage display** in bar: shows real-time CPU percentage with 1-second refresh
- **Memory usage display** in bar: shows RAM usage percentage with 1-second refresh
- **Live clock** in bar: updates every frame with current time (HH:MM format)
- `show_cpu` and `show_memory` config options in `BarConfig` (both enabled by default)
- `BarState::cpu` and `BarState::memory` fields for system metrics
- `AppBar::set_cpu(usage)` and `AppBar::set_memory(usage)` methods
- `Platform::get_cpu_usage()` using `GetSystemTimes` API
- `Platform::get_memory_usage()` using `GlobalMemoryStatusEx` API
- `Platform::update_bar_system_info()` updates clock, CPU, and memory at configurable intervals
- CPU/memory updates run every 60 frames (~1 second) to reduce overhead
- Right-aligned system info indicators in bar (after workspace/title, before battery/volume)

### Config example
```toml
[bar]
show_cpu = true      # show CPU usage percentage
show_memory = true   # show memory usage percentage
show_clock = true    # show live clock
```

### Bar layout (right-aligned)
```
[Workspaces] [Title] [CPU: 45%] [MEM: 62%] [14:32] [VOL: 80%] [BAT: 95%]
```

### Changed
- Bar now displays system metrics by default
- Clock updates every frame for smooth time display
- CPU/memory update frequency controlled by `config_reload_counter`

## [9.9.0] - 2026-08-29 — Animated Workspace Switching
### Added
- **Spring-based workspace fade animation** replaces linear fade with smooth physics
- `ws_fade_anim: SpringValue` on Platform for animated workspace transitions
- `ws_animating: bool` tracks active workspace animation state
- `switch_workspace(ws)` triggers spring animation: fades to black, switches workspace, fades back
- Spring animation uses stiffness/damping/mass for natural motion (overshoot & settle)
- Animation fades border overlay alpha from 1.0→0.0→1.0 with spring physics
- **`switch-workspace` IPC command**: trigger animated workspace switch by number
- IPC command validates workspace number, checks current workspace, and triggers animation

### Config
```toml
# Workspace switch animation uses spring physics (hardcoded tuning)
# stiffness: 180, damping: 20, mass: 1.0 (same as window animations)
```

### IPC usage
```
echo '{"command":"switch-workspace","workspace":2}' | \\.\pipe\ultrawm-ipc
```

### Changed
- Workspace switching animation is now physics-based instead of linear step
- Animation triggers from IPC, bar clicks, and keyboard shortcuts

## [9.8.0] - 2026-08-29 — Minimize-to-Tray Support
### Added
- **Minimize-to-tray** support: windows can minimize to system tray instead of taskbar
- `tray_windows: HashMap<u64, HWND>` on Platform tracks tray-hidden windows
- `tray_hwnd: Option<HWND>` for the hidden tray message-only window
- `minimize_to_tray(wid)` — hides window and creates tray icon with exe name
- `restore_from_tray(wid)` — restores a specific tray window by clicking its icon
- `restore_all_tray()` — restores all tray windows (restarts WM recovery)
- `ensure_tray_icon(hwnd, exe)` — creates/updates tray icon with tooltip
- **`minimize-to-tray` IPC command**: minimize focused window to tray
- **`restore-from-tray` IPC command**: restore window by pid
- **`restore-all-tray` IPC command**: restore all tray windows
- `tray_wnd_proc` window procedure handles tray icon callbacks (WM_LBUTTONUP/WM_RBUTTONUP)
- Tray icons use `Shell_NotifyIconW` with `NOTIFYICONDATAW` and round icon
- Tray icons display first letter of exe name centered in colored icon
- Windows restore with original style/flags after tray restoration

### IPC usage
```
echo '{"command":"minimize-to-tray"}' | \\.\pipe\ultrawm-ipc
echo '{"command":"restore-from-tray","wid":12345}' | \\.\pipe\ultrawm-ipc
echo '{"command":"restore-all-tray"}' | \\.\pipe\ultrawm-ipc
```

## [8.2.0] - 2026-08-29 — Window Snap Mode (Win+G)
### Added
- Win+G toggles snap mode — focused window snaps to screen positions with arrow/number keys
- Arrow keys snap: Left=left half, Right=right half, Up=top half, Down=bottom half
- Number keys 1-9 snap to quadrants and center: 1=TL, 2=TR, 3=BL, 4=BR, 5=center, 6-9=edges
- 0 snaps to fullscreen
- Esc exits snap mode
- Snap mode shows "SNAP" indicator in bar (cyan text) when active
- Focused window border flashes cyan during snap mode and on snap action
- `snap_mode`, `snap_flash` fields on Platform
- `toggle_snap_mode()`, `exit_snap_mode()`, `snap_window(pos)` methods
- `edge_tile_window` reuses existing positioning logic with min/max rule constraints

### Changed
- Keyboard handler in `keyboard.rs` dispatches snap keys when `platform.snap_mode` is true

## [8.3.0] - 2026-08-29 — Floating Window Grid/Edge Snapping
### Added
- Floating windows snap to configurable grid (default 10px) when dragged
- Edge snapping: floating windows snap to other floating windows' edges within 8px distance
- `snap_grid_size` and `snap_edge_distance` config options in LayoutConfig
- `snap_to_grid(x, y, w, h)` rounds position/size to nearest grid multiples
- `snap_floating_window(hwnd)` applies grid + edge snapping to a floating window
- `EVENT_OBJECT_LOCATIONCHANGE` WinEvent hook triggers snapping on floating window drag
- Second SetWinEventHook call for location change events (separate from foreground/create hook)
- Snap positions clamped to monitor bounds

### Changed
- Floating windows auto-snap during drag without any keybinding required
- Grid size and edge distance configurable via config.toml

## [8.4.0] - 2026-08-29 — Dynamic Workspace Creation and Deletion
### Added
- `add_workspace()` — adds a new empty workspace to all monitors, switches to it
- `remove_workspace()` — removes current workspace if empty, switches to adjacent
- Win+Shift+N creates a new workspace (max 10)
- Win+Shift+W removes current workspace (min 1)
- Workspace count limited to 1-10, notification on attempt to exceed limits
- Empty workspace check prevents accidental removal of workspaces with windows

### Changed
- `set_workspace_count` reused internally for add/remove (handles grid allocation/migration)

## [8.5.0] - 2026-08-29 — Per-Monitor DPI Scaling
### Added
- Per-monitor DPI detection via `GetDpiForMonitor` in `enumerate_monitors`
- `dpi` and `scale_factor` fields on `MonitorInfo` populated from system
- `get_monitor_dpi(hmonitor)` returns DPI for a specific monitor
- `effective_width()` and `effective_height()` apply scale factor to dimensions
- `WM_DPICHANGED` handler re-enumerates monitors on DPI change
- `on_dpi_changed()` rescales floating window positions by DPI ratio
- `get-dpi` IPC command returns per-monitor DPI and scale factor info
- DPI info included in `get-config` IPC response
- DPI-aware snapping that uses effective pixel coordinates

### Changed
- App uses `DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2` for correct per-monitor rendering
- Floating windows auto-rescale when dragged between monitors with different DPI

## [8.6.0] - 2026-08-29 — IPC Screenshot Capture
### Added
- `screenshot` IPC command captures focused window to PNG
- Screenshots saved to `%Pictures%/UltraWM/screenshot_YYYYMMDD_HHMMSS.png`
- Uses BitBlt + GetDIBits for pixel capture, png crate for encoding
- BGRA to RGBA color conversion for correct PNG output
- Returns file path and success status in IPC response
- Win32_Graphics_Gdi and Win32_Graphics_Printing features added to Cargo.toml

### Changed
- `get-dpi` and `screenshot` return data directly from IPC thread using PLATFORM_PTR

## [9.0.0] - 2026-08-29 — Configurable Keybinds via config.toml
### Added
- All Win+key bindings now configurable in config.toml `[keybinds]` section
- `focus_left/right/up/down` — focus movement keys (default: arrows)
- `move_left/right/up/down` — window movement keys (default: arrows + Shift)
- `pan_left/right/up/down` — camera pan keys (default: arrows + Ctrl)
- `grow_width/shrink_width` — width resize keys (default: OemMinus/Oemplus)
- `grow_height/shrink_height` — height resize keys (default: OemMinus+Shift/Oemplus+Shift)
- `fullscreen` — fullscreen toggle key (default: F)
- `close` — close window key (default: C)
- `float` — toggle float key (same as close + Shift)
- `sticky` — toggle sticky key (default: Y)
- `theme_next/theme_prev/theme_picker` — theme control keys (default: T/G)
- `launcher` — app launcher key (default: Space)
- String values parsed to VK codes: names ("left", "space", "escape") and chars ("F", "C", "G")
- Special key names: OemMinus, OemPlus, OemComma, OemPeriod, Tab, Return, Back, etc.
- `ParsedKeybinds` struct stores all VK codes for fast comparison in keyboard hook
- `keybinds` field on Platform for runtime access
- Keybinds re-parsed automatically on config hot-reload

### Changed
- Keyboard hook uses `platform.keybinds` instead of hardcoded VK constants
- Modifier keys (Shift/Ctrl/Alt) still handled separately — only base key is configurable

## [9.1.0] - 2026-08-29 — IPC Config Write Commands Persist to Disk
### Added
- `Config::save()` writes config back to config.toml with pretty TOML formatting
- `save-config` IPC command persists current config to disk
- Config-modifying IPC commands now auto-persist:
  - `set-gap <value>` — saves after changing gap
  - `set-corner-radius <value>` — saves after changing corner radius
  - `set-border-width <value>` — saves after changing border width
- `adjust_gap(delta)` also persists on every gap change
- All config structs derive `Serialize` for TOML output

### Changed
- IPC `reload-config` re-parses keybinds after loading new config
- Config save creates parent directory if it doesn't exist
- Failed saves are logged as warnings but don't crash the WM

## [9.7.0] - 2026-08-29 — Configurable Snap Positions (Custom Grid Layouts)
### Added
- **Custom snap layouts** in config: define named layouts with custom column widths and row heights
- `SnapLayout` struct with `name`, `widths`, `heights` fields
- `snap_layouts: Vec<SnapLayout>` field on `LayoutConfig`
- `GridState::custom_widths` and `custom_heights` override equal-cell rendering
- `GridState::custom_cell_size()` returns per-cell size from custom layout or default
- `GridState::snap_layout_custom()` places windows with custom dimensions
- `Platform::apply_custom_layout(name)` applies named custom layout
- **`snap-custom` IPC command**: apply custom layout by name

### Config example
```toml
[[layout.snap_layouts]]
name = "sidebar"
widths = [400, 800]  # narrow left pane + wide right pane

[[layout.snap_layouts]]
name = "golden"
widths = [600, 900]  # golden ratio split

[[layout.snap_layouts]]
name = "triple"
widths = [400, 400, 400]  # three equal columns
```

### IPC usage
```
echo '{"command":"snap-custom","name":"sidebar"}' | \\.\pipe\ultrawm-ipc
```

## [9.6.0] - 2026-08-29 — Multi-Step Resize with Visual Size Guide
### Added
- **`resize_step_px`** config: pixel-based resize when > 0 (0 = preset steps)
- Pixel resize uses `SetWindowPos` with configurable step size (e.g. 20px per keypress)
- **Visual size guide** in bar: centered box showing WxH dimensions during resize
- `resize_flash` counter on BarState shows size for ~1 second (60 frames)
- `AppBar::show_resize_size(text)` displays dimensions in a bordered box
- `Platform::adjust_window_size()` helper for pixel-based resize via SetWindowPos
- `Platform::show_resize_size()` gets current rect and displays in bar
- Size indicator uses fg color text on bg color box with border

### Config example
```toml
[layout]
resize_step_px = 20  # 20px per resize step (0 = preset steps)
```

## [9.5.0] - 2026-08-29 — Session Auto-Save Interval Config
### Added
- **`session_auto_save_interval`** config field: auto-save session every N seconds (0 = disabled)
- `last_session_save: Instant` field on Platform tracks last auto-save time
- Event loop checks interval each frame and saves when elapsed
- Replaces hardcoded ~5 second periodic save with configurable interval
- Setting to 0 disables auto-save (manual saves still work on workspace switch, window close, etc.)

### Example config
```toml
[layout]
session_auto_save_interval = 30  # save every 30 seconds
```

## [9.4.0] - 2026-08-29 — Layout Presets: Named Window Arrangements
### Added
- **Layout presets** in config: named window arrangement definitions
- `LayoutPreset` struct with `name`, `kind`, `cols`, `rows` fields
- `layout_presets: Vec<LayoutPreset>` field on `LayoutConfig`
- Six preset kinds: `grid`, `columns`, `rows`, `master`, `fibonacci`, `fullscreen`
- `Platform::apply_layout_preset(name)` applies named preset to visible windows
- **`layout-preset` IPC command**: apply preset by name via JSON

### Config example
```toml
[[layout.layout_presets]]
name = "coding"
kind = "master"
cols = 1
rows = 2

[[layout.layout_presets]]
name = "quad"
kind = "grid"
cols = 2
rows = 2

[[layout.layout_presets]]
name = "full"
kind = "fullscreen"
```

### IPC usage
```
echo '{"command":"layout-preset","name":"coding"}' | \\.\pipe\ultrawm-ipc
```

## [9.3.0] - 2026-08-29 — Window Rule Import/Export via IPC
### Added
- **`export-rules` IPC command**: exports all window rules as JSON array
- **`import-rules` IPC command**: imports rules from JSON array, validates, and saves to config
- Full rule fields exported: match, float, workspace, width, height, max/min sizes, float position, opacity, sticky
- Import skips rules with empty `match` fields and persists to config.toml
- `Platform::import_rules_from_json()` method for batch rule import with validation
- `Platform::apply_rule_from_json()` made public for IPC use

### Example IPC usage
```
# Export rules
echo {"command":"export-rules"} | \\.\pipe\ultrawm-ipc

# Import rules from JSON array
echo '{"command":"import-rules","rules":[{"match":"class:Notepad","float":true,"opacity":0.8}]}' | \\.\pipe\ultrawm-ipc
```

## [9.2.0] - 2026-08-29 — Live Config Reload Indicator in Bar
### Added
- Config reload triggers a brief green flash animation in the bar (~0.5s)
- `BarState::reload_flash` field tracks flash animation frames
- `AppBar::trigger_reload_flash()` method to start flash from platform
- Green tint overlay drawn in `WM_PAINT` with alpha decay over 30 frames
- Config hot-reload (`reload_if_changed`) calls `bar.trigger_reload_flash()` after loading new config

### Changed
- `Config::reload_if_changed` triggers visual reload notification
- Bar WM_PAINT handler decays `reload_flash` counter each frame
- Reload flash uses green color (RGB 0,128,0) for positive feedback

## [Unreleased]
### Planned
- Window rule import/export via IPC (JSON config backup)
- Window minimize-to-tray support
- Multi-step window resize with visual guides
- Configurable snap positions (custom grid layouts)
- GPU-accelerated screenshot via Windows.Graphics.Capture
- Session auto-save interval config

## [9.1.0] - 2026-08-29 — IPC Config Write Commands Persist to Disk
### Added
- `opacity_anim: HashMap<u64, SpringValue>` on Platform for per-window opacity springs
- `on_focus_changed` creates spring animation for old focused window (target: 70% opacity) and new focused window (target: 100% opacity)
- `tile_all_windows` updates opacity springs each frame using `step(dt)` and applies via `SetLayeredWindowAttributes`
- `retain` cleanup removes settled springs to avoid unnecessary work

### Changed
- Window opacity transitions smoothly with spring physics (stiffness: 200, damping: 25) instead of instant changes

## [8.0.0] - 2026-08-29 — Master-Stack Layout with Variable Cell Sizes
### Added
- `LayoutMode` enum: `Grid` (equal cells) and `Master` (50/50 split)
- `layout_mode` field on `GridState` controls cell rendering
- `cell_rect` handles `Master` mode: column 0 = 50% width (master), column 1 = 50% width divided among stack windows
- `layout-master` IPC command sets Master mode and arranges windows: first window as master, rest stacked
- `layout-columns`/`layout-rows`/`layout-fibonacci` reset to Grid mode
- `snap_layout` also resets to Grid mode

### Changed
- Grid rendering uses variable cell sizes in Master mode
- Master window gets full viewport height, 50% width
- Stack windows share 50% width, equal height division

## [7.7.0] - 2026-08-29 — Runtime Window Opacity Control
### Added
- `set-window-opacity <value>` — set focused window opacity (0.0-1.0)
- `increase-opacity` — increase focused window opacity by 0.05
- `decrease-opacity` — decrease focused window opacity by 0.05
- `adjust_opacity(delta)` method for incremental opacity changes
- Per-window opacity persists in WindowInfo and session save/restore

### Changed
- `set-window-opacity` delegates to existing `set_opacity` method
- Opacity changes apply immediately via `SetLayeredWindowAttributes` with LWA_ALPHA

## [7.6.0] - 2026-08-29 — Layout Presets
### Added
- `layout-columns N` — arrange windows into N equal-width columns
- `layout-rows N` — arrange windows into N equal-height rows
- `layout-master` — first window in left column, rest stacked in right column
- `layout-fibonacci` — columns based on Fibonacci sequence (1,1,2,3,5,8...)
- `collect_visible_wids()` helper for layout preset methods
- Layout presets save session after rearrangement

### Changed
- IPC commands for layout presets use prefix matching: `layout-columns 3`
- Layout methods sort windows by Z-order before placement

## [7.5.0] - 2026-08-29 — Per-Window Border Colors
### Added
- Each window gets a unique border color based on its exe name hash
- Focused windows display full border color
- Unfocused windows display dimmed (50%) border color
- Swap flash blends white with per-window color instead of global accent
- `border_color` field added to `WindowInfo`
- `dim_color()` utility function for dimming border colors
- Border color set during `manage_window` using `exe_hash_color()`

### Changed
- Border rendering uses per-window `border_color` instead of global accent/inactive
- Title background uses per-window color (focused = full, unfocused = dimmed)

## [7.4.0] - 2026-08-29 — Enhanced Swap Animation
### Added
- Swap flash duration increased from 20 to 35 frames for more visible animation
- Swap notification toast ("Swapped") appears after window swap
- Flash blend is brighter — starts near-white and fades to accent color
- Swap and drag-move both use 35-frame flash for consistent animation

### Changed
- `swap_flash` timers now count down from 35 instead of 20
- Flash alpha uses `min(35)` instead of `min(20)` for longer fade
- Blend factor `alpha * 0.8 + 0.2` ensures initial flash is near-white

## [7.3.0] - 2026-08-29 — Bar Click-to-Switch Workspace
### Added
- Clicking on a workspace indicator in the bar switches to that workspace
- Bar WM_LBUTTONDOWN handler detects which workspace pill was clicked
- Custom message `WM_BAR_WORKSPACE_CLICK` posted to platform message loop
- Platform message loop handles bar click and calls `switch_workspace(idx)`

### Changed
- Workspace indicators are now interactive — click to navigate between workspaces
- Bar uses `PostMessageW` to communicate clicks to the platform

## [7.2.0] - 2026-08-29 — Session Save/Restore (Multi-Monitor)
### Added
- Session now saves all monitors, all workspaces, and floating windows
- Floating windows saved with float_x/y/w/h positions
- Session restored across all monitors (was only monitor 0)
- Session restored across all workspaces (was only current workspace)
- Session saved automatically on: window close, workspace switch, float toggle, app exit
- Backward-compatible with old session.json format (version 1)

### Changed
- `SessionState` now includes per-monitor `MonitorSessionState` with per-workspace `GridSessionState`
- `SessionWindowState` includes float_x/y/w/h for floating window restoration
- `save_session` iterates all monitors and workspaces
- Session restore searches all monitors/workspaces by exe name match

## [7.1.0] - 2026-08-29 — Window Rules Engine
### Added
- Window rules are now fully applied at window creation time
- Rule-based floating: windows matching rules are auto-floated and positioned
- Rule-based workspace assignment with correct workspace count validation
- Rule-based size constraints: `width`, `height`, `max_width`, `max_height`, `min_width`, `min_height`
- Rule-based float position: `float_x`, `float_y`, `float_w`, `float_h` are enforced
- Rule-based opacity and sticky flags applied on window creation
- `apply_rules()` now checks `workspace_count` instead of hardcoded 4
- `width`/`height` rules now set `float_w`/`float_h` for floating windows
- Rule-floated windows are removed from grid and positioned via `SetWindowPos`
- IPC `add-rule` accepts `width` and `height` fields
- IPC `list-rules` includes `width` and `height` in response

### Fixed
- Workspace assignment in rules now respects configurable workspace_count (was hardcoded to 4)
- Floating windows from rules were placed on grid but never positioned — now correctly floated

## [7.0.0] - 2026-08-29 — Multi-Monitor Workspace Awareness
### Added
- Bar workspace indicators update when focus moves to a different monitor
- `on_focus_changed()` detects monitor change and updates bar workspace names
- Bar shows workspace indicators for the monitor containing the focused window
- Compares old and new monitor of focused window to trigger bar update

### Changed
- Workspace indicators in bar now reflect the focused window's monitor
- Focus change triggers both flash animation and workspace indicator update

## [6.5.0] - 2026-08-29 — Bar Position Configuration
### Added
- Bar can be positioned at top or bottom of screen via `bar.position` config
- Default position: "top"
- Bar window repositioned using `SetWindowPos` after creation
- `SetWindowPos` with `SWP_FRAMECHANGED` flag ensures proper redraw

### Example config
```toml
[bar]
position = "bottom"
```

## [6.4.0] - 2026-08-29 — Config Validation
### Added
- `Config::validate()` method checks config on load for common errors
- Validates `workspace_count` is 1-10
- Validates `workspace_names` length <= `workspace_count`
- Validates `monitor_layouts` length <= 8 (max monitors supported)
- Validates each rule has non-empty `match` field
- Validates rule `workspace` field < `workspace_count`
- Validation called automatically after `Config::load()` and `reload_if_changed()`

### Changed
- Config load fails with descriptive error message if validation fails
- Rule validation reports rule index for easy debugging

## [6.3.0] - 2026-08-29 — IPC Query Enhancements
### Added
- `get-state` now returns: managed_windows count, current theme name, per-monitor workspace info
- Per-monitor workspace data: monitor index, current workspace, total count
- `managed_windows` field in get-state response
- `theme` field in get-state response with current theme name

### Changed
- `get-state` response enriched with 4 new fields (managed_windows, theme, workspaces array)
- Workspace info includes monitor index, current workspace number, total workspace count

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
