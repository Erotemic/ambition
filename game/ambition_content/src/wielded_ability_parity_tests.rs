//! Each migrated wielded ability, run three ways on the same inputs: its old
//! NATIVE system (`wielded_ability_reference_tests`), its procedural module
//! linked into this process, and the same module built for
//! `wasm32-unknown-unknown`.
//!
//! Each road must make the same damage boxes and projectile requests, pay the
//! same mana and play the same sounds, tick for tick, for: a driven body and
//! a body a brain drives; gravity down, sideways and up; an aim stick, a
//! movement stick and no stick; a body with too little mana and one with no
//! mana pool. The WASM road compares floats to 1e-3 (the guest's `atan2`,
//! `sin` and `cos` differ in the last bit).
//!
//! ⚠ Deliberately different, and so not compared: the native boxes carried an
//! inspector name ("Shockwave AOE", "Focus Beam"); a module's box has none.

use std::collections::BTreeMap;

use ambition_characters::control::{ActorControl, DrivingParticipant, PlayerSlot};
use ambition_combat::held_items::HeldItem;
use ambition_extension_host::{ExtensionAppExt, ExtensionHostPlugin};
use ambition_platformer2d_core::{self as ae, BodyKinematics};
use ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame;
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use ambition_projectiles::ProjectileSpawnRequest;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;

#[derive(Clone, Copy, PartialEq)]
enum Road {
    NativeSystem,
    Module,
    Wasm,
}

struct Ability {
    item: &'static str,
    native: fn(&mut App),
}

const SHOCKWAVE: Ability = Ability {
    item: "shockwave",
    native: |app| {
        app.add_systems(Sim, super::wielded_ability_reference_tests::shockwave::fire_shockwave_system);
    },
};
const BEAM: Ability = Ability {
    item: "beam",
    native: |app| {
        app.add_systems(Sim, super::wielded_ability_reference_tests::beam::fire_beam_system);
    },
};
const VOLLEY: Ability = Ability {
    item: "volley",
    native: |app| {
        app.add_systems(Sim, super::wielded_ability_reference_tests::volley::fire_volley_system);
    },
};
const METEOR: Ability = Ability {
    item: "meteor",
    native: |app| {
        app.add_systems(Sim, super::wielded_ability_reference_tests::meteor::fire_meteor_system);
    },
};

const ALL: [&Ability; 4] = [&SHOCKWAVE, &BEAM, &VOLLEY, &METEOR];

/// One body: who drives it, its gravity, and its mana (`None`: no pool).
struct BodySpec {
    driven: bool,
    gravity: ae::Vec2,
    mana: Option<f32>,
}

const BODIES: [BodySpec; 5] = [
    BodySpec { driven: true, gravity: ae::Vec2::new(0.0, 1.0), mana: Some(100.0) },
    BodySpec { driven: true, gravity: ae::Vec2::new(1.0, 0.0), mana: Some(100.0) },
    BodySpec { driven: false, gravity: ae::Vec2::new(0.0, -1.0), mana: Some(100.0) },
    // Too little for any use.
    BodySpec { driven: true, gravity: ae::Vec2::new(0.0, 1.0), mana: Some(10.0) },
    BodySpec { driven: true, gravity: ae::Vec2::new(-1.0, 0.0), mana: None },
];

/// What each body presses on one tick: (attack, shield, aim, movement).
type Press = (bool, bool, [f32; 2], [f32; 2]);

fn press_on(tick: usize, body: usize) -> Press {
    let attack = (tick + body) % 3 == 0;
    let shield = tick == 6 && body == 0;
    let aim = match (tick + body) % 4 {
        0 => [0.0, 0.0],
        1 => [0.0, -1.0],
        2 => [-0.8, 0.3],
        _ => [0.9, 0.9],
    };
    let movement = if tick % 5 == 0 { [-1.0, 0.0] } else { [0.0, 0.0] };
    (attack, shield, aim, movement)
}

const TICKS: usize = 12;

fn wasm_modules() -> (
    std::sync::Arc<ambition_extension_wasm::WasmModules>,
    Vec<ambition_extension_sdk::ModuleDescriptor>,
) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = ambition_extension_wasm::build_module_crate(&root, "ambition_content_modules")
        .expect("the module crate builds for wasm32-unknown-unknown");
    ambition_extension_wasm::WasmModules::load(&std::fs::read(path).unwrap()).expect("the module file loads")
}

