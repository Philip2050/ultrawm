use super::{GridState, WindowId};

/// A workspace holds an optional grid state.
#[derive(Debug, Clone)]
pub struct Workspace {
    pub id: u32,
    pub name: String,
    pub grid: Option<GridState>,
    pub monitor: Option<String>,
}

impl Workspace {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: format!("{}", id),
            grid: Some(GridState::new()),
            monitor: None,
        }
    }

    pub fn has_grid(&self) -> bool {
        self.grid.is_some()
    }

    pub fn focused_window(&self) -> Option<WindowId> {
        self.grid.as_ref().and_then(|g| g.focused_window)
    }

    pub fn cell_rect(&self, wid: WindowId, vw: i32, vh: i32) -> Option<(i32, i32, u32, u32)> {
        self.grid.as_ref().and_then(|g| {
            g.window_positions.get(&wid).map(|&cell| g.cell_rect(cell, vw, vh))
        })
    }
}
