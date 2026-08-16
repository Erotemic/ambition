//! **The engine draws its effects from art the ENGINE ships.**
//!
//! ⛔⛔ the defect this closes: `spawn_effect` reached for its art through
//! `GameAssets.characters.props` — *a map keyed by the LDtk `Prop.kind` field* —
//! and the only things that ever wrote to it were GAME systems. Ambition's intro
//! prop table listed `generic_explosions` beside a cart and a piano; nothing else
//! in the workspace listed any FX sheet at all. So in Smash, Sanic and Mary-O
//! every effect degraded to one particle burst, forever, while 189 authored
//! effect rows sat on disk unreachable. An engine that draws an asset has to be
//! able to ship it.
//!
//! ⚠ **asserted on the composed SHIPPED host**, not a hand-built App: "the
//! engine ships it" is a claim about the composition, and a fixture that
//! inserted the sheets itself would prove nothing about what a player gets.

use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::render::fx::resolve_drawable;
use ambition_platformer2d::sprite_sheet::fx::FX_SHEETS;
use ambition_platformer2d::sprite_sheet::game_assets::GameAssets;
use ambition_platformer2d::vfx::FxId;
use bevy::prelude::*;

/// Boot far enough for the engine's `Startup` asset binding to have run.
fn booted() -> App {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.update();
    app
}

/// **AN AUTHORED EFFECT DRAWS FROM A REAL SHEET, AND NO GAME REGISTERED IT.**
///
/// The assertion runs through `resolve_drawable`, the exact decision
/// `spawn_effect` makes: `Some` is art, `None` is the particle fallback. Asking
/// the engine rather than re-deriving its lookup is what keeps this from
/// drifting away from what the renderer does.
///
/// ⚠ **the non-vacuity guard is the first assertion, and it is exact.** The one
/// registration that ever existed was Ambition's intro prop row for
/// `generic_explosions`; that row is deleted, so `characters.props` no longer
/// carries it in this very host. Absent-there and drawable-here together say
/// the ENGINE is the one shipping the art — and a world where no assets loaded
/// at all fails the second half rather than passing the first.
#[test]
fn effects_draw_from_engine_shipped_sheets_with_no_game_registering_them() {
    let app = booted();
    let assets = app
        .world()
        .get_resource::<GameAssets>()
        .expect("the composed host binds game assets");

    assert!(
        assets.characters.props.get("generic_explosions").is_none(),
        "NON-VACUITY: the intro's LDtk-prop registration of the explosion sheet \
         is gone. If this is Some again, some game is registering FX art as a \
         level prop and the assertions below stopped measuring the engine."
    );

    assert_eq!(
        assets.fx.len(),
        FX_SHEETS.len(),
        "every declared FX sheet decoded: {:?}",
        assets.fx.targets()
    );

    // One from the sheet the old five lived on, one from a sheet nothing could
    // reach before, one from a per-character effect sheet.
    for (name, sheet) in [
        ("classic_burst", "generic_explosions"),
        ("sonic_boom", "generic_exotic_fx"),
        ("reductio_impact", "george_booul_vfx"),
    ] {
        let (effect, _asset, slot) = resolve_drawable(Some(assets), FxId::new(name))
            .unwrap_or_else(|| panic!("`{name}` must resolve to sheet art, not a particle burst"));
        assert_eq!(effect.name, name);
        assert_eq!(effect.sheet, sheet);
        assert_eq!(
            effect.slot, slot,
            "the slot the LOADED spec resolves must be the slot the baked index named"
        );
    }

    // …and an id no sheet carries still falls back, so the check above is a
    // property of the name rather than of `resolve_drawable` answering `Some`.
    assert!(resolve_drawable(Some(assets), FxId::new("kaboom")).is_none());
}
