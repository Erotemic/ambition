//! THE PLAYER IS BUILT WHOLE: its prepared character body lands at
//! construction, not on its first tick.
//!
//! A per-update sample cannot tell the two apart — the re-template pass that
//! used to grant it runs inside the same update — so this probe looks at the
//! first SIM TICK the player exists, at `PlayerInputSet::Device`, which is
//! before `CharacterProjection` could have granted anything.

use ambition_demo_mary_o_app::build_demo_app;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::platformer::schedule::{PlayerInputSet, SimScheduleExt};
use ambition_platformer2d::sprite_sheet::character::SpritePosedBody;
use bevy::prelude::*;

/// What the player carried at the first sim tick it existed.
#[derive(Resource, Default)]
struct FirstTick(Option<(bool, ae::Vec2)>);

fn probe(
    mut seen: ResMut<FirstTick>,
    player: Query<(Option<&SpritePosedBody>, &ae::BodyBaseSize), With<PrimaryPlayer>>,
) {
    if seen.0.is_some() {
        return;
    }
    if let Ok((posed, base)) = player.single() {
        let posed = posed.is_some();
        seen.0 = Some((posed, base.base_size));
    }
}

#[test]
fn the_players_prepared_body_is_on_it_before_its_first_tick_projects_anything() {
    let mut app = build_demo_app();
    app.init_resource::<FirstTick>();
    let sim = app.sim_schedule();
    app.add_systems(sim, probe.in_set(PlayerInputSet::Device));
    for _ in 0..600 {
        app.update();
        if app.world().resource::<FirstTick>().0.is_some() {
            break;
        }
    }
    let (posed, base) = app
        .world()
        .resource::<FirstTick>()
        .0
        .expect("the demo never ran a sim tick with a playable body");
    assert!(
        posed,
        "the player reached its first sim tick without its posed body: the \
         prepared-body grant is still arriving from the first tick's projection"
    );
    let default_base = ae::BodyBaseSize::default().base_size;
    assert_ne!(
        base, default_base,
        "the player reached its first sim tick in the engine's default box rather \
         than its character's standing box — something re-answered the body's \
         size after construction (the developer body profile once did, by \
         treating the untouched selector as an edit)",
    );
}
