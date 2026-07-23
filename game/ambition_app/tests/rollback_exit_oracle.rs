//! **The track-0 exit oracle: cross-feature state survives forced rollback.**
//!
//! Track 0's exit criterion, verbatim: *"a sync-test run that lands a melee
//! hit, spends armor, flips a switch, and breaks a brick across a forced
//! rollback window stays checksum-identical."* The registrations for combat,
//! equipment, switch, and breakable state each landed separately; this is the
//! one run that exercises them TOGETHER inside GGRS's save/rewind/resimulate
//! loop, where an unregistered interaction between two of them would finally
//! show as a checksum divergence.
//!
//! The scenario runs in `combat_calibration_lab` — the combat-verb calibration
//! room — which authors a patrol enemy, a striker pair, a breakable brick, and
//! the classify-console switch along one floor route. A steering policy walks
//! the route: melee whatever is in reach (enemies and the brick), absorb one
//! enemy hit with a worn armor row, and flip the switch at the end. Every
//! event is asserted from world state, so a green run can't be vacuous — if
//! the policy never actually landed the hit, the test fails on the
//! observation, not the checksum.

#![cfg(feature = "rl_sim")]

use ambition::characters::actor::BodyHealth;
use ambition::characters::equipment::{EquipmentRow, OnHit, WornEquipment};
use ambition_app::rl_sim::{AgentAction, AmbitionSim, SandboxSim, SandboxSimOptions, TimestepMode};
use bevy::prelude::{Entity, With, Without};

const ORACLE_ARMOR_ID: &str = "oracle_armor";
const MAX_FRAMES: usize = 2400;

fn oracle_sim() -> SandboxSim {
    SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("combat_calibration_lab")
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("Ambition GGRS sync-test harness builds in the calibration lab")
}

/// Dress the player in one armor row so the first enemy hit is an armor spend
/// rather than an HP loss. `WornEquipment` is registered rollback state, so
/// this pre-run mutation is part of frame-0 state like any authored loadout.
fn wear_oracle_armor(sim: &mut SandboxSim) {
    let world = sim.world_mut();
    let player = {
        let mut q =
            world.query_filtered::<Entity, With<ambition::platformer::markers::PrimaryPlayer>>();
        q.single(world)
            .expect("the sim boots exactly one primary player")
    };
    let row = EquipmentRow {
        id: ORACLE_ARMOR_ID.to_string(),
        on_hit: Some(OnHit::ConsumeAsArmor { downgrade_to: None }),
        ..Default::default()
    };
    match world.get_mut::<WornEquipment>(player) {
        Some(mut worn) => worn.rows.push(row),
        None => {
            world
                .entity_mut(player)
                .insert(WornEquipment::new(vec![row]));
        }
    }
    // Deep HP so the run cannot die: a player death triggers a sim-side room
    // RESET, and room reconstruction runs through Commands that no rollback
    // can undo — a reset inside the resim window is a guaranteed divergence
    // (observed at frame ~2147 during development: enemy HP snapped back to
    // full mid-brawl, then checksums split). That boundary is a recorded
    // Phase-5 finding, not this oracle's subject; the oracle stays inside the
    // proven envelope.
    if let Some(mut health) = world.get_mut::<BodyHealth>(player) {
        health.health.max = 200;
        health.health.current = 200;
    }
    // Direct world_mut mutations must become the rollback baseline — GGRS's
    // stored history predates them, and a restore would resurrect the
    // pre-setup state (harness contract on `world_mut`; GPT 5.6 review §2).
    sim.rebase_rollback_history()
        .expect("oracle armor setup becomes the rollback baseline");
}

/// Stage the player on the open arena floor as part of the frame-0 baseline.
///
/// The authored spawn corner is capped by a head-height ledge + rebound pad
/// (the room's parkour tutorial) — crossing it is a platforming exercise, and
/// platforming is not this oracle's subject. The oracle's route (spitter,
/// brick, striker, switch) all lives on the arena floor to the right, so the
/// baseline places the player just east of the hazard cycle (x=720; the
/// hazard band spans x 592-688 and eats a body staged inside it), like the
/// armor row:
/// a setup mutation folded into rollback frame zero by the rebase that follows.
fn stage_player_on_arena_floor(sim: &mut SandboxSim) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&mut ambition::platformer::body::BodyKinematics, With<ambition::platformer::markers::PrimaryPlayer>>();
    let mut kin = q
        .single_mut(world)
        .expect("the sim boots exactly one primary player");
    kin.pos = ambition::engine_core::Vec2::new(720.0, kin.pos.y);
    kin.vel = ambition::engine_core::Vec2::ZERO;
    sim.rebase_rollback_history()
        .expect("arena-floor staging becomes the rollback baseline");
}

