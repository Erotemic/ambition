//! Ambition Engine
//!
//! This crate is the backend-neutral simulation layer for Ambition. Bevy owns
//! windowing, rendering, ECS scheduling, audio playback, and input plumbing.
//! This crate owns the rules that should remain testable without a renderer:
//! small math types, collision geometry, generated room specs, player movement,
//! combo traces, and simple sandbox enemy fixtures.
//!
//! The design goal is intentionally modest: keep the core game logic readable,
//! deterministic, and easy to port. If a future renderer replaces Bevy, this
//! crate should still be useful.

pub mod abilities;
pub mod combat;
pub mod enemy;
pub mod geometry;
pub mod math;
pub mod movement;
pub mod music;
pub mod world;

// Re-export the public surface so older sandbox code can continue to refer to
// `ambition_engine::Player`, `ambition_engine::World`, etc. Internally the code
// is now split by concern, but the crate remains convenient to use.
pub use abilities::AbilitySet;
pub use combat::slash_hitbox;
pub use enemy::{spawn_dummies, Dummy, DummyKind};
pub use geometry::Aabb;
pub use math::{approach, Vec2};
pub use movement::{
    blink_destination, update_player, update_player_with_tuning, BlinkEvent, ComboMark, FrameEvents,
    InputState, MovementOp, MovementTuning, Player, AIR_ACCEL, AIR_FRICTION, AIR_JUMPS,
    BLINK_COOLDOWN, BLINK_DISTANCE, BLINK_HOLD_THRESHOLD, COYOTE_TIME, DASH_COOLDOWN,
    DASH_SPEED, DASH_TIME, DEFAULT_TUNING, DOUBLE_JUMP_SPEED, FAST_FALL_ACCEL, FAST_FALL_SPEED,
    GRAVITY, GROUND_FRICTION, JUMP_BUFFER, JUMP_SPEED, MAX_FALL_SPEED, MAX_RUN_SPEED,
    POGO_SPEED, PRECISION_BLINK_DISTANCE, RUN_ACCEL, SLASH_RECOIL, WALL_JUMP_X,
    WALL_SLIDE_SPEED, WALL_CLIMB_SPEED,
};
pub use world::{build_endgame_sandbox, Block, BlockKind, World};
