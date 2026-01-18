//! Tileset and terrain editor window
//!
//! Implements Tiled-style terrain corner/edge painting where users click
//! directly on zones within each tile to assign terrain types.
//!
//! This follows Tiled's tileset editor approach:
//! - Each tile has clickable zones (corners, edges, or both for Mixed)
//! - Clicking assigns the selected terrain to that zone of that tile
//! - Visual overlays show assigned terrain with curved boundaries

use bevy_egui::egui::{self, Color32, Pos2, Shape};
use bevy_map_autotile::terrain::Color as TerrainColor;
use bevy_map_autotile::TerrainSetType;
use std::f32::consts::PI;

use super::{find_base_tile_for_position, EditorTheme, TilesetTextureCache};
use crate::project::Project;
use crate::EditorState;

/// State for the tileset editor
pub struct TilesetEditorState {
    pub selected_tab: TilesetEditorTab,
    pub selected_image_idx: Option<usize>,
    /// Selected terrain index for painting on tiles
    pub selected_terrain_for_assignment: Option<usize>,
    /// Selected tile for property editing
    pub selected_tile_for_properties: Option<u32>,
    /// Zoom level for terrain tile display (1.0 = 32px, 2.0 = 64px, etc.)
    pub terrain_tile_zoom: f32,
    /// State for the collision editor tab
    pub collision_editor: CollisionEditorState,
    /// Active shift+drag selection start for tile merging (col, row, image_idx)
    pub merge_drag_start: Option<(u32, u32, usize)>,
    /// Active shift+drag selection current position for tile merging (col, row)
    pub merge_drag_current: Option<(u32, u32)>,
}

impl Default for TilesetEditorState {
    fn default() -> Self {
        Self {
            selected_tab: TilesetEditorTab::default(),
            selected_image_idx: None,
            selected_terrain_for_assignment: None,
            selected_tile_for_properties: None,
            terrain_tile_zoom: 1.0, // Default to 1x zoom (32px tiles)
            collision_editor: CollisionEditorState::default(),
            merge_drag_start: None,
            merge_drag_current: None,
        }
    }
}

// ============================================================================
// Per-Tile Zone Detection (Tiled-style)
// ============================================================================

/// Determine which zone was clicked within a tile based on local coordinates.
///
/// For Corner terrain sets (4 zones):
/// ```text
/// +-------+
/// | 0 | 1 |  TL=0, TR=1
/// |---+---|
/// | 2 | 3 |  BL=2, BR=3
/// +-------+
/// ```
///
/// For Edge terrain sets (4 zones):
/// ```text
/// +---0---+
/// |       |  Top=0, Right=1, Bottom=2, Left=3
/// 3       1
/// |       |
/// +---2---+
/// ```
///
/// For Mixed terrain sets (8 zones, center ignored):
/// ```text
/// +---+---+---+
/// | 0 | 1 | 2 |  TL=0, Top=1, TR=2
/// +---+---+---+
/// | 7 | X | 3 |  Left=7, (center ignored), Right=3
/// +---+---+---+
/// | 6 | 5 | 4 |  BL=6, Bottom=5, BR=4
/// +---+---+---+
/// ```
fn get_tile_zone(local_x: f32, local_y: f32, set_type: TerrainSetType) -> Option<usize> {
    match set_type {
        TerrainSetType::Corner => {
            // Simple quadrant detection
            let right = local_x > 0.5;
            let bottom = local_y > 0.5;
            Some(match (right, bottom) {
                (false, false) => 0, // TL
                (true, false) => 1,  // TR
                (false, true) => 2,  // BL
                (true, true) => 3,   // BR
            })
        }
        TerrainSetType::Edge => {
            // Find nearest edge by comparing distance to center
            let dx = (local_x - 0.5).abs();
            let dy = (local_y - 0.5).abs();
            if dy > dx {
                // Vertical distance larger = closer to top/bottom edge
                Some(if local_y < 0.5 { 0 } else { 2 }) // Top or Bottom
            } else {
                // Horizontal distance larger = closer to left/right edge
                Some(if local_x < 0.5 { 3 } else { 1 }) // Left or Right
            }
        }
        TerrainSetType::Mixed => {
            // 3x3 grid, center returns None (not clickable)
            let zone_x = if local_x < 0.33 {
                0
            } else if local_x < 0.67 {
                1
            } else {
                2
            };
            let zone_y = if local_y < 0.33 {
                0
            } else if local_y < 0.67 {
                1
            } else {
                2
            };
            match (zone_x, zone_y) {
                (0, 0) => Some(0), // TL
                (1, 0) => Some(1), // Top
                (2, 0) => Some(2), // TR
                (2, 1) => Some(3), // Right
                (2, 2) => Some(4), // BR
                (1, 2) => Some(5), // Bottom
                (0, 2) => Some(6), // BL
                (0, 1) => Some(7), // Left
                (1, 1) => Some(8), // Center zone - visual marker only, doesn't affect Wang matching
                _ => None,
            }
        }
    }
}

/// Generate points for an arc approximation
fn arc_points(cx: f32, cy: f32, r: f32, start: f32, end: f32, segments: usize) -> Vec<Pos2> {
    (0..=segments)
        .map(|i| {
            let t = i as f32 / segments as f32;
            let angle = start + t * (end - start);
            Pos2::new(cx + r * angle.cos(), cy + r * angle.sin())
        })
        .collect()
}

/// Paint terrain at a specific zone within a single tile.
///
/// This is the Tiled-style approach: each click assigns terrain to exactly
/// one position on one tile, rather than affecting multiple tiles.
fn paint_terrain_zone(
    terrain_set: &mut bevy_map_autotile::TerrainSet,
    tile_index: u32,
    position: usize,
    terrain_idx: Option<usize>,
) {
    terrain_set.set_tile_terrain(tile_index, position, terrain_idx);
}

// ============================================================================
// Overlay Drawing - renders colored shapes at terrain positions
// ============================================================================

/// Draw all terrain overlays for a tile based on its TileTerrainData
fn draw_terrain_overlays(
    painter: &egui::Painter,
    rect: egui::Rect,
    set_type: TerrainSetType,
    terrain_data: &bevy_map_autotile::terrain::TileTerrainData,
    terrain_colors: &[Color32],
) {
    let position_count = set_type.position_count();

    // Draw edge/corner positions
    for pos in 0..position_count {
        if let Some(terrain_idx) = terrain_data.get(pos) {
            if let Some(&color) = terrain_colors.get(terrain_idx) {
                // Make color semi-transparent
                let overlay_color =
                    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 180);

                match set_type {
                    TerrainSetType::Corner => {
                        draw_corner_overlay(painter, rect, pos, overlay_color);
                    }
                    TerrainSetType::Edge => {
                        draw_edge_overlay(painter, rect, pos, overlay_color);
                    }
                    TerrainSetType::Mixed => {
                        draw_mixed_overlay(painter, rect, pos, overlay_color);
                    }
                }
            }
        }
    }

    // Draw center for Mixed sets (position 8) - visual marker only
    if set_type == TerrainSetType::Mixed {
        if let Some(terrain_idx) = terrain_data.get(8) {
            if let Some(&color) = terrain_colors.get(terrain_idx) {
                let overlay_color =
                    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 180);
                draw_mixed_overlay(painter, rect, 8, overlay_color);
            }
        }
    }
}

/// Draw a Tiled-style corner overlay with curved inner boundary.
/// Position: 0=TL, 1=TR, 2=BL, 3=BR
/// Uses d = 1/6 of tile size, matching Tiled's wangoverlay.cpp
fn draw_corner_overlay(painter: &egui::Painter, rect: egui::Rect, position: usize, color: Color32) {
    let w = rect.width();
    let d = w / 6.0; // Tiled uses 1/6 of tile size
    let arc_segments = 6;

    let points = match position {
        0 => corner_fill_tl(rect, d, arc_segments),
        1 => corner_fill_tr(rect, d, arc_segments),
        2 => corner_fill_bl(rect, d, arc_segments),
        3 => corner_fill_br(rect, d, arc_segments),
        _ => return,
    };

    painter.add(Shape::convex_polygon(points, color, egui::Stroke::NONE));
}

/// Generate filled region for top-left corner with concave arc boundary
fn corner_fill_tl(rect: egui::Rect, d: f32, segments: usize) -> Vec<Pos2> {
    let tl = rect.left_top();
    let mut pts = vec![tl, Pos2::new(tl.x + 2.0 * d, tl.y)];
    // Arc from (tl.x + 2d, tl.y) curving inward to (tl.x, tl.y + 2d)
    // Arc center at (tl.x + 2d, tl.y + 2d), radius 2d, from -PI/2 to -PI
    pts.extend(arc_points(
        tl.x + 2.0 * d,
        tl.y + 2.0 * d,
        2.0 * d,
        -PI / 2.0,
        -PI,
        segments,
    ));
    pts.push(Pos2::new(tl.x, tl.y + 2.0 * d));
    pts
}

/// Generate filled region for top-right corner with concave arc boundary
fn corner_fill_tr(rect: egui::Rect, d: f32, segments: usize) -> Vec<Pos2> {
    let tr = rect.right_top();
    let mut pts = vec![tr, Pos2::new(tr.x, tr.y + 2.0 * d)];
    // Arc from (tr.x, tr.y + 2d) curving inward to (tr.x - 2d, tr.y)
    // Arc center at (tr.x - 2d, tr.y + 2d), radius 2d, from 0 to -PI/2
    pts.extend(arc_points(
        tr.x - 2.0 * d,
        tr.y + 2.0 * d,
        2.0 * d,
        0.0,
        -PI / 2.0,
        segments,
    ));
    pts.push(Pos2::new(tr.x - 2.0 * d, tr.y));
    pts
}

/// Generate filled region for bottom-left corner with concave arc boundary
fn corner_fill_bl(rect: egui::Rect, d: f32, segments: usize) -> Vec<Pos2> {
    let bl = rect.left_bottom();
    let mut pts = vec![bl, Pos2::new(bl.x, bl.y - 2.0 * d)];
    // Arc from (bl.x, bl.y - 2d) curving inward to (bl.x + 2d, bl.y)
    // Arc center at (bl.x + 2d, bl.y - 2d), radius 2d, from PI to PI/2
    pts.extend(arc_points(
        bl.x + 2.0 * d,
        bl.y - 2.0 * d,
        2.0 * d,
        PI,
        PI / 2.0,
        segments,
    ));
    pts.push(Pos2::new(bl.x + 2.0 * d, bl.y));
    pts
}

/// Generate filled region for bottom-right corner with concave arc boundary
fn corner_fill_br(rect: egui::Rect, d: f32, segments: usize) -> Vec<Pos2> {
    let br = rect.right_bottom();
    let mut pts = vec![br, Pos2::new(br.x - 2.0 * d, br.y)];
    // Arc from (br.x - 2d, br.y) curving inward to (br.x, br.y - 2d)
    // Arc center at (br.x - 2d, br.y - 2d), radius 2d, from PI/2 to 0
    pts.extend(arc_points(
        br.x - 2.0 * d,
        br.y - 2.0 * d,
        2.0 * d,
        PI / 2.0,
        0.0,
        segments,
    ));
    pts.push(Pos2::new(br.x, br.y - 2.0 * d));
    pts
}

/// Draw a Tiled-style edge overlay (strip along edge with rounded ends).
/// Position: 0=Top, 1=Right, 2=Bottom, 3=Left
/// Uses d = 1/6 of tile size, matching Tiled's wangoverlay.cpp
fn draw_edge_overlay(painter: &egui::Painter, rect: egui::Rect, position: usize, color: Color32) {
    let w = rect.width();
    let d = w / 6.0; // Tiled uses 1/6 of tile size
    let arc_segments = 4;

    let points = match position {
        0 => edge_strip_top(rect, d, arc_segments),
        1 => edge_strip_right(rect, d, arc_segments),
        2 => edge_strip_bottom(rect, d, arc_segments),
        3 => edge_strip_left(rect, d, arc_segments),
        _ => return,
    };

    painter.add(Shape::convex_polygon(points, color, egui::Stroke::NONE));
}

