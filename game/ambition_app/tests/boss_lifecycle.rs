//! Boss lifecycle CONTRACT pins — the headless safety net for Stage R3 of the
//! boss entity-local refactor (`docs/systems/boss-encounter-architecture.md`).
//!
//! R3 flips boss HP/phase authority off the global `BossEncounterRegistry` onto
//! the entity and deletes the live map. The *consequences* of a boss dying —
//! the save records it Cleared, its reward chest drops, and the adaptive-music
//! request restores to room music — have no other headless coverage and can't
//! be eyeballed in CI. These tests pin that observable contract against the
//! CURRENT (registry) implementation so R3 must preserve it.
//!
//! Stable parts: the ASSERTIONS (music set during the fight; save Cleared +
//! reward chest + music cleared after death). Authority-coupled part: the
//! `force_kill_boss` helper drives the current registry authority — R3 repoints
//! it to the entity (`BossEncounter.health`/`encounter.phase`), and the assertions
//! must still pass. The cut-rope victory NPC + in-place replay are content-
//! specific + headless-hard (R5 rewrites cut-rope as an EncounterScript); they
//! remain an explicit in-game verification item, but cut-rope's death
//! consequences share the generic entity-death path pinned here.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::actors::actor::{BodyKinematics, PrimaryPlayerOnly};
use ambition_platformer2d::boss_encounter::BossOverrides;
use ambition_platformer2d::boss_encounter::{BossConfig, BossEncounter};
use ambition_platformer2d::boss_encounter::{
    BossEncounterPhase, EncounterBeat, EncounterDef, EncounterEffect, EncounterGate,
    EncounterScript, EncounterTrigger,
};
use ambition_platformer2d::actors::features::{BossRewardChest};
use ambition_platformer2d::combat::events::{ResetRoomFeaturesEvent, RoomResetReason};
use ambition_platformer2d::encounter::EncounterMusicRequest;
use ambition_platformer2d::entity_catalog::placements::BossBrain;
use ambition_platformer2d::persistence::save::AmbitionGameSave;
use ambition_platformer2d::persistence::save_data::PersistedEncounterState;
use bevy::prelude::World;

const MOCKINGBIRD_TRACK: &str = "how_to_kill_a_mockingbird";

fn player_pos(world: &mut World) -> (f32, f32) {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    let kin = q.single(world).expect("primary player exists");
    (kin.pos.x, kin.pos.y)
}

fn spawn_mockingbird(sim: &mut Platformer2dSimHarness, runtime_id: &str) {
    let (px, py) = player_pos(sim.world_mut());
    sim.spawn_boss_at(
        runtime_id,
        "mockingbird",
        (px, py),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
    );
}

/// Drive the boss to death by mutating its ENTITY-LOCAL state (R3: the entity is
/// the source of truth). `update_boss_encounters` then runs the death outro +
/// records the consequences the tests assert.
fn force_kill_boss(sim: &mut Platformer2dSimHarness, runtime_id: &str) {
    let world = sim.world_mut();
    let mut q = world.query::<(
        &BossConfig,
        &mut BossEncounter,
        &mut ambition_platformer2d::characters::actor::BodyHealth,
    )>();
    for (config, mut status, mut health) in q.iter_mut(world) {
        if config.id == runtime_id {
            health.health.current = 0;
            if let Some(phase) = status.encounter.as_mut() {
                let _ = phase.kill();
            }
            return;
        }
    }
    panic!("boss {runtime_id} not found");
}

fn music_track(sim: &Platformer2dSimHarness) -> Option<String> {
    ambition_platformer2d::platformer::lifecycle::session_world_component::<EncounterMusicRequest>(
        sim.world(),
    )
    .expect("live encounter-music request")
    .priority_track
    .clone()
}

/// R4: "cleared" is keyed by the boss PLACEMENT id (its runtime/LDtk id), not
/// the archetype.
fn boss_cleared(sim: &Platformer2dSimHarness, placement_id: &str) -> bool {
    matches!(
        sim.world()
            .resource::<AmbitionGameSave>()
            .data()
            .boss(placement_id),
        PersistedEncounterState::Cleared
    )
}

