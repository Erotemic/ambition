//! GNU-ton's conductor, run two ways on the same fight: the NATIVE conductor
//! (`conduct_gnu_ton`) and its procedural module linked into this process
//! (`ambition_content_modules::gnu_ton::conductor`, a conducted boss on the
//! extension host).
//!
//! The fight is scripted through the scholar's pattern state: every move's
//! tell and strike, the apple rain and the eureka after it, and his death. A
//! fist is struck twice while it is buried (the first blow passes on, the
//! second only heals it), the player stands on the gnu's back when a blow
//! hurts him (the gnu's reflex bucks), and a blow lands while the encounter
//! is between phases (refused). Each road must pose the fists, the gnu and
//! the scholar the same way, draw them with the same rows, hold the same
//! volumes and refuse and pass the same blows, tick for tick.
//!
//! The module has a phase the native conductor did not: the giant's arms are
//! ON it until the encounter leaves phase 1. The trace is of the fight after
//! that, with the fists as bodies of their own: one tick of an enraged
//! encounter before the trace puts the arms in their lines (`run`), and each
//! fist rests where the module rests it (`ch::arms::rest`).
//!
//! ⚠ Deliberately different, and so not compared:
//! - the OWNER of a fist's hit volume: the native conductor gave it to the
//!   fist, the held-box port gives it to the scholar the module runs for;
//! - a volume's inspector name: a held box has none;
//! - what the shocks do after his death: the native conductor left them
//!   where they stood until their lifetime ran out, the module stops holding
//!   them. The trace ends before then.
//! - the giant's row while it walks between beats or looks round at its
//!   back (round 3): the native conductor stood it still. On those ticks its
//!   row is not compared.
//! - all of it after his death (round 3): the module plays the end of the
//!   fight (`ch::finale`); the native conductor let go and stood. The ticks
//!   from his death on are not compared.

use ambition_boss_encounter::{BossClusterScratch, BossEncounter, BossEncounterPhase};
use ambition_characters::actor::{BodyHealth, Health, Invulnerability, Limb, LimbRig, LimbSlot, MountBack};
use ambition_characters::brain::{BossAttackProfile, BossAttackState};
use ambition_combat::components::ActorTarget;
use ambition_combat::strike::{Hitbox, HitboxAnchor, HitboxLifetime};
use ambition_content_modules::gnu_ton::choreography as ch;
use ambition_extension_host::{ExtensionAppExt, ExtensionHostPlugin, ExtensionSet};
use ambition_platformer2d_core::{self as ae, BodyKinematics};
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
use ambition_platformer2d::sprite_sheet::character::PinnedRow;
use bevy::ecs::message::Messages;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use super::conductor_reference_tests::{conduct_gnu_ton, GnuTonConductor, GNU_TON_ID};

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Road {
    NativeSystem,
    Module,
}

const DT: f32 = 1.0 / 60.0;
const DEMONSTRATE_STRIKE: usize = 46;
const PAIR_STRIKE: usize = 741;
const RAIN_ENDS: usize = 1060;
const DIES_AT: usize = 1400;
const TICKS: usize = 1420;

/// The pattern's script: (first tick, last tick, key, striking).
const SCRIPT: &[(usize, usize, &str, bool)] = &[
    (10, 45, "demonstrate", false),
    (DEMONSTRATE_STRIKE, 135, "demonstrate", true),
    (151, 180, "pendulum", false),
    (181, 240, "pendulum", true),
    (251, 300, "fluxions", false),
    (301, 360, "fluxions", true),
    (371, 400, "cradle", false),
    (401, 500, "cradle", true),
    (511, 540, "orbit", false),
    (541, 600, "orbit", true),
    (611, 630, "buck", false),
    (631, 650, "buck", true),
    (661, 680, "stomp", false),
    (681, 700, "stomp", true),
    (711, 740, "demonstrate_pair", false),
    (PAIR_STRIKE, 860, "demonstrate_pair", true),
    (871, 910, "fluxions_pair", false),
    (911, 970, "fluxions_pair", true),
    (981, 1000, "apple_rain", false),
    (1001, RAIN_ENDS, "apple_rain", true),
];

fn attack_at(tick: usize) -> BossAttackState {
    let mut state = BossAttackState::default();
    if let Some(&(_, last, key, striking)) = SCRIPT.iter().find(|(first, last, ..)| (*first..=*last).contains(&tick)) {
        let remaining = (last + 1 - tick) as f32 * DT;
        let profile = Some(BossAttackProfile::Special(key.to_string()));
        if striking {
            state.active_profile = profile;
            state.active_remaining = remaining;
        } else {
            state.telegraph_profile = profile;
            state.telegraph_remaining = remaining;
        }
    }
    state
}

