//! Layer types for tile and object layers

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Sentinel value for cells occupied by multi-cell tiles (but not the base cell)
pub const OCCUPIED_CELL: u32 = u32::MAX;

/// A layer (tiles or objects)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub data: LayerData,
}

impl Layer {
    /// Create a new tile layer with the given tileset
    pub fn new_tile_layer(name: String, tileset_id: Uuid, width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        Self {
            name,
            visible: true,
            data: LayerData::Tiles {
                tileset_id,
                tiles: vec![None; size],
                occupied_cells: HashMap::new(),
            },
        }
    }

    /// Create a new object layer
    pub fn new_object_layer(name: String) -> Self {
        Self {
            name,
            visible: true,
            data: LayerData::Objects {
                entities: Vec::new(),
            },
        }
    }

    /// Get the type of this layer
    pub fn layer_type(&self) -> LayerType {
        match &self.data {
            LayerData::Tiles { .. } => LayerType::Tiles,
            LayerData::Objects { .. } => LayerType::Objects,
        }
    }

    /// Get the tileset ID if this is a tile layer
    pub fn tileset_id(&self) -> Option<Uuid> {
        match &self.data {
            LayerData::Tiles { tileset_id, .. } => Some(*tileset_id),
            LayerData::Objects { .. } => None,
        }
    }
}

/// The type of a layer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerType {
    Tiles,
    Objects,
}

/// The data contained in a layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayerData {
    /// Tile layer with virtual tile indices
    Tiles {
        /// The tileset used for this layer
        tileset_id: Uuid,
        /// Tile data - None means empty, Some(idx) is a virtual tile index
        /// For multi-cell tiles: base cell has the tile index, other cells have OCCUPIED_CELL
        tiles: Vec<Option<u32>>,
        /// Maps occupied cell indices to their base cell index (for multi-cell tiles)
        /// Only populated for cells that are part of a multi-cell tile but not the base
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        occupied_cells: HashMap<usize, usize>,
    },
    /// Object layer containing entity references
    Objects {
        /// Entity IDs placed on this layer
        entities: Vec<Uuid>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tile_layer() {
        let tileset_id = Uuid::new_v4();
        let layer = Layer::new_tile_layer("Ground".to_string(), tileset_id, 10, 10);

        assert_eq!(layer.name, "Ground");
        assert!(layer.visible);
        assert_eq!(layer.layer_type(), LayerType::Tiles);

        if let LayerData::Tiles {
            tiles,
            occupied_cells,
            ..
        } = &layer.data
        {
            assert_eq!(tiles.len(), 100);
            assert!(tiles.iter().all(|t| t.is_none()));
            assert!(occupied_cells.is_empty());
        } else {
            panic!("Expected tile layer");
        }
    }

    #[test]
    fn test_new_object_layer() {
        let layer = Layer::new_object_layer("Entities".to_string());

        assert_eq!(layer.name, "Entities");
        assert!(layer.visible);
        assert_eq!(layer.layer_type(), LayerType::Objects);
    }
}
