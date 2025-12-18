//! Animation Editor - Godot-Style Dopesheet Layout
//!
//! This module provides an animation editor with a timeline-focused dopesheet view:
//! - Compact toolbar with animation selection and playback controls
//! - Main dopesheet view with Frames, Windows, and Triggers tracks
//! - Floating preview window (toggleable)
//! - Collapsible frame picker for building animations
//!
//! For spritesheet setup (image loading, grid config), use the SpriteSheet Editor.

use bevy_egui::egui;
use bevy_map_animation::{
    AnimationDef, AnimationTrigger, AnimationWindow, LoopMode, SpriteData, TriggerPayload,
};
use std::collections::HashMap;
use uuid::Uuid;

/// Drag handle type for window edges
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragHandle {
    /// Dragging window start edge
    Start,
    /// Dragging window end edge
    End,
    /// Dragging entire window body
    Body,
}

/// State for the Animation Editor
#[derive(Default, Clone)]
pub struct AnimationEditorState {
    /// Asset ID being edited (project-level asset)
    pub asset_id: Option<Uuid>,
    /// Instance ID being edited (for inline property editing)
    pub instance_id: Option<Uuid>,
    /// Property name containing the SpriteData (for inline editing)
    pub property_name: String,
    /// Copy of sprite data being edited
    pub sprite_data: SpriteData,
    /// Currently selected animation name (for editing)
    pub selected_animation: Option<String>,
    /// New animation name input
    pub new_animation_name: String,
    /// Currently selected frames (for building animation)
    pub selected_frames: Vec<usize>,
    /// Animation preview state
    pub preview_playing: bool,
    pub preview_frame: usize,
    pub preview_timer: f32,
    /// Scroll position for spritesheet view
    pub scroll_offset: egui::Vec2,
    /// Zoom level for spritesheet
    pub zoom: f32,
    /// Last loaded sheet path (to detect changes)
    pub loaded_sheet_path: Option<String>,
    /// Texture ID for the loaded spritesheet (set externally)
    pub spritesheet_texture_id: Option<egui::TextureId>,
    /// Size of the loaded spritesheet in pixels
    pub spritesheet_size: Option<(f32, f32)>,
    // === Trigger/Window Editor State ===
    /// Selected trigger ID for editing
    pub selected_trigger: Option<Uuid>,
    /// Selected window ID for editing
    pub selected_window: Option<Uuid>,
    /// Timeline zoom level (pixels per 100ms)
    pub timeline_zoom: f32,
    /// New trigger name input
    pub new_trigger_name: String,
    /// New window name input
    pub new_window_name: String,
    // === UI Toggle State ===
    /// Whether to show the frame picker panel
    pub show_frame_picker: bool,
    /// Whether to show the floating preview window
    pub show_preview: bool,
    // === Animation Renaming State ===
    /// Animation being renamed (original name while editing)
    pub renaming_animation: Option<String>,
    /// Temporary name during rename edit
    pub rename_buffer: String,
    // === Drag State ===
    /// Trigger being dragged
    pub dragging_trigger: Option<Uuid>,
    /// Window being dragged with handle type
    pub dragging_window: Option<(Uuid, DragHandle)>,
    /// Original time value at drag start (for triggers or window start/end)
    pub drag_start_time: u32,
    /// Original window end time at drag start (for body drag to preserve duration)
    pub drag_original_end: u32,
    // === Context Menu State ===
    /// Time position where right-click occurred (for adding events)
    pub context_menu_time: Option<u32>,
}

impl AnimationEditorState {
    pub fn new() -> Self {
        Self {
            zoom: 2.0,           // Start at 2x zoom for better visibility
            timeline_zoom: 50.0, // 50 pixels per 100ms
            show_preview: true,  // Preview visible by default
            ..Default::default()
        }
    }

    /// Create editor state from sprite data for editing a project-level asset
    pub fn from_sprite_data(sprite_data: SpriteData, asset_id: Uuid) -> Self {
        Self {
            asset_id: Some(asset_id),
            instance_id: None,
            property_name: String::new(),
            sprite_data: sprite_data.clone(),
            selected_animation: None,
            selected_frames: Vec::new(),
            preview_playing: false,
            preview_frame: 0,
            preview_timer: 0.0,
            scroll_offset: egui::Vec2::ZERO,
            zoom: 2.0,
            new_animation_name: String::new(),
            loaded_sheet_path: None,
            spritesheet_texture_id: None,
            spritesheet_size: None,
            // Trigger/window state
            selected_trigger: None,
            selected_window: None,
            timeline_zoom: 50.0,
            new_trigger_name: String::new(),
            new_window_name: String::new(),
            // UI toggles
            show_frame_picker: false,
            show_preview: true,
            // Animation renaming
            renaming_animation: None,
            rename_buffer: String::new(),
            // Drag state
            dragging_trigger: None,
            dragging_window: None,
            drag_start_time: 0,
            drag_original_end: 0,
            // Context menu
            context_menu_time: None,
        }
    }

    /// Initialize the editor with sprite data from an instance (inline property editing)
    pub fn open(&mut self, instance_id: Uuid, property_name: String, sprite_data: SpriteData) {
        self.asset_id = None;
        self.instance_id = Some(instance_id);
        self.property_name = property_name;
        self.sprite_data = sprite_data.clone();
        self.selected_animation = None;
        self.selected_frames.clear();
        self.preview_playing = false;
        self.preview_frame = 0;
        self.zoom = 1.0;
        self.loaded_sheet_path = None;
        self.spritesheet_texture_id = None;
        self.spritesheet_size = None;
        self.selected_trigger = None;
        self.selected_window = None;
        self.timeline_zoom = 50.0;
        self.new_trigger_name.clear();
        self.new_window_name.clear();
        self.show_frame_picker = false;
        self.show_preview = true;
        // Reset new state fields
        self.renaming_animation = None;
        self.rename_buffer.clear();
        self.dragging_trigger = None;
        self.dragging_window = None;
        self.drag_start_time = 0;
        self.drag_original_end = 0;
        self.context_menu_time = None;
    }

    /// Get the edited sprite data
    pub fn get_sprite_data(&self) -> SpriteData {
        self.sprite_data.clone()
    }

    /// Check if the spritesheet needs to be (re)loaded
    pub fn needs_texture_load(&self) -> bool {
        let current_path = &self.sprite_data.sheet_path;
        !current_path.is_empty()
            && (self.loaded_sheet_path.as_ref() != Some(current_path)
                || self.spritesheet_texture_id.is_none())
    }

    /// Set the loaded texture info
    pub fn set_texture(&mut self, texture_id: egui::TextureId, width: f32, height: f32) {
        self.spritesheet_texture_id = Some(texture_id);
        self.spritesheet_size = Some((width, height));
        self.loaded_sheet_path = Some(self.sprite_data.sheet_path.clone());
    }