struct OracleEvents {
    melee_landed: bool,
    armor_spent: bool,
    brick_broken: bool,
    switch_flipped: bool,
}

impl OracleEvents {
    fn all(&self) -> bool {
        self.melee_landed && self.armor_spent && self.brick_broken && self.switch_flipped
    }
}

/// Read every oracle observation from live world state.
fn observe(sim: &mut SandboxSim, enemy_health_baseline: i32, events: &mut OracleEvents) {
    let world = sim.world_mut();

    let enemy_health: i32 = {
        let mut q = world
            .query_filtered::<&BodyHealth, Without<ambition::platformer::markers::PrimaryPlayer>>();
        q.iter(world).map(|body| body.health.current).sum()
    };
    if enemy_health < enemy_health_baseline {
        events.melee_landed = true;
    }

    {
        let mut q = world
            .query_filtered::<&WornEquipment, With<ambition::platformer::markers::PrimaryPlayer>>();
        if let Ok(worn) = q.single(world) {
            if !worn.wears(ORACLE_ARMOR_ID) {
                events.armor_spent = true;
            }
        }
    }

    {
        let mut q = world.query::<&ambition::combat::components::BreakableFeature>();
        if q.iter(world).any(|feature| feature.broken()) {
            events.brick_broken = true;
        }
    }

    {
        let mut q = world.query::<&ambition::actors::encounter::SwitchOn>();
        if q.iter(world).any(|on| on.0) {
            events.switch_flipped = true;
        }
    }
}

/// Positions of the actionable things, in sim space, queried live so the
/// policy needs no knowledge of the room's coordinate frame.
fn target_positions(
    sim: &mut SandboxSim,
) -> (Vec<(f32, f32)>, Option<(f32, f32)>, Option<(f32, f32)>) {
    let world = sim.world_mut();

    let enemies: Vec<(f32, f32)> = {
        let mut q = world.query_filtered::<(
            &ambition::platformer::body::BodyKinematics,
            &BodyHealth,
        ), Without<ambition::platformer::markers::PrimaryPlayer>>();
        q.iter(world)
            .filter(|(_, health)| health.health.current > 0)
            .map(|(kin, _)| {
                use bevy::math::bounding::BoundingVolume;
                let center = kin.aabb().center();
                (center.x, center.y)
            })
            .collect()
    };

    let brick = {
        let mut q = world.query::<(
            &ambition::combat::components::BreakableFeature,
            &ambition::engine_core::geometry::CenteredAabb,
        )>();
        q.iter(world)
            .find(|(feature, _)| !feature.broken())
            .map(|(_, aabb)| (aabb.center.x, aabb.center.y))
    };

    let switch = {
        let mut q = world.query::<(
            &ambition::actors::encounter::SwitchFeature,
            &ambition::engine_core::geometry::CenteredAabb,
        )>();
        q.iter(world)
            .next()
            .map(|(_, aabb)| (aabb.center.x, aabb.center.y))
    };

    (enemies, brick, switch)
}

/// Sharpest probe: no armor, no attacks — stand in the striker's path and take
/// repeated hits. Isolates the victim-side damage path under rollback: every
/// hit crosses the staging FIFO, the striker's swing runs its strike volume
/// through GGRS despawn/respawn, and the post-hit clock ramp rewinds. This
/// caught (in order) the unregistered `Collected` latch, the in-flight
/// victim-hit loss (`PendingPlayerHitEvents`), and the strike-volume family
/// living outside the rollback envelope.
#[test]
fn a_player_taking_hp_damage_survives_rollback() {
    let mut sim = oracle_sim();
    let mut last_hp = i32::MAX;
    for frame in 0..600 {
        let (enemies, _brick, _switch) = target_positions(&mut sim);
        let obs = sim.observation();
        let (px, _) = obs.player_pos;
        if obs.hp != last_hp {
            eprintln!("[hit] frame {frame}: player_hp={} px={px:.1}", obs.hp);
            last_hp = obs.hp;
        }
        let nearest = enemies
            .iter()
            .copied()
            .map(|(x, y)| (x, y, (x - px).abs()))
            .min_by(|a, b| a.2.total_cmp(&b.2));
        let action = match nearest {
            Some((x, _, d)) if d > 10.0 => AgentAction::move_x((x - px).signum()),
            _ => AgentAction::default(),
        };
        sim.step(action);
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame}: {error}"));
    }
}

