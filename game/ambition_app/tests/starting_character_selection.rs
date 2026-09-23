//! Choosing a starting character still starts the game.
//!
//! `AMBITION_START_CHARACTER=<id>` (and `capture_scene --character <id>`) insert a
//! `StartingCharacterOverride`, which preparation moves onto the session root as
//! `StartingCharacter`.
//!
//! It stayed broken because the corrected copy runs at `PREPARE_SESSION`, downstream of the
//! barrier's early return — one question asked at two sites, and the site that answered first was
//! the one nobody changed. `a_starting_character_other_than_the_default_prepares` (in
//! `ambition_platformer2d_provider`) covers the pure function; only a composed App reaches the
//! barrier, which is why this test is here.
//!
//! Sanic is deliberately the id under test: its row is owned by ANOTHER provider
//! (`ambition_demo_sanic`), so this also states that a linked provider's
//! character is wearable in the Ambition launcher host — the composition fact the
//! catalog comment promises.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{
    AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::World;

const SELECTED: &str = "sanic";

fn body_pos(world: &mut World) -> bevy::prelude::Vec2 {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player").pos
}

#[test]
fn a_selected_starting_character_activates_a_controllable_session() {
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions {
            timestep: TimestepMode::fixed_60hz(),
            ..Default::default()
        },
        |app, options| {
            // Exactly what `cli.rs::insert_starting_character_override` does for
            // `AMBITION_START_CHARACTER`, and `capture_scene` for `--character`.
            app.insert_resource(ambition_app::app::StartingCharacterOverride(
                ambition_platformer2d::actors::avatar::StartingCharacter::new(SELECTED),
            ));
            ambition_app::rl_sim::ambition_sim_composition(app, options)
        },
    )
    .expect("the sandbox sim builds with a selected starting character");

    // A live session exists at all (this reads the session-root `RoomSet`).
    sim.step(AgentAction::default());

    let worn = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&WornCharacter, PrimaryPlayerOnly>();
        q.single(world).expect("primary player").id().to_string()
    };
    assert_eq!(
        worn, SELECTED,
        "the activated session's body wears the SELECTED character, not the \
         provider's authored default"
    );

    // Settle, then prove the body answers the stick. "Can't move" was half of
    // the report; the honest form of it is displacement through the real
    // schedule, not the presence of an ability flag.
    for _ in 0..60 {
        sim.step(AgentAction::default());
    }
    let before = body_pos(sim.world_mut());
    for _ in 0..60 {
        sim.step(AgentAction::move_x(1.0));
    }
    let after = body_pos(sim.world_mut());
    assert!(
        after.x - before.x > 50.0,
        "the selected persona's body integrates rightward input: x {} -> {}",
        before.x,
        after.x,
    );
}

/// With NO selection the body still wears a character — the content default —
/// and it is built as that character, authored kit included. The unselected
/// start used to take a host-code road that built the generic player kit while
/// `PersonaBaseline` told the persona derive the default's kit was already
/// applied, so nothing ever put the authored one on.
#[test]
fn an_unselected_start_is_built_with_the_default_characters_authored_kit() {
    let mut sim = Platformer2dSimHarness::new_with_options(Platformer2dSimHarnessOptions {
        timestep: TimestepMode::fixed_60hz(),
        ..Default::default()
    })
    .expect("the sandbox sim builds");
    sim.step(AgentAction::default());

    let world = sim.world_mut();
    let (worn, identity) = {
        let mut q = world.query_filtered::<(
            &WornCharacter,
            &ambition_platformer2d::characters::brain::action_set::IdentityKit,
        ), PrimaryPlayerOnly>();
        let (worn, identity) = q.single(world).expect("primary player");
        (worn.id().to_string(), identity.clone())
    };
    let registry = world
        .resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>();
    let prepared = registry
        .get(&worn)
        .unwrap_or_else(|| panic!("the default character `{worn}` is not in the prepared cast"));
    // The MOVESET, not the action set: the default's authored action set is
    // the host kit's on purpose, so only its moves tell the two roads apart.
    let authored = prepared.kit.projectable_moveset().unwrap_or_else(|| {
        panic!("the premise: the default character `{worn}` authors its own moveset")
    });
    assert_eq!(
        &identity.moveset, authored,
        "the unselected start wears `{worn}` but was built with moves that are not \
         its authored ones",
    );
}
