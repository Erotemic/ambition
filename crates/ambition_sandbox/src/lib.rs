//! Ambition sandbox library.
//!
//! The sandbox crate exposes both the playable Bevy app (`src/main.rs`) and a
//! headless simulation entry point (`run_headless`, used by `bin/headless.rs`
//! and tests/CI on machines without a display). Both binaries depend on this
//! library; the library owns the module graph and the cross-cutting types
//! (`GameWorld`, `SandboxRuntime`) that submodules reference via `crate::*`.
//!
//! See `docs/headless_simulation.md` for the sim/presentation contract this
//! library is being shaped toward, and `docs/architecture_targets.md` for the
//! longer-term events refactor that will let `sandbox_update` itself run
//! headless.

pub mod audio;
pub mod body_mode;
pub mod bubble_shield;
pub mod boss_encounter;
pub mod boss_sprites;
pub mod character_sprites;
pub mod config;
pub mod content_validation;
pub mod cutscene;
pub mod data;
pub mod debug_overlay;
pub mod dev_tools;
pub mod dialog;
pub mod encounter;
pub mod features;
pub mod feel;
pub mod fx;
pub mod game_assets;
pub mod game_mode;
pub mod input;
pub mod inventory;
pub mod ldtk_world;
pub mod ledge_grab;
pub mod loading;
pub mod map_menu;
pub mod mechanics;
pub mod mobile_input;
pub mod music;
pub mod parallax;
pub mod pause_menu;
pub mod physics;
pub mod player;
pub mod platform;
pub mod platforms;
pub mod profiling;
pub mod projectile;
pub mod quest;
pub mod rendering;
pub mod reset;
pub mod room_builder;
pub mod rooms;
pub mod save;
pub mod settings;
pub mod swim;
pub mod trace;
pub mod ui_fonts;
pub mod ui_nav;
pub mod windowing;

pub mod app;
pub mod headless;
#[cfg(feature = "rl_sim")]
pub mod rl_sim;
pub mod setup;

/// Android shared-library entry point.
///
/// Desktop builds enter through `src/main.rs`, but the Android Gradle project
/// packages this crate as `libambition_sandbox.so`. GameActivity /
/// android-activity expects that shared library to export `android_main`;
/// Bevy's `#[bevy_main]` macro generates that boilerplate and registers the
/// Android app handle for `bevy_winit` before calling into our normal visible
/// app builder.
#[cfg(target_os = "android")]
#[bevy::prelude::bevy_main]
fn main() {
    app::run_visible();
}

pub use game_mode::GameMode;
pub use headless::{run_headless, HeadlessReport};
#[cfg(feature = "rl_sim")]
pub use rl_sim::{AgentAction, AgentObservation, SandboxSim, SandboxSimOptions};

use ambition_engine as ae;
use bevy::prelude::{Message, Resource};

use feel::SandboxFeelTuning;
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
/// Replaces the previous `SandboxRuntime::player_died_pending` bool —
/// the Vec-collector → `MessageWriter` pattern matches the rest of the
/// sim → presentation seam (`SfxMessage` / `VfxMessage` /
/// `DebrisBurstMessage`) and keeps the runtime resource a pure state
/// store rather than a half-event-channel.
#[derive(Message, Clone, Copy, Debug)]
pub struct PlayerDiedMessage {
    pub pos: ae::Vec2,
}

/// Per-frame conditions that gate writes to `SandboxRuntime::last_safe_player_pos`.
/// We refuse to record a position as "safe" while any of these flags are
/// set so an in-flight reset / hazard respawn / room transition cannot
/// pollute the safe spawn point. Construct with [`SafePositionContext::frame`].
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

/// Legacy sandbox scratch state used by older phase helpers.
///
/// Player movement/health/combat/interaction now have ECS components; the
/// player fields here are synchronized from/to that entity while the remaining
/// monolithic phase helpers are split apart.
///
/// AMBITION_REVIEW: this is currently a global resource holding what belongs
/// on a Player entity. Per the architecture targets memory, per-player state
/// should migrate onto a Player component / entity once the events refactor
/// lands; a global `SandboxRuntime` is the SP-only shape that does not extend
/// to multi-player. The headless binary deliberately does not install this
/// resource — Phase 1 headless validates only the asset/world/spine pipeline,
/// not gameplay.
pub const BLINK_IN_ANIM_TIME: f32 = 0.34;
pub const ROOM_DOOR_CAMERA_SNAP_TIME: f32 = 0.08;

/// Active player melee swing, isolated in its own resource so rendering and
/// debug systems can read it without acquiring the full `SandboxRuntime` lock.
/// `None` when no swing is in progress.
///
/// Authoritative source: set/cleared by `start_attack` / `advance_attack`
/// inside `sandbox_update`. `write_player_ecs_components` mirrors
/// `is_some()` into `PlayerCombatState::attacking` each frame.
#[derive(Resource, Default)]
pub struct CurrentPlayerAttack(pub Option<PlayerAttackState>);

