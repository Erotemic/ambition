//! UI font loading for the presentation layer.
//!
//! Loads the regular / semibold / mono `Handle<Font>`s into the [`UiFonts`]
//! resource for the dialog overlay, HUD, and menus. All path/existence policy
//! goes through `ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog`.

// There are no `target_os = "android"` cfg branches or `BEVY_ASSET_ROOT`
// probes in this module.

use bevy::log::{info, warn};
use bevy::prelude::*;

use ambition_asset_manager::AssetId;

use ambition_asset_manager::platformer_assets::{ids, Platformer2dAssetCatalog};

/// The family name Parley resolves Ambition's product typeface under.
///
/// Measured from the files. Bevy 0.19 registers each loaded [`Font`] asset
/// into Parley's collection under the family name embedded in the file, and
/// fontique prefers name id 16 (typographic family) over name id 1 (legacy
/// family). The two bundled files:
///
/// ```text
/// InterDisplay-Regular.otf    name 1  = "Inter Display"           weight 400
/// InterDisplay-SemiBold.otf   name 1  = "Inter Display SemiBold"   weight 600
///                             name 16 = "Inter Display"  name 17 = "SemiBold"
/// ```
///
/// Because name 16 wins, both faces are one family and the weight picks the
/// face. If fontique preferred name 1, they would be two unrelated families
/// and [`UiFonts::text_font`] could not select by weight.
pub const PRODUCT_FAMILY: &str = "Inter Display";

/// `JetBrainsMono-Regular.ttf`, family name "JetBrains Mono", weight 400.
///
/// A separate family on purpose: debug monospace is a different role, not a
/// weight of the product face.
pub const DEBUG_MONO_FAMILY: &str = "JetBrains Mono";

/// The bundled product faces, and the one place their identity is known.
///
/// The handles matter even though layout does not read them. Text resolves
/// through [`PRODUCT_FAMILY`], but a family stays in Parley's collection only
/// while its `Font` asset is alive, and Bevy rebuilds the whole collection
/// when one is removed. Holding strong handles here keeps the families
/// resolvable for the life of the app.
///
/// They also answer "did the bundled fonts load" ([`UiFonts::has_dialog_font`]).
/// A family name cannot: an unresolvable `FontSource::Family` falls back
/// silently.
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

    /// A [`TextFont`] for a semantic role ("product UI, semibold", "debug
    /// monospace"), with no caller naming an asset path, file, or handle.
    ///
    /// The source is a family plus a weight, not a handle. A handle names one
    /// face, so semibold needed a second handle through every seam. A family lets
    /// the weight pick the face.
    ///
    /// It handles async arrival: Bevy's `load_font_assets_into_font_collection`
    /// marks a `TextFont` changed when its `FontSource::Family` resolves, so text
    /// spawned before its font loads is laid out again when it does.
    ///
    /// The family is named only when its asset loaded. An unresolvable
    /// `FontSource::Family` does not error (Parley falls back silently), so a
    /// missing file would show as tofu. With no handle the source stays at Bevy's
    /// default, and [`Self::has_dialog_font`] reports the condition.
    ///
    /// `size` takes `impl Into<FontSize>` so a caller can pass a `TextFont`'s own
    /// `font_size`; a bare `f32` means pixels.
    pub fn text_font(&self, size: impl Into<FontSize>, weight: UiFontWeight) -> TextFont {
        let mut font = TextFont {
            font_size: size.into(),
            ..default()
        };
        if let Some(source) = self.font_source(weight) {
            font.font = source;
            font.weight = weight.font_weight();
        }
        font
    }

    /// The [`FontSource`] a role resolves to, or `None` when nothing is loaded.
    ///
    /// Names only a family that is registered. `UiFontWeight::family` says which
    /// family a role wants; this says which one is there. Naming the monospace
    /// family when the JetBrains file is missing would fall back silently to
    /// tofu. Instead this falls back to the regular face.
    pub fn font_source(&self, weight: UiFontWeight) -> Option<FontSource> {
        let family = match weight {
            UiFontWeight::Regular => self.regular.is_some().then_some(PRODUCT_FAMILY),
            // Both product weights are one family, so a missing semibold file
            // matters only when regular is missing too. Otherwise the request uses
            // the 400 face.
            UiFontWeight::Semibold => {
                (self.semibold.is_some() || self.regular.is_some()).then_some(PRODUCT_FAMILY)
            }
            UiFontWeight::Monospace => match (self.mono.is_some(), self.regular.is_some()) {
                (true, _) => Some(DEBUG_MONO_FAMILY),
                (false, true) => Some(PRODUCT_FAMILY),
                (false, false) => None,
            },
        };
        family.map(|family| FontSource::Family(family.into()))
    }
}

/// The typographic roles Ambition draws in (not a list of files).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiFontWeight {
    Regular,
    Semibold,
    Monospace,
}