/// Minimal repro probe: kill the patrol enemy, then stand still through its
/// in-place revive and re-aggro. Isolates the death → respawn-timer → revive →
/// re-engage cycle that the full oracle exposed.
#[test]
fn enemy_death_and_inplace_revive_survive_rollback() {
    let mut sim = oracle_sim();
    wear_oracle_armor(&mut sim);
    let mut phase = "approach";
    let mut last_hp = i32::MAX;
    for frame in 0..900 {
        let (enemies, _brick, _switch) = target_positions(&mut sim);
        let obs = sim.observation();
        let (px, _) = obs.player_pos;
        let nearest = enemies
            .iter()
            .copied()
            .map(|(x, y)| (x, y, (x - px).abs()))
            .min_by(|a, b| a.2.total_cmp(&b.2));
        let (hp, count) = {
            let world = sim.world_mut();
            let mut q = world.query_filtered::<&BodyHealth, Without<ambition::platformer::markers::PrimaryPlayer>>();
            let hp: i32 = q.iter(world).map(|b| b.health.current).sum();
            let count = q.iter(world).count();
            (hp, count)
        };
        if hp != last_hp {
            eprintln!(
                "[repro] frame {frame}: phase={phase} enemy_hp={hp} enemies={count} px={px:.1}"
            );
            last_hp = hp;
        }
        let action = match (phase, nearest) {
            ("approach", Some((x, _, d))) => {
                if d < 60.0 {
                    phase = "kill";
                }
                AgentAction::move_x((x - px).signum())
            }
            ("kill", Some((x, _, d))) => AgentAction {
                move_x: if d < 30.0 { 0.0 } else { (x - px).signum() },
                attack: frame % 6 == 2,
                ..AgentAction::default()
            },
            _ => AgentAction::default(),
        };
        sim.step(action);
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame} (phase {phase}): {error}"));
    }
}

/// Narrowing probe: the lab must be checksum-stable with NO player input at
/// all — only the enemy brains, patrol paths, and feature timers running. A
/// divergence here isolates the fault to the room's autonomous population
/// before the full oracle's combat even starts.
///
/// During development this test carried a five-variant despawn matrix
/// (no_enemies / no_brick / no_switch / no_pickups) plus a print-only pickup
/// census — the bisection tools that cornered the `Collected` latch. Those
/// cost five extra sim boots per suite run and their findings are fixed and
/// pinned elsewhere, so the standing probe keeps only the intact room
/// (2026-07-23 rollback review: trim the diagnostic matrix). Resurrect the
/// matrix from git history if this ever goes red again.
#[test]
fn the_calibration_lab_is_checksum_stable_at_rest() {
    let mut sim = oracle_sim();
    for frame in 0..48 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame}: {error}"));
    }
}

