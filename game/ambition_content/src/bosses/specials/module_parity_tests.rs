//! Each migrated boss technique, run three ways on the same inputs:
//!
//! 1. its old NATIVE system (kept test-only, `*_reference_tests.rs`): the
//!    reference trace;
//! 2. its procedural MODULE, linked into this process, on the extension host;
//! 3. the same module built for `wasm32-unknown-unknown` and run by the
//!    interpreter.
//!
//! Each road must make the same projectile requests, with the same owner and
//! move-use credit, tick for tick. The WASM road compares floats to 1e-3: the
//! guest's `sin`, `cos` and `atan2` are its own compiled code and differ in
//! the last f32 bit (measured on the echo fan: -130.4897 against -130.48969).

use std::collections::BTreeMap;

use ambition_boss_encounter::{BossClusterScratch, BossConfig};
use ambition_characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_characters::brain::{ActorActionMessage, BossAttackProfile, BossAttackState};
use ambition_combat::components::ActorTarget;
use ambition_extension_host::{ExtensionAppExt, ExtensionHostPlugin};
use ambition_platformer2d::actor::FeatureSimEntity;
use ambition_platformer2d_core::{self as ae, BodyKinematics};
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
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

/// One migrated technique: its key and its old native system.
struct Technique {
    key: &'static str,
    native: fn(&mut App),
}

const ECHO_FAN: Technique = Technique {
    key: "echo_fan",
    native: |app| {
        use super::echo_fan_reference_tests as r;
        app.register_required_components::<BossConfig, r::EchoFanState>();
        app.add_systems(Sim, r::spawn_echo_fan_from_special_messages);
    },
};

const EYE_BEAM: Technique = Technique {
    key: "eye_beam",
    native: |app| {
        use super::eye_beam_reference_tests as r;
        app.register_required_components::<BossConfig, r::EyeBeamState>();
        app.add_systems(Sim, r::spawn_eye_beam_from_special_messages);
    },
};

const SEISMIC_STOMP: Technique = Technique {
    key: "seismic_stomp",
    native: |app| {
        use super::seismic_stomp_reference_tests as r;
        app.register_required_components::<BossConfig, r::SeismicStompState>();
        app.add_systems(Sim, r::spawn_seismic_stomp_from_special_messages);
    },
};

const GRADIENT_NOVA: Technique = Technique {
    key: "gradient_nova",
    native: |app| {
        use super::gradient_nova_reference_tests as r;
        app.register_required_components::<BossConfig, r::ExplodingGradientState>();
        app.add_systems(Sim, r::spawn_gradient_nova_from_special_messages);
    },
};

/// Every migrated technique.
const ALL: [&Technique; 5] = [&ECHO_FAN, &EYE_BEAM, &GRADIENT_NOVA, &MODE_COLLAPSE, &SEISMIC_STOMP];

const MODE_COLLAPSE: Technique = Technique {
    key: "mode_collapse_converge",
    native: |app| {
        use super::mode_collapse_reference_tests as r;
        app.register_required_components::<BossConfig, r::ModeCollapseState>();
        app.add_systems(Sim, r::spawn_mode_collapse_converge_from_special_messages);
    },
};

/// One tick of input. `press`: which bosses press the key, with which move
/// use. `telegraph`: which bosses' patterns telegraph the key.
#[derive(Clone, Copy)]
struct Tick {
    press: &'static [(usize, Option<u32>)],
    telegraph: &'static [usize],
}

const fn press(press: &'static [(usize, Option<u32>)]) -> Tick {
    Tick {
        press,
        telegraph: &[],
    }
}

const fn telegraph(telegraph: &'static [usize]) -> Tick {
    Tick { press: &[], telegraph }
}

const IDLE: Tick = press(&[]);

/// Each tick's requests, by boss index, in emission order.
type Trace = Vec<BTreeMap<usize, Vec<String>>>;

fn wasm_modules() -> (
    std::sync::Arc<ambition_extension_wasm::WasmModules>,
    Vec<ambition_extension_sdk::ModuleDescriptor>,
) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = ambition_extension_wasm::build_module_crate(&root, "ambition_content_modules")
        .expect("the module crate builds for wasm32-unknown-unknown");
    ambition_extension_wasm::WasmModules::load(&std::fs::read(path).unwrap())
        .expect("the module file loads")
}