fn boss_alive(world: &mut World, placement_id: &str) -> Option<bool> {
    let mut q = world.query::<(
        &BossConfig,
        &ambition_platformer2d::characters::actor::BodyHealth,
    )>();
    q.iter(world)
        .find(|(config, _)| config.id == placement_id)
        .map(|(_, health)| health.alive())
}

fn boss_phase(world: &mut World, placement_id: &str) -> Option<BossEncounterPhase> {
    let mut q = world.query::<(&BossConfig, &BossEncounter)>();
    q.iter(world)
        .find(|(config, _)| config.id == placement_id)
        .and_then(|(_, status)| status.encounter.as_ref().map(|p| p.phase))
}

fn boss_max_hp(world: &mut World, placement_id: &str) -> Option<i32> {
    let mut q = world.query::<(
        &BossConfig,
        &ambition_platformer2d::characters::actor::BodyHealth,
    )>();
    q.iter(world)
        .find(|(config, _)| config.id == placement_id)
        .map(|(_, health)| health.max())
}

fn set_boss_hp(world: &mut World, placement_id: &str, hp: i32) {
    let mut q = world.query::<(
        &BossConfig,
        &mut ambition_platformer2d::characters::actor::BodyHealth,
    )>();
    for (config, mut health) in q.iter_mut(world) {
        if config.id == placement_id {
            health.health.current = hp;
        }
    }
}

fn has_encounter_for(world: &mut World, placement_id: &str) -> bool {
    let mut q = world.query::<&EncounterDef>();
    q.iter(world).any(|d| d.placement_id == placement_id)
}

fn boss_reward_chest_count(world: &mut World) -> usize {
    world.query::<&BossRewardChest>().iter(world).count()
}

/// While a boss is alive and woken, the adaptive-music request carries its
/// track. (R3 must keep boss music playing through the fight.)
#[test]
fn boss_music_plays_during_the_fight() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    spawn_mockingbird(&mut sim, "music_boss");

    // A few frames to wake the boss (Dormant → Intro) + publish its music.
    for _ in 0..15 {
        sim.step(AgentAction::default());
    }

    assert_eq!(
        music_track(&sim).as_deref(),
        Some(MOCKINGBIRD_TRACK),
        "a woken boss's encounter requests its own music track"
    );
}

/// A defeated boss leaves a Cleared save record, drops its reward chest, and
/// releases the adaptive-music request back to room music. This is the generic
/// entity-death CONTRACT R3 must preserve (cut-rope's environmental kill flows
/// through the same death path).
#[test]
fn defeated_boss_is_recorded_cleared_drops_reward_and_clears_music() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    spawn_mockingbird(&mut sim, "dying_boss");

    // Wake + register the boss, then confirm the fight music is up.
    for _ in 0..15 {
        sim.step(AgentAction::default());
    }
    assert_eq!(
        music_track(&sim).as_deref(),
        Some(MOCKINGBIRD_TRACK),
        "precondition: the boss fight music is playing before the kill"
    );
    assert!(
        !boss_cleared(&sim, "dying_boss"),
        "precondition: the boss is not pre-marked cleared"
    );

    force_kill_boss(&mut sim, "dying_boss");

    // Step past the death outro (mockingbird `death_seconds` = 2.2s ≈ 132
    // frames) so the death resolves: save Cleared + reward-chest sync + music
    // lifetime restore all run.
    for _ in 0..200 {
        sim.step(AgentAction::default());
    }

    assert!(
        boss_cleared(&sim, "dying_boss"),
        "a defeated boss must be recorded Cleared in the save (by placement id)"
    );
    assert_eq!(
        boss_reward_chest_count(sim.world_mut()),
        1,
        "a defeated boss with a DropChest reward must drop exactly one chest"
    );
    assert_eq!(
        music_track(&sim),
        None,
        "after the fight the boss-music request clears (room music resumes)"
    );
}