#[test]
fn combat_equipment_switch_and_breakable_survive_forced_rollback_identically() {
    let mut sim = oracle_sim();
    wear_oracle_armor(&mut sim);
    stage_player_on_arena_floor(&mut sim);

    let enemy_health_baseline: i32 = {
        let world = sim.world_mut();
        let mut q = world
            .query_filtered::<&BodyHealth, Without<ambition::platformer::markers::PrimaryPlayer>>();
        let total = q.iter(world).map(|body| body.health.current).sum();
        assert!(
            total > 0,
            "the calibration lab booted with no live enemies — the melee-hit \
             observation would be vacuous"
        );
        total
    };

    let mut events = OracleEvents {
        melee_landed: false,
        armor_spent: false,
        brick_broken: false,
        switch_flipped: false,
    };

    let mut frames_run = 0usize;
    for frame in 0..MAX_FRAMES {
        let (enemies, brick, switch) = target_positions(&mut sim);
        let player = sim.observation();
        let (px, _py) = player.player_pos;

        // The next objective, in route order: take the armor hit from the
        // nearest enemy first, then the brick, then any remaining melee proof,
        // then the switch. The brick outranks enemies once armor is spent
        // because the lab's enemies revive in place — "nearest melee target"
        // forever re-selects the respawned neighbor and the walk never leaves
        // the spawn corner.
        let nearest_enemy = enemies
            .iter()
            .copied()
            .map(|(x, y)| (x, y, (x - px).abs()))
            .min_by(|a, b| a.2.total_cmp(&b.2));
        let target_x = if events.switch_flipped {
            px
        } else if !events.armor_spent {
            nearest_enemy.map(|(x, _, _)| x).unwrap_or(px)
        } else if !events.brick_broken {
            brick.map(|(x, _)| x).unwrap_or(px)
        } else if !events.melee_landed {
            nearest_enemy.map(|(x, _, _)| x).unwrap_or(px)
        } else if let Some((x, _)) = switch {
            x
        } else {
            px
        };

        let dx = target_x - px;
        let near = dx.abs() < 70.0;
        // Until the armor row is spent, walk INTO the target without swinging —
        // the point is to TAKE a hit, and a policy that kills everything first
        // never exercises the equipment path.
        let brawling = events.armor_spent;
        let action = AgentAction {
            move_x: if dx.abs() < 8.0 { 0.0 } else { dx.signum() },
            // Melee in reach; the moveset faces along move_x.
            attack: brawling && near && frame % 6 == 2,
            // Interact pulses flip the switch once the player stands in its
            // region; harmless elsewhere (single-press Up never triggers).
            interact: near && frame % 10 == 5,
            // An occasional hop un-sticks the walk against bodies and debris.
            jump: frame % 90 == 40,
            jump_held: frame % 90 >= 40 && frame % 90 < 48,
            ..AgentAction::default()
        };

        sim.step(action);
        sim.rollback_health().unwrap_or_else(|error| {
            let late = crate::rollback_coverage::unaccounted_components(&mut sim);
            panic!(
                "frame {frame}: resimulation diverged: {error} \
                 (events at failure: melee={} armor={} brick={} switch={}, px={px:.1}, target_x={target_x:.1})\n\
                 unaccounted components at failure (candidates inserted mid-run): {late:?}",
                events.melee_landed, events.armor_spent, events.brick_broken, events.switch_flipped
            )
        });
        let before = (
            events.melee_landed,
            events.armor_spent,
            events.brick_broken,
            events.switch_flipped,
        );
        observe(&mut sim, enemy_health_baseline, &mut events);
        let after = (
            events.melee_landed,
            events.armor_spent,
            events.brick_broken,
            events.switch_flipped,
        );
        if before != after {
            eprintln!(
                "[oracle] frame {frame}: events now melee={} armor={} brick={} switch={}",
                after.0, after.1, after.2, after.3
            );
        }
        frames_run = frame + 1;
        if events.all() {
            break;
        }
    }

    assert!(
        events.melee_landed,
        "no melee hit landed in {frames_run} frames — the oracle never \
         exercised combat state, so its checksum agreement proves nothing"
    );
    assert!(
        events.armor_spent,
        "the armor row was never consumed in {frames_run} frames — the oracle \
         never exercised equipment state"
    );
    assert!(
        events.brick_broken,
        "the brick was never broken in {frames_run} frames — the oracle never \
         exercised breakable state"
    );
    assert!(
        events.switch_flipped,
        "the switch was never flipped in {frames_run} frames — the oracle \
         never exercised switch state"
    );

    let stats = sim
        .rollback_execution_stats()
        .expect("GGRS instrumentation is installed");
    assert!(
        stats.load_runs > 0,
        "no LoadWorld request was ever issued, so nothing was rewound and the \
         checksum agreement above is agreement with itself: {stats:?}"
    );
    assert!(
        stats.advance_runs > frames_run as u64,
        "resimulation must execute more GGRS frames than the {frames_run} \
         harness steps, or the same frames were never replayed: {stats:?}"
    );
}