/// The ticks a buried fist is struck at, and by how much.
fn blows() -> [(usize, usize, i32); 4] {
    let buried = (ch::demonstrate::FALL / DT) as usize + 8;
    [
        // The lead (the left fist leads the first beat) is struck twice.
        (DEMONSTRATE_STRIKE + buried, 0, 3),
        (DEMONSTRATE_STRIKE + buried + 6, 0, 2),
        // Between phases: refused. Both fists, so the pair's lead is struck
        // whichever fist leads it.
        (PAIR_STRIKE + buried, 0, 3),
        (PAIR_STRIKE + buried, 1, 3),
    ]
}

fn hall_world() -> ae::World {
    ae::World::new(
        "gnu_hall",
        ae::Vec2::new(2000.0, 1300.0),
        ae::Vec2::ZERO,
        vec![
            ae::Block::solid("floor", ae::Vec2::new(0.0, 1250.0), ae::Vec2::new(2000.0, 50.0)),
            ae::Block::solid("left", ae::Vec2::new(0.0, 0.0), ae::Vec2::new(48.0, 1250.0)),
            ae::Block::solid("right", ae::Vec2::new(1800.0, 0.0), ae::Vec2::new(200.0, 1250.0)),
        ],
    )
}

fn body(pos: ae::Vec2, size: ae::Vec2, facing: f32) -> BodyKinematics {
    BodyKinematics { pos, vel: ae::Vec2::ZERO, size, facing }
}

struct Cast {
    scholar: Entity,
    gnu: Entity,
    fists: [Entity; 2],
    player: Entity,
}

fn world(road: Road) -> (App, Cast) {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_message::<ambition_characters::brain::ActorActionMessage>()
        .add_message::<ambition_projectiles::ProjectileSpawnRequest>()
        .add_message::<ambition_vfx::EffectRequest>()
        .add_message::<ambition_vfx::vfx::VfxInRoom>()
        .add_message::<ambition_sfx::OwnedSfxMessage>()
        .init_resource::<ambition_time::SimTick>()
        .insert_resource(ambition_time::WorldTime::new(DT, DT));
    match road {
        Road::NativeSystem => {
            app.add_systems(Sim, conduct_gnu_ton);
        }
        Road::Module => {
            app.add_plugins(ExtensionHostPlugin::new(Sim));
            ambition_platformer2d_runtime::extension_composition::install_ports(&mut app);
            app.add_systems(
                Sim,
                ambition_boss_encounter::conduct::face_conducted_bosses
                    .before(ExtensionSet::Collect(ambition_extension_sdk::phases::BOSS_CONDUCT)),
            );
            app.add_extension_module(ambition_content_modules::gnu_ton::conductor::module());
            app.finish();
        }
    }
    app.world_mut().spawn((RoomInstanceRoot, LiveRoomInstance::ACTIVATION, ae::RoomGeometry(hall_world())));
    let room = InRoomInstance(LiveRoomInstance::ACTIVATION);
    let world = app.world_mut();
    let player = world.spawn((body(ae::Vec2::new(500.0, 1235.0), ae::Vec2::new(20.0, 30.0), 1.0), room)).id();
    let fist = |world: &mut World, pos: ae::Vec2| {
        world
            .spawn((
                body(pos, ae::Vec2::new(130.0, 100.0), 1.0),
                ae::CenteredAabb::new(pos, ae::Vec2::new(65.0, 50.0)),
                BodyHealth::new(Health::new(10)),
                ae::BodyGroundState::default(),
                ae::PoseOwnedExternally::only(ae::PoseHold::Conductor),
                PinnedRow::default(),
                room,
            ))
            .id()
    };
    let gnu_pos = ae::Vec2::new(900.0, 1040.0);
    // Each fist rests out from the line its arm is in.
    let rests = [ch::arms::rest(0), ch::arms::rest(1)].map(|rest| ae::Vec2::new(rest.x, rest.y));
    let fists = [fist(world, gnu_pos + rests[0]), fist(world, gnu_pos + rests[1])];
    let gnu = world
        .spawn((
            body(gnu_pos, ae::Vec2::new(439.0, 414.0), -1.0),
            ae::BodyFlightState::default(),
            ae::Unmirrored,
            ambition_mount::BackLedge::new(MountBack { left: -208.0, right: 169.0, top: -9.75, thickness: 14.0 }),
            LimbRig::from_pairs([(LimbSlot::HAND_LEFT, fists[0]), (LimbSlot::HAND_RIGHT, fists[1])]),
            PinnedRow::default(),
            room,
        ))
        .id();
    for (i, slot) in [LimbSlot::HAND_LEFT, LimbSlot::HAND_RIGHT].into_iter().enumerate() {
        world.entity_mut(fists[i]).insert(Limb { of: gnu, slot, home_offset: rests[i] });
    }
    let catalog = crate::bosses::shipped_boss_catalog();
    let mut boss = BossClusterScratch::new(
        &catalog,
        "boss_gnu_ton_rider",
        "GNU-ton",
        ae::Aabb::new(ae::Vec2::new(950.0, 800.0), ae::Vec2::new(27.0, 48.0)),
        ambition_entity_catalog::placements::BossBrain::PhaseScript { script_id: GNU_TON_ID.to_string() },
    );
    assert_eq!(boss.config.behavior.id, GNU_TON_ID, "the premise: the scholar's behaviour");
    let mut state = ambition_characters::boss_encounter::ActorPhaseState::new(Vec::new());
    state.phase = BossEncounterPhase::Phase1;
    boss.status.encounter = Some(state);
    let kin = boss.kin;
    let mut scholar = world.spawn((
        boss.kin,
        ae::CenteredAabb::new(kin.pos, kin.size * 0.5),
        boss.config,
        boss.status,
        boss.health,
        ambition_characters::actor::BodyCombat::default(),
        BossAttackState::default(),
        ActorTarget { entity: Some(player), pos: ae::Vec2::new(500.0, 1235.0) },
        ambition_characters::control::ActorControl::default(),
        ambition_characters::actor::ActorFaction::Boss,
        ambition_mount::RidingOn { mount: gnu },
        PinnedRow::default(),
        room,
    ));
    if road == Road::NativeSystem {
        scholar.insert(GnuTonConductor::unseated());
    }
    let scholar = scholar.id();
    world.entity_mut(gnu).insert(ambition_mount::MountSlot { rider: Some(scholar) });
    (app, Cast { scholar, gnu, fists, player })
}

