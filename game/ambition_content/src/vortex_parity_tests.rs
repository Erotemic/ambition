//! The vortex, run three ways on the same inputs: its old NATIVE systems
//! (`wielded_ability_reference_tests::vortex`), its procedural module linked
//! into this process (`ambition_content_modules::vortex`, on the
//! module-entity ports), and the same module built for
//! `wasm32-unknown-unknown`.
//!
//! Each road must open the same wells (identity, place) and move every body
//! to the same place, tick for tick, play the same sounds and pay the same
//! mana, for: a driven body with gravity down and one with gravity sideways
//! and an aim stick, a body a brain drives (it does not cast), a possessed
//! enemy body (it casts, and is not pulled), a body with too little mana, and
//! a body with no identity (refused, and not charged). The bodies in reach:
//! enemies inside and outside the radius, a dead enemy, and an enemy body a
//! participant drives. The step is 1/60 s, so the well's last tick (0.9 s is
//! not a whole number of steps) is compared too.
//!
//! ⚠ Deliberately different, and so not compared: as the sentry, the native
//! cast minted the well's identity before it asked for mana.

use ambition_characters::control::{ActorControl, DrivingParticipant, PlayerSlot};
use ambition_combat::components::ActorFaction;
use ambition_combat::held_items::HeldItem;
use ambition_extension_host::{ExtensionAppExt, ExtensionHostPlugin};
use ambition_platformer2d::abilities::module_entity::ModuleEntity;
use ambition_platformer2d_core::{self as ae, BodyKinematics};
use ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use super::wielded_ability_reference_tests::vortex as native;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Road {
    NativeSystem,
    Module,
    Wasm,
}

const DT: f32 = 1.0 / 60.0;
const TICKS: usize = 140;

struct Caster {
    at: ae::Vec2,
    gravity: ae::Vec2,
    aim: [f32; 2],
    side: ActorFaction,
    driven: bool,
    mana: f32,
    named: bool,
}

const CASTERS: [Caster; 6] = [
    Caster { at: ae::Vec2::new(200.0, 300.0), gravity: ae::Vec2::new(0.0, 1.0), aim: [0.0, 0.0], side: ActorFaction::Player, driven: true, mana: 100.0, named: true },
    Caster { at: ae::Vec2::new(1000.0, 300.0), gravity: ae::Vec2::new(1.0, 0.0), aim: [0.6, -0.8], side: ActorFaction::Player, driven: true, mana: 100.0, named: true },
    Caster { at: ae::Vec2::new(1800.0, 300.0), gravity: ae::Vec2::new(0.0, 1.0), aim: [0.0, 0.0], side: ActorFaction::Player, driven: false, mana: 100.0, named: true },
    // Possessed: authored Enemy, driven, so its side is the player's.
    Caster { at: ae::Vec2::new(2600.0, 300.0), gravity: ae::Vec2::new(0.0, 1.0), aim: [-1.0, 0.0], side: ActorFaction::Enemy, driven: true, mana: 100.0, named: true },
    Caster { at: ae::Vec2::new(3400.0, 300.0), gravity: ae::Vec2::new(0.0, 1.0), aim: [0.0, 0.0], side: ActorFaction::Player, driven: true, mana: 10.0, named: true },
    Caster { at: ae::Vec2::new(4200.0, 300.0), gravity: ae::Vec2::new(0.0, 1.0), aim: [0.0, 0.0], side: ActorFaction::Player, driven: true, mana: 100.0, named: false },
];

