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

use bevy::prelude::*;

use ambition_engine as ae;
use bevy::math::bounding::IntersectsVolume;

use crate::input::ControlFrame;
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
    runtime: Res<SandboxRuntime>,
    control_frame: Res<ControlFrame>,
    user_settings: Res<crate::settings::UserSettings>,
    mut state: ResMut<PlayerProjectileState>,
    mut trace: ResMut<GameplayTraceBuffer>,
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
        let aabb = p.body.aabb();
        let mut hit = false;
        for block in &world.0.blocks {
            if matches!(
                block.kind,
                ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
            ) && block.aabb.intersects(&aabb)
            {
                hit = true;
                break;
            }
        }
        if hit {
            events.push(ProjectileTraceEvent::Hit {
                kind: p.body.kind,
                damage: p.body.damage,
            });
            continue;
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
}