/// Generate edge strip for top edge
fn edge_strip_top(rect: egui::Rect, d: f32, segments: usize) -> Vec<Pos2> {
    let left = rect.left() + 2.0 * d;
    let right = rect.right() - 2.0 * d;
    let top = rect.top();

    let mut pts = Vec::new();
    // Start at top-left of strip
    pts.push(Pos2::new(left, top));
    // Go to top-right
    pts.push(Pos2::new(right, top));
    // Arc at right end (semicircle going down and back)
    pts.extend(arc_points(right, top + d, d, -PI / 2.0, PI / 2.0, segments));
    // Go back left along bottom
    pts.push(Pos2::new(left, top + 2.0 * d));
    // Arc at left end (semicircle going up and back)
    pts.extend(arc_points(
        left,
        top + d,
        d,
        PI / 2.0,
        3.0 * PI / 2.0,
        segments,
    ));
    pts
}

/// Generate edge strip for right edge
fn edge_strip_right(rect: egui::Rect, d: f32, segments: usize) -> Vec<Pos2> {
    let top = rect.top() + 2.0 * d;
    let bottom = rect.bottom() - 2.0 * d;
    let right = rect.right();

    let mut pts = Vec::new();
    pts.push(Pos2::new(right, top));
    pts.push(Pos2::new(right, bottom));
    // Arc at bottom (semicircle going left and back)
    pts.extend(arc_points(right - d, bottom, d, 0.0, PI, segments));
    pts.push(Pos2::new(right - 2.0 * d, top));
    // Arc at top (semicircle going right and back)
    pts.extend(arc_points(right - d, top, d, PI, 2.0 * PI, segments));
    pts
}

/// Generate edge strip for bottom edge
fn edge_strip_bottom(rect: egui::Rect, d: f32, segments: usize) -> Vec<Pos2> {
    let left = rect.left() + 2.0 * d;
    let right = rect.right() - 2.0 * d;
    let bottom = rect.bottom();

    let mut pts = Vec::new();
    pts.push(Pos2::new(left, bottom));
    pts.push(Pos2::new(right, bottom));
    // Arc at right end (semicircle going up and back)
    pts.extend(arc_points(
        right,
        bottom - d,
        d,
        PI / 2.0,
        -PI / 2.0,
        segments,
    ));
    pts.push(Pos2::new(left, bottom - 2.0 * d));
    // Arc at left end (semicircle going down and back)
    pts.extend(arc_points(
        left,
        bottom - d,
        d,
        -PI / 2.0,
        -3.0 * PI / 2.0,
        segments,
    ));
    pts
}

/// Generate edge strip for left edge
fn edge_strip_left(rect: egui::Rect, d: f32, segments: usize) -> Vec<Pos2> {
    let top = rect.top() + 2.0 * d;
    let bottom = rect.bottom() - 2.0 * d;
    let left = rect.left();

    let mut pts = Vec::new();
    pts.push(Pos2::new(left, top));
    pts.push(Pos2::new(left, bottom));
    // Arc at bottom (semicircle going right and back)
    pts.extend(arc_points(left + d, bottom, d, PI, 0.0, segments));
    pts.push(Pos2::new(left + 2.0 * d, top));
    // Arc at top (semicircle going left and back)
    pts.extend(arc_points(left + d, top, d, 0.0, -PI, segments));
    pts
}

/// Draw a mixed overlay (for Mixed type terrain sets - 3x3 grid)
/// Position: 0=TL, 1=Top, 2=TR, 3=Right, 4=BR, 5=Bottom, 6=BL, 7=Left
fn draw_mixed_overlay(painter: &egui::Painter, rect: egui::Rect, position: usize, color: Color32) {
    let w = rect.width() / 3.0;
    let h = rect.height() / 3.0;
    let left = rect.left();
    let top = rect.top();

    // Calculate cell position based on 3x3 grid layout
    let cell_rect = match position {
        0 => egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(w, h)), // TL
        1 => egui::Rect::from_min_size(egui::pos2(left + w, top), egui::vec2(w, h)), // Top
        2 => egui::Rect::from_min_size(egui::pos2(left + 2.0 * w, top), egui::vec2(w, h)), // TR
        3 => egui::Rect::from_min_size(egui::pos2(left + 2.0 * w, top + h), egui::vec2(w, h)), // Right
        4 => egui::Rect::from_min_size(egui::pos2(left + 2.0 * w, top + 2.0 * h), egui::vec2(w, h)), // BR
        5 => egui::Rect::from_min_size(egui::pos2(left + w, top + 2.0 * h), egui::vec2(w, h)), // Bottom
        6 => egui::Rect::from_min_size(egui::pos2(left, top + 2.0 * h), egui::vec2(w, h)),     // BL
        7 => egui::Rect::from_min_size(egui::pos2(left, top + h), egui::vec2(w, h)), // Left
        8 => egui::Rect::from_min_size(egui::pos2(left + w, top + h), egui::vec2(w, h)), // Center
        _ => return,
    };

    painter.rect_filled(cell_rect, 0.0, color);
}

/// Draw hover highlight for the currently hovered zone within a tile.
/// This gives visual feedback showing which zone will be affected on click.
fn draw_zone_hover_highlight(
    painter: &egui::Painter,
    rect: egui::Rect,
    position: usize,
    set_type: TerrainSetType,
) {
    let hover_color = Color32::from_rgba_unmultiplied(255, 255, 100, 100);

    match set_type {
        TerrainSetType::Corner => {
            draw_corner_overlay(painter, rect, position, hover_color);
        }
        TerrainSetType::Edge => {
            draw_edge_overlay(painter, rect, position, hover_color);
        }
        TerrainSetType::Mixed => {
            draw_mixed_overlay(painter, rect, position, hover_color);
        }
    }
}

#[derive(Default, PartialEq)]
pub enum TilesetEditorTab {
    #[default]
    Images,
    TerrainSets,
    TileProperties,
    Collision,
}

// ============================================================================
// Collision Editor State & Types
// ============================================================================

/// Drawing modes for the collision editor
#[derive(Default, PartialEq, Clone, Copy)]
pub enum CollisionDrawMode {
    #[default]
    Select,
    Rectangle,
    Circle,
    Polygon,
}

/// Corner identifiers for rectangle handles
#[derive(Clone, Copy, PartialEq)]
pub enum RectCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Types of drag operations in the collision editor
#[derive(Clone)]
pub enum CollisionDragOperation {
    /// Creating a new rectangle (drag from corner to corner)
    NewRectangle,
    /// Creating a new circle (center set, dragging radius)
    NewCircle { center: [f32; 2] },
    /// Moving the entire shape
    MoveShape { original_offset: [f32; 2] },
    /// Resizing a rectangle via a corner handle
    ResizeRect {
        corner: RectCorner,
        original: ([f32; 2], [f32; 2]),
    },
    /// Resizing a circle via the edge handle
    ResizeCircle { original_radius: f32 },
    /// Moving a polygon vertex
    MoveVertex { index: usize, original: [f32; 2] },
}

/// Drag state for collision shape editing
pub struct CollisionDragState {
    pub operation: CollisionDragOperation,
    pub start_pos: [f32; 2],
    pub current_pos: [f32; 2],
}

/// State for the collision editor within the tileset editor
pub struct CollisionEditorState {
    /// Currently selected tile for collision editing
    pub selected_tile: Option<u32>,
    /// Zoom level for the tile preview canvas (default 8.0 = 256px for 32px tiles)
    pub preview_zoom: f32,
    /// Zoom level for the tile selector grid
    pub grid_zoom: f32,
    /// Current drawing mode
    pub drawing_mode: CollisionDrawMode,
    /// Drag state for shape manipulation
    pub drag_state: Option<CollisionDragState>,
    /// Points being drawn for an incomplete polygon
    pub polygon_points: Vec<[f32; 2]>,
    /// Context menu position (normalized coords) - Some when menu is open
    pub context_menu_pos: Option<[f32; 2]>,
    /// Vertex index if context menu was opened on a vertex
    pub context_menu_vertex: Option<usize>,
}

impl Default for CollisionEditorState {
    fn default() -> Self {
        Self {
            selected_tile: None,
            preview_zoom: 8.0,
            grid_zoom: 1.0,
            drawing_mode: CollisionDrawMode::Select,
            drag_state: None,
            polygon_points: Vec::new(),
            context_menu_pos: None,
            context_menu_vertex: None,
        }
    }
}

/// Render the tileset editor window
pub fn render_tileset_editor(
    ctx: &egui::Context,
    editor_state: &mut EditorState,
    project: &mut Project,
    cache: Option<&TilesetTextureCache>,
) {
    if !editor_state.show_tileset_editor {
        return;
    }

    let mut is_open = editor_state.show_tileset_editor;
    egui::Window::new("Tileset Editor")
        .open(&mut is_open)
        .collapsible(true)
        .resizable(true)
        .default_size([800.0, 600.0])
        .min_size([600.0, 400.0])
        .show(ctx, |ui| {
            // Tileset selector
            let current_tileset_name = editor_state
                .selected_tileset
                .and_then(|id| project.tilesets.iter().find(|t| t.id == id))
                .map(|t| t.name.as_str())
                .unwrap_or("(none)");

            ui.horizontal(|ui| {
                ui.label("Tileset:");
                egui::ComboBox::from_id_salt("tileset_editor_selector")
                    .selected_text(current_tileset_name)
                    .show_ui(ui, |ui| {
                        for tileset in &project.tilesets {
                            ui.selectable_value(
                                &mut editor_state.selected_tileset,
                                Some(tileset.id),
                                &tileset.name,
                            );
                        }
                    });
            });

            ui.separator();

            // Tab bar
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(
                        editor_state.tileset_editor_state.selected_tab == TilesetEditorTab::Images,
                        "Images",
                    )
                    .clicked()
                {
                    editor_state.tileset_editor_state.selected_tab = TilesetEditorTab::Images;
                }
                if ui
                    .selectable_label(
                        editor_state.tileset_editor_state.selected_tab
                            == TilesetEditorTab::TerrainSets,
                        "Terrain Sets",
                    )
                    .clicked()
                {
                    editor_state.tileset_editor_state.selected_tab = TilesetEditorTab::TerrainSets;
                }
                if ui
                    .selectable_label(
                        editor_state.tileset_editor_state.selected_tab
                            == TilesetEditorTab::TileProperties,
                        "Tile Properties",
                    )
                    .clicked()
                {
                    editor_state.tileset_editor_state.selected_tab =
                        TilesetEditorTab::TileProperties;
                }
                if ui
                    .selectable_label(
                        editor_state.tileset_editor_state.selected_tab
                            == TilesetEditorTab::Collision,
                        "Collision",
                    )
                    .clicked()
                {
                    editor_state.tileset_editor_state.selected_tab = TilesetEditorTab::Collision;
                }
            });

            ui.separator();

            // Tab content
            match editor_state.tileset_editor_state.selected_tab {
                TilesetEditorTab::Images => {
                    render_images_tab(ui, editor_state, project, cache);
                }
                TilesetEditorTab::TerrainSets => {
                    render_terrain_sets_tab(ui, editor_state, project, cache);
                }
                TilesetEditorTab::TileProperties => {
                    render_tile_properties_tab(ui, editor_state, project, cache);
                }
                TilesetEditorTab::Collision => {
                    render_collision_tab(ui, editor_state, project, cache);
                }
            }
        });

    // Update the editor state if the window was closed via the title bar button
    editor_state.show_tileset_editor = is_open;
}

fn render_images_tab(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    project: &mut Project,
    cache: Option<&TilesetTextureCache>,
) {
    let Some(tileset_id) = editor_state.selected_tileset else {
        ui.label("No tileset selected");
        return;
    };

    let Some(tileset) = project.tilesets.iter_mut().find(|t| t.id == tileset_id) else {
        ui.label("Tileset not found");
        return;
    };

    ui.heading("Images");
    ui.separator();

    // List images with previews
    for (idx, image) in tileset.images.iter().enumerate() {
        ui.horizontal(|ui| {
            // Image preview thumbnail
            if let Some(cache) = cache {
                if let Some((_, texture_id, width, height)) = cache.loaded.get(&image.id) {
                    // Scale to fit a reasonable preview size (max 64px)
                    let max_size = 64.0;
                    let scale = (max_size / width.max(*height)).min(1.0);
                    let preview_size = egui::vec2(width * scale, height * scale);

                    ui.image(egui::load::SizedTexture::new(*texture_id, preview_size));
                } else {
                    // Placeholder while loading
                    ui.add_sized([64.0, 64.0], egui::Label::new("Loading..."));
                }
            }

            // Image info (name + dimensions)
            ui.vertical(|ui| {
                let selected = editor_state.tileset_editor_state.selected_image_idx == Some(idx);
                if ui.selectable_label(selected, &image.name).clicked() {
                    editor_state.tileset_editor_state.selected_image_idx = Some(idx);
                }
                ui.small(format!("{}x{} tiles", image.columns, image.rows));
            });
        });
        ui.add_space(4.0);
    }

    ui.separator();

    if ui.button("Add Image...").clicked() {
        editor_state.show_add_tileset_image_dialog = true;
    }
}

