//! Active crowd measurements through real brains and the ECS body phase.

use super::*;
use ambition_characters::brain::{Brain, BrainSnapshot};
use ambition_characters::control::ActorControl;
use ambition_combat::components::{ActorAggression, ActorTarget};
use ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay;
use ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Workload { Settled, Roaming, Converging, MovingWall }

#[derive(Resource)]
struct CrowdClock { tick: u32, workload: Workload }

#[derive(Resource, Default)]
struct BodyTiming { start: Option<std::time::Instant>, nanos: u128 }

fn start_body_timing(mut timing: ResMut<BodyTiming>) { timing.start = Some(std::time::Instant::now()); }
fn finish_body_timing(mut timing: ResMut<BodyTiming>) {
    timing.nanos += timing.start.take().unwrap().elapsed().as_nanos();
}

#[derive(Component)]
struct Travel { previous: ae::Vec2, distance: f64 }

fn drive_brains(
    mut clock: ResMut<CrowdClock>,
    mut bodies: Query<(&ae::BodyKinematics, &ae::BodyGroundState, &ActorTarget,
        &mut Brain, &mut ActorControl, &mut Travel)>,
    mut overlays: Query<&mut FeatureEcsWorldOverlay, With<RoomInstanceRoot>>,
) {
    for (kin, ground, target, mut brain, mut control, mut travel) in &mut bodies {
        travel.distance += f64::from(kin.pos.distance(travel.previous));
        travel.previous = kin.pos;
        let snapshot = BrainSnapshot {
            actor_pos: kin.pos, actor_vel: kin.vel, actor_facing: kin.facing,
            actor_half_width: kin.size().x * 0.5, actor_on_ground: ground.on_ground,
            target_pos: target.pos, target_alive: target.entity.is_some(),
            max_run_speed: 155.0, sim_time: clock.tick as f32 / 60.0,
            ..BrainSnapshot::idle()
        };
        crate::brain_tick::tick_brain(&mut brain, &snapshot, &mut control.0);
    }
    if clock.workload == Workload::MovingWall {
        for mut overlay in &mut overlays {
            let wall = &mut overlay.blocks[0];
            let x = 3000.0 + (clock.tick as f32 * 0.03).sin() * 250.0;
            wall.velocity = ae::Vec2::new(x - wall.aabb.min.x, 0.0);
            wall.aabb = wall.aabb.translated(wall.velocity);
        }
    }
    clock.tick += 1;
}

fn crowd(count: usize, workload: Workload) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(1.0 / 60.0)));
    app.insert_resource(ambition_time::WorldTime::new(1.0 / 60.0, 1.0 / 60.0));
    app.insert_resource(CrowdClock { tick: 0, workload });
    app.init_resource::<BodyTiming>();
    app.init_resource::<ActorSteering>();
    app.init_resource::<ae::ActiveMovementTuning>();
    app.init_resource::<ambition_combat::feel::FeelTuning>();
    app.init_resource::<ambition_platformer2d_shared_tangle::body::BodyContactSnapshot>();
    app.init_resource::<ambition_platformer2d_world::collision::ComposedSolids>();
    app.add_message::<ambition_combat::events::HitEvent>();
    #[cfg(feature = "causal")]
    app.add_message::<crate::causal::BodyMovementOps>();
    app.add_systems(Update, (
        ambition_combat::targeting::select_actor_targets,
        drive_brains,
        snapshot_body_contact,
        ambition_platformer2d_world::collision::retire_stale_composed_solids,
        start_body_timing,
        integrate_sim_bodies,
        finish_body_timing,
    ).chain());
    let rows = count.div_ceil(32);
    let floors = (0..rows).map(|row| ae::Block::solid(format!("floor-{row}"),
        ae::Vec2::new(0.0, 160.0 + row as f32 * 128.0), ae::Vec2::new(6400.0, 24.0))).collect();
    let world = ae::World::new("active-crowd", ae::Vec2::new(6400.0, rows as f32 * 128.0 + 400.0),
        ae::Vec2::new(64.0, 128.0), floors);
    let mut overlay = FeatureEcsWorldOverlay::default();
    if workload == Workload::MovingWall {
        overlay.blocks.push(ae::Block::solid("moving-wall", ae::Vec2::new(3000.0, 0.0),
            ae::Vec2::new(24.0, rows as f32 * 128.0 + 200.0)));
    }
    app.world_mut().spawn((RoomInstanceRoot, ae::RoomGeometry(world), overlay));
    let sprites = ambition_sprite_sheet::test_support::shared_sprites();
    for index in 0..count {
        let x = if workload == Workload::Converging {
            2480.0 + (index % 32) as f32 * 40.0
        } else {
            64.0 + (index % 32) as f32 * 192.0
        };
        let pos = ae::Vec2::new(x, 128.0 + (index / 32) as f32 * 128.0);
        let id = format!("crowd-{index:05}");
        let seed = ambition_body_seed::ActorClusterSeed::new(&sprites, id.clone(), id.clone(),
            ae::Aabb::new(pos, ae::Vec2::new(11.0, 19.0)),
            ambition_entity_catalog::placements::CharacterBrain::Custom("stand_still".into()), &[]);
        let mut brain = match workload {
            Workload::Settled => Brain::stand_still(),
            Workload::Converging => Brain::npc_patrol(3100.0, 20.0),
            _ => Brain::npc_patrol(pos.x, 80.0),
        };
        let Brain::StateMachine(machine) = &mut brain;
        if let ambition_characters::brain::StateMachineCfg::Patrol { cfg, .. } = machine {
            cfg.aggro_radius = 0.0;
        }
        let entity = app.world_mut().spawn((seed.into_components(), (
            ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
            SimId::placement(&id), ae::CenteredAabb::new(pos, ae::Vec2::new(11.0, 19.0)),
            ambition_characters::actor::BodyCombat::default(), ActorControl::default(),
            ActorTarget::default(), ActorAggression::hostile(),
            ambition_combat::components::ActorDisposition::Hostile,
            if index % 2 == 0 { ambition_characters::actor::ActorFaction::Enemy }
                else { ambition_characters::actor::ActorFaction::Player },
            ae::MotionModel::default(), ae::BodyFrameOutput::default(), brain,
            Travel { previous: pos, distance: 0.0 },
        ))).id();
        let mut frame = ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default();
        frame.publish_resolved_frame(ae::MotionFrame::from_acceleration(ae::Vec2::new(0.0, 1600.0)).unwrap());
        app.world_mut().entity_mut(entity).insert(frame);
        if workload == Workload::Converging {
            app.world_mut().entity_mut(entity).insert(ae::movement::BodyContact::default());
        }
    }
    app
}