fn presses(tick: usize, caster: usize) -> bool {
    match caster {
        0 => tick == 0 || tick == 70,
        _ => tick == 1,
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

fn body(app: &mut App, name: &str, at: ae::Vec2, side: ActorFaction) -> Entity {
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            BodyKinematics {
                pos: at,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            side,
            SimId::placement(name),
        ))
        .id()
}

fn world(road: Road) -> (App, Vec<Entity>) {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_message::<ambition_projectiles::ProjectileSpawnRequest>()
        .add_message::<ambition_vfx::EffectRequest>()
        .add_message::<ambition_sfx::OwnedSfxMessage>()
        .add_message::<ambition_characters::brain::ActorActionMessage>()
        .init_resource::<ambition_time::SimTick>()
        .insert_resource(ambition_time::WorldTime { raw_dt: DT, scaled_dt: DT });
    match road {
        Road::NativeSystem => {
            app.add_systems(Sim, (native::fire_vortex_system, native::update_vortex_wells).chain());
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
    let mut casters = Vec::new();
    for (i, c) in CASTERS.iter().enumerate() {
        let held = ambition_characters::brain::held_item_by_id("vortex").expect("a known item");
        let mut bank = ambition_platformer2d::abilities::mana::bank();
        assert!(ambition_platformer2d::abilities::mana::spend(Some(&mut bank), 100.0 - c.mana));
        let mut frame = ResolvedMotionFrame::default();
        frame.publish_resolved_frame(ae::MotionFrame::from_direction(c.gravity, 900.0));
        let mut control = ActorControl::default();
        control.0.aim = ae::LocalAxes::new(c.aim[0], c.aim[1]);
        let mut entity = app.world_mut().spawn((
            BodyKinematics {
                pos: c.at,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            control,
            HeldItem::new(held),
            frame,
            c.side,
            bank,
            ambition_sfx::BodyPresentationSource(ambition_sfx::PresentationSourceId::new(format!("caster_{i}"))),
        ));
        if c.named {
            entity.insert(SimId::placement(&format!("caster_{i}")));
        }
        if c.driven {
            entity.insert(DrivingParticipant(PlayerSlot(i as u8)));
        }
        if c.side == ActorFaction::Enemy {
            entity.insert(FeatureSimEntity);
        }
        casters.push(entity.id());
    }
    // Around caster 0's well at (400, 300): two in reach, one out, a corpse,
    // and an enemy body a participant drives.
    body(&mut app, "in_near", ae::Vec2::new(450.0, 320.0), ActorFaction::Enemy);
    body(&mut app, "in_edge", ae::Vec2::new(600.0, 300.0), ActorFaction::Enemy);
    body(&mut app, "out", ae::Vec2::new(700.0, 300.0), ActorFaction::Enemy);
    let dead = body(&mut app, "dead", ae::Vec2::new(380.0, 280.0), ActorFaction::Enemy);
    app.world_mut().entity_mut(dead).insert(ambition_characters::actor::BodyHealth::new(
        ambition_characters::actor::Health {
            current: 0,
            max: 3,
            invulnerable: Default::default(),
        },
    ));
    let driven = body(&mut app, "driven", ae::Vec2::new(420.0, 300.0), ActorFaction::Enemy);
    app.world_mut().entity_mut(driven).insert(DrivingParticipant(PlayerSlot(9)));
    // Near caster 1's well (gravity sideways, aimed) and caster 3's.
    body(&mut app, "side_a", ae::Vec2::new(1100.0, 200.0), ActorFaction::Enemy);
    body(&mut app, "side_b", ae::Vec2::new(1150.0, 450.0), ActorFaction::Enemy);
    body(&mut app, "possessed_target", ae::Vec2::new(2350.0, 320.0), ActorFaction::Enemy);
    app.insert_resource(ControlledSubject(None));
    (app, casters)
}

type Trace = Vec<Vec<String>>;

fn wells(app: &mut App, road: Road) -> Vec<String> {
    let world = app.world_mut();
    let mut out: Vec<String> = match road {
        Road::NativeSystem => world
            .query::<(&native::VortexWell, &SimId)>()
            .iter(world)
            .map(|(w, id)| format!("well {id:?} {:?}", w.center))
            .collect(),
        Road::Module | Road::Wasm => world
            .query::<(&ModuleEntity, &SimId)>()
            .iter(world)
            .map(|(e, id)| format!("well {id:?} {:?}", e.pos))
            .collect(),
    };
    out.sort();
    out
}

fn run(road: Road) -> Trace {
    let (mut app, casters) = world(road);
    let mut trace = Vec::new();
    for tick in 0..TICKS {
        for (i, caster) in casters.iter().enumerate() {
            app.world_mut().get_mut::<ActorControl>(*caster).unwrap().0.melee_pressed = presses(tick, i);
        }
        app.world_mut().run_schedule(Sim);
        let mut out = Vec::new();
        // Sorted: as the sentry, a sound is presentation.
        let mut sounds: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<ambition_sfx::OwnedSfxMessage>>()
            .drain()
            .map(|sound| format!("{:?} {:?}", sound.source, sound.request))
            .collect();
        sounds.sort();
        out.extend(sounds);
        out.extend(wells(&mut app, road));
        let world = app.world_mut();
        let mut bodies: Vec<String> = world
            .query_filtered::<(&SimId, &BodyKinematics), With<FeatureSimEntity>>()
            .iter(world)
            .map(|(id, kin)| format!("body {id:?} {:?}", kin.pos))
            .collect();
        bodies.sort();
        out.extend(bodies);
        for caster in &casters {
            let mana = ambition_platformer2d::abilities::mana::level(
                app.world().get::<ambition_platformer2d_core::resources::ActorResources>(*caster),
            )
            .map(|l| l.current);
            out.push(format!("mana {mana:?}"));
        }
        trace.push(out);
        app.world_mut().resource_mut::<ambition_time::SimTick>().0 += 1;
    }
    trace
}

fn count_in(tick: &[String], what: &str) -> usize {
    tick.iter().filter(|line| line.contains(what)).count()
}

#[test]
fn the_vortex_module_opens_and_pulls_as_its_native_systems_did() {
    let native = run(Road::NativeSystem);
    // The premise: the wells opened, pulled, and closed.
    assert_eq!(count_in(&native[2], "well "), 3, "casters 0, 1 and 3 opened wells: {:#?}", native[2]);
    assert_eq!(count_in(&native[65], "well "), 0, "every well closed: {:#?}", native[65]);
    assert_ne!(
        native[0].iter().find(|l| l.contains("in_near")),
        native[40].iter().find(|l| l.contains("in_near")),
        "the near enemy was pulled"
    );
    let module = run(Road::Module);
    for (tick, (m, n)) in module.iter().zip(&native).enumerate() {
        assert_eq!(m, n, "tick {tick}: the linked module");
    }
}

#[test]
fn the_vortex_wasm_build_opens_and_pulls_as_its_native_systems_did() {
    let native = run(Road::NativeSystem);
    let wasm = run(Road::Wasm);
    for (tick, (w, n)) in wasm.iter().zip(&native).enumerate() {
        assert_eq!(w, n, "tick {tick}: the WASM build");
    }
}

/// OW: a well pulls only the bodies in its own live room (its caster's).
/// Two bodies stand at the same place near the well, one in each of two live
/// rooms. (The native well predates live rooms: the module road only.)
#[test]
fn a_well_pulls_only_the_bodies_in_its_own_live_room() {
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
    let (mut app, casters) = world(Road::Module);
    let ours = LiveRoomInstance::from_ordinal(1);
    let theirs = LiveRoomInstance::ACTIVATION;
    app.world_mut().spawn((RoomInstanceRoot, theirs));
    app.world_mut().spawn((RoomInstanceRoot, ours));
    app.world_mut().entity_mut(casters[0]).insert(InRoomInstance(ours));
    let start = ae::Vec2::new(500.0, 300.0);
    let in_ours = body(&mut app, "in_ours", start, ActorFaction::Enemy);
    app.world_mut().entity_mut(in_ours).insert(InRoomInstance(ours));
    let in_theirs = body(&mut app, "in_theirs", start, ActorFaction::Enemy);
    app.world_mut().entity_mut(in_theirs).insert(InRoomInstance(theirs));
    for tick in 0..20 {
        app.world_mut().get_mut::<ActorControl>(casters[0]).unwrap().0.melee_pressed = tick == 0;
        app.world_mut().run_schedule(Sim);
        app.world_mut().resource_mut::<ambition_time::SimTick>().0 += 1;
    }
    let pos = |app: &App, e: Entity| app.world().get::<BodyKinematics>(e).unwrap().pos;
    assert_ne!(pos(&app, in_ours), start, "the premise: the well pulled the body in its own room");
    assert_eq!(pos(&app, in_theirs), start, "the well pulled a body in another live room");
}
