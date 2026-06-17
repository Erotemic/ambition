//! Ambition's gameplay-systems ("machinery") layer.
//!
//! This crate owns the reusable game SYSTEMS — world/rooms, actors & brains,
//! player clusters, abilities, combat, items/inventory, dialog, menu, music,
//! persistence — assembled into Bevy plugins. It is the middle of the stack:
//!
//! - below it, `ambition_engine_core` is the pure (Bevy-free) movement model
//!   and `ambition_platformer_primitives` holds content-free Bevy primitives
//!   (kinematic body, gravity, projectile, lifecycle, schedule). Both are
//!   re-exported here at stable `crate::engine_core` / `crate::kinematic` paths.
//! - above it, `ambition_content` provides the named game DATA (rooms, bosses,
//!   rosters) and `ambition_app` does final wiring + the binaries.
//!
//! Despite the historical `ambition_sandbox` name, it is content-light: concrete
//! content has been migrated out to `ambition_content`. Other foundation crates
//! (`ambition_combat`, `ambition_input`, `ambition_characters`, `ambition_render`, …)
//! are re-exported under their historical `crate::*` paths so call sites resolve
//! unchanged.
//!
//! Top-level modules group coherent slices: `world`, `player`, `abilities`,
//! `mechanics`, `combat`, `items`/`inventory`, `dialog`, `menu`, `music`,
//! `persistence`, `effects`, `projectile`, `enemy_projectile`, `boss_encounter`,
//! `quest`, plus the `app`/`host`/`runtime` assembly and `dev` tooling.
//!
//! This crate owns the module graph and the cross-cutting types (`GameWorld`,
//! `SandboxSimState`, `SandboxDevState`) that submodules reference via `crate::*`.
//! It is a library only; the playable app, the headless entry point
//! (`run_headless`), and all binaries live in `ambition_app`.
//!
//! Player state is authoritative on the 18 player cluster components
//! (`BodyKinematics`, `PlayerGroundState`, …, `PlayerComboTrace`).
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
pub use ambition_characters::actor;
pub mod app;
pub mod audio;
pub mod character_roster;
// The pure combat MODEL (Damage / Hitbox / AttackSpec / DamageVolume / slots)
// is the reusable `ambition_combat` foundation crate. Re-exported at the
// historical `crate::combat` path so every `crate::combat::*` consumer resolves
// unchanged; the ECS combat SYSTEMS that consume it stay in `mechanics::combat`.
pub use ambition_combat as combat;
pub mod debug_label;
// Re-export the pure-logic core under the sandbox's stable `crate::engine_core` path.
pub use ambition_engine_core as engine_core;
// Re-export input types under the sandbox's stable `crate::input` path.
pub use ambition_input as input;
pub mod host;
pub use ambition_interaction as interaction;
// Re-export the reusable kinematic body/sweep runtime.
pub use ambition_platformer_primitives::kinematic;
pub mod platformer_runtime;
pub mod player;
pub mod quest;
// Stable facade for save-game data shapes used by dialogue bindings.
pub use persistence::save_data as save;

// Themed module umbrellas. Each owns a coherent slice of the sandbox.
pub mod abilities;
pub mod ability_cooldown;
pub mod assets;
pub mod body_mode;
pub mod boss_encounter;
pub mod character_sprites;
pub use ambition_characters::brain;
pub mod config;
pub mod cutscene_trigger;
pub mod dev;
pub mod dialog;
// Test-only (`#![cfg(test)]`): static arity lint for the Yarn dialogue commands.
mod dialog_lint;
pub mod effects;
pub mod encounter;
pub mod enemy_projectile;
#[cfg(feature = "falling_sand")]
pub mod falling_sand;
pub mod inventory;
pub mod items;
pub mod player_rope;
// Stable facade for dialogue shop bindings.
pub use items::shop;
pub mod mechanics;
pub mod music;
// Unified menu content (model + concrete settings IR + Map tab). See
// `docs/planning/unified_tabbed_menu.md` §10.
pub mod menu;
pub mod persistence;
pub mod physics;
#[cfg(feature = "portal")]
pub mod portal;
// The presentation layer was extracted to the `ambition_render` crate (the
// sim/render seam is now a crate boundary). Consumers import `ambition_render::*`.
pub mod projectile;
pub mod runtime;
pub mod shrine;
pub mod time;
pub use ambition_ui_nav as ui_nav;
pub mod world;

// Public re-exports double as the external API for bins, tests, and docs.
pub mod features;
pub use dev::trace;
pub use runtime::game_mode;
pub use world::{ldtk_world, rooms};

