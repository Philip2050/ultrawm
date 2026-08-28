# Changelog

All notable changes to UltraWM will be documented in this file.

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
