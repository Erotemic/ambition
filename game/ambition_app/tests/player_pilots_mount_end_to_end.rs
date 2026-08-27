//! (ADR 0020 §4) — player-piloting a mount works END-TO-END through the real headless sim.
//!
//! The payoff of the two-linked-actors mount model: a human drives a VEHICLE
//! through the exact same control seam that drives every other body. The rider
//! is the pilot; the mount is the physics body. When the primary seat sits on the
//! rider, the player's slot input flows through the universal brain path
//! (`SlotControls` → the rider's `ActorControl`) and `steer_mount_from_rider`
//! routes that intent onto the mount — so pressing right drives the MOUNT right,
//! with the rider welded to the saddle. This is rider-agnostic by construction:
//! the mount cannot tell an AI Skirmisher rider from a possessing human.
//!
//! This pins the loop through `Platformer2dSimHarness::step` with REAL slot input:
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

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::actors::features::FeatureId;
use ambition_platformer2d::characters::brain::Brain;
use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::mount::{MountSlot, Mounted, RidingOn};
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
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
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");

    // 1. Spawn the mount + rider near the player. Their archetypes carry the
    //    mount roles (shark → `Mountable{class:"shark"}`; pirate raider →
    //    `CanPilot(["shark"])`) via `attach_mount_role`, so an authored link
    //    resolves into a live weld.
    let home = home_entity(sim.world_mut());
    let p = pos_of(sim.world_mut(), home);
    let mount_pos = (p.x + 120.0, p.y);
    let rider_pos = (p.x + 120.0, p.y - 66.0); // ~saddle, above the mount
                                               // the mount NAMES ITS CHARACTER. `burning_flying_shark` stopped being an
                                               // archetype row — the shark authors its own body, including
                                               // that it is rideable — so a request naming only the brain key would resolve
                                               // the `combatant` fallback: not a mount, not a flyer, and this test would
                                               // watch it fall.
    sim.spawn_enemy_character_at(
        MOUNT_ID,
        "Burning Flying Shark",
        mount_pos,
        (63.0, 26.0),
        CharacterBrain::Custom("burning_flying_shark".to_string()),
        "npc_burning_flying_shark",
    );
    // The mount named its character three lines up and the pilot beside it did not, so the pair
    // under test was half migrated: the raider's `CanPilot(["shark"])` comes from the
    // character, and a generic `combatant` cannot pilot anything.
    sim.spawn_enemy_character_at(
        RIDER_ID,
        "Pirate Raider",
        rider_pos,
        (22.0, 39.0),
        CharacterBrain::Custom("pirate_raider".to_string()),
        "npc_pirate_raider",
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

    // 2. Control-seam handover: take the primary SEAT off the home avatar and
    //    place it on the RIDER (exactly what possession does; done directly
    //    here). The control invariant — exactly one body holds
    //    `DrivingParticipant(PRIMARY)` — is preserved: home loses it, the rider
    //    gains it. neither body's `Brain` is touched, which is the whole point
    //    of the seat being its own component.
    sim.world_mut()
        .entity_mut(home)
        .remove::<DrivingParticipant>()
        .insert(ActorControl::default());
    sim.world_mut()
        .entity_mut(rider)
        .insert(DrivingParticipant(PlayerSlot::PRIMARY))
        .insert(ActorControl::default());
    // Let the handover settle (ControlledSubject re-resolves to the rider).
    sim.step(AgentAction::default());

    // 3. Drive right. The MOUNT should travel: player input → rider ActorControl
    //    → steer_mount_from_rider → the mount body integrates the routed intent,
    //    while the rider welds to the saddle. The vacated home avatar (neutral
    //    control, no seat) stays put.
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
    // What this test is actually about is whether DRIVE INPUT reaches the
    // vacated avatar, and the input is rightward: any leftward travel is
    // somebody else's physics.
    assert!(
        home_after.x - home_before.x < 1.0,
        "the vacated home avatar moved RIGHT — the drive input is reaching a \
         body nobody is driving: {home_before:?} -> {home_after:?}",
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

/// The dismount brain rebuild survives the schedule, not just the builder.
///
/// ⛔ WHY THIS ARM EXISTS AT THE APP LEVEL. The rebuild used to be a direct call
/// inside `enforce_mount_rider_link`; it is now a second system answering the
/// `MountDied` that system already wrote, so mount can be carved without
/// dragging the character runtime with it. Every unit arm in the mount module
/// hand-lists its systems, so all of them keep passing if the reactor is dropped
/// from `CombatSchedulePlugin` — a hand-listed chain pins the FUNCTION, not the
/// WIRING. This drives the real headless sim and would go red on that drop.
///
/// The rider is parked on `stand_still` before the mount dies, so the brain
/// below can only have come from the rebuild.
///
/// ⭐ AND THE BRAIN IT LANDS ON IS THE RANGED ONE, which is the builder's own
/// rule rather than an incidental: a pirate raider carries a gun-sword, and a
/// rider whose held item still grants ranged keeps a ranged-capable brain so the
/// weapon stays live after the shark dies. `forced_hostile_melee_brute_brain` is
/// the OTHER branch — the mount module's unit arm covers that one.
#[test]
fn a_dead_mount_rebuilds_its_riders_brain_through_the_real_schedule() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");

    let home = home_entity(sim.world_mut());
    let p = pos_of(sim.world_mut(), home);
    sim.spawn_enemy_character_at(
        MOUNT_ID,
        "Burning Flying Shark",
        (p.x + 120.0, p.y),
        (63.0, 26.0),
        CharacterBrain::Custom("burning_flying_shark".to_string()),
        "npc_burning_flying_shark",
    );
    sim.spawn_enemy_character_at(
        RIDER_ID,
        "Pirate Raider",
        (p.x + 120.0, p.y - 66.0),
        (22.0, 39.0),
        CharacterBrain::Custom("pirate_raider".to_string()),
        "npc_pirate_raider",
    );
    let mount = entity_by_feature_id(sim.world_mut(), MOUNT_ID);
    let rider = entity_by_feature_id(sim.world_mut(), RIDER_ID);
    sim.world_mut()
        .entity_mut(mount)
        .insert(Brain::stand_still());
    sim.world_mut()
        .entity_mut(rider)
        .insert(Brain::stand_still());
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
        matches!(
            sim.world_mut().get::<Brain>(rider),
            Some(Brain::StateMachine(
                ambition_platformer2d::characters::brain::StateMachineCfg::StandStill
            ))
        ),
        "premise: the mounted rider is still parked on the brain this test gave it, \
         so a MeleeBrute below can only come from the dismount rebuild",
    );

    // Kill the mount by its entity-local state, the way `boss_lifecycle` does.
    sim.world_mut()
        .get_mut::<ambition_platformer2d::characters::actor::BodyHealth>(mount)
        .expect("the mount carries health")
        .health
        .current = 0;
    for _ in 0..2 {
        sim.step(AgentAction::default());
    }

    assert!(
        sim.world_mut().get::<Mounted>(rider).is_none(),
        "premise: the dissolution ran at all (the `Mounted` marker is gone)",
    );
    let Some(Brain::StateMachine(
        ambition_platformer2d::characters::brain::StateMachineCfg::Skirmisher { cfg, .. },
    )) = sim.world_mut().get::<Brain>(rider).cloned()
    else {
        panic!(
            "the fallen rider kept its ranged brain through the COMPOSED schedule, not just \
             through a hand-listed pair; got {:?}",
            sim.world_mut().get::<Brain>(rider),
        );
    };
    assert_eq!(
        cfg.aggressiveness, 1.0,
        "and the dismount rebuild is what makes it hostile — a peaceful rider that fell off \
         still fights",
    );
}
