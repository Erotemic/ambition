//! The clockwork warden's overfit volley runs as a procedural module on the extension
//! host (fast-iteration I4), in the assembled game, from a real brain press.
//!
//! The road under test, with nothing injected: the boss pattern presses
//! `Special("overfit_volley")` → the boss domain's trigger adapter → the host runs
//! `ambition_content_modules::overfit_volley` → the projectile domain's request
//! adapter → `ProjectileSpawnRequest` → five bolts in flight.
//!
//! The second arm runs the same fight under a GGRS sync-test session. A
//! rollback resimulates the strike; if the module's strike latch were not
//! rollback state, the replay would fire a different number of shots and the
//! checksum would disagree.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::BossBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::projectiles::entity::ProjectileOwner;
use bevy::prelude::{Entity, World};

const BOSS: &str = "volley_warden";
/// One bolt per memorised point, five points over the volley's telegraph.
const FAN: usize = 5;
/// The volley's bolt speed (`overfit_volley::SHOT_SPEED`), which tells its
/// bolts from every other projectile the warden owns.
const FAN_SPEED: f32 = 360.0;
/// Long enough for the warden's phase-1 cycle to reach its volley twice.
/// Measured: 900 ticks reach it once.
const TWO_STRIKES: usize = 1800;
/// Long enough to reach it once under the slower sync-test harness.
const ONE_STRIKE: usize = 900;

fn spawn_volley_warden(sim: &mut Platformer2dSimHarness) -> Entity {
    let (px, py) = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
        let kin = q.single(world).expect("primary player exists");
        (kin.pos.x, kin.pos.y)
    };
    sim.spawn_boss_at(
        BOSS,
        "clockwork_warden",
        (px + 150.0, py - 40.0),
        (40.0, 40.0),
        BossBrain::PhaseScript {
            script_id: "clockwork_warden".to_string(),
        },
    );
    let world = sim.world_mut();
    let mut q = world.query::<(Entity, &FeatureId)>();
    q.iter(world)
        .find(|(_, f)| f.as_str() == BOSS)
        .map(|(e, _)| e)
        .expect("the spawned boss is present")
}

/// How many shots the boss owns that fly at the volley's speed. A count,
/// not a set of entities: a rollback may rebuild a shot under a new entity.
fn fan_shots(world: &mut World, boss: Entity) -> usize {
    let mut q = world.query::<(&ProjectileOwner, &BodyKinematics)>();
    q.iter(world)
        .filter(|(owner, kin)| owner.0 == boss && (kin.vel.length() - FAN_SPEED).abs() < 1.0)
        .count()
}

/// The size of each burst of volley SPAWN REQUESTS the boss made, in tick
/// order. Exact without a rollback session: a request is counted once, when
/// it is made, whatever happens to the shot after. (Under a sync-test session
/// a resimulated tick makes its requests again, so that arm counts live shots
/// instead: [`fight`].)
fn fight_requests(sim: &mut Platformer2dSimHarness, ticks: usize) -> Vec<usize> {
    use ambition_platformer2d::projectiles::spawn_request::ProjectileSpawnRequest;
    use bevy::ecs::message::Messages;
    let boss = spawn_volley_warden(sim);
    let mut cursor = sim.world().resource::<Messages<ProjectileSpawnRequest>>().get_cursor();
    let mut bursts = Vec::new();
    for _ in 0..ticks {
        sim.step(AgentAction::default());
        let messages = sim.world().resource::<Messages<ProjectileSpawnRequest>>();
        let fan = cursor
            .read(messages)
            .filter(|r| r.owner == boss && (r.projectile.body.kin.vel.length() - FAN_SPEED).abs() < 1.0)
            .count();
        if fan > 0 {
            bursts.push(fan);
        }
    }
    bursts
}

