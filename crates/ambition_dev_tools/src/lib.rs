//! Reusable developer-tooling state and simulation-side logic.
//!
//! Owns:
//!
//! - [`dev_tools`] — the [`DeveloperTools`](dev_tools::DeveloperTools) debug/
//!   gizmo toggle resource, the reflected editable player-tuning / ability /
//!   stats resources + their engine conversions, the movement/debug profile
//!   enums, and the inspector-visibility run conditions. Plus the live-edit
//!   sync systems that push inspector edits onto the authoritative player body
//!   (they name only the foundational `Body*` clusters + `PrimaryPlayerOnly`).
//! - [`profiling`] — the startup profiler marks (read by audio + setup).
//! - [`runtime_census`] — the profiling-only workload censuses (off unless
//!   `AMBITION_PROFILE_CENSUS` is set) and the clock every census samples on.
//! - [`persistence`] — `DeveloperTools` disk persistence (developer.ron).
//! - [`sync_live_player_dev_edits_system`] — the host-scheduled system that
//!   applies live ability/tuning edits to the player each frame.
//!
//! Presentation UI remains in `ambition_app`; gameplay tracing remains with the
//! simulation state it samples.

pub mod brain_override;
pub mod dev_tools;
/// The authored world file's mtime watch + reload status — the state half of
/// the Developer page's auto-apply row.
pub mod hot_reload;
pub mod persistence;
pub mod perception_census;
pub mod population_cap;
pub mod profiling;
/// The profiling-only workload censuses and the shared census clock they sample on.
pub mod runtime_census;
pub mod sim_plugin;

pub use hot_reload::{poll_world_source_changes, WorldSourceHotReload};
pub use persistence::DeveloperPersistenceSchedulePlugin;
pub use sim_plugin::{DevEditApplySet, DevInspectorMirrorSet, DevToolsSimPlugin};

use bevy::prelude::*;

use ambition_platformer2d_core::{
    AbilityBase, ActiveMovementTuning, AuthoredMovementTuning, BodyAbilities, BodyDashState,
    BodyFlightState, BodyJumpState, MotionModel,
};
use ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly;
use dev_tools::EditableAbilitySet;

/// Push live dev-tools ability/tuning edits onto the authoritative player.
///
/// Registered by the host to run even while gameplay is suspended so the F3
/// inspector stays responsive; the logic is body-state mutation and lives here
/// beside the dev STATE it reads.
///
/// The editable ability set is a session MASK, not a wholesale replacement:
/// the effective set is the body's intrinsic [`AbilityBase`] intersected with
/// the editable set. A mask can only ever gate a verb OFF, never conjure one the
/// character was not authored to have — so a restricted character (a demo
/// protagonist authored with a run-and-jump kit) keeps its identity instead of
/// being clobbered up to the inspector's `sandbox_all` default every frame. For
/// the sandbox protagonist (base `sandbox_all`) the intersection equals the
/// editable set, so the F3 experiment workflow is unchanged.
pub fn sync_live_player_dev_edits_system(
    // The neutral authority, NOT the inspector mirror: `apply_editable_movement_tuning`
    // is chained immediately before this in `DevEditApplySet`, so an F3 edit is
    // already here — and a body whose tuning came from content rather than the
    // inspector now resolves correctly too.
    active_tuning: Res<ActiveMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
    mut player_q: Query<
        (
            &mut BodyAbilities,
            &AbilityBase,
            &mut BodyFlightState,
            &mut MotionModel,
            &mut BodyDashState,
            &mut BodyJumpState,
            // Presence means the body's feel is authored (a demo protagonist),
            // so the resource-refresh below uses THAT tuning's air-jump count,
            // never the shared editable's — the same rule the live integrator
            // applies. Absent for the sandbox protagonist, which tracks F3.
            Option<&AuthoredMovementTuning>,
        ),
        PrimaryPlayerOnly,
    >,
) {
    let Ok((mut abilities, base, mut flight, mut model, mut dash, mut jump, authored_tuning)) =
        player_q.single_mut()
    else {
        return;
    };
    let desired_abilities = base.abilities.intersect(editable_abilities.as_engine());
    let effective_tuning = authored_tuning.map(|t| t.0).unwrap_or(active_tuning.0);
    // Reading through `Mut<T>` is change-neutral; coercing it to `&mut T` is
    // not. Keep the equality guard here, before the helper call, so an
    // unchanged inspector resource does not mark `BodyAbilities` changed every
    // frame and spuriously refresh every downstream derived persona system.
    if abilities.abilities == desired_abilities {
        return;
    }
    dev_tools::sync_live_ability_edits_clusters(
        &mut abilities,
        &mut flight,
        &mut model,
        &mut dash,
        &mut jump,
        desired_abilities,
        effective_tuning,
    );
}