fn world(road: Road, ability: &Ability) -> (App, Vec<Entity>) {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_message::<ProjectileSpawnRequest>()
        .add_message::<ambition_vfx::EffectRequest>()
        .add_message::<ambition_sfx::OwnedSfxMessage>()
        .add_message::<ambition_characters::brain::ActorActionMessage>()
        .init_resource::<ambition_time::SimTick>()
        .init_resource::<ambition_time::WorldTime>();
    match road {
        Road::NativeSystem => (ability.native)(&mut app),
        Road::Module | Road::Wasm => {
            app.add_plugins(ExtensionHostPlugin::new(Sim));
            // Every module is declared, so every port the game offers is.
            ambition_platformer2d_runtime::extension_composition::install_ports(&mut app);
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
    let mut bodies = Vec::new();
    for (i, spec) in BODIES.iter().enumerate() {
        let held = ambition_characters::brain::held_item_by_id(ability.item).expect("a known item");
        let mut frame = ResolvedMotionFrame::default();
        frame.publish_resolved_frame(ae::MotionFrame::from_direction(spec.gravity, 900.0));
        let mut entity = app.world_mut().spawn((
            BodyKinematics {
                pos: ae::Vec2::new(200.0 + 50.0 * i as f32, 300.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: if i % 2 == 0 { 1.0 } else { -1.0 },
            },
            ActorControl::default(),
            HeldItem::new(held),
            frame,
            ambition_characters::actor::ActorFaction::Player,
            SimId::placement(&format!("wielder_{i}")),
        ));
        if spec.driven {
            entity.insert(DrivingParticipant(PlayerSlot(i as u8)));
        }
        if let Some(level) = spec.mana {
            let mut bank = ambition_platformer2d::abilities::mana::bank();
            assert!(ambition_platformer2d::abilities::mana::spend(Some(&mut bank), 100.0 - level));
            entity.insert(bank);
        }
        bodies.push(entity.id());
    }
    app.insert_resource(ControlledSubject(None));
    (app, bodies)
}

/// Each tick's output by body index: requests, effects, sounds, then mana.
type Trace = Vec<BTreeMap<usize, Vec<String>>>;

fn run(road: Road, ability: &Ability) -> Trace {
    let (mut app, bodies) = world(road, ability);
    let index = |e: Entity| bodies.iter().position(|b| *b == e).expect("a body");
    let mut trace = Vec::new();
    for tick in 0..TICKS {
        for (i, body) in bodies.iter().enumerate() {
            let (attack, shield, aim, movement) = press_on(tick, i);
            let mut control = app.world_mut().get_mut::<ActorControl>(*body).unwrap();
            control.0.melee_pressed = attack;
            control.0.shield_held = shield;
            control.0.aim = ae::LocalAxes::new(aim[0], aim[1]);
            control.0.locomotion = ae::LocalAxes::new(movement[0], movement[1]);
        }
        app.world_mut().run_schedule(Sim);
        let mut out: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        for mut request in app.world_mut().resource_mut::<Messages<ProjectileSpawnRequest>>().drain() {
            let i = index(request.owner);
            request.owner = Entity::PLACEHOLDER;
            out.entry(i).or_default().push(format!("{request:?}"));
        }
        for effect in app.world_mut().resource_mut::<Messages<ambition_vfx::EffectRequest>>().drain() {
            let i = index(effect.owner);
            let ambition_vfx::Effect::DamageBox(b) = effect.effect else {
                panic!("only damage boxes here");
            };
            out.entry(i).or_default().push(format!(
                "DamageBox {{ center: {:?}, faction: {:?}, half_extent: {:?}, damage: {}, knockback: {:?}, lifetime_s: {:?} }}",
                b.center, b.faction, b.half_extent, b.damage, b.knockback, b.lifetime_s
            ));
        }
        let sounds: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<ambition_sfx::OwnedSfxMessage>>()
            .drain()
            .collect();
        for sound in sounds {
            out.entry(usize::MAX).or_default().push(format!("{:?}", sound.request));
        }
        for (i, body) in bodies.iter().enumerate() {
            let mana = ambition_platformer2d::abilities::mana::level(
                app.world().get::<ambition_platformer2d_core::resources::ActorResources>(*body),
            )
            .map(|l| l.current);
            out.entry(i).or_default().push(format!("mana {mana:?}"));
        }
        trace.push(out);
        app.world_mut().resource_mut::<ambition_time::SimTick>().0 += 1;
    }
    trace
}

fn uses(trace: &Trace) -> usize {
    trace
        .iter()
        .flat_map(|t| t.values())
        .flatten()
        .filter(|line| !line.starts_with("mana"))
        .count()
}

#[test]
fn each_module_uses_its_item_as_its_native_system_did() {
    for ability in ALL {
        let native = run(Road::NativeSystem, ability);
        assert!(uses(&native) > 0, "{}: the premise: the reference fired", ability.item);
        let module = run(Road::Module, ability);
        assert_eq!(module, native, "{}: the linked module", ability.item);
    }
}

#[test]
fn each_wasm_build_uses_its_item_as_its_native_system_did() {
    for ability in ALL {
        let native = run(Road::NativeSystem, ability);
        let wasm = run(Road::Wasm, ability);
        assert_eq!(
            crate::bosses::specials::module_parity_tests::quantize(&wasm),
            crate::bosses::specials::module_parity_tests::quantize(&native),
            "{}",
            ability.item
        );
    }
}
