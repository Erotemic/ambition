//! Gameplay-system assembly layer for platformer actors, world interaction, abilities,
//! items, sessions, and related Bevy plugins.
//!
//! Pure movement lives below this crate in `ambition_platformer2d_core`; content-free
//! Bevy primitives live in `ambition_platformer2d_shared_tangle`; named game content
//! lives above it. This crate is intended to converge on assembly/cross-domain wiring,
//! with coherent gameplay domains carved into their own crates rather than growing new
//! resident subsystems here. Player simulation remains component-authoritative; do not
//! introduce a replacement god-object runtime resource.

// External API surface — bins, tests, and Android/wasm entry points reach
// into these modules. Everything else stays `pub(crate)` so the compiler
// can tell us what's actually depended on from outside.
pub mod audio;
/// The HOME AVATAR — the body slot 0 owns and returns to, plus the policy that is genuinely the
/// local human's rather than any body's: its identity bundle, its respawn safety and blink camera,
/// its starting character, its emitted trail, and the tick that integrates it. What is named here
/// is named correctly.
pub mod avatar;
pub mod brain_tick;
#[cfg(feature = "causal")]
pub mod causal;
#[cfg(test)]
mod character_roster;
pub mod construction;
/// The local control seam: device frame -> slot -> the body carrying that slot's
/// player brain. See `control/mod.rs`.
pub mod control;
pub mod host;
// ⛔ `platformer_runtime` IS GONE (2026-08-30), and its own TODO is what asked
// for it: *"migrate `platformer_runtime::{lifecycle,schedule,math,transit,body,
// collision}` callers to their owning crates, then delete those re-export
// modules and leave no generic runtime facade inside the actor monolith."* The
// callers had migrated; what was left was two EMPTY files, a `mod.rs` of `pub
// mod` lines, and a prelude re-exporting one function that every caller already
// named through `ambition_platformer2d_core::cast`. Four workspace policies
// described the shape of that shim; `engine.no-generic-runtime-facade-in-the-actor-crate`
// now states the rule the shim was standing in for.
pub mod quest;
pub mod schedule;
// Stable facade for save-game data shapes used by dialogue bindings.

// Themed module umbrellas. Each owns a coherent slice of the sandbox.
pub mod abilities;
pub mod ability_cooldown;
pub mod action_scheme;
/// Neutral actor-vocabulary home for shared sim-state (the keystone re-home target).
/// "What would each button do right now?" — the per-frame verb table the HUD
/// labels its buttons from. A BRIDGE (input x body x world -> verb), which is why
/// it is neither `control` nor `features`. Moved off `player/` in R6d.
pub mod assets;
pub mod body_custody;
pub mod body_mode;
mod checkpoint_horizon;
pub use checkpoint_horizon::ActorCheckpointHorizonPlugin;
pub mod character_runtime;
pub mod character_sprites;
pub mod config;
// No facade re-export stands here on purpose: callers name the crate. The departure and what earned
// it are in that crate's header; the short version is that its zero import edges were never the
// blocker — the SCHEDULE was.
pub mod cutscene;
pub mod dev;
pub mod encounter;
#[cfg(test)]
pub mod enemy_projectile;
pub mod items;
// Stable facade for dialogue shop bindings.
pub mod gravity;
pub mod music;
// Unified menu content (model + concrete settings IR + Map tab).
/// The `ParticipantId` ↔ `PlayerSlot` correspondence, in one place, so the
/// eventual split of "a person" from "a seat" is localized.
pub mod participant_seat;
// The presentation layer was extracted to the `ambition_render` crate (the
// sim/render seam is now a crate boundary). Consumers import `ambition_render::*`.
pub mod projectile;
pub mod session;
pub mod shrine;
mod snapshot_impls;
pub mod time;
pub mod world;
pub mod world_facts;

// Public re-exports double as the external API for bins, tests, and docs.
pub mod features;
pub use dev::trace;
pub use world::rooms;

// Crate-root types/consts whose definitions live in themed modules of this
// crate. (Generic time vocabulary — `WorldTime`, `ClockState`, `ClockDomain`,
// `ProperTimeScale`, `refresh_world_time` — lives in `ambition_time`; name it
// there directly. Only the sandbox-owned `mirror_sim_dt_into_runtime` bridge
// still surfaces at the crate root.)
pub use time::move_toward;
pub use time::world_time::{mirror_sim_dt_into_runtime, SimDtMirrored};

// ⛔ THE ACTOR-DEATH ANNOUNCEMENT LEFT THIS CRATE ROOT, 2026-08-26.
// `ActorDiedMessage` and `DeathCause` are `ambition_combat::death_rules`', beside
// `BodyKnockedOut` and the death rules they belong with — the runtime, two demos
// and five app tests read them, and `DeathCause` is built from combat's own
// `HitSource`. Imported, never re-exported.

// ⛔ THE SAFE-POSITION MEMORY LEFT THIS CRATE ROOT, 2026-08-26.
// `SafePositionContext`, `RoomTransitionCooldown`,
// `remember_safe_player_position` and `PlayerSafetyState` are ONE mechanic, and
// three of the four were parked here — a mechanic wearing the crate's address,
// the same shape `Mass` had before the mount carve. They live in
// `ambition_platformer2d_shared_tangle::safe_position` now, below the runtime's
// room transition and this crate's damage road alike. Imported, never
// re-exported: a `pub use` here would let callers keep spelling the old address.

// `RoomGeometry` (the session-root component wrapping the active room's collision
// geometry) lives in `ambition_platformer2d_core`, next to the `World` it wraps
// — so the renderer and a future
// `ambition_platformer2d_world` name it there directly, not through this 95k crate.

pub const ROOM_DOOR_CAMERA_SNAP_TIME: f32 = 0.08;

#[cfg(test)]
mod safe_pos_tests;

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
