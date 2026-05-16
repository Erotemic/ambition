//! Ambition sandbox library.
//!
//! The sandbox crate exposes both the playable Bevy app (`src/main.rs`) and a
//! headless simulation entry point (`run_headless`, used by `bin/headless.rs`
//! and tests/CI on machines without a display). Both binaries depend on this
//! library; the library owns the module graph and the cross-cutting types
//! (`GameWorld`, `SandboxSimState`, `SandboxDevState`) that submodules reference
//! via `crate::*`.
//!
//! Player state is authoritative on the `PlayerMovementAuthority` ECS component.
//! Do not introduce a god-object runtime resource; add narrow resources or ECS
//! components instead.
//!
//! See `docs/headless_simulation.md` for the sim/presentation contract this
//! library is being shaped toward, and `docs/architecture_targets.md` for the
//! longer-term events refactor that will let `sandbox_update` itself run
//! headless.

pub mod audio;
pub mod banter;
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
pub mod intro;
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
pub mod time_control;
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

pub use game_mode::{gameplay_allowed, GameMode};
pub use headless::{run_headless, HeadlessReport};
#[cfg(feature = "rl_sim")]
pub use rl_sim::{AgentAction, AgentObservation, SandboxSim, SandboxSimOptions};

use ambition_engine as ae;
use bevy::prelude::{Message, Resource};

use feel::SandboxFeelTuning;
use input::KeyboardPreset;
use player::components::PlayerSlot;

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

pub const BLINK_IN_ANIM_TIME: f32 = 0.34;
pub const ROOM_DOOR_CAMERA_SNAP_TIME: f32 = 0.08;

/// Active player melee swing. `None` when no swing is in progress.
///
/// Authoritative source: set/cleared by `start_attack` / `advance_attack`
/// inside `sandbox_update`. `write_player_ecs_components` mirrors
/// `is_some()` into `PlayerCombatState::attacking` each frame.
///
/// **Multiplayer caveat (primary-player-only):** this is a global
/// `Resource` and therefore implicitly the primary player's attack.
/// Future co-op / split-screen will need per-player attack state —
/// likely a `PlayerAttackState` component on the player entity rather
/// than a `Resource`. Until then, treat any new reads/writes as
/// "primary player only" and avoid adding additional one-player
/// scalars here.
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

/// ADR 0010 vocabulary — the named clocks gameplay code can read.
///
/// `SimClock` ticks at the gameplay rate; bullet-time / hitstop /
/// pause scale this. `PlayerClock(slot)` is a per-player cognitive
/// rate (ADR 0011) and is what multiplayer-coherent time abilities
/// rebind. `WallClock` is the host's real time, never scaled —
/// used by UI fades, hot-reload polling, audio.
///
/// In single-player today every PlayerClock equals SimClock, so
/// the operationally-equivalent SP path "slow sim" and MP-correct
/// path "boost player proper time" are observationally identical.
/// See ADR 0011 §"Two time-control operations".
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ClockDomain {
    SimClock,
    PlayerClock(PlayerSlot),
    WallClock,
}

/// Per-frame dt snapshots keyed by [`ClockDomain`].
///
/// Use the typed accessors instead of `Res<Time>::delta_secs()`:
///
/// - [`WorldTime::sim_dt`] — gameplay state machines + world-anchored
///   animation. Scales with bullet-time / hitstop / pause.
/// - [`WorldTime::player_dt`] — per-player cognitive rate. In SP this
///   equals `sim_dt`; in future MP it's the seam where one player's
///   bullet-time doesn't slow the other's world.
/// - [`WorldTime::wall_dt`] — real time. UI fades, hot-reload polling,
///   debug overlays — anything that must NOT freeze with the world.
///
/// Default `sim_dt` for new code; reach for the others only when you
/// can articulate why ([feedback-time-domains]).
///
/// The legacy fields [`WorldTime::raw_dt`] / [`WorldTime::scaled_dt`]
/// remain as aliases (`raw_dt == wall_dt`, `scaled_dt == sim_dt`) so
/// existing callers keep compiling. Migrate to the accessors at
/// touch time; the fields are slated for removal in a follow-up.
///
/// Refreshed once per Update via [`refresh_world_time`] before
/// every system that reads it; the resource is always one frame
/// fresh by construction.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct WorldTime {
    /// Wall-clock dt from Bevy's `Time` resource. Unscaled — for
    /// UI / debug only. Legacy alias for [`WorldTime::wall_dt`].
    pub raw_dt: f32,
    /// `raw_dt * SandboxSimState::time_scale`. The canonical
    /// dt for gameplay + world-anchored animation timers. Zero
    /// while paused (`time_scale == 0`). Legacy alias for
    /// [`WorldTime::sim_dt`].
    pub scaled_dt: f32,
}