/// Live platform-simulation state for the current room.
///
/// Owned by the physics/rendering pipeline; `sandbox_update` advances each
/// platform per frame and carries the player by its delta. The physics plugin
/// registers this as a resource; the room-load path (setup, load_room,
/// LDtk hot-reload, sandbox reset) replaces the Vec when the active room
/// changes.
#[derive(Resource, Default)]
pub struct MovingPlatformSet(pub Vec<platforms::MovingPlatformState>);

/// Pure simulation scalars extracted from `SandboxRuntime`.
/// Holds only values that belong to the running simulation,
/// not to developer/debug tools or the player shadow cache.
#[derive(Resource, Clone, Copy, Debug)]
pub struct SandboxSimState {
    pub last_safe_player_pos: ae::Vec2,
    pub time_scale: f32,
    pub room_transition_cooldown: f32,
}

impl Default for SandboxSimState {
    fn default() -> Self {
        Self {
            last_safe_player_pos: ae::Vec2::ZERO,
            time_scale: 1.0,
            room_transition_cooldown: 0.0,
        }
    }
}

#[derive(Resource)]
pub struct SandboxRuntime {
    pub player: ae::Player,
    pub debug: bool,
    pub slowmo: bool,
    pub presets: Vec<KeyboardPreset>,
    pub preset_index: usize,
    pub preset_flash: f32,
    // Active player attack state has moved to the `CurrentPlayerAttack` resource.
    // Moving platform state has moved to the `MovingPlatformSet` standalone Resource.
    // Physics settings have moved to the `physics::PhysicsSandboxSettings` standalone Resource.
    // Dialogue state lives on the `dialog::DialogState` standalone Resource.
    // Blink/camera presentation state has moved to `PlayerBlinkCameraState` ECS component.
    // Simulation scalars (last_safe_player_pos, time_scale, room_transition_cooldown)
    // have moved to the `SandboxSimState` standalone Resource.
}

/// Sandbox-side state for one active player melee swing.
#[derive(Clone, Debug)]
pub struct PlayerAttackState {
    pub spec: ae::AttackSpec,
    pub elapsed: f32,
    pub hit_targets: Vec<String>,
    pub active_started: bool,
    /// True once a downward/pogo active-frame attack has produced its bounce.
    /// Prevents one long active window from repeatedly bouncing every frame.
    pub pogo_applied: bool,
}

impl PlayerAttackState {
    pub fn new(spec: ae::AttackSpec) -> Self {
        Self {
            spec,
            elapsed: 0.0,
            hit_targets: Vec::new(),
            active_started: false,
            pogo_applied: false,
        }
    }

    pub fn phase(&self) -> Option<ae::AttackPhase> {
        self.spec.phase_at(self.elapsed)
    }

    pub fn done(&self) -> bool {
        self.phase().is_none()
    }

    pub fn progress(&self) -> f32 {
        (self.elapsed / self.spec.total_seconds().max(0.001)).clamp(0.0, 1.0)
    }
}

impl SandboxRuntime {
    pub fn new(
        world: &ae::World,
        abilities: ae::AbilitySet,
        tuning: ae::MovementTuning,
    ) -> Self {
        let mut player = ae::Player::new_with_abilities(world.spawn, abilities);
        player.refresh_movement_resources(tuning);
        Self {
            player,
            debug: !cfg!(target_os = "android"),
            slowmo: false,
            presets: KeyboardPreset::presets().to_vec(),
            preset_index: 0,
            preset_flash: 1.2,
        }
    }

    pub fn reset(&mut self, world: &ae::World, tuning: ae::MovementTuning) {
        self.player.reset_to(world.spawn);
        self.player.refresh_movement_resources(tuning);
        // Combat timers live on `PlayerCombatState`.
        // Interaction timers live on `PlayerInteractionState`.
        // Blink/camera presentation state lives on `PlayerBlinkCameraState`.
        // Health lives on `PlayerHealth`; callers reset it directly on the component.
        // Simulation scalars (last_safe_player_pos, time_scale, room_transition_cooldown)
        // live on `SandboxSimState`; callers reset those separately.
        // Active attack state lives on `CurrentPlayerAttack` resource;
        // sandbox_update callers reset it separately.
        // Refill mana on reset; the editor-tuned damage_multiplier /
        // invincible flag now lives on `Player` and survives reset
        // because `reset_to` only touches movement state, not these
        // gameplay tunables — so testers don't lose their F3
        // settings on every respawn.
        self.player.mana.refill_full();
        // Animation signal timers live on the `PlayerAnimState` ECS component;
        // callers that trigger a full reset (reset_sandbox, death_respawn_player)
        // call `PlayerAnimState::reset()` on the component directly.
    }