/// The size of each rise in the boss's volley shot count, in tick order.
/// A fan lives 2 s and the boss's cycle reaches it every ~7 s, so two fans
/// never overlap.
fn fight(sim: &mut Platformer2dSimHarness, ticks: usize, mut each_tick: impl FnMut(&Platformer2dSimHarness)) -> Vec<usize> {
    let boss = spawn_volley_warden(sim);
    let mut seen = fan_shots(sim.world_mut(), boss);
    let mut bursts = Vec::new();
    for _ in 0..ticks {
        sim.step(AgentAction::default());
        each_tick(sim);
        let now = fan_shots(sim.world_mut(), boss);
        if now > seen {
            bursts.push(now - seen);
        }
        seen = now;
    }
    bursts
}

#[test]
fn the_warden_fires_one_volley_per_strike_through_the_extension_host() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let bursts = fight_requests(&mut sim, TWO_STRIKES);
    assert!(
        bursts.len() >= 2,
        "the boss reached its volley {} times in {TWO_STRIKES} ticks; the arm needs two \
         strikes to show that one strike fires once: {bursts:?}",
        bursts.len()
    );
    assert!(
        bursts.iter().all(|&n| n == FAN),
        "each strike fires exactly one fan of {FAN}: {bursts:?}"
    );
}

#[test]
fn a_rollback_replays_the_fan_and_its_strike_latch() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("the GGRS sync-test harness builds");
    let mut first_error: Option<String> = None;
    let bursts = fight(&mut sim, ONE_STRIKE, |sim| {
        if first_error.is_none() {
            first_error = ambition_platformer2d::rollback::session_health(sim.world()).err();
        }
    });
    // ⭐ The premise: a sync test that never saw a fan certified nothing.
    assert!(
        !bursts.is_empty(),
        "no volley fired under the sync test, so no rollback replayed one"
    );
    assert!(
        bursts.iter().all(|&n| n == FAN),
        "each strike fires exactly one fan of {FAN} under rollback: {bursts:?}"
    );
    assert_eq!(first_error, None, "the sync-test session stayed healthy");
}

/// ⭐ THE NO-RELINK ROAD, IN THE ASSEMBLED GAME. The module crate is built for
/// `wasm32-unknown-unknown` (only the module and its port values compile), the
/// same game binary loads it, and the volley fires from the file: the
/// admitted entry is the loaded one, and the fight still fires one fan of
/// seven per strike.
#[test]
fn a_module_rebuilt_as_wasm_replaces_the_linked_one_in_the_same_game() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let wasm = ambition_platformer2d::extension::build_module_crate(&root, "ambition_content_modules")
        .expect("the module crate builds for wasm32-unknown-unknown");
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_extension_module_files(vec![wasm]),
    )
    .expect("the sandbox builds with the loaded module");
    let admitted = sim
        .world()
        .resource::<ambition_platformer2d::extension::AdmittedExtensions>()
        .0
        .clone();
    let fan = admitted
        .entries
        .iter()
        .find(|e| e.path == "ambition::overfit_volley/volley")
        .expect("the overfit volley is admitted");
    assert!(
        matches!(fan.runner, ambition_platformer2d::extension::EntryRunner::Loaded { .. }),
        "the loaded build replaced the linked one: {:?}",
        fan.runner
    );
    // Every module the file provides replaced its linked build: all its
    // entries loaded, none linked. A module can have more than one entry (the
    // sentry deploys and ticks its turret), so the replacements are counted
    // by module.
    let loaded = admitted
        .entries
        .iter()
        .filter(|e| matches!(e.runner, ambition_platformer2d::extension::EntryRunner::Loaded { .. }))
        .count();
    assert_eq!(loaded, admitted.entries.len(), "no linked entry runs beside the loaded ones");
    let modules: std::collections::BTreeSet<_> = admitted.entries.iter().map(|e| e.module.to_string()).collect();
    assert_eq!(admitted.replaced.len(), modules.len(), "{:?}", admitted.replaced);
    let bursts = fight_requests(&mut sim, TWO_STRIKES);
    assert!(bursts.len() >= 2, "{bursts:?}");
    assert!(bursts.iter().all(|&n| n == FAN), "{bursts:?}");
}

