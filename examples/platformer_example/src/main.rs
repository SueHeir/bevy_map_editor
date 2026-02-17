//! platformer_example - Game created with bevy_map_editor
//!
//! Run with hot-reload: cargo run --features dev
//! Run for release: cargo run --release

use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_map_runtime::{MapHandle, MapRuntimePlugin};

mod generated;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "platformer_example".to_string(),
                resolution: WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        // Add the map runtime plugin for rendering tilemaps
    .add_plugins(MapRuntimePlugin::default())
        // Add the generated plugin for entity types and behaviors
        .add_plugins(generated::GeneratedPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, camera_controls)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn camera
    commands.spawn((
        Camera2d,
        Transform::from_xyz(400.0, 300.0, 0.0),
    ));

    // Auto-discover and load all maps from assets/maps/
    let maps = discover_maps();
    if maps.is_empty() {
        warn!("No maps found in assets/maps/ - click 'Run Game' in the editor to sync maps");
    } else {
        for map_file in &maps {
            info!("Loading map: {}", map_file);
            commands.spawn(MapHandle(asset_server.load(format!("maps/{}", map_file))));
        }
    }

    info!("Game started!");
    info!("Controls: WASD to pan, Q/E to zoom");

    #[cfg(feature = "dev")]
    info!("Hot-reload enabled: Save your map in the editor to see changes!");
}

/// Discover all map files in assets/maps/
fn discover_maps() -> Vec<String> {
    let maps_dir = std::path::Path::new("assets/maps");
    if !maps_dir.exists() {
        return vec![];
    }

    std::fs::read_dir(maps_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|x| x == "json")
                .unwrap_or(false)
        })
        .filter(|e| e.file_name().to_string_lossy().ends_with(".map.json"))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect()
}

/// Basic camera controls: WASD to pan, Q/E to zoom
fn camera_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
    time: Res<Time>,
) {
    let Ok((mut transform, mut projection)) = query.single_mut() else {
        return;
    };

    let speed = 300.0 * time.delta_secs();

    // WASD or Arrow keys to pan
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        transform.translation.y += speed;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        transform.translation.y -= speed;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        transform.translation.x -= speed;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        transform.translation.x += speed;
    }

    // Q/E to zoom
    if let Projection::Orthographic(ref mut ortho) = *projection {
        if keyboard.pressed(KeyCode::KeyQ) {
            ortho.scale *= 1.0 + time.delta_secs();
        }
        if keyboard.pressed(KeyCode::KeyE) {
            ortho.scale *= 1.0 - time.delta_secs();
        }
        ortho.scale = ortho.scale.clamp(0.25, 4.0);
    }
}