/// Developer/debug state: debug flags and the HUD flash timer. Keyboard preset
/// selection is owned by persisted user settings, not duplicated here.
#[derive(Resource)]
pub struct DeveloperRuntimeState {
    pub debug: bool,
    pub slowmo: bool,
    /// How far slow-motion slows the sim clock when [`Self::slowmo`] is on.
    ///
    /// ⛔ THIS LIVED IN `Platformer2dFeelTuningMonolith` until 2026-08-31, whose
    /// own module doc says those values *"are gameplay parameters rather than
    /// developer-tool state"*. Nothing but the developer rung ever read it. A
    /// comment stating a rule is a rule to check.
    pub slowmo_scale: f32,
    pub preset_flash: f32,
}

/// Ask the clock to slow while developer slow-motion is on.
///
/// ⭐⭐ THE DEV CRATE ASKS; THE KERNEL NO LONGER LOOKS. This was rung 4 of the
/// actor kernel's five-rung time-scale ladder — twice, counting the
/// no-primary-player path — and it was the last thing making a SIMULATION
/// package read developer state. The seam to invert it was already built:
/// `ClockRequester::DevTool` exists, `RegimePolicy` already grants it in `Solo`
/// and denies it in `RLDeterministic` and `Cinematic`, and
/// `apply_clock_scale_requests` reduces by `min` so nothing depends on schedule
/// or query order.
///
/// ⚠ `min` IS NOT THE LADDER, and the difference is real: under the ladder
/// bullet-time's 0.5 outranked slow-motion's 0.25 because blink sat at rung 2;
/// now the STRONGEST SLOWDOWN wins and slow-motion does. That is the right
/// reading for a debugging override — a developer who asked the world to crawl
/// and got half speed because the player was aiming a blink has been told
/// "no" by a priority table they cannot see.
///
/// ⛔ EVERY FRAME, not on the toggle's edge. Every other rung writes a request
/// every frame and the reduction is per-frame, so a one-shot write would be
/// overwritten by the kernel's own `default` rung on the next tick.
pub fn request_developer_slow_motion(
    dev_state: bevy::prelude::Res<DeveloperRuntimeState>,
    mut writer: bevy::prelude::MessageWriter<ambition_time::time_control::ClockScaleRequest>,
) {
    if !dev_state.slowmo {
        return;
    }
    writer.write(ambition_time::time_control::ClockScaleRequest {
        domain: ambition_time::ClockDomain::SimClock,
        scale: dev_state.slowmo_scale,
        requester: ambition_time::time_control::ClockRequester::DevTool,
        reason: "dev_slowmo",
    });
}

/// Wind down the HUD's preset flash.
///
/// ⭐⭐ THE CRATE THAT OWNS THE TIMER OWNS ITS DECAY. This one line lived in the
/// actor kernel's `cleanup_timers_system`, and it was the ONLY reason that
/// system — and through it the simulation kernel — held a
/// `ResMut<DeveloperRuntimeState>` at all: a simulation package winding down a
/// developer HUD's flash. The value is written by a room commit and read by the
/// app's HUD; nothing in the sim reads it.
///
/// ⛔ STILL THE SIM SCHEDULE, NOT `Update`, and that is deliberate: its old home
/// ran in `PresentationSync` precisely so presentation timers decay while
/// gameplay is suspended, and moving it to `Update` would change WHICH CLOCK it
/// counts on under a rollback host. Ownership moved; the clock did not.
///
/// ⚠ UNORDERED within the tick, on purpose. It is a monotonic decay of a value
/// no sim system reads, so what it needs is to run once per tick — and the set
/// its old neighbour sits in (`Platformer2dSimulationPhaseMonolith`) is a name
/// this crate is forbidden to reach for.
pub fn decay_developer_presentation_flash(
    time: bevy::prelude::Res<bevy::prelude::Time>,
    mut dev_state: bevy::prelude::ResMut<DeveloperRuntimeState>,
) {
    dev_state.preset_flash = (dev_state.preset_flash - time.delta_secs()).max(0.0);
}

impl Default for DeveloperRuntimeState {
    fn default() -> Self {
        Self {
            debug: false,
            slowmo: false,
            // The value it carried in the feel table.
            slowmo_scale: 0.25,
            preset_flash: 1.2,
        }
    }
}

impl DeveloperRuntimeState {
    pub fn debug_enabled(&self) -> bool {
        self.debug
    }
}

/// Which layers of the combat overlay to draw.
///
/// ⭐⭐ THE LAYERS ARE INDEPENDENT BECAUSE THE QUESTIONS ARE. "Is this volume
/// inside the sprite?" needs the art; "where exactly does this reach?" is easier
/// with the art off; "why did this miss?" wants the hurtboxes without the
/// strikes on top of them. A single on/off switch answers one of the three.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatOverlayLayers {
    /// The rendered character art. Off draws the world grid instead, which is
    /// what `hide_sprites` has always meant.
    pub art: bool,
    /// The coarse collision envelope and the effective hurtboxes.
    pub hurtboxes: bool,
    /// Live strike volumes and the move/timing readout.
    pub strikes: bool,
}