/// ⭐ HOT RELOAD, IN THE ASSEMBLED GAME. A module file that changes while the
/// game runs is PROPOSED and the rollback timeline's owner decides. With no
/// timeline it is published and the fight goes on with the new code. Under a
/// sync-test session the HARNESS started, the host may not rebase a timeline
/// it does not own, so the reload is REFUSED: it stays pending, the old code
/// keeps running, and the session stays healthy (`Q120`'s model 1).
#[test]
fn a_module_file_that_changes_while_the_game_runs_is_reloaded() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let built = ambition_platformer2d::extension::build_module_crate(&root, "ambition_content_modules")
        .expect("the module crate builds for wasm32-unknown-unknown");
    for rollback in [false, true] {
        // A file of this arm's own, so touching it disturbs no other test.
        let watched = root.join(format!(
            "target/extension-modules/hot_reload_{}_{rollback}.wasm",
            std::process::id()
        ));
        std::fs::copy(&built, &watched).unwrap();
        let mut options = Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_extension_module_files(vec![watched.clone()]);
        if rollback {
            options = options.with_sync_test_rollback_settings(4, 10);
        }
        let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the sandbox builds");
        let reloads = |sim: &Platformer2dSimHarness| {
            sim.world()
                .resource::<ambition_platformer2d::extension::reload::StagedModuleReplacement>()
                .published()
        };
        for _ in 0..30 {
            sim.step(AgentAction::default());
        }
        assert_eq!(reloads(&sim), 0, "nothing changed yet");
        let before = modules_section(&mut sim);

        // The rebuilt module: same bytes, a newer file.
        let file = std::fs::File::options().write(true).open(&watched).unwrap();
        file.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5))
            .unwrap();
        drop(file);
        for _ in 0..60 {
            sim.step(AgentAction::default());
        }
        if rollback {
            assert_eq!(reloads(&sim), 0, "a caller's timeline is not crossed by new code");
            assert_eq!(
                *sim.world().resource::<ambition_platformer2d::engine_core::MechanicalEditAdmission>(),
                ambition_platformer2d::engine_core::MechanicalEditAdmission::Refuse,
                "the timeline's owner refused the edit"
            );
            assert!(
                sim.world()
                    .resource::<ambition_platformer2d::extension::reload::StagedModuleReplacement>()
                    .is_staged(),
                "the refused reload stays staged, not lost"
            );
            assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
        } else {
            assert_eq!(reloads(&sim), 1, "the changed file was reloaded once");
            // D6: the same bytes are the same generation; the identity holds.
            assert_eq!(modules_section(&mut sim), before, "a reload of the same code is no new generation");
        }
        if rollback {
            // Live shots, because a resimulated tick repeats its requests. ⚠ A
            // shot that dies on the tick it spawns (a fan fired into a wall)
            // is never seen alive, so a later burst can read short; the first
            // one, fired in open air, must be whole.
            let bursts = fight(&mut sim, ONE_STRIKE, |_| {});
            assert_eq!(bursts.first(), Some(&FAN), "{bursts:?}");
            assert!(bursts.iter().all(|&n| n <= FAN), "{bursts:?}");
            assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
        } else {
            let bursts = fight_requests(&mut sim, TWO_STRIKES);
            assert!(!bursts.is_empty() && bursts.iter().all(|&n| n == FAN), "{bursts:?}");
        }
        let _ = std::fs::remove_file(&watched);
    }
}

/// The `extension.modules` section of the session's prepared content.
fn modules_section(sim: &mut Platformer2dSimHarness) -> (String, String) {
    let world = sim.world_mut();
    let mut q = world.query::<&ambition_platformer2d::runtime::PreparedContent>();
    let content = q.single(world).expect("one prepared session");
    let section = content
        .sections()
        .iter()
        .find(|s| s.name == ambition_platformer2d::extension::EXTENSION_MODULES_SECTION)
        .expect("the prepared content has an extension.modules section");
    (
        String::from_utf8(section.canonical_bytes().to_vec()).unwrap(),
        content.fingerprint().to_string(),
    )
}

