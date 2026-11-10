//! The cut-rope fight's ONE content-specific trigger, driven headlessly.
//!
//! ⛔⛔ `boss_lifecycle`'s header records this fight as *"content-specific +
//! headless-hard (R5 rewrites cut-rope as an EncounterScript); they remain an
//! explicit in-game verification item"*. **R5 HAS LANDED** — the fight is the
//! behemoth's authored `encounter_script` (its `boss_profiles.ron` row): the
//! generic encounter pieces, with no cut-rope-specific physics or steering.
//! ⇒ The deferral's stated condition has
//! been met, and nobody re-derived it. Measured: the room boots headlessly in
//! **0.87 s** with both authored props present.
//!
//! ⭐ WHAT THIS PINS, and why it is the right seam rather than a convenient one.
//! `detect_cut_rope_rope_cut`'s doc: *"The rope-cut is the ONLY cut-rope-specific
//! trigger; everything after it is the generic encounter script + falling-hazard
//! mechanic."* So `EncounterGate("rope_cut")` is the whole contract between this
//! content and the generic machinery. The arena's own state is entirely private
//! — a test cannot read `rope_cut` even from inside the workspace — and that is
//! correct: the published message is the seam, and asserting it is asserting what
//! other code depends on.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions};
use ambition_app::TimestepMode;
use ambition_platformer2d::boss_encounter::EncounterGate;
use ambition_platformer2d::world::rooms::RoomSet;

const CUT_ROPE_ROOM: &str = "you_have_to_cut_the_rope";
const ROPE_KIND: &str = "cut_rope_rope";

fn cut_rope_sim() -> Platformer2dSimHarness {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        // ⚠ REQUIRED, not optional: the tolerant road falls back to the authored
        // start room and would run this whole test in the wrong room, green.
        .with_required_start_room(CUT_ROPE_ROOM);
    Platformer2dSimHarness::new_with_options(opts).expect("the cut-rope room builds headlessly")
}

/// The authored rope prop's centre, read from the live room rather than authored
/// into the test — a literal here would go stale the moment the map moves.
fn rope_pos(sim: &mut Platformer2dSimHarness) -> ambition_platformer2d::engine_core::Vec2 {
    let world = sim.world_mut();
    let live_definition = ambition_platformer2d::world::rooms::sole_live_room_definition(world)
        .expect("the session has a live room");
    let mut q = world.query::<&RoomSet>();
    q.iter(world)
        .next()
        .and_then(|rooms| {
            let spec = rooms.spec(live_definition);
            assert_eq!(spec.id, CUT_ROPE_ROOM, "the harness started in the wrong room");
            spec.props
                .iter()
                .find(|p| p.kind == ROPE_KIND)
                .map(|p| p.pos)
        })
        .expect("the cut-rope room authors a rope prop")
}

fn slash(sim: &mut Platformer2dSimHarness, at: ambition_platformer2d::engine_core::Vec2) {
    use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
    sim.world_mut().write_message(HitEvent {
        contact: None,
        pieces: Vec::new(),
        volume: ambition_platformer2d::engine_core::Aabb::new(
            at,
            ambition_platformer2d::engine_core::Vec2::splat(24.0),
        )
        .into(),
        damage: 10,
        source: HitSource::Melee,
        attacker: None,
        room: None,
        target: HitTarget::UnresolvedFeatures,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
        strike_sfx: None,
            attacker_move_instance: None,
    });
}

fn rope_cut_gates(sim: &mut Platformer2dSimHarness) -> usize {
    sim.world_mut()
        .get_resource::<bevy::ecs::message::Messages<EncounterGate>>()
        .map(|gates| {
            gates
                .iter_current_update_messages()
                .filter(|gate| gate.gate == "rope_cut")
                .count()
        })
        .unwrap_or(0)
}

