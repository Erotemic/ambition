//! **Choosing a starting character still starts the game.**
//!
//! `AMBITION_START_CHARACTER=<id>` (and `capture_scene --character <id>`) insert
//! a `StartingCharacterOverride`, which preparation moves onto the session root
//! as `StartingCharacter`. Until 2026-08-08 the shell's preparation barrier
//! failed `validate-provider-defaults` whenever that selection differed from the
//! provider's authored default — `retryable(false)`, before publishing anything
//! — so the session NEVER ACTIVATED. There was no world, no body and no message
//! a player sees; Jon reported it as "sanic grants the wrong verbs and loses
//! move/jump", and sanic was only the id he happened to type.
//!
//! ⚠ **the identical check had already been deleted from
//! `prepare_platformer_content` on 2026-07-29**, whose own commit says
//! `--character` "had never worked for any id". It stayed broken because the
//! corrected copy runs at `PREPARE_SESSION`, downstream of the barrier's early
//! return — one question asked at two sites, and the site that answered first
//! was the one nobody changed. `a_starting_character_other_than_the_default_prepares`
//! (in `ambition_platformer2d_provider`) covers the pure function; only a
//! composed App reaches the barrier, which is why this test is here.
//!
//! Sanic is deliberately the id under test: its row is owned by ANOTHER provider
//! (`ambition_demo_sanic`), so this also states that a linked provider's
//! character is wearable in the Ambition launcher host — the composition fact the
//! catalog comment promises.

#![cfg(feature = "rl_sim")]

use ambition_app::{
    AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::actors::actor::{BodyKinematics, PrimaryPlayerOnly};
use ambition_platformer2d::characters::actor::WornCharacter;
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
    // ⚠ before the fix the failure was NOT here — preparation reported a failed
    // load work item and the harness came up worldless, so the first `step`
    // panicked on a missing session instead.
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
