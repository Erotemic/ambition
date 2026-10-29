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

/// App-local family names for the UI font roles.
pub const PRODUCT_FAMILY: &str = "ambition.ui.product";
pub const DEBUG_MONO_FAMILY: &str = "ambition.ui.debug";

/// Strong handles keep each selected font face available.
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
        app.add_systems(PostUpdate, bind_ui_font_families
            .after(bevy::text::load_font_assets_into_font_collection)
            .before(bevy::text::detect_text_needs_rerender)
            .before(bevy::text::Text2dUpdateSystems)
            .before(bevy::ui::UiSystems::Content));

        // Keep system font files loaded; see `ShelterFontSources`.
        app.init_resource::<ShelteredFontSources>()
            .init_schedule(ShelterFontSources)
            .add_systems(ShelterFontSources, shelter_font_sources)
            .add_systems(bevy::prelude::First, return_font_sources);
        if let Some(mut order) = app
            .world_mut()
            .get_resource_mut::<bevy::app::MainScheduleOrder>()
        {
            order.insert_after(bevy::prelude::PostUpdate, ShelterFontSources);
        }
    }
}

/// Runs between `PostUpdate` and `Last`, to hide the loaded system font files
/// from Bevy's `trim_source_cache`.
///
/// Bevy 0.19 unloads a system font file that no text used for two frames, and
/// loads it again on the next use. A reloaded file is a new font to Bevy's
/// glyph cache: [`bevy::text::FontAtlasKey`] contains the file's blob id, and
/// each load gets a fresh id. So a text that falls back to a system font and
/// changes a few times a second gets a new 512 px atlas image per change, and
/// [`bevy::text::FontAtlasSet`] never drops one. Measured on a Pixel 5: one
/// atlas about every frame, 3% of all samples spent filling them, until the
/// process ran out of memory.
///
/// Bundled fonts are not affected: they are loaded from memory and keep one
/// id. Only a build with `bevy/system_font_discovery` has system fonts, and
/// Android is the one composition that asks for it.
///
/// The trim is a private system in `Last`, so it cannot be ordered against or
/// removed. Instead the cache is taken out after the last text layout
/// (`PostUpdate`) and put back in [`First`]. The trim sees an empty cache. The
/// cost is that each system font file a text used stays mapped for the run.
#[derive(bevy::ecs::schedule::ScheduleLabel, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShelterFontSources;

/// The font source cache while it is out of [`bevy::text::FontCx`].
///
/// Boxed as `Any` because the cache's type is Parley's, and this crate does
/// not depend on Parley. The downcast target is inferred from the field it is
/// assigned back to.
#[derive(Resource, Default)]
struct ShelteredFontSources(Option<Box<dyn std::any::Any + Send + Sync>>);

fn shelter_font_sources(
    font_cx: Option<ResMut<bevy::text::FontCx>>,
    mut sheltered: ResMut<ShelteredFontSources>,
) {
    if let Some(mut font_cx) = font_cx {
        sheltered.0 = Some(Box::new(std::mem::take(&mut font_cx.source_cache)));
    }
}

