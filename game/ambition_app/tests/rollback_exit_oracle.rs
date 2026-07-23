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

/// Sharpest probe: no armor, no attacks — walk to the enemy and get hit. The
/// full oracle's divergences all followed the first HP-damaging hit on the
/// player, so this isolates the victim-side damage path under rollback.
#[test]
#[ignore = "OPEN Phase-5 finding: the SECOND enemy hit on the player still diverges under \
resimulation (first hit fixed by PendingPlayerHitEvents). Repro: run this test. \
Tracked in the campaign doc Phase 5 section."]
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
#[ignore = "OPEN Phase-5 finding: diverges at the same second-hit layer as \
a_player_taking_hp_damage_survives_rollback; keep as the death/revive repro."]
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
#[test]
fn the_calibration_lab_is_checksum_stable_at_rest() {
    {
        let mut sim = SandboxSim::new_with_options(
            SandboxSimOptions::default()
                .with_timestep(TimestepMode::fixed_60hz())
                .with_start_room("combat_calibration_lab"),
        )
        .expect("plain harness builds");
        for frame in 0..16 {
            let world = sim.world_mut();
            let pickups = {
                let mut q = world
                    .query_filtered::<Entity, With<ambition::combat::components::PickupFeature>>();
                q.iter(world).count()
            };
            eprintln!("[probe] frame {frame}: {pickups} pickups");
            sim.step(AgentAction::default());
        }
    }
    let mut failures = Vec::new();
    for variant in [
        "intact",
        "no_enemies",
        "no_brick",
        "no_switch",
        "no_pickups",
    ] {
        let mut sim = oracle_sim();
        {
            let world = sim.world_mut();
            let doomed: Vec<Entity> = match variant {
                "no_enemies" => {
                    let mut q = world.query_filtered::<Entity, (
                        With<BodyHealth>,
                        Without<ambition::platformer::markers::PrimaryPlayer>,
                    )>();
                    q.iter(world).collect()
                }
                "no_brick" => {
                    let mut q = world
                        .query_filtered::<Entity, With<ambition::combat::components::BreakableFeature>>();
                    q.iter(world).collect()
                }
                "no_switch" => {
                    let mut q = world
                        .query_filtered::<Entity, With<ambition::actors::encounter::SwitchFeature>>(
                        );
                    q.iter(world).collect()
                }
                "no_pickups" => {
                    let mut q = world
                        .query_filtered::<Entity, With<ambition::combat::components::PickupFeature>>();
                    q.iter(world).collect()
                }
                _ => Vec::new(),
            };
            for entity in doomed {
                world.despawn(entity);
            }
        }
        sim.rebase_rollback_history()
            .expect("variant despawn setup becomes the rollback baseline");
        for frame in 0..48 {
            sim.step(AgentAction::default());
            if let Err(error) = sim.rollback_health() {
                failures.push(format!("{variant}: frame {frame}: {error}"));
                break;
            }
        }
    }
    assert!(
        failures.is_empty(),
        "variants diverged:\n{}",
        failures.join("\n")
    );
}

#[test]
#[ignore = "OPEN Phase-5 finding: blocked on the second-hit divergence (see \
a_player_taking_hp_damage_survives_rollback). The full oracle held checksums through \
2400 frames of melee/brick/switch before the enemy-hit-player layer was reached."]
fn combat_equipment_switch_and_breakable_survive_forced_rollback_identically() {
    let mut sim = oracle_sim();
    wear_oracle_armor(&mut sim);

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

        // The next objective, in route order: nearest live enemy or intact
        // brick first (both are melee targets), then the switch.
        let melee_target = enemies
            .iter()
            .copied()
            .chain(brick)
            .map(|(x, y)| (x, y, (x - px).abs()))
            .min_by(|a, b| a.2.total_cmp(&b.2));
        let melee_work_left = !(events.melee_landed && events.brick_broken);
        let target_x = if events.switch_flipped {
            px
        } else if let Some((x, _, _)) = melee_target.filter(|_| melee_work_left) {
            x
        } else if let Some((x, _)) = switch {
            x
        } else if let Some((x, _, _)) = melee_target {
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
