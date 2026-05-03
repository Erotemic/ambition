//! Ambition Engine
//!
//! This crate is the reusable Bevy-native mechanics layer for Ambition. It owns
//! features a game or story crate should be able to assemble without rewriting
//! details: movement, collision semantics, ability gates, combat hitboxes,
//! enemies, room geometry, generated audio/music specs, and testable gameplay
//! rules.
//!
//! Story/sandbox crates should generally provide data, presentation, and input
//! wiring. The engine may depend on small Bevy crates such as `bevy_math` when
//! they provide battle-tested primitives that are better than bespoke versions.

pub mod abilities;
pub mod actor;
pub mod boss_patterns;
pub mod combat;
pub mod debug;
pub mod enemy;
pub mod geometry;
pub mod interaction;
pub mod movement;
pub mod music;
pub mod physics;
pub mod scalar;
pub mod state_machines;
pub mod world;

// Re-export the public surface so story/sandbox crates can treat the engine as
// the main mechanics API while the internals stay organized by concern.
pub use abilities::AbilitySet;
pub use actor::{
    Actor, ActorFaction, ActorKind, BossBrain, EnemyBrain, Health, KinematicPath,
    KinematicPathMode, RespawnPolicy,
};
pub use bevy_math::Vec2;
pub use boss_patterns::{BossAttackKind, BossPatternSchedule, BossPatternStep};
pub use combat::{
    player_slash_hitbox, slash_hitbox, Damage, DamageKind, DamageVolume, Hitbox, Hurtbox,
};
pub use debug::{DebugLabel, DebugLabelKind, DestinationLabel};
pub use enemy::{spawn_dummies, Dummy, DummyKind};
pub use geometry::{aabb_from_min_size, Aabb, AabbExt};
pub use interaction::{
    Breakable, BreakableCollision, BreakableState, BreakableTrigger, Chest, ChestState,
    Interactable, InteractionKind, Pickup, PickupKind,
};
pub use movement::{
    blink_destination, blink_destination_to_point, update_player, update_player_control,
    update_player_control_with_tuning, update_player_simulation,
    update_player_simulation_with_tuning, update_player_with_tuning, BlinkEvent, ComboMark,
    FrameEvents, InputState, MovementOp, MovementTuning, Player, AIR_ACCEL, AIR_FRICTION,
    AIR_JUMPS, BLINK_COOLDOWN, BLINK_DISTANCE, BLINK_HOLD_THRESHOLD, COYOTE_TIME, DASH_BUFFER,
    DASH_COOLDOWN, DASH_SPEED, DASH_TIME, DEFAULT_TUNING, DOUBLE_JUMP_SPEED, FAST_FALL_ACCEL,
    FAST_FALL_SPEED, FLIGHT_ACCEL, FLIGHT_DRAG, FLIGHT_HOVER_HZ, FLIGHT_HOVER_SPEED,
    FLIGHT_TERMINAL_SPEED, GRAVITY, GROUND_FRICTION, JUMP_BUFFER, JUMP_SPEED, MAX_FALL_SPEED,
    MAX_RUN_SPEED, POGO_SPEED, PRECISION_BLINK_AIM_SPEED, PRECISION_BLINK_DISTANCE, RUN_ACCEL,
    SLASH_RECOIL, WALL_CLIMB_SPEED, WALL_JUMP_X, WALL_SLIDE_SPEED,
};
pub use physics::{
    PhysicsBodyKind, PhysicsBodyRole, PhysicsBodySpec, PhysicsMaterial, PhysicsShape, RagdollSpec,
};
pub use scalar::approach;
pub use state_machines::{
    state_machine_vocabulary, AmbitionStateMachineActor, AmbitionStateMachinePlugin, BossDefeated,
    BossDormant, BossIntro, BossPhase, BreakableBroken, BreakableCracking, BreakableIntact,
    BreakableRespawning, ChestClosed, ChestOpened, ChestOpening, EnemyAttack, EnemyDead, EnemyIdle,
    EnemyPatrol, EnemyRecover, EnemyStunned, EnemyTelegraph,
};
pub use world::{BlinkWallTier, Block, BlockKind, RoomObject, RoomObjectKind, World};
