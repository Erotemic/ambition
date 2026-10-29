//! Possession redirects participant seats through the real headless simulation.
//! The target keeps its authored policy. Release returns the seat to its home.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::actor::ActorFaction;
use ambition_platformer2d::actors::control::possession::PossessionState;
use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::{Entity, World};

const ACTOR_ID: &str = "possess_target";

fn player_pos(world: &mut World) -> ae::Vec2 {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player").pos
}

fn actor_entity(world: &mut World) -> Entity {
    let mut q = world.query::<(Entity, &FeatureId)>();
    q.iter(world)
        .find(|(_, f)| f.as_str() == ACTOR_ID)
        .map(|(e, _)| e)
        .expect("the spawned actor is present")
}

fn possessed(sim: &mut Platformer2dSimHarness) -> Option<Entity> {
    sim.world_mut()
        .resource::<PossessionState>()
        .get(ambition_platformer2d::characters::control::PlayerSlot::PRIMARY)
        .possessed
}

fn faction(world: &mut World, e: Entity) -> ActorFaction {
    *world.get::<ActorFaction>(e).expect("actor faction")
}

/// Possess the actor 60 px to the player's right, returning its entity.
/// Shared setup for the possession tests below.
///
/// The target is a Smash-duelist fighter that plays neutral-game FOOTSIES — it weaves in and
/// out around its attack_range (≈ the 150 px possession radius), so on any single 2 s commit
/// frame it may be spaced just out of reach. This oscillation crossing the radius knife-edge is
/// what the ranged subsumption (E54) nudged us onto — the mechanic itself is unchanged.
fn spawn_and_possess(sim: &mut Platformer2dSimHarness) -> Entity {
    let p = player_pos(sim.world_mut());
    // ⛔ SPAWN BY CHARACTER, NOT BY ARCHETYPE. A `Custom(..)` row that no longer
    // exists falls back to a generic `combatant`, so the fixture keeps passing
    // while asserting on a body that is not the one it names.

    sim.spawn_enemy_character_at(
        ACTOR_ID,
        "Perfect Cellular Automaton",
        (p.x + 60.0, p.y),
        (14.0, 23.0),
        CharacterBrain::Custom("cellular_automaton_fighter".to_string()),
        "perfect_cellular_automaton",
    );
    let actor = actor_entity(sim.world_mut());
    for i in 0..900 {
        sim.step(down_interact(i == 0));
        if possessed(sim).is_some() {
            break;
        }
    }
    assert_eq!(
        possessed(sim),
        Some(actor),
        "setup: holding Down+Interact should possess the actor within a few commit windows"
    );
    settle_out_of_any_move(sim, actor);
    actor
}

/// Step until the possessed body is out of whatever move it was playing.
///
/// ⛔⛔ POSSESSION LANDS ON WHATEVER STEP THE RADIUS IS CROSSED ON, and the
/// automaton is a fighter — so it may be mid-move when the seat changes hands.
/// On 2026-08-25 a ranged-cadence change shifted that step and possession began
/// landing inside `generation_wipe`, a long rooted special that publishes NO
/// locomotion: one test then read `locomotion.x = 0` on the frame it pressed
/// right, and another drove for thirty steps and travelled half a pixel.
///
/// ⭐ NEITHER TEST IS ABOUT MOVES. One is a SCHEDULE invariant (same-frame slot
/// input), the other is "a possessed body integrates like every other body" —
/// and a rooted body is a state in which neither can show. Measured: with no
/// move playing, the pressed frame reads `locomotion.x = 1.0` and the drive
/// travels.
fn settle_out_of_any_move(sim: &mut Platformer2dSimHarness, actor: Entity) {
    let playing = |sim: &mut Platformer2dSimHarness| {
        sim.world_mut()
            .get::<ambition_platformer2d::combat::moveset::MovePlayback>(actor)
            .is_some()
    };
    for _ in 0..240 {
        if !playing(sim) {
            break;
        }
        sim.step(AgentAction::default());
    }
    assert!(
        !playing(sim),
        "setup: the possessed actor never came out of the move possession landed \
         inside, so every measurement after this is of a rooted body"
    );
}

