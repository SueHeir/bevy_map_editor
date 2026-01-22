//! Game project settings dialog
//!
//! This dialog allows users to configure the associated game project,
//! including the project path, starting level, build options, and code generation.

use bevy_egui::egui;
use std::path::PathBuf;
use uuid::Uuid;

use crate::bevy_cli;
use crate::external_editor::{self, PreferredEditor};
use crate::project::Project;

/// State for the game settings dialog
#[derive(Default)]
pub struct GameSettingsDialogState {
    /// Whether the dialog is open
    pub open: bool,
    /// Full path to the game project (e.g., C:\Dev\Games\my_game)
    pub project_path_input: String,
    /// Selected starting level ID
    pub selected_starting_level: Option<Uuid>,
    /// Whether to use release build
    pub use_release_build: bool,
    /// Status message to display
    pub status_message: Option<String>,
    /// Whether Bevy CLI is installed (cached)
    pub cli_installed: Option<bool>,

    // Code generation settings
    /// Whether code generation is enabled
    pub enable_codegen: bool,
    /// Output path for generated code
    pub codegen_output_path: String,
    /// Whether to generate entity structs
    pub generate_entities: bool,
    /// Whether to generate stub systems
    pub generate_stubs: bool,
    /// Whether to generate behavior systems
    pub generate_behaviors: bool,
    /// Whether to generate enums
    pub generate_enums: bool,
    /// Preferred external editor
    pub preferred_editor: PreferredEditor,
}

impl GameSettingsDialogState {
    /// Initialize dialog state from project config
    pub fn load_from_project(&mut self, project: &Project) {
        self.project_path_input = project
            .game_config
            .project_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        self.selected_starting_level = project.game_config.starting_level;
        self.use_release_build = project.game_config.use_release_build;
        self.status_message = None;

        // Load codegen settings
        self.enable_codegen = project.game_config.enable_codegen;
        self.codegen_output_path = project.game_config.codegen_output_path.clone();
        self.generate_entities = project.game_config.generate_entities;
        self.generate_stubs = project.game_config.generate_stubs;
        self.generate_behaviors = project.game_config.generate_behaviors;
        self.generate_enums = project.game_config.generate_enums;

        // Detect preferred editor
        self.preferred_editor = external_editor::detect_best_editor();
    }

    /// Check and cache CLI installation status
    pub fn check_cli_status(&mut self) {
        if self.cli_installed.is_none() {
            self.cli_installed = Some(bevy_cli::is_bevy_cli_installed());
        }
    }

    /// Extract the project name from the path (last component)
    pub fn get_project_name(&self) -> Option<String> {
        let path = PathBuf::from(&self.project_path_input);
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    }

    /// Get the parent directory of the project path
    pub fn get_parent_dir(&self) -> Option<PathBuf> {
        let path = PathBuf::from(&self.project_path_input);
        path.parent().map(|p| p.to_path_buf())
    }
}

/// Result of rendering the game settings dialog
#[derive(Default)]
pub struct GameSettingsDialogResult {
    /// User wants to save the settings
    pub save_requested: bool,
    /// User wants to create a new game project
    pub create_project_requested: bool,
    /// User wants to create a new level
    pub create_level_requested: bool,
    /// User wants to install Bevy CLI
    pub install_cli_requested: bool,
    /// User wants to generate code now
    pub generate_code_requested: bool,
    /// User wants to preview generated code
    pub preview_code_requested: bool,
    /// User wants to open game project in external editor
    pub open_in_editor_requested: bool,
}

