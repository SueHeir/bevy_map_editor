//! External editor integration
//!
//! Provides functions to open game projects in external code editors
//! like VS Code, Cursor, or the system default application.

use std::io;
use std::path::Path;
use std::process::Command;

/// Error type for external editor operations
#[derive(Debug)]
pub enum EditorError {
    /// The specified editor is not installed
    NotInstalled(String),
    /// Failed to launch the editor
    LaunchFailed(String),
    /// The path does not exist
    PathNotFound(String),
}

impl std::fmt::Display for EditorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditorError::NotInstalled(editor) => {
                write!(f, "{} is not installed or not in PATH", editor)
            }
            EditorError::LaunchFailed(msg) => write!(f, "Failed to launch editor: {}", msg),
            EditorError::PathNotFound(path) => write!(f, "Path not found: {}", path),
        }
    }
}

impl std::error::Error for EditorError {}

impl From<io::Error> for EditorError {
    fn from(e: io::Error) -> Self {
        EditorError::LaunchFailed(e.to_string())
    }
}

/// Check if VS Code is installed
pub fn is_vscode_installed() -> bool {
    Command::new("code")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if Cursor is installed
pub fn is_cursor_installed() -> bool {
    Command::new("cursor")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Open a path in VS Code
///
/// If a file is specified, VS Code will open the containing folder and the file.
/// If a directory is specified, VS Code will open the directory.
pub fn open_in_vscode(path: &Path) -> Result<(), EditorError> {
    if !path.exists() {
        return Err(EditorError::PathNotFound(path.display().to_string()));
    }

    let output = Command::new("code").arg(path).spawn()?;

    // We don't wait for the process - VS Code runs independently
    std::mem::forget(output);

    Ok(())
}

/// Open a path in Cursor (VS Code fork)
pub fn open_in_cursor(path: &Path) -> Result<(), EditorError> {
    if !path.exists() {
        return Err(EditorError::PathNotFound(path.display().to_string()));
    }

    let output = Command::new("cursor").arg(path).spawn()?;

    std::mem::forget(output);

    Ok(())
}

/// Open a path with the system default application
///
/// On Windows, this uses `explorer`.
/// On macOS, this uses `open`.
/// On Linux, this uses `xdg-open`.
pub fn open_with_default(path: &Path) -> Result<(), EditorError> {
    if !path.exists() {
        return Err(EditorError::PathNotFound(path.display().to_string()));
    }

    #[cfg(target_os = "windows")]
    let result = Command::new("explorer").arg(path).spawn();

    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(path).spawn();

    #[cfg(target_os = "linux")]
    let result = Command::new("xdg-open").arg(path).spawn();

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let result: Result<std::process::Child, io::Error> = Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Platform not supported",
    ));

    match result {
        Ok(child) => {
            std::mem::forget(child);
            Ok(())
        }
        Err(e) => Err(EditorError::LaunchFailed(e.to_string())),
    }
}

/// Open a specific file at a line number in VS Code
pub fn open_file_at_line_vscode(path: &Path, line: u32) -> Result<(), EditorError> {
    if !path.exists() {
        return Err(EditorError::PathNotFound(path.display().to_string()));
    }

    let arg = format!("{}:{}", path.display(), line);
    let output = Command::new("code").arg("-g").arg(arg).spawn()?;

    std::mem::forget(output);

    Ok(())
}

/// Preferred editor type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreferredEditor {
    /// VS Code
    #[default]
    VSCode,
    /// Cursor (VS Code fork)
    Cursor,
    /// System default
    SystemDefault,
}

impl PreferredEditor {
    /// Get all available editors
    pub fn all() -> &'static [PreferredEditor] {
        &[
            PreferredEditor::VSCode,
            PreferredEditor::Cursor,
            PreferredEditor::SystemDefault,
        ]
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            PreferredEditor::VSCode => "VS Code",
            PreferredEditor::Cursor => "Cursor",
            PreferredEditor::SystemDefault => "System Default",
        }
    }

    /// Check if this editor is available
    pub fn is_available(&self) -> bool {
        match self {
            PreferredEditor::VSCode => is_vscode_installed(),
            PreferredEditor::Cursor => is_cursor_installed(),
            PreferredEditor::SystemDefault => true,
        }
    }

    /// Open a path with this editor
    pub fn open(&self, path: &Path) -> Result<(), EditorError> {
        match self {
            PreferredEditor::VSCode => open_in_vscode(path),
            PreferredEditor::Cursor => open_in_cursor(path),
            PreferredEditor::SystemDefault => open_with_default(path),
        }
    }
}

/// Detect the best available editor
pub fn detect_best_editor() -> PreferredEditor {
    if is_vscode_installed() {
        PreferredEditor::VSCode
    } else if is_cursor_installed() {
        PreferredEditor::Cursor
    } else {
        PreferredEditor::SystemDefault
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preferred_editor() {
        let editors = PreferredEditor::all();
        assert_eq!(editors.len(), 3);

        assert_eq!(PreferredEditor::VSCode.display_name(), "VS Code");
        assert_eq!(PreferredEditor::Cursor.display_name(), "Cursor");
        assert_eq!(
            PreferredEditor::SystemDefault.display_name(),
            "System Default"
        );
    }

    #[test]
    fn test_detect_best_editor() {
        // This test just ensures the function runs without panicking
        let _ = detect_best_editor();
    }
}
