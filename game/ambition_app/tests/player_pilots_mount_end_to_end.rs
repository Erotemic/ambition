//! M5 (ADR 0020 §4) — **player-piloting a mount works END-TO-END through the
//! real headless sim.**
//!
//! The payoff of the two-linked-actors mount model: a human drives a VEHICLE
//! through the exact same control seam that drives every other body. The rider
//! is the pilot; the mount is the physics body. When `Brain::Player` sits on the
//! rider, the player's slot input flows through the universal brain path
//! (`SlotControls` → the rider's `ActorControl`) and `steer_mount_from_rider`
//! routes that intent onto the mount — so pressing right drives the MOUNT right,
//! with the rider welded to the saddle. This is rider-agnostic by construction:
//! the mount cannot tell an AI Skirmisher rider from a possessing human.
//!
//! This pins the loop through `SandboxSim::step` with REAL slot input:
//! 1. Spawn a shark mount + a pirate rider and weld them (`RidingOn` +
//!    `Mounted` + `MountSlot` — the exact components the planned
//!    `ambition.mount` relation wiring installs for an authored pair; welded
//!    directly here because this pair is runtime-spawned, not room-authored).
//! 2. Transfer the player brain onto the rider (the control-seam handover
//!    possession performs — here done directly so the test isolates the
//!    piloting invariant, not the 2 s possess gesture, which
//!    `possession_end_to_end.rs` already pins).
//! 3. Drive `move_x`: the MOUNT travels under player input while the vacated
//!    home avatar stays put.

#![cfg(feature = "rl_sim")]

use ambition::actors::actor::{BodyKinematics, PrimaryPlayerOnly};
use ambition::actors::features::FeatureId;
use ambition::actors::features::{MountSlot, Mounted, RidingOn};
use ambition::characters::brain::{ActorControl, Brain, PlayerSlot};
use ambition::engine_core as ae;
use ambition::entity_catalog::placements::CharacterBrain;
use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, SandboxSim, TimestepMode};
use bevy::prelude::{Entity, World};

const MOUNT_ID: &str = "pilot_shark";
const RIDER_ID: &str = "pilot_rider";

fn entity_by_feature_id(world: &mut World, id: &str) -> Entity {
    let mut q = world.query::<(Entity, &FeatureId)>();
    q.iter(world)
        .find(|(_, f)| f.as_str() == id)
        .map(|(e, _)| e)
        .unwrap_or_else(|| panic!("entity with FeatureId {id} is present"))
}

fn home_entity(world: &mut World) -> Entity {
    let mut q = world.query_filtered::<Entity, PrimaryPlayerOnly>();
    q.single(world).expect("primary player")
}

fn pos_of(world: &mut World, e: Entity) -> ae::Vec2 {
    world.get::<BodyKinematics>(e).expect("body kinematics").pos
}

#[test]
fn a_player_pilots_a_mount_end_to_end() {
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");

    // 1. Spawn the mount + rider near the player. Their archetypes carry the
    //    mount roles (shark → `Mountable{class:"shark"}`; pirate raider →
    //    `CanPilot(["shark"])`) via `attach_mount_role`, so an authored link
    //    resolves into a live weld.
    let home = home_entity(sim.world_mut());
    let p = pos_of(sim.world_mut(), home);
    let mount_pos = (p.x + 120.0, p.y);
    let rider_pos = (p.x + 120.0, p.y - 66.0); // ~saddle, above the mount
    sim.spawn_enemy_at(
        MOUNT_ID,
        "Burning Flying Shark",
        mount_pos,
        (63.0, 26.0),
        CharacterBrain::Custom("burning_flying_shark".to_string()),
    );
    sim.spawn_enemy_at(
        RIDER_ID,
        "Pirate Raider",
        rider_pos,
        (22.0, 39.0),
        CharacterBrain::Custom("pirate_raider".to_string()),
    );
    let mount = entity_by_feature_id(sim.world_mut(), MOUNT_ID);
    let rider = entity_by_feature_id(sim.world_mut(), RIDER_ID);

    // Neutralize both AI brains so the pair stays put while the link resolves —
    // this test isolates PILOTING, not the pair's autonomous approach.
    sim.world_mut()
        .entity_mut(mount)
        .insert(Brain::stand_still());
    sim.world_mut()
        .entity_mut(rider)
        .insert(Brain::stand_still());

    // Weld the pair — both ends, the same components the planned
    // `ambition.mount` relation wiring installs for a room-authored pair. (The
    // frame-later `PendingMountLinks` resolver this test used to exercise is
    // deleted; authored links are planned relations now, and this runtime pair
    // is welded directly so the test keeps isolating PILOTING.)
    sim.world_mut()
        .entity_mut(rider)
        .insert((RidingOn { mount }, Mounted));
    sim.world_mut()
        .entity_mut(mount)
        .insert(MountSlot { rider: Some(rider) });
    for _ in 0..4 {
        sim.step(AgentAction::default());
    }
    assert!(
        sim.world_mut().get::<RidingOn>(rider).is_some()
            && sim.world_mut().get::<Mounted>(rider).is_some(),
        "the weld holds through live frames (enforce_mount_rider_link keeps it)",
    );

    // 2. Control-seam handover: vacate the home avatar's player brain and place
    //    it on the RIDER (exactly what possession does; done directly here). The
    //    control invariant — exactly one body carries `Brain::Player(PRIMARY)` —
    //    is preserved: home loses it, the rider gains it.
    sim.world_mut()
        .entity_mut(home)
        .remove::<Brain>()
        .insert(ActorControl::default());
    sim.world_mut()
        .entity_mut(rider)
        .insert(Brain::Player(PlayerSlot::PRIMARY))
        .insert(ActorControl::default());
    // Let the handover settle (ControlledSubject re-resolves to the rider).
    sim.step(AgentAction::default());

    // 3. Drive right. The MOUNT should travel: player input → rider ActorControl
    //    → steer_mount_from_rider → the mount body integrates the routed intent,
    //    while the rider welds to the saddle. The vacated home avatar (neutral
    //    control, no player brain) stays put.
    let mount_before = pos_of(sim.world_mut(), mount);
    let rider_before = pos_of(sim.world_mut(), rider);
    let home_before = pos_of(sim.world_mut(), home);
    for _ in 0..40 {
        sim.step(AgentAction::move_x(1.0));
    }
    let mount_after = pos_of(sim.world_mut(), mount);
    let home_after = pos_of(sim.world_mut(), home);
    let rider_after = pos_of(sim.world_mut(), rider);

    assert!(
        mount_after.x - mount_before.x > 20.0,
        "the MOUNT travels right under player input (piloting through the control seam): \
         {mount_before:?} -> {mount_after:?}",
    );
    assert!(
        (home_after.x - home_before.x).abs() < 1.0,
        "the vacated home avatar does NOT respond to the drive input: \
         {home_before:?} -> {home_after:?}",
    );
    // The rider RODE ALONG (its own locomotion is suppressed while mounted; it
    // moves only because the mount carried it) and stays welded to the saddle —
    // the authored `rider_offset` is (0, -66): directly above the mount, x-aligned.
    assert!(
        rider_after.x - rider_before.x > 20.0,
        "the player rider rides along with the mount it pilots: {rider_before:?} -> {rider_after:?}",
    );
    assert!(
        (rider_after.x - mount_after.x).abs() < 12.0 && rider_after.y < mount_after.y,
        "the player rider stays welded above the mount at the saddle offset: \
         rider {rider_after:?} vs mount {mount_after:?}",
    );
}
