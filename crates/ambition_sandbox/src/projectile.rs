//! Sandbox player projectile (Fireball / Hadouken).
//!
//! The engine owns the reusable primitives:
//!
//! * `ae::ProjectileSpec` / `ProjectileBody` (data + per-frame tick),
//! * `ae::ProjectileSpawner` (cooldown + resource meter),
//! * `ae::MotionInputBuffer` (quarter / half-circle motion recognition).
//!
//! This module wires those primitives into the Bevy sandbox: input
//! sampling, collision against the active world, and trace events.
//! Damage is routed through the unified `FeatureRuntime::apply_damage_event`
//! path — the same entry point slashes, pogo-bounces, and any future
//! tool / hazard / spell that produces a damage volume go through.

use bevy::prelude::*;

use ambition_engine as ae;
use ambition_engine::AabbExt;

use crate::audio::SfxMessage;
use crate::features::{DamageEvent, DamageSource, FeatureEventBus};
use crate::fx::VfxMessage;
use crate::input::ControlFrame;
use crate::physics::DebrisBurstMessage;
use crate::trace::{GameplayTraceBuffer, GameplayTraceEvent};
use crate::{GameWorld, SandboxRuntime};

/// Bevy resource holding the player's projectile spawner state plus
/// the rolling motion-input buffer.
#[derive(Resource)]
pub struct PlayerProjectileState {
    pub spawner: ae::ProjectileSpawner,
    pub motion_buffer: ae::MotionInputBuffer,
    /// Time since first sample, in monotonic seconds.
    pub clock: f32,
    /// Live projectiles in flight. Sandbox owns this rather than
    /// spawning Bevy entities per projectile so headless tests can
    /// observe motion / collision without rendering machinery.
    pub bodies: Vec<PlayerProjectile>,
    pub unlocked: ProjectileUnlocks,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectileUnlocks {
    pub fireball: bool,
    pub hadouken: bool,
}

impl Default for ProjectileUnlocks {
    fn default() -> Self {
        Self {
            fireball: true,
            hadouken: true,
        }
    }
}

impl Default for PlayerProjectileState {
    fn default() -> Self {
        Self {
            spawner: ae::ProjectileSpawner::new(8.0, 1.5),
            motion_buffer: ae::MotionInputBuffer::new(0.45),
            clock: 0.0,
            bodies: Vec::new(),
            unlocked: ProjectileUnlocks::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PlayerProjectile {
    pub body: ae::ProjectileBody,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProjectileTraceEvent {
    Fired {
        kind: ae::ProjectileKind,
    },
    BlockedByResource {
        kind: ae::ProjectileKind,
    },
    Hit {
        kind: ae::ProjectileKind,
        damage: i32,
    },
    Expired {
        kind: ae::ProjectileKind,
    },
}

impl ProjectileTraceEvent {
    pub fn into_trace_event(self, tick: u64) -> GameplayTraceEvent {
        match self {
            Self::Fired { kind } => GameplayTraceEvent::Projectile {
                tick,
                kind: kind.label().to_string(),
                event: "fired".into(),
                damage: 0,
            },
            Self::BlockedByResource { kind } => GameplayTraceEvent::Projectile {
                tick,
                kind: kind.label().to_string(),
                event: "blocked_by_resource".into(),
                damage: 0,
            },
            Self::Hit { kind, damage } => GameplayTraceEvent::Projectile {
                tick,
                kind: kind.label().to_string(),
                event: "hit".into(),
                damage,
            },
            Self::Expired { kind } => GameplayTraceEvent::Projectile {
                tick,
                kind: kind.label().to_string(),
                event: "expired".into(),
                damage: 0,
            },
        }
    }
}

pub fn update_projectiles(
    time: Res<Time>,
    world: Res<GameWorld>,
    mut runtime: ResMut<SandboxRuntime>,
    control_frame: Res<ControlFrame>,
    user_settings: Res<crate::settings::UserSettings>,
    mut state: ResMut<PlayerProjectileState>,
    mut trace: ResMut<GameplayTraceBuffer>,
    mut feature_bus: ResMut<FeatureEventBus>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
) {
    let dt = time.delta_secs();
    state.clock += dt;
    state.spawner.tick(dt);

    // Sample motion for Hadouken recognition. axis_y is +Y-down in
    // sandbox conventions; engine MotionDirection treats +Y as up.
    let dir = ae::MotionDirection::from_axis(control_frame.axis_x, -control_frame.axis_y, 0.55);
    let now = state.clock;
    state.motion_buffer.push(dir, now);

    let mut events: Vec<ProjectileTraceEvent> = Vec::new();
    let mut still_alive = Vec::with_capacity(state.bodies.len());
    let mut bodies = std::mem::take(&mut state.bodies);
    for mut p in bodies.drain(..) {
        let alive = p.body.tick(dt);
        if !alive {
            events.push(ProjectileTraceEvent::Expired { kind: p.body.kind });
            continue;
        }

        // Step 1: damage check against actors (enemies / bosses /
        // breakables / NPCs) via the unified pathway. If anything was
        // hit, the projectile expires this frame (no piercing today).
        let damage_event = DamageEvent {
            volume: p.body.aabb(),
            damage: p.body.damage,
            source: DamageSource::PlayerProjectile { kind: p.body.kind },
        };
        let report = runtime.features.apply_damage_event(&damage_event);
        if report.any_actor_hit() {
            forward_damage_feedback(&mut vfx, &mut debris, &report.events);
            // Forward boss-damage / quest / flag writes / NPC-struck
            // events so the rest of the systems (boss encounter, save
            // flags, quest registry) react to projectile hits the same
            // way they react to slash hits.
            feature_bus.ingest(&report.events);
            sfx.write(SfxMessage::Hit { pos: p.body.pos });
            events.push(ProjectileTraceEvent::Hit {
                kind: p.body.kind,
                damage: p.body.damage,
            });
            continue;
        }

        // Step 2: solid-wall test. Fireball bounces off floors (per
        // its `bounces_remaining` budget); side / ceiling / out-of-
        // budget hits expire. Hadouken spawns with 0 bounces, so the
        // first solid hit always expires it.
        let aabb = p.body.aabb();
        let solid_hit = world.0.blocks.iter().find(|block| {
            matches!(
                block.kind,
                ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
            ) && block.aabb.strict_intersects(aabb)
        });
        if let Some(block) = solid_hit {
            match p.body.resolve_solid_hit(block.aabb) {
                ae::ProjectileSolidHit::Bounced => {
                    sfx.write(SfxMessage::Hit { pos: p.body.pos });
                    still_alive.push(p);
                    continue;
                }
                ae::ProjectileSolidHit::Expired => {
                    events.push(ProjectileTraceEvent::Hit {
                        kind: p.body.kind,
                        damage: p.body.damage,
                    });
                    vfx.write(VfxMessage::Impact { pos: p.body.pos });
                    continue;
                }
            }
        }

        still_alive.push(p);
    }
    state.bodies = still_alive;

    if control_frame.projectile_pressed {
        let facing = if runtime.player.facing.abs() < f32::EPSILON {
            1.0
        } else {
            runtime.player.facing.signum()
        };
        let half_circle = state.motion_buffer.detect_half_circle();
        let want_hadouken = half_circle.is_some()
            && state.unlocked.hadouken
            && state.spawner.meter.current >= ae::ProjectileKind::Hadouken.cost();
        let kind = if want_hadouken {
            ae::ProjectileKind::Hadouken
        } else {
            ae::ProjectileKind::Fireball
        };
        let unlocked = match kind {
            ae::ProjectileKind::Hadouken => state.unlocked.hadouken,
            ae::ProjectileKind::Fireball => state.unlocked.fireball,
        };
        if unlocked {
            let origin = ae::Vec2::new(
                runtime.player.pos.x + facing * (runtime.player.size.x * 0.5 + 4.0),
                runtime.player.pos.y - runtime.player.size.y * 0.20,
            );
            let direction = ae::Vec2::new(facing, 0.0);
            match state.spawner.try_spawn(
                kind,
                origin,
                direction,
                user_settings.gameplay.player_damage_multiplier,
            ) {
                Ok(spec) => {
                    state.bodies.push(PlayerProjectile {
                        body: ae::ProjectileBody::from_spec(spec),
                    });
                    events.push(ProjectileTraceEvent::Fired { kind });
                    if matches!(kind, ae::ProjectileKind::Hadouken) {
                        state.motion_buffer.clear();
                    }
                }
                Err(ae::SpawnFailure::OutOfResource) => {
                    events.push(ProjectileTraceEvent::BlockedByResource { kind });
                }
                Err(ae::SpawnFailure::Cooldown) => {}
            }
        }
    }

    let tick = trace.current_tick();
    for event in events {
        trace.push_event(event.into_trace_event(tick));
    }
}

pub fn projectile_status_summary(state: &PlayerProjectileState) -> String {
    format!(
        "fire {:.1}/{:.1}  cd {:.2}s  in-flight {}",
        state.spawner.meter.current,
        state.spawner.meter.max,
        state.spawner.cooldown_remaining,
        state.bodies.len()
    )
}

/// Push the audio / VFX / debris cues from a `FeatureEvents` bundle
/// onto the message writers visible to the projectile system. This
/// is the projectile-side counterpart to `app::handle_feature_events`
/// (which uses the `Vec` collectors that `sandbox_update` builds);
/// keeping a small writer-shaped variant local avoids exposing those
/// collectors outside `sandbox_update`'s scope.
fn forward_damage_feedback(
    vfx: &mut MessageWriter<VfxMessage>,
    debris: &mut MessageWriter<DebrisBurstMessage>,
    events: &crate::features::FeatureEvents,
) {
    use crate::physics::PhysicsDebrisCue;
    for burst in &events.physics_bursts {
        let cue = match burst.cue {
            crate::features::FeaturePhysicsCue::Breakable => PhysicsDebrisCue::Breakable,
            crate::features::FeaturePhysicsCue::EnemyRagdoll => PhysicsDebrisCue::EnemyRagdoll,
            crate::features::FeaturePhysicsCue::BossRagdoll => PhysicsDebrisCue::BossRagdoll,
        };
        debris.write(DebrisBurstMessage {
            pos: burst.pos,
            cue,
        });
    }
    for &pos in &events.impacts {
        vfx.write(VfxMessage::Impact { pos });
        vfx.write(VfxMessage::Burst {
            pos,
            count: 14,
            speed: 300.0,
            color: [1.0, 0.34, 0.28, 0.88],
            kind: crate::fx::ParticleKind::Shard,
        });
        debris.write(DebrisBurstMessage {
            pos,
            cue: PhysicsDebrisCue::Impact,
        });
    }
    for &pos in &events.bursts {
        vfx.write(VfxMessage::Burst {
            pos,
            count: 16,
            speed: 230.0,
            color: [0.84, 0.95, 1.0, 0.82],
            kind: crate::fx::ParticleKind::Spark,
        });
    }
}

/// Marker on the per-frame projectile sprite entities produced by
/// `sync_projectile_visuals`. Despawned and rebuilt each tick so the
/// entity set always matches `PlayerProjectileState::bodies`.
#[derive(Component)]
pub struct PlayerProjectileVisual;

/// Mirror `PlayerProjectileState::bodies` onto Bevy sprite entities so
/// the player can actually see what they fired. Runs after
/// `update_projectiles` (which produces the body Vec) and on the
/// presentation half only — headless drains `state.bodies` without
/// needing visuals.
///
/// Despawn-and-respawn is the simplest match for a small ring of
/// short-lived projectiles (typical in-flight count is 1–3, capped by
/// the spawner's cooldown + resource meter). Anything fancier
/// (per-projectile entity reuse) would need a stable id on
/// `PlayerProjectile`, which today doesn't exist.
pub fn sync_projectile_visuals(
    mut commands: Commands,
    world: Res<crate::GameWorld>,
    state: Res<PlayerProjectileState>,
    assets: Option<Res<crate::game_assets::GameAssets>>,
    existing: Query<Entity, With<PlayerProjectileVisual>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let handle = assets
        .as_deref()
        .and_then(|a| a.entities.get(crate::game_assets::EntitySprite::ProjectileEnergy))
        .cloned();
    for projectile in &state.bodies {
        let body = &projectile.body;
        let render_size = bevy::math::Vec2::new(
            (body.half_extent.x * 2.0).max(8.0),
            (body.half_extent.y * 2.0).max(8.0),
        );
        // Hadouken tint (cooler / blue-shifted) vs Fireball (warmer
        // orange). The tint applies whether or not the textured sprite
        // loads; a missing texture falls through to a colored quad.
        let tint = match body.kind {
            ae::ProjectileKind::Fireball => Color::srgba(1.0, 0.74, 0.30, 0.95),
            ae::ProjectileKind::Hadouken => Color::srgba(0.45, 0.78, 1.0, 0.96),
        };
        let mut sprite = match handle.clone() {
            Some(image) => Sprite {
                image,
                color: tint,
                custom_size: Some(render_size),
                ..Default::default()
            },
            None => Sprite::from_color(tint, render_size),
        };
        // Flip the sprite to face travel direction so a leftward
        // fireball doesn't look upside-down.
        sprite.flip_x = body.vel.x < 0.0;
        commands.spawn((
            sprite,
            Transform::from_translation(crate::config::world_to_bevy(
                &world.0,
                body.pos,
                crate::config::WORLD_Z_PLAYER + 2.0,
            )),
            PlayerProjectileVisual,
            Name::new(match body.kind {
                ae::ProjectileKind::Fireball => "Player projectile: fireball",
                ae::ProjectileKind::Hadouken => "Player projectile: hadouken",
            }),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_engine::{Block, MotionDirection, ProjectileKind, World};

    fn dummy_world() -> World {
        World::new(
            "test",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            vec![Block::solid(
                "right wall",
                ae::Vec2::new(800.0, 100.0),
                ae::Vec2::new(40.0, 400.0),
            )],
        )
    }

    fn min_app() -> App {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        app.insert_resource(GameWorld(dummy_world()));
        let mut runtime = SandboxRuntime::new(
            &dummy_world(),
            ae::AbilitySet::sandbox_all(),
            ae::DEFAULT_TUNING,
            crate::physics::PhysicsSandboxSettings::default(),
        );
        runtime.player.pos = ae::Vec2::new(300.0, 300.0);
        runtime.player.facing = 1.0;
        app.insert_resource(runtime);
        app.insert_resource(ControlFrame::default());
        app.insert_resource(crate::settings::UserSettings::default());
        app.insert_resource(GameplayTraceBuffer::default());
        app.insert_resource(PlayerProjectileState::default());
        app.insert_resource(FeatureEventBus::default());
        // Buffered-message channels the system writes into.
        app.add_message::<SfxMessage>();
        app.add_message::<VfxMessage>();
        app.add_message::<DebrisBurstMessage>();
        app.add_systems(Update, update_projectiles);
        app
    }

    fn advance_time(app: &mut App, dt_seconds: f32) {
        let mut time = app.world_mut().resource_mut::<Time<()>>();
        time.advance_by(std::time::Duration::from_secs_f32(dt_seconds));
    }

    #[test]
    fn projectile_pressed_spawns_one_fireball() {
        let mut app = min_app();
        {
            let mut frame = app.world_mut().resource_mut::<ControlFrame>();
            frame.projectile_pressed = true;
        }
        advance_time(&mut app, 0.016);
        app.update();
        let state = app.world().resource::<PlayerProjectileState>();
        assert_eq!(state.bodies.len(), 1);
        assert_eq!(state.bodies[0].body.kind, ProjectileKind::Fireball);
    }

    #[test]
    fn half_circle_then_press_spawns_hadouken() {
        let mut app = min_app();
        {
            let mut state = app.world_mut().resource_mut::<PlayerProjectileState>();
            let mut t = 0.0;
            for dir in [
                MotionDirection::Right,
                MotionDirection::DownRight,
                MotionDirection::Down,
                MotionDirection::DownLeft,
                MotionDirection::Left,
            ] {
                state.motion_buffer.push(dir, t);
                t += 0.04;
            }
            state.clock = t;
        }
        {
            let mut frame = app.world_mut().resource_mut::<ControlFrame>();
            frame.projectile_pressed = true;
        }
        advance_time(&mut app, 0.016);
        app.update();
        let state = app.world().resource::<PlayerProjectileState>();
        assert_eq!(state.bodies.len(), 1);
        assert_eq!(state.bodies[0].body.kind, ProjectileKind::Hadouken);
    }

    #[test]
    fn cooldown_blocks_second_fire_in_same_window() {
        let mut app = min_app();
        {
            let mut frame = app.world_mut().resource_mut::<ControlFrame>();
            frame.projectile_pressed = true;
        }
        advance_time(&mut app, 0.016);
        app.update();
        {
            let mut frame = app.world_mut().resource_mut::<ControlFrame>();
            frame.projectile_pressed = true;
        }
        advance_time(&mut app, 0.016);
        app.update();
        let state = app.world().resource::<PlayerProjectileState>();
        assert_eq!(state.bodies.len(), 1);
    }

    #[test]
    fn out_of_resource_blocks_fire() {
        let mut app = min_app();
        {
            let mut state = app.world_mut().resource_mut::<PlayerProjectileState>();
            state.spawner.meter.current = 0.0;
        }
        {
            let mut frame = app.world_mut().resource_mut::<ControlFrame>();
            frame.projectile_pressed = true;
        }
        advance_time(&mut app, 0.016);
        app.update();
        let state = app.world().resource::<PlayerProjectileState>();
        assert!(state.bodies.is_empty());
    }

    /// Pre-spawn a fireball directly into the body list and place it
    /// just above an enemy. After one tick the fireball moves into
    /// the enemy's AABB → `apply_damage_event` does its thing → enemy
    /// loses HP and the projectile is despawned.
    #[test]
    fn fireball_damages_enemy_on_intersect() {
        let mut app = min_app();
        // Spawn an enemy in the runtime.
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            runtime.features.spawn_enemy(
                "test_enemy".into(),
                ae::EnemyBrain::Custom("medium_striker".into()),
                ae::Vec2::new(400.0, 300.0),
                ae::Vec2::new(28.0, 46.0),
            );
        }
        // Inject a fireball moving toward the enemy.
        {
            let mut state = app.world_mut().resource_mut::<PlayerProjectileState>();
            let spec = ae::ProjectileSpec::new(
                ae::ProjectileKind::Fireball,
                ae::Vec2::new(395.0, 300.0),
                ae::Vec2::new(1.0, 0.0),
                1.0,
            );
            let mut body = ae::ProjectileBody::from_spec(spec);
            // Override velocity / pos so the next tick definitely
            // overlaps the enemy AABB regardless of arc tuning.
            body.pos = ae::Vec2::new(395.0, 300.0);
            body.vel = ae::Vec2::new(50.0, 0.0);
            state.bodies.push(PlayerProjectile { body });
        }
        let starting_health = {
            let runtime = app.world().resource::<SandboxRuntime>();
            runtime.features.enemies[0].health.current
        };
        advance_time(&mut app, 0.016);
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        let state = app.world().resource::<PlayerProjectileState>();
        assert!(
            runtime.features.enemies[0].health.current < starting_health,
            "enemy must lose HP from a projectile hit (was {}, now {})",
            starting_health,
            runtime.features.enemies[0].health.current
        );
        assert!(
            state.bodies.is_empty(),
            "fireball must despawn after hitting an actor"
        );
    }

    /// Drop a fireball onto a floor block. The first tick should
    /// produce a bounce (vy reflects upward, bounce budget drops by
    /// one) and the projectile must remain in the body list.
    #[test]
    fn fireball_bounces_off_floor_in_system() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        // World with a single floor block well below the spawn point.
        let world = ae::World::new(
            "bounce_test",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            vec![ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 400.0),
                ae::Vec2::new(2000.0, 32.0),
            )],
        );
        app.insert_resource(GameWorld(world.clone()));
        let mut runtime = SandboxRuntime::new(
            &world,
            ae::AbilitySet::sandbox_all(),
            ae::DEFAULT_TUNING,
            crate::physics::PhysicsSandboxSettings::default(),
        );
        runtime.player.pos = ae::Vec2::new(200.0, 200.0);
        app.insert_resource(runtime);
        app.insert_resource(ControlFrame::default());
        app.insert_resource(crate::settings::UserSettings::default());
        app.insert_resource(GameplayTraceBuffer::default());
        app.insert_resource(PlayerProjectileState::default());
        app.insert_resource(FeatureEventBus::default());
        app.add_message::<SfxMessage>();
        app.add_message::<VfxMessage>();
        app.add_message::<DebrisBurstMessage>();
        app.add_systems(Update, update_projectiles);

        // Spawn a fireball just above the floor moving downward.
        let starting_bounces;
        {
            let mut state = app.world_mut().resource_mut::<PlayerProjectileState>();
            let spec = ae::ProjectileSpec::new(
                ae::ProjectileKind::Fireball,
                ae::Vec2::new(500.0, 380.0),
                ae::Vec2::new(1.0, 0.0),
                1.0,
            );
            let mut body = ae::ProjectileBody::from_spec(spec);
            body.pos = ae::Vec2::new(500.0, 395.0);
            body.vel = ae::Vec2::new(60.0, 240.0);
            starting_bounces = body.bounces_remaining;
            assert!(starting_bounces > 0);
            state.bodies.push(PlayerProjectile { body });
        }
        advance_time(&mut app, 0.016);
        app.update();
        let state = app.world().resource::<PlayerProjectileState>();
        assert_eq!(state.bodies.len(), 1, "fireball must survive a floor bounce");
        let body = &state.bodies[0].body;
        assert!(body.vel.y < 0.0, "post-bounce vy must be upward; got {}", body.vel.y);
        assert_eq!(body.bounces_remaining, starting_bounces - 1);
    }

    /// Hadouken spawns with `bounces_remaining = 0`. Hitting any solid
    /// expires it on the first contact — pinning the "horizontal
    /// projectile that disappears on first wall" behavior at the
    /// system level (engine test pinned it at the unit level).
    #[test]
    fn hadouken_expires_on_solid_in_system() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        let world = ae::World::new(
            "wall_test",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            vec![ae::Block::solid(
                "wall",
                ae::Vec2::new(600.0, 0.0),
                ae::Vec2::new(40.0, 800.0),
            )],
        );
        app.insert_resource(GameWorld(world.clone()));
        let mut runtime = SandboxRuntime::new(
            &world,
            ae::AbilitySet::sandbox_all(),
            ae::DEFAULT_TUNING,
            crate::physics::PhysicsSandboxSettings::default(),
        );
        runtime.player.pos = ae::Vec2::new(500.0, 300.0);
        app.insert_resource(runtime);
        app.insert_resource(ControlFrame::default());
        app.insert_resource(crate::settings::UserSettings::default());
        app.insert_resource(GameplayTraceBuffer::default());
        app.insert_resource(PlayerProjectileState::default());
        app.insert_resource(FeatureEventBus::default());
        app.add_message::<SfxMessage>();
        app.add_message::<VfxMessage>();
        app.add_message::<DebrisBurstMessage>();
        app.add_systems(Update, update_projectiles);

        {
            let mut state = app.world_mut().resource_mut::<PlayerProjectileState>();
            let spec = ae::ProjectileSpec::new(
                ae::ProjectileKind::Hadouken,
                ae::Vec2::new(580.0, 300.0),
                ae::Vec2::new(1.0, 0.0),
                1.0,
            );
            let mut body = ae::ProjectileBody::from_spec(spec);
            body.pos = ae::Vec2::new(595.0, 300.0);
            body.vel = ae::Vec2::new(520.0, 0.0);
            state.bodies.push(PlayerProjectile { body });
        }
        advance_time(&mut app, 0.016);
        app.update();
        let state = app.world().resource::<PlayerProjectileState>();
        assert!(
            state.bodies.is_empty(),
            "Hadouken must expire on first solid hit (no bounces); still alive: {}",
            state.bodies.len()
        );
    }
}
