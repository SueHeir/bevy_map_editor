//! Tileset palette display

use bevy_egui::egui;
use bevy_map_core::Tileset;

use super::{ImageLoadState, TilesetTextureCache};
use crate::project::Project;
use crate::EditorState;

pub fn render_tileset_palette(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    project: &Project,
) {
    render_tileset_palette_with_cache(ui, editor_state, project, None)
}

pub fn render_tileset_palette_with_cache(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    project: &Project,
    tileset_cache: Option<&TilesetTextureCache>,
) {
    ui.horizontal(|ui| {
        ui.heading("Tileset");

        ui.separator();

        // Tileset selector
        let current_name = editor_state
            .selected_tileset
            .and_then(|id| project.tilesets.iter().find(|t| t.id == id))
            .map(|t| t.name.as_str())
            .unwrap_or("(none)");

        egui::ComboBox::from_id_salt("tileset_selector")
            .selected_text(current_name)
            .show_ui(ui, |ui| {
                for tileset in &project.tilesets {
                    if ui
                        .selectable_value(
                            &mut editor_state.selected_tileset,
                            Some(tileset.id),
                            &tileset.name,
                        )
                        .clicked()
                    {
                        // Clear selected tile when changing tileset
                        editor_state.selected_tile = None;
                    }
                }
            });

        ui.separator();

        // Import tileset button
        if ui.button("+ Import").clicked() {
            editor_state.show_new_tileset_dialog = true;
        }

        // Add image to existing tileset button
        if editor_state.selected_tileset.is_some() {
            if ui.button("+ Add Image").clicked() {
                editor_state.show_add_tileset_image_dialog = true;
            }
        }
    });

    ui.separator();

    if let Some(tileset_id) = editor_state.selected_tileset {
        if let Some(tileset) = project.tilesets.iter().find(|t| t.id == tileset_id) {
            // Show tileset summary info
            let total_tiles = tileset.total_tile_count();
            let image_count = tileset.images.len();
            ui.label(format!(
                "{} tiles across {} image{}, {}px each",
                total_tiles,
                image_count,
                if image_count == 1 { "" } else { "s" },
                tileset.tile_size
            ));

            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if tileset.images.is_empty() {
                        // Fallback for legacy tilesets
                        render_legacy_tileset(ui, editor_state, tileset, tileset_cache);
                    } else {
                        // Render all images in the tileset
                        render_multi_image_tileset(ui, editor_state, tileset, tileset_cache);
                    }
                });
        }
    } else {
        ui.centered_and_justified(|ui| {
            ui.label("No tileset selected");
        });
    }
}

/// Render tiles from all images in a multi-image tileset
fn render_multi_image_tileset(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    tileset: &Tileset,
    tileset_cache: Option<&TilesetTextureCache>,
) {
    let display_size = egui::vec2(32.0, 32.0);
    let mut virtual_offset = 0u32;

    for (img_idx, image) in tileset.images.iter().enumerate() {
        // Image header with collapsible section
        let header_id = format!("tileset_image_{}", img_idx);

        // Get load state for this image
        let load_state = tileset_cache
            .map(|cache| cache.get_load_state(&image.id))
            .unwrap_or(ImageLoadState::Pending);

        // Add status indicator to header
        let header_text = match &load_state {
            ImageLoadState::Loading => format!("{} (loading...)", image.name),
            ImageLoadState::Failed(_) => format!("{} (ERROR)", image.name),
            _ => image.name.clone(),
        };

        egui::CollapsingHeader::new(&header_text)
            .id_salt(&header_id)
            .default_open(true)
            .show(ui, |ui| {
                // Show tile info
                ui.label(format!(
                    "{}x{} tiles ({})",
                    image.columns,
                    image.rows,
                    std::path::Path::new(&image.path)
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                ));

                // Handle different load states
                match &load_state {
                    ImageLoadState::Failed(error_msg) => {
                        // Show error message
                        ui.colored_label(egui::Color32::RED, "Failed to load image:");
                        ui.colored_label(egui::Color32::LIGHT_RED, error_msg);
                        ui.label(format!("Path: {}", image.path));
                        ui.small("Check that the file exists in the assets folder.");
                    }
                    ImageLoadState::Loading | ImageLoadState::Pending => {
                        // Show loading indicator
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Loading tileset image...");
                        });
                        ui.label(format!("Path: {}", image.path));
                    }
                    ImageLoadState::Loaded => {
                        // Get texture for this image
                        let texture_id = tileset_cache
                            .and_then(|cache| cache.loaded.get(&image.id))
                            .map(|(_, tex_id, _, _)| *tex_id);

                        if let Some(tex_id) = texture_id {
                            // Check if we have valid dimensions
                            if image.columns == 0 || image.rows == 0 {
                                ui.colored_label(
                                    egui::Color32::YELLOW,
                                    "Tile size may be incorrect (0x0 tiles detected)",
                                );
                            }

                            // Render tiles with texture
                            let uv_tile_width = 1.0 / image.columns.max(1) as f32;
                            let uv_tile_height = 1.0 / image.rows.max(1) as f32;

                            for row in 0..image.rows {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(1.0, 1.0);
                                    for col in 0..image.columns {
                                        let local_index = row * image.columns + col;
                                        let virtual_index = virtual_offset + local_index;
                                        let selected =
                                            editor_state.selected_tile == Some(virtual_index);

                                        let uv_min = egui::pos2(
                                            col as f32 * uv_tile_width,
                                            row as f32 * uv_tile_height,
                                        );
                                        let uv_max = egui::pos2(
                                            (col + 1) as f32 * uv_tile_width,
                                            (row + 1) as f32 * uv_tile_height,
                                        );

                                        #[allow(deprecated)]
                                        let response = ui.add(
                                            egui::ImageButton::new(egui::load::SizedTexture::new(
                                                tex_id,
                                                display_size,
                                            ))
                                            .uv(egui::Rect::from_min_max(uv_min, uv_max))
                                            .selected(selected)
                                            .rounding(0.0),
                                        );

                                        if response.clicked() {
                                            editor_state.selected_tile = Some(virtual_index);
                                        }

                                        response.on_hover_text(format!(
                                            "Tile {} ({} #{})",
                                            virtual_index, image.name, local_index
                                        ));
                                    }
                                });
                            }
                        } else {
                            // Fallback: numbered buttons (shouldn't happen if Loaded)
                            render_fallback_tiles(
                                ui,
                                editor_state,
                                image.tile_count(),
                                virtual_offset,
                            );
                        }
                    }
                }
            });

        virtual_offset += image.tile_count();
        ui.add_space(4.0);
    }
}