    /// Clear the loaded texture
    pub fn clear_texture(&mut self) {
        self.spritesheet_texture_id = None;
        self.spritesheet_size = None;
        self.loaded_sheet_path = None;
    }

    /// Refresh grid configuration from external source (when SpriteSheet Editor saves)
    pub fn refresh_grid_config(&mut self, updated: &SpriteData) {
        self.sprite_data.sheet_path = updated.sheet_path.clone();
        self.sprite_data.frame_width = updated.frame_width;
        self.sprite_data.frame_height = updated.frame_height;
        self.sprite_data.columns = updated.columns;
        self.sprite_data.rows = updated.rows;
        self.sprite_data.pivot_x = updated.pivot_x;
        self.sprite_data.pivot_y = updated.pivot_y;
        self.sprite_data.name = updated.name.clone();

        if self.loaded_sheet_path.as_ref() != Some(&updated.sheet_path) {
            self.clear_texture();
        }
    }
}

/// Result from animation editor rendering
#[derive(Default)]
pub struct AnimationEditorResult {
    /// Whether sprite data was changed and should be saved
    pub changed: bool,
    /// Whether the editor should close
    pub close: bool,
    /// Whether to open the SpriteSheet Editor for this asset
    pub open_spritesheet_editor: bool,
}

// ============================================================================
// Main Editor - Godot-Style Dopesheet Layout
// ============================================================================

/// Render the full animation editor window with Godot-style dopesheet layout
pub fn render_animation_editor(
    ctx: &egui::Context,
    state: &mut AnimationEditorState,
) -> AnimationEditorResult {
    let mut result = AnimationEditorResult::default();
    let mut is_open = true;

    // Render floating preview window (separate from main window)
    render_floating_preview(ctx, state); // state is already &mut

    egui::Window::new("Animation Editor")
        .open(&mut is_open)
        .resizable(true)
        .default_size([900.0, 600.0])
        .show(ctx, |ui| {
            // 1. TOOLBAR (compact, single row)
            render_compact_toolbar(ui, state, &mut result);
            ui.separator();

            // 2. FRAME PICKER (collapsible, at very bottom)
            if state.show_frame_picker {
                egui::TopBottomPanel::bottom("frame_picker_panel")
                    .resizable(true)
                    .default_height(150.0)
                    .show_separator_line(true)
                    .show_inside(ui, |ui| {
                        render_frame_picker(ui, state, &mut result);
                    });
            }

            // 3. DETAILS PANEL (above frame picker) - shows selected trigger/window properties
            egui::TopBottomPanel::bottom("anim_details_panel")
                .resizable(true)
                .default_height(80.0)
                .show_separator_line(true)
                .show_inside(ui, |ui| {
                    render_details_panel(ui, state, &mut result);
                });

            // 4. DOPESHEET (main area - fills remaining space)
            egui::CentralPanel::default().show_inside(ui, |ui| {
                render_dopesheet(ui, state, &mut result);
            });

            // Update animation preview timer if playing
            update_preview_timer(ui, state);
        });

    if !is_open {
        result.close = true;
    }

    result
}

/// Compact toolbar with animation selector and playback controls
fn render_compact_toolbar(
    ui: &mut egui::Ui,
    state: &mut AnimationEditorState,
    result: &mut AnimationEditorResult,
) {
    ui.horizontal(|ui| {
        // Save/Cancel buttons
        if ui.button("Save").clicked() {
            result.changed = true;
            result.close = true;
        }
        if ui.button("Cancel").clicked() {
            result.close = true;
        }
        ui.separator();

        // Spritesheet name display
        let sheet_name = std::path::Path::new(&state.sprite_data.sheet_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("(no sheet)");
        ui.label(format!("📄 {}", sheet_name));
        ui.separator();

        // Animation selector dropdown OR inline rename mode
        if let Some(original_name) = &state.renaming_animation.clone() {
            // Rename mode: show text edit
            let response = ui.add(
                egui::TextEdit::singleline(&mut state.rename_buffer)
                    .desired_width(100.0)
                    .hint_text("animation name"),
            );

            // Auto-focus on first show
            if response.gained_focus() || state.rename_buffer.is_empty() {
                state.rename_buffer = original_name.clone();
            }

            // Confirm rename on Enter or clicking away
            if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let new_name = state.rename_buffer.trim().to_string();
                if !new_name.is_empty()
                    && new_name != *original_name
                    && !state.sprite_data.animations.contains_key(&new_name)
                {
                    // Perform rename
                    if let Some(anim) = state.sprite_data.animations.remove(original_name) {
                        state.sprite_data.animations.insert(new_name.clone(), anim);
                        state.selected_animation = Some(new_name);
                        result.changed = true;
                    }
                }
                state.renaming_animation = None;
                state.rename_buffer.clear();
            }

            // Cancel on Escape
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                state.renaming_animation = None;
                state.rename_buffer.clear();
            }
        } else {
            // Normal mode: show dropdown
            let selected_text = state
                .selected_animation
                .as_deref()
                .unwrap_or("(select animation)");
            egui::ComboBox::from_id_salt("anim_selector")
                .selected_text(selected_text)
                .show_ui(ui, |ui| {
                    let anim_names: Vec<String> =
                        state.sprite_data.animations.keys().cloned().collect();
                    for name in anim_names {
                        if ui
                            .selectable_label(
                                state.selected_animation.as_ref() == Some(&name),
                                &name,
                            )
                            .clicked()
                        {
                            state.selected_animation = Some(name.clone());
                            // Load frames into selection
                            if let Some(anim) = state.sprite_data.animations.get(&name) {
                                state.selected_frames = anim.frames.clone();
                            }
                            state.preview_frame = 0;
                            state.preview_timer = 0.0;
                        }
                    }
                });

            // Rename button
            if ui
                .add_enabled(
                    state.selected_animation.is_some(),
                    egui::Button::new("✏").small(),
                )
                .on_hover_text("Rename animation")
                .clicked()
            {
                if let Some(name) = &state.selected_animation {
                    state.renaming_animation = Some(name.clone());
                    state.rename_buffer = name.clone();
                }
            }
        }

        // Add new animation
        if ui.button("+").on_hover_text("New animation").clicked() {
            // Generate unique name
            let mut name = "new_anim".to_string();
            let mut counter = 1;
            while state.sprite_data.animations.contains_key(&name) {
                name = format!("new_anim_{}", counter);
                counter += 1;
            }
            state
                .sprite_data
                .animations
                .insert(name.clone(), AnimationDef::default());
            state.selected_animation = Some(name);
            state.selected_frames.clear();
            result.changed = true;
        }

        // Delete selected animation
        if ui
            .add_enabled(
                state.selected_animation.is_some() && state.renaming_animation.is_none(),
                egui::Button::new("-").small(),
            )
            .on_hover_text("Delete animation")
            .clicked()
        {
            if let Some(name) = state.selected_animation.take() {
                state.sprite_data.animations.remove(&name);
                state.selected_frames.clear();
                result.changed = true;
            }
        }

        ui.separator();

        // Playback controls
        let frame_count = state
            .selected_animation
            .as_ref()
            .and_then(|name| state.sprite_data.animations.get(name))
            .map(|anim| anim.frames.len())
            .unwrap_or(0);

        if ui.button("|<").on_hover_text("First frame").clicked() {
            state.preview_frame = 0;
            state.preview_timer = 0.0;
        }

        if state.preview_playing {
            if ui.button("||").on_hover_text("Pause").clicked() {
                state.preview_playing = false;
            }
        } else if ui
            .add_enabled(frame_count > 0, egui::Button::new(">"))
            .on_hover_text("Play")
            .clicked()
        {
            state.preview_playing = true;
            state.preview_timer = 0.0;
        }

        // Frame counter
        let current_frame = if frame_count > 0 {
            (state.preview_frame % frame_count) + 1
        } else {
            0
        };
        ui.label(format!("{}/{}", current_frame, frame_count));

        ui.separator();

        // Frame duration setting (if animation selected)
        if let Some(anim_name) = &state.selected_animation.clone() {
            if let Some(anim) = state.sprite_data.animations.get_mut(anim_name) {
                ui.label("Frame Dur:");
                let mut duration = anim.frame_duration_ms as i32;
                if ui
                    .add(
                        egui::DragValue::new(&mut duration)
                            .range(16..=2000)
                            .suffix("ms"),
                    )
                    .changed()
                {
                    anim.frame_duration_ms = duration.max(16) as u32;
                    result.changed = true;
                }

                // Loop mode
                egui::ComboBox::from_id_salt("loop_mode_toolbar")
                    .width(60.0)
                    .selected_text(anim.loop_mode.display_name())
                    .show_ui(ui, |ui| {
                        for mode in LoopMode::all() {
                            if ui
                                .selectable_label(anim.loop_mode == *mode, mode.display_name())
                                .clicked()
                            {
                                anim.loop_mode = *mode;
                                result.changed = true;
                            }
                        }
                    });
            }
        }

        // Right-aligned toggle buttons
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Frame picker toggle
            let frames_label = if state.show_frame_picker {
                "Hide Frames"
            } else {
                "Frames"
            };
            if ui.button(frames_label).clicked() {
                state.show_frame_picker = !state.show_frame_picker;
            }

            // Preview toggle
            let preview_label = if state.show_preview {
                "Hide Preview"
            } else {
                "Preview"
            };
            if ui.button(preview_label).clicked() {
                state.show_preview = !state.show_preview;
            }

            // Edit sheet settings (if project asset)
            if state.asset_id.is_some() {
                if ui.button("Sheet...").clicked() {
                    result.open_spritesheet_editor = true;
                }
            }
        });
    });
}