fn render_terrain_sets_tab(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    project: &mut Project,
    cache: Option<&TilesetTextureCache>,
) {
    let Some(tileset_id) = editor_state.selected_tileset else {
        ui.label("No tileset selected");
        return;
    };

    // Clone tileset data to avoid borrow conflicts
    let tileset_data = project
        .tilesets
        .iter()
        .find(|t| t.id == tileset_id)
        .map(|t| (t.tile_size, t.images.clone(), !t.images.is_empty()));

    // Split into left panel (terrain list) and right panel (tileset preview)
    // Using resizable SidePanel for better UX
    egui::SidePanel::left("terrain_list_panel")
        .resizable(true)
        .default_width(250.0)
        .show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.heading("Terrain Sets");

                    // List terrain sets for this tileset
                    let terrain_sets: Vec<_> = project
                        .autotile_config
                        .terrain_sets
                        .iter()
                        .filter(|ts| ts.tileset_id == tileset_id)
                        .map(|ts| (ts.id, ts.name.clone(), ts.set_type, ts.terrains.len()))
                        .collect();

                    for (ts_id, name, set_type, terrain_count) in &terrain_sets {
                        ui.horizontal(|ui| {
                            let selected = editor_state.selected_terrain_set == Some(*ts_id);
                            if ui.selectable_label(selected, name).clicked() {
                                editor_state.selected_terrain_set = Some(*ts_id);
                                editor_state
                                    .tileset_editor_state
                                    .selected_terrain_for_assignment = None;
                            }
                            ui.small(format!("{:?} ({})", set_type, terrain_count));
                        });
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("+ New Set").clicked() {
                            editor_state.show_new_terrain_set_dialog = true;
                        }
                    });

                    // If a terrain set is selected, show its terrains
                    if let Some(ts_id) = editor_state.selected_terrain_set {
                        if let Some(terrain_set) = project.autotile_config.get_terrain_set(ts_id) {
                            ui.separator();
                            ui.heading("Terrains");

                            // Terrain list with selection
                            for (idx, terrain) in terrain_set.terrains.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    let color = terrain_color_to_egui(&terrain.color);
                                    let selected = editor_state
                                        .tileset_editor_state
                                        .selected_terrain_for_assignment
                                        == Some(idx);

                                    // Color swatch
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(16.0, 16.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(rect, 2.0, color);

                                    // Terrain name with selection
                                    if ui.selectable_label(selected, &terrain.name).clicked() {
                                        editor_state
                                            .tileset_editor_state
                                            .selected_terrain_for_assignment = Some(idx);
                                    }
                                });
                            }

                            // Buttons
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("+ Add Terrain").clicked() {
                                    editor_state.show_add_terrain_to_set_dialog = true;
                                }
                                if editor_state
                                    .tileset_editor_state
                                    .selected_terrain_for_assignment
                                    .is_some()
                                {
                                    if ui.button("Clear Selection").clicked() {
                                        editor_state
                                            .tileset_editor_state
                                            .selected_terrain_for_assignment = None;
                                    }
                                }
                            });

                            // Instructions based on terrain set type
                            ui.separator();
                            ui.small("Click on tile zones to paint terrain.");
                            ui.small("Ctrl+click to clear a position.");
                            ui.add_space(4.0);
                            match terrain_set.set_type {
                                TerrainSetType::Corner => {
                                    ui.small("Corner mode: 4 zones per tile (corners)");
                                }
                                TerrainSetType::Edge => {
                                    ui.small("Edge mode: 4 zones per tile (edges)");
                                }
                                TerrainSetType::Mixed => {
                                    ui.small("Mixed mode: 8 zones (4 corners + 4 edges)");
                                }
                            }
                        }
                    }
                });
        });

    // Right side: Tileset preview with terrain overlays (fills remaining space)
    egui::CentralPanel::default().show_inside(ui, |ui| {
        // Header with zoom slider
        ui.horizontal(|ui| {
            ui.heading("Tile Assignments");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    egui::Slider::new(
                        &mut editor_state.tileset_editor_state.terrain_tile_zoom,
                        0.5..=3.0,
                    )
                    .text("Zoom")
                    .step_by(0.25),
                );
            });
        });

        if let Some((tile_size, images, has_images)) = tileset_data.clone() {
            if !has_images {
                ui.label("No images in tileset");
            } else {
                // Use ScrollArea with available height
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        render_tileset_with_terrain_overlay(
                            ui,
                            editor_state,
                            project,
                            tileset_id,
                            tile_size,
                            &images,
                            cache,
                        );
                    });
            }
        }
    });
}

/// Convert terrain color to egui Color32
fn terrain_color_to_egui(color: &TerrainColor) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        (color.a * 200.0) as u8, // Slightly transparent
    )
}