/// R4: "cleared" is keyed to the encounter PLACEMENT, not the archetype — so the
/// SAME boss archetype reused at a different placement is NOT pre-marked cleared.
#[test]
fn reused_archetype_at_a_new_placement_is_not_pre_cleared() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");

    // Placement A: a mockingbird the player defeats.
    spawn_mockingbird(&mut sim, "placement_a");
    for _ in 0..6 {
        sim.step(AgentAction::default());
    }
    force_kill_boss(&mut sim, "placement_a");
    for _ in 0..200 {
        sim.step(AgentAction::default());
    }
    assert!(
        boss_cleared(&sim, "placement_a"),
        "placement A must be recorded cleared after its defeat"
    );

    // Placement B: the SAME archetype, a different placement id. It must NOT be
    // pre-marked cleared just because another mockingbird was beaten.
    spawn_mockingbird(&mut sim, "placement_b");
    for _ in 0..6 {
        sim.step(AgentAction::default());
    }
    assert!(
        !boss_cleared(&sim, "placement_b"),
        "a fresh placement of a beaten archetype must NOT be pre-marked cleared"
    );
    assert_eq!(
        boss_alive(sim.world_mut(), "placement_b"),
        Some(true),
        "the reused-archetype boss at a new placement must spawn alive, not skipped"
    );
}

/// The in-place reset resets `health`/`alive` but must also clear the entity-local phase state
/// — otherwise the boss stays in last attempt's `Death` phase and the death-resolution re-kills
/// it the instant it "respawns".
#[test]
fn boss_revives_after_a_room_reset() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    spawn_mockingbird(&mut sim, "respawner");
    for _ in 0..15 {
        sim.step(AgentAction::default());
    }
    force_kill_boss(&mut sim, "respawner");
    for _ in 0..200 {
        sim.step(AgentAction::default());
    }
    assert_eq!(boss_alive(sim.world_mut(), "respawner"), Some(false));
    assert!(
        boss_cleared(&sim, "respawner"),
        "precondition: defeated + cleared"
    );

    // The NPC replay does two things: clear the placement save record + reset the
    // room features. Do both, then let it settle.
    sim.world_mut()
        .resource_mut::<AmbitionGameSave>()
        .data_mut()
        .set_boss("respawner", PersistedEncounterState::Untouched);
    sim.world_mut().write_message(ResetRoomFeaturesEvent {
        reason: RoomResetReason::Manual,
    });
    for _ in 0..15 {
        sim.step(AgentAction::default());
    }

    assert_eq!(
        boss_alive(sim.world_mut(), "respawner"),
        Some(true),
        "the boss must revive after a room reset, not stay dead from the last attempt"
    );
    assert_ne!(
        boss_phase(sim.world_mut(), "respawner"),
        Some(BossEncounterPhase::Death),
        "and not be stuck in the Death phase"
    );
    assert!(
        !boss_cleared(&sim, "respawner"),
        "and not be re-marked cleared on the revive"
    );
}

// ===== R6: spawn seam "tweaks Z" =====

/// Two DIFFERENT boss archetypes spawned via the seam are both fightable, each
/// with its own profile-derived state (independent HP pools + encounters).
#[test]
fn two_different_bosses_are_both_fightable() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let (px, py) = player_pos(sim.world_mut());

    sim.spawn_boss_at(
        "mock",
        "mockingbird",
        (px - 350.0, py),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
    );
    sim.spawn_boss_at(
        "sentinel",
        "clockwork_warden",
        (px + 350.0, py),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "clockwork_warden".to_string(),
        },
    );
    for _ in 0..6 {
        sim.step(AgentAction::default());
    }

    assert_eq!(boss_alive(sim.world_mut(), "mock"), Some(true));
    assert_eq!(boss_alive(sim.world_mut(), "sentinel"), Some(true));
    assert!(has_encounter_for(sim.world_mut(), "mock"));
    assert!(has_encounter_for(sim.world_mut(), "sentinel"));
    // Different archetypes  different authored HP pools (independent profiles).
    let mock_hp = boss_max_hp(sim.world_mut(), "mock").expect("mock seeded");
    let sentinel_hp = boss_max_hp(sim.world_mut(), "sentinel").expect("sentinel seeded");
    assert_ne!(
        mock_hp, sentinel_hp,
        "two different boss archetypes should resolve distinct HP pools"
    );
}

/// A boss spawned with `no_encounter` is a plain tough enemy: it exists + fights,
/// but NO encounter entity wraps it (so no HUD / lock-walls / win-lose).
#[test]
fn boss_spawned_with_no_encounter_has_no_encounter_entity() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let (px, py) = player_pos(sim.world_mut());

    sim.spawn_boss_at_with(
        "plain_brute",
        "mockingbird",
        (px, py),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
        BossOverrides {
            no_encounter: true,
            ..BossOverrides::default()
        },
    );
    for _ in 0..6 {
        sim.step(AgentAction::default());
    }

    assert_eq!(
        boss_alive(sim.world_mut(), "plain_brute"),
        Some(true),
        "the no-encounter boss still exists + lives (a plain tough enemy)"
    );
    assert!(
        !has_encounter_for(sim.world_mut(), "plain_brute"),
        "a no_encounter boss must NOT be wrapped by an encounter entity (no HUD)"
    );
}

