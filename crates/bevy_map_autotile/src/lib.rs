//! Tiled-compatible terrain autotile system
//!
//! This crate provides the WangFiller algorithm for automatic tile selection
//! based on terrain definitions, compatible with Tiled's terrain system.
//!
//! # Features
//! - Corner, Edge, and Mixed terrain set types
//! - Tiled-compatible Wang tile matching
//! - Runtime terrain modification support
//! - Legacy 47-tile blob format support
//!
//! # Example
//!
//! ```rust,ignore
//! use bevy_map_autotile::{
//!     terrain::{TerrainSet, TerrainSetType, Color},
//!     wang::{WangFiller, paint_terrain},
//! };
//! use uuid::Uuid;
//!
//! // Create a terrain set for a tileset
//! let mut terrain_set = TerrainSet::new(
//!     "Ground".to_string(),
//!     Uuid::new_v4(), // tileset ID
//!     TerrainSetType::Corner,
//! );
//!
//! // Add terrain types
//! terrain_set.add_terrain("Grass".to_string(), Color::GREEN);
//! terrain_set.add_terrain("Dirt".to_string(), Color::rgb(0.6, 0.4, 0.2));
//!
//! // Define tile terrain data for each tile in the tileset
//! terrain_set.set_tile_terrain(0, 0, Some(0)); // Tile 0, corner 0 = Grass
//! // ... more tile definitions
//!
//! // Paint terrain onto a tile map
//! let mut tiles = vec![None; 100]; // 10x10 map
//! paint_terrain(&mut tiles, 10, 10, 5, 5, &terrain_set, 0);
//! ```

pub mod config;
pub mod legacy;
pub mod terrain;
pub mod wang;

// Re-export main types at crate root
pub use config::{AutotileConfig, LegacyTerrainType, TerrainBrush, TerrainType};
pub use terrain::{Color, Terrain, TerrainSet, TerrainSetType, TileConstraints, TileTerrainData};
pub use wang::{
    get_paint_target, paint_terrain, paint_terrain_at_target, paint_terrain_at_target_with_debug,
    paint_terrain_horizontal_edge, paint_terrain_vertical_edge, paint_terrain_with_debug,
    preview_terrain_at_target, preview_terrain_at_targets, update_tile_with_neighbors, CellInfo,
    PaintTarget, TerrainId, WangFiller, WangId, WangPosition,
};

// Re-export legacy module contents for backward compatibility
pub use legacy::{
    apply_autotile_to_region, calculate_bitmask, erase_autotile, neighbors, optimize_bitmask,
    paint_autotile,
};

// Re-export bevy_map_core
pub use bevy_map_core;
