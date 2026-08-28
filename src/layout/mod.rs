//! UltraWM Layout Engine — hyprscroll2d-style 2D infinite lattice with bidirectional tiling
//!
//! Windows live on an infinite 2D grid. A camera pans over it.
//! Edge peeks keep neighboring rows/columns visible.
//! Cells can be split horizontally or vertically for bidirectional tiling.

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SplitDir {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CellNode {
    Leaf(WindowId),
    Split {
        dir: SplitDir,
        ratio: f32,
        primary: Box<CellNode>,
        secondary: Box<CellNode>,
    },
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
    /// Bidirectional split tree per cell
    pub cell_nodes: BTreeMap<Cell, CellNode>,
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
            cell_nodes: BTreeMap::new(),
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
                    if !self.cells.contains_key(&cell) && !self.cell_nodes.contains_key(&cell) {
                        self.cells.insert(cell, wid);
                        self.window_positions.insert(wid, cell);
                        self.cell_nodes.insert(cell, CellNode::Leaf(wid));
                        return cell;
                    }
                }
            }
        }
        let cell = Cell::new(0, 1000);
        self.cells.insert(cell, wid);
        self.window_positions.insert(wid, cell);
        self.cell_nodes.insert(cell, CellNode::Leaf(wid));
        cell
    }

    /// Split a cell in the given direction, placing the focused window as primary
    pub fn split_cell(&mut self, wid: WindowId, dir: SplitDir) -> bool {
        let cell = match self.window_positions.get(&wid) {
            Some(&c) => c,
            None => return false,
        };

        let existing = match self.cell_nodes.get(&cell) {
            Some(CellNode::Leaf(w)) => *w,
            Some(CellNode::Split { .. }) => return false,
            None => wid,
        };

        let node = CellNode::Split {
            dir,
            ratio: 0.5,
            primary: Box::new(CellNode::Leaf(wid)),
            secondary: Box::new(CellNode::Leaf(existing)),
        };

        self.cell_nodes.insert(cell, node);
        true
    }

    /// Remove split from a cell (merge children into one leaf)
    pub fn unsplit_cell(&mut self, wid: WindowId) -> bool {
        let cell = match self.window_positions.get(&wid) {
            Some(&c) => c,
            None => return false,
        };

        if let Some(CellNode::Split { primary, .. }) = self.cell_nodes.get(&cell) {
            if let CellNode::Leaf(keep_wid) = primary.as_ref() {
                // Keep the primary window, remove secondary
                self.cell_nodes.insert(cell, CellNode::Leaf(*keep_wid));
                return true;
            }
        }
        false
    }

    /// Adjust the split ratio for a cell (0.1 steps, clamped 0.1–0.9)
    pub fn adjust_split_ratio(&mut self, wid: WindowId, grow_primary: bool) {
        let cell = match self.window_positions.get(&wid) {
            Some(&c) => c,
            None => return,
        };

        if let Some(CellNode::Split { mut ratio, ref mut primary, ref mut secondary, .. }) = self.cell_nodes.get_mut(&cell) {
            if grow_primary {
                *ratio = (ratio + 0.1).min(0.9);
            } else {
                *ratio = (ratio - 0.1).max(0.1);
            }
        }
    }

    /// Get all leaf window IDs in a cell's split tree
    pub fn leaves_in_cell(&self, cell: Cell) -> Vec<WindowId> {
        let mut result = Vec::new();
        if let Some(node) = self.cell_nodes.get(&cell) {
            self.collect_leaves(node, &mut result);
        }
        result
    }

    fn collect_leaves(&self, node: &CellNode, out: &mut Vec<WindowId>) {
        match node {
            CellNode::Leaf(wid) => out.push(*wid),
            CellNode::Split { primary, secondary, .. } => {
                self.collect_leaves(primary, out);
                self.collect_leaves(secondary, out);
            }
        }
    }

    /// Get the rect for a leaf within a split tree
    pub fn leaf_rect(&self, cell: Cell, leaf_wid: WindowId, viewport_w: i32, viewport_h: i32) -> Option<(i32, i32, u32, u32)> {
        let base_x = (cell.col - self.camera.col) * (self.default_cell_w() as i32 + self.gap_x)
            - (self.peek_x - self.gap_x / 2).max(0) + viewport_w / 2;
        let base_y = (cell.row - self.camera.row) * (self.default_cell_h() as i32 + self.gap_y)
            - (self.peek_y - self.gap_y / 2).max(0) + viewport_h / 2;

        let cw = self.default_cell_w() as i32;
        let ch = self.default_cell_h() as i32;

        self.leaf_rect_in_node(self.cell_nodes.get(&cell)?, base_x - cw / 2, base_y - ch / 2, cw, ch, leaf_wid)
    }

    fn leaf_rect_in_node(&self, node: &CellNode, x: i32, y: i32, w: i32, h: i32, target: WindowId) -> Option<(i32, i32, u32, u32)> {
        match node {
            CellNode::Leaf(wid) if *wid == target => {
                Some((x.max(0), y.max(0), w as u32, h as u32))
            }
            CellNode::Leaf(_) => None,
            CellNode::Split { dir, ratio, primary, secondary } => {
                match dir {
                    SplitDir::Horizontal => {
                        let primary_w = (w as f32 * ratio) as i32;
                        let secondary_w = w - primary_w;
                        let primary_x = x;
                        let secondary_x = x + primary_w + self.gap_x;
                        self.leaf_rect_in_node(primary, primary_x, y, primary_w, h, target)
                            .or_else(|| self.leaf_rect_in_node(secondary, secondary_x, y, secondary_w, h, target))
                    }
                    SplitDir::Vertical => {
                        let primary_h = (h as f32 * ratio) as i32;
                        let secondary_h = h - primary_h;
                        let primary_y = y;
                        let secondary_y = y + primary_h + self.gap_y;
                        self.leaf_rect_in_node(primary, x, primary_y, w, primary_h, target)
                            .or_else(|| self.leaf_rect_in_node(secondary, x, secondary_y, w, secondary_h, target))
                    }
                }
            }
        }
    }

    fn default_cell_w(&self) -> u32 { self.presets.default_width() }
    fn default_cell_h(&self) -> u32 { self.presets.default_height() }

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
            // Clean up split nodes containing this window
            self.cleanup_splits(cell, wid);
        }
        self.window_sizes.remove(&wid);
        if self.focused_window == Some(wid) {
            self.focused_window = None;
        }
    }

    fn cleanup_splits(&mut self, cell: Cell, removed_wid: WindowId) {
        if let Some(node) = self.cell_nodes.get_mut(&cell) {
            self.cleanup_node(node, removed_wid);
        }
    }

    fn cleanup_node(&self, node: &mut CellNode, removed: WindowId) {
        match node {
            CellNode::Leaf(w) => {
                if *w == removed {
                    // This leaf was already removed from window_positions
                }
            }
            CellNode::Split { primary, secondary, .. } => {
                self.cleanup_node(primary, removed);
                self.cleanup_node(secondary, removed);
            }
        }
    }

    pub fn apply_layout_config(&mut self, gaps: u32, peek_x: i32, peek_y: i32) {
        self.gap_x = gaps as i32;
        self.gap_y = gaps as i32;
        self.peek_x = peek_x;
        self.peek_y = peek_y;
    }

    /// Calculate rects for all windows including split cells
    pub fn all_window_rects(&self, viewport_w: i32, viewport_h: i32) -> Vec<(WindowId, i32, i32, u32, u32)> {
        let mut result = Vec::new();
        for (&cell, &wid) in &self.cells {
            if let Some(node) = self.cell_nodes.get(&cell) {
                self.rects_for_node(cell, node, viewport_w, viewport_h, &mut result);
            }
        }
        result
    }

    fn rects_for_node(&self, cell: Cell, node: &CellNode, vw: i32, vh: i32, out: &mut Vec<(WindowId, i32, i32, u32, u32)>) {
        let cw = self.default_cell_w() as i32;
        let ch = self.default_cell_h() as i32;
        let base_x = (cell.col - self.camera.col) * (cw + self.gap_x)
            - (self.peek_x - self.gap_x / 2).max(0) + vw / 2;
        let base_y = (cell.row - self.camera.row) * (ch + self.gap_y)
            - (self.peek_y - self.gap_y / 2).max(0) + vh / 2;

        match node {
            CellNode::Leaf(wid) => {
                out.push((*wid, base_x - cw / 2, base_y - ch / 2, cw as u32, ch as u32));
            }
            CellNode::Split { dir, ratio, primary, secondary } => {
                match dir {
                    SplitDir::Horizontal => {
                        let pw = (cw as f32 * ratio) as i32;
                        let sw = cw - pw;
                        let sx = base_x + pw / 2 + self.gap_x / 2;
                        let px = base_x - pw / 2 - self.gap_x / 2;
                        self.add_rects_for(primary, px, base_y, pw, ch, out);
                        self.add_rects_for(secondary, sx, base_y, sw, ch, out);
                    }
                    SplitDir::Vertical => {
                        let ph = (ch as f32 * ratio) as i32;
                        let sh = ch - ph;
                        let py = base_y + ph / 2 + self.gap_y / 2;
                        let ty = base_y - ph / 2 - self.gap_y / 2;
                        self.add_rects_for(primary, base_x, ty, cw, ph, out);
                        self.add_rects_for(secondary, base_x, py, cw, sh, out);
                    }
                }
            }
        }
    }

    fn add_rects_for(&self, node: &CellNode, x: i32, y: i32, w: i32, h: i32, out: &mut Vec<(WindowId, i32, i32, u32, u32)>) {
        match node {
            CellNode::Leaf(wid) => {
                out.push((*wid, x, y, w as u32, h as u32));
            }
            CellNode::Split { primary, secondary, .. } => {
                match node {
                    CellNode::Split { dir, ratio, .. } => {
                        match dir {
                            SplitDir::Horizontal => {
                                let pw = (w as f32 * ratio) as i32;
                                let sw = w - pw;
                                let px = x;
                                let sx = x + pw + self.gap_x;
                                self.add_rects_for(primary, px, y, pw, h, out);
                                self.add_rects_for(secondary, sx, y, sw, h, out);
                            }
                            SplitDir::Vertical => {
                                let ph = (h as f32 * ratio) as i32;
                                let sh = h - ph;
                                let py = y;
                                let sy = y + ph + self.gap_y;
                                self.add_rects_for(primary, x, py, w, ph, out);
                                self.add_rects_for(secondary, x, sy, w, sh, out);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
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
