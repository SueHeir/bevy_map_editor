//! Editor preferences and persistent settings
//!
//! Manages user preferences stored in platform-specific config directories:
//! - Windows: %APPDATA%/bevy_map_editor/
//! - Linux: ~/.config/bevy_map_editor/
//! - macOS: ~/Library/Application Support/bevy_map_editor/

mod file;

pub use file::*;

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::ui::{EditorTool, ToolMode};

/// Maximum number of recent projects to track
pub const MAX_RECENT_PROJECTS: usize = 10;

/// Editor preferences that persist across sessions
#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct EditorPreferences {
    /// Version for future migrations
    pub version: u32,

    // App Settings
    pub app_scale: f32,

    // UI Panel State
    pub show_tree_view: bool,
    pub show_inspector: bool,
    pub tree_view_width: f32,
    pub inspector_width: f32,

    // Editor View Settings
    pub show_grid: bool,
    pub show_collisions: bool,
    pub snap_to_grid: bool,
    pub zoom: f32,

    // Default Tool
    pub default_tool: EditorTool,
    pub default_tool_mode: ToolMode,

    // Recent Projects
    pub recent_projects: Vec<RecentProject>,

    // Startup behavior
    pub auto_open_last_project: bool,

    // Custom keybindings (action name -> key combo string)
    pub keybindings: HashMap<String, String>,

    // Theme settings
    pub theme: ThemeSettings,

    // Input settings
    /// Trackpad mode: swap scroll behavior (scroll = pan, Ctrl+scroll = zoom)
    pub trackpad_mode: bool,
    /// Pan sensitivity for trackpad mode (multiplier, default 1.0)
    pub trackpad_pan_sensitivity: f32,
    /// Zoom sensitivity for trackpad mode (multiplier, default 1.0)
    pub trackpad_zoom_sensitivity: f32,

    // Window settings
    /// Remembered window width (None = use default 1920)
    pub window_width: Option<f32>,
    /// Remembered window height (None = use default 1080)
    pub window_height: Option<f32>,
    /// Remembered window X position
    pub window_x: Option<i32>,
    /// Remembered window Y position
    pub window_y: Option<i32>,
    /// Remembered window mode ("windowed", "borderless_fullscreen", "fullscreen")
    pub window_mode: Option<String>,
}

/// A recent project entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub path: String,
    pub name: String,
    pub last_opened: u64, // Unix timestamp
}

/// Theme customization settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeSettings {
    pub use_dark_theme: bool,
    pub accent_color: Option<[u8; 3]>,
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            version: 1,
            app_scale: 1.0,
            show_tree_view: true,
            show_inspector: true,
            tree_view_width: 200.0,
            inspector_width: 250.0,
            show_grid: true,
            show_collisions: false,
            snap_to_grid: true,
            zoom: 1.0,
            default_tool: EditorTool::Select,
            default_tool_mode: ToolMode::Point,
            recent_projects: Vec::new(),
            auto_open_last_project: false,
            keybindings: HashMap::new(),
            theme: ThemeSettings::default(),
            trackpad_mode: false,
            trackpad_pan_sensitivity: 0.5,
            trackpad_zoom_sensitivity: 1.0,
            window_width: None,
            window_height: None,
            window_x: None,
            window_y: None,
            window_mode: None,
        }
    }
}

impl EditorPreferences {
    /// Add a project to recent projects list
    pub fn add_recent_project(&mut self, path: PathBuf, name: String) {
        use std::time::{SystemTime, UNIX_EPOCH};

        let path_str = path.to_string_lossy().to_string();

        // Remove if already exists (will re-add at front)
        self.recent_projects.retain(|p| p.path != path_str);

        // Add at front
        self.recent_projects.insert(
            0,
            RecentProject {
                path: path_str,
                name,
                last_opened: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            },
        );

        // Trim to max size
        self.recent_projects.truncate(MAX_RECENT_PROJECTS);
    }

    /// Remove a project from recent list (e.g., if file no longer exists)
    pub fn remove_recent_project(&mut self, path: &str) {
        self.recent_projects.retain(|p| p.path != path);
    }

    /// Get the most recently opened project path
    pub fn last_project(&self) -> Option<&RecentProject> {
        self.recent_projects.first()
    }

    /// Clear all recent projects
    pub fn clear_recent_projects(&mut self) {
        self.recent_projects.clear();
    }
}
