//! Game project settings dialog
//!
//! This dialog allows users to configure the associated game project,
//! including the project path, starting level, build options, and code generation.

use bevy_egui::egui;
use std::path::PathBuf;
use uuid::Uuid;

use crate::bevy_cli;
use crate::external_editor;
use crate::project::Project;

/// State for the game settings dialog
#[derive(Default)]
pub struct GameSettingsDialogState {
    /// Whether the dialog is open
    pub open: bool,
    /// Parent directory for the game project (e.g., C:\Dev\Games)
    pub parent_directory: String,
    /// Project name (e.g., my_game) - must be a valid Rust crate name
    pub project_name: String,
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
    /// Custom VS Code path (empty = auto-detect)
    pub vscode_path: String,
    /// Cached VS Code availability status (None = not checked yet)
    pub vscode_available: Option<bool>,
}

impl GameSettingsDialogState {
    /// Initialize dialog state from project config
    pub fn load_from_project(&mut self, project: &Project) {
        // Parse project_path into parent_directory and project_name
        if let Some(project_path) = &project.game_config.project_path {
            self.parent_directory = project_path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            self.project_name = project_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .unwrap_or_default();
        } else {
            self.parent_directory.clear();
            self.project_name.clear();
        }

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

        // Load VS Code path
        self.vscode_path = project.game_config.vscode_path.clone().unwrap_or_default();
    }

    /// Check and cache CLI installation status
    pub fn check_cli_status(&mut self) {
        if self.cli_installed.is_none() {
            self.cli_installed = Some(bevy_cli::is_bevy_cli_installed());
        }
    }

    /// Check and cache VS Code availability status
    pub fn check_vscode_status(&mut self) {
        if self.vscode_available.is_none() {
            self.vscode_available = Some(if self.vscode_path.is_empty() {
                external_editor::is_vscode_installed()
            } else {
                std::path::Path::new(&self.vscode_path).exists()
            });
        }
    }

    /// Invalidate VS Code cache (call when vscode_path changes)
    pub fn invalidate_vscode_cache(&mut self) {
        self.vscode_available = None;
    }

    /// Get the full project path (parent_directory / project_name)
    pub fn get_full_project_path(&self) -> Option<PathBuf> {
        if self.parent_directory.is_empty() || self.project_name.is_empty() {
            return None;
        }
        Some(PathBuf::from(&self.parent_directory).join(&self.project_name))
    }

    /// Get the project name (for compatibility)
    pub fn get_project_name(&self) -> Option<String> {
        if self.project_name.is_empty() {
            None
        } else {
            Some(self.project_name.clone())
        }
    }

    /// Get the parent directory as a PathBuf (for compatibility)
    pub fn get_parent_dir(&self) -> Option<PathBuf> {
        if self.parent_directory.is_empty() {
            None
        } else {
            Some(PathBuf::from(&self.parent_directory))
        }
    }
}

