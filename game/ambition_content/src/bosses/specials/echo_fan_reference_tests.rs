//! The NATIVE REFERENCE of the Mockingbird echo fan — test-only.
//!
//! Production runs the procedural module `ambition_content_modules::echo_fan`
//! on the extension host (fast-iteration I4). This file keeps the native
//! system as the reference trace the module must reproduce
//! (`module_parity` below). Do not compose it into an App: one producer.

use bevy::prelude::*;

use ambition_boss_encounter::BossClusterRef;
use ambition_characters::brain::{
    ActorActionMessage,
};
use ambition_combat::components::ActorTarget;
use ambition_platformer2d::actor::FeatureSimEntity;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
use ambition_projectiles::{ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart};

// ---- Mockingbird's echo fan (content-only, open-seam; mimic spread) ----

/// Content key for the Mockingbird echo fan — matches the `Special("echo_fan")`
/// beats in `boss_profiles.ron`.
pub const ECHO_FAN_KEY: &str = "echo_fan";

const ECHO_FAN_COUNT: u32 = 7;
const ECHO_FAN_SPREAD_RAD: f32 = 0.9; // total cone width (~52°)
const ECHO_FAN_SPEED: f32 = 300.0;
const ECHO_FAN_DAMAGE: i32 = 1;
const ECHO_FAN_HALF_EXTENT: ae::Vec2 = ae::Vec2::new(9.0, 9.0);
const ECHO_FAN_LIFETIME: f32 = 2.0;

/// Per-boss gate for the echo fan. One spread per strike.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct EchoFanState {
    pub fired_this_strike: bool,
}

/// Pure: `count` unit directions evenly fanned across a `spread` cone centered on
/// `aim` — the same shot mimicked across the fan. A single shot when `count == 1`
/// flies straight along `aim`. Deterministic — the testable core of the Technique.
fn echo_fan(aim: ae::Vec2, count: u32, spread: f32) -> Vec<ae::Vec2> {
    let n = count.max(1);
    let base = if aim.length_squared() < 1e-6 {
        0.0
    } else {
        aim.y.atan2(aim.x)
    };
    (0..n)
        .map(|i| {
            // Even spread across [-spread/2, +spread/2]; single shot → straight.
            let t = if n == 1 {
                0.0
            } else {
                (i as f32) / ((n - 1) as f32) - 0.5
            };
            let theta = base + t * spread;
            ae::Vec2::new(theta.cos(), theta.sin())
        })
        .collect()
}

