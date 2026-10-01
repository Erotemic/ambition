//! The Mockingbird's echo fan runs as a procedural module on the extension
//! host (fast-iteration I4), in the assembled game, from a real brain press.
//!
//! The road under test, with nothing injected: the boss pattern presses
//! `Special("echo_fan")` → the boss domain's trigger adapter → the host runs
//! `ambition_content_modules::echo_fan` → the projectile domain's request
//! adapter → `ProjectileSpawnRequest` → seven shots in flight.
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

const BOSS: &str = "echo_fan_boss";
const FAN: usize = 7;
/// Long enough for the Mockingbird's attack cycle to reach the echo fan twice.
/// Measured: 900 ticks reach it once.
const TWO_STRIKES: usize = 1800;
/// Long enough to reach it once under the slower sync-test harness.
const ONE_STRIKE: usize = 900;

fn spawn_mockingbird(sim: &mut Platformer2dSimHarness) -> Entity {
    let (px, py) = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
        let kin = q.single(world).expect("primary player exists");
        (kin.pos.x, kin.pos.y)
    };
    sim.spawn_boss_at(
        BOSS,
        "mockingbird",
        (px + 120.0, py - 60.0),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
    );
    let world = sim.world_mut();
    let mut q = world.query::<(Entity, &FeatureId)>();
    q.iter(world)
        .find(|(_, f)| f.as_str() == BOSS)
        .map(|(e, _)| e)
        .expect("the spawned boss is present")
}

/// How many shots the boss owns that fly at the echo fan's speed. A count,
/// not a set of entities: a rollback may rebuild a shot under a new entity.
fn fan_shots(world: &mut World, boss: Entity) -> usize {
    let mut q = world.query::<(&ProjectileOwner, &BodyKinematics)>();
    q.iter(world)
        .filter(|(owner, kin)| owner.0 == boss && (kin.vel.length() - 300.0).abs() < 1.0)
        .count()
}

/// The size of each burst of echo-fan SPAWN REQUESTS the boss made, in tick
/// order. Exact without a rollback session: a request is counted once, when
/// it is made, whatever happens to the shot after. (Under a sync-test session
/// a resimulated tick makes its requests again, so that arm counts live shots
/// instead: [`fight`].)
fn fight_requests(sim: &mut Platformer2dSimHarness, ticks: usize) -> Vec<usize> {
    use ambition_platformer2d::projectiles::spawn_request::ProjectileSpawnRequest;
    use bevy::ecs::message::Messages;
    let boss = spawn_mockingbird(sim);
    let mut cursor = sim.world().resource::<Messages<ProjectileSpawnRequest>>().get_cursor();
    let mut bursts = Vec::new();
    for _ in 0..ticks {
        sim.step(AgentAction::default());
        let messages = sim.world().resource::<Messages<ProjectileSpawnRequest>>();
        let fan = cursor
            .read(messages)
            .filter(|r| r.owner == boss && (r.projectile.body.kin.vel.length() - 300.0).abs() < 1.0)
            .count();
        if fan > 0 {
            bursts.push(fan);
        }
    }
    bursts
}

/// The size of each rise in the boss's echo-fan shot count, in tick order.
/// A fan lives 2 s and the boss's cycle reaches it every ~7 s, so two fans
/// never overlap.
fn fight(sim: &mut Platformer2dSimHarness, ticks: usize, mut each_tick: impl FnMut(&Platformer2dSimHarness)) -> Vec<usize> {
    let boss = spawn_mockingbird(sim);
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
fn the_mockingbird_fires_one_fan_per_strike_through_the_extension_host() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let bursts = fight_requests(&mut sim, TWO_STRIKES);
    assert!(
        bursts.len() >= 2,
        "the boss reached its echo fan {} times in {TWO_STRIKES} ticks; the arm needs two \
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
        "no echo fan fired under the sync test, so no rollback replayed one"
    );
    assert!(
        bursts.iter().all(|&n| n == FAN),
        "each strike fires exactly one fan of {FAN} under rollback: {bursts:?}"
    );
    assert_eq!(first_error, None, "the sync-test session stayed healthy");
}

/// ⭐ THE NO-RELINK ROAD, IN THE ASSEMBLED GAME. The module crate is built for
/// `wasm32-unknown-unknown` (only the module and its port values compile), the
/// same game binary loads it, and the echo fan fires from the file: the
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
        .find(|e| e.path == "ambition::echo_fan/fire")
        .expect("the echo fan is admitted");
    assert!(
        matches!(fan.runner, ambition_platformer2d::extension::EntryRunner::Loaded { .. }),
        "the loaded build replaced the linked one: {:?}",
        fan.runner
    );
    // Every module the file provides replaced its linked build: one entry
    // each, all loaded, none linked.
    let loaded = admitted
        .entries
        .iter()
        .filter(|e| matches!(e.runner, ambition_platformer2d::extension::EntryRunner::Loaded { .. }))
        .count();
    assert_eq!(loaded, admitted.entries.len(), "no linked entry runs beside the loaded ones");
    assert_eq!(admitted.replaced.len(), loaded, "{:?}", admitted.replaced);
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
        .find(|s| s.name == "extension.modules")
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
        linked_modules.contains("module\tambition::echo_fan\tnative ambition_content_modules "),
        "the linked echo fan is in the generation:\n{linked_modules}"
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
        loaded_modules.contains("module\tambition::echo_fan\tloaded ambition-ext-1 "),
        "the loaded echo fan, with its byte digest, is in the generation:\n{loaded_modules}"
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