/// ⭐ D6: THE PREPARED CONTENT IDENTITY NAMES THE MODULE CODE. A session that
/// runs a loaded `.wasm` build is a different generation from one that runs
/// the linked build, so a peer or a saved timeline cannot take one for the
/// other.
#[test]
fn the_prepared_content_identity_names_the_module_code_the_session_runs() {
    let mut linked = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let (linked_modules, linked_fingerprint) = modules_section(&mut linked);
    assert!(
        linked_modules.contains("module\tambition::overfit_volley\tnative ambition_content_modules "),
        "the linked overfit volley is in the generation:\n{linked_modules}"
    );
    assert!(!linked_modules.contains("loaded"), "{linked_modules}");

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let wasm = ambition_platformer2d::extension::build_module_crate(&root, "ambition_content_modules")
        .expect("the module crate builds for wasm32-unknown-unknown");
    let mut loaded = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_extension_module_files(vec![wasm]),
    )
    .expect("the sandbox builds with the loaded module");
    let (loaded_modules, loaded_fingerprint) = modules_section(&mut loaded);
    assert!(
        loaded_modules.contains("module\tambition::overfit_volley\tloaded ambition-ext-1 "),
        "the loaded overfit volley, with its byte digest, is in the generation:\n{loaded_modules}"
    );
    assert_ne!(linked_fingerprint, loaded_fingerprint);
}

/// The saddle point's arms the boss owns this tick: world damage boxes of the
/// arm's size, as (horizontal, vertical).
fn saddle_arms(world: &mut World, boss: Entity) -> (usize, usize) {
    use ambition_platformer2d::combat::strike::Hitbox;
    let near = |a: f32, b: f32| (a - b).abs() < 0.5;
    let mut q = world.query::<&Hitbox>();
    q.iter(world).filter(|h| h.owner == boss).fold((0, 0), |(h, v), b| {
        let he = b.half_extent;
        (
            h + usize::from(near(he.x, 220.0) && near(he.y, 36.0)),
            v + usize::from(near(he.x, 36.0) && near(he.y, 220.0)),
        )
    })
}

/// ⭐ I5: A ROLLBACK REPLAYS A MODULE'S HELD ENTITY. The saddle point's arm is
/// a box entity the combat domain spawns and holds for the module
/// (`HeldDamageBoxPort`), and turns every 1.2 s. Under a GGRS sync-test session
/// every tick is rewound and resimulated: the box entities are re-created and
/// the adapter's record of them (`combat.held_damage_boxes`) remapped. If that
/// record were not rollback state, a replay would spawn a second arm; if the
/// module's generation were not, the arm would turn at a different tick and
/// the checksums would disagree.
#[test]
fn a_rollback_replays_the_saddle_points_held_arm() {
    const WARDEN: &str = "saddle_warden";
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("the GGRS sync-test harness builds");
    let (px, py) = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
        let kin = q.single(world).expect("primary player exists");
        (kin.pos.x, kin.pos.y)
    };
    sim.spawn_boss_at(
        WARDEN,
        "clockwork_warden",
        (px + 150.0, py - 40.0),
        (40.0, 40.0),
        BossBrain::PhaseScript {
            script_id: "clockwork_warden".to_string(),
        },
    );
    let boss = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &FeatureId)>();
        q.iter(world)
            .find(|(_, f)| f.as_str() == WARDEN)
            .map(|(e, _)| e)
            .expect("the spawned boss is present")
    };
    // Phase 2 from the start, where the saddle point is, with the health
    // that phase has. (Before any tick: the session's snapshots all include
    // it.)
    {
        let world = sim.world_mut();
        let mut health = world
            .get_mut::<ambition_platformer2d::characters::actor::BodyHealth>(boss)
            .expect("the boss has health");
        health.health.current = health.health.max / 2;
        let mut status = world
            .get_mut::<ambition_platformer2d::boss_encounter::BossEncounter>(boss)
            .expect("the boss has an encounter");
        let phase = status.encounter.as_mut().expect("the encounter has a phase state");
        phase.phase = ambition_platformer2d::characters::brain::BossEncounterPhase::Phase2;
        phase.phase_elapsed = 0.0;
    }
    let mut first_error: Option<String> = None;
    let (mut horizontal_ticks, mut vertical_ticks, mut most_arms) = (0, 0, 0);
    for _ in 0..1500 {
        sim.step(AgentAction::default());
        if first_error.is_none() {
            first_error = ambition_platformer2d::rollback::session_health(sim.world()).err();
        }
        let (h, v) = saddle_arms(sim.world_mut(), boss);
        horizontal_ticks += usize::from(h > 0);
        vertical_ticks += usize::from(v > 0);
        most_arms = most_arms.max(h + v);
    }
    assert!(
        horizontal_ticks > 0 && vertical_ticks > 0,
        "the premise: the warden's saddle point turned its arm in 1500 ticks \
         ({horizontal_ticks} horizontal, {vertical_ticks} vertical)"
    );
    assert_eq!(most_arms, 1, "the replays never doubled the arm");
    assert_eq!(first_error, None, "the sync-test session stayed healthy");
}