/// A boss spawned with EMPTY phase triggers never phases up — it fights its one
/// phase to death. Proves phases are trivially-flippable DATA (no code change).
#[test]
fn boss_with_empty_phase_triggers_never_phases_up() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let (px, py) = player_pos(sim.world_mut());

    sim.spawn_boss_at_with(
        "no_phases",
        "mockingbird",
        (px, py),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
        BossOverrides {
            phase_triggers: Some(Vec::new()),
            ..BossOverrides::default()
        },
    );
    for _ in 0..6 {
        sim.step(AgentAction::default());
    }

    // No triggers  no intro tell, so it wakes straight into Phase1 and stays.
    assert_eq!(
        boss_phase(sim.world_mut(), "no_phases"),
        Some(BossEncounterPhase::Phase1),
        "empty triggers ⇒ the boss wakes into Phase1 (no forced Intro)"
    );

    // Drop HP well past where the mockingbird would normally phase up (66% / 25%)
    // and confirm it does NOT advance — there are no triggers to fire.
    set_boss_hp(sim.world_mut(), "no_phases", 1);
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    assert_eq!(
        boss_phase(sim.world_mut(), "no_phases"),
        Some(BossEncounterPhase::Phase1),
        "with no phase triggers the boss never phases up, even at 1 HP"
    );
}

// ===== R5: encounter script + on-death payload =====

/// An `EncounterScript` beat waiting on a gate force-kills its member when the
/// gate fires — exercised through the REAL Progression schedule (this is the
/// generic mechanism the cut-rope fight is expressed with: rope/anvil fire the
/// gate, the script does the kill, the entity death pipeline records Cleared).
#[test]
fn encounter_script_gate_force_kills_through_the_real_schedule() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let (px, py) = player_pos(sim.world_mut());
    sim.spawn_boss_at(
        "scripted",
        "mockingbird",
        (px, py),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
    );
    // Wake the boss + let `sync_boss_encounter_entities` create its encounter.
    for _ in 0..8 {
        sim.step(AgentAction::default());
    }

    // Attach a gate→ForceKill script to the boss's encounter entity.
    {
        let world = sim.world_mut();
        let mut q = world.query::<(bevy::prelude::Entity, &EncounterDef)>();
        let enc = q
            .iter(world)
            .find(|(_, def)| def.placement_id == "scripted")
            .map(|(e, _)| e)
            .expect("the woken boss has an encounter entity");
        world
            .entity_mut(enc)
            .insert(EncounterScript::new(vec![EncounterBeat::new(
                EncounterTrigger::Gate("kill_now".to_string()),
                vec![EncounterEffect::ForceKill(0)],
            )]));
    }

    // The boss is alive until the gate fires.
    assert_eq!(boss_alive(sim.world_mut(), "scripted"), Some(true));

    // Fire the gate → the script force-kills member 0; step past the death outro.
    sim.world_mut()
        .write_message(EncounterGate::new("kill_now"));
    for _ in 0..200 {
        sim.step(AgentAction::default());
    }

    assert_eq!(
        boss_alive(sim.world_mut(), "scripted"),
        Some(false),
        "the encounter script's gate→ForceKill must kill the member"
    );
    assert!(
        boss_cleared(&sim, "scripted"),
        "a script-killed boss flows through the same death→Cleared pipeline"
    );
}