    /// Record the current player position as "the last known safe spot"
    /// when (and only when) every predicate of safety holds. Call sites
    /// pass the same augmented collision world the engine simulated
    /// against this frame so the gate matches reality.
    ///
    /// The flags allow the caller to suppress this write during damage
    /// resolution, hazard respawn, hitstun, post-blink grace, or room
    /// transitions where the player position is intentionally being
    /// teleported and shouldn't be remembered as safe. See
    /// `docs/lessons_learned.md` for the OOB trace where a wall-cling
    /// teleport polluted `last_safe_player_pos` with `(62, -23)`.
    pub fn remember_safe_player_position(
        &self,
        sim_state: &mut SandboxSimState,
        player: &ae::Player,
        world: &ae::World,
        ctx: SafePositionContext,
    ) {
        if !player.on_ground {
            return;
        }
        if !ctx.is_eligible() {
            return;
        }
        let verdict = ae::classify_player_safety(player, world, 0.0, |block| {
            matches!(
                block.kind,
                ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
            )
        });
        if verdict.is_safe() {
            sim_state.last_safe_player_pos = player.pos;
        }
    }

    pub fn update_time_scale(&self, sim_state: &mut SandboxSimState, player: &ae::Player, hitstop_timer: f32, frame_dt: f32, feel: SandboxFeelTuning) {
        let target = if hitstop_timer > 0.0 {
            0.0
        } else if player.blink_aiming {
            feel.bullet_time_scale
        } else if player.blink_hold_active {
            feel.blink_hold_slow_scale
        } else if self.slowmo {
            feel.debug_slowmo_scale
        } else {
            1.0
        };
        let rate = if target < sim_state.time_scale {
            feel.time_ramp_down_rate
        } else {
            feel.time_ramp_up_rate
        };
        sim_state.time_scale = move_toward(sim_state.time_scale, target, rate * frame_dt);
    }

    pub fn preset(&self) -> KeyboardPreset {
        self.presets[self.preset_index]
    }

    pub fn debug_enabled(&self) -> bool {
        self.debug
    }
}

/// Approach `target` from `value` by at most `delta`. Used for time-scale
/// ramping in `SandboxRuntime::update_time_scale`.
pub fn move_toward(value: f32, target: f32, delta: f32) -> f32 {
    if value < target {
        (value + delta).min(target)
    } else {
        (value - delta).max(target)
    }
}

/// Live camera scale + ease state. The camera reads target scale from
/// the encounter registry (or developer overview override) every
/// frame; this resource holds the smoothed value so transitions feel
/// like a breath instead of a snap.
#[derive(Resource, Clone, Copy, Debug)]
pub struct CameraEaseState {
    pub live_scale: f32,
    /// Smoothed world-space camera target. Presentation-only: avoids hard
    /// jumps when look-ahead flips with facing or when framing presets change.
    pub live_target_world: ae::Vec2,
    pub target_initialized: bool,
}

impl Default for CameraEaseState {
    fn default() -> Self {
        Self {
            live_scale: 1.0,
            live_target_world: ae::Vec2::ZERO,
            target_initialized: false,
        }
    }
}

/// Scale-units per second when easing camera *into* an encounter
/// (zoom-out). Faster than the recovery rate so the player feels the
/// arena widen quickly when the lock-wall slams.
pub const DEFAULT_CAMERA_ZOOM_OUT_RATE: f32 = 1.6;

/// Scale-units per second when easing camera *out of* an encounter
/// (zoom-in). Slower than zoom-out; the post-fight breathing room is
/// the moment to savor.
pub const DEFAULT_CAMERA_ZOOM_IN_RATE: f32 = 0.9;

/// Below this absolute delta the camera-ease snap completes — prevents
/// floating-point drift from accumulating into never-converges
/// territory at the tail of the ease.
pub const DEFAULT_CAMERA_ZOOM_SNAP_EPSILON: f32 = 0.0025;

/// Tunable knobs for the camera-ease behavior. Replaces the
/// hardcoded `CAMERA_ZOOM_{IN,OUT}_RATE` constants so the sandbox or
/// tests can override the rates without recompiling. The defaults
/// match the previous constants (`1.6` zoom-out, `0.9` zoom-in).
///
/// `target_scale > live_scale` (zooming out) uses `zoom_out_rate`;
/// the inverse direction uses `zoom_in_rate`. `snap_epsilon` is the
/// distance at which the ease finalizes onto the target value.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct CameraEaseTuning {
    /// Scale-units per second when easing into a wider view
    /// (encounter starts; lock-wall slam moment).
    pub zoom_out_rate: f32,
    /// Scale-units per second when easing back to the close view
    /// (post-encounter breathing room).
    pub zoom_in_rate: f32,
    /// Snap-to-target threshold to terminate the ease.
    pub snap_epsilon: f32,
}