/// SAME-FRAME slot input (schedule invariant): a possessed actor ticks inside
/// `WorldPrep`, which runs BEFORE `PlayerInput`. `SlotControls` +
/// `ControlledSubject` must be published even earlier (before `WorldPrep`), so a
/// SINGLE `move_x` step must show up in the possessed body's `ActorControl` THIS
/// frame — not next frame. Before the schedule fix this read last frame's input.
#[test]
fn possessed_actor_reads_this_frame_slot_input() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let actor = spawn_and_possess(&mut sim);

    // The possess gesture drove `move_y` (down); horizontal was zero, so the
    // actor's control x is ~0 going into this step. ONE step of move_x(1.0):
    sim.step(AgentAction::move_x(1.0));

    let control = sim
        .world_mut()
        .get::<ActorControl>(actor)
        .expect("possessed actor carries ActorControl")
        .0;
    assert!(
        control.locomotion.x > 0.5,
        "the possessed actor's ActorControl must reflect THIS frame's move_x \
         (same-frame slot input); got locomotion.x = {} — the WorldPrep actor tick \
         read a stale SlotControls",
        control.locomotion.x,
    );
}

/// Attack authority follows the primary seat, not the home body.
#[test]
fn attack_while_possessing_starts_the_possessed_actors_melee_not_the_home() {
    use ambition_platformer2d::combat::hitbox::Hitbox;
    use ambition_platformer2d::combat::moveset::{ActorMoveset, MovePlayback, melee_swing_of};

    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let home = {
        let mut q = sim
            .world_mut()
            .query_filtered::<Entity, PrimaryPlayerOnly>();
        q.single(sim.world_mut()).expect("primary player").clone()
    };
    let actor = spawn_and_possess(&mut sim);

    // Robust to catch-up; still proves "attack started this body's melee".
    let melee_engaged = |sim: &mut Platformer2dSimHarness, e: Entity| {
        let world = sim.world_mut();
        melee_swing_of(world.get::<MovePlayback>(e), world.get::<ActorMoveset>(e)).is_some()
    };

    // Hold Attack across a window. The possessed actor holds the primary seat, so its
    // `melee_pressed` edge starts its `"attack"` moveset move (the ONE body melee lifecycle:
    // `trigger_moveset_moves` → `advance_move_playback`) and, at the active window, spawns a
    // strike it OWNS. The vacated home avatar has no player brain, so its melee never engages
    // and it owns no strike.
    let mut actor_engaged = false;
    let mut home_engaged = false;
    let mut actor_owns_strike = false;
    let mut home_owns_strike = false;
    for _ in 0..30 {
        sim.step(AgentAction {
            attack: true,
            ..AgentAction::default()
        });
        actor_engaged |= melee_engaged(&mut sim, actor);
        home_engaged |= melee_engaged(&mut sim, home);
        let mut q = sim.world_mut().query::<&Hitbox>();
        for hb in q.iter(sim.world_mut()) {
            if hb.owner == actor {
                actor_owns_strike = true;
            }
            if hb.owner == home {
                home_owns_strike = true;
            }
        }
    }

    assert!(
        actor_engaged,
        "the POSSESSED actor's melee lifecycle engaged on Attack"
    );
    assert!(
        !home_engaged,
        "the vacated home avatar's melee did NOT engage — attack authority is the \
         body holding the primary seat, not the home body"
    );
    assert!(
        actor_owns_strike,
        "the possessed actor's swing spawned a strike hitbox OWNED by the actor",
    );
    assert!(
        !home_owns_strike,
        "the vacated home avatar spawned no strike",
    );
}

/// Hold Down (`move_y > 0.35`) + Interact — the possession gesture. The HOLD
/// accumulates on `interact_held` (the real binding is `pressed`, i.e. held);
/// the single-frame `interact` edge fires only when `edge` is set (frame one of
/// a press), exactly as the device pipeline reports a real button hold.
fn down_interact(edge: bool) -> AgentAction {
    AgentAction {
        move_y: 1.0,
        interact: edge,
        interact_held: true,
        ..AgentAction::default()
    }
}

