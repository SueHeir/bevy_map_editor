//! Standalone Bevy Map Editor binary
//!
//! Install with: cargo install bevy_map_editor
//! Run with: bevy_map_editor

use bevy::prelude::*;
use bevy_map_editor::EditorPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Map Editor".to_string(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EditorPlugin::default())
        .run();
}
