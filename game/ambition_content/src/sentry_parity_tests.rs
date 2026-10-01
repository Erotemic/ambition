//! The sentry, run three ways on the same inputs: its old NATIVE systems
//! (`wielded_ability_reference_tests::sentry`), its procedural module linked
//! into this process (`ambition_content_modules::sentry`, on the
//! module-entity ports), and the same module built for
//! `wasm32-unknown-unknown`.
//!
//! Each road must deploy the same turrets (identity, side, team,
//! presentation source, position, lifetime), fire the same bolts from the
//! same owners, play the same sounds and pay the same mana, tick for tick,
//! for: a driven body, a body a brain drives, a possessed enemy body (its
//! turret fights on its driver's side), a body with too little mana, and a
//! body with no identity (refused, and not charged). The targets are: two
//! enemies at the same distance (the tie goes by identity), a dead enemy,
//! an enemy out of range, and the possessed body (its side is not `Enemy`).
//! The turrets live out their lifetime.
//!
//! ⚠ Deliberately different, and so not compared: the native deploy minted
//! the turret's identity BEFORE it asked for mana, so a press the body could
//! not pay for used a number of the body's mint stream. The module asks for
//! the spawn only when it pays. No turret's identity changes here (the body
//! that cannot pay never deploys); a later spawn of that body would get a
//! lower number.

use ambition_characters::control::{ActorControl, DrivingParticipant, PlayerSlot};
use ambition_combat::components::{ActorFaction, CenteredAabb};
use ambition_combat::held_items::HeldItem;
use ambition_extension_host::{ExtensionAppExt, ExtensionHostPlugin};
use ambition_platformer2d::abilities::module_entity::ModuleEntity;
use ambition_platformer2d_core::{self as ae, BodyKinematics};
use ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use ambition_projectiles::ProjectileSpawnRequest;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use super::wielded_ability_reference_tests::sentry as native;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Road {
    NativeSystem,
    Module,
    Wasm,
}

const DT: f32 = 0.1;
/// Past the 5 s lifetime of a turret deployed on tick 20.
const TICKS: usize = 80;

/// A deployer: where it stands, its authored side, whether a participant
/// drives it, its mana, and whether it has an identity.
struct Deployer {
    at: ae::Vec2,
    side: ActorFaction,
    driven: bool,
    mana: f32,
    named: bool,
}

const DEPLOYERS: [Deployer; 5] = [
    Deployer { at: ae::Vec2::new(200.0, 300.0), side: ActorFaction::Player, driven: true, mana: 100.0, named: true },
    Deployer { at: ae::Vec2::new(900.0, 300.0), side: ActorFaction::Player, driven: false, mana: 100.0, named: true },
    // Possessed: authored Enemy, driven, so its side is the player's. It is
    // also a target candidate that must not be shot.
    Deployer { at: ae::Vec2::new(500.0, 300.0), side: ActorFaction::Enemy, driven: true, mana: 100.0, named: true },
    Deployer { at: ae::Vec2::new(1300.0, 300.0), side: ActorFaction::Player, driven: true, mana: 10.0, named: true },
    Deployer { at: ae::Vec2::new(1500.0, 300.0), side: ActorFaction::Player, driven: true, mana: 100.0, named: false },
];

/// The ticks each deployer presses Attack on.
fn presses(tick: usize, deployer: usize) -> bool {
    match deployer {
        0 => tick == 0 || tick == 20,
        _ => tick == 2,
    }
}

fn wasm_modules() -> (
    std::sync::Arc<ambition_extension_wasm::WasmModules>,
    Vec<ambition_extension_sdk::ModuleDescriptor>,
) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = ambition_extension_wasm::build_module_crate(&root, "ambition_content_modules")
        .expect("the module crate builds for wasm32-unknown-unknown");
    ambition_extension_wasm::WasmModules::load(&std::fs::read(path).unwrap()).expect("the module file loads")
}