/// One run of the arena, from its first tick through a cut of the rope.
///
/// * The behemoth and its fight are built whole: on the first tick each exists,
///   the behemoth frees its payload when it dies and its encounter runs the
///   authored beats, with nothing added to it later.
/// * The control: with no hit, and with a hit far from the rope, no `rope_cut`
///   gate fires. Without it, a gate that fired every frame on its own, or on
///   any `HitEvent`, would pass the cut below.
/// * Slashing the authored rope publishes `rope_cut`, the gate the encounter
///   script waits on, and throws the rules' `effects.cut` shards at the rope.
/// * While the heavy object waits over the boss, the rope sparks the rules'
///   `effects.rope_sparks` rows at their size.
/// * The rope's beat walks the boss to its mark under the heavy object: the
///   script gives it a `CommandedMove`, and the boss integrator moves it by
///   that steering, the one walk road of every body (`drive_commanded_moves`).
///
/// The rules rows are changed from the shipped ones first, so what plays is
/// the rules' row and not a copy of the shipped values.
#[test]
fn slashing_the_rope_publishes_the_gate_the_encounter_script_waits_on() {
    use ambition_platformer2d::boss_encounter::{BossConfig, CommandedMove, EncounterScript, ReleaseOnDeath};
    use ambition_platformer2d::vfx::fx::FxId;
    use ambition_platformer2d::vfx::{FxRequest, VfxInRoom, VfxMessage};

    let mut sim = cut_rope_sim();
    let mut stepped = 0;

    // ── BUILT WHOLE ─────────────────────────────────────────────────────────
    let (placement, behemoth_first) = (0..300)
        .find_map(|_| {
            let world = sim.world_mut();
            let found = world
                .query::<(&BossConfig, bevy::prelude::Has<ReleaseOnDeath>)>()
                .iter(world)
                .find(|(config, _)| config.behavior.id == ambition_content::bosses::CUT_ROPE_BOSS_ID)
                .map(|(config, frees)| (config.id.clone(), frees));
            if found.is_none() {
                sim.step(AgentAction::default());
                stepped += 1;
            }
            found
        })
        .expect("the room builds its behemoth");
    assert!(behemoth_first, "the behemoth, on the first tick it exists, frees its payload when it dies");
    let script_first = (0..300)
        .find_map(|_| {
            let world = sim.world_mut();
            let found = world
                .query::<(&ambition_platformer2d::encounter::Encounter, Option<&EncounterScript>)>()
                .iter(world)
                // An encounter wrap is named by its boss's placement.
                .find(|(encounter, _)| encounter.id == placement)
                .map(|(_, script)| script.map(|script| script.beats.len()));
            if found.is_none() {
                sim.step(AgentAction::default());
                stepped += 1;
            }
            found
        })
        .expect("the behemoth's fight begins");
    assert_eq!(script_first, Some(2), "its encounter, on the first tick it exists, runs the authored beats");
    for _ in stepped..10 {
        sim.step(AgentAction::default());
    }

    // ── THE CONTROL: NO GATE WITHOUT A HIT ON THE ROPE ──────────────────────
    assert_eq!(
        rope_cut_gates(&mut sim),
        0,
        "the `rope_cut` gate fired without anything striking the rope"
    );
    let rope = rope_pos(&mut sim);
    slash(
        &mut sim,
        ambition_platformer2d::engine_core::Vec2::new(rope.x + 4_000.0, rope.y + 4_000.0),
    );
    sim.step(AgentAction::default());
    assert_eq!(
        rope_cut_gates(&mut sim),
        0,
        "a hit 4000px from the rope cut the rope, so the detector is not reading \
         the rope's geometry at all"
    );

    // ── THE RULES' ROWS, CHANGED FROM THE SHIPPED ONES ──────────────────────
    let cut = {
        let mut tuning = sim.world_mut().resource_mut::<ambition_content::tuning::AmbitionTuning>();
        let effects = &mut tuning.0.cut_rope_trap.effects;
        effects.cut.count += 3;
        effects.cut.speed += 17.0;
        effects.cut.color = [0.11, 0.22, 0.33, 0.44];
        effects.rope_sparks.effects = vec!["sonic_boom".to_string()];
        effects.rope_sparks.scale = 0.9;
        effects.cut
    };

    // ── THE CUT ─────────────────────────────────────────────────────────────
    // Stand the boss well away from the heavy object, so walking there is
    // measurable.
    {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&mut ambition_platformer2d::engine_core::BodyKinematics, bevy::prelude::With<BossConfig>>();
        for mut kin in q.iter_mut(world) {
            kin.pos.x -= 300.0;
        }
    }
    slash(&mut sim, rope);
    sim.step(AgentAction::default());
    assert!(
        rope_cut_gates(&mut sim) > 0,
        "a hit on the authored rope published no `rope_cut` gate, so the encounter \
         script never gets the lure-and-drop beat and the fight cannot be won"
    );
    let shards: Vec<_> = sim
        .world_mut()
        .resource::<bevy::ecs::message::Messages<VfxInRoom>>()
        .iter_current_update_messages()
        .filter_map(|row| match row.vfx {
            VfxMessage::Burst { pos, count, speed, color, .. } if pos == rope => Some((count, speed, color)),
            _ => None,
        })
        .collect();
    assert_eq!(shards, vec![(cut.count, cut.speed, cut.color)], "the cut's shards at the rope");

    // ── THE WAITING ROPE'S SPARKS, AND THE BOSS LURED TO ITS MARK ───────────
    let fx_this_tick = |sim: &mut Platformer2dSimHarness| -> Vec<(FxId, f32)> {
        sim.world_mut()
            .resource::<bevy::ecs::message::Messages<FxRequest>>()
            .iter_current_update_messages()
            .map(|request| (request.fx, request.scale))
            .collect()
    };
    let lured_by = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        let mut q = world.query::<(bevy::prelude::Entity, &CommandedMove, &ambition_platformer2d::engine_core::BodyKinematics)>();
        q.iter(world).next().map(|(boss, cmd, kin)| (boss, cmd.target.x, kin.pos.x, cmd.arrive_tolerance))
    };
    let mut played = fx_this_tick(&mut sim);
    let mut lured = lured_by(&mut sim);
    for _ in 0..29 {
        sim.step(AgentAction::default());
        played.extend(fx_this_tick(&mut sim));
        lured = lured.or_else(|| lured_by(&mut sim));
    }
    assert!(!played.is_empty(), "premise: the waiting rope sparked");
    assert!(
        played.iter().all(|&(fx, scale)| fx == FxId::new("sonic_boom") && scale == 0.9),
        "each spark of the waiting rope is the rules' row at the rules' size: {played:?}"
    );
    let (boss, mark, from, tolerance) = lured.expect("premise: the rope's beat gave the boss a CommandedMove");
    assert!((mark - from).abs() > tolerance * 2.0, "premise: the boss starts away from its mark ({from} vs {mark})");
    let mut closest = (mark - from).abs();
    for _ in 0..240 {
        sim.step(AgentAction::default());
        let Some(kin) = sim.world().get::<ambition_platformer2d::engine_core::BodyKinematics>(boss) else {
            break;
        };
        closest = closest.min((mark - kin.pos.x).abs());
        if closest < (mark - from).abs() * 0.5 {
            break;
        }
    }
    assert!(
        closest < (mark - from).abs() * 0.5,
        "the lured boss did not walk to its mark: it started {:.0} away and came no nearer than {closest:.0}",
        (mark - from).abs()
    );
}