/// Render fallback numbered buttons when texture isn't available
fn render_fallback_tiles(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    tile_count: u32,
    virtual_offset: u32,
) {
    ui.horizontal_wrapped(|ui| {
        for i in 0..tile_count.min(64) {
            let virtual_index = virtual_offset + i;
            let selected = editor_state.selected_tile == Some(virtual_index);
            let response = ui.add(
                egui::Button::new(format!("{}", virtual_index))
                    .min_size(egui::vec2(28.0, 28.0))
                    .selected(selected),
            );

            if response.clicked() {
                editor_state.selected_tile = Some(virtual_index);
            }
        }

        if tile_count > 64 {
            ui.label(format!("... +{}", tile_count - 64));
        }
    });
}

/// Render tiles from a legacy single-image tileset (backward compatibility)
fn render_legacy_tileset(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    tileset: &Tileset,
    tileset_cache: Option<&TilesetTextureCache>,
) {
    // Get primary image texture using tileset_primary_image mapping
    let texture_id = tileset_cache.and_then(|cache| {
        cache
            .tileset_primary_image
            .get(&tileset.id)
            .and_then(|img_id| cache.loaded.get(img_id))
            .map(|(_, tex_id, _, _)| *tex_id)
    });

    if let Some(tex_id) = texture_id {
        render_tileset_tiles(ui, editor_state, tileset, tex_id);
    } else {
        render_tileset_placeholder(ui, editor_state, tileset);
    }
}

/// Render tiles from the actual tileset texture (legacy)
fn render_tileset_tiles(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    tileset: &Tileset,
    texture_id: egui::TextureId,
) {
    let display_size = egui::vec2(32.0, 32.0);
    let uv_tile_width = 1.0 / tileset.columns.max(1) as f32;
    let uv_tile_height = 1.0 / tileset.rows.max(1) as f32;

    for row in 0..tileset.rows {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(1.0, 1.0);
            for col in 0..tileset.columns {
                let tile_index = row * tileset.columns + col;
                let selected = editor_state.selected_tile == Some(tile_index);

                let uv_min = egui::pos2(col as f32 * uv_tile_width, row as f32 * uv_tile_height);
                let uv_max = egui::pos2(
                    (col + 1) as f32 * uv_tile_width,
                    (row + 1) as f32 * uv_tile_height,
                );

                #[allow(deprecated)]
                let response = ui.add(
                    egui::ImageButton::new(egui::load::SizedTexture::new(texture_id, display_size))
                        .uv(egui::Rect::from_min_max(uv_min, uv_max))
                        .selected(selected)
                        .rounding(0.0),
                );

                if response.clicked() {
                    editor_state.selected_tile = Some(tile_index);
                }

                response.on_hover_text(format!("Tile {}", tile_index));
            }
        });
    }
}

/// Render placeholder numbered buttons when texture isn't loaded
fn render_tileset_placeholder(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    tileset: &Tileset,
) {
    ui.horizontal_wrapped(|ui| {
        let total_tiles = tileset.columns * tileset.rows;
        for i in 0..total_tiles.min(256) {
            let selected = editor_state.selected_tile == Some(i);
            let response = ui.add(
                egui::Button::new(format!("{}", i))
                    .min_size(egui::vec2(28.0, 28.0))
                    .selected(selected),
            );

            if response.clicked() {
                editor_state.selected_tile = Some(i);
            }
        }

        if total_tiles > 256 {
            ui.label(format!("... and {} more", total_tiles - 256));
        }
    });
}

/// Open a file dialog to select a tileset image (native only)
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn open_tileset_dialog() -> Option<String> {
    use rfd::FileDialog;

    FileDialog::new()
        .add_filter("Image Files", &["png", "jpg", "jpeg", "bmp"])
        .add_filter("All Files", &["*"])
        .set_title("Select Tileset Image")
        .pick_file()
        .map(|p| p.to_string_lossy().to_string())
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub fn open_tileset_dialog() -> Option<String> {
    None
}