impl UiFontWeight {
    /// The family this role wants. What it gets, given which bundled files
    /// loaded, is [`UiFonts::font_source`].
    pub fn family(self) -> &'static str {
        match self {
            // Both product weights are one family; see `PRODUCT_FAMILY`.
            UiFontWeight::Regular | UiFontWeight::Semibold => PRODUCT_FAMILY,
            UiFontWeight::Monospace => DEBUG_MONO_FAMILY,
        }
    }

    /// The weight that picks the face within that family.
    pub fn font_weight(self) -> FontWeight {
        match self {
            UiFontWeight::Regular | UiFontWeight::Monospace => FontWeight::NORMAL,
            UiFontWeight::Semibold => FontWeight::SEMIBOLD,
        }
    }
}

/// The set [`load_ui_fonts`] runs in; after it, UI font handles exist.
///
/// Anything that spawns text at Startup must run after it (for example the
/// touch overlay in another crate, and the app's UI). If the set is empty,
/// `.before`/`.after` do nothing, and Bevy does not warn.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UiFontsLoaded;

/// Owns the UI font load: the resource and the system that fills it.
///
/// A consumer that orders against [`UiFontsLoaded`] should install this
/// itself with [`ensure_installed`], so the edge holds in every composition.
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
    /// Every call site must use this: Bevy panics on a duplicate plugin, and
    /// more than one crate (the app, the touch overlay) installs it without
    /// knowing which ran first.
    pub fn ensure_installed(app: &mut bevy::prelude::App) {
        if !app.is_plugin_added::<Self>() {
            app.add_plugins(Self);
        }
    }
}

/// Startup system: for each font, walk its canonical and legacy catalog ids,
/// pick the first present under the active
/// [`ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog`]
/// profile, and store the `Handle<Font>` in [`UiFonts`].
///
/// Missing fonts are not fatal; rendering falls back to Bevy's default font
/// and the ASCII selector. The catalog uses
/// `MissingAssetPolicy::WarnAndPlaceholder` (canonical) and
/// `SilentPlaceholder` (legacy), so the warning below is the only noise on a
/// checkout that has not run `scripts/grab_font_assets.py`.
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

    /// With no bundled font, do not name a family.
    ///
    /// An unresolvable `FontSource::Family` falls back silently. The default
    /// source keeps the old behaviour, and `has_dialog_font` reports it.
    #[test]
    fn a_missing_bundled_font_names_no_family() {
        let fonts = UiFonts::default();
        assert_eq!(fonts.font_source(UiFontWeight::Regular), None);
        assert_eq!(fonts.font_source(UiFontWeight::Semibold), None);
        assert_eq!(fonts.font_source(UiFontWeight::Monospace), None);
        assert_eq!(
            fonts.text_font(14.0, UiFontWeight::Semibold).font,
            FontSource::default(),
            "a request with nothing loaded must resolve exactly as it did before \
             the family layer existed"
        );
    }

    /// The two product weights are one family with two weights.
    ///
    /// A caller asks for semibold and gets the semibold face without naming
    /// `InterDisplay-SemiBold.otf`. `crates/ambition_render/tests/typography.rs`
    /// checks resolution against the real files; this checks the request.
    #[test]
    fn semibold_is_a_weight_of_the_product_family_not_a_second_family() {
        let fonts = UiFonts {
            regular: Some(Handle::default()),
            semibold: Some(Handle::default()),
            mono: Some(Handle::default()),
        };
        let regular = fonts.text_font(14.0, UiFontWeight::Regular);
        let semibold = fonts.text_font(14.0, UiFontWeight::Semibold);
        assert_eq!(regular.font, semibold.font, "one family");
        assert_eq!(regular.font, FontSource::Family(PRODUCT_FAMILY.into()));
        assert_ne!(regular.weight, semibold.weight, "two weights");
        assert_eq!(semibold.weight, FontWeight::SEMIBOLD);

        // Debug monospace is a different role, so a different family.
        let mono = fonts.text_font(14.0, UiFontWeight::Monospace);
        assert_eq!(mono.font, FontSource::Family(DEBUG_MONO_FAMILY.into()));
        assert_ne!(mono.font, regular.font);
    }

    /// A role whose own file is missing must not name its family.
    ///
    /// A checkout can have the product face and no monospace face. Naming
    /// `JetBrains Mono` there asks for an unregistered family, and Parley falls
    /// back silently to tofu. Like the handle path, this falls back to the
    /// regular face.
    #[test]
    fn a_role_falls_back_to_the_family_that_actually_loaded() {
        let fonts = UiFonts {
            regular: Some(Handle::default()),
            semibold: None,
            mono: None,
        };
        assert_eq!(
            fonts.font_source(UiFontWeight::Monospace),
            Some(FontSource::Family(PRODUCT_FAMILY.into())),
            "no monospace file loaded, so the request must land on the family that did"
        );
        assert_eq!(
            fonts.font_source(UiFontWeight::Semibold),
            Some(FontSource::Family(PRODUCT_FAMILY.into())),
            "semibold shares the product family, so the regular file alone still serves it"
        );
    }
}