/// The show when the object lands on him is the rules'
/// `cut_rope_trap.effects.landing`: a row that is not the shipped one is
/// what the landing plays. Its own run: the landing ends the waiting rope's
/// sparks, so it cannot share the cut's run.
#[test]
fn the_landing_plays_the_show_its_rules_state() {
    use ambition_platformer2d::vfx::{FireworksRequest, FxRequest, VfxInRoom, VfxMessage};
    let mut sim = cut_rope_sim();
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    let landing = {
        let mut tuning = sim.world_mut().resource_mut::<ambition_content::tuning::AmbitionTuning>();
        let landing = &mut tuning.0.cut_rope_trap.effects.landing;
        landing.explosion_scale = 0.75;
        landing.fireworks.count += 2;
        landing.fireworks.spread = (111.0, 222.0);
        landing.fireworks.seconds = 1.5;
        landing.sparks.count += 5;
        landing.sparks.color = [0.5, 0.25, 0.125, 1.0];
        *landing
    };
    let room = *ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<
        ambition_platformer2d::platformer::lifecycle::LiveRoomInstance,
    >(sim.world_mut())
    .expect("the session has a live room");
    sim.world_mut().write_message(EncounterGate::new("cut_rope_impact").in_room(Some(room)));
    sim.step(AgentAction::default());

    let world = sim.world_mut();
    let scales: Vec<f32> = world
        .resource::<bevy::ecs::message::Messages<FxRequest>>()
        .iter_current_update_messages()
        .map(|request| request.scale)
        .collect();
    assert!(scales.contains(&landing.explosion_scale), "the explosion's scale: {scales:?}");
    let shows: Vec<_> = world
        .resource::<bevy::ecs::message::Messages<FireworksRequest>>()
        .iter_current_update_messages()
        .map(|show| (show.count, show.spread, show.duration))
        .collect();
    let spread = ambition_platformer2d::engine_core::Vec2::new(landing.fireworks.spread.0, landing.fireworks.spread.1);
    assert_eq!(shows, vec![(landing.fireworks.count, spread, landing.fireworks.seconds)], "the fireworks");
    let sparks: Vec<_> = world
        .resource::<bevy::ecs::message::Messages<VfxInRoom>>()
        .iter_current_update_messages()
        .filter_map(|row| match row.vfx {
            VfxMessage::Burst { count, speed, color, .. } if color == landing.sparks.color => Some((count, speed)),
            _ => None,
        })
        .collect();
    assert_eq!(sparks, vec![(landing.sparks.count, landing.sparks.speed)], "the landing's sparks");
}