/// Render the tileset with Tiled-style per-tile zone painting.
/// Users click on zones within each tile to assign terrain to that tile's corner/edge/mixed position.
fn render_tileset_with_terrain_overlay(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    project: &mut Project,
    tileset_id: uuid::Uuid,
    tile_size: u32,
    images: &[bevy_map_core::TilesetImage],
    cache: Option<&TilesetTextureCache>,
) {
    // Display size based on zoom level (base 32px * zoom)
    let tile_display_size = 32.0 * editor_state.tileset_editor_state.terrain_tile_zoom;
    let display_size = egui::vec2(tile_display_size, tile_display_size);
    let mut virtual_offset = 0u32;

    // Get terrain set info for overlays
    let terrain_info = editor_state
        .selected_terrain_set
        .and_then(|ts_id| project.autotile_config.get_terrain_set(ts_id))
        .map(|ts| {
            let colors: Vec<Color32> = ts
                .terrains
                .iter()
                .map(|t| terrain_color_to_egui(&t.color))
                .collect();
            (ts.set_type, colors)
        });

    let (set_type, terrain_colors) = match terrain_info {
        Some((st, tc)) => (Some(st), tc),
        None => (None, Vec::new()),
    };

    for image in images {
        // Get texture for this image
        let texture_id = cache
            .and_then(|c| c.loaded.get(&image.id))
            .map(|(_, tex_id, _, _)| *tex_id);

        let image_virtual_offset = virtual_offset;

        ui.collapsing(&image.name, |ui| {
            if image.columns == 0 || image.rows == 0 {
                ui.label("Image not loaded yet");
                return;
            }

            let uv_tile_width = 1.0 / image.columns.max(1) as f32;
            let uv_tile_height = 1.0 / image.rows.max(1) as f32;

            // Calculate full grid size (with spacing)
            let spacing = 2.0f32;
            let grid_width =
                image.columns as f32 * tile_display_size + (image.columns - 1) as f32 * spacing;
            let grid_height =
                image.rows as f32 * tile_display_size + (image.rows - 1) as f32 * spacing;

            // Allocate the entire grid area for interaction
            let (grid_rect, grid_response) = ui.allocate_exact_size(
                egui::vec2(grid_width, grid_height),
                egui::Sense::click_and_drag(),
            );

            let grid_origin = grid_rect.left_top();

            // Draw all tiles
            for row in 0..image.rows {
                for col in 0..image.columns {
                    let local_index = row * image.columns + col;
                    let virtual_index = image_virtual_offset + local_index;

                    // Calculate tile rect (accounting for spacing)
                    let tile_x = grid_origin.x + col as f32 * (tile_display_size + spacing);
                    let tile_y = grid_origin.y + row as f32 * (tile_display_size + spacing);
                    let rect = egui::Rect::from_min_size(egui::pos2(tile_x, tile_y), display_size);

                    // Draw tile texture
                    if let Some(tex_id) = texture_id {
                        let uv_min =
                            egui::pos2(col as f32 * uv_tile_width, row as f32 * uv_tile_height);
                        let uv_max = egui::pos2(
                            (col + 1) as f32 * uv_tile_width,
                            (row + 1) as f32 * uv_tile_height,
                        );

                        // Draw texture using mesh
                        let mut mesh = egui::Mesh::with_texture(tex_id);
                        mesh.add_rect_with_uv(
                            rect,
                            egui::Rect::from_min_max(uv_min, uv_max),
                            Color32::WHITE,
                        );
                        ui.painter().add(Shape::mesh(mesh));
                    } else {
                        // Fallback: draw placeholder
                        ui.painter().rect_filled(rect, 0.0, Color32::from_gray(60));
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("{}", virtual_index),
                            egui::FontId::default(),
                            Color32::WHITE,
                        );
                    }

                    // Draw terrain overlays at all assigned positions
                    if let Some(st) = set_type {
                        if let Some(terrain_data) = editor_state
                            .selected_terrain_set
                            .and_then(|ts_id| project.autotile_config.get_terrain_set(ts_id))
                            .and_then(|ts| ts.get_tile_terrain(virtual_index))
                        {
                            draw_terrain_overlays(
                                ui.painter(),
                                rect,
                                st,
                                terrain_data,
                                &terrain_colors,
                            );
                        }
                    }

                    // Draw thin border to show tile boundaries
                    ui.painter().rect_stroke(
                        rect,
                        0.0,
                        egui::Stroke::new(1.0, Color32::from_gray(80)),
                        egui::StrokeKind::Inside,
                    );

                    // Draw origin indicator if tile has custom origin
                    if let Some(tileset) = project.tilesets.iter().find(|t| t.id == tileset_id) {
                        if let Some(props) = tileset.get_tile_properties(virtual_index) {
                            if props.origin_x.is_some() || props.origin_y.is_some() {
                                let tile_pixel_size = tile_size as f32;
                                let (ox, oy) = props.get_origin(tile_size, tile_size);

                                // Scale origin to display size
                                let scale = tile_display_size / tile_pixel_size;
                                let origin_screen_x = rect.left() + ox as f32 * scale;
                                let origin_screen_y = rect.top() + oy as f32 * scale;

                                // Draw small red dot
                                let dot_radius = 2.0 * (tile_display_size / 32.0).max(1.0);
                                ui.painter().circle_filled(
                                    egui::pos2(origin_screen_x, origin_screen_y),
                                    dot_radius,
                                    Color32::from_rgb(255, 100, 100),
                                );
                            }
                        }
                    }
                }
            }

            // Handle zone-based interaction (Tiled-style per-tile zones)
            if let Some(st) = set_type {
                let effective_tile_size = tile_display_size + spacing;

                // Helper to convert pointer position to tile + local coordinates
                let get_tile_and_local = |pos: egui::Pos2| -> Option<(u32, f32, f32, egui::Rect)> {
                    if !grid_rect.contains(pos) {
                        return None;
                    }
                    let rel_x = pos.x - grid_origin.x;
                    let rel_y = pos.y - grid_origin.y;

                    // Find which tile cell (accounting for spacing)
                    let col = (rel_x / effective_tile_size).floor() as i32;
                    let row = (rel_y / effective_tile_size).floor() as i32;

                    if col < 0 || col >= image.columns as i32 || row < 0 || row >= image.rows as i32
                    {
                        return None;
                    }

                    let col = col as u32;
                    let row = row as u32;

                    // Calculate tile rect
                    let tile_x = grid_origin.x + col as f32 * effective_tile_size;
                    let tile_y = grid_origin.y + row as f32 * effective_tile_size;
                    let tile_rect = egui::Rect::from_min_size(
                        egui::pos2(tile_x, tile_y),
                        egui::vec2(tile_display_size, tile_display_size),
                    );

                    // Only consider clicks within the actual tile (not spacing)
                    if !tile_rect.contains(pos) {
                        return None;
                    }

                    // Calculate local coordinates (0.0 to 1.0) within tile
                    let local_x = (pos.x - tile_rect.left()) / tile_display_size;
                    let local_y = (pos.y - tile_rect.top()) / tile_display_size;

                    let tile_index = image_virtual_offset + row * image.columns + col;
                    Some((tile_index, local_x, local_y, tile_rect))
                };

                // Draw hover highlight on the zone under cursor
                if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                    if let Some((tile_index, local_x, local_y, tile_rect)) =
                        get_tile_and_local(hover_pos)
                    {
                        if let Some(position) = get_tile_zone(local_x, local_y, st) {
                            // Draw hover highlight
                            draw_zone_hover_highlight(ui.painter(), tile_rect, position, st);

                            // Show tooltip
                            let zone_name = match st {
                                TerrainSetType::Corner => match position {
                                    0 => "Top-Left",
                                    1 => "Top-Right",
                                    2 => "Bottom-Left",
                                    3 => "Bottom-Right",
                                    _ => "Unknown",
                                },
                                TerrainSetType::Edge => match position {
                                    0 => "Top",
                                    1 => "Right",
                                    2 => "Bottom",
                                    3 => "Left",
                                    _ => "Unknown",
                                },
                                TerrainSetType::Mixed => match position {
                                    0 => "Top-Left",
                                    1 => "Top",
                                    2 => "Top-Right",
                                    3 => "Right",
                                    4 => "Bottom-Right",
                                    5 => "Bottom",
                                    6 => "Bottom-Left",
                                    7 => "Left",
                                    8 => "Center",
                                    _ => "Unknown",
                                },
                            };
                            let tooltip = format!(
                                "Tile {} - {} zone\nClick to paint\nCtrl+click to clear",
                                tile_index, zone_name
                            );
                            grid_response.clone().on_hover_text(tooltip);
                        }
                    }
                }

                // Handle clicks - paint to single tile's zone
                if grid_response.clicked() || grid_response.dragged() {
                    if let Some(click_pos) = grid_response.interact_pointer_pos() {
                        if let Some((tile_index, local_x, local_y, _tile_rect)) =
                            get_tile_and_local(click_pos)
                        {
                            if let Some(position) = get_tile_zone(local_x, local_y, st) {
                                let ctrl_held = ui.input(|i| i.modifiers.ctrl);
                                let terrain_idx = if ctrl_held {
                                    None // Erase
                                } else {
                                    editor_state
                                        .tileset_editor_state
                                        .selected_terrain_for_assignment
                                };

                                // Paint to this tile's zone only
                                if let Some(ts_id) = editor_state.selected_terrain_set {
                                    if let Some(terrain_set) =
                                        project.autotile_config.get_terrain_set_mut(ts_id)
                                    {
                                        paint_terrain_zone(
                                            terrain_set,
                                            tile_index,
                                            position,
                                            terrain_idx,
                                        );
                                        project.mark_dirty();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        virtual_offset += image.tile_count();
    }
}

fn render_tile_properties_tab(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    project: &mut Project,
    cache: Option<&TilesetTextureCache>,
) {
    let Some(tileset_id) = editor_state.selected_tileset else {
        ui.label("No tileset selected");
        return;
    };

    // Clone tileset data to avoid borrow conflicts
    let tileset_data = project
        .tilesets
        .iter()
        .find(|t| t.id == tileset_id)
        .map(|t| (t.tile_size, t.images.clone(), !t.images.is_empty()));

    // Left panel: Tile selector (resizable)
    egui::SidePanel::left("tile_properties_selector")
        .resizable(true)
        .default_width(250.0)
        .show_inside(ui, |ui| {
            ui.heading("Select Tile");

            if let Some((tile_size, images, has_images)) = tileset_data.clone() {
                if !has_images {
                    ui.label("No images in tileset");
                } else {
                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            render_tile_selector_for_properties(
                                ui,
                                editor_state,
                                project,
                                tileset_id,
                                tile_size,
                                &images,
                                cache,
                            );
                        });
                }
            }
        });

    // Right panel: Property editor (fills remaining space)
    egui::CentralPanel::default().show_inside(ui, |ui| {
        ui.heading("Tile Properties");

        if let Some(tile_idx) = editor_state
            .tileset_editor_state
            .selected_tile_for_properties
        {
            ui.label(format!("Editing Tile {}", tile_idx));
            ui.separator();

            // Get current properties (or default)
            let tileset = project.tilesets.iter().find(|t| t.id == tileset_id);
            let current_props = tileset
                .and_then(|t| t.get_tile_properties(tile_idx))
                .cloned()
                .unwrap_or_default();

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Collision settings
                    ui.heading("Collision");

                    let collision_data = current_props.collision.clone();
                    let mut has_collision = collision_data.has_collision();

                    if ui.checkbox(&mut has_collision, "Has collision").changed() {
                        if let Some(tileset) =
                            project.tilesets.iter_mut().find(|t| t.id == tileset_id)
                        {
                            tileset.set_tile_full_collision(tile_idx, has_collision);
                            project.mark_dirty();
                        }
                    }

                    ui.add_enabled_ui(has_collision, |ui| {
                        // Shape selector
                        let shape_name = collision_data.shape.name();
                        egui::ComboBox::from_label("Shape")
                            .selected_text(shape_name)
                            .show_ui(ui, |ui| {
                                let shapes = [
                                    ("Full", bevy_map_core::CollisionShape::Full),
                                    (
                                        "Rectangle",
                                        bevy_map_core::CollisionShape::Rectangle {
                                            offset: [0.0, 0.0],
                                            size: [1.0, 1.0],
                                        },
                                    ),
                                    (
                                        "Circle",
                                        bevy_map_core::CollisionShape::Circle {
                                            offset: [0.0, 0.0],
                                            radius: 0.5,
                                        },
                                    ),
                                ];
                                for (name, shape) in shapes {
                                    if ui.selectable_label(shape_name == name, name).clicked() {
                                        if let Some(tileset) =
                                            project.tilesets.iter_mut().find(|t| t.id == tileset_id)
                                        {
                                            let props = tileset.get_tile_properties_mut(tile_idx);
                                            props.collision.shape = shape;
                                            project.mark_dirty();
                                        }
                                    }
                                }
                            });

                        // One-way direction
                        let one_way_name = collision_data.one_way.name();
                        egui::ComboBox::from_label("One-way")
                            .selected_text(one_way_name)
                            .show_ui(ui, |ui| {
                                let directions = [
                                    bevy_map_core::OneWayDirection::None,
                                    bevy_map_core::OneWayDirection::Top,
                                    bevy_map_core::OneWayDirection::Bottom,
                                    bevy_map_core::OneWayDirection::Left,
                                    bevy_map_core::OneWayDirection::Right,
                                ];
                                for dir in directions {
                                    if ui
                                        .selectable_label(collision_data.one_way == dir, dir.name())
                                        .clicked()
                                    {
                                        if let Some(tileset) =
                                            project.tilesets.iter_mut().find(|t| t.id == tileset_id)
                                        {
                                            let props = tileset.get_tile_properties_mut(tile_idx);
                                            props.collision.one_way = dir;
                                            project.mark_dirty();
                                        }
                                    }
                                }
                            });

                        // Collision layer
                        let mut layer = collision_data.layer;
                        ui.horizontal(|ui| {
                            ui.label("Layer:");
                            if ui
                                .add(egui::DragValue::new(&mut layer).range(0..=31))
                                .changed()
                            {
                                if let Some(tileset) =
                                    project.tilesets.iter_mut().find(|t| t.id == tileset_id)
                                {
                                    let props = tileset.get_tile_properties_mut(tile_idx);
                                    props.collision.layer = layer;
                                    project.mark_dirty();
                                }
                            }
                        });
                    });

                    ui.separator();

                    // Grid size settings (for multi-cell tiles like trees)
                    ui.heading("Grid Size");
                    ui.small("For multi-cell tiles that span multiple grid cells");

                    let mut grid_width = current_props.grid_width;
                    let mut grid_height = current_props.grid_height;

                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        if ui
                            .add(egui::DragValue::new(&mut grid_width).range(1..=8))
                            .changed()
                        {
                            if let Some(tileset) =
                                project.tilesets.iter_mut().find(|t| t.id == tileset_id)
                            {
                                tileset.set_tile_grid_size(tile_idx, grid_width, grid_height);
                                project.mark_dirty();
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Height:");
                        if ui
                            .add(egui::DragValue::new(&mut grid_height).range(1..=8))
                            .changed()
                        {
                            if let Some(tileset) =
                                project.tilesets.iter_mut().find(|t| t.id == tileset_id)
                            {
                                tileset.set_tile_grid_size(tile_idx, grid_width, grid_height);
                                project.mark_dirty();
                            }
                        }
                    });

                    if grid_width > 1 || grid_height > 1 {
                        ui.label(format!(
                            "This tile spans {}x{} grid cells",
                            grid_width, grid_height
                        ));
                    }

                    ui.separator();

                    // Origin point editor (for multi-cell tiles)
                    ui.heading("Origin Point");
                    ui.small("Click and drag to set where the tile anchors when placed");

                    // Get tileset info for rendering
                    if let Some(tileset) = project.tilesets.iter().find(|t| t.id == tileset_id) {
                        let tile_size = tileset.tile_size;

                        // Calculate tile dimensions in pixels
                        let tile_pixel_width = current_props.grid_width * tile_size;
                        let tile_pixel_height = current_props.grid_height * tile_size;

                        // Current origin (default to center)
                        let origin_x = current_props.origin_x.unwrap_or(tile_pixel_width / 2);
                        let origin_y = current_props.origin_y.unwrap_or(tile_pixel_height / 2);

                        // Display scale (fit in reasonable size, max 200px)
                        let max_display = 200.0;
                        let scale = (max_display / tile_pixel_width.max(tile_pixel_height) as f32)
                            .clamp(0.5, 2.0);
                        let display_width = tile_pixel_width as f32 * scale;
                        let display_height = tile_pixel_height as f32 * scale;

                        // Allocate interactive area for tile preview
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(display_width, display_height),
                            egui::Sense::click_and_drag(),
                        );

                        // Try to draw tile texture preview
                        if let Some((image_index, local_idx)) = tileset.virtual_to_local(tile_idx) {
                            if let Some(image) = tileset.images.get(image_index) {
                                if let Some(tex_id) = cache
                                    .and_then(|c| c.loaded.get(&image.id))
                                    .map(|(_, tex_id, _, _)| *tex_id)
                                {
                                    // Calculate UV coordinates for this tile
                                    let tile_col = local_idx % image.columns;
                                    let tile_row = local_idx / image.columns;
                                    let uv_tile_width = 1.0 / image.columns as f32;
                                    let uv_tile_height = 1.0 / image.rows as f32;

                                    let uv_min = egui::pos2(
                                        tile_col as f32 * uv_tile_width,
                                        tile_row as f32 * uv_tile_height,
                                    );
                                    let uv_max = egui::pos2(
                                        (tile_col + current_props.grid_width) as f32
                                            * uv_tile_width,
                                        (tile_row + current_props.grid_height) as f32
                                            * uv_tile_height,
                                    );

                                    // Draw tile texture
                                    let mut mesh = egui::Mesh::with_texture(tex_id);
                                    mesh.add_rect_with_uv(
                                        rect,
                                        egui::Rect::from_min_max(uv_min, uv_max),
                                        Color32::WHITE,
                                    );
                                    ui.painter().add(egui::Shape::mesh(mesh));
                                }
                            }
                        }

                        // Draw border around preview
                        ui.painter().rect_stroke(
                            rect,
                            0.0,
                            egui::Stroke::new(1.0, Color32::from_gray(100)),
                            egui::StrokeKind::Inside,
                        );

                        // Draw origin point marker (crosshair)
                        let origin_screen_x = rect.left() + origin_x as f32 * scale;
                        let origin_screen_y = rect.top() + origin_y as f32 * scale;
                        let origin_pos = egui::pos2(origin_screen_x, origin_screen_y);

                        // Crosshair lines
                        let crosshair_size = 10.0;
                        ui.painter().line_segment(
                            [
                                origin_pos - egui::vec2(crosshair_size, 0.0),
                                origin_pos + egui::vec2(crosshair_size, 0.0),
                            ],
                            egui::Stroke::new(2.0, Color32::RED),
                        );
                        ui.painter().line_segment(
                            [
                                origin_pos - egui::vec2(0.0, crosshair_size),
                                origin_pos + egui::vec2(0.0, crosshair_size),
                            ],
                            egui::Stroke::new(2.0, Color32::RED),
                        );
                        // Center dot
                        ui.painter().circle_filled(origin_pos, 4.0, Color32::RED);

                        // Handle drag to move origin
                        if response.dragged() || response.clicked() {
                            if let Some(pointer_pos) = response.interact_pointer_pos() {
                                // Convert screen position to tile pixel coordinates
                                let new_x = ((pointer_pos.x - rect.left()) / scale)
                                    .clamp(0.0, tile_pixel_width as f32 - 1.0)
                                    as u32;
                                let new_y = ((pointer_pos.y - rect.top()) / scale)
                                    .clamp(0.0, tile_pixel_height as f32 - 1.0)
                                    as u32;

                                if let Some(tileset) =
                                    project.tilesets.iter_mut().find(|t| t.id == tileset_id)
                                {
                                    let props = tileset.get_tile_properties_mut(tile_idx);
                                    props.origin_x = Some(new_x);
                                    props.origin_y = Some(new_y);
                                    project.mark_dirty();
                                }
                            }
                        }

                        // Show coordinates and reset button
                        ui.horizontal(|ui| {
                            ui.label(format!("Origin: ({}, {})", origin_x, origin_y));
                            if ui.button("Reset to Center").clicked() {
                                if let Some(tileset) =
                                    project.tilesets.iter_mut().find(|t| t.id == tileset_id)
                                {
                                    let props = tileset.get_tile_properties_mut(tile_idx);
                                    props.origin_x = None;
                                    props.origin_y = None;
                                    project.mark_dirty();
                                }
                            }
                        });
                    }

                    ui.separator();

                    // Animation settings
                    ui.heading("Animation");

                    let has_anim = current_props.animation_frames.is_some();
                    let mut enable_anim = has_anim;

                    if ui.checkbox(&mut enable_anim, "Enable animation").changed() {
                        if let Some(tileset) =
                            project.tilesets.iter_mut().find(|t| t.id == tileset_id)
                        {
                            let props = tileset.get_tile_properties_mut(tile_idx);
                            if enable_anim {
                                props.animation_frames = Some(vec![tile_idx]);
                                props.animation_speed = Some(10.0);
                            } else {
                                props.animation_frames = None;
                                props.animation_speed = None;
                            }
                            project.mark_dirty();
                        }
                    }

                    if enable_anim {
                        // Animation speed
                        let mut speed = current_props.animation_speed.unwrap_or(10.0);
                        ui.horizontal(|ui| {
                            ui.label("Speed (FPS):");
                            if ui
                                .add(egui::DragValue::new(&mut speed).range(0.1..=60.0))
                                .changed()
                            {
                                if let Some(tileset) =
                                    project.tilesets.iter_mut().find(|t| t.id == tileset_id)
                                {
                                    let props = tileset.get_tile_properties_mut(tile_idx);
                                    props.animation_speed = Some(speed);
                                    project.mark_dirty();
                                }
                            }
                        });

                        // Animation frames display
                        ui.label("Frames:");
                        let frames = current_props.animation_frames.clone().unwrap_or_default();
                        ui.horizontal_wrapped(|ui| {
                            for frame in &frames {
                                ui.label(format!("{}", frame));
                            }
                        });
                        ui.small("(Frame editing coming in future update)");
                    }

                    ui.separator();

                    // Custom properties
                    ui.heading("Custom Properties");
                    if current_props.custom.is_empty() {
                        ui.label("No custom properties set");
                    } else {
                        for (key, value) in &current_props.custom {
                            ui.horizontal(|ui| {
                                ui.label(key);
                                ui.label(":");
                                ui.label(format!("{}", value));
                            });
                        }
                    }
                    ui.small("(Custom property editing coming in future update)");
                });
        } else {
            ui.label("Click a tile on the left to edit its properties");
        }
    });
}

/// Render tile selector grid for the properties tab with shift+drag tile merging
fn render_tile_selector_for_properties(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    project: &mut Project,
    tileset_id: uuid::Uuid,
    _tile_size: u32,
    images: &[bevy_map_core::TilesetImage],
    cache: Option<&TilesetTextureCache>,
) {
    let display_size = egui::vec2(32.0, 32.0);
    let spacing = 1.0;
    let mut virtual_offset = 0u32;

    // Check input state for shift+drag merging
    let shift_held = ui.input(|i| i.modifiers.shift);
    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    let primary_pressed = ui.input(|i| i.pointer.primary_pressed());
    let primary_released = ui.input(|i| i.pointer.primary_released());
    let primary_down = ui.input(|i| i.pointer.primary_down());

    // Show hint for shift+drag
    if shift_held {
        ui.small("Shift+Drag to merge tiles");
    }

    for (image_idx, image) in images.iter().enumerate() {
        let texture_id = cache
            .and_then(|c| c.loaded.get(&image.id))
            .map(|(_, tex_id, _, _)| *tex_id);

        ui.collapsing(&image.name, |ui| {
            if image.columns == 0 || image.rows == 0 {
                ui.label("Image not loaded yet");
                return;
            }

            let uv_tile_width = 1.0 / image.columns.max(1) as f32;
            let uv_tile_height = 1.0 / image.rows.max(1) as f32;

            // Store tile rects for shift+drag interaction
            let mut tile_rects: Vec<(u32, u32, egui::Rect, u32)> = Vec::new();

            for row in 0..image.rows {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(spacing, spacing);

                    for col in 0..image.columns {
                        let local_index = row * image.columns + col;
                        let virtual_index = virtual_offset + local_index;
                        let selected = editor_state
                            .tileset_editor_state
                            .selected_tile_for_properties
                            == Some(virtual_index);

                        let response = if let Some(tex_id) = texture_id {
                            let uv_min =
                                egui::pos2(col as f32 * uv_tile_width, row as f32 * uv_tile_height);
                            let uv_max = egui::pos2(
                                (col + 1) as f32 * uv_tile_width,
                                (row + 1) as f32 * uv_tile_height,
                            );

                            ui.add(
                                egui::Button::image(
                                    egui::Image::new(egui::load::SizedTexture::new(
                                        tex_id,
                                        display_size,
                                    ))
                                    .uv(egui::Rect::from_min_max(uv_min, uv_max)),
                                )
                                .frame(false) // Remove button padding
                                .corner_radius(0.0),
                            )
                        } else {
                            ui.add(
                                egui::Button::new(format!("{}", virtual_index))
                                    .min_size(display_size)
                                    .selected(selected),
                            )
                        };

                        // Draw selection border manually (doesn't obscure content)
                        if selected {
                            ui.painter().rect_stroke(
                                response.rect,
                                0.0,
                                egui::Stroke::new(2.0, EditorTheme::ACCENT_BLUE),
                                egui::StrokeKind::Inside,
                            );
                        }

                        // Track rect for shift+drag interaction
                        tile_rects.push((col, row, response.rect, virtual_index));

                        // Normal click selection (only if not shift-dragging)
                        if response.clicked() && !shift_held {
                            // Find base tile if this is part of a merged tile
                            let tile_to_select = if let Some(tileset) =
                                project.tilesets.iter().find(|t| t.id == tileset_id)
                            {
                                find_base_tile_for_position(
                                    tileset,
                                    virtual_offset,
                                    image.columns,
                                    image.rows,
                                    virtual_index,
                                )
                            } else {
                                virtual_index
                            };
                            editor_state
                                .tileset_editor_state
                                .selected_tile_for_properties = Some(tile_to_select);
                        }

                        response.on_hover_text(format!("Tile {}", virtual_index));
                    }
                });
            }

            // Handle shift+drag for tile merging
            if shift_held {
                if let Some(pos) = pointer_pos {
                    // Find which tile the pointer is over
                    for &(col, row, rect, _idx) in &tile_rects {
                        if rect.contains(pos) {
                            if primary_pressed {
                                // Start drag
                                editor_state.tileset_editor_state.merge_drag_start =
                                    Some((col, row, image_idx));
                                editor_state.tileset_editor_state.merge_drag_current =
                                    Some((col, row));
                            } else if primary_down
                                && editor_state.tileset_editor_state.merge_drag_start.is_some()
                            {
                                // Update current position during drag
                                if let Some((_, _, start_img_idx)) =
                                    editor_state.tileset_editor_state.merge_drag_start
                                {
                                    if start_img_idx == image_idx {
                                        editor_state.tileset_editor_state.merge_drag_current =
                                            Some((col, row));
                                    }
                                }
                            }
                            break;
                        }
                    }
                }
            }

            // Draw selection overlay if dragging within this image
            if let (Some((start_col, start_row, start_img_idx)), Some((curr_col, curr_row))) = (
                editor_state.tileset_editor_state.merge_drag_start,
                editor_state.tileset_editor_state.merge_drag_current,
            ) {
                if start_img_idx == image_idx {
                    // Calculate selection rectangle
                    let min_col = start_col.min(curr_col);
                    let max_col = start_col.max(curr_col);
                    let min_row = start_row.min(curr_row);
                    let max_row = start_row.max(curr_row);

                    // Find the combined rect from tile rects
                    let mut combined_rect: Option<egui::Rect> = None;
                    for &(col, row, rect, _) in &tile_rects {
                        if col >= min_col && col <= max_col && row >= min_row && row <= max_row {
                            combined_rect = Some(match combined_rect {
                                None => rect,
                                Some(r) => r.union(rect),
                            });
                        }
                    }

                    // Draw yellow selection border
                    if let Some(rect) = combined_rect {
                        let painter = ui.painter();
                        painter.rect_stroke(
                            rect.expand(2.0),
                            2.0,
                            egui::Stroke::new(3.0, Color32::YELLOW),
                            egui::StrokeKind::Outside,
                        );

                        // Show size hint
                        let width = max_col - min_col + 1;
                        let height = max_row - min_row + 1;
                        let hint = format!("{}x{}", width, height);
                        painter.text(
                            rect.right_bottom() + egui::vec2(4.0, -4.0),
                            egui::Align2::LEFT_BOTTOM,
                            hint,
                            egui::FontId::default(),
                            Color32::YELLOW,
                        );
                    }
                }
            }

            // Draw borders around existing merged tile regions
            if let Some(tileset) = project.tilesets.iter().find(|t| t.id == tileset_id) {
                for row in 0..image.rows {
                    for col in 0..image.columns {
                        let tile_idx = virtual_offset + row * image.columns + col;
                        if let Some(props) = tileset.get_tile_properties(tile_idx) {
                            if props.grid_width > 1 || props.grid_height > 1 {
                                // This is a merged tile base - draw border
                                let end_col = col + props.grid_width - 1;
                                let end_row = row + props.grid_height - 1;

                                // Find combined rect
                                let mut combined_rect: Option<egui::Rect> = None;
                                for &(tc, tr, rect, _) in &tile_rects {
                                    if tc >= col && tc <= end_col && tr >= row && tr <= end_row {
                                        combined_rect = Some(match combined_rect {
                                            None => rect,
                                            Some(r) => r.union(rect),
                                        });
                                    }
                                }

                                if let Some(rect) = combined_rect {
                                    let painter = ui.painter();
                                    // Use cyan for existing merged tiles (different from yellow selection)
                                    painter.rect_stroke(
                                        rect.expand(1.0),
                                        1.0,
                                        egui::Stroke::new(2.0, Color32::LIGHT_BLUE),
                                        egui::StrokeKind::Outside,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // Finalize merge on release
            if primary_released {
                if let (Some((start_col, start_row, start_img_idx)), Some((curr_col, curr_row))) = (
                    editor_state.tileset_editor_state.merge_drag_start,
                    editor_state.tileset_editor_state.merge_drag_current,
                ) {
                    if start_img_idx == image_idx {
                        let min_col = start_col.min(curr_col);
                        let max_col = start_col.max(curr_col);
                        let min_row = start_row.min(curr_row);
                        let max_row = start_row.max(curr_row);

                        let width = max_col - min_col + 1;
                        let height = max_row - min_row + 1;

                        // Calculate base tile index (top-left of selection)
                        let base_tile_idx = virtual_offset + min_row * image.columns + min_col;

                        // Set grid size on the base tile
                        if let Some(tileset) =
                            project.tilesets.iter_mut().find(|t| t.id == tileset_id)
                        {
                            tileset.set_tile_grid_size(base_tile_idx, width, height);
                            project.mark_dirty();

                            // Also select this tile for editing
                            editor_state
                                .tileset_editor_state
                                .selected_tile_for_properties = Some(base_tile_idx);
                        }
                    }
                }

                // Clear drag state
                editor_state.tileset_editor_state.merge_drag_start = None;
                editor_state.tileset_editor_state.merge_drag_current = None;
            }
        });

        virtual_offset += image.tile_count();
    }

    // Clear drag state if shift released
    if !shift_held && !primary_down {
        editor_state.tileset_editor_state.merge_drag_start = None;
        editor_state.tileset_editor_state.merge_drag_current = None;
    }
}

// ============================================================================
// Collision Editor Tab
// ============================================================================

fn render_collision_tab(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    project: &mut Project,
    cache: Option<&TilesetTextureCache>,
) {
    let Some(tileset_id) = editor_state.selected_tileset else {
        ui.label("No tileset selected");
        return;
    };

    // Get tileset data
    let tileset_data = project
        .tilesets
        .iter()
        .find(|t| t.id == tileset_id)
        .map(|t| (t.tile_size, t.images.clone()));

    let Some((tile_size, images)) = tileset_data else {
        ui.label("Tileset not found");
        return;
    };

    if images.is_empty() {
        ui.label("No images in tileset");
        return;
    }

    // Three-panel layout with resizable splitters
    // Left panel: Tile selector grid (resizable)
    egui::SidePanel::left("collision_tile_list")
        .resizable(true)
        .default_width(200.0)
        .show_inside(ui, |ui| {
            ui.heading("Tiles");

            // Zoom slider for grid
            ui.horizontal(|ui| {
                ui.label("Zoom:");
                ui.add(
                    egui::Slider::new(
                        &mut editor_state.tileset_editor_state.collision_editor.grid_zoom,
                        0.5..=3.0,
                    )
                    .show_value(false),
                );
            });

            ui.separator();

            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    render_collision_tile_selector(
                        ui,
                        editor_state,
                        project,
                        tile_size as f32,
                        &images,
                        cache,
                    );
                });
        });

    // Right panel: Tools and properties (resizable)
    egui::SidePanel::right("collision_tools_panel")
        .resizable(true)
        .default_width(180.0)
        .show_inside(ui, |ui| {
            ui.heading("Tools");

            ui.separator();

            // Drawing mode selector
            ui.label("Drawing Mode:");
            let collision_state = &mut editor_state.tileset_editor_state.collision_editor;

            ui.horizontal(|ui| {
                if ui
                    .selectable_label(
                        collision_state.drawing_mode == CollisionDrawMode::Select,
                        "Select",
                    )
                    .clicked()
                {
                    collision_state.drawing_mode = CollisionDrawMode::Select;
                    collision_state.polygon_points.clear();
                }
                if ui
                    .selectable_label(
                        collision_state.drawing_mode == CollisionDrawMode::Rectangle,
                        "Rect",
                    )
                    .clicked()
                {
                    collision_state.drawing_mode = CollisionDrawMode::Rectangle;
                    collision_state.polygon_points.clear();
                }
            });
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(
                        collision_state.drawing_mode == CollisionDrawMode::Circle,
                        "Circle",
                    )
                    .clicked()
                {
                    collision_state.drawing_mode = CollisionDrawMode::Circle;
                    collision_state.polygon_points.clear();
                }
                if ui
                    .selectable_label(
                        collision_state.drawing_mode == CollisionDrawMode::Polygon,
                        "Polygon",
                    )
                    .clicked()
                {
                    collision_state.drawing_mode = CollisionDrawMode::Polygon;
                }
            });

            ui.separator();

            // Show current mode instructions
            ui.label("Instructions:");
            match collision_state.drawing_mode {
                CollisionDrawMode::Select => {
                    ui.small("Click shape to select\nDrag handles to resize");
                }
                CollisionDrawMode::Rectangle => {
                    ui.small("Click and drag to\ndraw rectangle");
                }
                CollisionDrawMode::Circle => {
                    ui.small("Click center, then\ndrag to set radius");
                }
                CollisionDrawMode::Polygon => {
                    ui.small("Click to add points\nDouble-click to finish");
                    if !collision_state.polygon_points.is_empty() {
                        ui.label(format!("Points: {}", collision_state.polygon_points.len()));
                    }
                }
            }

            ui.separator();

            // Properties for selected tile
            render_collision_properties(ui, editor_state, project);
        });

    // Center panel: Large tile preview canvas (fills remaining space)
    egui::CentralPanel::default().show_inside(ui, |ui| {
        ui.heading("Collision Shape");

        // Zoom slider for preview
        ui.horizontal(|ui| {
            ui.label("Preview Zoom:");
            ui.add(
                egui::Slider::new(
                    &mut editor_state
                        .tileset_editor_state
                        .collision_editor
                        .preview_zoom,
                    4.0..=16.0,
                )
                .show_value(false),
            );
        });

        ui.separator();

        // Render the canvas with collision shape
        render_collision_canvas(ui, editor_state, project, tile_size as f32, &images, cache);
    });
}

/// Render the tile selector grid for collision editing
fn render_collision_tile_selector(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    project: &Project,
    tile_size: f32,
    images: &[bevy_map_core::TilesetImage],
    cache: Option<&TilesetTextureCache>,
) {
    let tileset_id = editor_state.selected_tileset.unwrap();
    let tileset = project.tilesets.iter().find(|t| t.id == tileset_id);

    let zoom = editor_state.tileset_editor_state.collision_editor.grid_zoom;
    let display_size = egui::vec2(tile_size * zoom, tile_size * zoom);
    let mut virtual_offset = 0u32;

    for image in images {
        let texture_id = cache
            .and_then(|c| c.loaded.get(&image.id))
            .map(|(_, tex_id, _, _)| *tex_id);

        ui.collapsing(&image.name, |ui| {
            if image.columns == 0 || image.rows == 0 {
                ui.label("Image not loaded");
                return;
            }

            let uv_tile_width = 1.0 / image.columns.max(1) as f32;
            let uv_tile_height = 1.0 / image.rows.max(1) as f32;

            for row in 0..image.rows {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(1.0, 1.0);

                    for col in 0..image.columns {
                        let local_index = row * image.columns + col;
                        let virtual_index = virtual_offset + local_index;
                        let selected = editor_state
                            .tileset_editor_state
                            .collision_editor
                            .selected_tile
                            == Some(virtual_index);

                        // Get collision shape for this tile (if any)
                        let collision_shape = tileset
                            .and_then(|t| t.get_tile_properties(virtual_index))
                            .map(|p| p.collision.shape.clone());

                        let (rect, response) =
                            ui.allocate_exact_size(display_size, egui::Sense::click());

                        // Draw tile texture
                        if let Some(tex_id) = texture_id {
                            let uv_min =
                                egui::pos2(col as f32 * uv_tile_width, row as f32 * uv_tile_height);
                            let uv_max = egui::pos2(
                                (col + 1) as f32 * uv_tile_width,
                                (row + 1) as f32 * uv_tile_height,
                            );

                            let mut mesh = egui::Mesh::with_texture(tex_id);
                            mesh.add_rect_with_uv(
                                rect,
                                egui::Rect::from_min_max(uv_min, uv_max),
                                Color32::WHITE,
                            );
                            ui.painter().add(Shape::mesh(mesh));
                        } else {
                            ui.painter().rect_filled(rect, 0.0, Color32::from_gray(60));
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                format!("{}", virtual_index),
                                egui::FontId::default(),
                                Color32::WHITE,
                            );
                        }

                        // Draw selection highlight
                        if selected {
                            ui.painter().rect_stroke(
                                rect,
                                0.0,
                                egui::Stroke::new(2.0, Color32::from_rgb(50, 150, 255)),
                                egui::StrokeKind::Inside,
                            );
                        }

                        // Draw collision shape preview on tile
                        if let Some(ref shape) = collision_shape {
                            draw_collision_shape_on_canvas(ui.painter(), rect, shape);
                        }

                        if response.clicked() {
                            editor_state
                                .tileset_editor_state
                                .collision_editor
                                .selected_tile = Some(virtual_index);
                            // Clear polygon points when selecting new tile
                            editor_state
                                .tileset_editor_state
                                .collision_editor
                                .polygon_points
                                .clear();
                        }

                        let has_collision = collision_shape
                            .as_ref()
                            .map(|s| !matches!(s, bevy_map_core::CollisionShape::None))
                            .unwrap_or(false);
                        response.on_hover_text(format!(
                            "Tile {}\n{}",
                            virtual_index,
                            if has_collision {
                                "Has collision"
                            } else {
                                "No collision"
                            }
                        ));
                    }
                });
            }
        });

        virtual_offset += image.tile_count();
    }
}