fn world(road: Road, reversed: bool) -> (App, Vec<Entity>) {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_message::<ProjectileSpawnRequest>()
        .add_message::<ambition_vfx::EffectRequest>()
        .add_message::<ambition_sfx::OwnedSfxMessage>()
        .add_message::<ambition_characters::brain::ActorActionMessage>()
        .init_resource::<ambition_time::SimTick>()
        .insert_resource(ambition_time::WorldTime { raw_dt: DT, scaled_dt: DT });
    match road {
        Road::NativeSystem => {
            app.add_systems(Sim, (native::fire_sentry_system, native::update_sentries).chain());
        }
        Road::Module | Road::Wasm => {
            app.add_plugins(ExtensionHostPlugin::new(Sim));
            ambition_platformer2d_runtime::extension_composition::install_ports(&mut app);
            ambition_platformer2d_runtime::extension_composition::order_phases(&mut app, Sim);
            if road == Road::Wasm {
                let (backend, modules) = wasm_modules();
                app.add_loaded_extension_modules(backend, modules, false);
            } else {
                for module in ambition_content_modules::modules() {
                    app.add_extension_module(module);
                }
            }
            app.finish();
        }
    }
    let mut deployers = vec![Entity::PLACEHOLDER; DEPLOYERS.len()];
    let mut order: Vec<usize> = (0..DEPLOYERS.len()).collect();
    if reversed {
        order.reverse();
    }
    for i in order {
        let d = &DEPLOYERS[i];
        let held = ambition_characters::brain::held_item_by_id("sentry").expect("a known item");
        let mut bank = ambition_platformer2d::abilities::mana::bank();
        assert!(ambition_platformer2d::abilities::mana::spend(Some(&mut bank), 100.0 - d.mana));
        let mut frame = ResolvedMotionFrame::default();
        frame.publish_resolved_frame(ae::MotionFrame::from_direction(ae::Vec2::new(0.0, 1.0), 900.0));
        let mut entity = app.world_mut().spawn((
            BodyKinematics {
                pos: d.at,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            ActorControl::default(),
            HeldItem::new(held),
            frame,
            d.side,
            bank,
            ambition_sfx::BodyPresentationSource(ambition_sfx::PresentationSourceId::new(format!("deployer_{i}"))),
            ambition_combat::targeting::MatchTeam(format!("team_{i}")),
        ));
        if d.named {
            entity.insert(SimId::placement(&format!("deployer_{i}")));
        }
        if d.driven {
            entity.insert(DrivingParticipant(PlayerSlot(i as u8)));
        }
        if d.side == ActorFaction::Enemy {
            entity.insert((FeatureSimEntity, CenteredAabb::new(d.at, ae::Vec2::new(12.0, 20.0))));
        }
        deployers[i] = entity.id();
    }
    let enemy = |app: &mut App, name: &str, at: ae::Vec2| {
        app.world_mut()
            .spawn((
                FeatureSimEntity,
                CenteredAabb::new(at, ae::Vec2::new(12.0, 20.0)),
                ActorFaction::Enemy,
                SimId::placement(name),
            ))
            .id()
    };
    // Equidistant from deployer 0: the tie goes by identity, so "enemy_a" is
    // shot whichever is spawned first.
    enemy(&mut app, "enemy_b", ae::Vec2::new(300.0, 200.0));
    enemy(&mut app, "enemy_a", ae::Vec2::new(100.0, 200.0));
    // Nearest to deployer 1, and dead: its turret shoots the live one behind.
    let dead = enemy(&mut app, "enemy_dead", ae::Vec2::new(950.0, 300.0));
    app.world_mut().entity_mut(dead).insert(ambition_characters::actor::BodyHealth::new(
        ambition_characters::actor::Health {
            current: 0,
            max: 3,
            invulnerable: Default::default(),
        },
    ));
    enemy(&mut app, "enemy_far", ae::Vec2::new(1200.0, 300.0));
    // Out of range of every turret.
    enemy(&mut app, "enemy_out", ae::Vec2::new(3000.0, 300.0));
    app.insert_resource(ControlledSubject(None));
    (app, deployers)
}

/// One tick's output: requests and sounds in emission order, then each
/// turret, then each deployer's mana.
type Trace = Vec<Vec<String>>;

fn turrets(app: &mut App, road: Road) -> Vec<String> {
    let world = app.world_mut();
    let mut out: Vec<String> = match road {
        Road::NativeSystem => world
            .query::<(&native::Sentry, &SimId, &ActorFaction, Option<&ambition_combat::targeting::MatchTeam>, Option<&ambition_sfx::BodyPresentationSource>)>()
            .iter(world)
            .map(|(s, id, side, team, source)| format!("turret {id:?} {side:?} {team:?} {source:?} {:?} {:?}", s.pos, s.remaining_s))
            .collect(),
        Road::Module | Road::Wasm => world
            .query::<(&ModuleEntity, &SimId, &ActorFaction, Option<&ambition_combat::targeting::MatchTeam>, Option<&ambition_sfx::BodyPresentationSource>)>()
            .iter(world)
            .map(|(e, id, side, team, source)| format!("turret {id:?} {side:?} {team:?} {source:?} {:?} {:?}", e.pos, e.remaining_s))
            .collect(),
    };
    out.sort();
    out
}

fn run(road: Road) -> Trace {
    run_spawned(road, false)
}

fn run_spawned(road: Road, reversed: bool) -> Trace {
    let (mut app, deployers) = world(road, reversed);
    let mut trace = Vec::new();
    for tick in 0..TICKS {
        for (i, body) in deployers.iter().enumerate() {
            app.world_mut().get_mut::<ActorControl>(*body).unwrap().0.melee_pressed = presses(tick, i);
        }
        app.world_mut().run_schedule(Sim);
        let mut out = Vec::new();
        let requests: Vec<_> = app.world_mut().resource_mut::<Messages<ProjectileSpawnRequest>>().drain().collect();
        for mut request in requests {
            let owner = app.world().get::<SimId>(request.owner).cloned();
            request.owner = Entity::PLACEHOLDER;
            out.push(format!("{owner:?} {request:?}"));
        }
        // Sorted: the native deploy played its sounds in query order, the
        // module in identity order. A sound is presentation: no simulation
        // state reads it, so only the set of a tick's sounds is compared.
        let mut sounds: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<ambition_sfx::OwnedSfxMessage>>()
            .drain()
            .map(|sound| format!("{:?} {:?}", sound.source, sound.request))
            .collect();
        sounds.sort();
        out.extend(sounds);
        out.extend(turrets(&mut app, road));
        for body in &deployers {
            let mana = ambition_platformer2d::abilities::mana::level(
                app.world().get::<ambition_platformer2d_core::resources::ActorResources>(*body),
            )
            .map(|l| l.current);
            out.push(format!("mana {mana:?}"));
        }
        trace.push(out);
        app.world_mut().resource_mut::<ambition_time::SimTick>().0 += 1;
    }
    trace
}

fn count(trace: &Trace, what: &str) -> usize {
    trace.iter().map(|tick| count_in(tick, what)).sum()
}

fn count_in(tick: &[String], what: &str) -> usize {
    tick.iter().filter(|line| line.contains(what)).count()
}

#[test]
fn the_sentry_module_deploys_and_fires_as_its_native_systems_did() {
    let native = run(Road::NativeSystem);
    // The premise: the reference deployed, fired, and its turrets expired.
    assert!(count(&native, "ProjectileSpawnRequest") >= 6, "the reference fired: {native:#?}");
    assert!(count_in(&native[1], "turret ") == 1, "deployer 0's turret stands on tick 1");
    assert!(count_in(&native[TICKS - 1], "turret ") == 0, "every turret expired");
    assert!(count_in(&native[10], "turret ") == 3, "three deploys stand on tick 10: {:#?}", native[10]);
    let module = run(Road::Module);
    for (tick, (m, n)) in module.iter().zip(&native).enumerate() {
        assert_eq!(m, n, "tick {tick}: the linked module");
    }
}

#[test]
fn the_sentry_wasm_build_deploys_and_fires_as_its_native_systems_did() {
    let native = run(Road::NativeSystem);
    let wasm = run(Road::Wasm);
    for (tick, (w, n)) in wasm.iter().zip(&native).enumerate() {
        assert_eq!(w, n, "tick {tick}: the WASM build");
    }
}

/// Which turret fires first decides which bolt gets which identity (the
/// projectile domain gives identities in request order). Spawning the
/// deployers in the other order changes the query order of every body and
/// turret, and must change nothing in the trace.
#[test]
fn the_spawn_order_of_the_deployers_decides_nothing() {
    let forwards = run_spawned(Road::Module, false);
    let backwards = run_spawned(Road::Module, true);
    for (tick, (b, f)) in backwards.iter().zip(&forwards).enumerate() {
        assert_eq!(b, f, "tick {tick}");
    }
}

/// OW: a turret is in its deployer's live room, and shoots only a body in
/// that room. Two rooms are live; the turret's room has an enemy 250 to its
/// right, the other room one 50 to its left, at the same coordinates a room
/// apart. (The native sentry predates live rooms and read no room: this is
/// the module road only.)
#[test]
fn a_turret_is_in_its_deployers_live_room_and_shoots_only_there() {
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
    let (mut app, deployers) = world(Road::Module, false);
    let ours = LiveRoomInstance::from_ordinal(1);
    let theirs = LiveRoomInstance::ACTIVATION;
    app.world_mut().spawn((RoomInstanceRoot, theirs));
    app.world_mut().spawn((RoomInstanceRoot, ours));
    app.world_mut().entity_mut(deployers[0]).insert(InRoomInstance(ours));
    let at = DEPLOYERS[0].at;
    let enemy = |app: &mut App, name: &str, x: f32, room: LiveRoomInstance| {
        app.world_mut().spawn((
            FeatureSimEntity,
            CenteredAabb::new(ae::Vec2::new(x, at.y), ae::Vec2::new(12.0, 20.0)),
            ActorFaction::Enemy,
            SimId::placement(name),
            InRoomInstance(room),
        ));
    };
    enemy(&mut app, "near_other_room", at.x - 50.0, theirs);
    enemy(&mut app, "far_own_room", at.x + 250.0, ours);
    let mut shots = Vec::new();
    for tick in 0..40 {
        app.world_mut().get_mut::<ActorControl>(deployers[0]).unwrap().0.melee_pressed = tick == 0;
        app.world_mut().run_schedule(Sim);
        let requests: Vec<_> = app.world_mut().resource_mut::<Messages<ProjectileSpawnRequest>>().drain().collect();
        shots.extend(requests.into_iter().map(|r| r.projectile.body.kin.vel));
        app.world_mut().resource_mut::<ambition_time::SimTick>().0 += 1;
    }
    let world = app.world_mut();
    let turrets: Vec<Option<InRoomInstance>> = world
        .query::<(&ModuleEntity, Option<&InRoomInstance>)>()
        .iter(world)
        .map(|(_, room)| room.copied())
        .collect();
    assert_eq!(turrets, vec![Some(InRoomInstance(ours))], "one turret, in its deployer's room");
    assert!(!shots.is_empty(), "the premise: the turret fired");
    assert!(
        shots.iter().all(|vel| vel.x > 0.0),
        "the turret shot at a body in another live room: {shots:?}"
    );
}
