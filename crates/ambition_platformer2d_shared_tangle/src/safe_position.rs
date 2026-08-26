//! Where the player was last standing safely, and the gate that decides it.
//!
//! ⛔⛔ THREE OF THESE FOUR WERE PARKED AT THE ACTOR MONOLITH'S CRATE ROOT — a
//! mechanic wearing the crate's address, which is the shape `Mass` had before
//! the mount carve. `PlayerSafetyState` sat one level down in `avatar`, and the
//! comment directly above it there had already written this move down for its
//! neighbour: *"every reader outside this crate sits ABOVE this crate, so owning
//! it here made the actor crate a way-station on an edge that never needed it."*
//!
//! ⭐ TWO DOMAINS, NEITHER ABLE TO OWN IT. The runtime's room transition and
//! reset write and clear it (`sandbox_reset`, `room_transition/commit`,
//! `sim_core_resources`); the actor crate's damage road gates it. That is
//! `shared_tangle`'s admission test said out loud.
//!
//! ⭐ THE GATE TAKES ONLY `_core` TYPES. Everything
//! `remember_safe_player_position` reads — the clusters, the world, the block
//! kinds, the safety classifier — belongs to `ambition_platformer2d_core`; the
//! single actor-owned input was `PlayerSafetyState`, which came with it.
//!
//! ⛔ THE STABLE ROLLBACK NAMES DO NOT MOVE. Both components are registered and
//! encoded; this is a repoint, not a schema change.

use ambition_platformer2d_core as ae;
use bevy::prelude::Resource;

/// Per-player "last known safe spot" used by hazard knockback and debug
/// respawn helpers. Stored on each player so future co-op builds keep safe
/// anchors independent.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerSafetyState {
    /// Last grounded, gameplay-safe position the safety gate approved (see
    /// [`remember_safe_player_position`]). The hazard / OOB respawn path warps
    /// the player here.
    pub last_safe_pos: ae::Vec2,
}

impl PlayerSafetyState {
    pub fn new(initial: ae::Vec2) -> Self {
        Self {
            last_safe_pos: initial,
        }
    }
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

/// Pure simulation scalars for the running sandbox session.
/// Holds values that belong to the simulation, not to
/// developer/debug tools or presentation state.
///
/// Multiplayer caveat: each field has different per-player vs.
/// shared semantics for a future co-op build:
/// - Per-player "last safe position" lives on each player entity as
///   `PlayerSafetyState`.
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
    safety: &mut PlayerSafetyState,
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
    safety: &mut PlayerSafetyState,
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
