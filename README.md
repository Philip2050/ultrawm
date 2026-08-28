# UltraWM — Ultimate Tiling Window Manager for Windows

An **ultimate tiling window manager for Windows 11** that combines:

- **hyprscroll2d**-style infinite 2D scrolling lattice (windows above/below/left/right)
- **Niri**-style disciplined gaps, borders, focus-rings, and dynamic workspaces
- **Hyprland**-grade visual polish: spring animations, rounded corners, blur
- **Omarchy**-style theme switching: 5 built-in themes (catppuccin-mocha, tokyo-night, gruvbox-dark, nord, rose-pine) with live wallpaper/accent/dark-mode/cursor switching
- **komorebi/GlazeWM/LeopardWM**-proven Win32 mechanics: SetWindowPos, DWM cloaking, WH_KEYBOARD_LL, WinEvent hooks

## Status

**Phase 1 MVP** — compiling and running. Core features implemented:
- **Window tab stacking**: group windows in a cell with tab switching
- 2D lattice layout engine with camera panning and collision-swapping focus
- **Bidirectional tiling**: split cells horizontally or vertically with adjustable ratios
- Multi-monitor workspaces (each monitor has independent workspace set)
- Window enumeration and tiling via SetWindowPos
- Theme engine with 5 built-in themes + JSON theme support
- Low-level keyboard hook for Win+key shortcuts
- Config system (TOML) with defaults and hot-reload
- Spring animation primitives
- Doctor diagnostics
- IPC named pipe with **JSON query/response** protocol for external control
- Session save/restore
- App launcher with search
- Theme picker UI
- Scratchpad windows
- Per-app rules (float, workspace assignment)
- Focus-follows-mouse (optional)
- Bar with workspace indicators, title, and clock
- DWM blur on windows
- Swap flash animation: white flash on windows during collision swap
- Touchpad gesture support (pan, pinch, two-finger tap)
- Shell replacement support

## Keybindings

| Keys | Action |
|---|---|
| `Win + ←/→/↑/↓` | Focus window (or pan camera) |
| `Win + Shift + ←/→/↑/↓` | Move window with collision swapping (visual flash) |
| `Win + Ctrl + ←/→/↑/↓` | Pan camera |
| `Win + 1/2/3/4` | Switch workspace |
| `Win + T` | Next theme |
| `Win + G` | Theme picker |
| `Win + Space` | App launcher |
| `Win + F` | Fullscreen toggle |
| `Win + C` | Close window |
| `Win + Shift + C` | Float/unfloat |
| `Win + S` | Scratchpad toggle |
| `Win + W` | Overview mode |
| `Win + Esc` | Exit overview |
| `Win + -` | Shrink width |
| `Win + =` | Grow width |
| `Win + Shift + -` | Shrink height |
| `Win + Shift + =` | Grow height |
| `Win + Alt + H` | Split focused cell horizontally |
| `Win + Alt + V` | Split focused cell vertically |
| `Win + Alt + U` | Unsplit focused cell |
| `Win + Alt + T` | Tab focused window with neighbor |
| `Win + Alt + Shift + T` | Untab focused cell |

## Touchpad Gestures

| Gesture | Action |
|---|---|
| Single-finger swipe | Move focus |
| Two-finger drag | Pan camera |
| Pinch | Resize window width |
| Two-finger tap | Toggle fullscreen |

## Build

```powershell
cd ultrawm
cargo build
cargo run -- start       # start daemon
cargo run -- doctor      # diagnostics
cargo run -- --list-themes
cargo run -- --show-theme
cargo run -- --theme     # cycle theme
```

## Configuration

`%USERPROFILE%\.config\ultrawm\config.toml` — gaps, peeks, cell sizes, ignored windows, autostart.

Example config.toml:
```toml
[layout]
gaps = 8
peek_x = 80
peek_y = 40
center_focused = false
focus_follows_mouse = false

[keybinds]
mod_key = "win"
focus_left = "Left"
focus_right = "Right"
# ... etc

[theme]
default = "catppuccin-mocha"

[bar]
enabled = true
height = 40
position = "top"
transparency = 0.85

[launcher]
enabled = true
hotkey = "Win+Space"

[[rules]]
match = "notepad"
float = false

[[rules]]
match = "calculator"
workspace = 2
```

## Themes

Themes in `%USERPROFILE%\.config\ultrawm\themes\` (JSON). Ships with 5 built-in themes. JSON schema matches the `Theme` struct.

## IPC Commands

Via named pipe `\\.\pipe\ultrawm-ipc`. Supports both plain text (legacy) and JSON with responses.

**Plain text** (backward compatible):
```
echo next-theme > \\.\pipe\ultrawm-ipc
```

**JSON with response**:
```json
{"command": "list-themes"}
{"command": "get-state"}
{"command": "focus-left"}
{"command": "split-horizontal"}
```

Available commands:
- `next-theme`, `prev-theme`
- `focus-left`, `focus-right`, `focus-up`, `focus-down`, `focus-next`, `focus-prev`
- `pan-left`, `pan-right`, `pan-up`, `pan-down`
- `grow-width`, `shrink-width`, `grow-height`, `shrink-height`
- `close`, `float`, `unfloat`
- `split-horizontal`, `split-vertical`, `unsplit`
- `launcher`, `overview`, `scratchpad`, `fullscreen`
- `quit`
- **Queries**: `get-state`, `list-themes`, `get-windows`

## Architecture

```
src/
├── main.rs             # CLI entry point
├── config.rs           # TOML config parsing
├── session.rs          # Session save/restore
├── anim/
│   └── mod.rs          # Spring physics primitives
├── layout/
│   ├── mod.rs          # 2D lattice engine (Cell, GridState)
│   └── workspace.rs    # Workspace state holder
├── platform/
│   ├── mod.rs          # Win32 platform (hooks, positioning, tiling)
│   ├── window.rs       # Window enumeration
│   ├── keyboard.rs     # WH_KEYBOARD_LL hook
│   ├── bar.rs          # Top bar with workspaces/clock
│   ├── launcher.rs     # App launcher
│   ├── theme_picker.rs # Theme picker UI
│   ├── blur.rs         # DWM blur
│   ├── scratchpad.rs   # Scratchpad windows
│   └── gesture.rs      # Gesture receiver (placeholder)
├── theme/
│   └── mod.rs          # Theme engine + 5 built-in themes
└── ipc/
    └── mod.rs          # Named pipe IPC server
```

## License

MIT