#[test]
fn a_player_can_possess_drive_and_release_an_actor_end_to_end() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");

    // Drop a normal actor one short stride from the player — inside POSSESS_RADIUS
    // (150px). Same known-good melee archetype the enemy-attacks test uses.
    let p = player_pos(sim.world_mut());
    // ⛔ SPAWN BY CHARACTER, NOT BY ARCHETYPE. A `Custom(..)` row that no longer
    // exists falls back to a generic `combatant`, so the fixture keeps passing
    // while asserting on a body that is not the one it names.

    sim.spawn_enemy_character_at(
        ACTOR_ID,
        "Perfect Cellular Automaton",
        (p.x + 60.0, p.y),
        (14.0, 23.0),
        CharacterBrain::Custom("cellular_automaton_fighter".to_string()),
        "perfect_cellular_automaton",
    );
    let actor = actor_entity(sim.world_mut());
    assert_eq!(
        faction(sim.world_mut(), actor),
        ActorFaction::Enemy,
        "the actor starts on its own (Enemy) faction"
    );

    // 1. Hold Down+Interact until the possession commits. The fighter footsies in
    //    and out of the 150px radius, so hold across several commit windows (see
    //    `spawn_and_possess`); the sim is deterministic, so this lands.
    for i in 0..900 {
        sim.step(down_interact(i == 0));
        if possessed(&mut sim).is_some() {
            break;
        }
    }
    assert_eq!(
        possessed(&mut sim),
        Some(actor),
        "holding Down+Interact next to the actor possesses it"
    );
    assert_eq!(
        faction(sim.world_mut(), actor),
        ActorFaction::Enemy,
        "possession does NOT mutate the authored faction — effective allegiance \
         (holding the primary seat) is what makes combat treat it as player-aligned"
    );

    settle_out_of_any_move(&mut sim, actor);
    // 2. Drive right. The POSSESSED body should move — it now integrates through
    //    the SAME unified `integrate_sim_bodies` phase every body uses. The vacated
    //    home avatar stays put because it holds no seat (its
    //    `ActorControl` is neutral), NOT because of any movement run-condition gate.
    let player_before = player_pos(sim.world_mut());
    let actor_before = sim.world_mut().get::<BodyKinematics>(actor).unwrap().pos;
    // A short burst — long enough to clearly travel, short enough to stay on the
    // platform (driven far enough at the body's own run speed it would walk off a
    // ledge and despawn OOB, which is realistic but not what this test isolates).
    for _ in 0..30 {
        sim.step(AgentAction::move_x(1.0));
    }
    let player_after = player_pos(sim.world_mut());
    let actor_after = sim.world_mut().get::<BodyKinematics>(actor).unwrap().pos;

    assert!(
        actor_after.x - actor_before.x > 20.0,
        "the possessed body moves right under player input: {actor_before:?} -> {actor_after:?}"
    );
    // It now is. That is the guarantee HOLDING, reported as a violation by a proxy that could not
    // tell the two apart.
    assert!(
        player_after.x - player_before.x < 1.0,
        "the player's OWN body ran RIGHT with the drive input while possessing, so \
         one input drove two bodies: {player_before:?} -> {player_after:?}"
    );

    // 3. A fresh Down+Interact press releases possession. `prev_down_interact` is
    //    false after the move phase, so this frame is the rising edge.
    sim.step(down_interact(true));
    assert_eq!(
        possessed(&mut sim),
        None,
        "a fresh Down+Interact press releases possession"
    );
    assert_eq!(
        faction(sim.world_mut(), actor),
        ActorFaction::Enemy,
        "on release the actor reverts to its original faction (its own brain again)"
    );
}

