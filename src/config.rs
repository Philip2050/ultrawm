use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WindowRule {
    #[serde(alias = "match")]
    pub match_: String,
    /// Match type: "exe", "class", "title", or "any" (default: exe)
    pub match_type: Option<String>,
    /// Rule priority (higher = applied later, overrides lower)
    pub priority: Option<i32>,
    pub float: Option<bool>,
    pub workspace: Option<usize>,
    pub monitor: Option<usize>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub float_x: Option<i32>,
    pub float_y: Option<i32>,
    pub float_w: Option<u32>,
    pub float_h: Option<u32>,
    pub opacity: Option<f32>,
    pub sticky: Option<bool>,
    pub always_on_top: Option<bool>,
    pub border_color: Option<u32>,
    pub fullscreen: Option<bool>,
    pub corner_radius: Option<u32>,
    pub border_width: Option<u32>,
}

impl WindowRule {
    pub fn match_exe(&self, exe: &str) -> bool {
        self.matches_any(exe, "", "")
    }

    pub fn match_class(&self, class: &str) -> bool {
        self.matches_any("", class, "")
    }

    pub fn match_title(&self, title: &str) -> bool {
        self.matches_any("", "", title)
    }

    pub fn matches_any(&self, exe: &str, class: &str, title: &str) -> bool {
        if self.match_.is_empty() { return false; }
        let mtype = self.match_type.as_deref().unwrap_or("exe");
        match mtype {
            "exe" => exe.contains(&self.match_),
            "class" => class.contains(&self.match_),
            "title" => title.contains(&self.match_),
            "any" => exe.contains(&self.match_) || class.contains(&self.match_) || title.contains(&self.match_),
            _ => exe.contains(&self.match_),
        }
    }

    pub fn priority(&self) -> i32 {
        self.priority.unwrap_or(0)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LayoutConfig {
    pub gaps: u32,
    pub inner_padding: u32,
    pub outer_padding: u32,
    pub border_width: u32,
    pub peek_x: i32,
    pub peek_y: i32,
    pub center_focused: bool,
    pub focus_follows_mouse: bool,
    pub corner_radius: u32,
    pub rounded_corners: bool,
    pub dwm_shadows: bool,
    pub window_opacity: f32,
    pub spring_stiffness: f32,
    pub spring_damping: f32,
    pub workspace_count: usize,
    pub workspace_names: Vec<String>,
    pub per_monitor_workspace_names: Vec<Vec<String>>, // per-monitor workspace names
    pub default_float_width: u32,
    pub default_float_height: u32,
    pub default_split_dir: String,
    pub auto_split: bool,
    pub monitor_layouts: Vec<MonitorLayout>,
    pub snap_grid_size: u32,
    pub snap_edge_distance: u32,
    pub layout_presets: Vec<LayoutPreset>,
    pub session_auto_save_interval: u32,
    pub resize_step_px: u32,
    pub snap_layouts: Vec<SnapLayout>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LayoutPreset {
    pub name: String,
    pub kind: String,
    pub cols: Option<u32>,
    pub rows: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SnapLayout {
    pub name: String,
    pub widths: Vec<u32>,
    pub heights: Vec<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitorLayout {
    pub gaps: Option<u32>,
    pub inner_padding: Option<u32>,
    pub outer_padding: Option<u32>,
    pub border_width: Option<u32>,
    pub corner_radius: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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
    pub sticky: String,
    pub theme_next: String,
    pub theme_prev: String,
    pub theme_picker: String,
    pub launcher: String,
    pub window_search: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThemeConfig {
    pub default: String,
    pub cycle_hotkey: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BarConfig {
    pub enabled: bool,
    pub height: u32,
    pub position: String,
    pub transparency: f32,
    pub show_workspaces: bool,
    pub show_clock: bool,
    pub show_volume: bool,
    pub show_battery: bool,
    pub show_cpu: bool,
    pub show_memory: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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
                inner_padding: 4,
                outer_padding: 0,
                border_width: 2,
                peek_x: 80,
                peek_y: 40,
                center_focused: false,
                focus_follows_mouse: false,
                corner_radius: 8,
                rounded_corners: true,
                dwm_shadows: true,
                window_opacity: 1.0,
                spring_stiffness: 180.0,
                spring_damping: 20.0,
                workspace_count: 4,
                workspace_names: vec![],
                per_monitor_workspace_names: vec![],
                default_float_width: 800,
                default_float_height: 600,
                default_split_dir: "vertical".into(),
                auto_split: false,
                monitor_layouts: vec![],
                snap_grid_size: 10,
                snap_edge_distance: 8,
                layout_presets: vec![],
                session_auto_save_interval: 0,
                resize_step_px: 0,
                snap_layouts: vec![],
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
                sticky: "Y".into(),
                theme_next: "T".into(),
                theme_prev: "Shift+T".into(),
                theme_picker: "G".into(),
                launcher: "Space".into(),
                window_search: "P".into(),
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
                show_cpu: true,
                show_memory: true,
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
            config.validate()?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.layout.workspace_count == 0 || self.layout.workspace_count > 10 {
            anyhow::bail!("workspace_count must be between 1 and 10, got {}", self.layout.workspace_count);
        }
        if self.layout.workspace_names.len() > self.layout.workspace_count {
            anyhow::bail!("workspace_names has {} entries but workspace_count is {}",
                self.layout.workspace_names.len(), self.layout.workspace_count);
        }
        if self.layout.monitor_layouts.len() > 8 {
            anyhow::bail!("monitor_layouts has {} entries (max 8 monitors supported)", self.layout.monitor_layouts.len());
        }
        for (i, rule) in self.rules.iter().enumerate() {
            if rule.match_.is_empty() {
                anyhow::bail!("rule[{}]: match field cannot be empty", i);
            }
            if let Some(ws) = rule.workspace {
                if ws >= self.layout.workspace_count {
                    anyhow::bail!("rule[{}]: workspace {} >= workspace_count ({})", i, ws, self.layout.workspace_count);
                }
            }
        }
        Ok(())
    }

    /// Save config to config.toml. Creates parent dir if needed.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let toml_str = toml::to_string_pretty(self)?;
        std::fs::write(&path, toml_str)?;
        Ok(())
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
