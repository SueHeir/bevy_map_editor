//! Project file save/load operations

use super::Project;
use bevy_map_codegen::{generate_all, CodegenConfig};
use std::path::Path;

#[derive(Debug)]
pub enum ProjectError {
    IoError(String),
    ParseError(String),
    SerializeError(String),
    NoPath,
    CodegenError(String),
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectError::IoError(e) => write!(f, "IO error: {}", e),
            ProjectError::ParseError(e) => write!(f, "Parse error: {}", e),
            ProjectError::SerializeError(e) => write!(f, "Serialize error: {}", e),
            ProjectError::NoPath => write!(f, "No file path set"),
            ProjectError::CodegenError(e) => write!(f, "Code generation error: {}", e),
        }
    }
}

impl std::error::Error for ProjectError {}

impl Project {
    /// Load project from file
    pub fn load(path: &Path) -> Result<Self, ProjectError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ProjectError::IoError(e.to_string()))?;

        let mut project: Project =
            serde_json::from_str(&content).map_err(|e| ProjectError::ParseError(e.to_string()))?;

        project.path = Some(path.to_path_buf());

        // Build lookup indices for O(1) access to levels, tilesets, etc.
        project.rebuild_indices();

        // Validate and clean up any orphaned references (e.g., terrain sets pointing to deleted tilesets)
        project.validate_and_cleanup();

        // Only mark dirty if we haven't modified anything
        // (validate_and_cleanup sets dirty=true if it removes orphaned data)
        if !project.dirty {
            project.dirty = false;
        }

        Ok(project)
    }

    /// Save project to file
    pub fn save(&mut self, path: &Path) -> Result<(), ProjectError> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ProjectError::SerializeError(e.to_string()))?;

        std::fs::write(path, content).map_err(|e| ProjectError::IoError(e.to_string()))?;

        self.path = Some(path.to_path_buf());
        self.dirty = false;

        // Run code generation if enabled
        if self.game_config.enable_codegen {
            if let Some(game_path) = &self.game_config.project_path {
                let output_dir = game_path.join(&self.game_config.codegen_output_path);
                let config = CodegenConfig {
                    output_dir,
                    generate_entities: self.game_config.generate_entities,
                    generate_enums: self.game_config.generate_enums,
                    generate_stubs: self.game_config.generate_stubs,
                    generate_behaviors: self.game_config.generate_behaviors,
                    generate_health: false,
                    generate_patrol: false,
                };

                if let Err(e) = generate_all(&self.schema, &self.entity_type_configs, &config) {
                    bevy::log::warn!("Code generation failed: {}", e);
                    // Don't fail the save, just warn
                } else {
                    bevy::log::info!("Code generated to {:?}", config.output_dir);
                }
            }
        }

        Ok(())
    }

    /// Save to current path if set
    pub fn save_current(&mut self) -> Result<(), ProjectError> {
        if let Some(path) = self.path.clone() {
            self.save(&path)
        } else {
            Err(ProjectError::NoPath)
        }
    }
}
