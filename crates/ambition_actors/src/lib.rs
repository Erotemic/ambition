//! Ambition's gameplay-systems ("machinery") layer.
//!
//! This crate owns the reusable game SYSTEMS — world/rooms, actors & brains,
//! player clusters, abilities, combat, items/inventory, dialog, menu, music,
//! persistence — assembled into Bevy plugins. It is the middle of the stack:
//!
//! - below it, `ambition_engine_core` is the pure (Bevy-free) movement model
//!   and `ambition_platformer_primitives` holds content-free Bevy primitives
//!   (kinematic body, gravity, projectile, lifecycle, schedule). Both are
//!   imported directly by their canonical crate paths.
//! - above it, `ambition_content` provides the named game DATA (rooms, bosses,
//!   rosters) and `ambition_app` does final wiring + the binaries.
//!
//! Despite the historical `ambition_actors` name, it is content-light: concrete
//! content has been migrated out to `ambition_content`. Foundation crates
//! (`ambition_combat`, `ambition_input`, `ambition_engine_core`, `ambition_characters`,
//! `ambition_render`, …) are imported directly by their canonical crate paths; the
//! old `crate::{input,engine_core,…}` compat re-exports have been removed.
//!
//! Top-level modules group coherent slices: `world`, `player`, `abilities`,
//! `combat`, `gravity`, `items`, `dialog`, `menu`, `music`,
//! `persistence`, `projectile`, `enemy_projectile`, `boss_encounter`,
//! `quest`, plus the `schedule`/`host`/`session` assembly and `dev` tooling.
//!
//! This crate owns the module graph and the cross-cutting types (`RoomGeometry`,
//! `SandboxSimState`) that submodules reference through the actor crate.
//! It is a library only; the playable app, the headless entry point
//! (`run_headless`), and all binaries live in `ambition_app`.
//!
//! Player state is authoritative on the 18 player cluster components
//! (`BodyKinematics`, `BodyGroundState`, …, `BodyComboTrace`).
//! Do not introduce a god-object runtime resource; add narrow resources or ECS
//! components instead.
//!
//! See `docs/systems/headless-simulation.md` for the sim/presentation contract this
//! library is being shaped toward, and `docs/adr/0012-sim-presentation-split-and-events-refactor.md` for the
//! longer-term events refactor that will let the player tick itself run
//! headless.

// External API surface — bins, tests, and Android/wasm entry points reach
// into these modules. Everything else stays `pub(crate)` so the compiler
// can tell us what's actually depended on from outside.
pub mod audio;
/// The HOME AVATAR — the body slot 0 owns and returns to, plus the policy that is
/// genuinely the local human's rather than any body's: its identity bundle, its
/// respawn safety and blink camera, its starting character, its emitted trail,
/// and the tick that integrates it. Formerly `player/`; the body vocabulary, the
/// control seam, the affordance table, and the body mechanics all left it in the
/// S5/S6 fold (refactor-chain R6). What is named here is named correctly.
pub mod avatar;
#[cfg(test)]
#[cfg(test)]
mod character_roster;
/// The local control seam: device frame -> slot -> the body carrying that slot's
/// player brain. See `control/mod.rs`.
pub mod control;
pub mod debug_label;
pub mod host;
pub mod platformer_runtime;
pub mod quest;
pub mod schedule;
// Stable facade for save-game data shapes used by dialogue bindings.
pub use persistence::save_data as save;

// Themed module umbrellas. Each owns a coherent slice of the sandbox.
pub mod abilities;
pub mod ability_cooldown;
/// Neutral actor-vocabulary home for shared sim-state (the keystone re-home target).
pub mod actor;
/// "What would each button do right now?" — the per-frame verb table the HUD
/// labels its buttons from. A BRIDGE (input x body x world -> verb), which is why
/// it is neither `control` nor `features`. Moved off `player/` in R6d.
pub mod affordances;
pub mod assets;
pub mod body_mode;
pub mod boss_encounter;
pub mod character_sprites;
pub mod config;
pub mod cutscene;
pub mod cutscene_trigger;
pub mod dev;
pub mod dialog;
pub mod encounter;
pub mod enemy_projectile;
pub mod items;
// Stable facade for dialogue shop bindings.
pub use items::shop;
// The combat kit is its own crate post-E2; the alias keeps `crate::combat::`
// paths resolving until the features hub dissolves (E7/E8 repoints consumers).
pub use ambition_combat as combat;
pub mod gravity;
pub mod music;
// Unified menu content (model + concrete settings IR + Map tab).
pub mod menu;
pub mod persistence;
pub mod physics;
// The presentation layer was extracted to the `ambition_render` crate (the
// sim/render seam is now a crate boundary). Consumers import `ambition_render::*`.
pub mod projectile;
pub mod session;
pub mod shrine;
pub mod time;
pub mod world;