type Trace = Vec<Vec<String>>;

fn run(road: Road) -> Trace {
    let (mut app, cast) = world(road);
    let mut seen = std::collections::HashSet::new();
    let back = {
        let world = app.world();
        let gnu = world.get::<BodyKinematics>(cast.gnu).unwrap();
        world.get::<ambition_mount::BackLedge>(cast.gnu).unwrap().aabb(gnu, true)
    };
    // Before the trace: one tick of an enraged encounter. The module puts the
    // giant's arms in their lines then, and the fight of the trace is the
    // fight of the fists as bodies of their own. Each road runs the tick, so
    // their clocks agree.
    app.world_mut().get_mut::<BossEncounter>(cast.scholar).unwrap().encounter.as_mut().unwrap().phase =
        BossEncounterPhase::Enrage;
    app.world_mut().run_schedule(Sim);
    app.world_mut().flush();
    observe(&mut app, &cast, &mut seen);
    app.world_mut().resource_mut::<ambition_time::SimTick>().0 += 1;
    let mut trace = Vec::new();
    for tick in 0..TICKS {
        // The player walks the floor, and climbs onto the gnu's back for the
        // first buried fist.
        let on_back = (DEMONSTRATE_STRIKE..DEMONSTRATE_STRIKE + 80).contains(&tick);
        let at = if on_back {
            ae::Vec2::new((back.min.x + back.max.x) * 0.5, back.min.y - 15.0)
        } else {
            ae::Vec2::new(500.0 + 300.0 * (tick as f32 * 0.02).sin(), 1235.0)
        };
        app.world_mut().get_mut::<BodyKinematics>(cast.player).unwrap().pos = at;
        app.world_mut().get_mut::<ActorTarget>(cast.scholar).unwrap().pos = at;
        *app.world_mut().get_mut::<BossAttackState>(cast.scholar).unwrap() = attack_at(tick);
        let between = (PAIR_STRIKE..PAIR_STRIKE + 60).contains(&tick);
        app.world_mut().get_mut::<BossEncounter>(cast.scholar).unwrap().encounter.as_mut().unwrap().phase =
            if between { BossEncounterPhase::Transition } else { BossEncounterPhase::Phase1 };
        for (at_tick, fist, damage) in blows() {
            if tick == at_tick {
                app.world_mut().get_mut::<BodyHealth>(cast.fists[fist]).unwrap().health.current -= damage;
            }
        }
        if tick == DIES_AT {
            app.world_mut().get_mut::<BodyHealth>(cast.scholar).unwrap().health.current = 0;
        }
        app.world_mut().run_schedule(Sim);
        app.world_mut().flush();
        trace.push(observe(&mut app, &cast, &mut seen));
        app.world_mut().resource_mut::<ambition_time::SimTick>().0 += 1;
    }
    trace
}