/// The primary player, holding `item`.
fn arm_the_player(sim: &mut Platformer2dSimHarness, item: &str) -> Entity {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<Entity, PrimaryPlayerOnly>();
    let player = q.single(world).expect("primary player exists");
    let spec = ambition_platformer2d::characters::brain::held_item_by_id(item).expect("a known held item");
    world
        .entity_mut(player)
        .insert(ambition_platformer2d::combat::held_items::HeldItem::new(spec));
    player
}

fn mana_of(sim: &Platformer2dSimHarness, body: Entity) -> Option<f32> {
    ambition_platformer2d::abilities::mana::level(
        sim.world().get::<ambition_platformer2d::engine_core::resources::ActorResources>(body),
    )
    .map(|level| level.current)
}

/// ⭐ A WIELDED ITEM'S USE RUNS ON THE EXTENSION HOST, in the assembled game:
/// Attack while holding the shockwave gauntlet → the held-item domain's
/// trigger adapter → the `shockwave` module → the mana and damage-box request
/// adapters → 25 mana paid and the slam's box in the world. The second arm runs
/// it under a GGRS sync-test session, where every use is rewound and replayed.
#[test]
fn a_wielded_shockwave_runs_on_the_extension_host() {
    use ambition_platformer2d::combat::strike::Hitbox;
    for rollback in [false, true] {
        let mut options = Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
        if rollback {
            options = options.with_sync_test_rollback_settings(4, 10);
        }
        let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the sandbox builds");
        let player = arm_the_player(&mut sim, "shockwave");
        for _ in 0..5 {
            sim.step(AgentAction::default());
        }
        let before = mana_of(&sim, player).expect("the home body holds mana");
        let slam_boxes = |sim: &mut Platformer2dSimHarness| {
            let world = sim.world_mut();
            let mut q = world.query::<&Hitbox>();
            q.iter(world)
                .filter(|h| h.owner == player && (h.half_extent.x - 120.0).abs() < 0.5)
                .count()
        };
        let mut seen = 0;
        for frame in 0..4 {
            sim.step(AgentAction {
                attack: frame == 0,
                ..AgentAction::default()
            });
            seen = seen.max(slam_boxes(&mut sim));
        }
        let after = mana_of(&sim, player).expect("the home body holds mana");
        assert_eq!(seen, 1, "rollback={rollback}: one slam box from one press");
        assert!(
            (before - after - 25.0).abs() < 1.0,
            "rollback={rollback}: the slam paid its 25 mana ({before} -> {after}, regen aside)"
        );
        assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
    }
}

