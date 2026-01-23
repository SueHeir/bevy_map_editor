//! Main code generation orchestration
//!
//! Coordinates all code generation modules to produce a complete
//! generated code directory for a game project.

use bevy_map_core::EntityTypeConfig;
use bevy_map_schema::Schema;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::behaviors::{generate_behaviors, generate_health_module, generate_patrol_module};
use crate::entities::generate_entities;
use crate::enums::generate_enums;
use crate::stubs::generate_stubs;
use crate::CodegenError;

/// Configuration for code generation
#[derive(Debug, Clone)]
pub struct CodegenConfig {
    /// Output directory for generated code (e.g., "src/generated")
    pub output_dir: PathBuf,

    /// Whether to generate entity structs
    pub generate_entities: bool,

    /// Whether to generate enum definitions
    pub generate_enums: bool,

    /// Whether to generate stub systems
    pub generate_stubs: bool,

    /// Whether to generate behavior systems
    pub generate_behaviors: bool,

    /// Whether to generate health system module
    pub generate_health: bool,

    /// Whether to generate patrol AI module
    pub generate_patrol: bool,
}

impl CodegenConfig {
    /// Create a new config with default settings (all enabled)
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self {
            output_dir: output_dir.into(),
            generate_entities: true,
            generate_enums: true,
            generate_stubs: true,
            generate_behaviors: true,
            generate_health: false, // Optional module
            generate_patrol: false, // Optional module
        }
    }

    /// Disable entity generation
    pub fn without_entities(mut self) -> Self {
        self.generate_entities = false;
        self
    }

    /// Disable enum generation
    pub fn without_enums(mut self) -> Self {
        self.generate_enums = false;
        self
    }

    /// Disable stub generation
    pub fn without_stubs(mut self) -> Self {
        self.generate_stubs = false;
        self
    }

    /// Disable behavior generation
    pub fn without_behaviors(mut self) -> Self {
        self.generate_behaviors = false;
        self
    }

    /// Enable health system generation
    pub fn with_health(mut self) -> Self {
        self.generate_health = true;
        self
    }

    /// Enable patrol AI generation
    pub fn with_patrol(mut self) -> Self {
        self.generate_patrol = true;
        self
    }
}

impl Default for CodegenConfig {
    fn default() -> Self {
        Self::new("src/generated")
    }
}

/// Result of code generation
#[derive(Debug, Clone)]
pub struct CodegenResult {
    /// Files that were generated
    pub generated_files: Vec<PathBuf>,

    /// Any warnings that occurred
    pub warnings: Vec<String>,
}

