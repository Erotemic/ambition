//! ONE melee lifecycle for every body — the convergence proof for the melee
//! unification. There is no player melee driver and no actor melee driver, and no
//! flat melee state machine at all: melee is a `"attack"`-verb MOVESET move for
//! EVERY body — the human player, a possessed actor, an autonomous hostile — run
//! through the real schedule:
//!
//!   `melee_pressed` (control edge, any brain)
//!     → `combat::moveset::trigger_moveset_moves`   (starts the `"attack"` move)
//!     → `combat::moveset::advance_move_playback`   (spawns the active-window
//!        strike + slash, owned by the body, on the owner's proper-time clock)
//!     → `project_moveset_melee_to_body_melee`      (`BodyMelee` read-model)
//!
//! This pins, through `SandboxSim::step`, that BOTH the player and an autonomous
//! hostile actor enter that identical melee lifecycle and own the strike their
//! swing spawns. The possessed-actor case is pinned by
//! `possession_end_to_end.rs`; the peaceful-NPC-with-kit case is the same path
//! gated by its ActionSet (capability) + brain (policy) — a peaceful brain never
//! presses attack, but a possessing human drives the identical lifecycle.

#![cfg(feature = "rl_sim")]

use ambition::actors::actor::BodyMelee;
use ambition::actors::actor::{BodyKinematics, PrimaryPlayerOnly};
use ambition::actors::combat::components::{ActorDisposition, ActorTarget};
use ambition::actors::combat::moveset::MovePlayback;
use ambition::actors::features::{FeatureId, Hitbox};
use ambition::characters::brain::{ActionSet, ActorControl};
use ambition::entity_catalog::{placements::CharacterBrain, WindowTag};
use ambition_app::{AgentAction, SandboxSim, TimestepMode};
use bevy::prelude::{Entity, World};
use std::sync::Mutex;

static UNIFIED_MELEE_TEST_LOCK: Mutex<()> = Mutex::new(());

fn player_entity(world: &mut World) -> Entity {
    let mut q = world.query_filtered::<Entity, PrimaryPlayerOnly>();
    q.single(world).expect("primary player")
}

fn player_pos(world: &mut World) -> ambition::engine_core::Vec2 {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player").pos
}

/// A body's melee lifecycle engaged: mid-swing, or its recovery cooldown is armed
/// (a swing began this window — robust to fixed-timestep catch-up completing a
/// short swing within one `sim.step`).
fn melee_engaged(world: &mut World, e: Entity) -> bool {
    world
        .get::<BodyMelee>(e)
        .map(|m| m.is_swinging() || m.cooldown > 0.0)
        .unwrap_or(false)
}

fn owns_a_strike(world: &mut World, e: Entity) -> bool {
    let mut q = world.query::<&Hitbox>();
    q.iter(world).any(|hb| hb.owner == e)
}

fn is_attack_move(playback: &MovePlayback) -> bool {
    let id = playback.spec.id.as_str();
    id == "attack" || id.starts_with("attack_")
}

fn attack_move_is_active(playback: &MovePlayback) -> bool {
    is_attack_move(playback)
        && playback.spec.windows.iter().any(|w| {
            matches!(w.tag, WindowTag::Active) && playback.t >= w.start_s && playback.t < w.end_s
        })
}

#[derive(Default, Debug)]
struct HostileMeleeTally {
    present_frames: usize,
    hostile_frames: usize,
    target_some_frames: usize,
    action_set_has_melee: bool,
    melee_pressed_frames: usize,
    engaged_frames: usize,
    attack_playback_frames: usize,
    active_attack_frames: usize,
    owns_strike_frames: usize,
    min_dist: f32,
}

fn observe_hostile_melee(
    world: &mut World,
    feature_id: &str,
    player: ambition::engine_core::Vec2,
    t: &mut HostileMeleeTally,
) {
    let observed = {
        let mut q = world.query::<(
            Entity,
            &FeatureId,
            &BodyKinematics,
            &ActorControl,
            &ActorDisposition,
            &ActorTarget,
            &ActionSet,
            &BodyMelee,
            Option<&MovePlayback>,
        )>();
        q.iter(world)
            .find(|(_, f, ..)| f.as_str() == feature_id)
            .map(
                |(entity, _, kin, control, disp, target, actions, melee, playback)| {
                    let attack_playback = playback.map(is_attack_move).unwrap_or(false);
                    let active_attack = playback.map(attack_move_is_active).unwrap_or(false);
                    (
                        entity,
                        kin.pos,
                        control.0.melee_pressed,
                        disp.is_hostile(),
                        target.entity.is_some(),
                        actions.melee.is_some(),
                        melee.is_swinging() || melee.cooldown > 0.0 || attack_playback,
                        attack_playback,
                        active_attack,
                    )
                },
            )
    };
    let Some((
        entity,
        pos,
        melee_pressed,
        hostile,
        has_target,
        has_melee,
        engaged,
        attack_playback,
        active_attack,
    )) = observed
    else {
        return;
    };

    t.present_frames += 1;
    if hostile {
        t.hostile_frames += 1;
    }
    if has_target {
        t.target_some_frames += 1;
    }
    t.action_set_has_melee |= has_melee;
    if melee_pressed {
        t.melee_pressed_frames += 1;
    }
    if engaged {
        t.engaged_frames += 1;
    }
    if attack_playback {
        t.attack_playback_frames += 1;
    }
    if active_attack {
        t.active_attack_frames += 1;
    }
    let owns_strike = {
        let mut q = world.query::<&Hitbox>();
        q.iter(world).any(|hb| hb.owner == entity)
    };
    if owns_strike {
        t.owns_strike_frames += 1;
    }
    let d = (pos - player).length();
    if t.present_frames == 1 || d < t.min_dist {
        t.min_dist = d;
    }
}

