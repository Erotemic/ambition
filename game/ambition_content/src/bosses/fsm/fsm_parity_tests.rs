//! The Flying Spaghetti Monster's conductor, run three ways on the same
//! fight: the NATIVE conductor (`conductor_reference_tests`), its procedural
//! module linked into this process (`ambition_content_modules::fsm`, a
//! conducted boss on the extension host), and the same module built for
//! `wasm32-unknown-unknown`.
//!
//! The fight is scripted through the boss's pattern state: a swim, then every
//! move's tell and strike (the lash, the volley, the pulse, the grasp, the
//! dive with its landing and the stranded god, the appendages lifting it),
//! the same move twice in a row, and its death. Each road must put the god in
//! the same place, hold or release its pose the same way, face it the same
//! way, draw it with the same row, and swing, throw, summon, burst and sound
//! the same, tick for tick. Three arms: the god alone, the god while a
//! participant drives it, and the god enraged.
//!
//! ⚠ Deliberately different, and so not compared:
//! - a summon's id: the summon port names it `<label>:<boss id>:<serial>`,
//!   the native conductor `fsm_noodling:<serial>`;
//! - the inspector name of the dive's shocks (a module's held box has none);
//! - the side of a DRIVEN god's volumes: the riding-hitbox port puts them on
//!   the owner's effective side (the driver's), the native conductor on the
//!   boss's always. The driven arm masks the side of every hitbox.

