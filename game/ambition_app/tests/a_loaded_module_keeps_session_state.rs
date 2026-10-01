//! ⭐ FAST-ITERATION I5 ON THE LOADED ROAD: state a newly loaded module
//! declares, kept by the host without a host rebuild, attached to the
//! SESSION, rewound and replayed by GGRS.
//!
//! `fixtures/extension_fixture_modules` is a crate the game does not link.
//! Its `session_tally` module counts every press of the shockwave in the
//! session in one session-attached record, and every third press fires a
//! bolt. The test builds it for `wasm32-unknown-unknown`, loads the file into
//! the shipped composition, presses, and reads the record on the session
//! root.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::extension::sdk::{FieldRef, SchemaKey, Value};
use ambition_platformer2d::extension::SessionRecords;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::platformer::sim_id::SimId;
use ambition_platformer2d::projectiles::entity::ProjectileOwner;
use bevy::prelude::Entity;

const TALLY: SchemaKey = SchemaKey::new("fixture", "session_tally.tally", 1);
const PRESSES: usize = 7;
const GAP: usize = 20;

fn tally(sim: &mut Platformer2dSimHarness) -> Option<Value> {
    let world = sim.world_mut();
    let mut stores = world.query::<&SessionRecords>();
    let stores: Vec<_> = stores.iter(world).collect();
    assert_eq!(stores.len(), 1, "one session, one store");
    stores[0].get(&TALLY).map(|r| r.get(FieldRef(0)).unwrap().clone())
}

#[test]
fn a_loaded_modules_session_record_counts_every_press_through_rollback() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let wasm = ambition_platformer2d::extension::build_module_crate(&root, "ambition_extension_fixture_modules")
        .expect("the fixture crate builds for wasm32-unknown-unknown");
    for rollback in [false, true] {
        let mut options = Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_extension_module_files(vec![wasm.clone()]);
        if rollback {
            options = options.with_sync_test_rollback_settings(4, 10);
        }
        let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the sandbox builds");
        let player = {
            let world = sim.world_mut();
            let mut q = world.query_filtered::<Entity, PrimaryPlayerOnly>();
            let player = q.single(world).expect("primary player exists");
            let spec = ambition_platformer2d::characters::brain::held_item_by_id("shockwave").expect("a known item");
            world
                .entity_mut(player)
                .insert(ambition_platformer2d::combat::held_items::HeldItem::new(spec));
            player
        };
        for _ in 0..5 {
            sim.step(AgentAction::default());
        }
        assert_eq!(tally(&mut sim), None, "rollback={rollback}: no press, no record");
        let mut bolts = std::collections::BTreeSet::new();
        for frame in 0..PRESSES * GAP {
            sim.step(AgentAction {
                attack: frame % GAP == 0,
                ..AgentAction::default()
            });
            let world = sim.world_mut();
            let mut q = world.query::<(&ProjectileOwner, &SimId)>();
            bolts.extend(q.iter(world).filter(|(owner, _)| owner.0 == player).map(|(_, id)| id.clone()));
        }
        assert_eq!(
            tally(&mut sim),
            Some(Value::U32(PRESSES as u32)),
            "rollback={rollback}: one count per press, however often a press was resimulated"
        );
        assert_eq!(
            bolts.len(),
            PRESSES / 3,
            "rollback={rollback}: the record decides the later spawns: {bolts:?}"
        );
        assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
    }
}

const TRAIL: SchemaKey = SchemaKey::new("fixture", "trail_loop.trail", 1);