/// ⭐ A WIELDED TRANSIT RUNS ON THE EXTENSION HOST, in the assembled game:
/// Attack while holding the blink → the `blink` module → the transit, the
/// movement cooldown and the strike adapters → the body is moved along its
/// facing, at most 150 px, and the cooldown runs. A second press while the
/// cooldown runs does not move it. The second arm runs it under a GGRS
/// sync-test session: a replayed tick that moved the body twice would carry
/// it past 150 px, or desync the checksum.
#[test]
fn a_wielded_blink_runs_on_the_extension_host() {
    use ambition_platformer2d::abilities::ability_cooldown::AbilityCooldown;
    use ambition_platformer2d::engine_core::BodyKinematics;
    for rollback in [false, true] {
        let mut options = Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
        if rollback {
            options = options.with_sync_test_rollback_settings(4, 10);
        }
        let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the sandbox builds");
        let player = arm_the_player(&mut sim, "blink");
        for _ in 0..30 {
            sim.step(AgentAction::default());
        }
        let x = |sim: &Platformer2dSimHarness| sim.world().get::<BodyKinematics>(player).expect("a body").pos.x;
        let press = |sim: &mut Platformer2dSimHarness| {
            sim.step(AgentAction { attack: true, ..AgentAction::default() });
            for _ in 0..3 {
                sim.step(AgentAction::default());
            }
        };
        let before = x(&sim);
        press(&mut sim);
        let blinked = (x(&sim) - before).abs();
        assert!(
            blinked > 1.0 && blinked <= 150.5,
            "rollback={rollback}: one blink moved the body {blinked} px along its facing"
        );
        let cooldown = sim.world().get::<AbilityCooldown>(player).map(|c| c.remaining);
        assert!(cooldown.is_some_and(|r| r > 0.0), "rollback={rollback}: the cooldown runs: {cooldown:?}");
        let between = x(&sim);
        press(&mut sim);
        assert!(
            (x(&sim) - between).abs() < 1.0,
            "rollback={rollback}: a press while the cooldown runs moved the body"
        );
        assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
    }
}

/// ⭐ THE DIVE RUNS ON THE EXTENSION HOST, in the assembled game: Attack
/// while holding the dive gauntlet → the `dive` module → the mana, transit
/// and strike adapters → 26 mana paid and the body moved along its facing,
/// at most 140 px. The second arm runs it under a GGRS sync-test session: a
/// replayed tick that moved the body twice would carry it past 140 px.
#[test]
fn a_wielded_dive_runs_on_the_extension_host() {
    use ambition_platformer2d::engine_core::BodyKinematics;
    for rollback in [false, true] {
        let mut options = Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
        if rollback {
            options = options.with_sync_test_rollback_settings(4, 10);
        }
        let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the sandbox builds");
        let player = arm_the_player(&mut sim, "dive");
        for _ in 0..30 {
            sim.step(AgentAction::default());
        }
        let x = |sim: &Platformer2dSimHarness| sim.world().get::<BodyKinematics>(player).expect("a body").pos.x;
        let (before, mana_before) = (x(&sim), mana_of(&sim, player).expect("the home body holds mana"));
        sim.step(AgentAction { attack: true, ..AgentAction::default() });
        for _ in 0..3 {
            sim.step(AgentAction::default());
        }
        let dived = (x(&sim) - before).abs();
        assert!(dived > 1.0 && dived <= 140.5, "rollback={rollback}: one dive moved the body {dived} px");
        let mana_after = mana_of(&sim, player).expect("the home body holds mana");
        assert!(
            (mana_before - mana_after - 26.0).abs() < 1.0,
            "rollback={rollback}: the dive paid its 26 mana ({mana_before} -> {mana_after}, regen aside)"
        );
        assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
    }
}