/// THE ACCEPTANCE TEST for the same-room replay — and it deliberately
/// compares BEHAVIOUR rather than components.
///
/// comes back and stands inert; *leaving the room and re-entering restores its
/// movement*. That second half is the important half. A room transition
/// reconstructs the room through `RoomConstructionPlan`; the same-room replay
/// instead mutates the surviving entities back toward a presumed spawn state in
/// `reset_ecs_room_features`. So the replay has become a SECOND, incomplete
/// room constructor, and every "which component did we forget?" answer makes
/// its ledger longer.
///
/// this test must therefore never enumerate components. It measures what a
/// player sees — does the boss wake, and does it move — on a freshly
/// constructed boss, and demands the replayed one match. Whatever the two
/// constructors disagree about, this fails until they are ONE.
#[test]
fn a_replayed_boss_behaves_like_a_freshly_constructed_one() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let (px, py) = player_pos(sim.world_mut());
    // A contact-chase boss, placed far enough away that "does it move" has an
    // unambiguous answer: it closes distance until the bodies touch.
    // Placed with a clear approach lane and clear of the floor: this room has a
    // blink wall further right that a 208px-wide body reads as a front wall,
    // which would legitimately hold it still and prove nothing.
    sim.spawn_boss_at(
        "behemoth",
        "smirking_behemoth_boss",
        (px + 150.0, py - 140.0),
        (104.0, 133.0),
        BossBrain::PhaseScript {
            script_id: "smirking_behemoth_boss".to_string(),
        },
    );

    let fresh = observe_boss(&mut sim, "behemoth");
    assert!(
        fresh.woke,
        "precondition: a freshly constructed boss wakes out of Dormant",
    );
    assert!(
        fresh.displacement > 8.0,
        "precondition: a freshly constructed contact boss closes distance \
         (it ended {} px from its spawn)",
        fresh.displacement,
    );

    force_kill_boss(&mut sim, "behemoth");
    for _ in 0..200 {
        sim.step(AgentAction::default());
    }
    assert!(
        boss_cleared(&sim, "behemoth"),
        "precondition: the boss was defeated and recorded cleared",
    );

    // The replay exactly as content asks for it: ONE generic message. The
    // content half (clearing the persisted attempt record) and the host half
    // (resetting the player + the room's features) both hang off it.
    sim.world_mut()
        .write_message(ambition_platformer2d::actors::session::reset::RoomReplayRequested);

    let replayed = observe_boss(&mut sim, "behemoth");
    assert_eq!(
        boss_alive(sim.world_mut(), "behemoth"),
        Some(true),
        "the replayed boss must be alive",
    );
    assert!(
        replayed.woke,
        "a replayed boss must wake like a fresh one; it stayed Dormant",
    );
    assert!(
        replayed.displacement > 8.0,
        "a replayed boss must act like a fresh one. The fresh one ended {} px \
         from its spawn; the replayed one ended {} px — it came back inert.",
        fresh.displacement,
        replayed.displacement,
    );
}

/// What a player can see of a boss over one observation window: whether it woke
/// out of `Dormant`, and how far it ended up from the spawn it was placed at.
///
/// DISPLACEMENT from spawn, not distance travelled per frame. A contact boss
/// closes its gap and then holds, so a per-frame sum only sees motion when the
/// window happens to straddle the approach — which made this measure the
/// fixture's timing rather than the boss's behaviour.
struct BossBehaviour {
    woke: bool,
    displacement: f32,
}

fn observe_boss(sim: &mut Platformer2dSimHarness, placement_id: &str) -> BossBehaviour {
    let mut woke = false;
    for _ in 0..240 {
        sim.step(AgentAction::default());
        woke |= !matches!(
            boss_phase(sim.world_mut(), placement_id),
            Some(BossEncounterPhase::Dormant) | None,
        );
    }
    let spawn = boss_spawn(sim.world_mut(), placement_id).expect("the boss exists");
    let now = boss_pos(sim.world_mut(), placement_id).expect("the boss exists");
    BossBehaviour {
        woke,
        displacement: now.distance(spawn),
    }
}

fn boss_spawn(world: &mut World, placement_id: &str) -> Option<bevy::prelude::Vec2> {
    let mut q = world.query::<&BossConfig>();
    q.iter(world)
        .find(|config| config.id == placement_id)
        .map(|config| bevy::prelude::Vec2::new(config.spawn.x, config.spawn.y))
}

fn boss_pos(world: &mut World, placement_id: &str) -> Option<bevy::prelude::Vec2> {
    let mut q = world.query::<(&BossConfig, &BodyKinematics)>();
    q.iter(world)
        .find(|(config, _)| config.id == placement_id)
        .map(|(_, kin)| bevy::prelude::Vec2::new(kin.pos.x, kin.pos.y))
}