/// ⭐ I7.3: A GRAPH ALGORITHM IN A LOADED MODULE, through rollback. The
/// fixture's `trail_loop` keeps the cells the player passes through as a
/// bounded graph and closes a cycle when a move joins two connected nodes.
/// The player walks right on the floor (a path: no cycle), then jumps back to
/// the left over the walked ground and lands on it: the landing closes a
/// loop, and the module puts a damage box over it. The sync-test arm rewinds
/// and replays the graph (nodes, edges, the last cell and the loop cursor)
/// and must close the same loops at the same ticks.
#[test]
fn a_loaded_graph_module_closes_a_loop_the_same_way_through_rollback() {
    use ambition_platformer2d::combat::strike::Hitbox;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let wasm = ambition_platformer2d::extension::build_module_crate(&root, "ambition_extension_fixture_modules")
        .expect("the fixture crate builds for wasm32-unknown-unknown");
    let mut runs = Vec::new();
    for rollback in [false, true] {
        let mut options = Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_extension_module_files(vec![wasm.clone()]);
        if rollback {
            options = options.with_sync_test_rollback_settings(4, 10);
        }
        let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the sandbox builds");
        let player = {
            let world = sim.world_mut();
            let mut q = world.query_filtered::<Entity, PrimaryPlayerOnly>();
            let player = q.single(world).expect("primary player exists");
            let spec = ambition_platformer2d::characters::brain::held_item_by_id("shockwave").expect("a known item");
            world
                .entity_mut(player)
                .insert(ambition_platformer2d::combat::held_items::HeldItem::new(spec));
            player
        };
        let loops = |sim: &Platformer2dSimHarness| -> u32 {
            sim.world()
                .get::<ambition_platformer2d::extension::BodyRecords>(player)
                .and_then(|r| r.get(&TRAIL))
                .map(|r| match r.get(FieldRef(3)).unwrap() {
                    Value::U32(n) => *n,
                    other => panic!("loops is a u32: {other:?}"),
                })
                .unwrap_or(0)
        };
        // The loop boxes: owned by the player, and wider than one cell.
        let loop_boxes = |sim: &mut Platformer2dSimHarness| -> usize {
            let world = sim.world_mut();
            let mut q = world.query::<&Hitbox>();
            q.iter(world)
                .filter(|h| h.owner == player && h.half_extent.x > 24.5)
                .count()
        };
        let mut trace = Vec::new();
        let mut seen_boxes = 0;
        let script: Vec<AgentAction> = (0..30)
            .map(|_| AgentAction::default())
            .chain((0..50).map(|_| AgentAction { move_x: 1.0, ..AgentAction::default() }))
            .chain((0..60).map(|f| AgentAction {
                move_x: -1.0,
                jump: f == 0,
                jump_held: f < 20,
                ..AgentAction::default()
            }))
            .chain((0..20).map(|_| AgentAction::default()))
            .collect();
        let walked = 80;
        let mut loops_after_walk = None;
        for (frame, action) in script.into_iter().enumerate() {
            sim.step(action);
            if frame + 1 == walked {
                loops_after_walk = Some(loops(&sim));
            }
            seen_boxes = seen_boxes.max(loop_boxes(&mut sim));
            let pos = sim.world().get::<ambition_platformer2d::engine_core::BodyKinematics>(player).unwrap().pos;
            trace.push((frame, pos, loops(&sim)));
        }
        assert_eq!(
            loops_after_walk,
            Some(0),
            "rollback={rollback}: a straight walk is a path, not a cycle: {trace:?}"
        );
        let closed = loops(&sim);
        assert!(closed >= 1, "rollback={rollback}: the jump back closed no loop: {trace:?}");
        assert!(seen_boxes >= 1, "rollback={rollback}: no damage box over the loop");
        assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
        runs.push((closed, trace.iter().map(|(f, _, l)| (*f, *l)).collect::<Vec<_>>()));
    }
    assert_eq!(runs[0], runs[1], "the rewound graph closed different loops at different ticks");
}

/// ⭐ THE NO-COMPILE LOOP, END TO END, IN THE SHIPPED COMPOSITION: the game
/// runs a loaded module file; the file is replaced while the game runs (as
/// `scripts/build_extension_modules.sh --watch` does); the app sees the new
/// file, proposes the reload through the mechanical-edit protocol, and the
/// new file's module takes over at publication. Here the new file adds a
/// module the running game did not have (the fixture crate's), and the
/// module counts the very next presses.
#[test]
fn a_module_file_replaced_while_the_game_runs_takes_over() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let first = ambition_platformer2d::extension::build_module_crate(&root, "ambition_content_modules")
        .expect("the module crate builds for wasm32-unknown-unknown");
    let second = ambition_platformer2d::extension::build_module_crate(&root, "ambition_extension_fixture_modules")
        .expect("the fixture crate builds for wasm32-unknown-unknown");
    let dir = root.join("target/extension-modules/hot_reload_test");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("game_modules.wasm");
    std::fs::copy(&first, &file).unwrap();
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_extension_module_files(vec![file.clone()]),
    )
    .expect("the sandbox builds with the loaded file");
    let has_tally = |sim: &Platformer2dSimHarness| {
        sim.world()
            .resource::<ambition_platformer2d::extension::AdmittedExtensions>()
            .0
            .entries
            .iter()
            .any(|e| e.path == "fixture::session_tally/count")
    };
    {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<Entity, PrimaryPlayerOnly>();
        let player = q.single(world).expect("primary player exists");
        let spec = ambition_platformer2d::characters::brain::held_item_by_id("shockwave").expect("a known item");
        world
            .entity_mut(player)
            .insert(ambition_platformer2d::combat::held_items::HeldItem::new(spec));
    }
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    assert!(!has_tally(&sim), "the premise: the running file has no tally module");
    let before = session_generation(&mut sim);
    let before_terms = stated_content_terms(&mut sim);
    assert!(!before_terms.is_empty(), "the premise: the live room's roots name their content");
    assert_eq!(before.modules, before.generation, "the premise: the identity names the running modules");

    // A file system's clock can be coarse: the replacement must look newer.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::copy(&second, &file).unwrap();
    let mut frames = 0;
    while !has_tally(&sim) {
        sim.step(AgentAction::default());
        frames += 1;
        assert!(frames < 120, "the replaced file was not reloaded within 120 frames");
    }
    for frame in 0..60 {
        sim.step(AgentAction {
            attack: frame % 20 == 0,
            ..AgentAction::default()
        });
    }
    assert_eq!(tally(&mut sim), Some(Value::U32(3)), "the reloaded module counts the presses after it took over");

    // D6: the session's content is a new generation, and says which modules.
    let after = session_generation(&mut sim);
    assert_ne!(after.generation, before.generation, "the premise: the modules changed");
    assert_eq!(after.modules, after.generation, "the identity names the modules that now run");
    assert_ne!(after.identity.fingerprint, before.identity.fingerprint);
    assert_ne!(after.identity.epoch, before.identity.epoch);
    assert_eq!(after.identity, after.prepared, "the identity is the prepared content's");
    assert!(after.binding.is_some(), "the premise: the sandbox's session root has a content binding");
    assert_eq!(after.binding, after.expected_binding, "the content binding moved with it");

    // A room built after the reload is built in the new generation, and the
    // boundary does not refuse it as stale.
    let before_room = sim.observation().active_room.clone();
    let door = stand_in_a_door(&mut sim).expect("the start room authors a door");
    for _ in 0..90 {
        sim.step(AgentAction { interact: true, ..AgentAction::default() });
        if sim.observation().active_room != before_room {
            break;
        }
    }
    assert_ne!(sim.observation().active_room, before_room, "the door `{door}` did not open after the reload");
    let after_terms = stated_content_terms(&mut sim);
    assert!(
        after_terms.iter().any(|t| !before_terms.contains(t)),
        "the room behind the door names no content generation but the old one: {after_terms:?}"
    );
}

