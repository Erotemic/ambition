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
