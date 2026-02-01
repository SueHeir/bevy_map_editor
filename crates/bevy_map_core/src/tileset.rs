//! Tileset configuration with multi-image support

use crate::collision::{CollisionData, CollisionShape, OneWayDirection};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Per-tile properties like collision, animation, and custom metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TileProperties {
    /// Collision configuration for this tile
    #[serde(default, deserialize_with = "deserialize_collision")]
    pub collision: CollisionData,
    /// Animation frames for this tile (list of tile indices)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_frames: Option<Vec<u32>>,
    /// Animation speed in frames per second
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_speed: Option<f32>,
    /// Custom user-defined properties
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, serde_json::Value>,
    /// Width in grid cells (for multi-cell tiles like trees)
    #[serde(default = "default_one", skip_serializing_if = "is_one")]
    pub grid_width: u32,
    /// Height in grid cells (for multi-cell tiles like trees)
    #[serde(default = "default_one", skip_serializing_if = "is_one")]
    pub grid_height: u32,
    /// Origin X offset in pixels from top-left. None = auto-center.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_x: Option<u32>,
    /// Origin Y offset in pixels from top-left. None = auto-center.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_y: Option<u32>,
}

/// Default value of 1 for grid dimensions
fn default_one() -> u32 {
    1
}

/// Check if value equals 1 (for skipping serialization)
fn is_one(val: &u32) -> bool {
    *val == 1
}

/// Deserialize collision data with backward compatibility for old bool format
fn deserialize_collision<'de, D>(deserializer: D) -> Result<CollisionData, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum CollisionOrBool {
        Data(CollisionData),
        Bool(bool),
    }

    match CollisionOrBool::deserialize(deserializer)? {
        CollisionOrBool::Data(data) => Ok(data),
        CollisionOrBool::Bool(has_collision) => {
            if has_collision {
                Ok(CollisionData::full())
            } else {
                Ok(CollisionData::none())
            }
        }
    }
}

impl TileProperties {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if this tile has an animation
    pub fn has_animation(&self) -> bool {
        self.animation_frames
            .as_ref()
            .map(|f| f.len() > 1)
            .unwrap_or(false)
    }

    /// Check if this tile has collision
    pub fn has_collision(&self) -> bool {
        self.collision.has_collision()
    }

    /// Check if this is a one-way platform
    pub fn is_one_way(&self) -> bool {
        self.collision.is_one_way()
    }

    /// Set collision shape for this tile
    pub fn with_collision(mut self, shape: CollisionShape) -> Self {
        self.collision.shape = shape;
        self
    }

    /// Set full collision for this tile
    pub fn with_full_collision(mut self) -> Self {
        self.collision = CollisionData::full();
        self
    }

    /// Set collision data for this tile
    pub fn with_collision_data(mut self, collision: CollisionData) -> Self {
        self.collision = collision;
        self
    }

    /// Set one-way platform direction for this tile
    pub fn with_one_way(mut self, direction: OneWayDirection) -> Self {
        self.collision.one_way = direction;
        self
    }

    /// Set animation for this tile
    pub fn with_animation(mut self, frames: Vec<u32>, speed: f32) -> Self {
        self.animation_frames = Some(frames);
        self.animation_speed = Some(speed);
        self
    }

    /// Set a custom property
    pub fn with_custom(mut self, key: String, value: serde_json::Value) -> Self {
        self.custom.insert(key, value);
        self
    }

    /// Get a custom property
    pub fn get_custom(&self, key: &str) -> Option<&serde_json::Value> {
        self.custom.get(key)
    }

    /// Check if any properties are set (non-default)
    pub fn is_empty(&self) -> bool {
        self.collision.is_empty()
            && self.animation_frames.is_none()
            && self.custom.is_empty()
            && self.grid_width == 1
            && self.grid_height == 1
            && self.origin_x.is_none()
            && self.origin_y.is_none()
    }

    /// Check if this is a multi-cell tile (spans more than 1x1 grid cells)
    pub fn is_multi_cell(&self) -> bool {
        self.grid_width > 1 || self.grid_height > 1
    }

    /// Get the grid size (width, height) in cells
    pub fn grid_size(&self) -> (u32, u32) {
        (self.grid_width, self.grid_height)
    }

