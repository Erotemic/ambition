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
//! See `docs/systems/headless-simulation.md` for the sim/presentation contract this
//! library is being shaped toward, and `docs/adr/0012-sim-presentation-split-and-events-refactor.md` for the
//! longer-term events refactor that will let `sandbox_update` itself run
//! headless.

// External API surface — bins, tests, and Android/wasm entry points reach
// into these modules. Everything else stays `pub(crate)` so the compiler
// can tell us what's actually depended on from outside.
pub mod app;
pub mod audio;
pub mod headless;
pub mod input;
pub mod player;
#[cfg(feature = "rl_sim")]
pub mod rl_sim;

// Themed umbrellas. Each owns a coherent slice of the sandbox; the long-
// term shape carves the reusable ones (`presentation`, `time`, `dev`,
// `world`, `persistence`) into the future `ambition` framework crate, and
// leaves the sandbox-specific ones (`content`, sandbox `assets`) behind.
pub(crate) mod assets;
pub(crate) mod body_mode;
pub(crate) mod boss_encounter;
pub(crate) mod combat_slots;
pub(crate) mod config;
pub(crate) mod content;
pub(crate) mod dev;
pub(crate) mod dialog;
pub(crate) mod encounter;
pub(crate) mod enemy_projectile;
pub(crate) mod host;
pub(crate) mod intro;
pub(crate) mod inventory;
pub(crate) mod map_menu;
pub(crate) mod music;
pub(crate) mod pause_menu;
pub(crate) mod persistence;
pub(crate) mod presentation;
pub(crate) mod projectile;
pub(crate) mod runtime;
pub(crate) mod time;
pub(crate) mod ui_nav;
pub(crate) mod world;

// Public re-exports double as the external API: `features`, `rooms`,
// `ldtk_world`, `game_mode`, and `trace` are referenced from bins,
// tests, and the engine crate's doc comments. Internal `pub(crate)`
// shims for the other themed modules (assets, content, dev, host,
// persistence, player::bubble_shield, presentation, runtime, time,
// world::{physics, platforms}, and boss_encounter::sprites) were
// removed in the 2026-05-19 shim-cleanup pass; internal call sites
// now use the canonical `crate::<theme>::<module>::…` paths.
pub use content::features;
pub use dev::trace;
pub use runtime::game_mode;
pub use world::{ldtk_world, rooms};

// Crate-root types/consts whose definitions moved into themed modules but
// still need to surface at `crate::WorldTime` / `ambition_sandbox::WorldTime`.
pub use time::camera_ease::{
    CameraEaseState, CameraEaseTuning, DEFAULT_CAMERA_ZOOM_IN_RATE, DEFAULT_CAMERA_ZOOM_OUT_RATE,
    DEFAULT_CAMERA_ZOOM_SNAP_EPSILON,
};
pub use time::move_toward;
pub use time::world_time::{refresh_world_time, ClockDomain, WorldTime};

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

/// Browser (`wasm32-unknown-unknown`) entry point.
///
/// The wasm module exports this as its `start` function via
/// `wasm-bindgen`, so the JS in `crates/ambition_sandbox/web/index.html`
/// gets to call `init()` (the generated JS shim) and the browser fires
/// `web_start` on its own as soon as the module finishes instantiating —
/// no manual wiring on the page.
///
/// This is the analog of the Android `#[bevy_main]` shim above: the
/// platform supplies the entry-point convention, and we hand off to the
/// browser-flavored Bevy app builder. Desktop builds compile this away
/// entirely.
#[cfg(all(target_arch = "wasm32", feature = "web_platform"))]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn web_start() {
    // Forward panics to `console.error` instead of the default `abort`
    // so a first-pass crash is debuggable from devtools without a
    // separate wasm symbol pass. Cheap; `set_once` is idempotent.
    console_error_panic_hook::set_once();
    app::run_web();
}

pub use game_mode::{gameplay_allowed, GameMode};
pub use headless::{run_headless, HeadlessReport};
#[cfg(feature = "rl_sim")]
pub use rl_sim::{AgentAction, AgentObservation, SandboxSim, SandboxSimOptions};

// Re-export the types that leak through public crate-root signatures
// (`MovingPlatformSet.0`, `WorldTime::entity_dt`) so the modules they
// live in can stay `pub(crate)`. Without these, downstream callers
// couldn't name the types cleanly — `platforms::MovingPlatformState`
// wouldn't be reachable via any pub path — and Rust's
// private-interfaces lint would fire under `-D warnings`.
pub use time::time_control::ProperTimeScale;
pub use world::platforms::MovingPlatformState;

use ambition_engine as ae;
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

/// Live platform-simulation state for the current room.
///
/// Owned by the physics/rendering pipeline; `sandbox_update` advances each
/// platform per frame and carries the player by its delta. The physics plugin
/// registers this as a resource; the room-load path (setup, load_room,
/// LDtk hot-reload, sandbox reset) replaces the Vec when the active room
/// changes.
#[derive(Resource, Default)]
pub struct MovingPlatformSet(pub Vec<world::platforms::MovingPlatformState>);

// `WorldTime`, `ClockDomain`, `refresh_world_time` moved to
// `time::world_time`; `CameraEaseState` / `CameraEaseTuning` and the
// `DEFAULT_CAMERA_*` constants moved to `time::camera_ease`. Re-exported
// below so `crate::WorldTime` / `ambition_sandbox::WorldTime` keep working.

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
/// `dev/journals/lessons_learned.md` for the OOB trace where a wall-cling
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

// `update_time_scale` (deprecated since 0.2.0) and `move_toward` removed
// from the crate root. `move_toward` now lives in `time::move_toward` and
// is re-exported below; the `update_time_scale` ramp was superseded by
// `time_control::emit_player_time_intent_system` +
// `smooth_sim_clock_toward_target_system` (ADR 0010 step 4) and had no
// runtime callers left.

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

// `world_time_clock_tests` moved alongside the implementation in
// `time::world_time`.
