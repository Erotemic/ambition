//! UI font loading for the presentation layer.
//!
//! Loads the regular / semibold / mono `Handle<Font>`s into the [`UiFonts`]
//! resource for the dialog overlay, HUD, and menus. All path/existence policy
//! goes through `ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog`.

// UI font loading. All path/existence policy goes through
// `ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog`; there are no
// `target_os = "android"` cfg branches or `BEVY_ASSET_ROOT` probes
// in this module.

use bevy::log::{info, warn};
use bevy::prelude::*;

use ambition_asset_manager::AssetId;

use ambition_asset_manager::platformer_assets::{ids, Platformer2dAssetCatalog};

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

    /// `size` takes `impl Into<FontSize>` so a caller holding a `TextFont`'s own
    /// `font_size` can pass it straight through; a bare `f32` still means pixels.
    pub fn text_font(&self, size: impl Into<FontSize>, weight: UiFontWeight) -> TextFont {
        let handle = match weight {
            UiFontWeight::Regular => self.regular.clone(),
            UiFontWeight::Semibold => self.semibold.clone().or_else(|| self.regular.clone()),
            UiFontWeight::Monospace => self.mono.clone().or_else(|| self.regular.clone()),
        };

        let mut font = TextFont {
            font_size: size.into(),
            ..default()
        };

        if let Some(handle) = handle {
            font.font = handle.into();
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

/// The set [`load_ui_fonts`] runs in — UI font handles exist.
///
/// Anything spawning text at Startup has to follow it, and both the touch
/// overlay (another crate) and the app's own UI said so by name.
///
/// an empty set makes `.before`/`.after` VACUOUS, and Bevy does not warn.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UiFontsLoaded;

/// Owns the UI font load — the resource AND the system that fills it.
///
/// a consumer that pins [`UiFontsLoaded`] should install this itself, via
/// [`ensure_installed`], rather than assuming some other plugin did. That is the
/// difference between an ordering edge that holds in every composition and one
/// that holds in the app that happened to wire it.
pub struct UiFontsPlugin;

impl bevy::prelude::Plugin for UiFontsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::IntoScheduleConfigs;
        app.add_systems(bevy::prelude::Startup, load_ui_fonts.in_set(UiFontsLoaded));
    }
}

impl UiFontsPlugin {
    /// Install [`UiFontsPlugin`] unless it is already present.
    ///
    /// every call site must go through this. Bevy PANICS on a duplicate
    /// plugin, and the whole point is that more than one crate now takes
    /// responsibility for the font load being present — the app and the touch
    /// overlay both do, and neither can know which built the `App` first.
    pub fn ensure_installed(app: &mut bevy::prelude::App) {
        if !app.is_plugin_added::<Self>() {
            app.add_plugins(Self);
        }
    }
}

/// Bevy startup system: walk each font's canonical + legacy catalog
/// ids, pick the first one whose asset is present under the active
/// [`ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog`] profile, and store
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
    catalog: Option<Res<Platformer2dAssetCatalog>>,
) {
    let Some(catalog) = catalog else {
        warn!(
            "ui_fonts: Platformer2dAssetCatalog resource missing; falling back to Bevy's default font. \
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
    catalog: &Platformer2dAssetCatalog,
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
        assert_eq!(font.font_size, FontSize::Px(14.0));
    }
}