// ============================================================================
// Dopesheet View
// ============================================================================

/// Render the main dopesheet view with tracks
fn render_dopesheet(
    ui: &mut egui::Ui,
    state: &mut AnimationEditorState,
    result: &mut AnimationEditorResult,
) {
    let Some(anim_name) = state.selected_animation.clone() else {
        ui.centered_and_justified(|ui| {
            ui.label("Select or create an animation to edit");
        });
        return;
    };

    let Some(anim) = state.sprite_data.animations.get(&anim_name) else {
        return;
    };

    let total_duration = anim.total_duration_ms();
    if total_duration == 0 {
        ui.centered_and_justified(|ui| {
            ui.label("Animation has no frames. Use Frame Picker to add frames.");
        });
        return;
    }

    // Clone for rendering (avoid borrow issues)
    let triggers = anim.triggers.clone();
    let windows = anim.windows.clone();
    let frames = anim.frames.clone();
    let frame_duration_ms = anim.frame_duration_ms;

    // Track add buttons and zoom control
    ui.horizontal(|ui| {
        if ui.small_button("+ Window").clicked() {
            if let Some(anim) = state.sprite_data.animations.get_mut(&anim_name) {
                let start = total_duration / 4;
                let end = total_duration * 3 / 4;
                let window = AnimationWindow::new("New Window".to_string(), start, end);
                anim.windows.push(window);
                result.changed = true;
            }
        }
        if ui.small_button("+ Trigger").clicked() {
            if let Some(anim) = state.sprite_data.animations.get_mut(&anim_name) {
                let trigger = AnimationTrigger::new("New Trigger".to_string(), total_duration / 2);
                anim.triggers.push(trigger);
                result.changed = true;
            }
        }
        ui.separator();
        ui.label("Zoom:");
        ui.add(
            egui::Slider::new(&mut state.timeline_zoom, 20.0..=150.0)
                .show_value(false)
                .logarithmic(true),
        );
    });

    ui.separator();

    // Scrollable dopesheet
    let label_width = 70.0;
    let ruler_height = 20.0;
    let timeline_width = (total_duration as f32 / 100.0) * state.timeline_zoom;

    // Dynamic track heights - one row per item
    let item_row_height = 22.0;
    let section_header_height = 18.0;
    let frames_track_height = 28.0; // Fixed for frames track
    let windows_track_height =
        section_header_height + windows.len().max(1) as f32 * item_row_height;
    let triggers_track_height =
        section_header_height + triggers.len().max(1) as f32 * item_row_height;
    let total_height =
        ruler_height + frames_track_height + windows_track_height + triggers_track_height;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (response, painter) = ui.allocate_painter(
                egui::vec2((timeline_width + label_width).max(400.0), total_height),
                egui::Sense::click_and_drag(),
            );
            let rect = response.rect;

            // Background
            painter.rect_filled(rect, 0.0, egui::Color32::from_gray(25));

            // Font sizes - scale ruler with zoom for readability
            let base_ruler_font = 11.0;
            let scaled_ruler_font = base_ruler_font * (state.timeline_zoom / 50.0).clamp(0.8, 1.5);

            // Time ruler (top)
            let ruler_rect =
                egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), ruler_height));
            painter.rect_filled(ruler_rect, 0.0, egui::Color32::from_gray(35));

            // Draw time markers
            let ms_per_tick = 100.0;
            let num_ticks = (total_duration as f32 / ms_per_tick).ceil() as usize + 1;
            for i in 0..num_ticks {
                let time_ms = i as f32 * ms_per_tick;
                let x = rect.min.x + label_width + (time_ms / 100.0) * state.timeline_zoom;

                // Tick line
                painter.line_segment(
                    [
                        egui::pos2(x, rect.min.y),
                        egui::pos2(x, rect.min.y + ruler_height),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
                );

                // Time label - scaled with zoom
                painter.text(
                    egui::pos2(x + 2.0, rect.min.y + 2.0),
                    egui::Align2::LEFT_TOP,
                    format!("{}ms", time_ms as u32),
                    egui::FontId::proportional(scaled_ruler_font),
                    egui::Color32::from_gray(180),
                );
            }

            // Draw frame boundaries
            let frame_width_px = (frame_duration_ms as f32 / 100.0) * state.timeline_zoom;
            for i in 0..=frames.len() {
                let x = rect.min.x + label_width + i as f32 * frame_width_px;
                painter.line_segment(
                    [
                        egui::pos2(x, rect.min.y + ruler_height),
                        egui::pos2(x, rect.max.y),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::from_gray(40)),
                );
            }

            // Track backgrounds and labels with dynamic heights
            let track_infos = [
                ("Frames", frames_track_height, 0usize), // (label, height, item_count)
                ("Windows", windows_track_height, windows.len()),
                ("Triggers", triggers_track_height, triggers.len()),
            ];
            let mut cumulative_y = rect.min.y + ruler_height;
            for (track_idx, (label, height, item_count)) in track_infos.iter().enumerate() {
                // Main track background
                let track_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.min.x, cumulative_y),
                    egui::vec2(rect.width(), *height),
                );
                let bg_color = if track_idx % 2 == 0 {
                    egui::Color32::from_gray(30)
                } else {
                    egui::Color32::from_gray(28)
                };
                painter.rect_filled(track_rect, 0.0, bg_color);

                // Draw alternating row backgrounds for multi-row tracks
                if track_idx > 0 && *item_count > 0 {
                    for row in 0..*item_count {
                        let row_y =
                            cumulative_y + section_header_height + row as f32 * item_row_height;
                        if row % 2 == 1 {
                            let row_rect = egui::Rect::from_min_size(
                                egui::pos2(rect.min.x + label_width, row_y),
                                egui::vec2(rect.width() - label_width, item_row_height),
                            );
                            painter.rect_filled(row_rect, 0.0, egui::Color32::from_gray(35));
                        }
                    }
                }

                // Track label background (full height)
                let label_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.min.x, cumulative_y),
                    egui::vec2(label_width, *height),
                );
                painter.rect_filled(label_rect, 0.0, egui::Color32::from_gray(40));

                // Track section header label (at top of section)
                painter.text(
                    egui::pos2(rect.min.x + 4.0, cumulative_y + section_header_height / 2.0),
                    egui::Align2::LEFT_CENTER,
                    *label,
                    egui::FontId::proportional(12.0),
                    egui::Color32::from_gray(200),
                );

                cumulative_y += height;
            }

            // Track 0: Frames (keyframes at START time, not centered)
            let frames_track_y = rect.min.y + ruler_height + frames_track_height / 2.0;
            for (i, &frame_num) in frames.iter().enumerate() {
                let time_ms = i as f32 * frame_duration_ms as f32;
                let x = rect.min.x + label_width + (time_ms / 100.0) * state.timeline_zoom;

                // Draw small rectangle with frame number - LEFT EDGE at start time
                let key_size = 18.0;
                let key_rect = egui::Rect::from_min_size(
                    egui::pos2(x, frames_track_y - key_size / 2.0), // Left edge at start time, centered vertically
                    egui::vec2(key_size, key_size),
                );
                painter.rect_filled(key_rect, 3.0, egui::Color32::from_rgb(100, 150, 220));
                painter.text(
                    key_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{}", frame_num),
                    egui::FontId::proportional(10.0),
                    egui::Color32::WHITE,
                );
            }

            // Track 1: Windows (bars) - one row per window
            let windows_base_y = rect.min.y + ruler_height + frames_track_height;
            for (row_idx, window) in windows.iter().enumerate() {
                let row_y = windows_base_y
                    + section_header_height
                    + (row_idx as f32 + 0.5) * item_row_height;
                let start_x = rect.min.x
                    + label_width
                    + (window.start_ms as f32 / 100.0) * state.timeline_zoom;
                let end_x =
                    rect.min.x + label_width + (window.end_ms as f32 / 100.0) * state.timeline_zoom;
                let bar_height = 16.0;
                let window_rect = egui::Rect::from_min_max(
                    egui::pos2(start_x, row_y - bar_height / 2.0),
                    egui::pos2(end_x, row_y + bar_height / 2.0),
                );

                let is_selected = state.selected_window == Some(window.id);
                let base_color = window.color.unwrap_or([60, 140, 60]);
                let color = if is_selected {
                    // Brighten the color when selected
                    egui::Color32::from_rgb(
                        base_color[0].saturating_add(40),
                        base_color[1].saturating_add(60),
                        base_color[2].saturating_add(40),
                    )
                } else {
                    egui::Color32::from_rgb(base_color[0], base_color[1], base_color[2])
                };

                painter.rect_filled(window_rect, 4.0, color);
                painter.rect_stroke(
                    window_rect,
                    4.0,
                    egui::Stroke::new(1.0, egui::Color32::WHITE),
                    egui::StrokeKind::Outside,
                );

                // Window name
                let text_x = start_x + 4.0;
                painter.text(
                    egui::pos2(text_x, row_y),
                    egui::Align2::LEFT_CENTER,
                    &window.name,
                    egui::FontId::proportional(10.0),
                    egui::Color32::WHITE,
                );
            }

            // Track 2: Triggers (diamonds) - one row per trigger
            let triggers_base_y =
                rect.min.y + ruler_height + frames_track_height + windows_track_height;
            for (row_idx, trigger) in triggers.iter().enumerate() {
                let row_y = triggers_base_y
                    + section_header_height
                    + (row_idx as f32 + 0.5) * item_row_height;
                let x = rect.min.x
                    + label_width
                    + (trigger.time_ms as f32 / 100.0) * state.timeline_zoom;

                let is_selected = state.selected_trigger == Some(trigger.id);
                let base_color = trigger.color.unwrap_or([255, 140, 40]);
                let color = if is_selected {
                    // Brighten the color when selected
                    egui::Color32::from_rgb(
                        base_color[0].saturating_add(0),
                        base_color[1].saturating_add(60),
                        base_color[2].saturating_add(60),
                    )
                } else {
                    egui::Color32::from_rgb(base_color[0], base_color[1], base_color[2])
                };

                // Diamond shape
                let size = 7.0;
                let points = vec![
                    egui::pos2(x, row_y - size),
                    egui::pos2(x + size, row_y),
                    egui::pos2(x, row_y + size),
                    egui::pos2(x - size, row_y),
                ];
                painter.add(egui::Shape::convex_polygon(
                    points,
                    color,
                    egui::Stroke::new(1.0, egui::Color32::WHITE),
                ));

                // Trigger name (to the right of diamond)
                painter.text(
                    egui::pos2(x + size + 4.0, row_y),
                    egui::Align2::LEFT_CENTER,
                    &trigger.name,
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_gray(220),
                );
            }

            // Draw playhead
            if !frames.is_empty() {
                let playhead_time = state.preview_frame as f32 * frame_duration_ms as f32;
                let playhead_x =
                    rect.min.x + label_width + (playhead_time / 100.0) * state.timeline_zoom;
                painter.line_segment(
                    [
                        egui::pos2(playhead_x, rect.min.y),
                        egui::pos2(playhead_x, rect.max.y),
                    ],
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 80, 80)),
                );
            }

            // Helper to determine which track and row was clicked based on dynamic heights
            let get_track_and_row = |y: f32| -> (i32, usize) {
                let rel_y = y - rect.min.y - ruler_height;
                if rel_y < 0.0 {
                    (-1, 0) // Ruler area
                } else if rel_y < frames_track_height {
                    (0, 0) // Frames track
                } else if rel_y < frames_track_height + windows_track_height {
                    let row_rel_y = rel_y - frames_track_height - section_header_height;
                    let row = (row_rel_y / item_row_height).max(0.0) as usize;
                    (1, row) // Windows track with row
                } else {
                    let row_rel_y =
                        rel_y - frames_track_height - windows_track_height - section_header_height;
                    let row = (row_rel_y / item_row_height).max(0.0) as usize;
                    (2, row) // Triggers track with row
                }
            };

            // Handle clicks for selection
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let timeline_x = pos.x - rect.min.x - label_width;
                    let (track_idx, row_idx) = get_track_and_row(pos.y);

                    match track_idx {
                        1 => {
                            // Windows track - check if clicked row matches a window's time range
                            let mut found = false;
                            if row_idx < windows.len() {
                                let window = &windows[row_idx];
                                let start_x =
                                    (window.start_ms as f32 / 100.0) * state.timeline_zoom;
                                let end_x = (window.end_ms as f32 / 100.0) * state.timeline_zoom;
                                if timeline_x >= start_x - 5.0 && timeline_x <= end_x + 5.0 {
                                    state.selected_window = Some(window.id);
                                    state.selected_trigger = None;
                                    found = true;
                                }
                            }
                            if !found {
                                state.selected_window = None;
                            }
                        }
                        2 => {
                            // Triggers track - check if clicked row matches a trigger
                            let mut found = false;
                            if row_idx < triggers.len() {
                                let trigger = &triggers[row_idx];
                                let trigger_x =
                                    (trigger.time_ms as f32 / 100.0) * state.timeline_zoom;
                                if (timeline_x - trigger_x).abs() < 20.0 {
                                    state.selected_trigger = Some(trigger.id);
                                    state.selected_window = None;
                                    found = true;
                                }
                            }
                            if !found {
                                state.selected_trigger = None;
                            }
                        }
                        _ => {
                            // Clicked elsewhere, deselect
                            state.selected_window = None;
                            state.selected_trigger = None;
                        }
                    }
                }
            }

            // Handle drag start - identify what's being dragged
            if response.drag_started() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let timeline_x = pos.x - rect.min.x - label_width;
                    let (track_idx, row_idx) = get_track_and_row(pos.y);

                    // Check for window drag (track 1)
                    if track_idx == 1 && row_idx < windows.len() {
                        let window = &windows[row_idx];
                        let start_x = (window.start_ms as f32 / 100.0) * state.timeline_zoom;
                        let end_x = (window.end_ms as f32 / 100.0) * state.timeline_zoom;
                        let edge_threshold = 8.0; // pixels

                        if (timeline_x - start_x).abs() < edge_threshold {
                            // Dragging start edge
                            state.dragging_window = Some((window.id, DragHandle::Start));
                            state.drag_start_time = window.start_ms;
                            state.drag_original_end = window.end_ms;
                            state.selected_window = Some(window.id);
                            state.selected_trigger = None;
                        } else if (timeline_x - end_x).abs() < edge_threshold {
                            // Dragging end edge
                            state.dragging_window = Some((window.id, DragHandle::End));
                            state.drag_start_time = window.start_ms;
                            state.drag_original_end = window.end_ms;
                            state.selected_window = Some(window.id);
                            state.selected_trigger = None;
                        } else if timeline_x > start_x - 5.0 && timeline_x < end_x + 5.0 {
                            // Dragging body (move entire window)
                            state.dragging_window = Some((window.id, DragHandle::Body));
                            state.drag_start_time = window.start_ms;
                            state.drag_original_end = window.end_ms;
                            state.selected_window = Some(window.id);
                            state.selected_trigger = None;
                        }
                    }

                    // Check for trigger drag (track 2)
                    if track_idx == 2 && row_idx < triggers.len() && state.dragging_window.is_none()
                    {
                        let trigger = &triggers[row_idx];
                        let trigger_x = (trigger.time_ms as f32 / 100.0) * state.timeline_zoom;
                        if (timeline_x - trigger_x).abs() < 20.0 {
                            state.dragging_trigger = Some(trigger.id);
                            state.drag_start_time = trigger.time_ms;
                            state.selected_trigger = Some(trigger.id);
                            state.selected_window = None;
                        }
                    }
                }
            }

            // Handle dragging - update positions
            if response.dragged() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let timeline_x = (pos.x - rect.min.x - label_width).max(0.0);
                    let time_ms = ((timeline_x / state.timeline_zoom) * 100.0) as u32;

                    // Drag window
                    if let Some((window_id, handle)) = state.dragging_window {
                        if let Some(anim) = state.sprite_data.animations.get_mut(&anim_name) {
                            if let Some(window) =
                                anim.windows.iter_mut().find(|w| w.id == window_id)
                            {
                                match handle {
                                    DragHandle::Start => {
                                        // Dragging start edge - don't let it exceed end
                                        window.start_ms =
                                            time_ms.min(window.end_ms.saturating_sub(10));
                                    }
                                    DragHandle::End => {
                                        // Dragging end edge - don't let it be before start
                                        window.end_ms = time_ms.max(window.start_ms + 10);
                                    }
                                    DragHandle::Body => {
                                        // Move entire window, preserving duration
                                        let duration = state
                                            .drag_original_end
                                            .saturating_sub(state.drag_start_time);
                                        window.start_ms = time_ms;
                                        window.end_ms = time_ms + duration;
                                    }
                                }
                                result.changed = true;
                            }
                        }
                    }

                    // Drag trigger
                    if let Some(trigger_id) = state.dragging_trigger {
                        if let Some(anim) = state.sprite_data.animations.get_mut(&anim_name) {
                            if let Some(trigger) =
                                anim.triggers.iter_mut().find(|t| t.id == trigger_id)
                            {
                                trigger.time_ms = time_ms;
                                result.changed = true;
                            }
                        }
                    }
                }
            }

            // Handle drag end - clear drag state
            if response.drag_stopped() {
                state.dragging_trigger = None;
                state.dragging_window = None;
            }

            // Handle right-click context menu
            response.context_menu(|ui| {
                if let Some(pos) = ui.ctx().pointer_interact_pos() {
                    let timeline_x = pos.x - rect.min.x - label_width;
                    let time_ms = ((timeline_x / state.timeline_zoom) * 100.0).max(0.0) as u32;
                    let (track_idx, row_idx) = get_track_and_row(pos.y);

                    // Check if we clicked on an existing item (row-based)
                    let clicked_window = if track_idx == 1 && row_idx < windows.len() {
                        let window = &windows[row_idx];
                        let start_x = (window.start_ms as f32 / 100.0) * state.timeline_zoom;
                        let end_x = (window.end_ms as f32 / 100.0) * state.timeline_zoom;
                        if timeline_x >= start_x - 5.0 && timeline_x <= end_x + 5.0 {
                            Some(window)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let clicked_trigger = if track_idx == 2 && row_idx < triggers.len() {
                        let trigger = &triggers[row_idx];
                        let trigger_x = (trigger.time_ms as f32 / 100.0) * state.timeline_zoom;
                        if (timeline_x - trigger_x).abs() < 20.0 {
                            Some(trigger)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some(window) = clicked_window {
                        // Context menu for existing window
                        let window_id = window.id;
                        ui.label(format!("Window: {}", window.name));
                        ui.separator();
                        if ui.button("Delete Window").clicked() {
                            if let Some(anim) = state.sprite_data.animations.get_mut(&anim_name) {
                                anim.windows.retain(|w| w.id != window_id);
                                state.selected_window = None;
                                result.changed = true;
                            }
                            ui.close();
                        }
                        if ui.button("Duplicate").clicked() {
                            if let Some(anim) = state.sprite_data.animations.get_mut(&anim_name) {
                                if let Some(w) =
                                    anim.windows.iter().find(|w| w.id == window_id).cloned()
                                {
                                    let mut new_window = w.clone();
                                    new_window.id = Uuid::new_v4();
                                    new_window.name = format!("{}_copy", w.name);
                                    new_window.start_ms = new_window.start_ms.saturating_add(50);
                                    new_window.end_ms = new_window.end_ms.saturating_add(50);
                                    anim.windows.push(new_window);
                                    result.changed = true;
                                }
                            }
                            ui.close();
                        }
                    } else if let Some(trigger) = clicked_trigger {
                        // Context menu for existing trigger
                        let trigger_id = trigger.id;
                        ui.label(format!("Trigger: {}", trigger.name));
                        ui.separator();
                        if ui.button("Delete Trigger").clicked() {
                            if let Some(anim) = state.sprite_data.animations.get_mut(&anim_name) {
                                anim.triggers.retain(|t| t.id != trigger_id);
                                state.selected_trigger = None;
                                result.changed = true;
                            }
                            ui.close();
                        }
                        if ui.button("Duplicate").clicked() {
                            if let Some(anim) = state.sprite_data.animations.get_mut(&anim_name) {
                                if let Some(t) =
                                    anim.triggers.iter().find(|t| t.id == trigger_id).cloned()
                                {
                                    let mut new_trigger = t.clone();
                                    new_trigger.id = Uuid::new_v4();
                                    new_trigger.name = format!("{}_copy", t.name);
                                    new_trigger.time_ms = new_trigger.time_ms.saturating_add(50);
                                    anim.triggers.push(new_trigger);
                                    result.changed = true;
                                }
                            }
                            ui.close();
                        }
                    } else {
                        // Context menu for empty area
                        ui.label(format!("At: {}ms", time_ms));
                        ui.separator();
                        if ui.button("Add Trigger Here").clicked() {
                            if let Some(anim) = state.sprite_data.animations.get_mut(&anim_name) {
                                let new_trigger = AnimationTrigger::new("new_trigger", time_ms);
                                anim.triggers.push(new_trigger);
                                result.changed = true;
                            }
                            ui.close();
                        }
                        if ui.button("Add Window Here").clicked() {
                            if let Some(anim) = state.sprite_data.animations.get_mut(&anim_name) {
                                let end_time = time_ms.saturating_add(100);
                                let new_window =
                                    AnimationWindow::new("new_window", time_ms, end_time);
                                anim.windows.push(new_window);
                                result.changed = true;
                            }
                            ui.close();
                        }
                    }
                }
            });
        });
}

// ============================================================================
// Details Panel (Compact)
// ============================================================================

/// Render compact details panel for selected trigger/window
fn render_details_panel(
    ui: &mut egui::Ui,
    state: &mut AnimationEditorState,
    result: &mut AnimationEditorResult,
) {
    let Some(anim_name) = state.selected_animation.clone() else {
        ui.label("No animation selected");
        return;
    };

    // Edit selected trigger
    if let Some(trigger_id) = state.selected_trigger {
        if let Some(anim) = state.sprite_data.animations.get_mut(&anim_name) {
            let total_duration = anim.total_duration_ms();
            let trigger_idx = anim.triggers.iter().position(|t| t.id == trigger_id);

            if let Some(idx) = trigger_idx {
                let mut delete_trigger = false;

                {
                    let trigger = &mut anim.triggers[idx];
                    ui.horizontal(|ui| {
                        ui.label("Trigger:");
                        ui.add(egui::TextEdit::singleline(&mut trigger.name).desired_width(100.0));

                        ui.separator();
                        ui.label("Time:");
                        let mut time = trigger.time_ms as i32;
                        if ui
                            .add(
                                egui::DragValue::new(&mut time)
                                    .range(0..=total_duration as i32)
                                    .suffix("ms"),
                            )
                            .changed()
                        {
                            trigger.time_ms = time.max(0) as u32;
                            result.changed = true;
                        }

                        ui.separator();

                        // Color picker
                        let default_color = [255u8, 140, 40]; // Default orange
                        let mut rgb = trigger.color.unwrap_or(default_color);
                        ui.label("Color:");
                        if egui::color_picker::color_edit_button_srgb(ui, &mut rgb).changed() {
                            trigger.color = Some(rgb);
                            result.changed = true;
                        }

                        ui.separator();
                        render_payload_selector(ui, &mut trigger.payload, result);
                    });
                }

                // Delete button outside the main horizontal to avoid borrow issues
                ui.horizontal(|ui| {
                    render_payload_details(ui, &mut anim.triggers[idx].payload, result);
                    if ui.button("🗑 Delete Trigger").clicked() {
                        delete_trigger = true;
                    }
                });

                if delete_trigger {
                    anim.triggers.remove(idx);
                    state.selected_trigger = None;
                    result.changed = true;
                }
            } else {
                state.selected_trigger = None;
            }
        }
        return;
    }

    // Edit selected window
    if let Some(window_id) = state.selected_window {
        if let Some(anim) = state.sprite_data.animations.get_mut(&anim_name) {
            let total_duration = anim.total_duration_ms();
            let window_idx = anim.windows.iter().position(|w| w.id == window_id);

            if let Some(idx) = window_idx {
                let mut delete_window = false;

                {
                    let window = &mut anim.windows[idx];
                    ui.horizontal(|ui| {
                        ui.label("Window:");
                        ui.add(egui::TextEdit::singleline(&mut window.name).desired_width(100.0));

                        ui.separator();
                        ui.label("Start:");
                        let mut start = window.start_ms as i32;
                        if ui
                            .add(
                                egui::DragValue::new(&mut start)
                                    .range(0..=window.end_ms as i32)
                                    .suffix("ms"),
                            )
                            .changed()
                        {
                            window.start_ms = start.max(0) as u32;
                            result.changed = true;
                        }

                        ui.label("End:");
                        let mut end = window.end_ms as i32;
                        if ui
                            .add(
                                egui::DragValue::new(&mut end)
                                    .range(window.start_ms as i32..=total_duration as i32)
                                    .suffix("ms"),
                            )
                            .changed()
                        {
                            window.end_ms = end as u32;
                            result.changed = true;
                        }

                        ui.separator();

                        // Color picker
                        let default_color = [60u8, 140, 60]; // Default green
                        let mut rgb = window.color.unwrap_or(default_color);
                        ui.label("Color:");
                        if egui::color_picker::color_edit_button_srgb(ui, &mut rgb).changed() {
                            window.color = Some(rgb);
                            result.changed = true;
                        }

                        ui.separator();
                        render_payload_selector(ui, &mut window.payload, result);
                    });
                }

                // Delete button outside the main horizontal to avoid borrow issues
                ui.horizontal(|ui| {
                    render_payload_details(ui, &mut anim.windows[idx].payload, result);
                    if ui.button("🗑 Delete Window").clicked() {
                        delete_window = true;
                    }
                });

                if delete_window {
                    anim.windows.remove(idx);
                    state.selected_window = None;
                    result.changed = true;
                }
            } else {
                state.selected_window = None;
            }
        }
        return;
    }

    ui.label("Select a trigger or window on the timeline to edit its properties");
}

/// Render compact payload type selector
fn render_payload_selector(
    ui: &mut egui::Ui,
    payload: &mut TriggerPayload,
    result: &mut AnimationEditorResult,
) {
    let current_type = payload.display_name();
    egui::ComboBox::from_id_salt("payload_type_selector")
        .width(80.0)
        .selected_text(current_type)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(matches!(payload, TriggerPayload::None), "None")
                .clicked()
            {
                *payload = TriggerPayload::None;
                result.changed = true;
            }
            if ui
                .selectable_label(matches!(payload, TriggerPayload::Sound { .. }), "Sound")
                .clicked()
            {
                *payload = TriggerPayload::Sound {
                    path: String::new(),
                    volume: 1.0,
                };
                result.changed = true;
            }
            if ui
                .selectable_label(
                    matches!(payload, TriggerPayload::Particle { .. }),
                    "Particle",
                )
                .clicked()
            {
                *payload = TriggerPayload::Particle {
                    effect: String::new(),
                    offset: (0.0, 0.0),
                };
                result.changed = true;
            }
            if ui
                .selectable_label(matches!(payload, TriggerPayload::Custom { .. }), "Custom")
                .clicked()
            {
                *payload = TriggerPayload::Custom {
                    event_name: String::new(),
                    params: HashMap::new(),
                };
                result.changed = true;
            }
        });
}

