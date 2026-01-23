//! Auto-generated code from bevy_map_editor
//!
//! This module is regenerated when you save your map project with code generation enabled.
//! Do not edit manually - your changes will be overwritten!

use bevy::prelude::*;

mod entities;
mod stubs;
mod behaviors;

pub use entities::*;
pub use stubs::StubsPlugin;
pub use behaviors::BehaviorsPlugin;

/// Plugin that registers all generated systems and components
pub struct GeneratedPlugin;

impl Plugin for GeneratedPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Player>();

        // Add plugins
        app.add_plugins(StubsPlugin);
        app.add_plugins(BehaviorsPlugin);
    }
}