// Public re-exports double as the external API for bins, tests, and docs.
pub mod features;
pub use dev::trace;
pub use world::{ldtk_world, rooms};

// Crate-root types/consts whose definitions live in themed modules of this
// crate. (Generic time vocabulary — `WorldTime`, `ClockState`, `ClockDomain`,
// `ProperTimeScale`, `refresh_world_time` — lives in `ambition_time`; name it
// there directly. Only the sandbox-owned `mirror_sim_dt_into_runtime` bridge
// still surfaces at the crate root.)
pub use time::move_toward;
pub use time::world_time::mirror_sim_dt_into_runtime;

pub use world::platforms::MovingPlatformState;

use ambition_engine_core as ae;
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
/// raw [`combat::HitEvent`] stream. Today the encounter system ignores both.
///
/// Replaces the previous `player_died_pending` bool — the Vec-collector →
/// `MessageWriter` pattern matches the rest of the sim → presentation seam
/// (`SfxMessage` / `VfxMessage` / `DebrisBurstMessage`).
#[derive(Message, Clone, Debug)]
pub struct ActorDiedMessage {
    pub pos: ae::Vec2,
    pub cause: DeathCause,
}

/// Attribution for an actor death — what dealt the killing blow.
///
/// Compact by design: the killing hit's [`combat::HitSource`] category plus the
/// attacker entity when the source carries one (player-side hits do; enemy /
/// boss / hazard sources identify by category only today — threading their
/// dealing entity is the deeper actor-attribution work). Reuses `HitSource`
/// rather than a parallel enum so a new attack source needs no second edit.
#[derive(Clone, Debug, PartialEq)]
pub struct DeathCause {
    /// The killing hit's source category (melee / projectile / hazard / …).
    pub source: combat::HitSource,
    /// The entity that dealt the killing blow, when known.
    pub attacker: Option<bevy::prelude::Entity>,
}

/// Per-frame conditions that gate writes to `SandboxSimState::last_safe_player_pos`.
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
// geometry) lives in `ambition_engine_core`, next to the `World` it wraps
// (fable review §D4, Jon-confirmed home) — so the renderer and a future
// `ambition_world` name it there directly, not through this 95k crate.

pub const BLINK_IN_ANIM_TIME: f32 = 0.34;
pub const ROOM_DOOR_CAMERA_SNAP_TIME: f32 = 0.08;

/// Pure simulation scalars for the running sandbox session.
/// Holds values that belong to the simulation, not to
/// developer/debug tools or presentation state.
///
/// **Multiplayer caveat:** each field has different per-player vs.
/// shared semantics for a future co-op build:
/// - Per-player "last safe position" lives on each player entity as
///   `crate::avatar::PlayerSafetyState`.
/// - `room_transition_cooldown` — **global shared-world** today
///   because the whole party shares one active room. If a future
///   build splits rooms per-player this would need to move per-room
///   or per-player.
#[derive(Resource, Clone, Copy, Debug)]
pub struct SandboxSimState {
    pub room_transition_cooldown: f32,
}

impl Default for SandboxSimState {
    fn default() -> Self {
        Self {
            room_transition_cooldown: 0.0,
        }
    }
}

/// The state of one in-flight player melee swing is now the unified
/// [`crate::features::MeleeSwing`] — the SAME swing every brain-driven actor
/// carries (the player is an actor). Re-exported at the crate root so existing
/// `crate::MeleeSwing` / `ambition_actors::MeleeSwing` paths resolve.
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