/// Render payload-specific details
fn render_payload_details(
    ui: &mut egui::Ui,
    payload: &mut TriggerPayload,
    result: &mut AnimationEditorResult,
) {
    match payload {
        TriggerPayload::Sound { path, volume } => {
            ui.horizontal(|ui| {
                ui.label("Path:");
                if ui
                    .add(egui::TextEdit::singleline(path).desired_width(200.0))
                    .changed()
                {
                    result.changed = true;
                }
                ui.label("Vol:");
                if ui
                    .add(egui::Slider::new(volume, 0.0..=1.0).show_value(false))
                    .changed()
                {
                    result.changed = true;
                }
            });
        }
        TriggerPayload::Particle { effect, offset } => {
            ui.horizontal(|ui| {
                ui.label("Effect:");
                if ui
                    .add(egui::TextEdit::singleline(effect).desired_width(150.0))
                    .changed()
                {
                    result.changed = true;
                }
                ui.label("Offset:");
                if ui
                    .add(egui::DragValue::new(&mut offset.0).prefix("X:"))
                    .changed()
                {
                    result.changed = true;
                }
                if ui
                    .add(egui::DragValue::new(&mut offset.1).prefix("Y:"))
                    .changed()
                {
                    result.changed = true;
                }
            });
        }
        TriggerPayload::Custom {
            event_name,
            params: _,
        } => {
            ui.horizontal(|ui| {
                ui.label("Event:");
                if ui
                    .add(egui::TextEdit::singleline(event_name).desired_width(200.0))
                    .changed()
                {
                    result.changed = true;
                }
                ui.label("(params editor coming soon)");
            });
        }
        TriggerPayload::None => {}
    }
}