fn state(app: &mut App) -> (u64, usize, f64) {
    use std::fmt::Write;
    let mut query = app.world_mut().query::<(&SimId,
        ambition_body_seed::clusters::ActorClusterQueryDataReadOnly,
        &ae::MotionModel, &Brain, &Travel, &ActorControl, &ae::BodyFrameOutput,
        &ambition_characters::actor::BodyCombat, &ActorTarget, &ae::CenteredAabb, &ae::BodyMotionFacts)>();
    let mut rows: Vec<_> = query.iter(app.world()).map(|(id, body, model, brain, travel,
        control, output, combat, target, bounds, facts)| {
        let mut text = String::new();
        macro_rules! append { ($($value:expr),* $(,)?) => { $(write!(&mut text, "{:?}", $value).unwrap();)* }; }
        append!(body.kin, body.sweep, body.status, body.health, body.surface, body.attack,
            body.identity, body.config, body.policy, body.spawn, body.motion, body.caps,
            body.held_item, body.abilities, body.base_size, body.ground, body.wall, body.jump,
            body.dash, body.flight, body.blink, body.ledge, body.dodge, body.shield, body.body_mode,
            body.env_contact, body.resources, body.effects, body.action_buffer, body.restart,
            model, brain, control, output.events, output.reset, output.stamp, combat, target, bounds, facts);
        (id.clone(), text, travel.distance)
    }).collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hash = 0_u64;
    for (_, text, _) in &rows { for byte in text.bytes() { hash = hash.wrapping_mul(31).wrapping_add(u64::from(byte)); } }
    (hash, rows.iter().filter(|(_, _, distance)| *distance > 50.0).count(), rows.iter().map(|r| r.2).sum())
}

#[test]
fn goal_driven_crowds_move_and_repeat_the_same_state() {
    for workload in [Workload::Roaming, Workload::Converging, Workload::MovingWall] {
        let mut a = crowd(32, workload);
        let mut b = crowd(32, workload);
        for _ in 0..90 { a.update(); b.update(); }
        let left = state(&mut a); let right = state(&mut b);
        assert_eq!(left, right, "{workload:?}");
        let minimum_moving = if workload == Workload::Converging { 16 } else { 24 };
        assert!(left.1 >= minimum_moving, "the active workload must move its bodies: {workload:?} {left:?}");
    }
}

#[test]
fn body_effects_keep_stable_identity_order_across_archetype_changes() {
    #[derive(Component)]
    struct Extra;
    let mut app = crowd(128, Workload::Settled);
    let mut bodies = app.world_mut().query::<(Entity, &SimId, &mut ae::BodyKinematics)>();
    let mut expected = Vec::new();
    let mut changed = Vec::new();
    for (entity, id, mut kin) in bodies.iter_mut(app.world_mut()) {
        kin.pos.y = 10000.0;
        expected.push(id.clone());
        if expected.len() % 2 == 0 { changed.push(entity); }
    }
    for entity in changed { app.world_mut().entity_mut(entity).insert(Extra); }
    expected.sort();
    app.update();
    let hits: Vec<_> = app.world_mut().resource_mut::<Messages<ambition_combat::events::HitEvent>>().drain().collect();
    let actual: Vec<_> = hits.iter().map(|hit| {
        assert_eq!(hit.source, ambition_combat::events::HitSource::LeftTheWorld);
        assert!(hit.origin.is_some(), "the native writer must enrich deferred hits");
        let ambition_combat::events::HitTarget::Body(body) = hit.target else { panic!("one body per hit") };
        app.world().get::<SimId>(body).unwrap().clone()
    }).collect();
    assert_eq!(actual, expected);
}

#[test]
#[ignore = "measurement: run explicitly with --ignored --nocapture"]
fn measure_active_crowds() {
    let _app = crowd(0, Workload::Settled);
    println!("compute_workers={}", bevy::tasks::ComputeTaskPool::get().thread_num());
    println!("workload,bodies,sample,ns_per_tick,ns_per_body_phase,state_hash,moved_bodies,distance");
    for workload in [Workload::Settled, Workload::Roaming, Workload::Converging, Workload::MovingWall] {
        for count in [32, 128, 512] {
            for sample in 0..3 {
                let mut app = crowd(count, workload);
                for _ in 0..30 { app.update(); }
                let body_start = app.world().resource::<BodyTiming>().nanos;
                let start = std::time::Instant::now();
                for _ in 0..120 { app.update(); }
                let elapsed = start.elapsed().as_nanos() as f64 / 120.0;
                let body_elapsed = (app.world().resource::<BodyTiming>().nanos - body_start) as f64 / 120.0;
                let (hash, moved, distance) = state(&mut app);
                println!("{workload:?},{count},{sample},{elapsed:.0},{body_elapsed:.0},{hash},{moved},{distance:.3}");
            }
        }
    }
}