/// Technique: Mockingbird echo fan — copies one shot across a cone aimed at the
/// player (content-only; open-seam special).
pub fn spawn_echo_fan_from_special_messages(
    mut projectiles: MessageWriter<ProjectileSpawnRequest>,
    mut messages: MessageReader<ActorActionMessage>,
    player_query: Query<&BodyKinematics, With<PlayerEntity>>,
    mut bosses: Query<
        (
            Entity,
            BossClusterRef,
            &ambition_characters::actor::BodyHealth,
            &mut EchoFanState,
            Option<&ActorTarget>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let firing = super::actors_firing(&mut messages, ECHO_FAN_KEY);
    for (entity, boss_feature, health, mut state, actor_target) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        if !firing.contains_key(&entity) {
            state.fired_this_strike = false;
            continue;
        }
        if !health.alive() || state.fired_this_strike {
            continue;
        }
        let origin = boss.kin.pos + boss.config.behavior.projectile_origin_offset;
        let player_pos = actor_target.and_then(|t| {
            t.entity
                .and_then(|e| player_query.get(e).ok())
                .map(|kin| kin.aabb().center())
                .or(Some(t.pos))
        });
        // Aim at the player; fall back to straight-ahead by facing if untracked.
        let aim = player_pos
            .map(|p| p - origin)
            .filter(|d| d.length_squared() > 1e-4)
            .unwrap_or_else(|| ae::Vec2::new(boss.kin.facing.signum(), 0.0));
        for dir in echo_fan(aim, ECHO_FAN_COUNT, ECHO_FAN_SPREAD_RAD) {
            projectiles.write(ProjectileSpawnRequest::open(
                entity,
                ProjectileSpawn {
                    origin,
                    dir,
                    speed: ECHO_FAN_SPEED,
                    damage: ECHO_FAN_DAMAGE,
                    max_lifetime: ECHO_FAN_LIFETIME,
                    half_extent: ECHO_FAN_HALF_EXTENT,
                    gravity: 0.0,
                    visual_id: String::new(),
                    // Straight shot: this ability authors no bounce.
                    bounces: 0,
                    bounce_on_world_contact: false,
                    splash_half_extent: 0.0,
                    boomerang_return_s: None,
                },
                ProjectileStart::StepThisTick,
            )
            .fired_by_move_if_any(firing.get(&entity).copied().flatten()),
            );
        }
        state.fired_this_strike = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_fan_spreads_evenly_around_the_aim() {
        let aim = ae::Vec2::new(1.0, 0.0); // straight right
        let fan = echo_fan(aim, 7, 0.9);
        assert_eq!(fan.len(), 7);
        for d in &fan {
            assert!((d.length() - 1.0).abs() < 1e-3, "unit dirs");
        }
        // Middle shot flies straight along the aim; ends are symmetric about it.
        let mid = fan[3];
        assert!(mid.y.abs() < 1e-3 && mid.x > 0.0, "center shot is the aim");
        assert!(
            (fan[0].y + fan[6].y).abs() < 1e-3,
            "fan symmetric about aim"
        );
        assert!(fan[0].y * fan[6].y < 0.0, "ends straddle the aim");
        // A single shot flies straight along the aim (no spread).
        let one = echo_fan(aim, 1, 0.9);
        assert_eq!(one.len(), 1);
        assert!(one[0].y.abs() < 1e-3 && one[0].x > 0.0);
    }
}

/// The module on the extension host emits the same projectile requests, with
/// the same owner and move-use credit, as this native system, tick for tick.
pub(super) mod module_parity {
    use std::collections::BTreeMap;

    use ambition_boss_encounter::{BossClusterScratch, BossConfig};
    use ambition_characters::brain::action_set::{ActionRequest, SpecialActionSpec};
    use ambition_combat::components::ActorTarget;
    use ambition_extension_host::{ExtensionAppExt, ExtensionHostPlugin};
    use ambition_platformer2d::actor::FeatureSimEntity;
    use bevy::ecs::schedule::ScheduleLabel;
    use bevy::prelude::*;

    use super::*;

    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    struct Sim;

    /// One tick of input: which boss presses `echo_fan`, with which move use.
    pub(super) type Presses = &'static [(usize, Option<u32>)];

    struct World0 {
        app: App,
        bosses: Vec<Entity>,
    }

    #[derive(Clone, Copy, PartialEq)]
    pub(super) enum Road {
        /// The old native system: the reference.
        NativeSystem,
        /// The module, linked into this process.
        Module,
        /// The module built for `wasm32-unknown-unknown` and run in wasmi.
        Wasm,
    }

    fn wasm_echo_fan() -> (std::sync::Arc<ambition_extension_wasm::WasmModules>, Vec<ambition_extension_sdk::ModuleDescriptor>) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let path = ambition_extension_wasm::build_module_crate(&root, "ambition_content_modules")
            .expect("the module crate builds for wasm32-unknown-unknown");
        let bytes = std::fs::read(path).unwrap();
        ambition_extension_wasm::WasmModules::load(&bytes).expect("the module file loads")
    }

    fn world(road: Road) -> World0 {
        let native = road == Road::NativeSystem;
        let mut app = App::new();
        app.init_schedule(Sim);
        app.add_message::<ActorActionMessage>()
            .add_message::<ProjectileSpawnRequest>()
            .init_resource::<ambition_time::SimTick>();
        if native {
            app.register_required_components::<BossConfig, EchoFanState>();
            app.add_systems(Sim, spawn_echo_fan_from_special_messages);
        } else {
            app.add_plugins(ExtensionHostPlugin::new(Sim));
            ambition_boss_encounter::extension::install(&mut app);
            ambition_projectiles::extension::install(&mut app);
            if road == Road::Wasm {
                let (backend, modules) = wasm_echo_fan();
                app.add_loaded_extension_modules(backend, modules, false);
            } else {
                app.add_extension_module(ambition_content_modules::echo_fan::module());
            }
            app.finish();
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
                format!("mockingbird_{i}"),
                "Mockingbird",
                ae::Aabb::new(ae::Vec2::new(100.0 * i as f32, 50.0), ae::Vec2::new(30.0, 30.0)),
                ambition_entity_catalog::placements::BossBrain::Dormant,
            );
            boss.kin.facing = if i == 1 { -1.0 } else { 1.0 };
            let mut entity = app.world_mut().spawn((
                boss.kin,
                boss.config,
                boss.status,
                boss.health,
                FeatureSimEntity,
            ));
            if let Some(target) = target {
                entity.insert(target);
            }
            bosses.push(entity.id());
        }
        World0 { app, bosses }
    }

    fn press(app: &mut App, actor: Entity, occurrence: Option<u32>) {
        app.world_mut().write_message(ActorActionMessage {
            actor,
            request: ActionRequest::Special {
                spec: SpecialActionSpec::Special(ECHO_FAN_KEY.into()),
                params: Default::default(),
            },
            move_instance: occurrence,
        });
    }

    /// Each tick's requests, grouped by owner in emission order.
    pub(super) fn run(road: Road, ticks: &[Presses], kill_boss_0_at: Option<usize>) -> Vec<BTreeMap<usize, Vec<String>>> {
        let World0 { mut app, bosses } = world(road);
        let mut trace = Vec::new();
        for (tick, presses) in ticks.iter().enumerate() {
            if kill_boss_0_at == Some(tick) {
                app.world_mut()
                    .get_mut::<ambition_characters::actor::BodyHealth>(bosses[0])
                    .unwrap()
                    .health
                    .current = 0;
            }
            for (boss, occurrence) in presses.iter() {
                press(&mut app, bosses[*boss], *occurrence);
            }
            app.world_mut().run_schedule(Sim);
            // Keyed by boss index: the two Apps allocate entities differently.
            let mut by_owner: BTreeMap<usize, Vec<String>> = BTreeMap::new();
            for request in app
                .world_mut()
                .resource_mut::<Messages<ProjectileSpawnRequest>>()
                .drain()
            {
                let boss = bosses
                    .iter()
                    .position(|b| *b == request.owner)
                    .expect("only a boss fires");
                let mut request = request;
                request.owner = Entity::PLACEHOLDER;
                by_owner.entry(boss).or_default().push(format!("{request:?}"));
            }
            trace.push(by_owner);
            app.world_mut().resource_mut::<Messages<ActorActionMessage>>().update();
            app.world_mut().resource_mut::<ambition_time::SimTick>().0 += 1;
        }
        trace
    }

    pub(super) const STRIKES: &[Presses] = &[
        &[],
        // A strike over three ticks fires once, on its first tick.
        &[(0, Some(4))],
        &[(0, Some(4))],
        &[(0, Some(4))],
        // A gap ends it; the next press is a new strike.
        &[],
        &[(0, None), (1, Some(9))],
        // Two presses of one key in one tick: the last move use counts.
        &[(2, Some(1)), (2, Some(2))],
        &[(1, Some(9))],
        &[],
        &[(1, None), (2, None)],
    ];

    #[test]
    fn the_module_emits_what_the_native_system_emitted() {
        let native = run(Road::NativeSystem, STRIKES, None);
        let module = run(Road::Module, STRIKES, None);
        let fired: usize = native.iter().map(|t| t.values().map(Vec::len).sum::<usize>()).sum();
        // ⭐ The premise: the reference must fire, or the comparison is empty.
        // Strikes begin on ticks 1, 5 (two bosses), 6, 7 (boss 1 again: tick 6
        // was a gap for it) and 9 (two bosses): seven fans of 7 shots.
        assert_eq!(fired, 7 * 7, "the native reference fired {fired} shots");
        assert_eq!(module, native);
    }

    #[test]
    fn a_dead_boss_does_not_fire_and_does_not_end_its_strike() {
        let ticks: &[Presses] = &[&[(0, None)], &[(0, None)], &[(0, None)], &[], &[(0, None)]];
        // Boss 0 starts a strike alive (fires), dies on tick 1 mid-strike,
        // and its next strike on tick 4 is a dead one: neither fires again.
        let native = run(Road::NativeSystem, ticks, Some(1));
        let module = run(Road::Module, ticks, Some(1));
        let fans: Vec<usize> = native.iter().map(|t| t.len()).collect();
        assert_eq!(fans, [1, 0, 0, 0, 0], "the native reference's fans by tick");
        assert_eq!(module, native);
    }
}