/// Render the large collision canvas with shape overlay
fn render_collision_canvas(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    project: &mut Project,
    tile_size: f32,
    images: &[bevy_map_core::TilesetImage],
    cache: Option<&TilesetTextureCache>,
) {
    let tileset_id = editor_state.selected_tileset.unwrap();
    let collision_state = &editor_state.tileset_editor_state.collision_editor;

    let Some(tile_idx) = collision_state.selected_tile else {
        ui.label("Select a tile to edit its collision shape");
        return;
    };

    let zoom = collision_state.preview_zoom;
    let canvas_size = egui::vec2(tile_size * zoom, tile_size * zoom);

    // Find which image and local index this tile belongs to
    let mut virtual_offset = 0u32;
    let mut found_tile: Option<(usize, u32, &bevy_map_core::TilesetImage)> = None;

    for (img_idx, image) in images.iter().enumerate() {
        let tile_count = image.tile_count();
        if tile_idx >= virtual_offset && tile_idx < virtual_offset + tile_count {
            found_tile = Some((img_idx, tile_idx - virtual_offset, image));
            break;
        }
        virtual_offset += tile_count;
    }

    let Some((_img_idx, local_index, image)) = found_tile else {
        ui.label("Tile not found in images");
        return;
    };

    // Allocate canvas area with click and drag sensing
    let (canvas_rect, canvas_response) =
        ui.allocate_exact_size(canvas_size, egui::Sense::click_and_drag());

    // 1. Draw tile texture as background
    let texture_id = cache
        .and_then(|c| c.loaded.get(&image.id))
        .map(|(_, tex_id, _, _)| *tex_id);

    if let Some(tex_id) = texture_id {
        if image.columns > 0 && image.rows > 0 {
            let uv_tile_width = 1.0 / image.columns as f32;
            let uv_tile_height = 1.0 / image.rows as f32;

            let col = local_index % image.columns;
            let row = local_index / image.columns;

            let uv_min = egui::pos2(col as f32 * uv_tile_width, row as f32 * uv_tile_height);
            let uv_max = egui::pos2(
                (col + 1) as f32 * uv_tile_width,
                (row + 1) as f32 * uv_tile_height,
            );

            let mut mesh = egui::Mesh::with_texture(tex_id);
            mesh.add_rect_with_uv(
                canvas_rect,
                egui::Rect::from_min_max(uv_min, uv_max),
                Color32::WHITE,
            );
            ui.painter().add(Shape::mesh(mesh));
        }
    } else {
        // Draw placeholder if no texture
        ui.painter()
            .rect_filled(canvas_rect, 0.0, Color32::from_gray(40));
    }

    // Draw canvas border
    ui.painter().rect_stroke(
        canvas_rect,
        0.0,
        egui::Stroke::new(1.0, Color32::from_gray(100)),
        egui::StrokeKind::Inside,
    );

    // 2. Handle mouse interaction FIRST (before drawing shapes)
    // This ensures drag_state is set before we check it for rendering
    handle_collision_canvas_input(
        ui,
        canvas_rect,
        canvas_response,
        editor_state,
        project,
        tile_idx,
    );

    // 3. Get current collision data (after input handling may have modified it)
    let tileset = project.tilesets.iter().find(|t| t.id == tileset_id);
    let collision_data = tileset
        .and_then(|t| t.get_tile_properties(tile_idx))
        .map(|p| p.collision.clone())
        .unwrap_or_default();

    // 4. Draw collision shape overlay (skip if dragging vertex - preview handles it)
    let is_dragging_vertex = matches!(
        &editor_state
            .tileset_editor_state
            .collision_editor
            .drag_state,
        Some(CollisionDragState {
            operation: CollisionDragOperation::MoveVertex { .. },
            ..
        })
    );
    if !is_dragging_vertex {
        draw_collision_shape_on_canvas(ui.painter(), canvas_rect, &collision_data.shape);
    }

    // 5. Draw drag handles in select mode (skip if dragging vertex - preview handles it)
    let drawing_mode = editor_state
        .tileset_editor_state
        .collision_editor
        .drawing_mode;
    if drawing_mode == CollisionDrawMode::Select && !is_dragging_vertex {
        draw_collision_handles(ui.painter(), canvas_rect, &collision_data.shape);
    }

    // 6. Draw in-progress polygon points
    if drawing_mode == CollisionDrawMode::Polygon {
        let polygon_points = &editor_state
            .tileset_editor_state
            .collision_editor
            .polygon_points;
        if !polygon_points.is_empty() {
            draw_polygon_in_progress(ui.painter(), canvas_rect, polygon_points);
        }
    }
}

