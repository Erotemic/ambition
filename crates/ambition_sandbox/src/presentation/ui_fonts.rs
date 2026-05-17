// crates/ambition_sandbox/src/ui_fonts.rs
//
// UI font loading. All path/existence policy goes through
// `crate::sandbox_assets::SandboxAssetCatalog`; there are no
// `target_os = "android"` cfg branches or `BEVY_ASSET_ROOT` probes
// in this module.

use bevy::log::{info, warn};
use bevy::prelude::*;

use ambition_asset_manager::AssetId;

use crate::sandbox_assets::{ids, SandboxAssetCatalog};

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

/// Bevy startup system: walk each font's canonical + legacy catalog
/// ids, pick the first one whose asset is present under the active
/// [`crate::sandbox_assets::SandboxAssetCatalog`] profile, and store
/// the resulting `Handle<Font>` in [`UiFonts`].
///
/// Missing fonts are non-fatal — the rendering layer falls back to
/// Bevy's default font and ASCII selector. The font catalog uses
/// `MissingAssetPolicy::WarnAndPlaceholder` (canonical) /
/// `SilentPlaceholder` (legacy) so the warning text below is the only
/// noise on a fresh checkout that hasn't run
/// `scripts/grab_font_assets.py`.
pub fn load_ui_fonts(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    catalog: Option<Res<SandboxAssetCatalog>>,
) {
    let Some(catalog) = catalog else {
        warn!(
            "ui_fonts: SandboxAssetCatalog resource missing; falling back to Bevy's default font. \
             This means the visible app forgot to install AmbitionAssetManagerPlugin."
        );
        commands.insert_resource(UiFonts::default());
        return;
    };

    let regular = load_first_available_font(
        &catalog,
        &asset_server,
        &[
            ids::font_dialog_regular(),
            AssetId::new("font.dialog_regular.legacy"),
        ],
        "regular dialogue UI font",
    );

    let semibold = load_first_available_font(
        &catalog,
        &asset_server,
        &[
            ids::font_dialog_semibold(),
            AssetId::new("font.dialog_semibold.legacy"),
        ],
        "semibold dialogue UI font",
    )
    .or_else(|| regular.clone());

    let mono = load_first_available_font(
        &catalog,
        &asset_server,
        &[
            ids::font_debug_mono(),
            AssetId::new("font.debug_mono.legacy"),
        ],
        "monospace debug UI font",
    );

    if regular.is_none() {
        warn!(
            "No bundled dialogue UI font found; falling back to Bevy default font and ASCII selector. \
             Run scripts/grab_font_assets.py and check in the generated IPFS-tracked assets."
        );
    }

    if mono.is_none() {
        warn!(
            "No bundled monospace debug UI font found; debug HUD will fall back to the regular UI font or Bevy default."
        );
    }

    commands.insert_resource(UiFonts {
        regular,
        semibold,
        mono,
    });
}

fn load_first_available_font(
    catalog: &SandboxAssetCatalog,
    asset_server: &AssetServer,
    ids: &[AssetId],
    label: &str,
) -> Option<Handle<Font>> {
    let mut tried = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(path) = catalog.try_path_for_load(id) {
            info!("Using {label}: assets/{path} (catalog id {id})");
            return Some(asset_server.load(path));
        }
        tried.push(format!("{id} (skipped by profile gate)"));
    }
    warn!("Missing {label}; tried {}", tried.join(", "));
    None
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