impl Default for CameraEaseTuning {
    fn default() -> Self {
        Self {
            zoom_out_rate: DEFAULT_CAMERA_ZOOM_OUT_RATE,
            zoom_in_rate: DEFAULT_CAMERA_ZOOM_IN_RATE,
            snap_epsilon: DEFAULT_CAMERA_ZOOM_SNAP_EPSILON,
        }
    }
}

#[cfg(test)]
mod safe_pos_tests {
    use super::*;
    use ambition_engine::Block;

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

    fn runtime_with_player_at(world: &ae::World, pos: ae::Vec2) -> (SandboxRuntime, SandboxSimState) {
        let mut runtime = SandboxRuntime::new(
            world,
            ae::AbilitySet::sandbox_all(),
            ae::DEFAULT_TUNING,
        );
        runtime.player.pos = pos;
        runtime.player.on_ground = true;
        // Force a known starting "safe pos" we can detect changes from.
        let mut sim = SandboxSimState::default();
        sim.last_safe_player_pos = ae::Vec2::new(170.0, 1695.0);
        (runtime, sim)
    }

    /// The OOB y=-23 position (above the world envelope) must NOT be
    /// recorded as safe even though `on_ground` is true. This is the
    /// invariant the wall-cling teleport bug violated for two consecutive
    /// reproductions before the fix.
    #[test]
    fn rejects_position_above_world_envelope() {
        let world = dummy_world();
        let (runtime, mut sim) = runtime_with_player_at(&world, ae::Vec2::new(62.0, -23.0));
        let initial = sim.last_safe_player_pos;
        let player = runtime.player.clone();
        runtime.remember_safe_player_position(&mut sim, &player, &world, SafePositionContext::ideal());
        assert_eq!(
            sim.last_safe_player_pos, initial,
            "above-world position must not become last_safe_player_pos"
        );
    }

    /// The position update should fire when the player is grounded inside
    /// the world envelope and not overlapping a Solid.
    #[test]
    fn accepts_legitimate_grounded_position() {
        let world = dummy_world();
        let (runtime, mut sim) = runtime_with_player_at(&world, ae::Vec2::new(200.0, 900.0));
        let player = runtime.player.clone();
        runtime.remember_safe_player_position(&mut sim, &player, &world, SafePositionContext::ideal());
        assert_eq!(
            sim.last_safe_player_pos,
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
        let (runtime, mut sim) = runtime_with_player_at(&world, ae::Vec2::new(200.0, 900.0));
        let initial = sim.last_safe_player_pos;
        let player = runtime.player.clone();

        let mut ctx = SafePositionContext::ideal();
        ctx.damaged_this_frame = true;
        runtime.remember_safe_player_position(&mut sim, &player, &world, ctx);
        assert_eq!(sim.last_safe_player_pos, initial);

        let mut ctx = SafePositionContext::ideal();
        ctx.feature_requested_reset = true;
        runtime.remember_safe_player_position(&mut sim, &player, &world, ctx);
        assert_eq!(sim.last_safe_player_pos, initial);

        let mut ctx = SafePositionContext::ideal();
        ctx.in_hitstun = true;
        runtime.remember_safe_player_position(&mut sim, &player, &world, ctx);
        assert_eq!(sim.last_safe_player_pos, initial);

        let mut ctx = SafePositionContext::ideal();
        ctx.blink_grace_active = true;
        runtime.remember_safe_player_position(&mut sim, &player, &world, ctx);
        assert_eq!(sim.last_safe_player_pos, initial);

        let mut ctx = SafePositionContext::ideal();
        ctx.room_transitioning = true;
        runtime.remember_safe_player_position(&mut sim, &player, &world, ctx);
        assert_eq!(sim.last_safe_player_pos, initial);
    }

    /// A position INSIDE a Solid block is not safe even if `on_ground`
    /// is true. Mirror of `classify_player_safety`'s InsideSolid case.
    #[test]
    fn rejects_position_inside_solid() {
        let world = dummy_world();
        // The left wall's right edge is at x=36; place the player center
        // at x=18 with half-width 14, body covers x=4..32 — fully inside
        // the wall.
        let (runtime, mut sim) = runtime_with_player_at(&world, ae::Vec2::new(18.0, 900.0));
        let initial = sim.last_safe_player_pos;
        let player = runtime.player.clone();
        runtime.remember_safe_player_position(&mut sim, &player, &world, SafePositionContext::ideal());
        assert_eq!(sim.last_safe_player_pos, initial);
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
