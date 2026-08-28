use serde::Deserialize;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub layout: LayoutConfig,
    pub keybinds: KeybindsConfig,
    pub theme: ThemeConfig,
    pub bar: BarConfig,
    pub launcher: LauncherConfig,
    pub rules: Vec<WindowRule>,
    #[serde(skip)]
    pub last_modified: Option<SystemTime>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WindowRule {
    #[serde(alias = "match")]
    pub match_: String,
    pub float: Option<bool>,
    pub workspace: Option<usize>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutConfig {
    pub gaps: u32,
    pub peek_x: i32,
    pub peek_y: i32,
    pub center_focused: bool,
    pub focus_follows_mouse: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KeybindsConfig {
    pub mod_key: String,  // "win" or "alt"
    pub focus_left: String,
    pub focus_right: String,
    pub focus_up: String,
    pub focus_down: String,
    pub move_left: String,
    pub move_right: String,
    pub move_up: String,
    pub move_down: String,
    pub pan_left: String,
    pub pan_right: String,
    pub pan_up: String,
    pub pan_down: String,
    pub grow_width: String,
    pub shrink_width: String,
    pub grow_height: String,
    pub shrink_height: String,
    pub fullscreen: String,
    pub close: String,
    pub float: String,
    pub theme_next: String,
    pub theme_picker: String,
    pub launcher: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThemeConfig {
    pub default: String,
    pub cycle_hotkey: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BarConfig {
    pub enabled: bool,
    pub height: u32,
    pub position: String,
    pub transparency: f32,
    pub show_workspaces: bool,
    pub show_clock: bool,
    pub show_volume: bool,
    pub show_battery: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LauncherConfig {
    pub enabled: bool,
    pub hotkey: String,
    pub show_recent: bool,
    pub fuzzy_search: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            layout: LayoutConfig {
                gaps: 8,
                peek_x: 80,
                peek_y: 40,
                center_focused: false,
                focus_follows_mouse: false,
            },
            keybinds: KeybindsConfig {
                mod_key: "win".into(),
                focus_left: "Left".into(),
                focus_right: "Right".into(),
                focus_up: "Up".into(),
                focus_down: "Down".into(),
                move_left: "Left".into(),
                move_right: "Right".into(),
                move_up: "Up".into(),
                move_down: "Down".into(),
                pan_left: "Left".into(),
                pan_right: "Right".into(),
                pan_up: "Up".into(),
                pan_down: "Down".into(),
                grow_width: "OemMinus".into(),
                shrink_width: "Oemplus".into(),
                grow_height: "OemMinus".into(),
                shrink_height: "Oemplus".into(),
                fullscreen: "F".into(),
                close: "C".into(),
                float: "C".into(),
                theme_next: "T".into(),
                theme_picker: "Space".into(),
                launcher: "Space".into(),
            },
            theme: ThemeConfig {
                default: "catppuccin-mocha".into(),
                cycle_hotkey: "Win+T".into(),
            },
            bar: BarConfig {
                enabled: true,
                height: 40,
                position: "top".into(),
                transparency: 0.85,
                show_workspaces: true,
                show_clock: true,
                show_volume: true,
                show_battery: true,
            },
            launcher: LauncherConfig {
                enabled: true,
                hotkey: "Win+Space".into(),
                show_recent: true,
                fuzzy_search: true,
            },
            rules: Vec::new(),
            last_modified: None,
        }
    }
}

impl Config {
    /// Load config from XDG-style path, falling back to defaults.
    pub fn load() -> anyhow::Result<Self> {
        let path = config_path();
        if path.exists() {
            let contents = std::fs::read_to_string(&path)?;
            let mut config: Config = toml::from_str(&contents)?;
            config.last_modified = std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .ok();
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Reload if the config file has been modified since last load.
    /// Returns Some(new_config) if reloaded, None if unchanged.
    pub fn reload_if_changed(&self) -> anyhow::Result<Option<Self>> {
        let path = config_path();
        if !path.exists() {
            return Ok(None);
        }

        let modified = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .unwrap_or(UNIX_EPOCH);

        // Only reload if file actually changed
        if let Some(ref last) = self.last_modified {
            if modified <= *last {
                return Ok(None);
            }
        }

        let contents = std::fs::read_to_string(&path)?;
        let mut new_config: Config = toml::from_str(&contents)?;
        new_config.last_modified = Some(modified);
        Ok(Some(new_config))
    }
}

fn config_path() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("ultrawm/config.toml")
    } else {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".config/ultrawm/config.toml")
    }
}