impl CodegenResult {
    fn new() -> Self {
        Self {
            generated_files: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn add_file(&mut self, path: PathBuf) {
        self.generated_files.push(path);
    }

    #[allow(dead_code)]
    fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }
}

/// Generate all code from schema and entity configurations
///
/// This is the main entry point for code generation. It creates:
/// - `entities.rs` - Entity structs with MapEntity derives
/// - `enums.rs` - Enum definitions with FromStr
/// - `stubs.rs` - Stub systems for each placeable type
/// - `behaviors.rs` - Movement and AI systems based on input profiles
/// - `mod.rs` - Module exports and plugin registration
pub fn generate_all(
    schema: &Schema,
    entity_configs: &HashMap<String, EntityTypeConfig>,
    config: &CodegenConfig,
) -> Result<CodegenResult, CodegenError> {
    let mut result = CodegenResult::new();

    // Ensure output directory exists
    fs::create_dir_all(&config.output_dir)?;

    // Generate entities
    if config.generate_entities {
        let entities_code = generate_entities(schema)?;
        let path = config.output_dir.join("entities.rs");
        fs::write(&path, &entities_code)?;
        result.add_file(path);
    }

    // Generate enums
    if config.generate_enums && !schema.enums.is_empty() {
        let enums_code = generate_enums(schema)?;
        let path = config.output_dir.join("enums.rs");
        fs::write(&path, &enums_code)?;
        result.add_file(path);
    }

    // Generate stubs
    if config.generate_stubs {
        let stubs_code = generate_stubs(schema)?;
        let path = config.output_dir.join("stubs.rs");
        fs::write(&path, &stubs_code)?;
        result.add_file(path);
    }

    // Generate behaviors
    if config.generate_behaviors {
        let behaviors_code = generate_behaviors(schema, entity_configs)?;
        let path = config.output_dir.join("behaviors.rs");
        fs::write(&path, &behaviors_code)?;
        result.add_file(path);
    }

    // Generate optional components
    let components_dir = config.output_dir.join("components");

    if config.generate_health {
        fs::create_dir_all(&components_dir)?;
        let health_code = generate_health_module()?;
        let path = components_dir.join("health.rs");
        fs::write(&path, &health_code)?;
        result.add_file(path);
    }

    if config.generate_patrol {
        fs::create_dir_all(&components_dir)?;
        let patrol_code = generate_patrol_module()?;
        let path = components_dir.join("patrol.rs");
        fs::write(&path, &patrol_code)?;
        result.add_file(path);
    }

    // Generate components mod.rs if any components were generated
    if config.generate_health || config.generate_patrol {
        let components_mod = generate_components_mod(config);
        let path = components_dir.join("mod.rs");
        fs::write(&path, &components_mod)?;
        result.add_file(path);
    }

    // Generate main mod.rs
    let mod_code = generate_mod_rs(schema, config);
    let path = config.output_dir.join("mod.rs");
    fs::write(&path, &mod_code)?;
    result.add_file(path);

    Ok(result)
}

/// Generate the main mod.rs file
fn generate_mod_rs(schema: &Schema, config: &CodegenConfig) -> String {
    let mut lines = vec![
        "//! Auto-generated code from bevy_map_editor".to_string(),
        "//!".to_string(),
        "//! This module is regenerated when you save your map project with code generation enabled."
            .to_string(),
        "//! Do not edit manually - your changes will be overwritten!".to_string(),
        "".to_string(),
        "use bevy::prelude::*;".to_string(),
        "".to_string(),
    ];

    // Module declarations
    if config.generate_entities {
        lines.push("mod entities;".to_string());
    }
    if config.generate_enums && !schema.enums.is_empty() {
        lines.push("mod enums;".to_string());
    }
    if config.generate_stubs {
        lines.push("mod stubs;".to_string());
    }
    if config.generate_behaviors {
        lines.push("mod behaviors;".to_string());
    }
    if config.generate_health || config.generate_patrol {
        lines.push("pub mod components;".to_string());
    }

    lines.push("".to_string());

    // Re-exports
    if config.generate_entities {
        lines.push("pub use entities::*;".to_string());
    }
    if config.generate_enums && !schema.enums.is_empty() {
        lines.push("pub use enums::*;".to_string());
    }
    if config.generate_stubs {
        lines.push("pub use stubs::StubsPlugin;".to_string());
    }
    if config.generate_behaviors {
        lines.push("pub use behaviors::BehaviorsPlugin;".to_string());
    }

    lines.push("".to_string());

    // Plugin
    lines.push("/// Plugin that registers all generated systems and components".to_string());
    lines.push("pub struct GeneratedPlugin;".to_string());
    lines.push("".to_string());
    lines.push("impl Plugin for GeneratedPlugin {".to_string());
    lines.push("    fn build(&self, app: &mut App) {".to_string());

    // Register components
    if config.generate_entities {
        // Register each placeable type as a component
        for (name, type_def) in &schema.data_types {
            if type_def.placeable {
                lines.push(format!("        app.register_type::<{}>();", name));
            }
        }
    }

    lines.push("".to_string());
    lines.push("        // Add plugins".to_string());

    if config.generate_stubs {
        lines.push("        app.add_plugins(StubsPlugin);".to_string());
    }
    if config.generate_behaviors {
        lines.push("        app.add_plugins(BehaviorsPlugin);".to_string());
    }
    if config.generate_health {
        lines.push("        app.add_plugins(components::health::HealthPlugin);".to_string());
    }
    if config.generate_patrol {
        lines.push("        app.add_plugins(components::patrol::PatrolPlugin);".to_string());
    }

    lines.push("    }".to_string());
    lines.push("}".to_string());

    lines.join("\n")
}

/// Generate the components/mod.rs file
fn generate_components_mod(config: &CodegenConfig) -> String {
    let mut lines = Vec::new();

    lines.push("//! Generated component modules".to_string());
    lines.push("".to_string());

    if config.generate_health {
        lines.push("pub mod health;".to_string());
    }
    if config.generate_patrol {
        lines.push("pub mod patrol;".to_string());
    }

    lines.join("\n")
}

/// Preview generated code without writing to disk
pub fn preview_entities(schema: &Schema) -> Result<String, CodegenError> {
    generate_entities(schema)
}

/// Preview generated enums without writing to disk
pub fn preview_enums(schema: &Schema) -> Result<String, CodegenError> {
    generate_enums(schema)
}

/// Preview generated stubs without writing to disk
pub fn preview_stubs(schema: &Schema) -> Result<String, CodegenError> {
    generate_stubs(schema)
}

/// Preview generated behaviors without writing to disk
pub fn preview_behaviors(
    schema: &Schema,
    entity_configs: &HashMap<String, EntityTypeConfig>,
) -> Result<String, CodegenError> {
    generate_behaviors(schema, entity_configs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_map_core::InputConfig;
    use bevy_map_schema::TypeDef;
    use std::env::temp_dir;

    fn make_test_schema_and_configs() -> (Schema, HashMap<String, EntityTypeConfig>) {
        let mut schema = Schema::default();

        schema.enums.insert(
            "ItemType".to_string(),
            vec!["Weapon".to_string(), "Armor".to_string()],
        );

        let mut player = TypeDef::default();
        player.placeable = true;
        schema.data_types.insert("Player".to_string(), player);

        let mut enemy = TypeDef::default();
        enemy.placeable = true;
        schema.data_types.insert("Enemy".to_string(), enemy);

        let mut configs = HashMap::new();
        configs.insert(
            "Player".to_string(),
            EntityTypeConfig {
                input: Some(InputConfig::platformer()),
                ..Default::default()
            },
        );

        (schema, configs)
    }

    #[test]
    fn test_generate_all() {
        let temp = temp_dir().join("test_bevy_map_codegen_generator");
        let _ = fs::remove_dir_all(&temp);

        let (schema, configs) = make_test_schema_and_configs();
        let config = CodegenConfig::new(&temp);

        let result = generate_all(&schema, &configs, &config);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(!result.generated_files.is_empty());
        assert!(temp.join("mod.rs").exists());
        assert!(temp.join("entities.rs").exists());
        assert!(temp.join("enums.rs").exists());
        assert!(temp.join("stubs.rs").exists());
        assert!(temp.join("behaviors.rs").exists());

        // Clean up
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_codegen_config() {
        let config = CodegenConfig::new("src/gen")
            .without_stubs()
            .with_health()
            .with_patrol();

        assert_eq!(config.output_dir, PathBuf::from("src/gen"));
        assert!(!config.generate_stubs);
        assert!(config.generate_health);
        assert!(config.generate_patrol);
    }
}
