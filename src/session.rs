use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionWindowState {
    pub exe: String,
    pub cell: crate::layout::Cell,
    pub floating: bool,
    pub workspace: usize,
    pub monitor: usize,
    pub opacity: Option<f32>,
    pub sticky: bool,
    pub maximized: bool,
    pub always_on_top: bool,
    pub z_order: usize,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub w: Option<i32>,
    pub h: Option<i32>,
    pub float_x: Option<i32>,
    pub float_y: Option<i32>,
    pub float_w: Option<i32>,
    pub float_h: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridSessionState {
    pub windows: Vec<SessionWindowState>,
    pub camera: crate::layout::Cell,
    pub focused: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorSessionState {
    pub grids: Vec<GridSessionState>,
    pub current: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub version: u32,
    pub monitors: Vec<MonitorSessionState>,
}

impl SessionState {
    pub fn path() -> PathBuf {
        let xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let base = xdg
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")));
        base.join(".config/ultrawm/session.json")
    }

    pub fn load() -> anyhow::Result<Option<Self>> {
        let path = Self::path();
        if !path.exists() {
            return Ok(None);
        }
        let contents = std::fs::read_to_string(&path)?;

        // Try new format (version 2)
        #[derive(Deserialize)]
        struct NewFormat {
            version: u32,
            monitors: Vec<MonitorSessionState>,
        }

        if let Ok(new) = serde_json::from_str::<NewFormat>(&contents) {
            if new.version >= 2 && !new.monitors.is_empty() {
                return Ok(Some(Self {
                    version: new.version,
                    monitors: new.monitors,
                }));
            }
        }

        // Fall back to old format (version 1)
        #[derive(Deserialize)]
        struct OldFormat {
            windows: Vec<SessionWindowState>,
            camera: crate::layout::Cell,
        }

        if let Ok(old) = serde_json::from_str::<OldFormat>(&contents) {
            let monitor = MonitorSessionState {
                grids: vec![GridSessionState {
                    windows: old.windows,
                    camera: old.camera,
                    focused: None,
                }],
                current: 0,
            };
            return Ok(Some(Self {
                version: 2,
                monitors: vec![monitor],
            }));
        }

        Ok(None)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, contents)?;
        Ok(())
    }
}
