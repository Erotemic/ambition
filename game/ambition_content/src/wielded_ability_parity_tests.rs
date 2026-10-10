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

/// Blink, the first body-motion module (`ambition.motion.transit`), run the
/// same three ways. Its output is a moved body, so the trace is each body's
/// position, velocity and movement cooldown, the Class-B record, the
/// strikes, the effects and the sounds, tick for tick.
///
/// Two live rooms: #0 open, #1 with a wall whose left side is at x = 380.
/// The bodies: a driven body in #1 that blinks into the wall; driven bodies
/// in gravity sideways and up in #0; a body a brain drives (it does not
/// blink); and a driven body that does not move by the swept kernel (it does
/// not blink either). The presses come faster than the cooldown, so some are
/// refused.
mod blink {
    use super::*;
    use ambition_extension_host::ExtensionSet;
    use ambition_extension_sdk::phases::WIELDED_USE;
    use ambition_platformer2d_shared_tangle::class_b::ClassBRemapLog;
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_live_room_component, spawn_live_room, InRoomInstance, LiveRoomInstance,
    };

    struct BlinkBody {
        driven: bool,
        gravity: ae::Vec2,
        room: usize,
        swept: bool,
    }

    const BODIES: [BlinkBody; 5] = [
        BlinkBody { driven: true, gravity: ae::Vec2::new(0.0, 1.0), room: 1, swept: true },
        BlinkBody { driven: true, gravity: ae::Vec2::new(1.0, 0.0), room: 0, swept: true },
        BlinkBody { driven: false, gravity: ae::Vec2::new(0.0, 1.0), room: 1, swept: true },
        BlinkBody { driven: true, gravity: ae::Vec2::new(0.0, -1.0), room: 0, swept: true },
        BlinkBody { driven: true, gravity: ae::Vec2::new(0.0, 1.0), room: 1, swept: false },
    ];

    /// More than the cooldown (0.45 s, 27 ticks), so a press is refused and a
    /// later one is not.
    const TICKS: usize = 70;

    fn press_on(tick: usize, body: usize) -> Press {
        let attack = (tick + 2 * body) % 9 == 0;
        let aim = match (tick / 9 + body) % 4 {
            0 => [0.0, 0.0],
            1 => [0.9, 0.0],
            2 => [-0.8, 0.3],
            _ => [0.9, 0.9],
        };
        let movement = if tick % 5 == 1 { [-1.0, 0.0] } else { [0.0, 0.0] };
        (attack, false, aim, movement)
    }

    fn room(blocks: Vec<ae::Block>) -> ambition_platformer2d_core::RoomGeometry {
        ambition_platformer2d_core::RoomGeometry(ae::World::new(
            "blink_parity",
            ae::Vec2::new(1000.0, 600.0),
            ae::Vec2::new(100.0, 300.0),
            blocks,
        ))
    }

    fn world(road: Road) -> (App, Vec<Entity>) {
        let mut app = App::new();
        app.init_schedule(Sim);
        app.add_message::<ProjectileSpawnRequest>()
            .add_message::<ambition_vfx::EffectRequest>()
            .add_message::<ambition_vfx::vfx::VfxInRoom>()
            .add_message::<ambition_combat::events::HitEvent>()
            .add_message::<ambition_sfx::OwnedSfxMessage>()
            .add_message::<ambition_characters::brain::ActorActionMessage>()
            .init_resource::<ambition_time::SimTick>()
            .init_resource::<ambition_time::WorldTime>()
            .init_resource::<ClassBRemapLog>();
        // The cooldown runs on simulation time: 60 ticks a second.
        app.world_mut().resource_mut::<ambition_time::WorldTime>().set_sim_dt(1.0 / 60.0);
        let tick_cooldowns = ambition_platformer2d::abilities::ability_cooldown::tick_ability_cooldown;
        match road {
            // The shipped order: the native chain, then the cooldowns tick.
            Road::NativeSystem => {
                app.add_systems(
                    Sim,
                    (super::super::wielded_ability_reference_tests::blink::blink_system, tick_cooldowns).chain(),
                );
            }
            // The shipped order: `wielded_use`, then the cooldowns tick.
            Road::Module | Road::Wasm => {
                app.add_plugins(ExtensionHostPlugin::new(Sim));
                ambition_platformer2d_runtime::extension_composition::install_ports(&mut app);
                app.add_systems(Sim, tick_cooldowns.after(ExtensionSet::Lower(WIELDED_USE)));
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
        insert_live_room_component(app.world_mut(), room(Vec::new()));
        let walled = LiveRoomInstance::ACTIVATION.next();
        spawn_live_room(
            app.world_mut(),
            walled,
            room(vec![ae::Block::solid("wall", ae::Vec2::new(380.0, 0.0), ae::Vec2::new(20.0, 600.0))]),
        );
        let rooms = [LiveRoomInstance::ACTIVATION, walled];
        let held = ambition_characters::brain::held_item_by_id("blink").expect("a known item");
        let mut bodies = Vec::new();
        for (i, spec) in BODIES.iter().enumerate() {
            let pos = ae::Vec2::new(150.0 + 20.0 * i as f32, 300.0);
            let mut frame = ResolvedMotionFrame::default();
            frame.publish_resolved_frame(ae::MotionFrame::from_direction(spec.gravity, 900.0));
            let model = if spec.swept {
                ambition_platformer2d_core::movement::MotionModel::default()
            } else {
                ambition_platformer2d_core::movement::MotionModel::surface_momentum(Default::default())
            };
            let mut entity = app.world_mut().spawn((
                BodyKinematics {
                    pos,
                    vel: ae::Vec2::new(30.0, -10.0),
                    size: ae::Vec2::new(24.0, 40.0),
                    facing: if i % 2 == 0 { 1.0 } else { -1.0 },
                },
                ActorControl::default(),
                HeldItem::new(held.clone()),
                model,
                ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle::from_scratch(
                    ae::BodyClusterScratch::new_with_abilities(pos, ae::AbilitySet::default()),
                ),
                ambition_characters::actor::ActorFaction::Player,
                SimId::placement(&format!("blinker_{i}")),
                InRoomInstance(rooms[spec.room]),
            ));
            // The bundle's own frame is replaced by the one with this gravity.
            entity.insert(frame);
            if spec.driven {
                entity.insert(DrivingParticipant(PlayerSlot(i as u8)));
            }
            bodies.push(entity.id());
        }
        app.insert_resource(ControlledSubject(None));
        (app, bodies)
    }

    fn run(road: Road) -> Trace {
        let (mut app, bodies) = world(road);
        let index = |e: Entity| bodies.iter().position(|b| *b == e).map_or(usize::MAX, |i| i);
        let mut trace = Vec::new();
        for tick in 0..TICKS {
            app.world_mut().resource_mut::<ClassBRemapLog>().clear();
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
            for hit in app.world_mut().resource_mut::<Messages<ambition_combat::events::HitEvent>>().drain() {
                let i = hit.attacker.map_or(usize::MAX, index);
                out.entry(i).or_default().push(format!(
                    "Hit {{ volume: {:?}, damage: {}, source: {:?}, mode: {:?}, knockback: {:?}, room: {:?} }}",
                    hit.volume, hit.damage, hit.source, hit.mode, hit.knockback, hit.room
                ));
            }
            for vfx in app.world_mut().resource_mut::<Messages<ambition_vfx::vfx::VfxInRoom>>().drain() {
                out.entry(usize::MAX).or_default().push(format!("{:?} {:?}", vfx.room, vfx.vfx));
            }
            let sounds: Vec<_> = app
                .world_mut()
                .resource_mut::<Messages<ambition_sfx::OwnedSfxMessage>>()
                .drain()
                .collect();
            for sound in sounds {
                out.entry(usize::MAX).or_default().push(format!("{:?}", sound.request));
            }
            let remaps: Vec<_> = app
                .world()
                .resource::<ClassBRemapLog>()
                .entries()
                .iter()
                .map(|entry| (index(entry.body), format!("remap {:?}", entry.kind)))
                .collect();
            for (i, remap) in remaps {
                out.entry(i).or_default().push(remap);
            }
            for (i, body) in bodies.iter().enumerate() {
                let kin = app.world().get::<BodyKinematics>(*body).unwrap();
                let cooldown = app
                    .world()
                    .get::<ambition_platformer2d::abilities::ability_cooldown::AbilityCooldown>(*body)
                    .map(|c| c.remaining);
                out.entry(i)
                    .or_default()
                    .push(format!("at {:?} moving {:?} cooldown {cooldown:?}", kin.pos, kin.vel));
            }
            trace.push(out);
            app.world_mut().resource_mut::<ambition_time::SimTick>().0 += 1;
        }
        trace
    }

    /// How many times each body was moved: each transit is a Class-B remap.
    fn blinks(trace: &Trace) -> Vec<usize> {
        (0..BODIES.len())
            .map(|i| trace.iter().flat_map(|t| t[&i].iter()).filter(|line| line.starts_with("remap")).count())
            .collect()
    }

    #[test]
    fn the_blink_module_moves_each_body_as_its_native_system_did() {
        let native = run(Road::NativeSystem);
        let moved = blinks(&native);
        // ⭐ The premise: the driven swept bodies blinked more than once (the
        // cooldown ran out), and the brain's body and the body off the swept
        // kernel did not.
        assert!(moved[0] >= 2 && moved[1] >= 2 && moved[3] >= 2, "the reference blinked: {moved:?}");
        assert_eq!((moved[2], moved[4]), (0, 0), "{moved:?}");
        // The premise of the wall: a blink of the body in #1 stopped short of
        // the wall at x = 380 (its right side at the wall, less a margin), not
        // 150 px on.
        let x = |line: &str| -> Option<f32> { line.strip_prefix("at Vec2(")?.split(',').next()?.parse().ok() };
        let xs: Vec<f32> = native.iter().filter_map(|t| t[&0].last().and_then(|s| x(s))).collect();
        assert!(
            xs.windows(2).any(|w| w[1] > w[0] && w[1] - w[0] < 149.0 && w[1] + 12.0 <= 380.0 && w[1] + 12.0 > 360.0),
            "no blink of the body in #1 stopped at the wall: {xs:?}"
        );
        let module = run(Road::Module);
        assert_eq!(module, native, "the linked module");
    }

    #[test]
    fn the_blink_wasm_build_moves_each_body_as_its_native_system_did() {
        let native = run(Road::NativeSystem);
        let wasm = run(Road::Wasm);
        assert_eq!(
            crate::bosses::specials::module_parity_tests::quantize(&wasm),
            crate::bosses::specials::module_parity_tests::quantize(&native),
        );
    }
}
