//! The engine draws its effects from art the ENGINE ships.
//!
//! Ambition's intro prop table listed `generic_explosions` beside a cart and a piano; nothing else
//! in the workspace listed any FX sheet at all. So in Smash, Sanic and Mary-O every effect degraded
//! to one particle burst, forever, while 189 authored effect rows sat on disk unreachable. An
//! engine that draws an asset has to be able to ship it.
//!
//! asserted on the composed SHIPPED host, not a hand-built App: "the
//! engine ships it" is a claim about the composition, and a fixture that
//! inserted the sheets itself would prove nothing about what a player gets.

use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::actors::character_runtime::CharacterLoadDemand;
use ambition_platformer2d::render::fx::resolve_drawable;
use ambition_platformer2d::sprite_sheet::fx::{core_fx_targets, FX_SHEETS};
use ambition_platformer2d::sprite_sheet::game_assets::GameAssets;
use ambition_platformer2d::vfx::FxId;
use bevy::prelude::*;

/// Boot far enough for the engine's `Startup` asset binding to have run.
fn booted() -> App {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.update();
    app
}

/// AN AUTHORED EFFECT DRAWS FROM A REAL SHEET, AND NO GAME REGISTERED IT.
///
/// The assertion runs through `resolve_drawable`, the exact decision
/// `spawn_effect` makes: `Some` is art, `None` is the particle fallback. Asking
/// the engine rather than re-deriving its lookup is what keeps this from
/// drifting away from what the renderer does.
///
/// Absent-there and drawable-here together say the ENGINE is the one shipping the art — and a
/// world where no assets loaded at all fails the second half rather than passing the first.
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

    // The CORE set at boot — the generic vocabulary and projectile art. The
    // character-owned sheets follow their character (below); before 2026-09-02
    // all thirteen decoded here and sat resident in every room.
    let mut core: Vec<&str> = core_fx_targets().collect();
    core.sort_unstable();
    assert_eq!(
        assets.fx.targets(),
        core,
        "the core FX sheets decoded at boot, and only those"
    );
    assert!(
        FX_SHEETS.len() > assets.fx.len(),
        "NON-VACUITY: some sheets are character-owned, or the seam below tests nothing"
    );

    for (name, sheet) in [
        ("classic_burst", "generic_explosions"),
        ("sonic_boom", "generic_exotic_fx"),
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

/// A CHARACTER-OWNED SHEET DECODES WHEN ITS CHARACTER IS REALIZED, NOT BEFORE.
///
/// `cell_birth` is the Perfect Cellular Automaton's up-tilt effect, on
/// `pca_vfx`: a particle burst at boot (nobody who fires it exists), sheet art
/// once the engine's demand drain realizes the fighter — the same road a room
/// transition or a spawn takes. Driven on the shipped host for the same reason
/// as above: the seam is in `materialize_character_demand`, and a fixture that
/// loaded the sheet itself would prove nothing about who owes it.
#[test]
fn a_characters_own_effect_sheet_follows_its_realization() {
    let mut app = booted();
    let fx = |app: &App| {
        let assets = app.world().resource::<GameAssets>();
        (
            assets.fx.contains("pca_vfx"),
            resolve_drawable(Some(assets), FxId::new("cell_birth")).is_some(),
        )
    };
    assert_eq!(fx(&app), (false, false), "premise: nobody at boot fires a PCA effect");

    app.world_mut()
        .resource_mut::<CharacterLoadDemand>()
        .request("perfect_cellular_automaton");
    // The drain realizes one character per frame at most and the sheet's
    // handle exists the frame the realization lands; the decode itself is
    // asynchronous and the reveal barrier (not this test) waits on it.
    for _ in 0..8 {
        app.update();
        if fx(&app).0 {
            break;
        }
    }
    assert_eq!(
        fx(&app),
        (true, true),
        "realizing the fighter demanded `pca_vfx`, and its effect resolves to sheet art"
    );
}
