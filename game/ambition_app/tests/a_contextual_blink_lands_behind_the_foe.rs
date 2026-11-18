//! Contextual Blink, worn from the inventory, takes the Blink slot: a press
//! puts the body behind the nearest foe, facing it.
//!
//! The press runs in `technique_execution` (the combat phase), after the
//! readers of the travelled path. The move is held on the body
//! (`PendingTransit`) and made in the next tick's carry, so the body has not
//! moved on the tick of the press and has arrived on the next. Under a GGRS
//! sync-test session: a rewind across the held move that lost it, or made it
//! twice, desyncs the checksum or leaves the body short or past.

#![cfg(feature = "rl_sim")]

use crate::common::base;
use ambition_app::rl_sim::{Platformer2dSimHarness, TimestepMode};
use ambition_app::AmbitionSim;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::items::{Item, ItemEquipRequested};
use bevy::prelude::{Entity, With};

const FOE: &str = "contextual_blink_foe";

fn primary(sim: &mut Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    let mut query = world.query_filtered::<Entity, With<ambition_platformer2d::platformer::body::PrimaryBody>>();
    query.single(world).expect("one primary body")
}

fn body(sim: &Platformer2dSimHarness, entity: Entity) -> BodyKinematics {
    *sim.world().get::<BodyKinematics>(entity).expect("a live body")
}

#[test]
fn a_contextual_blink_lands_behind_the_foe_a_tick_after_the_press() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::sandbox_options()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("the sandbox builds under a GGRS sync-test session");
    sim.step_n(base(), 30);
    let me = primary(&mut sim);
    ambition_platformer2d::platformer::host_intents::write_host_intent(
        sim.world_mut(),
        ItemEquipRequested { participant: ambition_platformer2d::characters::control::PlayerSlot(0), item: Item::ContextualBlink },
    );
    sim.step_n(base(), 3);

    let start = body(&sim, me);
    sim.spawn_enemy_character_at(
        FOE,
        "Perfect Cellular Automaton",
        (start.pos.x + 110.0, start.pos.y),
        (14.0, 23.0),
        CharacterBrain::Passive,
        "perfect_cellular_automaton",
    );
    sim.step_n(base(), 45);
    let foe = {
        let world = sim.world_mut();
        let mut query = world.query::<(Entity, &FeatureId)>();
        query.iter(world).find(|(_, id)| id.as_str() == FOE).map(|(entity, _)| entity).expect("the foe was built")
    };
    let (before, them) = (body(&sim, me), body(&sim, foe));
    let ahead = (them.pos.x - before.pos.x).signum();
    assert!(
        before.pos.distance(them.pos) < 300.0,
        "the foe stands within the worn level's reach: {:?} and {:?}",
        before.pos,
        them.pos
    );

    sim.step(ambition_app::AgentAction { blink: true, blink_held: true, ..base() });
    let pressed = body(&sim, me);
    assert!(
        (pressed.pos.x - before.pos.x).abs() < 8.0,
        "the body has not moved on the tick of the press: {:?} -> {:?}",
        before.pos,
        pressed.pos
    );
    assert!(
        sim.world().get::<ambition_platformer2d::abilities::pending_transit::PendingTransit>(me).is_some(),
        "the move is held on the body for the next tick"
    );

    sim.step(ambition_app::AgentAction { blink_released: true, ..base() });
    let (arrived, them) = (body(&sim, me), body(&sim, foe));
    let past = (arrived.pos.x - them.pos.x) * ahead;
    let clear = them.half_extent().x + arrived.half_extent().x;
    assert!(
        past >= clear && past < clear + 12.0,
        "the body is behind the foe, clear of it: {past} past its centre (boxes {clear}); {:?} -> {:?}, foe at {:?}",
        before.pos,
        arrived.pos,
        them.pos
    );
    assert_eq!(arrived.facing.signum(), -ahead, "and turned to face it");
    assert!(
        sim.world().get::<ambition_platformer2d::abilities::pending_transit::PendingTransit>(me).is_none(),
        "the held move is spent"
    );
    // Rewound and replayed: no second blink, no desync.
    sim.step_n(base(), 20);
    assert!(
        ((body(&sim, me).pos.x - them.pos.x) * ahead).abs() < clear + 40.0,
        "a replayed tick did not blink the body again"
    );
    assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
}