// Crate-root types/consts whose definitions moved into themed modules but
// still need to surface at `crate::WorldTime` / `ambition_sandbox::WorldTime`.
pub use time::camera_ease::{
    CameraEaseState, CameraEaseTuning, DEFAULT_CAMERA_ZOOM_IN_RATE, DEFAULT_CAMERA_ZOOM_OUT_RATE,
    DEFAULT_CAMERA_ZOOM_SNAP_EPSILON,
};
pub use time::clock_state::ClockState;
pub use time::move_toward;
pub use time::world_time::{
    mirror_sim_dt_into_runtime, refresh_world_time, ClockDomain, WorldTime,
};

pub use game_mode::{gameplay_allowed, GameMode};

// Re-export the types that leak through public crate-root signatures
// (`MovingPlatformSet.0`, `WorldTime::entity_dt`) so the modules they
// live in can stay `pub(crate)`. Without these, downstream callers
// couldn't name the types cleanly — `platforms::MovingPlatformState`
// wouldn't be reachable via any pub path — and Rust's
// private-interfaces lint would fire under `-D warnings`.
pub use time::time_control::ProperTimeScale;
pub use world::platforms::MovingPlatformState;

use crate::engine_core as ae;
use bevy::prelude::{Message, Resource};

use input::KeyboardPreset;

/// Sandbox-side death notification. Emitted from `death_respawn_player`
/// the frame the player's HP drops to zero and they respawn at the room
/// spawn. The encounter system reads this through `MessageReader` to
/// fail any in-flight encounter (despawn mobs, drop the lock wall,
/// re-arm the trigger) without sandbox-runtime polling.
///
/// `pos` carries the impact location for downstream consumers (vfx,
/// future death-replay tooling). Today the encounter system ignores it.
///
/// Replaces the previous `player_died_pending` bool — the Vec-collector →
/// `MessageWriter` pattern matches the rest of the sim → presentation seam
/// (`SfxMessage` / `VfxMessage` / `DebrisBurstMessage`).
#[derive(Message, Clone, Copy, Debug)]
pub struct PlayerDiedMessage {
    pub pos: ae::Vec2,
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

/// Active room's collision world, exposed as a Bevy resource.
///
/// Sandbox systems read collision through this wrapper so simulation logic
/// stays decoupled from how the world was authored. LDtk hot reload mutates
/// this resource as part of the transactional reload path.
#[derive(Resource, Clone)]
pub struct GameWorld(pub ae::World);

pub const BLINK_IN_ANIM_TIME: f32 = 0.34;
pub const ROOM_DOOR_CAMERA_SNAP_TIME: f32 = 0.08;

/// Live platform-simulation state for the current room.
///
/// Owned by the physics/rendering pipeline; the player tick advances each
/// platform per frame and carries the player by its delta. The physics plugin
/// registers this as a resource; the room-load path (setup, load_room,
/// LDtk hot-reload, sandbox reset) replaces the Vec when the active room
/// changes.
#[derive(Resource, Default)]
pub struct MovingPlatformSet(pub Vec<world::platforms::MovingPlatformState>);

/// Pure simulation scalars for the running sandbox session.
/// Holds values that belong to the simulation, not to
/// developer/debug tools or presentation state.
///
/// **Multiplayer caveat:** each field has different per-player vs.
/// shared semantics for a future co-op build:
/// - Per-player "last safe position" lives on each player entity as
///   `crate::player::PlayerSafetyState`.
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

/// Developer/debug state: keyboard preset selection and debug flags.
#[derive(Resource)]
pub struct SandboxDevState {
    pub debug: bool,
    pub slowmo: bool,
    pub presets: Vec<KeyboardPreset>,
    pub preset_index: usize,
    pub preset_flash: f32,
}

impl Default for SandboxDevState {
    fn default() -> Self {
        Self {
            debug: !cfg!(target_os = "android"),
            slowmo: false,
            presets: KeyboardPreset::presets().to_vec(),
            preset_index: 0,
            preset_flash: 1.2,
        }
    }
}

impl SandboxDevState {
    pub fn preset(&self) -> KeyboardPreset {
        self.presets[self.preset_index]
    }

    pub fn debug_enabled(&self) -> bool {
        self.debug
    }
}

/// Sandbox-side state for one active player melee swing.
#[derive(Clone, Debug)]
pub struct PlayerAttackState {
    pub spec: crate::combat::AttackSpec,
    pub elapsed: f32,
    pub hit_targets: Vec<String>,
    pub active_started: bool,
    /// True once a downward/pogo active-frame attack has produced its bounce.
    /// Prevents one long active window from repeatedly bouncing every frame.
    pub pogo_applied: bool,
}

impl PlayerAttackState {
    pub fn new(spec: crate::combat::AttackSpec) -> Self {
        Self {
            spec,
            elapsed: 0.0,
            hit_targets: Vec::new(),
            active_started: false,
            pogo_applied: false,
        }
    }