impl WorldTime {
    /// Dt for the gameplay sim clock — bullet-time / hitstop / pause
    /// scale this. Canonical choice for world-anchored timers,
    /// animation, AI ticks, and any gameplay state machine.
    #[inline]
    pub fn sim_dt(&self) -> f32 {
        self.scaled_dt
    }

    /// Dt for the host's wall clock — never scaled. Use for UI
    /// fades, hot-reload polling, debug overlays, audio buses;
    /// anything that must keep ticking when the world freezes.
    #[inline]
    pub fn wall_dt(&self) -> f32 {
        self.raw_dt
    }

    /// Dt for player `slot`'s cognitive clock (ADR 0011). In the
    /// single-player Solo regime every `PlayerClock` equals
    /// `SimClock`, so this is observationally identical to
    /// [`Self::sim_dt`] today. The accessor is the seam where a
    /// future CoopConsensual / Competitive regime can give each
    /// player a distinct rate without the call sites changing.
    #[inline]
    pub fn player_dt(&self, _slot: PlayerSlot) -> f32 {
        // SP regime: every PlayerClock == SimClock.
        // ADR 0010 §Regimes — Solo is permissive; multi-observer
        // regimes (future) will diverge here.
        self.sim_dt()
    }

    /// Dt for an arbitrary [`ClockDomain`]. Prefer the typed
    /// accessors above for known domains; this exists for systems
    /// that take a domain as data (the regime-policy dispatch).
    #[inline]
    pub fn dt_for(&self, domain: ClockDomain) -> f32 {
        match domain {
            ClockDomain::SimClock => self.sim_dt(),
            ClockDomain::PlayerClock(slot) => self.player_dt(slot),
            ClockDomain::WallClock => self.wall_dt(),
        }
    }

    /// Per-entity proper-time dt (ADR 0011). Multiplies [`Self::sim_dt`]
    /// by the entity's [`crate::time_control::ProperTimeScale`] —
    /// `1.0` by default, so callers that pass
    /// [`crate::time_control::ProperTimeScale::ONE`] (the missing-
    /// component case) get the same `sim_dt` value as before.
    ///
    /// Pattern: animator + AI systems query `Option<&ProperTimeScale>`
    /// alongside the entity's other components and feed the result
    /// through [`crate::time_control::ProperTimeScale::or_default`]
    /// before calling `entity_dt`. SP gameplay is unchanged because no
    /// entity sets the component today.
    #[inline]
    pub fn entity_dt(&self, scale: crate::time_control::ProperTimeScale) -> f32 {
        self.sim_dt() * scale.value()
    }
}

/// Refresh [`WorldTime`] from `Time × SandboxSimState::time_scale`.
/// Registered early in the Update schedule so every downstream
/// system sees a current value.
pub fn refresh_world_time(
    time: bevy::prelude::Res<bevy::prelude::Time>,
    sim_state: bevy::prelude::Res<SandboxSimState>,
    mut world_time: bevy::prelude::ResMut<WorldTime>,
) {
    let raw = time.delta_secs();
    world_time.raw_dt = raw;
    world_time.scaled_dt = raw * sim_state.time_scale;
}