    /// Set the grid size for multi-cell tiles
    pub fn with_grid_size(mut self, width: u32, height: u32) -> Self {
        self.grid_width = width.max(1);
        self.grid_height = height.max(1);
        self
    }

    /// Get origin in pixels from top-left of tile.
    /// If not set, returns center of the tile.
    pub fn get_origin(&self, tile_width: u32, tile_height: u32) -> (u32, u32) {
        let x = self.origin_x.unwrap_or(tile_width / 2);
        let y = self.origin_y.unwrap_or(tile_height / 2);
        (x, y)
    }

    /// Set the origin point for the tile
    pub fn with_origin(mut self, x: Option<u32>, y: Option<u32>) -> Self {
        self.origin_x = x;
        self.origin_y = y;
        self
    }
}

/// A single image source within a tileset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilesetImage {
    pub id: Uuid,
    pub name: String,
    /// Path to the image file (relative to assets directory)
    pub path: String,
    pub columns: u32,
    pub rows: u32,
}

impl TilesetImage {
    /// Create a new tileset image
    pub fn new(name: String, path: String, columns: u32, rows: u32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            path,
            columns,
            rows,
        }
    }

    /// Total number of tiles in this image
    pub fn tile_count(&self) -> u32 {
        self.columns * self.rows
    }
}

/// Tileset configuration - can contain multiple images (Godot-style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tileset {
    pub id: Uuid,
    pub name: String,
    /// Tile size in pixels (assumes square tiles)
    pub tile_size: u32,
    /// Multiple image sources
    #[serde(default)]
    pub images: Vec<TilesetImage>,
    /// Per-tile properties (collision, animation, custom data)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tile_properties: HashMap<u32, TileProperties>,
    /// Legacy single-image path (for backward compatibility)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Legacy columns (for backward compatibility)
    #[serde(default)]
    pub columns: u32,
    /// Legacy rows (for backward compatibility)
    #[serde(default)]
    pub rows: u32,
}

impl Tileset {
    /// Create a new tileset with a single image
    pub fn new(name: String, path: String, tile_size: u32, columns: u32, rows: u32) -> Self {
        let image = TilesetImage::new("Main".to_string(), path.clone(), columns, rows);
        Self {
            id: Uuid::new_v4(),
            name,
            tile_size,
            images: vec![image],
            tile_properties: HashMap::new(),
            path: Some(path),
            columns,
            rows,
        }
    }