fn world(road: Road, technique: &Technique) -> (App, Vec<Entity>) {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_message::<ActorActionMessage>()
        .add_message::<ProjectileSpawnRequest>()
        .add_message::<ambition_vfx::EffectRequest>()
        .init_resource::<ambition_time::SimTick>();
    match road {
        Road::NativeSystem => (technique.native)(&mut app),
        Road::Module | Road::Wasm => {
            app.add_plugins(ExtensionHostPlugin::new(Sim));
            ambition_boss_encounter::extension::install(&mut app);
            ambition_projectiles::extension::install(&mut app);
            ambition_combat::extension::install(&mut app);
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
    let catalog = crate::bosses::authored_boss_catalog();
    let player = app
        .world_mut()
        .spawn((
            BodyKinematics {
                pos: ae::Vec2::new(40.0, -30.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(20.0, 30.0),
                facing: 1.0,
            },
            PlayerEntity,
        ))
        .id();
    let mut bosses = Vec::new();
    for (i, target) in [
        Some(ActorTarget {
            entity: Some(player),
            pos: ae::Vec2::new(999.0, 999.0),
        }),
        None,
        Some(ActorTarget {
            entity: None,
            pos: ae::Vec2::new(-300.0, 10.0),
        }),
    ]
    .into_iter()
    .enumerate()
    {
        let mut boss = BossClusterScratch::new(
            &catalog,
            format!("boss_{i}"),
            "Mockingbird",
            ae::Aabb::new(ae::Vec2::new(100.0 * i as f32, 50.0), ae::Vec2::new(30.0, 30.0)),
            ambition_entity_catalog::placements::BossBrain::Dormant,
        );
        boss.kin.facing = if i == 1 { -1.0 } else { 1.0 };
        // A projectile offset with an x part, so a technique that mirrors it
        // by facing and one that does not give different origins.
        boss.config.behavior.projectile_origin_offset = ae::Vec2::new(12.0, 6.0);
        let mut entity = app.world_mut().spawn((
            boss.kin,
            boss.config,
            boss.status,
            boss.health,
            BossAttackState::default(),
            ambition_combat::components::ActorFaction::Boss,
            FeatureSimEntity,
        ));
        if let Some(target) = target {
            entity.insert(target);
        }
        bosses.push(entity.id());
    }
    (app, bosses)
}

/// Move the player between ticks, so a lock taken during a telegraph is
/// distinguishable from the target at the strike.
fn move_player(app: &mut App, tick: usize) {
    let mut q = app
        .world_mut()
        .query_filtered::<&mut BodyKinematics, With<PlayerEntity>>();
    for mut kin in q.iter_mut(app.world_mut()) {
        kin.pos = ae::Vec2::new(40.0 + 13.0 * tick as f32, -30.0 + 7.0 * tick as f32);
    }
}

fn run(road: Road, technique: &Technique, ticks: &[Tick], kill_boss_0_at: Option<usize>) -> Trace {
    let (mut app, bosses) = world(road, technique);
    let mut trace = Vec::new();
    for (index, tick) in ticks.iter().enumerate() {
        move_player(&mut app, index);
        if kill_boss_0_at == Some(index) {
            app.world_mut()
                .get_mut::<ambition_characters::actor::BodyHealth>(bosses[0])
                .unwrap()
                .health
                .current = 0;
        }
        for (i, boss) in bosses.iter().enumerate() {
            let telegraphing = tick.telegraph.contains(&i);
            app.world_mut()
                .get_mut::<BossAttackState>(*boss)
                .unwrap()
                .telegraph_profile =
                telegraphing.then(|| BossAttackProfile::Special(technique.key.into()));
        }
        for (boss, occurrence) in tick.press {
            app.world_mut().write_message(ActorActionMessage {
                actor: bosses[*boss],
                request: ActionRequest::Special {
                    spec: SpecialActionSpec::Special(technique.key.into()),
                    params: Default::default(),
                },
                move_instance: *occurrence,
            });
        }
        app.world_mut().run_schedule(Sim);
        let mut by_boss: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        for mut request in app
            .world_mut()
            .resource_mut::<Messages<ProjectileSpawnRequest>>()
            .drain()
        {
            let boss = bosses
                .iter()
                .position(|b| *b == request.owner)
                .expect("only a boss fires");
            // The two Apps allocate entities differently.
            request.owner = Entity::PLACEHOLDER;
            by_boss.entry(boss).or_default().push(format!("{request:?}"));
        }
        for effect in app
            .world_mut()
            .resource_mut::<Messages<ambition_vfx::EffectRequest>>()
            .drain()
        {
            let boss = bosses
                .iter()
                .position(|b| *b == effect.owner)
                .expect("only a boss emits");
            let ambition_vfx::Effect::DamageBox(b) = effect.effect else {
                panic!("only damage boxes are emitted here");
            };
            by_boss.entry(boss).or_default().push(format!(
                "DamageBox {{ center: {:?}, faction: {:?}, half_extent: {:?}, damage: {}, knockback: {:?}, lifetime_s: {:?}, name: {:?} }}",
                b.center, b.faction, b.half_extent, b.damage, b.knockback, b.lifetime_s, b.name
            ));
        }
        trace.push(by_boss);
        app.world_mut()
            .resource_mut::<Messages<ActorActionMessage>>()
            .update();
        app.world_mut().resource_mut::<ambition_time::SimTick>().0 += 1;
    }
    trace
}

/// Strikes without a telegraph: the fire-once-per-strike rule.
const STRIKES: &[Tick] = &[
    IDLE,
    // A strike over three ticks fires once, on its first tick.
    press(&[(0, Some(4))]),
    press(&[(0, Some(4))]),
    press(&[(0, Some(4))]),
    // A gap ends it; the next press is a new strike.
    IDLE,
    press(&[(0, None), (1, Some(9))]),
    // Two presses of one key in one tick: the last move use counts.
    press(&[(2, Some(1)), (2, Some(2))]),
    press(&[(1, Some(9))]),
    IDLE,
    press(&[(1, None), (2, None)]),
];

/// Telegraphs before strikes: a lock is taken on the first telegraph tick and
/// the strike fires at it, though the player has moved on.
const LOCKED: &[Tick] = &[
    telegraph(&[0, 2]),
    telegraph(&[0, 2]),
    telegraph(&[0]),
    press(&[(0, Some(3)), (2, None)]),
    press(&[(0, Some(3))]),
    IDLE,
    // A strike with no telegraph fires at the live target.
    press(&[(0, None), (1, None)]),
    telegraph(&[1]),
    press(&[(1, Some(5))]),
];

fn fired(trace: &Trace) -> usize {
    trace.iter().map(|t| t.values().map(Vec::len).sum::<usize>()).sum()
}

fn assert_module_matches_native(technique: &Technique, ticks: &[Tick], kill: Option<usize>) {
    let native = run(Road::NativeSystem, technique, ticks, kill);
    assert!(fired(&native) > 0, "{}: the premise: the reference fired", technique.key);
    let module = run(Road::Module, technique, ticks, kill);
    assert_eq!(module, native, "{}: the linked module", technique.key);
}

#[test]
fn each_module_emits_what_its_native_system_emitted() {
    for technique in ALL {
        assert_module_matches_native(technique, STRIKES, None);
        assert_module_matches_native(technique, LOCKED, None);
    }
}

#[test]
fn a_boss_that_dies_mid_strike_matches_its_native_system() {
    let ticks: &[Tick] = &[
        telegraph(&[0]),
        press(&[(0, None)]),
        press(&[(0, None)]),
        press(&[(0, None)]),
        IDLE,
        press(&[(0, None)]),
    ];
    for technique in ALL {
        assert_module_matches_native(technique, ticks, Some(2));
    }
}

#[test]
fn the_locked_target_is_the_one_taken_during_the_telegraph() {
    // The premise of the LOCKED arms: the lock changes what fires. With the
    // player moving every tick, a beam fired at the live target lands
    // elsewhere than one fired at the lock.
    let locked = run(Road::Module, &EYE_BEAM, LOCKED, None);
    let unlocked: Vec<Tick> = LOCKED
        .iter()
        .map(|t| Tick {
            press: t.press,
            telegraph: &[],
        })
        .collect();
    let leaked: &'static [Tick] = Box::leak(unlocked.into_boxed_slice());
    assert_ne!(run(Road::Module, &EYE_BEAM, leaked, None), locked);
}

#[test]
fn each_wasm_build_emits_what_its_native_system_emitted() {
    for technique in ALL {
        for ticks in [STRIKES, LOCKED] {
            let native = run(Road::NativeSystem, technique, ticks, None);
            let wasm = run(Road::Wasm, technique, ticks, None);
            assert_eq!(quantize(&wasm), quantize(&native), "{}", technique.key);
        }
    }
}

/// Each trace with every decimal literal rounded to 1e-3.
fn quantize(trace: &Trace) -> Trace {
    trace
        .iter()
        .map(|tick| {
            tick.iter()
                .map(|(b, shots)| (*b, shots.iter().map(|s| round_floats(s)).collect()))
                .collect()
        })
        .collect()
}

/// Replace every decimal literal `-?\d+\.\d+` with the same value at three
/// places.
fn round_floats(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let neg = bytes[i] == b'-' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit();
        let mut j = if neg { i + 1 } else { i };
        let at_word_start =
            start == 0 || !(bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_');
        if j < bytes.len() && bytes[j].is_ascii_digit() && at_word_start {
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j + 1 < bytes.len() && bytes[j] == b'.' && bytes[j + 1].is_ascii_digit() {
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                let v: f64 = s[start..j].parse().unwrap();
                out.push_str(&format!("{v:.3}"));
                i = j;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}
