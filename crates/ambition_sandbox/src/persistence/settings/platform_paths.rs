//! OS-conventional data directory resolution shared by every
//! sandbox file persistence path (settings.ron, sandbox_save.ron,
//! and any future user-data file).
//!
//! Resolves the platform-correct user data root, falling back to the
//! current working directory when no env var is set (tests, CI).
//!
//! - **Linux**: `$XDG_DATA_HOME` or `$HOME/.local/share`.
//! - **macOS**: `$HOME/Library/Application Support`.
//! - **Windows**: `%APPDATA%`.
//! - **Android**: `/data/data/<app_id>/files`. App ID comes from
//!   `AMBITION_ANDROID_APP_ID` at compile time
//!   (default: `org.erotemic.ambition.sandbox`).
//! - **`AMBITION_DATA_DIR` override**: tests / sandbox sessions can
//!   set this env var to force a known directory. Always wins.

use std::path::PathBuf;

/// Subpath under the data root where every Ambition file lives.
pub const AMBITION_DIR: &str = "ambition";

/// Resolve the OS-conventional data directory for Ambition.
///
/// On Linux this resolves to `$XDG_DATA_HOME/ambition` or
/// `$HOME/.local/share/ambition`; on macOS to
/// `~/Library/Application Support/ambition`; on Windows to
/// `%APPDATA%\ambition`; on Android to the app's internal files
/// directory.
pub fn data_dir_root() -> PathBuf {
    if let Ok(value) = std::env::var("AMBITION_DATA_DIR") {
        // Tests / sandbox sessions can override the dir explicitly.
        return PathBuf::from(value);
    }
    #[cfg(target_os = "android")]
    {
        // Android's process working directory is read-only in a GameActivity
        // APK. Use the app's internal files directory instead so settings and
        // saves persist without spamming logcat with EROFS warnings. The build
        // script passes AMBITION_ANDROID_APP_ID at compile time so custom app
        // IDs still get the matching package directory.
        let app_id =
            option_env!("AMBITION_ANDROID_APP_ID").unwrap_or("org.erotemic.ambition.sandbox");
        return PathBuf::from("/data/data").join(app_id).join("files");
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            return PathBuf::from(xdg);
        }
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".local/share");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join("Library/Application Support");
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata);
        }
    }
    PathBuf::from(".")
}
