// crates/ambition_sandbox/src/ui_fonts.rs

use bevy::log::{info, warn};
use bevy::prelude::*;

pub const DIALOG_FONT_REGULAR: &str = "fonts/bundled/InterDisplay-Regular.otf";
pub const DIALOG_FONT_SEMIBOLD: &str = "fonts/bundled/InterDisplay-SemiBold.otf";
pub const DEBUG_FONT_MONO: &str = "fonts/bundled/JetBrainsMono-Regular.ttf";

const LEGACY_DIALOG_FONT_REGULAR: &str = "fonts/local/InterDisplay-Regular.otf";
const LEGACY_DIALOG_FONT_SEMIBOLD: &str = "fonts/local/InterDisplay-SemiBold.otf";
const LEGACY_DEBUG_FONT_MONO: &str = "fonts/local/DejaVuSansMono.ttf";

#[derive(Resource, Clone, Debug, Default)]
pub struct UiFonts {
    pub regular: Option<Handle<Font>>,
    pub semibold: Option<Handle<Font>>,
    pub mono: Option<Handle<Font>>,
}

impl UiFonts {
    pub fn has_dialog_font(&self) -> bool {
        self.regular.is_some()
    }

    pub fn selected_marker(&self) -> &'static str {
        if self.has_dialog_font() {
            "►"
        } else {
            ">"
        }
    }

    pub fn text_font(&self, size: f32, weight: UiFontWeight) -> TextFont {
        let handle = match weight {
            UiFontWeight::Regular => self.regular.clone(),
            UiFontWeight::Semibold => self.semibold.clone().or_else(|| self.regular.clone()),
            UiFontWeight::Monospace => self.mono.clone().or_else(|| self.regular.clone()),
        };

        let mut font = TextFont {
            font_size: size,
            ..default()
        };

        if let Some(handle) = handle {
            font.font = handle;
        }

        font
    }
}

#[derive(Clone, Copy, Debug)]
pub enum UiFontWeight {
    Regular,
    Semibold,
    Monospace,
}

pub fn load_ui_fonts(mut commands: Commands, asset_server: Res<AssetServer>) {
    let regular = load_first_available_font(
        &asset_server,
        &[DIALOG_FONT_REGULAR, LEGACY_DIALOG_FONT_REGULAR],
        "regular dialogue UI font",
    );

    let semibold = load_first_available_font(
        &asset_server,
        &[DIALOG_FONT_SEMIBOLD, LEGACY_DIALOG_FONT_SEMIBOLD],
        "semibold dialogue UI font",
    )
    .or_else(|| regular.clone());

    let mono = load_first_available_font(
        &asset_server,
        &[DEBUG_FONT_MONO, LEGACY_DEBUG_FONT_MONO],
        "monospace debug UI font",
    );

    if regular.is_none() {
        warn!(
            "No bundled dialogue UI font found; falling back to Bevy default font and ASCII selector. \
             Run scripts/grab_font_assets.py and check in the generated IPFS-tracked assets. \
             Expected asset path: assets/{DIALOG_FONT_REGULAR}"
        );
    }

    if mono.is_none() {
        warn!(
            "No bundled monospace debug UI font found; debug HUD will fall back to the regular UI font or Bevy default. \
             Expected asset path: assets/{DEBUG_FONT_MONO}"
        );
    }

    commands.insert_resource(UiFonts {
        regular,
        semibold,
        mono,
    });
}

fn load_first_available_font(
    asset_server: &AssetServer,
    relative_asset_paths: &[&'static str],
    label: &str,
) -> Option<Handle<Font>> {
    for relative_asset_path in relative_asset_paths {
        if asset_exists(relative_asset_path) {
            info!("Using {label}: assets/{relative_asset_path}");
            return Some(asset_server.load(*relative_asset_path));
        }
    }
    warn!(
        "Missing {label}; tried {}",
        relative_asset_paths
            .iter()
            .map(|path| format!("assets/{path}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    None
}

fn asset_exists(relative_asset_path: &str) -> bool {
    // Android assets live inside the APK. Let Bevy's Android asset reader try
    // the load instead of probing a host filesystem path.
    #[cfg(target_os = "android")]
    {
        let _ = relative_asset_path;
        true
    }

    // Desktop / Steam Deck bundles can run from a different path than the
    // Linux machine that built them. Check the same app-root layout Bevy uses
    // first, but tolerate both BEVY_ASSET_ROOT=<app> and
    // BEVY_ASSET_ROOT=<app>/assets while preserving local cargo-run fallback.
    #[cfg(not(target_os = "android"))]
    {
        desktop_asset_exists(relative_asset_path)
    }
}

#[cfg(not(target_os = "android"))]
fn desktop_asset_exists(rel_path: &str) -> bool {
    let rel = std::path::Path::new(rel_path);
    let mut candidates = Vec::new();

    if let Some(root) = std::env::var_os("BEVY_ASSET_ROOT") {
        let root = std::path::PathBuf::from(root);
        // Preferred form: BEVY_ASSET_ROOT points at the app/project root,
        // and Bevy's file asset reader loads from root/assets/<rel>.
        candidates.push(root.join("assets").join(rel));
        // Tolerate launchers that set BEVY_ASSET_ROOT to the assets dir.
        candidates.push(root.join(rel));
    }

    if let Ok(cwd) = std::env::current_dir() {
        // Direct binary launches from the app dir.
        candidates.push(cwd.join("assets").join(rel));
        // Tolerate launches from the assets dir or compatibility symlinks.
        candidates.push(cwd.join(rel));
    }

    // Local cargo run / tests fallback.
    candidates.push(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(rel),
    );

    candidates.into_iter().any(|path| path.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_no_fonts() {
        let fonts = UiFonts::default();
        assert!(!fonts.has_dialog_font());
        assert!(fonts.regular.is_none());
        assert!(fonts.semibold.is_none());
        assert!(fonts.mono.is_none());
    }

    #[test]
    fn selected_marker_falls_back_to_ascii_when_no_dialog_font() {
        // Without a dialog font, use a portable ">" marker (the
        // unicode "►" pointer needs a bundled UI font to render legibly).
        let fonts = UiFonts::default();
        assert_eq!(fonts.selected_marker(), ">");
    }

    #[test]
    fn text_font_uses_size_even_without_handle() {
        let fonts = UiFonts::default();
        let font = fonts.text_font(14.0, UiFontWeight::Regular);
        assert_eq!(font.font_size, 14.0);
    }
}