fn hostile_body_present(world: &mut World, feature_id: &str) -> bool {
    let mut q = world.query::<(&FeatureId, &BodyMelee)>();
    q.iter(world).any(|(f, _)| f.as_str() == feature_id)
}

/// The PLAYER's own melee flows through the moveset lifecycle (no flat melee
/// driver exists): pressing Attack starts its `"attack"` move, projects a
/// `BodyMelee` swing, and spawns a strike it OWNS.
#[test]
fn the_player_enters_the_body_melee_lifecycle_and_owns_its_strike() {
    let _guard = UNIFIED_MELEE_TEST_LOCK
        .lock()
        .expect("unified melee test lock");
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    let player = player_entity(sim.world_mut());

    let mut engaged = false;
    let mut owns_strike = false;
    for _ in 0..30 {
        sim.step(AgentAction {
            attack: true,
            ..AgentAction::default()
        });
        engaged |= melee_engaged(sim.world_mut(), player);
        owns_strike |= owns_a_strike(sim.world_mut(), player);
    }

    assert!(
        engaged,
        "the player's BodyMelee lifecycle engages on Attack (via trigger_moveset_moves)"
    );
    assert!(
        owns_strike,
        "the player's swing spawns a strike hitbox it OWNS (via advance_move_playback)"
    );
}

/// An autonomous hostile actor enters the SAME `BodyMelee` lifecycle from the SAME
/// `ActorActionMessage::Melee` path — no separate actor melee driver.
#[test]
fn a_hostile_actor_enters_the_same_body_melee_lifecycle() {
    let _guard = UNIFIED_MELEE_TEST_LOCK
        .lock()
        .expect("unified melee test lock");
    const ENEMY_ID: &str = "test_aggressor";
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    let p = player_pos(sim.world_mut());
    sim.spawn_enemy_at(
        ENEMY_ID,
        "Perfect Cellular Automaton",
        (p.x + 60.0, p.y),
        (14.0, 23.0),
        CharacterBrain::Custom("cellular_automaton_fighter".to_string()),
    );
    assert!(
        hostile_body_present(sim.world_mut(), ENEMY_ID),
        "spawned enemy body present"
    );

    // Stand still; the in-range hostile fighter commits swings on its own. Observe
    // the same production components pinned by `enemy_attacks_player`: target
    // acquisition, ActionSet availability, brain-published melee intent, then the
    // body-owned `BodyMelee`/hitbox lifecycle. This keeps the test about the real
    // unified body path instead of about which support entity a narrow query happens
    // to see first.
    let mut t = HostileMeleeTally::default();
    for _ in 0..240 {
        sim.step(AgentAction::default());
        let p = player_pos(sim.world_mut());
        observe_hostile_melee(sim.world_mut(), ENEMY_ID, p, &mut t);
    }

    println!("unified melee hostile tally: {t:#?}");
    assert!(t.present_frames > 100, "enemy should persist: {t:#?}");
    assert!(
        t.hostile_frames == t.present_frames,
        "the hostile actor must stay hostile while targeting the player: {t:#?}"
    );
    assert!(
        t.target_some_frames > 0,
        "the hostile actor must acquire a target before it can commit melee: {t:#?}"
    );
    assert!(
        t.action_set_has_melee,
        "the hostile actor's ActionSet must carry a melee slot: {t:#?}"
    );
    assert!(
        t.melee_pressed_frames > 0,
        "the hostile actor brain must publish a melee press: {t:#?}"
    );
    assert!(
        t.engaged_frames > 0,
        "the hostile actor must enter the shared melee lifecycle (BodyMelee projection \
         or attack MovePlayback): {t:#?}"
    );
    assert!(
        t.owns_strike_frames > 0 || t.active_attack_frames > 0,
        "the hostile actor's attack must reach an owned strike window (hitbox observed \
         or active attack MovePlayback): {t:#?}"
    );
}