/// A possessed body goes through the door, and arrives.
///
/// The two halves were only ever inferred to compose.
#[test]
fn a_possessed_body_is_carried_through_a_room_transition() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    let actor = spawn_and_possess(&mut sim);

    let before_room = sim.observation().active_room.clone();
    // Stand the POSSESSED body in the door — not the home avatar, which is the
    // whole distinction.
    let door_centre = {
        let world = sim.world_mut();
        let live_definition = ambition_platformer2d::world::rooms::sole_live_room_definition(world)
            .expect("the session has a live room");
        let mut rooms = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let Some(zone) = rooms.iter(world).next().and_then(|set| {
            set.spec(live_definition)
                .loading_zones
                .iter()
                .find(|zone| {
                    zone.activation
                        == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door
                })
                .cloned()
        }) else {
            panic!(
                "the start room '{before_room}' authors no Door zone, so this test \
                 measures nothing — point it at a room that has one"
            );
        };
        zone.aabb.center()
    };
    {
        let world = sim.world_mut();
        let mut kin = world
            .get_mut::<BodyKinematics>(actor)
            .expect("the possessed body has kinematics");
        kin.pos = door_centre;
        kin.vel = ae::Vec2::ZERO;
    }

    let mut arrived = None;
    // The door's passage (0.7s) plays before the room changes, and the
    // possessed body comes out of the far door before it is asked about.
    for _ in 0..120 {
        sim.step(AgentAction {
            interact: true,
            interact_held: true,
            ..AgentAction::default()
        });
        if sim.observation().active_room != before_room {
            arrived = Some(sim.observation().active_room.clone());
            crate::common::finish_passages(&mut sim);
            break;
        }
    }
    let after_room = arrived.unwrap_or_else(|| {
        panic!(
            "holding interact inside the door of '{before_room}' while possessing \
             never changed the room; the possessed body cannot use a door at all"
        )
    });

    assert_eq!(
        possessed(&mut sim),
        Some(actor),
        "the room changed but control did not survive the crossing; a body that \
         arrives without its driver is a different bug from one that does not \
         arrive, and both would pass a room-id assertion alone"
    );
    let world = sim.world_mut();
    let Some(kin) = world.get::<BodyKinematics>(actor) else {
        panic!(
            "the possessed body does not exist after arriving in '{after_room}' — \
             a transition that despawns the body it is driving is worse than one \
             that leaves it behind"
        );
    };
    // a LARGE displacement, not merely a nonzero one. The body falls a few
    // pixels under gravity across the commit window, so `> 0` would pass on a
    // transition that carried nothing. An arrival is a jump into the target
    // room's coordinates — this one crosses ~2000 px into `vertical_shaft`.
    assert!(
        kin.pos.distance(door_centre) > 200.0,
        "the room changed to '{after_room}' but the possessed body is still within \
         200 px of the door it left ({:?}); `carry_body` did not carry the driven \
         body, which is exactly the branch the home-avatar path never exercises",
        kin.pos
    );
}

fn possess_two_seats(sprite_content: &ambition_platformer2d::sprite_sheet::SpriteContent, rollback: bool) -> (Platformer2dSimHarness, [Entity; 2], [Entity; 2], u64) {
    use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
    use ambition_platformer2d::platformer::lifecycle::InCustodyOf;
    let mut options = ambition_app::rl_sim::Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_fixed_tick(true)
        .with_required_start_room("switch_lab");
    if rollback {
        options = options
            .with_sync_test_rollback_settings(4, 10)
            .with_rollback_players(2);
    }
    let mut sim =
        Platformer2dSimHarness::new_with_options(options).expect("two-seat simulation builds");
    let home_a = {
        let world = sim.world_mut();
        world
            .query_filtered::<Entity, PrimaryPlayerOnly>()
            .single(world)
            .unwrap()
    };
    let home_b =
        crate::a_home_body_is_built_for_a_seat::build_a_home_body(&sprite_content, &mut sim, PlayerSlot(1), ());
    let start = player_pos(sim.world_mut());
    sim.world_mut()
        .get_mut::<BodyKinematics>(home_b)
        .unwrap()
        .pos = start + ae::Vec2::new(400.0, 0.0);
    for (id, offset) in [("seat_a_target", 30.0), ("seat_b_target", 430.0)] {
        sim.spawn_enemy_character_at(
            id,
            id,
            (start.x + offset, start.y),
            (14.0, 23.0),
            CharacterBrain::Passive,
            "perfect_cellular_automaton",
        );
    }
    let targets = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &FeatureId)>();
        ["seat_a_target", "seat_b_target"].map(|id| {
            q.iter(world)
                .find(|(_, feature)| feature.as_str() == id)
                .unwrap()
                .0
        })
    };
    for target in targets {
        sim.world_mut()
            .entity_mut(target)
            .insert(ambition_platformer2d::characters::brain::Brain::stand_still());
    }
    sim.rebase_rollback_history()
        .expect("setup enters the rollback baseline");
    let loads_before = sim
        .rollback_execution_stats()
        .map_or(0, |stats| stats.lifetime_load_runs);
    let gesture = ae::ControlFrame {
        axis_y: 1.0,
        interact_held: true,
        ..Default::default()
    };
    for _ in 0..150 {
        sim.drive_seat(1, gesture);
        sim.step_frame(gesture);
    }
    for (slot, target, home) in [
        (PlayerSlot::PRIMARY, targets[0], home_a),
        (PlayerSlot(1), targets[1], home_b),
    ] {
        assert_eq!(
            sim.world()
                .resource::<PossessionState>()
                .get(slot)
                .possessed,
            Some(target),
            "seat {slot:?}"
        );
        assert_eq!(
            sim.world().get::<DrivingParticipant>(target).unwrap().0,
            slot
        );
        assert!(sim.world().get::<DrivingParticipant>(home).is_none());
        assert_eq!(
            sim.world().get::<InCustodyOf>(target).unwrap().custodian,
            home
        );
    }
    (sim, [home_a, home_b], targets, loads_before)
}