/// Draw the collision shape on the canvas
fn draw_collision_shape_on_canvas(
    painter: &egui::Painter,
    canvas_rect: egui::Rect,
    shape: &bevy_map_core::CollisionShape,
) {
    let fill = Color32::from_rgba_unmultiplied(0, 150, 255, 80);
    let stroke = egui::Stroke::new(2.0, Color32::from_rgb(0, 120, 255));

    match shape {
        bevy_map_core::CollisionShape::None => {}
        bevy_map_core::CollisionShape::Full => {
            painter.rect(canvas_rect, 0.0, fill, stroke, egui::StrokeKind::Inside);
        }
        bevy_map_core::CollisionShape::Rectangle { offset, size } => {
            let shape_rect = normalized_to_canvas_rect(canvas_rect, offset, size);
            painter.rect(shape_rect, 0.0, fill, stroke, egui::StrokeKind::Inside);
        }
        bevy_map_core::CollisionShape::Circle { offset, radius } => {
            let center = normalized_to_canvas_point(canvas_rect, offset);
            let r = radius * canvas_rect.width();
            painter.circle(center, r, fill, stroke);
        }
        bevy_map_core::CollisionShape::Polygon { points } => {
            if points.len() >= 3 {
                let screen_points: Vec<Pos2> = points
                    .iter()
                    .map(|p| normalized_to_canvas_point(canvas_rect, p))
                    .collect();
                painter.add(Shape::convex_polygon(screen_points, fill, stroke));
            }
        }
    }
}

