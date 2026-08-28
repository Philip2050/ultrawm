use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct Theme {
    pub name: String,
    pub author: Option<String>,

    pub background: String,
    pub foreground: String,
    pub accent: String,
    pub inactive: String,
    pub urgent: String,
    pub shadow: String,

    pub wallpaper: Option<String>,

    pub dark_mode: bool,
    pub cursor_theme: Option<String>,
    pub icon_theme: Option<String>,

    pub bar_background: Option<String>,
    pub bar_foreground: Option<String>,
    pub bar_height: Option<u32>,

    pub launcher_background: Option<String>,
    pub launcher_foreground: Option<String>,
    pub launcher_accent: Option<String>,
    pub launcher_border_radius: Option<u32>,
}

impl Theme {
    pub fn catppuccin_mocha() -> Self {
        Self {
            name: "catppuccin-mocha".into(),
            author: Some("catppuccin".into()),
            background: "#1e1e2e".into(),
            foreground: "#cdd6f4".into(),
            accent: "#cba6f7".into(),
            inactive: "#45475a".into(),
            urgent: "#f38ba8".into(),
            shadow: "#00000066".into(),
            wallpaper: None,
            dark_mode: true,
            cursor_theme: Some("Bibata-Modern-Classic".into()),
            icon_theme: Some("Papirus-Dark".into()),
            bar_background: Some("#181825".into()),
            bar_foreground: Some("#cdd6f4".into()),
            bar_height: Some(40),
            launcher_background: Some("#11111b".into()),
            launcher_foreground: Some("#cdd6f4".into()),
            launcher_accent: Some("#cba6f7".into()),
            launcher_border_radius: Some(12),
        }
    }

    pub fn tokyo_night() -> Self {
        Self {
            name: "tokyo-night".into(),
            author: Some("enkia".into()),
            background: "#1a1b26".into(),
            foreground: "#c0caf5".into(),
            accent: "#7aa2f7".into(),
            inactive: "#292e42".into(),
            urgent: "#f7768e".into(),
            shadow: "#00000066".into(),
            wallpaper: None,
            dark_mode: true,
            cursor_theme: Some("Bibata-Modern-Classic".into()),
            icon_theme: Some("Papirus-Dark".into()),
            bar_background: Some("#16161e".into()),
            bar_foreground: Some("#c0caf5".into()),
            bar_height: Some(40),
            launcher_background: Some("#16161e".into()),
            launcher_foreground: Some("#c0caf5".into()),
            launcher_accent: Some("#7aa2f7".into()),
            launcher_border_radius: Some(12),
        }
    }

    pub fn gruvbox_dark() -> Self {
        Self {
            name: "gruvbox-dark".into(),
            author: Some("morhetz".into()),
            background: "#282828".into(),
            foreground: "#ebdbb2".into(),
            accent: "#fe8019".into(),
            inactive: "#3c3836".into(),
            urgent: "#fb4934".into(),
            shadow: "#00000066".into(),
            wallpaper: None,
            dark_mode: true,
            cursor_theme: Some("Bibata-Modern-Classic".into()),
            icon_theme: Some("Papirus-Dark".into()),
            bar_background: Some("#1d2021".into()),
            bar_foreground: Some("#ebdbb2".into()),
            bar_height: Some(40),
            launcher_background: Some("#1d2021".into()),
            launcher_foreground: Some("#ebdbb2".into()),
            launcher_accent: Some("#fe8019".into()),
            launcher_border_radius: Some(12),
        }
    }

    pub fn nord() -> Self {
        Self {
            name: "nord".into(),
            author: Some("arcticicestudio".into()),
            background: "#2e3440".into(),
            foreground: "#d8dee9".into(),
            accent: "#88c0d0".into(),
            inactive: "#3b4252".into(),
            urgent: "#bf616a".into(),
            shadow: "#00000066".into(),
            wallpaper: None,
            dark_mode: true,
            cursor_theme: Some("Bibata-Modern-Classic".into()),
            icon_theme: Some("Papirus-Dark".into()),
            bar_background: Some("#272c36".into()),
            bar_foreground: Some("#d8dee9".into()),
            bar_height: Some(40),
            launcher_background: Some("#272c36".into()),
            launcher_foreground: Some("#d8dee9".into()),
            launcher_accent: Some("#88c0d0".into()),
            launcher_border_radius: Some(12),
        }
    }