/// Render the game settings dialog
pub fn render_game_settings_dialog(
    ctx: &egui::Context,
    state: &mut GameSettingsDialogState,
    project: &mut Project,
) -> GameSettingsDialogResult {
    let mut result = GameSettingsDialogResult::default();

    if !state.open {
        return result;
    }

    // Check CLI status on first open
    state.check_cli_status();

    // Modal overlay - blocks all input behind the dialog
    egui::Area::new(egui::Id::new("game_settings_modal_overlay"))
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
            // Consume all interactions
            response.context_menu(|_| {});
        });

    egui::Window::new("Game Project Settings")
        .collapsible(false)
        .resizable(true)
        .default_width(500.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.heading("Game Project Configuration");
            ui.separator();

            // CLI Status
            let cli_installed = state.cli_installed.unwrap_or(false);
            ui.horizontal(|ui| {
                ui.label("Bevy CLI:");
                if cli_installed {
                    ui.colored_label(egui::Color32::GREEN, "Installed");
                    if let Some(version) = bevy_cli::get_bevy_cli_version() {
                        ui.label(format!("({})", version));
                    }
                } else {
                    ui.colored_label(egui::Color32::RED, "Not installed");
                    if ui.button("Install").clicked() {
                        result.install_cli_requested = true;
                    }
                }
            });

            ui.add_space(8.0);

            // Project Path - single full path input
            ui.label("Game Project Path:");
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut state.project_path_input)
                        .desired_width(350.0)
                        .hint_text("C:\\Dev\\Games\\my_game"),
                );
                #[cfg(feature = "native")]
                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_directory(std::env::current_dir().unwrap_or_default())
                        .pick_folder()
                    {
                        state.project_path_input = path.to_string_lossy().to_string();
                    }
                }
            });

            // Show path status and derived project name
            let path = PathBuf::from(&state.project_path_input);
            let project_name = state.get_project_name();
            let project_exists = path.join("Cargo.toml").exists();

            if !state.project_path_input.is_empty() {
                if project_exists {
                    ui.colored_label(
                        egui::Color32::GREEN,
                        "Valid game project found - ready to run",
                    );
                } else if path.exists() {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        "Directory exists but no Cargo.toml - use Create to scaffold",
                    );
                } else if let Some(ref name) = project_name {
                    ui.colored_label(
                        egui::Color32::LIGHT_GRAY,
                        format!("Will create new project \"{}\"", name),
                    );
                } else {
                    ui.colored_label(egui::Color32::RED, "Invalid path");
                }
            }

            ui.add_space(8.0);

            // Starting Level dropdown
            ui.horizontal(|ui| {
                ui.label("Starting Level:");

                let current_name = state
                    .selected_starting_level
                    .and_then(|id| project.get_level(id))
                    .map(|l| l.name.clone())
                    .unwrap_or_else(|| "(Select a level)".to_string());

                egui::ComboBox::from_id_salt("starting_level_combo")
                    .selected_text(current_name)
                    .show_ui(ui, |ui| {
                        for level in &project.levels {
                            let is_selected = state.selected_starting_level == Some(level.id);
                            if ui.selectable_label(is_selected, &level.name).clicked() {
                                state.selected_starting_level = Some(level.id);
                            }
                        }
                    });

                result.create_level_requested = ui.button("+").clicked();
            });

            ui.add_space(8.0);

            // Build options
            ui.checkbox(
                &mut state.use_release_build,
                "Use release build (slower to compile, faster to run)",
            );

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(4.0);

            // Code Generation Section
            ui.heading("Code Generation");
            ui.add_space(4.0);

            ui.checkbox(&mut state.enable_codegen, "Auto-generate code on save");

            ui.add_enabled_ui(state.enable_codegen, |ui| {
                ui.indent("codegen_options", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Output path:");
                        ui.add(
                            egui::TextEdit::singleline(&mut state.codegen_output_path)
                                .desired_width(200.0)
                                .hint_text("src/generated"),
                        );
                    });

                    ui.add_space(4.0);
                    ui.label("Generate:");
                    ui.checkbox(&mut state.generate_entities, "Entity structs");
                    ui.checkbox(&mut state.generate_enums, "Enum definitions");
                    ui.checkbox(&mut state.generate_stubs, "Behavior stubs");
                    ui.checkbox(
                        &mut state.generate_behaviors,
                        "Movement systems (from Input profiles)",
                    );
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.button("Generate Now").clicked() {
                        result.generate_code_requested = true;
                    }
                    if ui.button("Preview Code...").clicked() {
                        result.preview_code_requested = true;
                    }
                });
            });

            ui.add_space(8.0);

            // External editor section
            ui.horizontal(|ui| {
                ui.label("External Editor:");
                egui::ComboBox::from_id_salt("preferred_editor")
                    .selected_text(state.preferred_editor.display_name())
                    .show_ui(ui, |ui| {
                        for editor in PreferredEditor::all() {
                            let label = if editor.is_available() {
                                editor.display_name().to_string()
                            } else {
                                format!("{} (not installed)", editor.display_name())
                            };
                            if ui
                                .selectable_label(state.preferred_editor == *editor, label)
                                .clicked()
                            {
                                state.preferred_editor = *editor;
                            }
                        }
                    });

                ui.add_enabled_ui(
                    project_exists && state.preferred_editor.is_available(),
                    |ui| {
                        if ui.button("Open in Editor").clicked() {
                            result.open_in_editor_requested = true;
                        }
                    },
                );
            });

            // Status message
            if let Some(msg) = &state.status_message {
                ui.separator();
                ui.label(msg);
            }

            ui.separator();

            // Action buttons
            ui.horizontal(|ui| {
                // Create Game Project button - enabled when CLI installed, path set, name valid, and doesn't exist
                let can_create = cli_installed && project_name.is_some() && !project_exists;

                ui.add_enabled_ui(can_create, |ui| {
                    if ui.button("Create Game Project").clicked() {
                        result.create_project_requested = true;
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        state.open = false;
                    }

                    // Can save if path is set and starting level selected
                    let can_save = !state.project_path_input.is_empty()
                        && state.selected_starting_level.is_some();

                    ui.add_enabled_ui(can_save, |ui| {
                        if ui.button("Save").clicked() {
                            // Update project config with full path
                            project.game_config.project_path =
                                Some(PathBuf::from(&state.project_path_input));
                            project.game_config.starting_level = state.selected_starting_level;
                            project.game_config.use_release_build = state.use_release_build;

                            // Save codegen settings
                            project.game_config.enable_codegen = state.enable_codegen;
                            project.game_config.codegen_output_path =
                                state.codegen_output_path.clone();
                            project.game_config.generate_entities = state.generate_entities;
                            project.game_config.generate_stubs = state.generate_stubs;
                            project.game_config.generate_behaviors = state.generate_behaviors;
                            project.game_config.generate_enums = state.generate_enums;

                            project.mark_dirty();

                            result.save_requested = true;
                            state.open = false;
                        }
                    });
                });
            });
        });

    result
}
