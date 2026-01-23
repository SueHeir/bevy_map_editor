//! Code preview dialog
//!
//! Shows a preview of generated code before writing to disk.

use std::path::PathBuf;

use bevy_egui::egui;

use crate::external_editor;

/// Tab selection for code preview
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CodePreviewTab {
    #[default]
    Entities,
    Enums,
    Stubs,
    Behaviors,
}

impl CodePreviewTab {
    pub fn all() -> &'static [CodePreviewTab] {
        &[
            CodePreviewTab::Entities,
            CodePreviewTab::Enums,
            CodePreviewTab::Stubs,
            CodePreviewTab::Behaviors,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            CodePreviewTab::Entities => "Entities",
            CodePreviewTab::Enums => "Enums",
            CodePreviewTab::Stubs => "Stubs",
            CodePreviewTab::Behaviors => "Behaviors",
        }
    }
}

/// State for the code preview dialog
#[derive(Default)]
pub struct CodePreviewDialogState {
    /// Whether the dialog is open
    pub open: bool,
    /// Currently selected tab
    pub selected_tab: CodePreviewTab,
    /// Generated entities code
    pub entities_code: String,
    /// Generated enums code
    pub enums_code: String,
    /// Generated stubs code
    pub stubs_code: String,
    /// Generated behaviors code
    pub behaviors_code: String,
    /// Error message if generation failed
    pub error: Option<String>,
    /// Scroll position for each tab
    pub scroll_positions: [f32; 4],
    /// Output path for generated code files (for opening in VS Code)
    pub output_path: Option<PathBuf>,
    /// Custom VS Code path (from project config)
    pub vscode_path: Option<String>,
    /// Cached VS Code availability status
    pub vscode_available: bool,
}

impl CodePreviewDialogState {
    /// Set the preview content
    pub fn set_content(
        &mut self,
        entities: String,
        enums: String,
        stubs: String,
        behaviors: String,
    ) {
        self.entities_code = entities;
        self.enums_code = enums;
        self.stubs_code = stubs;
        self.behaviors_code = behaviors;
        self.error = None;
    }

    /// Set an error message
    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
    }

    /// Get the current tab's code
    fn current_code(&self) -> &str {
        match self.selected_tab {
            CodePreviewTab::Entities => &self.entities_code,
            CodePreviewTab::Enums => &self.enums_code,
            CodePreviewTab::Stubs => &self.stubs_code,
            CodePreviewTab::Behaviors => &self.behaviors_code,
        }
    }

    /// Get the file path for the current tab's generated file
    pub fn current_file_path(&self) -> Option<PathBuf> {
        self.output_path.as_ref().map(|base| {
            let filename = match self.selected_tab {
                CodePreviewTab::Entities => "entities.rs",
                CodePreviewTab::Enums => "enums.rs",
                CodePreviewTab::Stubs => "stubs.rs",
                CodePreviewTab::Behaviors => "behaviors.rs",
            };
            base.join(filename)
        })
    }
}

/// Render the code preview dialog
pub fn render_code_preview_dialog(ctx: &egui::Context, state: &mut CodePreviewDialogState) -> bool {
    let mut close_requested = false;

    if !state.open {
        return false;
    }

    // Modal overlay
    egui::Area::new(egui::Id::new("code_preview_modal_overlay"))
        .fixed_pos(egui::pos2(0.0, 0.0))
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            let screen_rect = ctx.input(|i| {
                i.raw.screen_rect.unwrap_or(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1920.0, 1080.0),
                ))
            });
            let response = ui.allocate_response(screen_rect.size(), egui::Sense::click_and_drag());
            ui.painter()
                .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(128));
            response.context_menu(|_| {});
        });

    egui::Window::new("Generated Code Preview")
        .collapsible(false)
        .resizable(true)
        .default_width(700.0)
        .default_height(500.0)
        .min_width(400.0)
        .min_height(300.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            // Error display
            if let Some(ref error) = state.error {
                ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
                ui.separator();
            }

            // Tab bar
            ui.horizontal(|ui| {
                for tab in CodePreviewTab::all() {
                    let is_selected = state.selected_tab == *tab;
                    if ui.selectable_label(is_selected, tab.label()).clicked() {
                        state.selected_tab = *tab;
                    }
                }
            });

            ui.separator();

            // Clone the code to avoid borrow conflicts with state mutation
            let code = state.current_code().to_string();
            let line_count = code.lines().count();

            // Calculate available height for the code area
            let available_height = ui.available_height() - 30.0; // Reserve space for bottom bar

            // Code display - fills available space
            egui::ScrollArea::vertical()
                .id_salt("code_preview_scroll")
                .max_height(available_height.max(100.0))
                .show(ui, |ui| {
                    let available_width = ui.available_width();
                    ui.add(
                        egui::TextEdit::multiline(&mut code.clone())
                            .font(egui::TextStyle::Monospace)
                            .code_editor()
                            .desired_width(available_width)
                            .interactive(false),
                    );
                });

            ui.separator();

            // Action buttons
            ui.horizontal(|ui| {
                ui.label(format!("{} lines", line_count));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        state.open = false;
                        close_requested = true;
                    }

                    // Copy to clipboard
                    if ui.button("Copy to Clipboard").clicked() {
                        ctx.copy_text(code.clone());
                    }

                    // Open current tab's file in VS Code (using cached status)
                    if state.vscode_available {
                        if let Some(file_path) = state.current_file_path() {
                            let file_exists = file_path.exists();
                            ui.add_enabled_ui(file_exists, |ui| {
                                if ui.button("Open in VS Code").clicked() {
                                    let _ = external_editor::open_in_vscode_with_custom_path(
                                        &file_path,
                                        state.vscode_path.as_deref(),
                                    );
                                }
                            });
                            if !file_exists {
                                ui.label("(Generate first to open file)");
                            }
                        }
                    }
                });
            });
        });

    close_requested
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_preview_tab() {
        assert_eq!(CodePreviewTab::all().len(), 4);
        assert_eq!(CodePreviewTab::Entities.label(), "Entities");
    }

    #[test]
    fn test_code_preview_state() {
        let mut state = CodePreviewDialogState::default();

        state.set_content(
            "entities".to_string(),
            "enums".to_string(),
            "stubs".to_string(),
            "behaviors".to_string(),
        );

        assert_eq!(state.current_code(), "entities");

        state.selected_tab = CodePreviewTab::Enums;
        assert_eq!(state.current_code(), "enums");
    }
}
