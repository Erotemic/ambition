//! Per-player projectile controller state: charge machine, motion-input
//! buffer, and tracked unlocks. In-flight projectiles live as ECS entities;
//! this component is only the player's firing state.

use bevy::prelude::Component;

use ambition_gameplay_trace::GameplayTraceEvent;

/// Per-player projectile controller state: spawner cooldowns +
/// motion-input buffer + charge accumulator. Attached to each player
/// entity by `PlayerSimulationBundle`; `update_projectiles` iterates
/// every player and ticks their own state independently. In-flight
/// projectiles are separate ECS entities owned by this player.
#[derive(Component)]
pub struct PlayerProjectileState {
    pub spawner: crate::ProjectileSpawner,
    pub motion_buffer: crate::MotionInputBuffer,
    /// Time since first sample, in monotonic seconds.
    pub clock: f32,
    pub unlocked: ProjectileUnlocks,
    pub charge_tuning: crate::FireballChargeTuning,
    /// Hold-time accumulator for the fireball charge mechanic.
    /// `Some(t)` while the player is holding the fire button without
    /// having consumed the press for a Hadouken / HadoukenSuper.
    /// `None` when not charging.
    ///
    /// Lifecycle:
    /// - press WITHOUT a recent motion gesture → `Some(0.0)`.
    /// - hold → tick `dt`.
    /// - release → fire a Fireball with the tier derived from the
    ///   accumulated hold, then back to `None`.
    /// - press WITH a recent motion gesture → fires Hadouken/Super
    ///   immediately and stays `None` (no charge bleeds into the
    ///   following frame).
    pub charging: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectileUnlocks {
    pub fireball: bool,
    pub hadouken: bool,
    pub hadouken_super: bool,
}

impl Default for ProjectileUnlocks {
    fn default() -> Self {
        Self {
            fireball: true,
            hadouken: true,
            hadouken_super: true,
        }
    }
}

impl Default for PlayerProjectileState {
    fn default() -> Self {
        Self {
            spawner: crate::ProjectileSpawner::new(8.0, 1.5),
            motion_buffer: crate::MotionInputBuffer::new(0.45),
            clock: 0.0,
            unlocked: ProjectileUnlocks::default(),
            charge_tuning: crate::FireballChargeTuning::DEFAULT,
            charging: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProjectileTraceEvent {
    Fired {
        kind: crate::ProjectileKind,
    },
    BlockedByResource {
        kind: crate::ProjectileKind,
    },
    Hit {
        /// `None` for kind-less (enemy) shots in the unified step loop.
        kind: Option<crate::ProjectileKind>,
        damage: i32,
    },
    Expired {
        /// `None` for kind-less (enemy) shots in the unified step loop.
        kind: Option<crate::ProjectileKind>,
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
                kind: kind.map(|k| k.label()).unwrap_or("projectile").to_string(),
                event: "hit".into(),
                damage,
            },
            Self::Expired { kind } => GameplayTraceEvent::Projectile {
                tick,
                kind: kind.map(|k| k.label()).unwrap_or("projectile").to_string(),
                event: "expired".into(),
                damage: 0,
            },
        }
    }
}