/// ⭐ A MODULE-OWNED ENTITY, in the assembled game: Attack while holding the
/// sentry gauntlet → the `sentry` module's `deploy` entry → 28 mana paid and a
/// turret spawned by the world (its identity minted from the player's, its
/// side the player's) → each tick the `turret` entry keeps its cadence in a
/// record on the turret → at the end of its 5 s lifetime the world removes
/// it. The second arm runs it under a GGRS sync-test session, where the
/// deploy and every turret tick are rewound and replayed.
#[test]
fn a_wielded_sentry_deploys_a_module_entity_on_the_extension_host() {
    use ambition_platformer2d::abilities::module_entity::ModuleEntity;
    use ambition_platformer2d::actor::ActorFaction;
    use ambition_platformer2d::platformer::sim_id::SimId;
    for rollback in [false, true] {
        let mut options = Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
        if rollback {
            options = options.with_sync_test_rollback_settings(4, 10);
        }
        let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the sandbox builds");
        let player = arm_the_player(&mut sim, "sentry");
        for _ in 0..5 {
            sim.step(AgentAction::default());
        }
        let before = mana_of(&sim, player).expect("the home body holds mana");
        let player_id = sim.world().get::<SimId>(player).cloned().expect("the player has an identity");
        let turrets = |sim: &mut Platformer2dSimHarness| {
            let world = sim.world_mut();
            let mut q = world.query::<(Entity, &ModuleEntity, &SimId, &ActorFaction)>();
            q.iter(world)
                .map(|(e, m, id, side)| (e, m.kind.clone(), id.clone(), *side))
                .collect::<Vec<_>>()
        };
        let mut after = before;
        for frame in 0..30 {
            sim.step(AgentAction {
                attack: frame == 0,
                ..AgentAction::default()
            });
            // Read close to the press: the pool refills over time.
            if frame == 3 {
                after = mana_of(&sim, player).expect("the home body holds mana");
            }
        }
        let deployed = turrets(&mut sim);
        assert_eq!(deployed.len(), 1, "rollback={rollback}: one turret from one press: {deployed:?}");
        let (turret, kind, id, side) = deployed[0].clone();
        assert_eq!(kind, "sentry");
        assert!(
            id.as_str().starts_with(player_id.as_str()),
            "rollback={rollback}: the turret's identity {id:?} is minted from the player's {player_id:?}"
        );
        assert_eq!(side, ActorFaction::Player, "rollback={rollback}");
        assert!(
            sim.world().get::<ambition_platformer2d::extension::BodyRecords>(turret).is_some(),
            "rollback={rollback}: the turret's cadence is a record on the turret"
        );
        assert!(
            (before - after - 28.0).abs() < 1.0,
            "rollback={rollback}: the deploy paid its 28 mana ({before} -> {after}, regen aside)"
        );
        // 5 s at 60 Hz, from the press.
        for _ in 0..280 {
            sim.step(AgentAction::default());
        }
        assert!(turrets(&mut sim).is_empty(), "rollback={rollback}: the turret's lifetime ended");
        assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
    }
}

/// The vortex in the assembled game: Attack while holding the vortex
/// gauntlet → the `vortex` module's `cast` → 22 mana paid and a well opened
/// ahead of the player → the `well` entry pulls each tick and ENDS the well
/// itself after 0.9 s (its world lifetime is only the backstop). The second
/// arm runs it under a GGRS sync-test session.
#[test]
fn a_wielded_vortex_opens_and_ends_its_well_on_the_extension_host() {
    use ambition_platformer2d::abilities::module_entity::ModuleEntity;
    for rollback in [false, true] {
        let mut options = Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
        if rollback {
            options = options.with_sync_test_rollback_settings(4, 10);
        }
        let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the sandbox builds");
        let player = arm_the_player(&mut sim, "vortex");
        for _ in 0..5 {
            sim.step(AgentAction::default());
        }
        let before = mana_of(&sim, player).expect("the home body holds mana");
        let wells = |sim: &mut Platformer2dSimHarness| {
            let world = sim.world_mut();
            let mut q = world.query::<&ModuleEntity>();
            q.iter(world).filter(|e| e.kind == "vortex").count()
        };
        let mut after = before;
        let mut open_frames = 0;
        for frame in 0..90 {
            sim.step(AgentAction {
                attack: frame == 0,
                ..AgentAction::default()
            });
            if frame == 3 {
                after = mana_of(&sim, player).expect("the home body holds mana");
            }
            if wells(&mut sim) > 0 {
                open_frames += 1;
            }
        }
        assert!(
            (before - after - 22.0).abs() < 1.0,
            "rollback={rollback}: the cast paid its 22 mana ({before} -> {after}, regen aside)"
        );
        // 0.9 s at 60 Hz: the module ends the well, long before the world's
        // backstop (1.9 s).
        assert!(
            (53..=56).contains(&open_frames),
            "rollback={rollback}: the well was open for {open_frames} frames"
        );
        assert_eq!(wells(&mut sim), 0, "rollback={rollback}: the well ended");
        assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
    }
}