use ambition_boss_encounter::BossClusterScratch;
use ambition_characters::brain::{BossAttackProfile, BossAttackState};
use ambition_combat::components::ActorTarget;
use ambition_combat::strike::{Hitbox, HitboxLifetime};
use ambition_extension_host::{ExtensionAppExt, ExtensionHostPlugin, ExtensionSet};
use ambition_platformer2d_core::{self as ae, BodyKinematics};
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, RoomInstanceRoot};
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
use ambition_projectiles::ProjectileSpawnRequest;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use super::conductor_reference_tests as native;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Road {
    NativeSystem,
    Module,
    Wasm,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Arm {
    Alone,
    Driven,
    Enraged,
}

const DT: f32 = 1.0 / 60.0;
const TICKS: usize = 340;
const DIES_AT: usize = 315;

/// The pattern's script: (first tick, last tick, key, striking).
const SCRIPT: &[(usize, usize, &str, bool)] = &[
    (10, 29, "noodle_lash", false),
    (30, 44, "noodle_lash", true),
    (55, 69, "meatball_volley", false),
    (70, 79, "meatball_volley", true),
    (80, 94, "noodly_pulse", false),
    (95, 104, "noodly_pulse", true),
    (105, 119, "noodly_grasp", false),
    (120, 134, "noodly_grasp", true),
    (135, 159, "noodly_dive", false),
    (160, 189, "noodly_dive", true),
    (220, 234, "lesser_appendages", false),
    (235, 249, "lesser_appendages", true),
    // The same move twice: the second tell restarts the part.
    (255, 264, "noodle_lash", false),
    (265, 274, "noodle_lash", false),
    (275, 284, "noodle_lash", true),
];

fn attack_at(tick: usize) -> BossAttackState {
    let mut state = BossAttackState::default();
    for (first, last, key, striking) in SCRIPT {
        if (*first..=*last).contains(&tick) {
            let remaining = (*last + 1 - tick) as f32 * DT;
            let profile = Some(BossAttackProfile::Special((*key).to_string()));
            if *striking {
                state.active_profile = profile;
                state.active_remaining = remaining;
            } else {
                state.telegraph_profile = profile;
                state.telegraph_remaining = remaining;
            }
        }
    }
    state
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

fn hall_world() -> ae::World {
    ae::World::new(
        "fsm_hall",
        ae::Vec2::new(1800.0, 1000.0),
        ae::Vec2::ZERO,
        vec![
            ae::Block::solid("floor", ae::Vec2::new(0.0, 900.0), ae::Vec2::new(1800.0, 100.0)),
            ae::Block::solid("left", ae::Vec2::new(0.0, 0.0), ae::Vec2::new(100.0, 900.0)),
            ae::Block::solid("right", ae::Vec2::new(1700.0, 0.0), ae::Vec2::new(100.0, 900.0)),
        ],
    )
}

fn world(road: Road, arm: Arm) -> (App, Entity, Entity) {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_message::<ambition_characters::brain::ActorActionMessage>()
        .add_message::<ProjectileSpawnRequest>()
        .add_message::<ambition_vfx::EffectRequest>()
        .add_message::<ambition_vfx::vfx::VfxInRoom>()
        .add_message::<ambition_sfx::OwnedSfxMessage>()
        .init_resource::<ambition_time::SimTick>()
        .insert_resource(ambition_time::WorldTime::new(DT, DT));
    match road {
        Road::NativeSystem => {
            app.add_systems(Sim, (native::face_conducted_gods, native::conduct_fsm).chain());
        }
        Road::Module | Road::Wasm => {
            app.add_plugins(ExtensionHostPlugin::new(Sim));
            ambition_platformer2d_runtime::extension_composition::install_ports(&mut app);
            app.add_systems(
                Sim,
                ambition_boss_encounter::conduct::face_conducted_bosses
                    .before(ExtensionSet::Collect(ambition_extension_sdk::phases::BOSS_CONDUCT)),
            );
            if road == Road::Wasm {
                let (backend, modules) = wasm_modules();
                app.add_loaded_extension_modules("ambition_content_modules.wasm", backend, modules, false);
            } else {
                for module in ambition_content_modules::modules() {
                    app.add_extension_module(module);
                }
            }
            app.finish();
        }
    }
    app.world_mut().spawn((RoomInstanceRoot, LiveRoomInstance::ACTIVATION, ae::RoomGeometry(hall_world())));
    let player = app
        .world_mut()
        .spawn((
            BodyKinematics {
                pos: ae::Vec2::new(500.0, 870.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(20.0, 30.0),
                facing: 1.0,
            },
            PlayerEntity,
        ))
        .id();
    let catalog = crate::bosses::shipped_boss_catalog();
    // Built as the game builds it: a placement id of its own, and the god's
    // behaviour through its phase script.
    let mut boss = BossClusterScratch::new(
        &catalog,
        "fsm_placement",
        "Flying Spaghetti Monster",
        ae::Aabb::new(ae::Vec2::new(900.0, 635.0), ae::Vec2::new(95.0, 44.0)),
        ambition_entity_catalog::placements::BossBrain::PhaseScript { script_id: native::FSM_ID.to_string() },
    );
    assert_eq!(boss.config.behavior.id, native::FSM_ID, "the premise: the god's behaviour");
    boss.kin.facing = -1.0;
    if arm == Arm::Enraged {
        let mut state = ambition_characters::boss_encounter::ActorPhaseState::new(Vec::new());
        state.phase = ambition_boss_encounter::BossEncounterPhase::Enrage;
        boss.status.encounter = Some(state);
    }
    let kin = boss.kin;
    let mut god = app.world_mut().spawn((
        boss.kin,
        ae::CenteredAabb::new(kin.pos, kin.size * 0.5),
        boss.config,
        boss.status,
        boss.health,
        BossAttackState::default(),
        ActorTarget {
            entity: Some(player),
            pos: ae::Vec2::new(500.0, 870.0),
        },
        ambition_characters::control::ActorControl::default(),
        ambition_characters::actor::ActorFaction::Boss,
        ambition_platformer2d::sprite_sheet::character::PinnedRow::default(),
    ));
    match road {
        Road::NativeSystem => {
            god.insert(native::FsmConductor::new(if kin.facing < 0.0 { -1.0 } else { 1.0 }));
        }
        Road::Module | Road::Wasm => {
            god.insert(ambition_boss_encounter::conduct::ConductedFacing::of_facing(kin.facing));
        }
    }
    if arm == Arm::Driven {
        god.insert(ambition_characters::control::DrivingParticipant(
            ambition_characters::control::PlayerSlot(0),
        ));
    }
    let god = god.id();
    (app, god, player)
}

type Trace = Vec<Vec<String>>;

fn run(road: Road, arm: Arm) -> Trace {
    let (mut app, god, player) = world(road, arm);
    let mut trace = Vec::new();
    for tick in 0..TICKS {
        // The target walks; the pattern scripts the moves.
        let walk = ae::Vec2::new(500.0 + 300.0 * (tick as f32 * 0.02).sin(), 870.0);
        app.world_mut().get_mut::<BodyKinematics>(player).unwrap().pos = walk;
        app.world_mut().get_mut::<ActorTarget>(god).unwrap().pos = walk;
        *app.world_mut().get_mut::<BossAttackState>(god).unwrap() = attack_at(tick);
        if tick == DIES_AT {
            *app.world_mut().get_mut::<ambition_characters::actor::BodyHealth>(god).unwrap() =
                ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health {
                    current: 0,
                    max: 18,
                    invulnerable: Default::default(),
                });
        }
        // A driven god is moved by its participant between ticks.
        if arm == Arm::Driven {
            let mut kin = app.world_mut().get_mut::<BodyKinematics>(god).unwrap();
            kin.pos.x += 1.5;
            kin.vel = ae::Vec2::new(90.0, 0.0);
            kin.facing = if (tick / 40) % 2 == 0 { 1.0 } else { -1.0 };
        }
        app.world_mut().run_schedule(Sim);
        app.world_mut().flush();
        trace.push(observe(&mut app, god, arm));
        app.world_mut().resource_mut::<ambition_time::SimTick>().0 += 1;
    }
    trace
}

fn observe(app: &mut App, god: Entity, arm: Arm) -> Vec<String> {
    let mut out = Vec::new();
    let world = app.world_mut();
    let kin = *world.get::<BodyKinematics>(god).unwrap();
    out.push(format!("god {:?} {:?} aabb {:?}", kin.pos, kin.vel, world.get::<ae::CenteredAabb>(god).unwrap().center));
    out.push(format!("pose owned {}", world.get::<ae::PoseOwnedExternally>(god).is_some()));
    out.push(format!(
        "control facing {:?}",
        world.get::<ambition_characters::control::ActorControl>(god).unwrap().0.facing
    ));
    out.push(format!("row {:?}", world.get::<ambition_platformer2d::sprite_sheet::character::PinnedRow>(god)));
    let mut hitboxes: Vec<String> = world
        .query::<(&Hitbox, &HitboxLifetime, Option<&Name>)>()
        .iter(world)
        .map(|(h, life, name)| {
            let follows = matches!(h.anchor, ambition_combat::strike::HitboxAnchor::FollowOwner { .. });
            let source = if arm == Arm::Driven { "masked".to_string() } else { format!("{:?}", h.source) };
            format!(
                "hitbox {} {source} {:?} {:?} {:?} {} {:?} {:?} {:?} {}",
                if follows { name.map_or(String::new(), |n| n.to_string()) } else { "shock".into() },
                h.anchor,
                h.half_extent,
                h.shape,
                h.damage,
                h.knockback,
                h.launch_dir,
                h.owner == god,
                life.remaining_s
            )
        })
        .collect();
    hitboxes.sort();
    out.extend(hitboxes);
    for mut request in world.resource_mut::<Messages<ProjectileSpawnRequest>>().drain() {
        request.owner = Entity::PLACEHOLDER;
        out.push(format!("{request:?}"));
    }
    for effect in world.resource_mut::<Messages<ambition_vfx::EffectRequest>>().drain() {
        match effect.effect {
            ambition_vfx::Effect::Summon(spec) => out.push(format!(
                "summon {:?} {:?} {} {} {:?} {:?} {:?} {}",
                spec.pos,
                spec.half_size,
                spec.character_id,
                spec.encounter_id,
                spec.faction,
                spec.ridden_by_summoner,
                spec.health,
                spec.keeps_contact_damage
            )),
            _ => out.push("effect (not a summon)".to_string()),
        }
    }
    for vfx in world.resource_mut::<Messages<ambition_vfx::vfx::VfxInRoom>>().drain().map(|m| m.vfx) {
        out.push(format!("vfx {vfx:?}"));
    }
    let mut sounds: Vec<String> = world
        .resource_mut::<Messages<ambition_sfx::OwnedSfxMessage>>()
        .drain()
        .map(|s| format!("sfx {:?} {:?}", s.source, s.request))
        .collect();
    sounds.sort();
    out.extend(sounds);
    out
}

fn count(trace: &Trace, what: &str) -> usize {
    trace.iter().flatten().filter(|l| l.contains(what)).count()
}

fn assert_same(module: &Trace, native: &Trace, what: &str) {
    for (tick, (m, n)) in module.iter().zip(native).enumerate() {
        assert_eq!(m, n, "tick {tick}: {what}");
    }
}

#[test]
fn the_fsm_module_conducts_the_god_as_its_native_conductor_did() {
    for arm in [Arm::Alone, Arm::Driven, Arm::Enraged] {
        let native = run(Road::NativeSystem, arm);
        // The premise: the reference performed every move.
        for what in ["fsm_lash", "fsm_pulse", "fsm_grasp", "fsm_sting", "shock", "summon", "meatball", "\"dive\""] {
            assert!(count(&native, what) > 0, "{arm:?}: the reference never showed {what}");
        }
        let module = run(Road::Module, arm);
        assert_same(&module, &native, &format!("{arm:?}: the linked module"));
    }
}

#[test]
fn the_fsm_wasm_build_conducts_the_god_as_its_native_conductor_did() {
    let native = run(Road::NativeSystem, Arm::Alone);
    let wasm = run(Road::Wasm, Arm::Alone);
    // The guest's `sqrt`, `sin` and `cos` may differ in the last bit: floats
    // are compared at three places, as the boss specials' WASM arms do.
    let quantize = crate::bosses::specials::module_parity_tests::quantize;
    let (wasm, native) = (quantize(&vec_of(&wasm)), quantize(&vec_of(&native)));
    for (tick, (w, n)) in wasm.iter().zip(&native).enumerate() {
        assert_eq!(w, n, "tick {tick}: the WASM build");
    }
}

/// The quantizer's shape: one map per tick.
fn vec_of(trace: &Trace) -> Vec<std::collections::BTreeMap<usize, Vec<String>>> {
    trace.iter().map(|t| std::iter::once((0, t.clone())).collect()).collect()
}
