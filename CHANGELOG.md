# Changelog

All notable changes to UltraWM will be documented in this file.

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
