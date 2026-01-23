//! bevy_map_codegen - Code generation for bevy_map_editor game projects
//!
//! This crate provides automatic Rust code generation from schema definitions,
//! including:
//!
//! - **Project validation** - Validate game project structure
//! - **Entity structs** - Auto-generate `#[derive(MapEntity)]` structs from schema types
//! - **Behavior stubs** - Generate empty system function signatures per entity type
//! - **Behavior systems** - Pre-built systems for common 2D patterns (movement, combat, AI)
//!
//! # Example
//!
//! ```rust,ignore
//! use bevy_map_codegen::{generate_all, is_valid_project, ensure_generated_module, CodegenConfig};
//!
//! // Check if project is valid
//! if is_valid_project(&project_path) {
//!     // Ensure generated module exists
//!     ensure_generated_module(&project_path)?;
//!
//!     // Generate code from schema
//!     let codegen_config = CodegenConfig::new(project_path.join("src/generated"));
//!     generate_all(&schema, &entity_configs, &codegen_config)?;
//! }
//! ```

pub mod behaviors;
pub mod entities;
pub mod enums;
pub mod generator;
pub mod scaffold;
pub mod stubs;

pub use generator::{generate_all, CodegenConfig, CodegenResult};
pub use scaffold::{ensure_generated_module, has_generated_module, is_valid_project};

use thiserror::Error;

/// Errors that can occur during code generation
#[derive(Debug, Error)]
pub enum CodegenError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse generated code: {0}")]
    ParseError(String),

    #[error("Failed to format code: {0}")]
    FormatError(String),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("Template error: {0}")]
    TemplateError(String),
}

/// Convert a PascalCase name to snake_case
pub fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert a snake_case name to PascalCase
pub fn to_pascal_case(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

/// Format Rust code using prettyplease
pub fn format_code(code: &str) -> Result<String, CodegenError> {
    let syntax_tree = syn::parse_file(code)
        .map_err(|e| CodegenError::ParseError(format!("Failed to parse: {}", e)))?;

    Ok(prettyplease::unparse(&syntax_tree))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("Player"), "player");
        assert_eq!(to_snake_case("PlayerCharacter"), "player_character");
        assert_eq!(to_snake_case("NPCController"), "n_p_c_controller");
        assert_eq!(to_snake_case("MyHTTPHandler"), "my_h_t_t_p_handler");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("player"), "Player");
        assert_eq!(to_pascal_case("player_character"), "PlayerCharacter");
        assert_eq!(to_pascal_case("my_type"), "MyType");
    }
}
