use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionWindowState {
    pub exe: String,
    pub cell: crate::layout::Cell,
    pub floating: bool,
    pub workspace: usize,
    pub opacity: Option<f32>,
    pub sticky: bool,
    pub maximized: bool,
    pub always_on_top: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub windows: Vec<SessionWindowState>,
    pub camera: crate::layout::Cell,
}

impl SessionState {
    pub fn path() -> PathBuf {
        let xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let base = xdg
            .as_deref()
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir())
            .unwrap_or_else(|| PathBuf::from("."));
        base.join(".config/ultrawm/session.json")
    }

    pub fn load() -> anyhow::Result<Option<Self>> {
        let path = Self::path();
        if !path.exists() {
            return Ok(None);
        }
        let contents = std::fs::read_to_string(&path)?;
        let state: SessionState = serde_json::from_str(&contents)?;
        Ok(Some(state))
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
