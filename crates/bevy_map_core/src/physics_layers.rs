//! Physics Layers Config for Bevy Map Editor

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{CollisionData, CollisionShape, OneWayDirection};

/// Configuration for autotiling in a project
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhysicsLayers {
    /// All physics layers in the tileset
    pub layers: Vec<PhysicsLayerSet>,
}

impl PhysicsLayers {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Add a physics layer set
    pub fn add_physics_layer(&mut self, physics_layer: PhysicsLayerSet) {
        self.layers.push(physics_layer);
    }

    /// Get physics layer set by ID
    pub fn get_physics_layer(&self, id: Uuid) -> Option<&PhysicsLayerSet> {
        self.layers.iter().find(|ts| ts.id == id)
    }

    /// Get mutable physics layer set by ID
    pub fn get_physics_layer_mut(&mut self, id: Uuid) -> Option<&mut PhysicsLayerSet> {
        self.layers.iter_mut().find(|ts| ts.id == id)
    }

    /// Remove physics layer set by ID
    pub fn remove_physics_layer(&mut self, id: Uuid) -> Option<PhysicsLayerSet> {
        if let Some(pos) = self.layers.iter().position(|ts| ts.id == id) {
            Some(self.layers.remove(pos))
        } else {
            None
        }
    }
}

/// A physics layer set attached to a tileset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsLayerSet {
    pub id: Uuid,
    pub name: String,

    #[serde(default)]
    pub debug_color: [u8; 3],

    pub layer: u8,
    pub mask: u32,

    /// Physics assignments for each tile (tile_index -> CollisionData)
    pub tile_physics: HashMap<u32, CollisionData>,
}

impl PhysicsLayerSet {
    /// Create a new physics layer set
    pub fn new(name: String, layer: u8, mask: u32, debug_color: [u8; 3]) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            debug_color,
            layer,
            mask,
            tile_physics: HashMap::new(),
        }
    }

    /// Assign collision data to a tile index
    pub fn assign_tile_physics(&mut self, tile_index: u32, collision: CollisionData) {
        self.tile_physics.insert(tile_index, collision);
    }

    /// Get collision data for a tile index
    pub fn get_tile_physics(&self, tile_index: u32) -> Option<&CollisionData> {
        self.tile_physics.get(&tile_index)
    }

    /// Get collision data for a tile index
    pub fn get_tile_physics_mut(&mut self, tile_index: u32) -> Option<&mut CollisionData> {
        self.tile_physics.get_mut(&tile_index)
    }

    pub fn set_tile_physics_shape(&mut self, tile_index: u32, shape: CollisionShape) {
        if let Some(collision_data) = self.tile_physics.get_mut(&tile_index) {
            collision_data.shape = shape;
        } else {
            self.tile_physics.insert(
                tile_index,
                CollisionData {
                    shape,
                    body_type: Default::default(),
                    one_way: Default::default(),
                    layer: self.layer,
                    mask: self.mask,
                },
            );
        }
    }

    pub fn set_tile_physics_one_way(&mut self, tile_index: u32, direction: OneWayDirection) {
        if let Some(collision_data) = self.tile_physics.get_mut(&tile_index) {
            collision_data.one_way = direction;
        }
    }

    pub fn update_all_tile_physics_layer(&mut self, layer: u8) {
        self.layer = layer;
        for collision_data in self.tile_physics.values_mut() {
            collision_data.layer = layer;
        }
    }

    pub fn update_all_tile_physics_mask(&mut self, mask: u32) {
        self.mask = mask;
        for collision_data in self.tile_physics.values_mut() {
            collision_data.mask = mask;
        }
    }
}