/// Check if a string is a valid Rust crate name
fn is_valid_crate_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_lowercase() && first_char != '_' {
        return false;
    }

    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
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
    /// User wants to open game project in VS Code
    pub open_in_vscode_requested: bool,
    /// User wants to open project folder in file browser
    pub open_folder_requested: bool,
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

    // Check CLI and VS Code status on first open
    state.check_cli_status();
    state.check_vscode_status();

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
        .default_width(550.0)
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

            // Parent Directory field
            ui.label("Parent Directory:");
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut state.parent_directory)
                        .desired_width(400.0)
                        .hint_text("C:\\Dev\\Games"),
                );
                #[cfg(feature = "native")]
                if ui.button("Browse...").clicked() {
                    let start_dir = if state.parent_directory.is_empty() {
                        std::env::current_dir().unwrap_or_default()
                    } else {
                        PathBuf::from(&state.parent_directory)
                    };
                    if let Some(path) = rfd::FileDialog::new()
                        .set_directory(start_dir)
                        .pick_folder()
                    {
                        state.parent_directory = path.to_string_lossy().to_string();
                    }
                }
            });

            ui.add_space(4.0);

            // Project Name field
            ui.horizontal(|ui| {
                ui.label("Project Name:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.project_name)
                        .desired_width(200.0)
                        .hint_text("my_game"),
                );

                // Validate name as user types
                let name_valid = is_valid_crate_name(&state.project_name);
                if !state.project_name.is_empty() && !name_valid {
                    ui.colored_label(egui::Color32::RED, "(invalid name)")
                        .on_hover_text("Must start with lowercase letter, contain only lowercase letters, digits, underscores, or hyphens");
                }
            });

            ui.add_space(4.0);

            // Show full path preview and status
            let full_path = state.get_full_project_path();
            let name_valid = is_valid_crate_name(&state.project_name);

            if let Some(ref path) = full_path {
                let project_exists = path.join("Cargo.toml").exists();
                let dir_exists = path.exists();

                ui.horizontal(|ui| {
                    ui.label("Will create:");
                    ui.monospace(path.to_string_lossy().to_string());
                });

                if project_exists {
                    ui.colored_label(
                        egui::Color32::GREEN,
                        "Valid game project found - ready to run",
                    );
                } else if dir_exists {
                    ui.colored_label(
                        egui::Color32::RED,
                        "Directory already exists! Choose a different project name.",
                    );
                } else if name_valid {
                    ui.colored_label(
                        egui::Color32::LIGHT_GRAY,
                        format!("Will create new project \"{}\"", state.project_name),
                    );
                }
            } else if !state.parent_directory.is_empty() || !state.project_name.is_empty() {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    "Enter both parent directory and project name",
                );
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

            // Check if game project exists (has Cargo.toml)
            let project_exists = full_path
                .as_ref()
                .map(|p| p.join("Cargo.toml").exists())
                .unwrap_or(false);

            // Code Generation Section - only show when project exists
            if project_exists {
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(4.0);

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
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // VS Code Settings section
            ui.heading("External Editor");
            ui.add_space(4.0);

            // VS Code path configuration
            let old_vscode_path = state.vscode_path.clone();
            ui.horizontal(|ui| {
                ui.label("VS Code Path:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.vscode_path)
                        .desired_width(300.0)
                        .hint_text("Leave empty for auto-detection"),
                );
                #[cfg(feature = "native")]
                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Executable", &["exe"])
                        .pick_file()
                    {
                        state.vscode_path = path.to_string_lossy().to_string();
                    }
                }
            });

            // Invalidate cache if path changed
            if state.vscode_path != old_vscode_path {
                state.invalidate_vscode_cache();
                state.check_vscode_status();
            }

            // Use cached VS Code detection status
            let vscode_available = state.vscode_available.unwrap_or(false);

            ui.horizontal(|ui| {
                if vscode_available {
                    ui.colored_label(egui::Color32::GREEN, "VS Code detected");
                    if state.vscode_path.is_empty() {
                        if let Some(default_path) = external_editor::get_default_vscode_path() {
                            ui.label(format!("({})", default_path));
                        }
                    }
                } else if !state.vscode_path.is_empty() {
                    ui.colored_label(egui::Color32::RED, "Path not found");
                } else {
                    ui.colored_label(egui::Color32::YELLOW, "VS Code not detected - specify path above");
                }
            });

            ui.add_space(8.0);

            // Open buttons
            ui.horizontal(|ui| {
                ui.add_enabled_ui(project_exists && vscode_available, |ui| {
                    if ui.button("Open Project in VS Code").clicked() {
                        result.open_in_vscode_requested = true;
                    }
                });

                ui.add_enabled_ui(project_exists, |ui| {
                    if ui.button("Open Folder").clicked() {
                        result.open_folder_requested = true;
                    }
                });
            });

            // Status message
            if let Some(msg) = &state.status_message {
                ui.separator();
                ui.label(msg);
            }

            ui.separator();

            // Action buttons
            ui.horizontal(|ui| {
                // Create Game Project button - enabled when:
                // - CLI installed
                // - project_name is valid crate name
                // - full path doesn't exist (neither directory nor project)
                let dir_exists = full_path.as_ref().map(|p| p.exists()).unwrap_or(true);
                let can_create = cli_installed && name_valid && !dir_exists && full_path.is_some();

                ui.add_enabled_ui(can_create, |ui| {
                    if ui.button("Create Game Project").clicked() {
                        result.create_project_requested = true;
                    }
                });

                if !cli_installed && full_path.is_some() && name_valid && !dir_exists {
                    ui.colored_label(egui::Color32::YELLOW, "Install Bevy CLI to create projects");
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        state.open = false;
                    }

                    // Can save if path is set and starting level selected
                    let can_save =
                        full_path.is_some() && state.selected_starting_level.is_some();

                    ui.add_enabled_ui(can_save, |ui| {
                        if ui.button("Save").clicked() {
                            // Update project config with full path
                            project.game_config.project_path = full_path.clone();
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

                            // Save VS Code path
                            project.game_config.vscode_path = if state.vscode_path.is_empty() {
                                None
                            } else {
                                Some(state.vscode_path.clone())
                            };

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