// ============================================================================
// Frame Picker (Collapsible)
// ============================================================================

/// Render the frame picker panel
fn render_frame_picker(
    ui: &mut egui::Ui,
    state: &mut AnimationEditorState,
    result: &mut AnimationEditorResult,
) {
    ui.horizontal(|ui| {
        ui.label("Frame Picker");
        ui.label("|");
        ui.label(format!("Selected: {:?}", state.selected_frames));

        if ui.button("Apply to Animation").clicked() {
            if let Some(anim_name) = &state.selected_animation {
                if let Some(anim) = state.sprite_data.animations.get_mut(anim_name) {
                    anim.frames = state.selected_frames.clone();
                    result.changed = true;
                }
            }
        }
        if ui.button("Clear").clicked() {
            state.selected_frames.clear();
        }

        ui.separator();
        ui.label("Zoom:");
        if ui.button("-").clicked() {
            state.zoom = (state.zoom - 0.5).max(0.5);
        }
        ui.label(format!("{:.0}%", state.zoom * 100.0));
        if ui.button("+").clicked() {
            state.zoom = (state.zoom + 0.5).min(4.0);
        }
    });

    ui.separator();

    // Spritesheet grid
    let frame_w = state.sprite_data.frame_width as f32 * state.zoom;
    let frame_h = state.sprite_data.frame_height as f32 * state.zoom;
    let cols = state.sprite_data.columns.max(1);
    let rows = state.sprite_data.rows.max(1);
    let total_w = frame_w * cols as f32;
    let total_h = frame_h * rows as f32;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (response, painter) =
                ui.allocate_painter(egui::vec2(total_w, total_h), egui::Sense::click());
            let rect = response.rect;

            // Background
            painter.rect_filled(rect, 0.0, egui::Color32::from_gray(40));

            // Draw spritesheet
            if let (Some(texture_id), Some((img_width, img_height))) =
                (state.spritesheet_texture_id, state.spritesheet_size)
            {
                let grid_width = cols as f32 * state.sprite_data.frame_width as f32;
                let grid_height = rows as f32 * state.sprite_data.frame_height as f32;
                let u_max = (grid_width / img_width).min(1.0);
                let v_max = (grid_height / img_height).min(1.0);

                let mesh = egui::Mesh {
                    texture_id,
                    indices: vec![0, 1, 2, 0, 2, 3],
                    vertices: vec![
                        egui::epaint::Vertex {
                            pos: rect.min,
                            uv: egui::pos2(0.0, 0.0),
                            color: egui::Color32::WHITE,
                        },
                        egui::epaint::Vertex {
                            pos: egui::pos2(rect.max.x, rect.min.y),
                            uv: egui::pos2(u_max, 0.0),
                            color: egui::Color32::WHITE,
                        },
                        egui::epaint::Vertex {
                            pos: rect.max,
                            uv: egui::pos2(u_max, v_max),
                            color: egui::Color32::WHITE,
                        },
                        egui::epaint::Vertex {
                            pos: egui::pos2(rect.min.x, rect.max.y),
                            uv: egui::pos2(0.0, v_max),
                            color: egui::Color32::WHITE,
                        },
                    ],
                };
                painter.add(egui::Shape::mesh(mesh));
            }

            // Draw grid overlay
            for row in 0..rows {
                for col in 0..cols {
                    let frame_idx = state.sprite_data.grid_to_frame(col, row);
                    let x = rect.min.x + col as f32 * frame_w;
                    let y = rect.min.y + row as f32 * frame_h;
                    let frame_rect =
                        egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(frame_w, frame_h));

                    // Highlight selected frames
                    if state.selected_frames.contains(&frame_idx) {
                        painter.rect_filled(
                            frame_rect,
                            0.0,
                            egui::Color32::from_rgba_unmultiplied(100, 150, 255, 100),
                        );
                    }

                    // Grid lines
                    painter.rect_stroke(
                        frame_rect,
                        0.0,
                        egui::Stroke::new(
                            1.0,
                            egui::Color32::from_rgba_unmultiplied(200, 200, 200, 100),
                        ),
                        egui::StrokeKind::Middle,
                    );

                    // Frame number
                    painter.text(
                        egui::pos2(x + 2.0, y + 2.0),
                        egui::Align2::LEFT_TOP,
                        format!("{}", frame_idx),
                        egui::FontId::proportional(9.0 * state.zoom.max(0.5)),
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180),
                    );
                }
            }

            // Handle clicks
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let local_x = pos.x - rect.min.x;
                    let local_y = pos.y - rect.min.y;
                    let col = (local_x / frame_w) as u32;
                    let row = (local_y / frame_h) as u32;

                    if col < cols && row < rows {
                        let frame_idx = state.sprite_data.grid_to_frame(col, row);
                        if ui.input(|i| i.modifiers.ctrl) {
                            // Toggle
                            if let Some(pos) =
                                state.selected_frames.iter().position(|&f| f == frame_idx)
                            {
                                state.selected_frames.remove(pos);
                            } else {
                                state.selected_frames.push(frame_idx);
                            }
                        } else {
                            // Add
                            state.selected_frames.push(frame_idx);
                        }
                    }
                }
            }
        });
}

