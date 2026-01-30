//! Standalone Bevy Map Editor binary
//!
//! Install with: cargo install bevy_map_editor
//! Run with: bevy_map_editor

use bevy::asset::{AssetPlugin, UnapprovedPathMode};
use bevy::ecs::message::MessageReader;
use bevy::image::{ImageFilterMode, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::window::{
    MonitorSelection, VideoModeSelection, WindowMode, WindowMoved, WindowPosition, WindowResized,
    WindowResolution,
};
use bevy_map_editor::preferences::EditorPreferences;
use bevy_map_editor::project::Project;
use bevy_map_editor::EditorPlugin;
use std::path::PathBuf;

fn main() {
    // Load preferences early to get saved window size
    let preferences = EditorPreferences::load();
    let window_width = preferences.window_width.unwrap_or(1920.0) as u32;
    let window_height = preferences.window_height.unwrap_or(1080.0) as u32;

    let window_position = match (preferences.window_x, preferences.window_y) {
        (Some(x), Some(y)) => WindowPosition::At(IVec2::new(x, y)),
        _ => WindowPosition::Automatic,
    };

    let window_mode = match preferences.window_mode.as_deref() {
        Some("borderless_fullscreen") => {
            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
        }
        Some("fullscreen") => {
            WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Current)
        }
        _ => WindowMode::Windowed,
    };

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Bevy Map Editor".to_string(),
                        // High DPI support: prevent OS-level scaling that causes blurriness
                        resolution: WindowResolution::new(window_width, window_height)
                            .with_scale_factor_override(1.0),
                        position: window_position,
                        mode: window_mode,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin {
                    // Pixel-perfect rendering: use Nearest (point) sampling for crisp pixel art
                    default_sampler: ImageSamplerDescriptor {
                        mag_filter: ImageFilterMode::Nearest,
                        min_filter: ImageFilterMode::Nearest,
                        mipmap_filter: ImageFilterMode::Nearest,
                        ..default()
                    },
                })
                .set(AssetPlugin {
                    // Allow loading assets from any path (absolute paths, outside assets folder)
                    // This is needed for a map editor where users can place assets anywhere
                    unapproved_path_mode: UnapprovedPathMode::Allow,
                    ..default()
                }),
        )
        .add_plugins(EditorPlugin::default())
        .add_systems(Startup, auto_open_last_project)
        .add_systems(Update, save_window_state_on_change)
        .add_systems(Last, save_window_size_on_exit)
        .run();
}

/// System to auto-open the last project on startup if enabled in preferences
fn auto_open_last_project(mut project: ResMut<Project>, preferences: Res<EditorPreferences>) {
    if !preferences.auto_open_last_project {
        return;
    }

    if let Some(recent) = preferences.last_project() {
        let path = PathBuf::from(&recent.path);
        if path.exists() {
            match Project::load(&path) {
                Ok(loaded) => {
                    *project = loaded;
                    info!("Auto-opened last project: {}", recent.name);
                }
                Err(e) => {
                    warn!("Failed to auto-open project '{}': {}", recent.name, e);
                }
            }
        } else {
            warn!(
                "Last project file not found: {} ({})",
                recent.name, recent.path
            );
        }
    }
}

/// Save window state to preferences whenever the window is moved or resized
fn save_window_state_on_change(
    mut moved_events: MessageReader<WindowMoved>,
    mut resized_events: MessageReader<WindowResized>,
    windows: Query<&Window>,
    mut preferences: ResMut<EditorPreferences>,
) {
    let has_moved = moved_events.read().last().is_some();
    let has_resized = resized_events.read().last().is_some();

    if !has_moved && !has_resized {
        return;
    }

    if let Ok(window) = windows.single() {
        preferences.window_width = Some(window.resolution.width());
        preferences.window_height = Some(window.resolution.height());
        if let WindowPosition::At(pos) = window.position {
            preferences.window_x = Some(pos.x);
            preferences.window_y = Some(pos.y);
        }
        preferences.window_mode = Some(match window.mode {
            WindowMode::BorderlessFullscreen(_) => "borderless_fullscreen".to_string(),
            WindowMode::Fullscreen(_, _) => "fullscreen".to_string(),
            _ => "windowed".to_string(),
        });
        if let Err(e) = preferences.save() {
            error!("Failed to save window state to preferences: {}", e);
        }
    }
}

/// Save window size to preferences when the app exits (fallback)
fn save_window_size_on_exit(
    mut exit_events: MessageReader<AppExit>,
    windows: Query<&Window>,
    mut preferences: ResMut<EditorPreferences>,
) {
    if exit_events.read().next().is_none() {
        return;
    }
    if let Ok(window) = windows.single() {
        preferences.window_width = Some(window.resolution.width());
        preferences.window_height = Some(window.resolution.height());
        if let WindowPosition::At(pos) = window.position {
            preferences.window_x = Some(pos.x);
            preferences.window_y = Some(pos.y);
        }
        preferences.window_mode = Some(match window.mode {
            WindowMode::BorderlessFullscreen(_) => "borderless_fullscreen".to_string(),
            WindowMode::Fullscreen(_, _) => "fullscreen".to_string(),
            _ => "windowed".to_string(),
        });
        if let Err(e) = preferences.save() {
            error!("Failed to save window size to preferences: {}", e);
        }
    }
}
