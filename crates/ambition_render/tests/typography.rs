//! The measurement the semantic typography layer stands on.
//!
//! [`ambition_render::ui_fonts::UiFonts::text_font`] answers "product UI,
//! semibold" with a FAMILY plus a WEIGHT instead of a handle to a particular
//! file. That is only true if Ambition's two bundled Inter faces actually
//! register as ONE family in Parley's collection, distinguished by weight — and
//! that depends on which OpenType name record fontique reads. This file asks the
//! real font files, through Bevy's own registration system.
//!
//! ⛔ THE BYTES ARE `include_bytes!`, NOT A FILESYSTEM PROBE. A missing bundled
//! font must not turn this into a test that silently passes by skipping; it is a
//! compile error instead, which is the same contract
//! `ambition_asset_manager::platformer_assets::embedded` already relies on.

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

/// A world holding the three bundled faces, registered exactly the way the app
/// registers them: as `Font` ASSETS, swept into the collection by Bevy's own
/// system. Nothing here reaches into fontique directly, because what has to be
/// true is a fact about the path the game uses.
fn app_with_bundled_fonts() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Font>()
        .init_resource::<FontCx>()
        .add_systems(Update, load_font_assets_into_font_collection);

    // ⛔ THE HANDLES MUST BE HELD. Dropping them releases the assets before the
    // next frame, the collection is rebuilt without them, and every assertion
    // below reads an EMPTY family list — which is the same contract `UiFonts`
    // states for holding its three handles, demonstrated here by having got it
    // wrong first.
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

/// ⭐⭐ THE LOAD-BEARING FACT: two files, ONE family, TWO weights.
///
/// `InterDisplay-Regular.otf` carries family name (id 1) "Inter Display".
/// `InterDisplay-SemiBold.otf` carries family name (id 1) "Inter Display
/// SemiBold" — a DIFFERENT string — but typographic family name (id 16)
/// "Inter Display", and fontique reads id 16 first. If that preference ever
/// flips, these are two unrelated families, `FontSource::Family` can no longer
/// express "semibold", and every caller has to go back to knowing which file is
/// which. This test is what would say so.
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

/// Debug monospace is a different ROLE, and so a different family — not a weight.
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

/// ⛔ AND THE FALLBACK STILL HAS TO BE STATED, not discovered.
///
/// An unresolvable `FontSource::Family` does not error — Parley falls back — so
/// naming a family whose file never loaded would turn "no bundled font" from a
/// warned condition into invisible tofu.
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
