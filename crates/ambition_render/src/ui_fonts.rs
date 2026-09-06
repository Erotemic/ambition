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

/// The family name Parley resolves Ambition's product typeface under.
///
/// ⭐⭐ MEASURED, NOT ASSUMED, and the whole semantic layer rests on it. Bevy
/// 0.19 registers every loaded [`Font`] asset into Parley's collection under the
/// family name EMBEDDED IN THE FILE, and fontique reads name id 16 (typographic
/// family) in preference to name id 1 (legacy family). Read out of the two
/// bundled files:
///
/// ```text
/// InterDisplay-Regular.otf    name 1  = "Inter Display"           weight 400
/// InterDisplay-SemiBold.otf   name 1  = "Inter Display SemiBold"   weight 600
///                             name 16 = "Inter Display"  name 17 = "SemiBold"
/// ```
///
/// Because name 16 WINS, both faces land in ONE family and the weight chooses
/// between them. Had fontique preferred name 1 they would have been two
/// unrelated families and [`UiFonts::text_font`] could not name a weight — it
/// would still have to know which file is semibold, which is the thing this
/// layer exists to stop callers doing.
pub const PRODUCT_FAMILY: &str = "Inter Display";

/// `JetBrainsMono-Regular.ttf`, family name "JetBrains Mono", weight 400.
///
/// A SEPARATE family on purpose: debug monospace is a different typographic
/// role, not a weight of the product face.
pub const DEBUG_MONO_FAMILY: &str = "JetBrains Mono";

/// The bundled product faces, and the one place their identity is known.
///
/// ⛔ THE HANDLES ARE NOT DECORATION EVEN THOUGH NOTHING READS THEM FOR LAYOUT.
/// Text resolves through [`PRODUCT_FAMILY`] now, but a family is registered in
/// Parley's collection only while its `Font` ASSET is alive, and Bevy CLEARS AND
/// REBUILDS the whole collection when one is removed. Holding the strong handles
/// here is what keeps the families resolvable for the life of the app.
///
/// They are also the honest answer to "did the bundled fonts load at all"
/// ([`UiFonts::has_dialog_font`]), which a family name cannot give: an
/// unresolvable `FontSource::Family` does not error, it silently falls back.
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

    /// A [`TextFont`] for a SEMANTIC role — "product UI, semibold", "debug
    /// monospace" — with no caller anywhere naming an asset path, a file, or a
    /// handle.
    ///
    /// The source is a FAMILY plus a WEIGHT rather than a handle, and that is
    /// the difference that matters:
    ///
    /// - a handle names ONE FACE, so semibold had to be a second handle
    ///   threaded through every presentation seam;
    /// - a family names a typeface and lets the weight pick the face, so
    ///   `Semibold` is a property of the request instead of a different asset.
    ///
    /// ⭐ AND IT SURVIVES ASYNC ARRIVAL BY ITSELF. Bevy's
    /// `load_font_assets_into_font_collection` marks every `TextFont` CHANGED
    /// whose `FontSource::Family` newly resolves, so text spawned before its
    /// font loads is re-laid-out the moment it does. That is what retired the
    /// every-frame world-label font repair.
    ///
    /// ⛔ THE FAMILY IS NAMED ONLY WHEN ITS ASSET LOADED. An unresolvable
    /// `FontSource::Family` does NOT error — Parley falls back silently — so
    /// asking for a family whose file is missing would turn "no bundled font" from
    /// a stated condition into invisible tofu. With no handle the source is left
    /// at Bevy's default, which is exactly what the old handle path did, and
    /// [`Self::has_dialog_font`] stays the one place that says so.
    ///
    /// `size` takes `impl Into<FontSize>` so a caller holding a `TextFont`'s own
    /// `font_size` can pass it straight through; a bare `f32` still means pixels.
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
    /// ⛔ THIS NAMES ONLY A FAMILY SOMETHING ACTUALLY REGISTERED, and the
    /// distinction is the whole reason it is a function rather than a lookup
    /// table. `UiFontWeight::family` says which family a role WANTS; this says
    /// which one is there. Naming the monospace family when the JetBrains file
    /// is missing would ask Parley for a family nothing registered, and an
    /// unresolvable family does not error — it falls back silently, which turns
    /// a missing bundled asset into tofu nobody reported. The old handle path
    /// fell back to the regular FACE in exactly that case, and so does this.
    pub fn font_source(&self, weight: UiFontWeight) -> Option<FontSource> {
        let family = match weight {
            UiFontWeight::Regular => self.regular.is_some().then_some(PRODUCT_FAMILY),
            // Both product weights are ONE family, so a missing semibold FILE is
            // only a problem when regular is missing too — and even then the
            // request degrades to the 400 face rather than to nothing.
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

/// The typographic ROLES Ambition draws in. Not a list of files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiFontWeight {
    Regular,
    Semibold,
    Monospace,
}

impl UiFontWeight {
    /// The family this role WANTS. What it actually gets, given which bundled
    /// files loaded, is [`UiFonts::font_source`].
    pub fn family(self) -> &'static str {
        match self {
            // Both product weights are one family; see `PRODUCT_FAMILY`.
            UiFontWeight::Regular | UiFontWeight::Semibold => PRODUCT_FAMILY,
            UiFontWeight::Monospace => DEBUG_MONO_FAMILY,
        }
    }

    /// The weight that picks the face WITHIN that family.
    pub fn font_weight(self) -> FontWeight {
        match self {
            UiFontWeight::Regular | UiFontWeight::Monospace => FontWeight::NORMAL,
            UiFontWeight::Semibold => FontWeight::SEMIBOLD,
        }
    }
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

    /// ⛔ WITH NO BUNDLED FONT, DO NOT NAME A FAMILY.
    ///
    /// An unresolvable `FontSource::Family` does not error; Parley falls back and
    /// the missing asset becomes invisible. Leaving the default source keeps the
    /// old behaviour AND keeps `has_dialog_font` the single statement of it.
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

    /// ⭐ THE TWO PRODUCT WEIGHTS ARE ONE FAMILY AND TWO WEIGHTS.
    ///
    /// This is the claim the whole layer rests on: a caller says "semibold" and
    /// gets the semibold FACE without anyone naming `InterDisplay-SemiBold.otf`.
    /// `crates/ambition_render/tests/typography.rs` proves the resolution against
    /// the real font files; this pins the REQUEST.
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

        // Debug monospace is a DIFFERENT ROLE, so a different family.
        let mono = fonts.text_font(14.0, UiFontWeight::Monospace);
        assert_eq!(mono.font, FontSource::Family(DEBUG_MONO_FAMILY.into()));
        assert_ne!(mono.font, regular.font);
    }

    /// ⛔ A ROLE WHOSE OWN FILE IS MISSING MUST NOT NAME ITS FAMILY ANYWAY.
    ///
    /// A fresh checkout that ran `grab_font_assets.py` only partly can have the
    /// product face and no monospace one. Asking for `JetBrains Mono` there names
    /// a family nothing registered, and Parley falls back WITHOUT SAYING SO —
    /// which is how a missing bundled asset becomes tofu nobody reports. The
    /// handle path fell back to the regular face; so does this.
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