/// Draw drag handles for the shape
fn draw_collision_handles(
    painter: &egui::Painter,
    canvas_rect: egui::Rect,
    shape: &bevy_map_core::CollisionShape,
) {
    let handle_radius = 6.0;
    let handle_fill = Color32::WHITE;
    let handle_stroke = egui::Stroke::new(2.0, Color32::from_rgb(0, 100, 200));

    match shape {
        bevy_map_core::CollisionShape::Rectangle { offset, size } => {
            // Draw handles at 4 corners
            let corners = [
                [offset[0], offset[1]],                     // top-left
                [offset[0] + size[0], offset[1]],           // top-right
                [offset[0], offset[1] + size[1]],           // bottom-left
                [offset[0] + size[0], offset[1] + size[1]], // bottom-right
            ];
            for corner in &corners {
                let pos = normalized_to_canvas_point(canvas_rect, corner);
                painter.circle(pos, handle_radius, handle_fill, handle_stroke);
            }
        }
        bevy_map_core::CollisionShape::Circle { offset, radius } => {
            // Center handle
            let center = normalized_to_canvas_point(canvas_rect, offset);
            painter.circle(center, handle_radius, handle_fill, handle_stroke);
            // Edge handle (right side)
            let edge = Pos2::new(center.x + radius * canvas_rect.width(), center.y);
            painter.circle(edge, handle_radius, handle_fill, handle_stroke);
        }
        bevy_map_core::CollisionShape::Polygon { points } => {
            // Handle at each vertex
            for p in points {
                let pos = normalized_to_canvas_point(canvas_rect, p);
                painter.circle(pos, handle_radius, handle_fill, handle_stroke);
            }
        }
        bevy_map_core::CollisionShape::Full => {
            // No handles for full collision
        }
        bevy_map_core::CollisionShape::None => {}
    }
}

/// Draw polygon points in progress
fn draw_polygon_in_progress(painter: &egui::Painter, canvas_rect: egui::Rect, points: &[[f32; 2]]) {
    let point_color = Color32::from_rgb(255, 200, 0);
    let line_color = Color32::from_rgba_unmultiplied(255, 200, 0, 180);

    // Draw lines connecting points
    if points.len() >= 2 {
        for i in 0..points.len() - 1 {
            let p1 = normalized_to_canvas_point(canvas_rect, &points[i]);
            let p2 = normalized_to_canvas_point(canvas_rect, &points[i + 1]);
            painter.line_segment([p1, p2], egui::Stroke::new(2.0, line_color));
        }
    }

    // Draw points
    for p in points {
        let pos = normalized_to_canvas_point(canvas_rect, p);
        painter.circle_filled(pos, 5.0, point_color);
    }
}