// ============================================================================
// Floating Preview Window
// ============================================================================

/// Render the floating preview window
fn render_floating_preview(ctx: &egui::Context, state: &mut AnimationEditorState) {
    if !state.show_preview {
        return;
    }

    let mut show = state.show_preview;
    egui::Window::new("Preview")
        .open(&mut show) // X button to close
        .resizable(true)
        .min_size([100.0, 100.0])
        .default_size([150.0, 180.0])
        .default_pos([750.0, 100.0])
        .show(ctx, |ui| {
            let Some(texture_id) = state.spritesheet_texture_id else {
                ui.label("No texture loaded");
                return;
            };
            let Some((img_width, img_height)) = state.spritesheet_size else {
                ui.label("No texture size");
                return;
            };
            let Some(anim_name) = &state.selected_animation else {
                ui.label("No animation selected");
                return;
            };
            let Some(anim) = state.sprite_data.animations.get(anim_name) else {
                ui.label("Animation not found");
                return;
            };

            if anim.frames.is_empty() {
                ui.label("No frames");
                return;
            }

            let frame_w = state.sprite_data.frame_width as f32;
            let frame_h = state.sprite_data.frame_height as f32;
            let cols = state.sprite_data.columns.max(1);

            let frame_idx = state.preview_frame % anim.frames.len();
            let frame_num = anim.frames[frame_idx];
            let col = frame_num as u32 % cols;
            let row = frame_num as u32 / cols;

            let u0 = (col as f32 * frame_w) / img_width;
            let v0 = (row as f32 * frame_h) / img_height;
            let u1 = ((col + 1) as f32 * frame_w) / img_width;
            let v1 = ((row + 1) as f32 * frame_h) / img_height;

            // Scale to fit available window space
            let available = ui.available_size();
            let label_reserve = 24.0; // Space for frame counter label
            let padding = 8.0;
            let available_width = (available.x - padding).max(50.0);
            let available_height = (available.y - label_reserve - padding).max(50.0);

            let scale_x = available_width / frame_w;
            let scale_y = available_height / frame_h;
            let scale = scale_x.min(scale_y).max(1.0);
            let display_size = egui::vec2(frame_w * scale, frame_h * scale);

            ui.vertical_centered(|ui| {
                ui.add(
                    egui::Image::new(egui::load::SizedTexture::new(texture_id, display_size)).uv(
                        egui::Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u1, v1)),
                    ),
                );
            });

            ui.label(format!("Frame {} / {}", frame_idx + 1, anim.frames.len()));
        });

    // Update state if closed via X button
    state.show_preview = show;
}

// ============================================================================
// Preview Timer Update
// ============================================================================

/// Update animation preview timer
fn update_preview_timer(ui: &egui::Ui, state: &mut AnimationEditorState) {
    if !state.preview_playing {
        return;
    }

    let Some(anim_name) = &state.selected_animation else {
        return;
    };
    let Some(anim) = state.sprite_data.animations.get(anim_name) else {
        return;
    };

    if anim.frames.is_empty() {
        return;
    }

    let dt = ui.input(|i| i.stable_dt);
    state.preview_timer += dt * 1000.0;

    let frame_duration = anim.frame_duration_ms as f32;
    if state.preview_timer >= frame_duration {
        state.preview_timer -= frame_duration;
        state.preview_frame = (state.preview_frame + 1) % anim.frames.len();
    }

    ui.ctx().request_repaint();
}