fn exercise_two_seat_handoffs(sprite_content: &ambition_platformer2d::sprite_sheet::SpriteContent, rollback: bool) {
    use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
    let (mut sim, [home_a, home_b], targets, loads_before) = possess_two_seats(&sprite_content, rollback);
    let gesture = ae::ControlFrame {
        axis_y: 1.0,
        interact_held: true,
        ..Default::default()
    };
    let before = targets.map(|body| sim.world().get::<BodyKinematics>(body).unwrap().pos.x);
    for _ in 0..20 {
        sim.drive_seat(
            1,
            ae::ControlFrame {
                axis_x: -1.0,
                ..Default::default()
            },
        );
        sim.step_frame(ae::ControlFrame {
            axis_x: 1.0,
            ..Default::default()
        });
    }
    assert!(sim.world().get::<BodyKinematics>(targets[0]).unwrap().pos.x > before[0]);
    assert!(sim.world().get::<BodyKinematics>(targets[1]).unwrap().pos.x < before[1]);
    sim.drive_seat(1, ae::ControlFrame::default());
    sim.step_frame(gesture);
    assert_eq!(
        sim.world().get::<DrivingParticipant>(home_a).unwrap().0,
        PlayerSlot::PRIMARY
    );
    assert!(sim.world().get::<DrivingParticipant>(targets[0]).is_none());
    assert_eq!(
        sim.world().get::<DrivingParticipant>(targets[1]).unwrap().0,
        PlayerSlot(1)
    );
    sim.drive_seat(1, gesture);
    sim.step_frame(ae::ControlFrame::default());
    for _ in 0..12 {
        sim.drive_seat(1, ae::ControlFrame::default());
        sim.step_frame(ae::ControlFrame::default());
    }
    assert_eq!(
        sim.world().get::<DrivingParticipant>(home_b).unwrap().0,
        PlayerSlot(1)
    );
    assert!(sim.world().get::<DrivingParticipant>(targets[1]).is_none());
    assert!(!sim.world().resource::<PossessionState>().any_possessed());
    if rollback {
        assert!(sim.rollback_execution_stats().unwrap().lifetime_load_runs > loads_before);
        sim.rollback_health()
            .expect("both handoffs remain valid under resimulation");
    }
}

#[test]
fn two_participants_possess_drive_and_release_independently() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
    exercise_two_seat_handoffs(&sprite_content, false);
}

#[test]
fn rollback_preserves_two_participants_possession_handoffs() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
    exercise_two_seat_handoffs(&sprite_content, true);
}

#[test]
fn a_removed_possessed_body_returns_only_its_participants_seat() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
    use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
    let (mut sim, homes, targets, _) = possess_two_seats(&sprite_content, false);
    assert!(sim.world_mut().despawn(targets[0]));
    let other_before = sim.world().get::<BodyKinematics>(targets[1]).unwrap().pos.x;
    for _ in 0..3 {
        sim.drive_seat(
            1,
            ae::ControlFrame {
                axis_x: -1.0,
                ..Default::default()
            },
        );
        sim.step_frame(ae::ControlFrame {
            axis_x: 1.0,
            ..Default::default()
        });
    }
    let state = sim.world().resource::<PossessionState>();
    assert!(state.get(PlayerSlot::PRIMARY).possessed.is_none());
    assert!(state.get(PlayerSlot::PRIMARY).home.is_none());
    assert_eq!(state.get(PlayerSlot(1)).possessed, Some(targets[1]));
    assert_eq!(
        sim.world().get::<DrivingParticipant>(homes[0]).unwrap().0,
        PlayerSlot::PRIMARY
    );
    assert!(sim.world().get::<DrivingParticipant>(homes[1]).is_none());
    assert_eq!(
        sim.world().get::<DrivingParticipant>(targets[1]).unwrap().0,
        PlayerSlot(1)
    );
    assert_eq!(
        sim.world()
            .resource::<ambition_platformer2d::platformer::markers::ControlledSubject>()
            .0,
        Some(homes[0])
    );
    assert!(
        sim.world()
            .get::<ActorControl>(homes[0])
            .unwrap()
            .0
            .locomotion
            .x
            > 0.5,
        "the returned seat did not accept current input"
    );
    assert!(
        sim.world().get::<BodyKinematics>(targets[1]).unwrap().pos.x < other_before,
        "the other participant stopped driving its possessed body"
    );
}