    /// Create a new empty tileset without an image
    pub fn new_empty(name: String, tile_size: u32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            tile_size,
            images: Vec::new(),
            tile_properties: HashMap::new(),
            path: None,
            columns: 0,
            rows: 0,
        }
    }

    /// Get properties for a tile (returns default if not set)
    pub fn get_tile_properties(&self, tile_index: u32) -> Option<&TileProperties> {
        self.tile_properties.get(&tile_index)
    }

    /// Get mutable properties for a tile, creating default if not exists
    pub fn get_tile_properties_mut(&mut self, tile_index: u32) -> &mut TileProperties {
        self.tile_properties.entry(tile_index).or_default()
    }

    /// Set properties for a tile
    pub fn set_tile_properties(&mut self, tile_index: u32, properties: TileProperties) {
        if properties.is_empty() {
            self.tile_properties.remove(&tile_index);
        } else {
            self.tile_properties.insert(tile_index, properties);
        }
    }

    /// Check if a tile has collision
    pub fn tile_has_collision(&self, tile_index: u32) -> bool {
        self.tile_properties
            .get(&tile_index)
            .map(|p| p.has_collision())
            .unwrap_or(false)
    }

    /// Get collision data for a tile
    pub fn get_tile_collision(&self, tile_index: u32) -> Option<&CollisionData> {
        self.tile_properties.get(&tile_index).map(|p| &p.collision)
    }

    /// Set collision data for a tile
    pub fn set_tile_collision(&mut self, tile_index: u32, collision: CollisionData) {
        let props = self.get_tile_properties_mut(tile_index);
        props.collision = collision;
        // Clean up if properties are now empty
        if props.is_empty() {
            self.tile_properties.remove(&tile_index);
        }
    }

    /// Set full collision for a tile (convenience method)
    pub fn set_tile_full_collision(&mut self, tile_index: u32, has_collision: bool) {
        let collision = if has_collision {
            CollisionData::full()
        } else {
            CollisionData::none()
        };
        self.set_tile_collision(tile_index, collision);
    }

    /// Set collision shape for a tile (preserving other collision properties)
    pub fn set_tile_collision_shape(&mut self, tile_index: u32, shape: CollisionShape) {
        let props = self.get_tile_properties_mut(tile_index);
        props.collision.shape = shape;
        // Clean up if properties are now empty
        if props.is_empty() {
            self.tile_properties.remove(&tile_index);
        }
    }

    /// Set one-way direction for a tile collision
    pub fn set_tile_one_way(&mut self, tile_index: u32, direction: OneWayDirection) {
        let props = self.get_tile_properties_mut(tile_index);
        props.collision.one_way = direction;
        // Clean up if properties are now empty
        if props.is_empty() {
            self.tile_properties.remove(&tile_index);
        }
    }

    /// Set collision layer for a tile
    pub fn set_tile_collision_layer(&mut self, tile_index: u32, layer: u8) {
        let props = self.get_tile_properties_mut(tile_index);
        props.collision.layer = layer;
        // Clean up if properties are now empty
        if props.is_empty() {
            self.tile_properties.remove(&tile_index);
        }
    }

    /// Set collision mask for a tile
    pub fn set_tile_collision_mask(&mut self, tile_index: u32, mask: u32) {
        let props = self.get_tile_properties_mut(tile_index);
        props.collision.mask = mask;
        // Clean up if properties are now empty
        if props.is_empty() {
            self.tile_properties.remove(&tile_index);
        }
    }

    /// Migrate legacy single-image format to multi-image format
    pub fn migrate_to_multi_image(&mut self) {
        if self.images.is_empty() {
            if let Some(path) = &self.path {
                let image =
                    TilesetImage::new("Main".to_string(), path.clone(), self.columns, self.rows);
                self.images.push(image);
            }
        }
    }

    /// Add a new image to this tileset
    pub fn add_image(&mut self, name: String, path: String, columns: u32, rows: u32) -> Uuid {
        let image = TilesetImage::new(name, path, columns, rows);
        let id = image.id;
        self.images.push(image);
        id
    }

    /// Remove an image by ID
    pub fn remove_image(&mut self, id: Uuid) -> bool {
        if let Some(pos) = self.images.iter().position(|img| img.id == id) {
            self.images.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get total tile count across all images
    pub fn total_tile_count(&self) -> u32 {
        if self.images.is_empty() {
            // Legacy mode
            self.columns * self.rows
        } else {
            self.images.iter().map(|img| img.tile_count()).sum()
        }
    }

    /// Convert virtual tile index to (image_index, local_tile_index)
    /// Returns None if the index is out of bounds
    pub fn virtual_to_local(&self, virtual_index: u32) -> Option<(usize, u32)> {
        if self.images.is_empty() {
            // Legacy mode - single image
            if virtual_index < self.columns * self.rows {
                return Some((0, virtual_index));
            }
            return None;
        }

        let mut offset = 0u32;
        for (img_idx, image) in self.images.iter().enumerate() {
            let tile_count = image.tile_count();
            if virtual_index < offset + tile_count {
                return Some((img_idx, virtual_index - offset));
            }
            offset += tile_count;
        }
        None
    }

    /// Convert (image_index, local_tile_index) to virtual tile index
    pub fn local_to_virtual(&self, image_index: usize, local_index: u32) -> Option<u32> {
        if self.images.is_empty() {
            // Legacy mode
            if image_index == 0 && local_index < self.columns * self.rows {
                return Some(local_index);
            }
            return None;
        }

        if image_index >= self.images.len() {
            return None;
        }

        let image = &self.images[image_index];
        if local_index >= image.tile_count() {
            return None;
        }

        let offset: u32 = self.images[..image_index]
            .iter()
            .map(|img| img.tile_count())
            .sum();
        Some(offset + local_index)
    }

    /// Get image info for a virtual tile index
    pub fn get_tile_image_info(&self, virtual_index: u32) -> Option<(&TilesetImage, u32)> {
        let (img_idx, local_idx) = self.virtual_to_local(virtual_index)?;
        if self.images.is_empty() {
            None
        } else {
            Some((&self.images[img_idx], local_idx))
        }
    }

    /// Get the first image path (for legacy compatibility)
    pub fn primary_path(&self) -> Option<&str> {
        if !self.images.is_empty() {
            Some(&self.images[0].path)
        } else {
            self.path.as_deref()
        }
    }

    /// Get image at index
    pub fn get_image(&self, index: usize) -> Option<&TilesetImage> {
        self.images.get(index)
    }

    /// Get mutable image at index
    pub fn get_image_mut(&mut self, index: usize) -> Option<&mut TilesetImage> {
        self.images.get_mut(index)
    }

    /// Convert local tile index to (column, row) within an image
    pub fn local_to_grid(&self, image_index: usize, local_index: u32) -> Option<(u32, u32)> {
        let image = self.images.get(image_index)?;
        if local_index >= image.tile_count() {
            return None;
        }
        let col = local_index % image.columns;
        let row = local_index / image.columns;
        Some((col, row))
    }

    /// Get the grid size for a tile (width, height in cells)
    /// Returns (1, 1) for tiles without multi-cell properties
    pub fn get_tile_grid_size(&self, tile_index: u32) -> (u32, u32) {
        self.tile_properties
            .get(&tile_index)
            .map(|p| p.grid_size())
            .unwrap_or((1, 1))
    }

    /// Check if a tile is multi-cell (spans more than 1x1 grid cells)
    pub fn is_multi_cell_tile(&self, tile_index: u32) -> bool {
        self.tile_properties
            .get(&tile_index)
            .map(|p| p.is_multi_cell())
            .unwrap_or(false)
    }

    /// Set the grid size for a tile
    pub fn set_tile_grid_size(&mut self, tile_index: u32, width: u32, height: u32) {
        let props = self.get_tile_properties_mut(tile_index);
        props.grid_width = width.max(1);
        props.grid_height = height.max(1);
        // Clean up if properties are now empty
        if props.is_empty() {
            self.tile_properties.remove(&tile_index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_image_tileset() {
        let tileset = Tileset::new("Test".to_string(), "tiles.png".to_string(), 32, 10, 10);

        assert_eq!(tileset.total_tile_count(), 100);
        assert_eq!(tileset.virtual_to_local(0), Some((0, 0)));
        assert_eq!(tileset.virtual_to_local(99), Some((0, 99)));
        assert_eq!(tileset.virtual_to_local(100), None);
    }

    #[test]
    fn test_multi_image_tileset() {
        let mut tileset = Tileset::new_empty("Test".to_string(), 32);
        tileset.add_image("First".to_string(), "first.png".to_string(), 4, 4); // 16 tiles
        tileset.add_image("Second".to_string(), "second.png".to_string(), 2, 2); // 4 tiles

        assert_eq!(tileset.total_tile_count(), 20);

        // First image tiles
        assert_eq!(tileset.virtual_to_local(0), Some((0, 0)));
        assert_eq!(tileset.virtual_to_local(15), Some((0, 15)));

        // Second image tiles
        assert_eq!(tileset.virtual_to_local(16), Some((1, 0)));
        assert_eq!(tileset.virtual_to_local(19), Some((1, 3)));

        // Out of bounds
        assert_eq!(tileset.virtual_to_local(20), None);
    }

    #[test]
    fn test_local_to_virtual() {
        let mut tileset = Tileset::new_empty("Test".to_string(), 32);
        tileset.add_image("First".to_string(), "first.png".to_string(), 4, 4);
        tileset.add_image("Second".to_string(), "second.png".to_string(), 2, 2);

        assert_eq!(tileset.local_to_virtual(0, 0), Some(0));
        assert_eq!(tileset.local_to_virtual(0, 15), Some(15));
        assert_eq!(tileset.local_to_virtual(1, 0), Some(16));
        assert_eq!(tileset.local_to_virtual(1, 3), Some(19));
    }
}
