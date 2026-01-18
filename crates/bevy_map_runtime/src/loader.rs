//! Asset loader for MapProject files
//!
//! This module provides a Bevy AssetLoader implementation for loading `.map.json` files.
//! When combined with Bevy's `file_watcher` feature, this enables hot-reloading of maps
//! during development.
//!
//! # Hot-Reload Workflow
//!
//! 1. Run your game with file watching: `cargo run --features bevy/file_watcher`
//! 2. Open the map in bevy_map_editor
//! 3. Edit and save - the game automatically reloads the map
//!
//! # Example
//!
//! ```rust,ignore
//! use bevy::prelude::*;
//! use bevy_map_runtime::{MapRuntimePlugin, MapHandle, SpawnMapCommand};
//!
//! fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     // Load map as a Bevy asset
//!     let map_handle = asset_server.load("maps/level1.map.json");
//!
//!     // Spawn the map - it will render once loaded
//!     commands.spawn(MapHandle(map_handle));
//! }
//! ```

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::reflect::TypePath;
use bevy_map_core::{EditorProject, MapProject};
use thiserror::Error;

/// Error type for map loading failures
#[derive(Debug, Error)]
pub enum MapLoadError {
    #[error("Failed to read file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid map format: {0}")]
    InvalidFormat(String),
}

/// Asset loader for MapProject JSON files
///
/// Supports `.map.json` file extension. The loader parses the JSON and returns
/// a `MapProject` asset that can be used with `SpawnMapCommand` or the
/// `handle_map_spawning` system.
#[derive(Default, TypePath)]
pub struct MapProjectLoader;

impl AssetLoader for MapProjectLoader {
    type Asset = MapProject;
    type Settings = ();
    type Error = MapLoadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        // Read the entire file
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        // Try EditorProject format first (what the editor exports)
        // EditorProject uses Vec collections (levels, tilesets arrays)
        if let Ok(editor_project) = serde_json::from_slice::<EditorProject>(&bytes) {
            return editor_project
                .to_map_project()
                .ok_or_else(|| MapLoadError::InvalidFormat("No levels in project".to_string()));
        }

        // Fall back to MapProject format (hand-crafted JSON with HashMap collections)
        let project: MapProject = serde_json::from_slice(&bytes)?;

        // Validate the project
        project.validate().map_err(MapLoadError::InvalidFormat)?;

        Ok(project)
    }

    fn extensions(&self) -> &[&str] {
        &["map.json"]
    }
}

/// Load a level from a JSON string (for backward compatibility)
pub fn load_level_from_str(json: &str) -> Result<bevy_map_core::Level, serde_json::Error> {
    serde_json::from_str(json)
}

/// Load a level from a reader (for backward compatibility)
pub fn load_level_from_reader<R: std::io::Read>(
    reader: R,
) -> Result<bevy_map_core::Level, serde_json::Error> {
    serde_json::from_reader(reader)
}

/// Load a level from bytes (for backward compatibility)
pub fn load_level_from_bytes(bytes: &[u8]) -> Result<bevy_map_core::Level, serde_json::Error> {
    serde_json::from_slice(bytes)
}

/// Load a MapProject from a JSON string
pub fn load_project_from_str(json: &str) -> Result<MapProject, serde_json::Error> {
    serde_json::from_str(json)
}

/// Load a MapProject from bytes
pub fn load_project_from_bytes(bytes: &[u8]) -> Result<MapProject, serde_json::Error> {
    serde_json::from_slice(bytes)
}
