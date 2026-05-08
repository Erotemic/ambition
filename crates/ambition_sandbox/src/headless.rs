//! Headless simulation entry point.
//!
//! Slice 5 of ADR 0012's events refactor: `run_headless` now drives the
//! actual gameplay loop (`sandbox_update` and friends) by calling the
//! shared `crate::app::add_simulation_plugins`. The visible binary's
//! `crate::app::run_visible` calls the same helper plus
//! `add_presentation_plugins`. Headless skips the presentation half so
//! audio, VFX, debris, HUD, and inspector plugins are absent — the
//! sim emits messages into the queue and the queue drains harmlessly.
//!
//! Phase 1 (the original `run_headless` shape) only ticked the LDtk
//! runtime-spine systems; this Phase 2 version runs the full gameplay
//! loop including movement, collision, and the typed-event channels.
//! `LdtkPlugin` is now installed by `add_simulation_plugins`; if its
//! tile-rendering pipeline ever requires the render plugins we'll
//! revisit by gating it or by promoting more LDtk entity categories
//! to direct Ambition ECS spawns (per the LDtk runtime-spine roadmap).

use std::fmt;

use bevy::asset::AssetPlugin;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimePlugin;
use bevy::transform::TransformPlugin;

use crate::app::{add_simulation_plugins, init_sandbox_resources};
use crate::game_mode::GameMode;
use crate::ldtk_world;
use crate::rooms::RoomSet;

/// Summary of a `run_headless` call. Used by tests, the headless binary, and
/// future RL drivers to verify the simulation actually progressed instead of
/// silently no-op.
#[derive(Debug, Clone)]
pub struct HeadlessReport {
    pub ticks_run: u32,
    pub active_room: String,
    pub room_count: usize,
    pub spawned_entities: usize,
    pub spine_revision: u64,
    pub solid_index_revision: u64,
}

impl fmt::Display for HeadlessReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "headless run completed: {} ticks", self.ticks_run)?;
        writeln!(f, "  active room    : {}", self.active_room)?;
        writeln!(f, "  rooms loaded   : {}", self.room_count)?;
        writeln!(
            f,
            "  ldtk entities  : {} (spawned by bevy_ecs_ldtk)",
            self.spawned_entities
        )?;
        writeln!(f, "  spine revision : {}", self.spine_revision)?;
        write!(f, "  solid revision : {}", self.solid_index_revision)?;
        Ok(())
    }
}

/// Run the sandbox simulation headless for `max_ticks` Bevy `Update` cycles.
///
/// Builds an `App` from `MinimalPlugins` plus the small set of Bevy
/// foundation plugins the sim's resources / assets / states / transforms
/// need, then composes `init_sandbox_resources` and
/// `add_simulation_plugins` from `crate::app`. Calls `app.update()`
/// `max_ticks` times and returns a `HeadlessReport`.
///
/// Validation failures from the embedded LDtk project propagate as `Err`,
/// matching the production policy that an invalid LDtk file is a hard
/// error rather than a `.expect()` panic.
pub fn run_headless(max_ticks: u32) -> Result<HeadlessReport, String> {
    // Validate the embedded LDtk file up front so we can return Err with a
    // useful diagnostic. `init_sandbox_resources` does this too but exits
    // the process on failure; tests want a structured error instead.
    let project = ldtk_world::LdtkProject::load_default()?;
    let report = project.validate();
    if !report.is_ok() {
        report.print_to_stderr();
        return Err(format!(
            "embedded LDtk validation failed: {} error(s)",
            report.errors.len()
        ));
    }
    if let Err(errors) = project.to_room_set() {
        return Err(errors.join("; "));
    }
    let room_count = project
        .to_room_set()
        .expect("just validated above")
        .rooms
        .len();

    let mut app = App::new();
    // Minimal Bevy foundation: time/transform/state/asset/image registries.
    // ImagePlugin is included because bevy_ecs_ldtk's tile spawning touches
    // Image asset handles even when no rendering happens; without it the
    // asset type is unregistered and LdtkPlugin panics during setup.
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<GameMode>();
    let _ = TimePlugin; // re-export reference; MinimalPlugins already adds it.

    init_sandbox_resources(&mut app);
    add_simulation_plugins(&mut app);

    for _ in 0..max_ticks {
        app.update();
    }

    let world = app.world();
    let stats = world
        .resource::<ldtk_world::LdtkRuntimeSpineStats>()
        .clone();
    let spine_index = world.resource::<ldtk_world::LdtkRuntimeSpineIndex>();
    let solid_index = world.resource::<ldtk_world::LdtkRuntimeSolidIndex>();
    let active_room_after = world.resource::<RoomSet>().active_spec().id.clone();

    Ok(HeadlessReport {
        ticks_run: max_ticks,
        active_room: active_room_after,
        room_count,
        spawned_entities: stats.spawned_entities,
        spine_revision: spine_index.revision,
        solid_index_revision: solid_index.revision,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::SfxMessage;
    use crate::input::ControlFrame;
    use bevy::ecs::message::Messages;

    #[test]
    fn run_headless_completes_one_tick_without_panicking() {
        let report = run_headless(1).expect("headless one-tick run succeeds");
        assert_eq!(report.ticks_run, 1);
        assert!(
            report.room_count > 0,
            "embedded LDtk should produce at least one room"
        );
        assert!(!report.active_room.is_empty());
    }

    #[test]
    fn run_headless_runs_multiple_ticks() {
        let report = run_headless(8).expect("headless eight-tick run succeeds");
        assert_eq!(report.ticks_run, 8);
    }

    /// ADR 0012 step B stop gate: with `MinimalPlugins` only and no
    /// AudioPlugin / RenderPlugin / inspector, can we drive
    /// `sandbox_update` end-to-end and observe `SfxMessage` flow? This
    /// proves the sim/presentation seam holds for the input + sfx
    /// channels. Reset is the cheapest path: pressing Reset emits
    /// `SfxMessage::Reset` synchronously, no spawn-position dependence.
    #[test]
    fn sim_emits_sfx_reset_when_control_frame_requests_reset() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.add_plugins(ImagePlugin::default());
        app.add_plugins(TransformPlugin);
        app.add_plugins(StatesPlugin);
        app.init_state::<GameMode>();

        crate::app::init_sandbox_resources(&mut app);
        crate::app::add_simulation_plugins(&mut app);

        // First tick runs Startup (spawns the player + SandboxRuntime).
        app.update();

        // Inject a "press reset" frame on the sim/presentation input seam.
        *app.world_mut().resource_mut::<ControlFrame>() = ControlFrame {
            reset_pressed: true,
            ..ControlFrame::default()
        };

        app.update();

        let messages = app.world().resource::<Messages<SfxMessage>>();
        let reset_count = messages
            .iter_current_update_messages()
            .filter(|m| matches!(m, SfxMessage::Reset { .. }))
            .count();
        assert!(
            reset_count >= 1,
            "expected at least one SfxMessage::Reset emitted by the sim; got {reset_count}",
        );
    }
}
