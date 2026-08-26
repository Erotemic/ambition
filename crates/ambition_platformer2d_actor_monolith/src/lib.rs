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
use bevy::prelude::{Message, Resource};

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

/// Per-frame conditions that gate writes to `RoomTransitionCooldown::last_safe_player_pos`.
/// We refuse to record a position as "safe" while any of these flags are
/// set so an in-flight reset / hazard respawn / room transition cannot
/// pollute the safe spawn point. Construct with [`SafePositionContext::ideal`]
/// for the "no contraindications" baseline, then flip individual flags as the
/// frame's events fire.
#[derive(Clone, Copy, Debug)]
pub struct SafePositionContext {
    /// True if the player took damage this frame.
    pub damaged_this_frame: bool,
    /// True if hitstun is active (player has reduced control).
    pub in_hitstun: bool,
    /// True if a feature requested a player reset this frame.
    pub feature_requested_reset: bool,
    /// True if the post-blink grace timer is currently active.
    pub blink_grace_active: bool,
    /// True if a room transition fired or is cooling down this frame.
    pub room_transitioning: bool,
}

impl SafePositionContext {
    /// "All safe": no damage, no hitstun, no reset, no blink grace, no
    /// transition. Useful for tests.
    pub fn ideal() -> Self {
        Self {
            damaged_this_frame: false,
            in_hitstun: false,
            feature_requested_reset: false,
            blink_grace_active: false,
            room_transitioning: false,
        }
    }

    pub fn is_eligible(&self) -> bool {
        !self.damaged_this_frame
            && !self.in_hitstun
            && !self.feature_requested_reset
            && !self.blink_grace_active
            && !self.room_transitioning
    }
}

// `RoomGeometry` (the session-root component wrapping the active room's collision
// geometry) lives in `ambition_platformer2d_core`, next to the `World` it wraps
// — so the renderer and a future
// `ambition_platformer2d_world` name it there directly, not through this 95k crate.

pub const BLINK_IN_ANIM_TIME: f32 = 0.34;
pub const ROOM_DOOR_CAMERA_SNAP_TIME: f32 = 0.08;

/// Pure simulation scalars for the running sandbox session.
/// Holds values that belong to the simulation, not to
/// developer/debug tools or presentation state.
///
/// Multiplayer caveat: each field has different per-player vs.
/// shared semantics for a future co-op build:
/// - Per-player "last safe position" lives on each player entity as
///   `crate::avatar::PlayerSafetyState`.
/// - `remaining` — global shared-world today
///   because the whole party shares one active room. If a future
///   build splits rooms per-player this would need to move per-room
///   or per-player.
#[derive(Resource, Clone, Copy, Debug)]
pub struct RoomTransitionCooldown {
    pub remaining: f32,
}

impl Default for RoomTransitionCooldown {
    fn default() -> Self {
        Self { remaining: 0.0 }
    }
}

/// The state of one in-flight player melee swing is now the unified
/// [`crate::features::MeleeSwing`] — the SAME swing every brain-driven actor
/// carries (the player is an actor). Re-exported at the crate root so existing
/// `crate::MeleeSwing` / `ambition_platformer2d_actor_monolith::MeleeSwing` paths resolve.
pub use crate::features::MeleeSwing;

/// Record the current player position as "the last known safe spot"
/// when (and only when) every predicate of safety holds. Call sites pass
/// the same augmented collision world the engine simulated against this
/// frame so the gate matches reality.
///
/// The flags allow the caller to suppress this write during damage
/// resolution, hazard respawn, hitstun, post-blink grace, or room
/// transitions where the player position is intentionally being
/// teleported and shouldn't be remembered as safe. See
/// `dev/journals/lessons_learned.md` for the OOB trace where a wall-cling
/// teleport polluted `last_safe_player_pos` with `(62, -23)`.
pub fn remember_safe_player_position(
    safety: &mut crate::avatar::PlayerSafetyState,
    clusters: &ae::BodyClustersMut<'_>,
    world: &ae::World,
    ctx: SafePositionContext,
) {
    remember_safe_player_position_from_kinematics(
        safety,
        clusters.kinematics.pos,
        clusters.kinematics.vel,
        clusters.kinematics.aabb(),
        clusters.ground.on_ground,
        world,
        ctx,
    );
}

/// Tuple-arg variant of [`remember_safe_player_position`] for callers
/// that already hold the four kinematic facts the safety classifier
/// reads. The cluster wrapper above is the natural production path;
/// this tuple form is exposed for tests that build a
/// `BodyClusterScratch` and pass individual fields.
pub fn remember_safe_player_position_from_kinematics(
    safety: &mut crate::avatar::PlayerSafetyState,
    pos: ae::Vec2,
    vel: ae::Vec2,
    aabb: ae::Aabb,
    on_ground: bool,
    world: &ae::World,
    ctx: SafePositionContext,
) {
    if !on_ground {
        return;
    }
    if !ctx.is_eligible() {
        return;
    }
    let verdict = ae::classify_safety_from_kinematics(pos, vel, aabb, world, 0.0, |block| {
        matches!(
            block.kind,
            ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
        )
    });
    if verdict.is_safe() {
        safety.last_safe_pos = pos;
    }
}

#[cfg(test)]
mod safe_pos_tests;

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
