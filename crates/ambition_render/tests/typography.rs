//! The measurement the semantic typography layer stands on.
//!
//! [`ambition_render::ui_fonts::UiFonts::text_font`] answers "product UI,
//! semibold" with a family plus a weight, not a handle to a file. That works
//! only if the two bundled Inter faces register as one family in Parley's
//! collection, told apart by weight. That depends on which OpenType name
//! record fontique reads. These tests check the real font files through
//! Bevy's own registration system.
//!
//! The bytes use `include_bytes!`, not a filesystem probe, so a missing
//! bundled font is a compile error, not a silent skip. Same contract as
//! `ambition_asset_manager::platformer_assets::embedded`.

use bevy::asset::Assets;
use bevy::prelude::*;
use bevy::text::{
    load_font_assets_into_font_collection, Font, FontCx, FontSource, FontStyle, FontWeight,
    FontWidth, TextFont,
};

use ambition_render::ui_fonts::{
    UiFontWeight, UiFonts, DEBUG_MONO_FAMILY, PRODUCT_FAMILY,
};

const INTER_REGULAR: &[u8] = include_bytes!(
    "../../ambition_platformer2d_actor_monolith/assets/fonts/bundled/InterDisplay-Regular.otf"
);
const INTER_SEMIBOLD: &[u8] = include_bytes!(
    "../../ambition_platformer2d_actor_monolith/assets/fonts/bundled/InterDisplay-SemiBold.otf"
);
const JETBRAINS_MONO: &[u8] = include_bytes!(
    "../../ambition_platformer2d_actor_monolith/assets/fonts/bundled/JetBrainsMono-Regular.ttf"
);

/// A world with the three bundled faces, registered like the app does: as
/// `Font` assets, added to the collection by Bevy's own system. Nothing here
/// calls fontique directly, because the fact must hold on the game's path.
fn app_with_bundled_fonts() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Font>()
        .init_resource::<FontCx>()
        .add_systems(Update, load_font_assets_into_font_collection);

    // Hold the handles. If they drop, the assets are released before the
    // next frame, the collection is rebuilt without them, and every assertion
    // below sees an empty family list. `UiFonts` holds its handles for the
    // same reason.
    let mut held = Vec::new();
    for bytes in [INTER_REGULAR, INTER_SEMIBOLD, JETBRAINS_MONO] {
        let font = Font::from_bytes(bytes.to_vec());
        held.push(app.world_mut().resource_mut::<Assets<Font>>().add(font));
    }
    app.insert_resource(HeldFonts(held));
    app.update();
    assert_eq!(
        app.world().resource::<Assets<Font>>().len(),
        3,
        "the three bundled faces must be live assets"
    );
    app
}

/// Keeps the loaded faces alive for the duration of a test.
#[derive(Resource)]
struct HeldFonts(#[allow(dead_code)] Vec<Handle<Font>>);

/// Two files, one family, two weights.
///
/// `InterDisplay-Regular.otf` has family name (id 1) "Inter Display".
/// `InterDisplay-SemiBold.otf` has family name (id 1) "Inter Display
/// SemiBold", but typographic family name (id 16) "Inter Display", and
/// fontique reads id 16 first. If that preference changes, these become two
/// unrelated families and `FontSource::Family` cannot express "semibold".
#[test]
fn the_two_bundled_inter_faces_are_one_family_with_two_weights() {
    let mut app = app_with_bundled_fonts();
    let mut font_cx = app.world_mut().resource_mut::<FontCx>();

    let family = font_cx
        .collection
        .family_by_name(PRODUCT_FAMILY)
        .unwrap_or_else(|| {
            panic!(
                "the bundled Inter faces did not register under {PRODUCT_FAMILY:?}; \
                 families present: {:?}",
                font_cx.collection.family_names().collect::<Vec<_>>()
            )
        });
    assert_eq!(
        family.fonts().len(),
        2,
        "both Inter faces must land in the one family, or the weight cannot choose between them"
    );

    let pick = |weight: FontWeight| {
        family
            .match_index(
                FontWidth::NORMAL.into(),
                FontStyle::Normal.into(),
                weight.into(),
                false,
            )
            .expect("a family with faces always matches something")
    };
    assert_ne!(
        pick(FontWeight::NORMAL),
        pick(FontWeight::SEMIBOLD),
        "asking for semibold picked the same FACE as regular — the weight is being \
         ignored, so `UiFonts::text_font` is not actually selecting a typeface"
    );
}

/// Debug monospace is a different role, so a different family, not a weight.
#[test]
fn debug_monospace_is_its_own_family() {
    let mut app = app_with_bundled_fonts();
    let mut font_cx = app.world_mut().resource_mut::<FontCx>();
    assert!(
        font_cx.collection.family_by_name(DEBUG_MONO_FAMILY).is_some(),
        "families present: {:?}",
        font_cx.collection.family_names().collect::<Vec<_>>()
    );
    assert_ne!(PRODUCT_FAMILY, DEBUG_MONO_FAMILY);
}

/// With nothing loaded, no family is named.
///
/// An unresolvable `FontSource::Family` falls back silently, so naming a
/// family whose file never loaded would turn a warned condition into
/// invisible tofu.
#[test]
fn nothing_loaded_names_no_family() {
    let fonts = UiFonts::default();
    assert_eq!(
        fonts.text_font(14.0, UiFontWeight::Semibold).font,
        FontSource::default()
    );
    assert!(!fonts.has_dialog_font());
    assert_eq!(fonts.selected_marker(), ">");
}

/// The request a caller makes is semantic all the way down.
#[test]
fn a_caller_names_a_role_and_never_a_file() {
    let fonts = UiFonts {
        regular: Some(Handle::default()),
        semibold: Some(Handle::default()),
        mono: Some(Handle::default()),
    };
    let semibold: TextFont = fonts.text_font(18.0, UiFontWeight::Semibold);
    assert_eq!(semibold.font, FontSource::Family(PRODUCT_FAMILY.into()));
    assert_eq!(semibold.weight, FontWeight::SEMIBOLD);
}