/// Handle mouse input on the collision canvas
fn handle_collision_canvas_input(
    ui: &mut egui::Ui,
    canvas_rect: egui::Rect,
    response: egui::Response,
    editor_state: &mut EditorState,
    project: &mut Project,
    tile_idx: u32,
) {
    let tileset_id = editor_state.selected_tileset.unwrap();
    let drawing_mode = editor_state
        .tileset_editor_state
        .collision_editor
        .drawing_mode;

    // Handle double-click to finish polygon (Polygon mode)
    if response.double_clicked() && drawing_mode == CollisionDrawMode::Polygon {
        let polygon_points = &editor_state
            .tileset_editor_state
            .collision_editor
            .polygon_points;
        if polygon_points.len() >= 3 {
            // Create polygon shape
            let shape = bevy_map_core::CollisionShape::Polygon {
                points: polygon_points.clone(),
            };
            if let Some(tileset) = project.tilesets.iter_mut().find(|t| t.id == tileset_id) {
                tileset.set_tile_collision_shape(tile_idx, shape);
                project.mark_dirty();
            }
            editor_state
                .tileset_editor_state
                .collision_editor
                .polygon_points
                .clear();
        }
        return;
    }

    // Handle double-click to add point (Select mode)
    if response.double_clicked() && drawing_mode == CollisionDrawMode::Select {
        if let Some(pointer_pos) = ui.input(|i| i.pointer.interact_pos()) {
            let normalized = canvas_point_to_normalized(canvas_rect, pointer_pos);

            // Only add if NOT clicking on existing vertex
            if let Some(tileset) = project.tilesets.iter_mut().find(|t| t.id == tileset_id) {
                if let Some(props) = tileset.tile_properties.get_mut(&tile_idx) {
                    if let bevy_map_core::CollisionShape::Polygon { points } =
                        &mut props.collision.shape
                    {
                        // Check if NOT on existing vertex
                        if hit_test_polygon_vertex(canvas_rect, points, pointer_pos, 8.0).is_none()
                        {
                            let clamped =
                                [normalized[0].clamp(0.0, 1.0), normalized[1].clamp(0.0, 1.0)];
                            let insert_idx = find_best_insertion_index(points, &clamped);
                            points.insert(insert_idx, clamped);
                            project.mark_dirty();
                        }
                    }
                }
            }
        }
        return;
    }

    // Handle click
    if response.clicked() {
        if let Some(pointer_pos) = ui.input(|i| i.pointer.interact_pos()) {
            let normalized = canvas_point_to_normalized(canvas_rect, pointer_pos);

            match drawing_mode {
                CollisionDrawMode::Polygon => {
                    // Add point to polygon
                    editor_state
                        .tileset_editor_state
                        .collision_editor
                        .polygon_points
                        .push(normalized);
                }
                CollisionDrawMode::Rectangle => {
                    // Start rectangle drag
                    editor_state
                        .tileset_editor_state
                        .collision_editor
                        .drag_state = Some(CollisionDragState {
                        operation: CollisionDragOperation::NewRectangle,
                        start_pos: normalized,
                        current_pos: normalized,
                    });
                }
                CollisionDrawMode::Circle => {
                    // Set center and start radius drag
                    editor_state
                        .tileset_editor_state
                        .collision_editor
                        .drag_state = Some(CollisionDragState {
                        operation: CollisionDragOperation::NewCircle { center: normalized },
                        start_pos: normalized,
                        current_pos: normalized,
                    });
                }
                CollisionDrawMode::Select => {
                    // Vertex dragging is handled by drag_started() below
                    // clicked() fires after mouse release, which is too late for drag setup
                }
            }
        }
    }

    // Handle drag start - for Select mode, allow click-and-drag in one motion
    if response.drag_started() && drawing_mode == CollisionDrawMode::Select {
        if let Some(pointer_pos) = ui.input(|i| i.pointer.interact_pos()) {
            let normalized = canvas_point_to_normalized(canvas_rect, pointer_pos);

            // Check if starting drag on a polygon vertex handle
            if let Some(tileset) = project.tilesets.iter().find(|t| t.id == tileset_id) {
                if let Some(props) = tileset.get_tile_properties(tile_idx) {
                    if let bevy_map_core::CollisionShape::Polygon { points } =
                        &props.collision.shape
                    {
                        if let Some(vertex_idx) =
                            hit_test_polygon_vertex(canvas_rect, points, pointer_pos, 8.0)
                        {
                            editor_state
                                .tileset_editor_state
                                .collision_editor
                                .drag_state = Some(CollisionDragState {
                                start_pos: normalized,
                                current_pos: normalized,
                                operation: CollisionDragOperation::MoveVertex {
                                    index: vertex_idx,
                                    original: points[vertex_idx],
                                },
                            });
                        }
                    }
                }
            }
        }
    }

    // Handle drag
    if response.dragged() {
        if let Some(pointer_pos) = ui.input(|i| i.pointer.interact_pos()) {
            let normalized = canvas_point_to_normalized(canvas_rect, pointer_pos);

            if let Some(ref mut drag_state) = editor_state
                .tileset_editor_state
                .collision_editor
                .drag_state
            {
                drag_state.current_pos = normalized;
            }
        }
    }

    // Handle drag release - commit shape
    if response.drag_stopped() {
        if let Some(drag_state) = editor_state
            .tileset_editor_state
            .collision_editor
            .drag_state
            .take()
        {
            match drag_state.operation {
                CollisionDragOperation::NewRectangle => {
                    let min_x = drag_state.start_pos[0]
                        .min(drag_state.current_pos[0])
                        .clamp(0.0, 1.0);
                    let min_y = drag_state.start_pos[1]
                        .min(drag_state.current_pos[1])
                        .clamp(0.0, 1.0);
                    let max_x = drag_state.start_pos[0]
                        .max(drag_state.current_pos[0])
                        .clamp(0.0, 1.0);
                    let max_y = drag_state.start_pos[1]
                        .max(drag_state.current_pos[1])
                        .clamp(0.0, 1.0);

                    let width = max_x - min_x;
                    let height = max_y - min_y;

                    if width > 0.01 && height > 0.01 {
                        let shape = bevy_map_core::CollisionShape::Rectangle {
                            offset: [min_x, min_y],
                            size: [width, height],
                        };
                        if let Some(tileset) =
                            project.tilesets.iter_mut().find(|t| t.id == tileset_id)
                        {
                            tileset.set_tile_collision_shape(tile_idx, shape);
                            project.mark_dirty();
                        }
                    }
                }
                CollisionDragOperation::NewCircle { center } => {
                    let dx = drag_state.current_pos[0] - center[0];
                    let dy = drag_state.current_pos[1] - center[1];
                    let radius = (dx * dx + dy * dy).sqrt();

                    if radius > 0.01 {
                        let shape = bevy_map_core::CollisionShape::Circle {
                            offset: center,
                            radius,
                        };
                        if let Some(tileset) =
                            project.tilesets.iter_mut().find(|t| t.id == tileset_id)
                        {
                            tileset.set_tile_collision_shape(tile_idx, shape);
                            project.mark_dirty();
                        }
                    }
                }
                CollisionDragOperation::MoveVertex { index, .. } => {
                    // Clamp position to tile bounds [0.0, 1.0]
                    let clamped = [
                        drag_state.current_pos[0].clamp(0.0, 1.0),
                        drag_state.current_pos[1].clamp(0.0, 1.0),
                    ];

                    // Update the polygon vertex in the tileset
                    if let Some(tileset) = project.tilesets.iter_mut().find(|t| t.id == tileset_id)
                    {
                        if let Some(props) = tileset.tile_properties.get_mut(&tile_idx) {
                            if let bevy_map_core::CollisionShape::Polygon { points } =
                                &mut props.collision.shape
                            {
                                if index < points.len() {
                                    points[index] = clamped;
                                    project.mark_dirty();
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Draw preview of shape being drawn
    if let Some(ref drag_state) = editor_state
        .tileset_editor_state
        .collision_editor
        .drag_state
    {
        let preview_fill = Color32::from_rgba_unmultiplied(255, 200, 0, 60);
        let preview_stroke = egui::Stroke::new(2.0, Color32::from_rgb(255, 200, 0));

        match &drag_state.operation {
            CollisionDragOperation::NewRectangle => {
                let min_x = drag_state.start_pos[0].min(drag_state.current_pos[0]);
                let min_y = drag_state.start_pos[1].min(drag_state.current_pos[1]);
                let max_x = drag_state.start_pos[0].max(drag_state.current_pos[0]);
                let max_y = drag_state.start_pos[1].max(drag_state.current_pos[1]);

                let preview_rect = normalized_to_canvas_rect(
                    canvas_rect,
                    &[min_x, min_y],
                    &[max_x - min_x, max_y - min_y],
                );
                ui.painter().rect(
                    preview_rect,
                    0.0,
                    preview_fill,
                    preview_stroke,
                    egui::StrokeKind::Inside,
                );
            }
            CollisionDragOperation::NewCircle { center } => {
                let dx = drag_state.current_pos[0] - center[0];
                let dy = drag_state.current_pos[1] - center[1];
                let radius = (dx * dx + dy * dy).sqrt();

                let center_pos = normalized_to_canvas_point(canvas_rect, center);
                let r = radius * canvas_rect.width();
                ui.painter()
                    .circle(center_pos, r, preview_fill, preview_stroke);
            }
            CollisionDragOperation::MoveVertex { index, .. } => {
                // Draw the polygon with the dragged vertex at its new position
                if let Some(tileset) = project.tilesets.iter().find(|t| t.id == tileset_id) {
                    if let Some(props) = tileset.get_tile_properties(tile_idx) {
                        if let bevy_map_core::CollisionShape::Polygon { points } =
                            &props.collision.shape
                        {
                            // Create a temporary copy with the moved vertex
                            let mut preview_points = points.clone();
                            if *index < preview_points.len() {
                                // Clamp during preview too
                                preview_points[*index] = [
                                    drag_state.current_pos[0].clamp(0.0, 1.0),
                                    drag_state.current_pos[1].clamp(0.0, 1.0),
                                ];
                            }
                            // Draw the preview polygon and handles
                            let preview_shape = bevy_map_core::CollisionShape::Polygon {
                                points: preview_points,
                            };
                            draw_collision_shape_on_canvas(
                                ui.painter(),
                                canvas_rect,
                                &preview_shape,
                            );
                            draw_collision_handles(ui.painter(), canvas_rect, &preview_shape);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Handle right-click for context menu in Select mode
    if response.secondary_clicked() && drawing_mode == CollisionDrawMode::Select {
        if let Some(pointer_pos) = ui.input(|i| i.pointer.interact_pos()) {
            let normalized = canvas_point_to_normalized(canvas_rect, pointer_pos);

            // Check if on existing vertex
            let vertex_idx = if let Some(tileset) =
                project.tilesets.iter().find(|t| t.id == tileset_id)
            {
                tileset.get_tile_properties(tile_idx).and_then(|p| {
                    if let bevy_map_core::CollisionShape::Polygon { points } = &p.collision.shape {
                        hit_test_polygon_vertex(canvas_rect, points, pointer_pos, 8.0)
                    } else {
                        None
                    }
                })
            } else {
                None
            };

            editor_state
                .tileset_editor_state
                .collision_editor
                .context_menu_pos = Some(normalized);
            editor_state
                .tileset_editor_state
                .collision_editor
                .context_menu_vertex = vertex_idx;
        }
    }

    // Render context menu if active
    if let Some(menu_pos) = editor_state
        .tileset_editor_state
        .collision_editor
        .context_menu_pos
    {
        let screen_pos = normalized_to_canvas_point(canvas_rect, &menu_pos);
        let menu_id = ui.make_persistent_id("collision_context_menu");

        egui::Area::new(menu_id)
            .order(egui::Order::Foreground)
            .fixed_pos(screen_pos)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    // Add Point Here - always available for polygons
                    let is_polygon = project
                        .tilesets
                        .iter()
                        .find(|t| t.id == tileset_id)
                        .and_then(|t| t.get_tile_properties(tile_idx))
                        .map(|p| {
                            matches!(
                                p.collision.shape,
                                bevy_map_core::CollisionShape::Polygon { .. }
                            )
                        })
                        .unwrap_or(false);

                    if is_polygon {
                        if ui.button("Add Point Here").clicked() {
                            if let Some(tileset) =
                                project.tilesets.iter_mut().find(|t| t.id == tileset_id)
                            {
                                if let Some(props) = tileset.tile_properties.get_mut(&tile_idx) {
                                    if let bevy_map_core::CollisionShape::Polygon { points } =
                                        &mut props.collision.shape
                                    {
                                        let clamped = [
                                            menu_pos[0].clamp(0.0, 1.0),
                                            menu_pos[1].clamp(0.0, 1.0),
                                        ];
                                        let insert_idx =
                                            find_best_insertion_index(points, &clamped);
                                        points.insert(insert_idx, clamped);
                                        project.mark_dirty();
                                    }
                                }
                            }
                            editor_state
                                .tileset_editor_state
                                .collision_editor
                                .context_menu_pos = None;
                            editor_state
                                .tileset_editor_state
                                .collision_editor
                                .context_menu_vertex = None;
                        }

                        // Delete Point - only if clicked on a vertex and polygon has > 3 points
                        if let Some(vertex_idx) = editor_state
                            .tileset_editor_state
                            .collision_editor
                            .context_menu_vertex
                        {
                            let point_count = project
                                .tilesets
                                .iter()
                                .find(|t| t.id == tileset_id)
                                .and_then(|t| t.get_tile_properties(tile_idx))
                                .map(|p| {
                                    if let bevy_map_core::CollisionShape::Polygon { points } =
                                        &p.collision.shape
                                    {
                                        points.len()
                                    } else {
                                        0
                                    }
                                })
                                .unwrap_or(0);

                            if point_count > 3 {
                                if ui.button("Delete Point").clicked() {
                                    if let Some(tileset) =
                                        project.tilesets.iter_mut().find(|t| t.id == tileset_id)
                                    {
                                        if let Some(props) =
                                            tileset.tile_properties.get_mut(&tile_idx)
                                        {
                                            if let bevy_map_core::CollisionShape::Polygon {
                                                points,
                                            } = &mut props.collision.shape
                                            {
                                                if vertex_idx < points.len() {
                                                    points.remove(vertex_idx);
                                                    project.mark_dirty();
                                                }
                                            }
                                        }
                                    }
                                    editor_state
                                        .tileset_editor_state
                                        .collision_editor
                                        .context_menu_pos = None;
                                    editor_state
                                        .tileset_editor_state
                                        .collision_editor
                                        .context_menu_vertex = None;
                                }
                            }
                        }
                    }
                });
            });

        // Close menu on Escape key
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            editor_state
                .tileset_editor_state
                .collision_editor
                .context_menu_pos = None;
            editor_state
                .tileset_editor_state
                .collision_editor
                .context_menu_vertex = None;
        }
    }
}

/// Render collision properties panel
fn render_collision_properties(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    project: &mut Project,
) {
    let Some(tileset_id) = editor_state.selected_tileset else {
        return;
    };

    let Some(tile_idx) = editor_state
        .tileset_editor_state
        .collision_editor
        .selected_tile
    else {
        ui.label("No tile selected");
        return;
    };

    ui.heading("Properties");

    // Get current collision data
    let tileset = project.tilesets.iter().find(|t| t.id == tileset_id);
    let collision_data = tileset
        .and_then(|t| t.get_tile_properties(tile_idx))
        .map(|p| p.collision.clone())
        .unwrap_or_default();

    // Shape info
    ui.label(format!("Shape: {}", collision_data.shape.name()));

    ui.separator();

    // One-way direction
    let mut one_way = collision_data.one_way;
    egui::ComboBox::from_label("One-way")
        .selected_text(format!("{:?}", one_way))
        .show_ui(ui, |ui| {
            use bevy_map_core::OneWayDirection;
            let directions = [
                OneWayDirection::None,
                OneWayDirection::Top,
                OneWayDirection::Bottom,
                OneWayDirection::Left,
                OneWayDirection::Right,
            ];
            for dir in directions {
                if ui
                    .selectable_label(one_way == dir, format!("{:?}", dir))
                    .clicked()
                {
                    one_way = dir;
                }
            }
        });

    if one_way != collision_data.one_way {
        if let Some(tileset) = project.tilesets.iter_mut().find(|t| t.id == tileset_id) {
            tileset.set_tile_one_way(tile_idx, one_way);
            project.mark_dirty();
        }
    }

    // Collision layer
    let mut layer = collision_data.layer;
    ui.horizontal(|ui| {
        ui.label("Layer:");
        if ui
            .add(egui::DragValue::new(&mut layer).range(0..=31))
            .changed()
        {
            if let Some(tileset) = project.tilesets.iter_mut().find(|t| t.id == tileset_id) {
                tileset.set_tile_collision_layer(tile_idx, layer);
                project.mark_dirty();
            }
        }
    });

    ui.separator();

    // Action buttons
    if ui.button("Set Full Collision").clicked() {
        if let Some(tileset) = project.tilesets.iter_mut().find(|t| t.id == tileset_id) {
            tileset.set_tile_collision_shape(tile_idx, bevy_map_core::CollisionShape::Full);
            project.mark_dirty();
        }
    }

    if ui.button("Clear Collision").clicked() {
        if let Some(tileset) = project.tilesets.iter_mut().find(|t| t.id == tileset_id) {
            tileset.set_tile_collision_shape(tile_idx, bevy_map_core::CollisionShape::None);
            project.mark_dirty();
        }
    }
}

// ============================================================================
// Coordinate Helpers
// ============================================================================

/// Convert normalized [0,1] coordinates to screen position within canvas
fn normalized_to_canvas_point(canvas_rect: egui::Rect, normalized: &[f32; 2]) -> Pos2 {
    Pos2::new(
        canvas_rect.left() + normalized[0] * canvas_rect.width(),
        canvas_rect.top() + normalized[1] * canvas_rect.height(),
    )
}

/// Convert screen position to normalized [0,1] coordinates
fn canvas_point_to_normalized(canvas_rect: egui::Rect, screen_pos: Pos2) -> [f32; 2] {
    [
        (screen_pos.x - canvas_rect.left()) / canvas_rect.width(),
        (screen_pos.y - canvas_rect.top()) / canvas_rect.height(),
    ]
}

/// Convert normalized offset + size to screen rect
fn normalized_to_canvas_rect(canvas: egui::Rect, offset: &[f32; 2], size: &[f32; 2]) -> egui::Rect {
    let min = normalized_to_canvas_point(canvas, offset);
    let max = Pos2::new(
        min.x + size[0] * canvas.width(),
        min.y + size[1] * canvas.height(),
    );
    egui::Rect::from_min_max(min, max)
}

/// Check if a screen position hits a polygon vertex handle
/// Returns the index of the hit vertex, if any
fn hit_test_polygon_vertex(
    canvas_rect: egui::Rect,
    points: &[[f32; 2]],
    screen_pos: Pos2,
    handle_radius: f32,
) -> Option<usize> {
    for (i, p) in points.iter().enumerate() {
        let handle_pos = normalized_to_canvas_point(canvas_rect, p);
        let distance = handle_pos.distance(screen_pos);
        if distance <= handle_radius {
            return Some(i);
        }
    }
    None
}

/// Find the best index to insert a new point into a polygon
/// Returns the index where the point should be inserted (after the closest edge)
fn find_best_insertion_index(points: &[[f32; 2]], new_point: &[f32; 2]) -> usize {
    if points.len() < 2 {
        return points.len();
    }

    let mut best_idx = points.len();
    let mut best_dist = f32::MAX;

    for i in 0..points.len() {
        let p1 = &points[i];
        let p2 = &points[(i + 1) % points.len()];

        let dist = point_to_segment_distance(new_point, p1, p2);
        if dist < best_dist {
            best_dist = dist;
            best_idx = i + 1;
        }
    }

    best_idx
}

/// Calculate the distance from a point to a line segment
fn point_to_segment_distance(p: &[f32; 2], a: &[f32; 2], b: &[f32; 2]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let ap = [p[0] - a[0], p[1] - a[1]];
    let ab_len_sq = ab[0] * ab[0] + ab[1] * ab[1];

    if ab_len_sq < 0.0001 {
        return (ap[0] * ap[0] + ap[1] * ap[1]).sqrt();
    }

    let t = ((ap[0] * ab[0] + ap[1] * ab[1]) / ab_len_sq).clamp(0.0, 1.0);
    let closest = [a[0] + t * ab[0], a[1] + t * ab[1]];
    let dx = p[0] - closest[0];
    let dy = p[1] - closest[1];
    (dx * dx + dy * dy).sqrt()
}
