//! UltraWM Layout Engine — hyprscroll2d-style 2D infinite lattice
//!
//! Windows live on an infinite 2D grid. A camera pans over it.
//! Edge peeks keep neighboring rows/columns visible.

mod workspace;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type WindowId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
pub struct Cell {
    pub row: i32,
    pub col: i32,
}

impl Cell {
    pub const fn new(row: i32, col: i32) -> Self {
        Self { row, col }
    }

    pub fn neighbor(&self, dr: i32, dc: i32) -> Self {
        Self::new(self.row + dr, self.col + dc)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellSizePresets {
    pub width_steps: Vec<u32>,
    pub height_steps: Vec<u32>,
    pub default_width_step: usize,
    pub default_height_step: usize,
}

impl Default for CellSizePresets {
    fn default() -> Self {
        Self {
            width_steps: vec![400, 600, 800, 1000, 1200],
            height_steps: vec![300, 450, 600, 750],
            default_width_step: 1,
            default_height_step: 1,
        }
    }
}

impl CellSizePresets {
    pub fn default_width(&self) -> u32 {
        self.width_steps
            .get(self.default_width_step)
            .copied()
            .unwrap_or(self.width_steps[0])
    }

    pub fn default_height(&self) -> u32 {
        self.height_steps
            .get(self.default_height_step)
            .copied()
            .unwrap_or(self.height_steps[0])
    }

    pub fn grow_width(&self, current: u32) -> u32 {
        self.width_steps.iter().find(|&&s| s > current).copied().unwrap_or(current)
    }

    pub fn shrink_width(&self, current: u32) -> u32 {
        self.width_steps.iter().rev().find(|&&s| s < current).copied().unwrap_or(current)
    }

    pub fn grow_height(&self, current: u32) -> u32 {
        self.height_steps.iter().find(|&&s| s > current).copied().unwrap_or(current)
    }

    pub fn shrink_height(&self, current: u32) -> u32 {
        self.height_steps.iter().rev().find(|&&s| s < current).copied().unwrap_or(current)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridState {
    pub cells: BTreeMap<Cell, WindowId>,
    pub window_positions: BTreeMap<WindowId, Cell>,
    pub presets: CellSizePresets,
    pub window_sizes: BTreeMap<WindowId, (u32, u32)>,
    pub camera: Cell,
    pub peek_x: i32,
    pub peek_y: i32,
    pub gap_x: i32,
    pub gap_y: i32,
    pub focused_window: Option<WindowId>,
}

impl GridState {
    pub fn new() -> Self {
        Self {
            cells: BTreeMap::new(),
            window_positions: BTreeMap::new(),
            presets: CellSizePresets::default(),
            window_sizes: BTreeMap::new(),
            camera: Cell::new(0, 0),
            peek_x: 80,
            peek_y: 40,
            gap_x: 8,
            gap_y: 8,
            focused_window: None,
        }
    }

    pub fn place_window(&mut self, wid: WindowId) -> Cell {
        if let Some(&existing) = self.window_positions.get(&wid) {
            return existing;
        }
        for dist in 0..1000 {
            for dc in (-dist..=dist).rev() {
                for dr in (-dist..=dist).rev() {
                    if dr == 0 && dc == 0 { continue; }
                    let cell = Cell::new(dr, dc);
                    if !self.cells.contains_key(&cell) {
                        self.cells.insert(cell, wid);
                        self.window_positions.insert(wid, cell);
                        return cell;
                    }
                }
            }
        }
        let cell = Cell::new(0, 1000);
        self.cells.insert(cell, wid);
        self.window_positions.insert(wid, cell);
        cell
    }

    pub fn focus_window(&mut self, wid: WindowId) -> Option<Cell> {
        if let Some(&cell) = self.window_positions.get(&wid) {
            self.focused_window = Some(wid);
            self.camera = cell;
            Some(cell)
        } else {
            None
        }
    }

    pub fn move_focus(&mut self, dr: i32, dc: i32) -> Option<WindowId> {
        let focused = self.focused_window?;
        let from_cell = self.window_positions.get(&focused).copied()?;
        let target = from_cell.neighbor(dr, dc);

        if let Some(&other_wid) = self.cells.get(&target) {
            self.cells.insert(target, focused);
            self.cells.insert(from_cell, other_wid);
            self.window_positions.insert(focused, target);
            self.window_positions.insert(other_wid, from_cell);
            if self.focused_window == Some(other_wid) {
                self.focused_window = Some(focused);
            }
            self.camera = target;
            Some(focused)
        } else {
            self.cells.insert(target, focused);
            self.cells.remove(&from_cell);
            self.window_positions.insert(focused, target);
            self.camera = target;
            Some(focused)
        }
    }

    pub fn move_window(&mut self, wid: WindowId, dr: i32, dc: i32) {
        if let Some(&from_cell) = self.window_positions.get(&wid) {
            let target = from_cell.neighbor(dr, dc);
            if let Some(&other_wid) = self.cells.get(&target) {
                self.cells.insert(target, wid);
                self.cells.insert(from_cell, other_wid);
                self.window_positions.insert(wid, target);
                self.window_positions.insert(other_wid, from_cell);
            } else {
                self.cells.insert(target, wid);
                self.cells.remove(&from_cell);
                self.window_positions.insert(wid, target);
            }
            self.camera = target;
        }
    }

    pub fn pan(&mut self, dr: i32, dc: i32) {
        self.camera = self.camera.neighbor(dr, dc);
        if let Some(wid) = self.focused_window {
            if let Some(cell) = self.window_positions.get(&wid) {
                self.camera = *cell;
            }
        }
    }

    pub fn remove_window(&mut self, wid: WindowId) {
        if let Some(cell) = self.window_positions.remove(&wid) {
            self.cells.remove(&cell);
        }
        self.window_sizes.remove(&wid);
        if self.focused_window == Some(wid) {
            self.focused_window = None;
        }
    }

    pub fn apply_layout_config(&mut self, gaps: u32, peek_x: i32, peek_y: i32) {
        self.gap_x = gaps as i32;
        self.gap_y = gaps as i32;
        self.peek_x = peek_x;
        self.peek_y = peek_y;
    }

    pub fn cell_rect(&self, cell: Cell, viewport_w: i32, viewport_h: i32) -> (i32, i32, u32, u32) {
        let cw = self.cell_width(cell);
        let ch = self.cell_height(cell);
        let dx = (cell.col - self.camera.col) * (cw as i32 + self.gap_x)
            - (self.peek_x - self.gap_x / 2).max(0);
        let dy = (cell.row - self.camera.row) * (ch as i32 + self.gap_y)
            - (self.peek_y - self.gap_y / 2).max(0);
        let x = dx + viewport_w / 2 - cw as i32 / 2;
        let y = dy + viewport_h / 2 - ch as i32 / 2;
        (x.max(0), y.max(0), cw, ch)
    }

    fn cell_width(&self, cell: Cell) -> u32 {
        self.cells.get(&cell).and_then(|&wid| self.window_sizes.get(&wid).map(|(w, _)| *w)).unwrap_or_else(|| self.presets.default_width())
    }

    fn cell_height(&self, cell: Cell) -> u32 {
        self.cells.get(&cell).and_then(|&wid| self.window_sizes.get(&wid).map(|(_, h)| *h)).unwrap_or_else(|| self.presets.default_height())
    }

    pub fn grow_width(&mut self, wid: WindowId) {
        let cur = self.window_sizes.get(&wid).map(|(w, _)| *w).unwrap_or_else(|| self.presets.default_width());
        let new = self.presets.grow_width(cur);
        self.window_sizes.insert(wid, (new, self.cell_h_for(wid)));
    }

    pub fn shrink_width(&mut self, wid: WindowId) {
        let cur = self.window_sizes.get(&wid).map(|(w, _)| *w).unwrap_or_else(|| self.presets.default_width());
        let new = self.presets.shrink_width(cur);
        self.window_sizes.insert(wid, (new, self.cell_h_for(wid)));
    }

    pub fn grow_height(&mut self, wid: WindowId) {
        let cur = self.window_sizes.get(&wid).map(|(_, h)| *h).unwrap_or_else(|| self.presets.default_height());
        let new = self.presets.grow_height(cur);
        self.window_sizes.insert(wid, (self.cell_w_for(wid), new));
    }

    pub fn shrink_height(&mut self, wid: WindowId) {
        let cur = self.window_sizes.get(&wid).map(|(_, h)| *h).unwrap_or_else(|| self.presets.default_height());
        let new = self.presets.shrink_height(cur);
        self.window_sizes.insert(wid, (self.cell_w_for(wid), new));
    }

    pub fn cell_w_for(&self, wid: WindowId) -> u32 {
        self.window_sizes.get(&wid).map(|(w, _)| *w).unwrap_or_else(|| self.presets.default_width())
    }

    pub fn cell_h_for(&self, wid: WindowId) -> u32 {
        self.window_sizes.get(&wid).map(|(_, h)| *h).unwrap_or_else(|| self.presets.default_height())
    }
}

impl Default for GridState {
    fn default() -> Self {
        Self::new()
    }
}