/// A replay lets the rope be cut AGAIN — the reset path, which nothing pinned.
///
/// ⭐⭐ THIS IS THE ARM THE CUT'S RUN ABOVE DOES NOT REACH. Once the script
/// has spent its rope beat, a hit on the rope drops nothing until something
/// re-arms the fight. If the reset never ran, a player who died mid-fight would
/// re-enter a room whose anvil will never drop again — the fight becomes
/// unwinnable, and the cut's run still passes because it cuts the rope exactly
/// once in a fresh world. It counts anvils, not gates: every hit on the rope
/// fires the gate, and whether a gate drops an anvil is the script's.
///
/// ⇒ It also pins the seam a refactor moved. The arena state is keyed by live
/// room (the review of OW1 cut 7e), so a new live room of the arena is a new,
/// uncut arena; the hand-rolled room-change detector (`active_room`) is gone.
/// A replay still clears the arena through
/// `reset_cut_rope_boss_arena_on_room_reset`, and this is the arm that would
/// catch a replay that did not.
#[test]
fn a_replay_lets_the_rope_be_cut_again() {

    let mut sim = cut_rope_sim();
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    let rope = rope_pos(&mut sim);
    // The anvils a slash drops, by live identity: a replayed fight's anvil has
    // the first one's `SimId` (its encounter's first drop) in the replay's new
    // live room.
    let mut dropped = std::collections::BTreeSet::new();
    let mut slash_and_count = |sim: &mut Platformer2dSimHarness| {
        slash(sim, rope);
        for _ in 0..5 {
            sim.step(AgentAction::default());
            let world = sim.world_mut();
            dropped.extend(
                world
                    .query_filtered::<(
                        &ambition_platformer2d::platformer::sim_id::SimId,
                        Option<&ambition_platformer2d::platformer::lifecycle::InRoomInstance>,
                    ), bevy::prelude::With<ambition_platformer2d::boss_encounter::FallingHazard>>()
                    .iter(world)
                    .map(|(id, room)| (id.clone(), room.map(|room| room.0))),
            );
        }
        dropped.len()
    };

    assert_eq!(slash_and_count(&mut sim), 1, "the first cut must drop the anvil");

    // ⚠ The premise the rest of this test rests on: a SECOND slash with no
    // replay drops nothing, because the script's rope beat is spent. Without
    // this, "an anvil dropped after a replay" would prove nothing -- it would
    // pass on a fight that drops an anvil on every hit.
    assert_eq!(
        slash_and_count(&mut sim),
        1,
        "a second slash dropped another anvil with no replay, so this test cannot \
         tell a working reset from a fight that never spends its rope"
    );

    // THE ASK, NOT THE FACT: `RoomReplayAdmitted` written by the test resets
    // the arena's effects and rebuilds nothing, so the fight runs on at its
    // spent beat. The replay road rebuilds the room, its behemoth and its
    // fight.
    sim.world_mut().write_message(
        ambition_platformer2d::session::reset::RoomReplayRequested::manual(),
    );
    for _ in 0..60 {
        sim.step(AgentAction::default());
    }
    assert_eq!(
        slash_and_count(&mut sim),
        2,
        "after a replay the rope could not be cut again: the anvil never drops and \
         the fight is unwinnable"
    );
}