#[test]
fn releasing_a_seat_keeps_the_active_move_on_its_original_body() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
    use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
    use ambition_platformer2d::combat::moveset::MovePlayback;
    let (mut sim, homes, targets, _) = possess_two_seats(&sprite_content, false);
    sim.drive_seat(1, ae::ControlFrame::default());
    sim.step_frame(ae::ControlFrame::default());
    sim.step_frame(ae::ControlFrame {
        axis_x: 1.0,
        attack_pressed: true,
        attack_held: true,
        ..Default::default()
    });
    let playing = sim
        .world()
        .get::<MovePlayback>(targets[0])
        .expect("the participant started its body's authored attack");
    let occurrence = playing.instance;
    let definition = playing.spec.clone();
    let elapsed = playing.t;
    sim.step_frame(ae::ControlFrame {
        axis_y: 1.0,
        interact_held: true,
        ..Default::default()
    });
    let playing = sim
        .world()
        .get::<MovePlayback>(targets[0])
        .expect("release must not cancel the body's active move");
    assert_eq!(playing.instance, occurrence);
    assert!(std::sync::Arc::ptr_eq(&playing.spec, &definition));
    assert!(playing.t >= elapsed, "release restarted the move timeline");
    assert!(
        sim.world().get::<MovePlayback>(homes[0]).is_none(),
        "the active move followed the seat into its home body"
    );
    assert_eq!(
        sim.world().get::<DrivingParticipant>(homes[0]).unwrap().0,
        PlayerSlot::PRIMARY
    );
    assert!(sim.world().get::<DrivingParticipant>(targets[0]).is_none());
    assert_eq!(
        sim.world().get::<DrivingParticipant>(targets[1]).unwrap().0,
        PlayerSlot(1)
    );
}

#[test]
fn possession_handoffs_do_not_advance_a_free_body_twice() {
    use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
    let (mut sim, homes, targets, _) = possess_two_seats(&ambition_platformer2d::sprite_sheet::test_support::shared_sprites(), false);
    let bodies = [homes[0], homes[1], targets[0], targets[1]];
    let start = player_pos(sim.world_mut());
    let neutral = ae::ControlFrame::default();

    // Release includes a discrete transit. Measure free motion after that transit.
    for releasing in [None, Some(PlayerSlot::PRIMARY), Some(PlayerSlot(1))] {
        if let Some(slot) = releasing {
            let release = ae::ControlFrame { axis_y: 1.0, interact_held: true, ..neutral };
            sim.drive_seat(1, if slot == PlayerSlot(1) { release } else { neutral });
            sim.step_frame(if slot == PlayerSlot::PRIMARY { release } else { neutral });
            assert_eq!(sim.world().get::<DrivingParticipant>(homes[slot.0 as usize]).unwrap().0, slot);
            assert!(sim.world().get::<DrivingParticipant>(targets[slot.0 as usize]).is_none());
        }
        let before: [f32; 4] = std::array::from_fn(|index| {
            let body = bodies[index];
            let mut kin = sim.world_mut().get_mut::<BodyKinematics>(body).unwrap();
            kin.pos = start + ae::Vec2::new(100.0 * (index + 1) as f32, -150.0);
            kin.vel = ae::Vec2::new(120.0, 0.0);
            let x = kin.pos.x;
            sim.world_mut().get_mut::<ae::BodyGroundState>(body).unwrap().invalidate();
            x
        });
        sim.drive_seat(1, neutral);
        sim.step_frame(neutral);
        let dt = sim.world().resource::<ambition_platformer2d::time::WorldTime>().sim_dt();
        assert!(dt > 0.0);
        for (body, x) in bodies.into_iter().zip(before) {
            let kin = sim.world().get::<BodyKinematics>(body).unwrap();
            let displacement = kin.pos.x - x;
            assert!(displacement.abs() > 0.01, "body {body:?} did not move: kin={kin:?}, dt={dt}, releasing={releasing:?}");
            assert!((displacement - kin.vel.x * dt).abs() < 0.001,
                "body {body:?} did not advance by one tick: displacement={displacement}, velocity={}, dt={dt}, releasing={releasing:?}", kin.vel.x);
        }
    }
}
