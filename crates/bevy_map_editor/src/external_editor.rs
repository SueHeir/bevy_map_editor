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

/// Check if VS Code is installed (or available at a custom path)
pub fn is_vscode_installed() -> bool {
    is_vscode_available(None)
}

/// Check if VS Code is available, optionally at a custom path
pub fn is_vscode_available(custom_path: Option<&str>) -> bool {
    // If a custom path is provided, check if it exists
    if let Some(path) = custom_path {
        if !path.is_empty() && std::path::Path::new(path).exists() {
            return true;
        }
    }

    // Try running code --version first (works if VS Code is in PATH)
    if Command::new("code")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return true;
    }

    // On Windows, check common install paths
    #[cfg(target_os = "windows")]
    {
        if let Some(path) = get_default_vscode_path() {
            if std::path::Path::new(&path).exists() {
                return true;
            }
        }
    }

    false
}

/// Get the default VS Code path on Windows
#[cfg(target_os = "windows")]
pub fn get_default_vscode_path() -> Option<String> {
    let paths = [
        std::env::var("LOCALAPPDATA")
            .ok()
            .map(|p| format!("{}/Programs/Microsoft VS Code/Code.exe", p)),
        std::env::var("PROGRAMFILES")
            .ok()
            .map(|p| format!("{}/Microsoft VS Code/Code.exe", p)),
    ];
    for path in paths.into_iter().flatten() {
        if std::path::Path::new(&path).exists() {
            return Some(path);
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
pub fn get_default_vscode_path() -> Option<String> {
    None
}

/// Open a path in VS Code
///
/// If a file is specified, VS Code will open the containing folder and the file.
/// If a directory is specified, VS Code will open the directory.
pub fn open_in_vscode(path: &Path) -> Result<(), EditorError> {
    open_in_vscode_with_custom_path(path, None)
}

/// Open a path in VS Code using a custom executable path
///
/// If `vscode_path` is provided and non-empty, uses that path to launch VS Code.
/// Otherwise falls back to the `code` command or auto-detected paths.
pub fn open_in_vscode_with_custom_path(
    path: &Path,
    vscode_path: Option<&str>,
) -> Result<(), EditorError> {
    if !path.exists() {
        return Err(EditorError::PathNotFound(path.display().to_string()));
    }

    // Determine which VS Code executable to use
    let vscode_cmd = if let Some(custom) = vscode_path {
        if !custom.is_empty() && std::path::Path::new(custom).exists() {
            custom.to_string()
        } else {
            get_vscode_command()
        }
    } else {
        get_vscode_command()
    };

    let output = Command::new(&vscode_cmd).arg(path).spawn()?;

    // We don't wait for the process - VS Code runs independently
    std::mem::forget(output);

    Ok(())
}

/// Get the VS Code command to use (either "code" or a detected path)
fn get_vscode_command() -> String {
    // First try the "code" command
    if Command::new("code")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return "code".to_string();
    }

    // Fall back to detected path on Windows
    #[cfg(target_os = "windows")]
    if let Some(path) = get_default_vscode_path() {
        return path;
    }

    // Default to "code" and let it fail if not found
    "code".to_string()
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
    /// System default (file browser)
    SystemDefault,
}

impl PreferredEditor {
    /// Get all available editors
    pub fn all() -> &'static [PreferredEditor] {
        &[PreferredEditor::VSCode, PreferredEditor::SystemDefault]
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            PreferredEditor::VSCode => "VS Code",
            PreferredEditor::SystemDefault => "File Browser",
        }
    }

    /// Check if this editor is available
    pub fn is_available(&self) -> bool {
        match self {
            PreferredEditor::VSCode => is_vscode_installed(),
            PreferredEditor::SystemDefault => true,
        }
    }

    /// Open a path with this editor
    pub fn open(&self, path: &Path) -> Result<(), EditorError> {
        match self {
            PreferredEditor::VSCode => open_in_vscode(path),
            PreferredEditor::SystemDefault => open_with_default(path),
        }
    }
}

/// Detect the best available editor
pub fn detect_best_editor() -> PreferredEditor {
    if is_vscode_installed() {
        PreferredEditor::VSCode
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
        assert_eq!(editors.len(), 2);

        assert_eq!(PreferredEditor::VSCode.display_name(), "VS Code");
        assert_eq!(
            PreferredEditor::SystemDefault.display_name(),
            "File Browser"
        );
    }

    #[test]
    fn test_detect_best_editor() {
        // This test just ensures the function runs without panicking
        let _ = detect_best_editor();
    }
}