/// ⭐⭐ **THE SCRIPT'S BEAT CLOCK, WITNESSED UNDER AN ACTUAL ROLLBACK WINDOW.**
///
/// `EncounterScript` became `component-clone-custom-checksum` on 2026-09-17
/// (schema v198) because `cursor` and `elapsed` are advanced every tick by
/// `tick_encounter_scripts`, in the sim schedule. ⛔ **THAT WAS A CLAIM ABOUT
/// REGISTRATION, NOT A READING OF THE NUMBER**, and it was made from the shape
/// of the code rather than from a rewind anybody had run.
///
/// ⚠ This room is the only place the claim can be tested through PRODUCTION:
/// the behemoth is the one shipped boss that authors an `encounter_script`, and
/// its script names this room's authored anvil. A test-side
/// insert would not do — a registered component inserted outside the rewinding
/// schedule is taken back by the first rewind, which is a different defect
/// wearing this one's clothes.
///
/// ⇒ It drives NO input at all. A test-written `HitEvent` (`slash`) is outside
/// the rewinding schedule, so a rewind takes it back and the script would sit
/// on beat 0 for a reason that has nothing to do with the registration.
/// Production spawns the encounter with its script the moment the fight begins,
/// and `EncounterScript::advance` adds `dt` to `beat_elapsed` on every tick
/// whether or not its trigger holds. The clock is the half a resimulated tick
/// inflates when nothing restores it, and it needs no gate to observe.
///
/// ⇒ The property is a RATE: over a window of the rewinding world, the beat
/// clock advances exactly one `dt` per `SimTick`. An unrestored clock advances
/// again on every resimulated tick — `check_distance` 4, so four extra
/// advances per step — and over the window that is hundreds of frames, not one.
const WINDOW: usize = 60;

fn cut_rope_rewinding_sim() -> Platformer2dSimHarness {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room(CUT_ROPE_ROOM)
        .with_sync_test_rollback_settings(4, 10);
    Platformer2dSimHarness::new_with_options(opts)
        .expect("the cut-rope room builds under a GGRS sync-test session too")
}

/// The beat the encounter's script is on and how long it has been on it.
///
/// `None` is a real answer and it is NOT "beat zero, zero seconds": it means no
/// script was ever attached, which is the vacuous case this arm has to refuse.
fn script_beat(sim: &mut Platformer2dSimHarness) -> Option<(usize, f32)> {
    use ambition_platformer2d::boss_encounter::EncounterScript;
    let world = sim.world_mut();
    let mut q = world.query::<&EncounterScript>();
    q.iter(world)
        .next()
        .map(|script| (script.cursor(), script.beat_elapsed()))
}

#[test]
fn the_encounter_script_clock_advances_one_frame_per_tick_under_a_rewind() {
    let mut sim = cut_rope_rewinding_sim();
    // Past the check distance, so every tick of the window is resimulated.
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    // ⛔⛔ THE FLOOR: no script means no subject, and a missing clock compares
    // equal to itself.
    let (beat, before) = script_beat(&mut sim).expect(
        "no `EncounterScript` exists in the rewinding world, so this arm has no \
         subject. The behemoth's encounter is spawned with the script its profile \
         authors — check its `boss_profiles.ron` row still authors one.",
    );
    let ticks_before = sim_ticks(&mut sim) as i64;
    for _ in 0..WINDOW {
        sim.step(AgentAction::default());
    }
    let (beat_after, after) = script_beat(&mut sim).expect("the script stays attached");
    let ticks = sim_ticks(&mut sim) as i64 - ticks_before;
    assert_eq!(beat_after, beat, "premise: with no cut the script stays on beat {beat}");

    // ⛔⛔ THE LIVENESS FLOOR. A frozen world freezes the clock too, and the
    // rate below would agree with itself.
    assert!(
        ticks >= WINDOW as i64,
        "the rewinding world ran {ticks} ticks for {WINDOW} steps, so its \
         sync-test session stopped advancing"
    );
    let clock_frames = |seconds: f32| (f64::from(seconds) * 60.0).round() as i64;
    assert_eq!(
        clock_frames(after) - clock_frames(before),
        ticks,
        "the encounter script's beat clock moved {} frame(s) while the rewinding \
         world ran {ticks} tick(s) ({before} -> {after} seconds). `cursor` and \
         `elapsed` are advanced by `tick_encounter_scripts` inside the rewinding \
         schedule, so the component has to be restored with the world — check its \
         registration in `ambition_encounter`'s `register_rollback_state` survived. \
         A scripted fight whose clock ran ahead fires its timed beats early, and \
         `ForceKill` is one of them.",
        clock_frames(after) - clock_frames(before),
    );
}