fn observe(app: &mut App, cast: &Cast, seen: &mut std::collections::HashSet<Entity>) -> Vec<String> {
    let mut out = Vec::new();
    let world = app.world_mut();
    let kin = *world.get::<BodyKinematics>(cast.scholar).unwrap();
    let health = *world.get::<BodyHealth>(cast.scholar).unwrap();
    out.push(format!(
        "scholar {:?} {:?} aabb {:?} hp {} flash {} facing {:?}",
        kin.pos,
        kin.vel,
        world.get::<ae::CenteredAabb>(cast.scholar).unwrap().center,
        health.current(),
        world.get::<ambition_characters::actor::BodyCombat>(cast.scholar).unwrap().hit_flash,
        world.get::<ambition_characters::control::ActorControl>(cast.scholar).unwrap().0.facing,
    ));
    out.push(format!(
        "encounter {:?}",
        world.get::<BossEncounter>(cast.scholar).unwrap().encounter.as_ref().map(|state| state.phase)
    ));
    let row = |world: &World, e: Entity| world.get::<PinnedRow>(e).map(|r| (r.rows.clone(), r.elapsed, r.looping));
    out.push(format!("scholar row {:?}", row(world, cast.scholar)));
    out.push(format!("gnu row {:?}", row(world, cast.gnu)));
    out.push(format!("gnu launch {:?}", world.get::<ae::BodyFlightState>(cast.gnu).unwrap().pending_launch_state()));
    for (i, fist) in cast.fists.iter().enumerate() {
        let kin = *world.get::<BodyKinematics>(*fist).unwrap();
        let health = world.get::<BodyHealth>(*fist).unwrap();
        out.push(format!(
            "fist {i} {:?} {:?} facing {} held {} closed {} hp {} ground {} aabb {:?} row {:?}",
            kin.pos,
            kin.vel,
            kin.facing,
            world.get::<ae::PoseOwnedExternally>(*fist).is_some(),
            health.health.invulnerable.holds(Invulnerability::SCRIPTED),
            health.current(),
            world.get::<ae::BodyGroundState>(*fist).unwrap().on_ground,
            world.get::<ae::CenteredAabb>(*fist).unwrap().center,
            row(world, *fist),
        ));
    }
    let positions: std::collections::HashMap<Entity, ae::Vec2> =
        world.query::<(Entity, &BodyKinematics)>().iter(world).map(|(e, k)| (e, k.pos)).collect();
    let mut hitboxes: Vec<String> = world
        .query::<(Entity, &Hitbox, &HitboxLifetime)>()
        .iter(world)
        .map(|(entity, h, life)| {
            // A re-armed volume is a new one (a fresh hit memory) at the same
            // place: only its arrival tells it from the one it replaced.
            let new = if seen.insert(entity) { "new " } else { "" };
            let center = match h.anchor {
                HitboxAnchor::FollowOwner { local_offset } => positions.get(&h.owner).copied().unwrap_or_default() + local_offset,
                HitboxAnchor::World { center } => center,
            };
            format!(
                "{new}hitbox {center:?} {:?} {:?} {} {:?} {:?} {:?} {}",
                h.half_extent, h.shape, h.damage, h.knockback, h.launch_dir, h.source, life.remaining_s
            )
        })
        .collect();
    hitboxes.sort();
    out.extend(hitboxes);
    // Sorted: each port's adapter writes in the order the ports were
    // installed, so a bubble and a burst of one tick can come in either order.
    let mut effects: Vec<String> = world
        .resource_mut::<Messages<ambition_vfx::vfx::VfxInRoom>>()
        .drain()
        .map(|vfx| format!("vfx {:?} {:?}", vfx.room, vfx.vfx))
        .collect();
    effects.sort();
    out.extend(effects);
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

/// ⭐ The module conducts the fight as the native conductor did, tick for
/// tick.
#[test]
fn the_gnu_ton_module_conducts_the_fight_as_its_native_conductor_did() {
    let native = run(Road::NativeSystem);
    // The premise: the reference performed what the fight is made of.
    let sound = |id: &'static str| format!("{:?}", ambition_platformer2d::sfx::SfxId::from_static(id));
    for what in [
        "SpeechBubble".to_string(),
        sound("boss.gnu_ton.stomp"),
        sound("boss.gnu_ton.snort"),
        "\"stuck\"".to_string(),
        "\"buck\"".to_string(),
        "flash 0.18".to_string(),
        "closed false".to_string(),
        // A stomp's shock rolling along the floor.
        "Vec2(26.0, 20.0) None 2".to_string(),
    ] {
        assert!(count(&native, &what) > 0, "the reference never showed {what}");
    }
    let module = run(Road::Module);
    for (tick, (m, n)) in module.iter().zip(&native).enumerate().take(DIES_AT) {
        // The trace names no rows of the giant's (its sheet is not loaded):
        // its walk is the one row of it that loops.
        let walks = m.iter().any(|l| l.starts_with("gnu row") && l.ends_with("true))"));
        let compared = |l: &&String| !(walks && l.starts_with("gnu row"));
        let (m, n): (Vec<&String>, Vec<&String>) = (m.iter().filter(compared).collect(), n.iter().filter(compared).collect());
        assert_eq!(m, n, "tick {tick}: the linked module");
    }
}