/// Each distinct content term the live construction stamps state.
fn stated_content_terms(sim: &mut Platformer2dSimHarness) -> Vec<String> {
    let world = sim.world_mut();
    let mut q = world.query::<&ambition_platformer2d::platformer::construction::TransactionId>();
    let mut out: Vec<String> = q
        .iter(world)
        .map(|stamp| stamp.peer_content_term().to_string())
        .filter(|t| t != "content-unstated" && t != "runtime-dynamic")
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Stand the primary player in an authored `Door` zone of the live room.
fn stand_in_a_door(sim: &mut Platformer2dSimHarness) -> Option<String> {
    use ambition_platformer2d::engine_core::AabbExt;
    let world = sim.world_mut();
    let live = ambition_platformer2d::world::rooms::sole_live_room_definition(world)?;
    let mut rooms = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
    let door = rooms
        .iter(world)
        .next()?
        .spec(live)
        .loading_zones
        .iter()
        .find(|z| z.activation == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door)
        .cloned()?;
    let mut player = world.query_filtered::<&mut ambition_platformer2d::platformer::body::BodyKinematics, PrimaryPlayerOnly>();
    let mut kin = player.single_mut(world).ok()?;
    kin.pos = door.aabb.center();
    kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
    Some(door.name.clone())
}

struct SessionGeneration {
    generation: String,
    modules: String,
    identity: ambition_platformer2d::runtime::PreparedContentIdentity,
    prepared: ambition_platformer2d::runtime::PreparedContentIdentity,
    binding: Option<ambition_platformer2d::actors::rooms::ActiveContentBinding>,
    expected_binding: Option<ambition_platformer2d::actors::rooms::ActiveContentBinding>,
}

/// The declared modules, and what the session root says about them.
fn session_generation(sim: &mut Platformer2dSimHarness) -> SessionGeneration {
    use ambition_platformer2d::actors::rooms::ActiveContentBinding;
    use ambition_platformer2d::runtime::{PreparedContent, PreparedContentIdentity};
    let world = sim.world_mut();
    let generation = world.resource::<ambition_platformer2d::extension::ExtensionGeneration>().0.clone();
    let mut q = world.query::<(&PreparedContent, &PreparedContentIdentity, Option<&ActiveContentBinding>)>();
    let (content, identity, binding) = q.single(world).expect("one prepared session");
    let modules = content
        .sections()
        .iter()
        .find(|s| s.name == ambition_platformer2d::extension::EXTENSION_MODULES_SECTION)
        .map(|s| String::from_utf8(s.canonical_bytes().to_vec()).unwrap())
        .expect("the prepared content has the modules section");
    SessionGeneration {
        generation,
        modules,
        identity: *identity,
        prepared: content.identity(),
        binding: binding.copied(),
        expected_binding: binding.map(|_| {
            ActiveContentBinding::content(
                content.epoch(),
                ambition_platformer2d::session::PeerContentIdentity::from_bytes(*content.fingerprint().as_bytes()),
            )
        }),
    }
}