/// How many fixed steps this world has executed.
fn sim_ticks(sim: &mut Platformer2dSimHarness) -> u64 {
    sim.world_mut()
        .get_resource::<ambition_platformer2d::time::SimTick>()
        .expect("the sim tick is installed by the engine plugins")
        .0
}

/// The tick the staging system kills the behemoth on. Far enough in that the
/// room, the boss and its `ReleaseOnDeath` marker all exist (one step builds
/// them) and the session is past its first check distance.
const DEATH_TICK: u64 = 30;

/// ⛔⛤ **WHY THIS ARM STAGES A DEATH INSTEAD OF CUTTING THE ROPE, MEASURED
/// 2026-09-17 SO THE NEXT READER DOES NOT PAY FOR IT AGAIN.** A test-written
/// `HitEvent` does not survive a rewind — 1 `rope_cut` gate without a rollback
/// window, 0 under one — so the rope has to be cut by a real PRESS, which the
/// harness does feed into the GGRS input stream and which therefore replays.
/// The obstacle is the ROUTE, not the input road: the authored rope sits at
/// `(908, 96)` and the player spawns at `(110, 712)`, **798 px right and 616 px
/// up**. A walk-and-swing script closes to 660 px; adding a jump cadence and a
/// held up-axis climbs to `y = 293` and closes to **187 px**, and tuning a blind
/// script onto a 24 px hit volume that high is a search, not a fixture. ⇒ The
/// whole-fight arm is priced at an authored platforming route and is not what
/// this registration owes.
///
/// What it owes is the RELEASE half, and a sim-schedule staging system is the
/// road this repo already uses for a deterministic mid-window event: it is
/// replayed by every resimulation, so the kill happens on the same tick in every
/// pass. `release_payloads_on_death` keys on `BodyHealth::alive()` alone.
fn kill_the_behemoth_at_the_death_tick(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut hosts: bevy::prelude::Query<
        &mut ambition_platformer2d::characters::actor::BodyHealth,
        bevy::prelude::With<ambition_platformer2d::boss_encounter::ReleaseOnDeath>,
    >,
) {
    if tick.0 < DEATH_TICK {
        return;
    }
    for mut health in &mut hosts {
        health.health.current = 0;
    }
}

/// Per `SimTick`, the marker count each PASS of that tick saw at the head of
/// the release system.
///
/// ⛔⛤ **THE OBSERVABLE HAD TO MOVE HERE, AND THE FIRST CHOICE WAS VACUOUS.**
/// This arm first asserted the victory NPC's PRESENCE, and the poison — deleting
/// `encounter.release_on_death` — passed. The NPC is not rollback state: it
/// spawns on the FIRST pass of the death frame, nothing despawns it on a rewind,
/// and `spawn_cut_rope_victory_npc` then returns early on `existing`. So its
/// presence answers "did the release ever fire", which is true either way, and
/// says nothing about whether a resimulation can fire it AGAIN. The property the
/// registration actually buys is the marker being BACK at the head of every
/// resimulated pass.
#[derive(bevy::prelude::Resource, Default)]
struct MarkersByPass(std::collections::BTreeMap<u64, Vec<usize>>);

fn record_the_markers_each_pass_sees(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    hosts: bevy::prelude::Query<
        bevy::prelude::Entity,
        bevy::prelude::With<ambition_platformer2d::boss_encounter::ReleaseOnDeath>,
    >,
    mut log: bevy::prelude::ResMut<MarkersByPass>,
) {
    let seen = hosts.iter().count();
    log.0.entry(tick.0).or_default().push(seen);
}

/// Whether any behemoth still carries the release marker, read from outside the
/// schedule.
fn release_markers(sim: &mut Platformer2dSimHarness) -> usize {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<
        bevy::prelude::Entity,
        bevy::prelude::With<ambition_platformer2d::boss_encounter::ReleaseOnDeath>,
    >();
    q.iter(world).count()
}