/// The same module, built as a `.wasm` file and run by the interpreter,
/// emits what the native system emitted: the no-relink road (I6/I7) changes
/// where the code runs, not what it does.
mod wasm_parity {
    #[test]
    fn the_wasm_build_of_the_module_emits_what_the_native_system_emitted() {
        use super::module_parity::*;
        let native = run(Road::NativeSystem, STRIKES, None);
        let wasm = run(Road::Wasm, STRIKES, None);
        assert!(native.iter().any(|t| !t.is_empty()), "the premise: the reference fired");
        // ⚠ NOT BIT-EQUAL, AND THAT IS EXPECTED: the guest's `sin`, `cos` and
        // `atan2` are its own compiled code, so one shot's velocity differs in
        // the last f32 bit (measured: -130.4897 against -130.48969). Every
        // number is compared to 1e-3 and everything else exactly: owner, move
        // use, count, order, damage, lifetime, size.
        assert_eq!(super::quantize(&wasm), super::quantize(&native));

    }
}

/// Each trace with every float rounded to 1e-3, so two executables whose
/// transcendentals differ in the last bit compare equal and nothing else does.
fn quantize(trace: &[std::collections::BTreeMap<usize, Vec<String>>]) -> Vec<std::collections::BTreeMap<usize, Vec<String>>> {
    let float = regex_lite_float;
    trace
        .iter()
        .map(|tick| tick.iter().map(|(b, shots)| (*b, shots.iter().map(|s| float(s)).collect())).collect())
        .collect()
}

/// Replace every decimal literal `-?\d+\.\d+` with the same value at three
/// places.
fn regex_lite_float(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let neg = bytes[i] == b'-' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit();
        let mut j = if neg { i + 1 } else { i };
        if j < bytes.len() && bytes[j].is_ascii_digit() && (start == 0 || !(bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_')) {
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j + 1 < bytes.len() && bytes[j] == b'.' && bytes[j + 1].is_ascii_digit() {
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                let v: f64 = s[start..j].parse().unwrap();
                out.push_str(&format!("{:.3}", v));
                i = j;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}
