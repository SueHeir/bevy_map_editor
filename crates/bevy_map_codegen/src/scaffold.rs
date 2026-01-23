//! Project validation and generated module utilities
//!
//! This module provides utilities for validating game project structures
//! and ensuring the generated code directory exists.

use crate::CodegenError;
use std::fs;
use std::path::Path;

/// Check if a path is a valid game project (has Cargo.toml and src/main.rs)
pub fn is_valid_project(path: &Path) -> bool {
    path.join("Cargo.toml").exists() && path.join("src/main.rs").exists()
}

/// Check if a path has the generated module
pub fn has_generated_module(path: &Path) -> bool {
    path.join("src/generated/mod.rs").exists()
}

/// Ensure the generated module exists, creating it if necessary
pub fn ensure_generated_module(path: &Path) -> Result<(), CodegenError> {
    let gen_dir = path.join("src/generated");
    if !gen_dir.exists() {
        fs::create_dir_all(&gen_dir)?;
    }

    let mod_file = gen_dir.join("mod.rs");
    if !mod_file.exists() {
        let placeholder = r#"//! Auto-generated code from bevy_map_editor
//!
//! This module will be populated when you save your map project
//! with code generation enabled.

use bevy::prelude::*;

pub struct GeneratedPlugin;

impl Plugin for GeneratedPlugin {
    fn build(&self, _app: &mut App) {
        // Code generation not yet run
    }
}
"#;
        fs::write(mod_file, placeholder)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_is_valid_project() {
        let temp = temp_dir().join("test_bevy_codegen_valid_project");
        let _ = fs::remove_dir_all(&temp);

        // Not valid - doesn't exist
        assert!(!is_valid_project(&temp));

        // Create directory structure
        fs::create_dir_all(temp.join("src")).unwrap();
        fs::write(temp.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        // Not valid - no src/main.rs
        assert!(!is_valid_project(&temp));

        fs::write(temp.join("src/main.rs"), "fn main() {}").unwrap();

        // Now valid
        assert!(is_valid_project(&temp));

        // Clean up
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_has_generated_module() {
        let temp = temp_dir().join("test_bevy_codegen_has_generated");
        let _ = fs::remove_dir_all(&temp);

        // Doesn't have generated module
        assert!(!has_generated_module(&temp));

        fs::create_dir_all(temp.join("src/generated")).unwrap();
        assert!(!has_generated_module(&temp));

        fs::write(temp.join("src/generated/mod.rs"), "// test").unwrap();
        assert!(has_generated_module(&temp));

        // Clean up
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_ensure_generated_module() {
        let temp = temp_dir().join("test_bevy_codegen_ensure_generated");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(temp.join("src")).unwrap();

        assert!(!has_generated_module(&temp));
        ensure_generated_module(&temp).unwrap();
        assert!(has_generated_module(&temp));

        // Calling again should be a no-op
        ensure_generated_module(&temp).unwrap();
        assert!(has_generated_module(&temp));

        // Clean up
        let _ = fs::remove_dir_all(&temp);
    }
}