fn arena_killing_the_boss() -> Platformer2dSimHarness {
    use ambition_platformer2d::sim::SimScheduleExt;
    let options = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room(CUT_ROPE_ROOM)
        .with_sync_test_rollback_settings(4, 10);
    Platformer2dSimHarness::build(options, |app, options| {
        use bevy::prelude::IntoScheduleConfigs as _;
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        let label = app.sim_schedule();
        app.init_resource::<MarkersByPass>();
        // ⚠ The recorder reads the marker as the RELEASER sees it, so it is
        // ordered before it. Unordered, it could read the world after the
        // removal and report 0 for a pass that did emit.
        //
        // ⚠ And both are after the tick advance, so both read the tick of
        // this pass. Without that edge the advance could run between them: the
        // recorder logged tick 89, the kill read 90, and the death frame's
        // pass saw no marker. Systems added elsewhere moved the advance there.
        app.add_systems(
            label,
            (
                record_the_markers_each_pass_sees,
                kill_the_behemoth_at_the_death_tick,
            )
                .chain()
                .after(ambition_platformer2d::time::advance_sim_tick)
                .before(ambition_platformer2d::boss_encounter::release_payloads_on_death),
        );
        Ok(())
    })
    .expect("the cut-rope room builds with a staged boss death")
}

/// ⭐⭐ **A RESIMULATED KILL FRAME STILL CARRIES THE RELEASE MARKER — WHICH IS
/// WHAT `encounter.release_on_death`'s ROLLBACK REGISTRATION BUYS.**
///
/// `ReleaseOnDeath` is a presence marker: `release_payloads_on_death` emits
/// `PayloadReleased` for a dead host and REMOVES the marker so it fires once.
/// `PayloadReleased` is `clear_message_on_rollback`, so the message does not
/// survive a rewind — restoring the marker is the only thing that lets a
/// resimulation emit it again. ⛔ Unregistered, the removal is permanent across
/// a rewind: the second pass of the kill frame sees no marker, emits nothing,
/// and the frame is not the frame it was the first time.
///
/// The first pass of the kill frame is the control: it saw the marker with no
/// rewind behind it. And the marker is gone at the end, so the staged death
/// reached the releaser at all.
#[test]
fn a_resimulated_kill_frame_still_carries_the_release_marker() {
    let mut sim = arena_killing_the_boss();

    // PREMISE: the marker EXISTS to be removed. A room whose boss carries no
    // `ReleaseOnDeath` says nothing about the registration.
    sim.step(AgentAction::default());
    assert_eq!(
        release_markers(&mut sim),
        1,
        "the arena did not attach ReleaseOnDeath to a behemoth, so there is no \
         marker for a rewind to restore and this arm has no subject"
    );

    // Past the kill frame by more than the check distance (4), so every pass
    // of it has run. The cap is the liveness floor's.
    for _ in 0..(DEATH_TICK as usize + 60) {
        if sim_ticks(&mut sim) > DEATH_TICK + 8 {
            break;
        }
        sim.step(AgentAction::default());
    }

    // ⛔ THE LIVENESS FLOOR. A sync-test harness that never advanced would
    // report no pass of the kill frame at all.
    assert!(
        sim_ticks(&mut sim) > DEATH_TICK + 8,
        "the rewinding arena reached tick {} and the staged death is at {DEATH_TICK}, \
         so nothing in this arm was exercised",
        sim_ticks(&mut sim)
    );
    assert_eq!(
        release_markers(&mut sim),
        0,
        "the staged death did not reach the releaser: the marker is still on"
    );

    let passes = sim
        .world()
        .resource::<MarkersByPass>()
        .0
        .get(&DEATH_TICK)
        .cloned()
        .unwrap_or_default();

    // ⛔ THE ANTI-VACUITY FLOOR: a host that ran the kill frame once would
    // satisfy the per-pass assertion below by having nothing to disagree with.
    assert!(
        passes.len() > 1,
        "the rewinding arena ran the kill frame {} time(s), so no resimulation \
         of it was ever compared. Check distance 4 should give several",
        passes.len()
    );
    assert!(
        passes.iter().all(|seen| *seen == 1),
        "a resimulated pass of the kill frame saw no release marker: {passes:?}. \
         A 0 after a 1 is the pre-registration signature — the removal was not \
         restored, PayloadReleased was cleared, and that pass of the frame \
         cannot emit what the first pass emitted"
    );
}
