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
pub struct TabGroup {
    pub windows: Vec<WindowId>,
    pub active: usize,
    pub title: Option<String>,
}

impl TabGroup {
    pub fn new(first: WindowId) -> Self {
        Self {
            windows: vec![first],
            active: 0,
            title: None,
        }
    }

    pub fn add(&mut self, wid: WindowId) {
        if !self.windows.contains(&wid) {
            self.windows.push(wid);
        }
    }

    pub fn remove(&mut self, wid: WindowId) -> bool {
        if let Some(pos) = self.windows.iter().position(|&w| w == wid) {
            self.windows.remove(pos);
            if self.windows.is_empty() {
                return true; // group is empty
            }
            if self.active >= self.windows.len() {
                self.active = self.windows.len() - 1;
            }
            false
        } else {
            false
        }
    }

    pub fn active(&self) -> WindowId {
        self.windows.get(self.active).copied().unwrap_or(self.windows[0])
    }

    pub fn switch(&mut self, idx: usize) {
        if idx < self.windows.len() {
            self.active = idx;
        }
    }

    pub fn len(&self) -> usize { self.windows.len() }
    pub fn is_empty(&self) -> bool { self.windows.is_empty() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutMode {
    Grid,
    Master,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CellNode {
    Leaf(WindowId),
    Tab(TabGroup),
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
    pub layout_mode: LayoutMode,
    /// Custom column widths (pixels). When set, overrides equal-width calculation.
    pub custom_widths: Vec<u32>,
    /// Custom row heights (pixels). When set, overrides equal-height calculation.
    pub custom_heights: Vec<u32>,
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
            layout_mode: LayoutMode::Grid,
            custom_widths: Vec::new(),
            custom_heights: Vec::new(),
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

    /// Rearrange all windows into a flat grid of cols x rows cells
    pub fn snap_layout(&mut self, windows: &[WindowId], cols: usize, rows: usize) {
        self.cells.clear();
        self.window_positions.clear();
        self.cell_nodes.clear();
        self.focused_window = None;

        if windows.is_empty() || cols == 0 || rows == 0 {
            return;
        }

        let total_cells = cols * rows;
        for (idx, &wid) in windows.iter().take(total_cells).enumerate() {
            let r = (idx / cols) as i32;
            let c = (idx % cols) as i32;
            let cell = Cell::new(r, c);
            self.cells.insert(cell, wid);
            self.window_positions.insert(wid, cell);
            self.cell_nodes.insert(cell, CellNode::Leaf(wid));
        }

        if !windows.is_empty() {
            self.focused_window = Some(windows[0]);
        }
    }

    /// Rearrange windows with custom column widths and/or row heights
    /// widths/heights are pixel values; empty = equal distribution
    pub fn snap_layout_custom(&mut self, windows: &[WindowId], widths: &[u32], heights: &[u32]) {
        self.cells.clear();
        self.window_positions.clear();
        self.cell_nodes.clear();
        self.focused_window = None;
        self.custom_widths = widths.to_vec();
        self.custom_heights = heights.to_vec();

        if windows.is_empty() {
            return;
        }

        let cols = if widths.is_empty() { 1 } else { widths.len() };
        let rows = if heights.is_empty() { 1 } else { heights.len() };
        let total_cells = cols * rows;

        for (idx, &wid) in windows.iter().take(total_cells).enumerate() {
            let r = (idx / cols) as i32;
            let c = (idx % cols) as i32;
            let cell = Cell::new(r, c);
            self.cells.insert(cell, wid);
            self.window_positions.insert(wid, cell);
            self.cell_nodes.insert(cell, CellNode::Leaf(wid));
        }

        if !windows.is_empty() {
            self.focused_window = Some(windows[0]);
        }
    }

    /// Split a cell in the given direction, placing the focused window as primary
    pub fn split_cell(&mut self, wid: WindowId, dir: SplitDir) -> bool {
        let cell = match self.window_positions.get(&wid) {
            Some(&c) => c,
            None => return false,
        };

        let existing = match self.cell_nodes.get(&cell) {
            Some(CellNode::Leaf(w)) => *w,
            Some(CellNode::Tab(_)) => return false, // can't split a tab group
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

    /// Tab windows together in the same cell (stack them)
    pub fn tab_cell(&mut self, wid: WindowId, with_wid: WindowId) -> bool {
        let cell = match self.window_positions.get(&wid) {
            Some(&c) => c,
            None => return false,
        };

        let other_cell = match self.window_positions.get(&with_wid) {
            Some(&c) => c,
            None => return false,
        };

        if cell != other_cell {
            return false; // windows must be in same cell
        }

        match self.cell_nodes.get_mut(&cell) {
            Some(CellNode::Leaf(_)) => {
                let mut group = TabGroup::new(wid);
                group.add(with_wid);
                self.cell_nodes.insert(cell, CellNode::Tab(group));
                true
            }
            Some(CellNode::Tab(ref mut group)) => {
                group.add(with_wid);
                true
            }
            _ => false,
        }
    }

    /// Untab a cell back to a single leaf (keeps focused window)
    pub fn untab_cell(&mut self, wid: WindowId) -> bool {
        let cell = match self.window_positions.get(&wid) {
            Some(&c) => c,
            None => return false,
        };

        if let Some(CellNode::Tab(ref group)) = self.cell_nodes.get(&cell) {
            let active = group.active();
            self.cell_nodes.insert(cell, CellNode::Leaf(active));
            true
        } else {
            false
        }
    }

    /// Switch to next/prev tab in focused cell
    pub fn cycle_tab(&mut self, wid: WindowId, forward: bool) -> bool {
        let cell = match self.window_positions.get(&wid) {
            Some(&c) => c,
            None => return false,
        };

        if let Some(CellNode::Tab(ref mut group)) = self.cell_nodes.get_mut(&cell) {
            if group.len() <= 1 {
                return false;
            }
            if forward {
                group.active = (group.active + 1) % group.len();
            } else {
                group.active = if group.active == 0 { group.len() - 1 } else { group.active - 1 };
            }
            true
        } else {
            false
        }
    }

    /// Adjust the split ratio for a cell (0.1 steps, clamped 0.1–0.9)
    pub fn adjust_split_ratio(&mut self, wid: WindowId, grow_primary: bool) {
        let cell = match self.window_positions.get(&wid) {
            Some(&c) => c,
            None => return,
        };

        if let Some(CellNode::Split { ratio, .. }) = self.cell_nodes.get_mut(&cell) {
            if grow_primary {
                *ratio += 0.1;
                if *ratio > 0.9 { *ratio = 0.9; }
            } else {
                *ratio -= 0.1;
                if *ratio < 0.1 { *ratio = 0.1; }
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
            CellNode::Tab(group) => {
                for &wid in &group.windows {
                    out.push(wid);
                }
            }
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
            CellNode::Tab(group) => {
                if group.active() == target {
                    Some((x.max(0), y.max(0), w as u32, h as u32))
                } else {
                    None
                }
            }
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

    /// Get custom cell size for a cell, falling back to default
    fn custom_cell_size(&self, cell: Cell) -> (u32, u32) {
        let cw = if !self.custom_widths.is_empty() {
            let idx = cell.col.max(0) as usize;
            if idx < self.custom_widths.len() {
                self.custom_widths[idx]
            } else {
                self.default_cell_w()
            }
        } else {
            self.default_cell_w()
        };
        let ch = if !self.custom_heights.is_empty() {
            let idx = cell.row.max(0) as usize;
            if idx < self.custom_heights.len() {
                self.custom_heights[idx]
            } else {
                self.default_cell_h()
            }
        } else {
            self.default_cell_h()
        };
        (cw, ch)
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
            cleanup_node(node, removed_wid);
        }
    }
}

fn cleanup_node(node: &mut CellNode, removed: WindowId) {
    match node {
        CellNode::Leaf(w) => {
            if *w == removed {
                // This leaf was already removed from window_positions
            }
        }
        CellNode::Tab(tg) => {
            tg.windows.retain(|w| *w != removed);
            if tg.active >= tg.windows.len() && !tg.windows.is_empty() {
                tg.active = tg.windows.len() - 1;
            }
        }
        CellNode::Split { primary, secondary, .. } => {
            cleanup_node(primary, removed);
            cleanup_node(secondary, removed);
        }
    }
}

impl GridState {
    pub fn apply_layout_config(&mut self, gaps: u32, _inner_padding: u32, peek_x: i32, peek_y: i32) {
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
        let (cw, ch) = self.custom_cell_size(cell);
        let cw = cw as i32;
        let ch = ch as i32;
        let base_x = (cell.col - self.camera.col) * (cw + self.gap_x)
            - (self.peek_x - self.gap_x / 2).max(0) + vw / 2;
        let base_y = (cell.row - self.camera.row) * (ch + self.gap_y)
            - (self.peek_y - self.gap_y / 2).max(0) + vh / 2;

        match node {
            CellNode::Leaf(wid) => {
                out.push((*wid, base_x - cw / 2, base_y - ch / 2, cw as u32, ch as u32));
            }
            CellNode::Tab(group) => {
                let active = group.active();
                out.push((active, base_x - cw / 2, base_y - ch / 2, cw as u32, ch as u32));
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
            CellNode::Tab(group) => {
                let active = group.active();
                out.push((active, x, y, w as u32, h as u32));
            }
            CellNode::Split { dir, ratio, primary, secondary } => {
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
        }
    }

    pub fn cell_rect(&self, cell: Cell, viewport_w: i32, viewport_h: i32) -> (i32, i32, u32, u32) {
        if self.layout_mode == LayoutMode::Master && cell.col == 0 {
            // Master window: full height, 50% width
            let mw = (viewport_w / 2).max(100);
            let mh = viewport_h.max(100);
            let x = (cell.col - self.camera.col) * (mw + self.gap_x)
                - (self.peek_x - self.gap_x / 2).max(0);
            let y = (cell.row - self.camera.row) * (mh + self.gap_y)
                - (self.peek_y - self.gap_y / 2).max(0);
            return (x.max(0), y.max(0), mw as u32, mh as u32);
        }

        if self.layout_mode == LayoutMode::Master && cell.col == 1 {
            // Stack windows: 50% width, height divided among stack windows
            let sw = (viewport_w - viewport_w / 2 - self.gap_x).max(100);
            let stack_count = self.cells.keys().filter(|c| c.col == 1).count().max(1);
            let sh = (viewport_h / stack_count as i32).max(100);
            let x = viewport_w / 2 + (cell.col - self.camera.col) * (sw + self.gap_x)
                - (self.peek_x - self.gap_x / 2).max(0);
            let y = (cell.row - self.camera.row) * (sh + self.gap_y)
                - (self.peek_y - self.gap_y / 2).max(0);
            return (x.max(0), y.max(0), sw as u32, sh as u32);
        }

        let (cw, ch) = self.custom_cell_size(cell);
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