impl Default for CombatOverlayLayers {
    /// Everything: the preset a tool asking for "the combat overlay" means.
    fn default() -> Self {
        Self {
            art: true,
            hurtboxes: true,
            strikes: true,
        }
    }
}

/// Turn the COMBAT overlay on, everywhere it is gated.
///
/// ⛔⛔ THE GIZMO PASS IS GATED ON THREE SEPARATE THINGS — the debug flag, the
/// gizmo toggle, and the per-view fields — and all three are off in a plain
/// build. Missing any one of them produces a photograph of a swing with no
/// volume on it, which reads as "the move has no hitbox" rather than "the
/// overlay is off". Every tool that wants combat geometry in a picture asks for
/// it here, so the count of gates lives in one place.
///
/// Idempotent: safe to call every frame, which is what a capture tool must do —
/// settings load and the developer-tools default both write this state, so a
/// startup-only write is a race against whichever of them runs last.
pub fn force_combat_overlay(
    state: &mut DeveloperRuntimeState,
    tools: &mut dev_tools::DeveloperTools,
    layers: CombatOverlayLayers,
) {
    if !state.debug {
        state.debug = true;
    }
    if !tools.gizmos_enabled {
        tools.gizmos_enabled = true;
    }
    if tools.debug_view_mode != dev_tools::DebugViewMode::Combat {
        tools.apply_debug_view_mode(dev_tools::DebugViewMode::Combat, false);
    }
    // ⛔ THE PRESET TURNS ON THE *COMBINED* GATE, and that one draws both halves
    // whatever the per-layer fields say — so asking for hurtboxes alone requires
    // clearing it. `draw_combat_geometry_view` reads
    // `show_player_hitbox || show_feature_hitboxes` for the hurt half and
    // `show_combat_preview || show_feature_hitboxes` for the strikes.
    tools.show_feature_hitboxes = false;
    tools.show_player_hitbox = layers.hurtboxes;
    tools.show_combat_preview = layers.strikes;
    tools.hide_sprites = !layers.art;
}

#[cfg(test)]
mod developer_runtime_state_tests {
    use super::*;

    #[test]
    fn debug_overlay_defaults_off_for_every_game() {
        assert!(!DeveloperRuntimeState::default().debug);
    }

    /// The HUD flash winds down, and it does so through a system THIS CRATE
    /// registers.
    ///
    /// ⛔⛔ TWO CLAIMS, AND THE REGISTRATION IS THE ONE THAT CAN GO WRONG. The
    /// decay is one subtraction and would pass as a direct call whether or not
    /// anything ran it; the line MOVED out of the actor kernel's
    /// `cleanup_timers_system`, and a timer nobody decays is a HUD flash that
    /// never clears — visible, and caught by nothing else in the tree.
    ///
    /// ⚠ The plugin's other systems need resources from crates this one does not
    /// depend on, so the registration is read off the SCHEDULE GRAPH rather than
    /// by running it. That is weaker than a behavioural check and it is the
    /// strongest thing available from inside this crate.
    #[test]
    fn the_developer_flash_decays_through_a_system_this_crate_registers() {
        use bevy::prelude::*;

        // 1. THE DECAY, and its floor. A HUD comparing `> 0.0` depends on the
        //    clamp, so a decay that ran negative would read as "still flashing".
        let mut world = World::new();
        world.insert_resource(DeveloperRuntimeState {
            preset_flash: 0.05,
            ..Default::default()
        });
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(20));
        world.insert_resource(time);
        let mut run =
            bevy::ecs::system::IntoSystem::into_system(decay_developer_presentation_flash);
        run.initialize(&mut world);
        run.run((), &mut world).expect("the decay system runs");
        let once = world.resource::<DeveloperRuntimeState>().preset_flash;
        assert!(once < 0.05, "the flash did not wind down at all: {once}");
        run.run((), &mut world).expect("the decay system runs");
        run.run((), &mut world).expect("the decay system runs");
        assert_eq!(
            world.resource::<DeveloperRuntimeState>().preset_flash,
            0.0,
            "the flash ran past zero, so a HUD asking `> 0.0` never stops drawing it"
        );

        // 2. AND SOMETHING RUNS IT — asserted in the SHIPPED APP rather than
        //    here. `DevToolsSimPlugin`'s other systems need resources from
        //    crates this one does not depend on, so a bare `App::new()` cannot
        //    run its schedule, and the schedule graph reports every system as
        //    `<Enable the debug feature to see the name>` without a bevy feature
        //    this crate will not turn on for a test. The registration guard is
        //    `the_developer_hud_flash_still_winds_down` in
        //    `game/ambition_app/tests/`, which boots a real app.
    }
}