    pub fn phase(&self) -> Option<crate::combat::AttackPhase> {
        self.spec.phase_at(self.elapsed)
    }

    pub fn done(&self) -> bool {
        self.phase().is_none()
    }

    pub fn progress(&self) -> f32 {
        (self.elapsed / self.spec.total_seconds().max(0.001)).clamp(0.0, 1.0)
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
    safety: &mut crate::player::PlayerSafetyState,
    clusters: &ae::PlayerClustersMut<'_>,
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
/// `PlayerClusterScratch` and pass individual fields.
pub fn remember_safe_player_position_from_kinematics(
    safety: &mut crate::player::PlayerSafetyState,
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
mod safe_pos_tests {
    use super::*;
    use crate::engine_core::Block;

    fn dummy_world() -> ae::World {
        ae::World::new(
            "test",
            ae::Vec2::new(1800.0, 1800.0),
            ae::Vec2::new(170.0, 1695.0),
            vec![Block::solid(
                "left wall",
                ae::Vec2::new(0.0, 0.0),
                ae::Vec2::new(36.0, 1800.0),
            )],
        )
    }

    fn player_at(
        world: &ae::World,
        pos: ae::Vec2,
    ) -> (ae::PlayerClusterScratch, crate::player::PlayerSafetyState) {
        let mut scratch =
            crate::player::primary_player_scratch(world.spawn, ae::AbilitySet::sandbox_all());
        ae::refresh_movement_resources_clusters(
            &scratch.abilities,
            &mut scratch.dash,
            &mut scratch.jump,
            ae::DEFAULT_TUNING,
        );
        scratch.kinematics.pos = pos;
        scratch.ground.on_ground = true;
        // Force a known starting "safe pos" we can detect changes from.
        let safety = crate::player::PlayerSafetyState::new(ae::Vec2::new(170.0, 1695.0));
        (scratch, safety)
    }

    /// The OOB y=-23 position (above the world envelope) must NOT be
    /// recorded as safe even though `on_ground` is true. This is the
    /// invariant the wall-cling teleport bug violated for two consecutive
    /// reproductions before the fix.
    #[test]
    fn rejects_position_above_world_envelope() {
        let world = dummy_world();
        let (player, mut safety) = player_at(&world, ae::Vec2::new(62.0, -23.0));
        let initial = safety.last_safe_pos;
        remember_safe_player_position_from_kinematics(
            &mut safety,
            player.kinematics.pos,
            player.kinematics.vel,
            player.kinematics.aabb(),
            player.ground.on_ground,
            &world,
            SafePositionContext::ideal(),
        );
        assert_eq!(
            safety.last_safe_pos, initial,
            "above-world position must not become last_safe_pos"
        );
    }

    /// The position update should fire when the player is grounded inside
    /// the world envelope and not overlapping a Solid.
    #[test]
    fn accepts_legitimate_grounded_position() {
        let world = dummy_world();
        let (player, mut safety) = player_at(&world, ae::Vec2::new(200.0, 900.0));
        remember_safe_player_position_from_kinematics(
            &mut safety,
            player.kinematics.pos,
            player.kinematics.vel,
            player.kinematics.aabb(),
            player.ground.on_ground,
            &world,
            SafePositionContext::ideal(),
        );
        assert_eq!(
            safety.last_safe_pos,
            ae::Vec2::new(200.0, 900.0),
            "a legal grounded position should be remembered"
        );
    }

    /// Even if the player is grounded somewhere legitimate, an in-flight
    /// reset / damage / hitstun / blink / room transition must veto the
    /// write so the safe pos doesn't drift while the player is being
    /// teleported.
    #[test]
    fn vetoes_write_during_damage_or_reset() {
        let world = dummy_world();
        let (player, mut safety) = player_at(&world, ae::Vec2::new(200.0, 900.0));
        let initial = safety.last_safe_pos;

        let mut ctx = SafePositionContext::ideal();
        ctx.damaged_this_frame = true;
        remember_safe_player_position_from_kinematics(
            &mut safety,
            player.kinematics.pos,
            player.kinematics.vel,
            player.kinematics.aabb(),
            player.ground.on_ground,
            &world,
            ctx,
        );
        assert_eq!(safety.last_safe_pos, initial);

        let mut ctx = SafePositionContext::ideal();
        ctx.feature_requested_reset = true;
        remember_safe_player_position_from_kinematics(
            &mut safety,
            player.kinematics.pos,
            player.kinematics.vel,
            player.kinematics.aabb(),
            player.ground.on_ground,
            &world,
            ctx,
        );
        assert_eq!(safety.last_safe_pos, initial);

        let mut ctx = SafePositionContext::ideal();
        ctx.in_hitstun = true;
        remember_safe_player_position_from_kinematics(
            &mut safety,
            player.kinematics.pos,
            player.kinematics.vel,
            player.kinematics.aabb(),
            player.ground.on_ground,
            &world,
            ctx,
        );
        assert_eq!(safety.last_safe_pos, initial);

        let mut ctx = SafePositionContext::ideal();
        ctx.blink_grace_active = true;
        remember_safe_player_position_from_kinematics(
            &mut safety,
            player.kinematics.pos,
            player.kinematics.vel,
            player.kinematics.aabb(),
            player.ground.on_ground,
            &world,
            ctx,
        );
        assert_eq!(safety.last_safe_pos, initial);

        let mut ctx = SafePositionContext::ideal();
        ctx.room_transitioning = true;
        remember_safe_player_position_from_kinematics(
            &mut safety,
            player.kinematics.pos,
            player.kinematics.vel,
            player.kinematics.aabb(),
            player.ground.on_ground,
            &world,
            ctx,
        );
        assert_eq!(safety.last_safe_pos, initial);
    }

    /// A position INSIDE a Solid block is not safe even if `on_ground`
    /// is true. Mirror of `classify_player_safety`'s InsideSolid case.
    #[test]
    fn rejects_position_inside_solid() {
        let world = dummy_world();
        // The left wall's right edge is at x=36; place the player center
        // at x=18 with half-width 14, body covers x=4..32 — fully inside
        // the wall.
        let (player, mut safety) = player_at(&world, ae::Vec2::new(18.0, 900.0));
        let initial = safety.last_safe_pos;
        remember_safe_player_position_from_kinematics(
            &mut safety,
            player.kinematics.pos,
            player.kinematics.vel,
            player.kinematics.aabb(),
            player.ground.on_ground,
            &world,
            SafePositionContext::ideal(),
        );
        assert_eq!(safety.last_safe_pos, initial);
    }

    #[test]
    fn register_down_tap_returns_true_on_double_tap_within_window() {
        let mut interaction = crate::player::PlayerInteractionState::default();
        // First tap: returns false, opens the window.
        assert!(!interaction.register_down_tap(true, 0.0, 0.25));
        // Tap again before window expires: returns true.
        assert!(interaction.register_down_tap(true, 0.05, 0.25));
    }

    #[test]
    fn register_down_tap_window_closes_on_idle_frames() {
        let mut interaction = crate::player::PlayerInteractionState::default();
        assert!(!interaction.register_down_tap(true, 0.0, 0.25));
        // Many idle frames — tap timer drains.
        for _ in 0..20 {
            let _ = interaction.register_down_tap(false, 0.05, 0.25);
        }
        // Next tap is treated as the FIRST tap (window had closed).
        assert!(!interaction.register_down_tap(true, 0.0, 0.25));
    }

    #[test]
    fn register_up_tap_mirrors_down_tap_semantics() {
        let mut interaction = crate::player::PlayerInteractionState::default();
        assert!(!interaction.register_up_tap(true, 0.0, 0.30));
        assert!(interaction.register_up_tap(true, 0.05, 0.30));
    }

    #[test]
    fn buffered_interact_holds_for_window_seconds() {
        let mut interaction = crate::player::PlayerInteractionState::default();
        // Press once → buffer holds for `window` seconds.
        assert!(interaction.buffered_interact(true, 0.0, 0.12));
        // Subsequent frames within the window also report true.
        assert!(interaction.buffered_interact(false, 0.05, 0.12));
        assert!(interaction.buffered_interact(false, 0.05, 0.12));
        // After the window passes, the buffer drains.
        assert!(!interaction.buffered_interact(false, 0.20, 0.12));
    }

    #[test]
    fn clear_interact_buffer_drops_buffer_immediately() {
        let mut interaction = crate::player::PlayerInteractionState::default();
        let _ = interaction.buffered_interact(true, 0.0, 1.0);
        // Without the clear, next frame would still report true.
        interaction.clear();
        assert!(!interaction.buffered_interact(false, 0.001, 1.0));
    }
}
