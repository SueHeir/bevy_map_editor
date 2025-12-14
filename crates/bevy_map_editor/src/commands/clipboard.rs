//! Tile clipboard for copy/paste operations

use bevy::prelude::*;
use std::collections::HashSet;
use uuid::Uuid;

use crate::project::Project;
use crate::EditorState;

/// Tile selection for copy/paste/delete operations
#[derive(Default, Clone)]
pub struct TileSelection {
    /// Selected tiles as (level_id, layer_idx, x, y)
    pub tiles: HashSet<(Uuid, usize, u32, u32)>,
    /// The level the selection is on
    pub level_id: Option<Uuid>,
    /// The layer the selection is on
    pub layer_idx: Option<usize>,
    /// Whether we're currently drawing a marquee selection
    pub is_selecting: bool,
    /// Start tile position for marquee selection
    pub drag_start: Option<(i32, i32)>,
}

impl TileSelection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.tiles.clear();
        self.level_id = None;
        self.layer_idx = None;
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    pub fn select_tile(
        &mut self,
        level_id: Uuid,
        layer_idx: usize,
        x: u32,
        y: u32,
        add_to_selection: bool,
    ) {
        if !add_to_selection {
            self.clear();
        }
        self.level_id = Some(level_id);
        self.layer_idx = Some(layer_idx);
        self.tiles.insert((level_id, layer_idx, x, y));
    }

    #[allow(clippy::too_many_arguments)]
    pub fn select_rectangle(
        &mut self,
        level_id: Uuid,
        layer_idx: usize,
        x1: u32,
        y1: u32,
        x2: u32,
        y2: u32,
        add_to_selection: bool,
    ) {
        if !add_to_selection {
            self.clear();
        }
        self.level_id = Some(level_id);
        self.layer_idx = Some(layer_idx);

        let min_x = x1.min(x2);
        let max_x = x1.max(x2);
        let min_y = y1.min(y2);
        let max_y = y1.max(y2);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                self.tiles.insert((level_id, layer_idx, x, y));
            }
        }
    }

    pub fn contains(&self, level_id: Uuid, layer_idx: usize, x: u32, y: u32) -> bool {
        self.tiles.contains(&(level_id, layer_idx, x, y))
    }
}

/// Clipboard content for tiles
#[derive(Default, Clone)]
pub struct ClipboardContent {
    /// Width of the clipboard region
    pub width: u32,
    /// Height of the clipboard region
    pub height: u32,
    /// Tiles as (relative_x, relative_y, tile_id)
    pub tiles: Vec<(u32, u32, Option<u32>)>,
}

/// Tile clipboard for copy/paste
#[derive(Resource, Default)]
pub struct TileClipboard {
    pub content: Option<ClipboardContent>,
}

impl TileClipboard {
    pub fn copy_selection(
        &mut self,
        selection: &TileSelection,
        project: &Project,
        _editor_state: &EditorState,
    ) {
        let Some(level_id) = selection.level_id else {
            return;
        };
        let Some(layer_idx) = selection.layer_idx else {
            return;
        };
        let Some(level) = project.get_level(level_id) else {
            return;
        };

        if selection.is_empty() {
            return;
        }

        // Find bounds
        let mut min_x = u32::MAX;
        let mut min_y = u32::MAX;
        let mut max_x = 0u32;
        let mut max_y = 0u32;

        for &(_, _, x, y) in &selection.tiles {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }

        let width = max_x - min_x + 1;
        let height = max_y - min_y + 1;

        let mut tiles = Vec::new();

        for &(_, layer, x, y) in &selection.tiles {
            if layer == layer_idx {
                let tile = level.get_tile(layer_idx, x, y);
                tiles.push((x - min_x, y - min_y, tile));
            }
        }

        self.content = Some(ClipboardContent {
            width,
            height,
            tiles,
        });
    }

    pub fn has_content(&self) -> bool {
        self.content.is_some()
    }

    pub fn clear(&mut self) {
        self.content = None;
    }
}
