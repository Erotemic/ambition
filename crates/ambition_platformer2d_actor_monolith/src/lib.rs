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
#[cfg(feature = "causal")]
pub mod causal;
#[cfg(test)]
mod character_roster;
pub mod construction;
/// The local control seam: device frame -> slot -> the body carrying that slot's
/// player brain. See `control/mod.rs`.
pub mod control;
pub mod host;
pub mod platformer_runtime;
pub mod quest;
pub mod schedule;
// Stable facade for save-game data shapes used by dialogue bindings.

// Themed module umbrellas. Each owns a coherent slice of the sandbox.
pub mod abilities;
pub mod ability_cooldown;
pub mod action_scheme;
/// Neutral actor-vocabulary home for shared sim-state (the keystone re-home target).
pub mod actor;
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
pub use items::shop;
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

use ambition_platformer2d_core as ae;
use bevy::prelude::{Message};

/// Sandbox-side actor-death notification. Emitted from `death_respawn_player`
/// the frame a controlled actor's HP drops to zero and it respawns at the room
/// spawn. The encounter system reads this through `MessageReader` to fail any
/// in-flight encounter (despawn mobs, drop the lock wall, re-arm the trigger)
/// without sandbox-runtime polling.
///
/// Named for the *actor* role, not "player": the relativity principle wants
/// death framed as a fact about whichever controlled actor died, so this stays
/// correct when more than the local player can die (multiplayer / scripted
/// actors). Today only the controlled player routes through it.
///
/// `pos` carries the impact location for downstream consumers (vfx, future
/// death-replay tooling). `cause` carries the attribution — what dealt the
/// killing blow — so causality exists for future death-replay / multiplayer
/// kill-credit without a downstream consumer having to reconstruct it from the
/// raw [`ambition_combat::HitEvent`] stream. Today the encounter system ignores both.
///
/// Replaces the previous `player_died_pending` bool — the Vec-collector →
/// `MessageWriter` pattern matches the rest of the sim → presentation seam
/// (`SfxMessage` / `VfxMessage` / `DebrisBurstMessage`).
#[derive(Message, Clone, Debug)]
pub struct ActorDiedMessage {
    /// WHO died.
    ///
    /// this message carried no victim at all, so a consumer could only take the last death
    /// and assume it was theirs. Mary-O does exactly that — reads the latest message and
    /// applies it to the current `ControlledSubject` — and it works only because emission is
    /// effectively restricted to the one controlled body today.
    ///
    /// an `Entity` is a SAME-FRAME identity, not a durable one. Bevy
    /// recycles indices, so this is right for a consumer filtering "was that my
    /// body, this tick" and wrong for a replay or a peer. A durable
    /// victim identity — participant, or the body's stable
    /// `PresentationSourceId` — is what multiplayer attribution will need, and
    /// naming that here is the point of writing it down rather than discovering
    /// it later.
    pub victim: bevy::prelude::Entity,
    pub pos: ae::Vec2,
    pub cause: DeathCause,
}

/// Attribution for an actor death — what dealt the killing blow.
///
/// Compact by design: the killing hit's [`ambition_combat::HitSource`] category plus the
/// attacker entity when the source carries one (player-side hits do; enemy /
/// boss / hazard sources identify by category only today — threading their
/// dealing entity is the deeper actor-attribution work). Reuses `HitSource`
/// rather than a parallel enum so a new attack source needs no second edit.
#[derive(Clone, Debug, PartialEq)]
pub struct DeathCause {
    /// The killing hit's source category (melee / projectile / hazard / …).
    pub source: ambition_combat::HitSource,
    /// The entity that dealt the killing blow, when known.
    pub attacker: Option<bevy::prelude::Entity>,
}

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

pub const BLINK_IN_ANIM_TIME: f32 = 0.34;
pub const ROOM_DOOR_CAMERA_SNAP_TIME: f32 = 0.08;



/// The state of one in-flight player melee swing is now the unified
/// [`crate::features::MeleeSwing`] — the SAME swing every brain-driven actor
/// carries (the player is an actor). Re-exported at the crate root so existing
/// `crate::MeleeSwing` / `ambition_platformer2d_actor_monolith::MeleeSwing` paths resolve.
pub use crate::features::MeleeSwing;



#[cfg(test)]
mod safe_pos_tests;

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