/// Pure simulation scalars for the running sandbox session.
/// Holds values that belong to the simulation, not to
/// developer/debug tools or presentation state.
///
/// **Multiplayer caveat:** each field has different per-player vs.
/// shared semantics for a future co-op build:
/// - `last_safe_player_pos` — currently **primary-player-only**;
///   needs to become a per-`PlayerSlot` map.
/// - `time_scale` — **global shared-world** (hitstop / bullet-time /
///   pause should affect everyone). Stays on the resource.
/// - `room_transition_cooldown` — **global shared-world** today
///   because the whole party shares one active room. If a future
///   build splits rooms per-player this would need to move per-room
///   or per-player.
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

/// Record the current player position as "the last known safe spot"
/// when (and only when) every predicate of safety holds. Call sites pass
/// the same augmented collision world the engine simulated against this
/// frame so the gate matches reality.
///
/// The flags allow the caller to suppress this write during damage
/// resolution, hazard respawn, hitstun, post-blink grace, or room
/// transitions where the player position is intentionally being
/// teleported and shouldn't be remembered as safe. See
/// `docs/lessons_learned.md` for the OOB trace where a wall-cling
/// teleport polluted `last_safe_player_pos` with `(62, -23)`.
pub fn remember_safe_player_position(
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

/// Drive the `time_scale` ramp: hitstop → bullet-time → slowmo → normal.
pub fn update_time_scale(
    slowmo: bool,
    sim_state: &mut SandboxSimState,
    player: &ae::Player,
    hitstop_timer: f32,
    frame_dt: f32,
    feel: SandboxFeelTuning,
) {
    let target = if hitstop_timer > 0.0 {
        0.0
    } else if player.blink_aiming {
        feel.bullet_time_scale
    } else if player.blink_hold_active {
        feel.blink_hold_slow_scale
    } else if slowmo {
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

/// Approach `target` from `value` by at most `delta`. Used for time-scale
/// ramping in `update_time_scale`.
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

    fn player_with_sim_at(world: &ae::World, pos: ae::Vec2) -> (ae::Player, SandboxSimState) {
        let mut player = ae::Player::new_with_abilities(world.spawn, ae::AbilitySet::sandbox_all());
        player.refresh_movement_resources(ae::DEFAULT_TUNING);
        player.pos = pos;
        player.on_ground = true;
        // Force a known starting "safe pos" we can detect changes from.
        let mut sim = SandboxSimState::default();
        sim.last_safe_player_pos = ae::Vec2::new(170.0, 1695.0);
        (player, sim)
    }

    /// The OOB y=-23 position (above the world envelope) must NOT be
    /// recorded as safe even though `on_ground` is true. This is the
    /// invariant the wall-cling teleport bug violated for two consecutive
    /// reproductions before the fix.
    #[test]
    fn rejects_position_above_world_envelope() {
        let world = dummy_world();
        let (player, mut sim) = player_with_sim_at(&world, ae::Vec2::new(62.0, -23.0));
        let initial = sim.last_safe_player_pos;
        remember_safe_player_position(&mut sim, &player, &world, SafePositionContext::ideal());
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
        let (player, mut sim) = player_with_sim_at(&world, ae::Vec2::new(200.0, 900.0));
        remember_safe_player_position(&mut sim, &player, &world, SafePositionContext::ideal());
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
        let (player, mut sim) = player_with_sim_at(&world, ae::Vec2::new(200.0, 900.0));
        let initial = sim.last_safe_player_pos;

        let mut ctx = SafePositionContext::ideal();
        ctx.damaged_this_frame = true;
        remember_safe_player_position(&mut sim, &player, &world, ctx);
        assert_eq!(sim.last_safe_player_pos, initial);

        let mut ctx = SafePositionContext::ideal();
        ctx.feature_requested_reset = true;
        remember_safe_player_position(&mut sim, &player, &world, ctx);
        assert_eq!(sim.last_safe_player_pos, initial);

        let mut ctx = SafePositionContext::ideal();
        ctx.in_hitstun = true;
        remember_safe_player_position(&mut sim, &player, &world, ctx);
        assert_eq!(sim.last_safe_player_pos, initial);

        let mut ctx = SafePositionContext::ideal();
        ctx.blink_grace_active = true;
        remember_safe_player_position(&mut sim, &player, &world, ctx);
        assert_eq!(sim.last_safe_player_pos, initial);

        let mut ctx = SafePositionContext::ideal();
        ctx.room_transitioning = true;
        remember_safe_player_position(&mut sim, &player, &world, ctx);
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
        let (player, mut sim) = player_with_sim_at(&world, ae::Vec2::new(18.0, 900.0));
        let initial = sim.last_safe_player_pos;
        remember_safe_player_position(&mut sim, &player, &world, SafePositionContext::ideal());
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

#[cfg(test)]
mod world_time_clock_tests {
    use super::*;
    use crate::player::components::PlayerSlot;

    /// Sim regime: SP grants every PlayerClock the SimClock rate, so
    /// player_dt(slot) is observationally identical to sim_dt(). This
    /// is the seam where future MP / RL regimes diverge — until they
    /// do, the SP path stays one-line.
    #[test]
    fn sp_player_clock_equals_sim_clock() {
        let wt = WorldTime { raw_dt: 1.0 / 60.0, scaled_dt: 1.0 / 240.0 };
        assert_eq!(wt.sim_dt(), 1.0 / 240.0);
        assert_eq!(wt.player_dt(PlayerSlot::PRIMARY), wt.sim_dt());
        assert_eq!(wt.player_dt(PlayerSlot(7)), wt.sim_dt());
    }

    /// Wall clock is never scaled. UI fades / hot-reload polling must
    /// keep ticking when the world freezes.
    #[test]
    fn wall_dt_ignores_sim_scale() {
        let wt = WorldTime { raw_dt: 1.0 / 60.0, scaled_dt: 0.0 };
        assert_eq!(wt.wall_dt(), 1.0 / 60.0);
        assert_eq!(wt.sim_dt(), 0.0);
    }

    /// `dt_for(ClockDomain)` is the data-driven dispatch used by the
    /// regime policy. Each domain routes to its typed accessor.
    #[test]
    fn dt_for_dispatches_by_domain() {
        let wt = WorldTime { raw_dt: 1.0 / 60.0, scaled_dt: 1.0 / 480.0 };
        assert_eq!(wt.dt_for(ClockDomain::SimClock), wt.sim_dt());
        assert_eq!(wt.dt_for(ClockDomain::WallClock), wt.wall_dt());
        assert_eq!(
            wt.dt_for(ClockDomain::PlayerClock(PlayerSlot::PRIMARY)),
            wt.player_dt(PlayerSlot::PRIMARY),
        );
    }

    /// Legacy fields remain as aliases — `raw_dt == wall_dt` and
    /// `scaled_dt == sim_dt`. Existing call sites keep compiling.
    #[test]
    fn legacy_fields_alias_new_accessors() {
        let wt = WorldTime { raw_dt: 0.016, scaled_dt: 0.004 };
        assert_eq!(wt.raw_dt, wt.wall_dt());
        assert_eq!(wt.scaled_dt, wt.sim_dt());
    }

    /// ADR 0011 — per-entity proper time. The default scale 1.0
    /// collapses entity_dt to sim_dt; non-1.0 scales independently
    /// stretch or shrink the entity's tick. SP today doesn't set
    /// the component, so every entity tickts at sim_dt — Galilean
    /// behavior unchanged.
    #[test]
    fn entity_dt_default_one_equals_sim_dt() {
        let wt = WorldTime { raw_dt: 0.016, scaled_dt: 0.008 };
        assert_eq!(wt.entity_dt(crate::time_control::ProperTimeScale::ONE), wt.sim_dt());
    }

    #[test]
    fn entity_dt_scales_sim_dt_by_proper_time() {
        let wt = WorldTime { raw_dt: 0.016, scaled_dt: 0.008 };
        assert!((wt.entity_dt(crate::time_control::ProperTimeScale(2.0)) - 0.016).abs() < 1e-7);
        assert!((wt.entity_dt(crate::time_control::ProperTimeScale(0.5)) - 0.004).abs() < 1e-7);
    }
}