fn return_font_sources(
    font_cx: Option<ResMut<bevy::text::FontCx>>,
    mut sheltered: ResMut<ShelteredFontSources>,
) {
    let (Some(mut font_cx), Some(sources)) = (font_cx, sheltered.0.take()) else {
        return;
    };
    if let Ok(sources) = sources.downcast() {
        font_cx.source_cache = *sources;
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
             Generate the selected library fonts with scripts/grab_font_assets.py."
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

/// Bind loaded faces to the semantic families before text layout.
pub fn bind_ui_font_families(
    selected: Option<Res<UiFonts>>,
    fonts: Res<Assets<Font>>,
    mut context: ResMut<bevy::text::FontCx>,
    mut seen: Local<Vec<(bevy::asset::AssetId<Font>, u64, &'static str)>>,
    mut text: Query<&mut TextFont>,
) {
    let Some(selected) = selected else { return; };
    let mut current = Vec::new();
    for (handle, family) in [(&selected.regular, PRODUCT_FAMILY), (&selected.semibold, PRODUCT_FAMILY), (&selected.mono, DEBUG_MONO_FAMILY)] {
        if let Some((handle, font)) = handle.as_ref().and_then(|handle| fonts.get(handle).map(|font| (handle, font))) {
            let entry = (handle.id(), font.data.id(), family);
            if !current.contains(&entry) { current.push(entry); }
        }
    }
    let missing = current.iter().any(|(_, _, family)| context.collection.family_id(family).is_none());
    if *seen == current && !missing { return; }
    for alias in [PRODUCT_FAMILY, DEBUG_MONO_FAMILY] {
        if let Some(family) = context.collection.family_by_name(alias) {
            for face in family.fonts() {
                context.collection.unregister_font(family.id(), face.width(), face.style(), face.weight());
            }
        }
    }
    for (handle, _, family) in &current {
        let font = fonts.get(*handle).expect("a selected face is loaded");
        context.collection.register_fonts(font.data.clone(), Some(fontique::FontInfoOverride {
            family_name: Some(family), ..Default::default()
        }));
    }
    *seen = current;
    for mut font in &mut text {
        if matches!(&font.font, FontSource::Family(family) if family == PRODUCT_FAMILY || family == DEBUG_MONO_FAMILY) {
            font.set_changed();
        }
    }
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

    /// One system font family, and the blob id each use of it resolved to.
    #[derive(Resource, Default)]
    struct SystemFontProbe {
        family: String,
        frame: u32,
        ids: Vec<u64>,
    }

    /// Use the family on every fourth frame, as a slowly changing text does.
    /// The gap is longer than the two frames Bevy's trim allows.
    fn probe_system_font(
        mut font_cx: ResMut<bevy::text::FontCx>,
        mut probe: ResMut<SystemFontProbe>,
    ) {
        probe.frame += 1;
        if probe.frame % 4 != 1 {
            return;
        }
        let context = &mut font_cx.context;
        let Some(family) = context.collection.family_by_name(&probe.family) else {
            return;
        };
        let blob = family
            .fonts()
            .first()
            .and_then(|font| context.source_cache.get(font.source()));
        if let Some(blob) = blob {
            probe.ids.push(blob.id());
        }
    }

    /// The ids three spaced uses of one system font resolve to, or `None`
    /// when the build discovers no system fonts.
    fn system_font_ids(sheltered: bool) -> Option<Vec<u64>> {
        let mut app = App::new();
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::text::TextPlugin,
        ));
        if sheltered {
            app.add_plugins(UiFontsPlugin);
        }
        // Chosen before the first frame, so only system families are registered.
        let family = {
            let mut font_cx = app.world_mut().resource_mut::<bevy::text::FontCx>();
            let context = &mut font_cx.context;
            let names: Vec<String> = context
                .collection
                .family_names()
                .map(str::to_owned)
                .collect();
            names.into_iter().find(|name| {
                context
                    .collection
                    .family_by_name(name)
                    .is_some_and(|family| !family.fonts().is_empty())
            })?
        };
        app.insert_resource(SystemFontProbe {
            family,
            ..default()
        })
        .add_systems(PostUpdate, probe_system_font);
        for _ in 0..9 {
            app.update();
        }
        Some(app.world().resource::<SystemFontProbe>().ids.clone())
    }

    #[test]
    fn a_system_font_keeps_one_identity_between_spaced_uses() {
        let (Some(trimmed), Some(sheltered)) = (system_font_ids(false), system_font_ids(true))
        else {
            // Only the Android composition asks for `bevy/system_font_discovery`.
            // Run with `--features bevy/system_font_discovery` to exercise this.
            eprintln!("skipped: this build discovers no system fonts");
            return;
        };
        // The premise. If this fails, Bevy no longer reloads an idle system
        // font under a new id, and `ShelterFontSources` can be deleted.
        assert_eq!(trimmed.len(), 3);
        assert!(
            trimmed.windows(2).all(|pair| pair[0] != pair[1]),
            "without the shelter each spaced use should reload the file: {trimmed:?}"
        );
        assert_eq!(sheltered.len(), 3);
        assert!(
            sheltered.windows(2).all(|pair| pair[0] == pair[1]),
            "with the shelter the file must stay loaded under one id: {sheltered:?}"
        );
    }

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