    pub fn rose_pine() -> Self {
        Self {
            name: "rose-pine".into(),
            author: Some("rose-pine".into()),
            background: "#191724".into(),
            foreground: "#e0def4".into(),
            accent: "#ebbcba".into(),
            inactive: "#26233a".into(),
            urgent: "#eb6f92".into(),
            shadow: "#00000066".into(),
            wallpaper: None,
            dark_mode: true,
            cursor_theme: Some("Bibata-Modern-Classic".into()),
            icon_theme: Some("Papirus-Dark".into()),
            bar_background: Some("#1f1d2e".into()),
            bar_foreground: Some("#e0def4".into()),
            bar_height: Some(40),
            launcher_background: Some("#1f1d2e".into()),
            launcher_foreground: Some("#e0def4".into()),
            launcher_accent: Some("#ebbcba".into()),
            launcher_border_radius: Some(12),
        }
    }
}

pub struct ThemeManager {
    themes_dir: PathBuf,
    current: usize,
    themes: Vec<Theme>,
}

impl ThemeManager {
    fn themes_dir() -> PathBuf {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg).join("ultrawm/themes")
        } else {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            home.join(".config/ultrawm/themes")
        }
    }

    pub fn list_themes() -> anyhow::Result<Vec<String>> {
        let mut themes = Vec::new();
        for name in ["catppuccin-mocha", "tokyo-night", "gruvbox-dark", "nord", "rose-pine"] {
            themes.push(name.into());
        }
        let dir = Self::themes_dir();
        if dir.exists() {
            for entry in std::fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        if !themes.contains(&name.to_string()) {
                            themes.push(name.into());
                        }
                    }
                }
            }
        }
        themes.sort();
        Ok(themes)
    }

    pub fn load() -> anyhow::Result<Self> {
        let mut themes = vec![
            Theme::catppuccin_mocha(),
            Theme::tokyo_night(),
            Theme::gruvbox_dark(),
            Theme::nord(),
            Theme::rose_pine(),
        ];

        let dir = Self::themes_dir();
        if dir.exists() {
            for entry in std::fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    if let Ok(contents) = std::fs::read_to_string(&path) {
                        if let Ok(theme) = serde_json::from_str::<Theme>(&contents) {
                            themes.push(theme);
                        }
                    }
                }
            }
        }

        let current = 0;
        let tm = Self {
            themes_dir: dir,
            current,
            themes,
        };

        tm.apply(tm.current_theme())?;
        Ok(tm)
    }

    pub fn current_theme(&self) -> &Theme {
        &self.themes[self.current]
    }

    pub fn apply_idx(&mut self, idx: usize) -> anyhow::Result<()> {
        if idx < self.themes.len() {
            self.current = idx;
            self.apply(self.current_theme())?;
        }
        Ok(())
    }

    pub fn next_theme(&mut self) -> anyhow::Result<()> {
        self.current = (self.current + 1) % self.themes.len();
        self.apply(self.current_theme())
    }

    pub fn prev_theme(&mut self) -> anyhow::Result<()> {
        self.current = if self.current == 0 { self.themes.len() - 1 } else { self.current - 1 };
        self.apply(self.current_theme())
    }

    pub fn apply(&self, theme: &Theme) -> anyhow::Result<()> {
        use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_SETDESKWALLPAPER, SPIF_UPDATEINIFILE, SPIF_SENDCHANGE};

        // Apply dark mode via registry
        if let Ok(key) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .open_subkey_with_flags("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize", winreg::enums::KEY_SET_VALUE)
        {
            let _ = key.set_value("AppsUseLightTheme", &if theme.dark_mode { 0u32 } else { 1u32 });
            let _ = key.set_value("SystemUsesLightTheme", &if theme.dark_mode { 0u32 } else { 1u32 });
        }

        // Apply accent color via registry (immersive color)
        if let Ok(key) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .open_subkey_with_flags("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\AccentColor", winreg::enums::KEY_SET_VALUE)
        {
            let rgb = parse_hex(&theme.accent);
            let _ = key.set_value("AccentColor", &rgb);
        }

        // Apply wallpaper if specified
        if let Some(ref wp) = theme.wallpaper {
            if Path::new(wp).exists() {
                let wp_w: Vec<u16> = wp.encode_utf16().chain(Some(0)).collect();
                unsafe {
                    let _ = SystemParametersInfoW(
                        SPI_SETDESKWALLPAPER,
                        0,
                        Some(wp_w.as_ptr() as *mut _),
                        SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
                    );
                }
            }
        }

        log::info!("Applied theme: {}", theme.name);
        Ok(())
    }
}

fn parse_hex(s: &str) -> u32 {
    let s = s.trim_start_matches('#');
    if s.len() == 6 {
        let r = u32::from_str_radix(&s[0..2], 16).unwrap_or(0);
        let g = u32::from_str_radix(&s[2..4], 16).unwrap_or(0);
        let b = u32::from_str_radix(&s[4..6], 16).unwrap_or(0);
        (0xFF << 24) | (b << 16) | (g << 8) | r
    } else if s.len() == 8 {
        u32::from_str_radix(s, 16).unwrap_or(0xFF000000)
    } else {
        0xFF7F7F7F
    }
}
