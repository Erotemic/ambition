//! Headless simulation entry point.
//!
//! `run_headless` drives the full gameplay loop by adding
//! `SandboxSimulationPlugin`. The visible binary uses `run_visible` which
//! additionally adds `SandboxLdtkPlugin` and `SandboxPresentationPlugin`.
//! Headless skips the presentation half so audio, VFX, debris, HUD, and
//! inspector plugins are absent — the sim emits messages into the queue
//! and the queue drains harmlessly.
//!
//! Phase 1 only ticked the LDtk runtime-spine systems; Phase 2 added the
//! full gameplay loop including movement, collision, and typed-event
//! channels.

use std::fmt;

use bevy::asset::AssetPlugin;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimePlugin;
use bevy::transform::TransformPlugin;

use crate::app::SandboxSimulationPlugin;
use ambition_gameplay_core::game_mode::GameMode;
use ambition_gameplay_core::ldtk_world;
use ambition_gameplay_core::rooms::RoomSet;

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
    /// One-line HUD summary per active or completed quest. Drawn from
    /// `QuestRegistry::quest_log_lines()` at the end of the run so the
    /// headless smoke can verify intro-v1 quest progression without
    /// spinning up a renderer.
    pub quest_log: Vec<String>,
    /// Room ids the player visited during the run, in stable order
    /// (`MapMenuState::visited`). Empty when the run starts and stays
    /// in the cold-launch room.
    pub visited_rooms: Vec<String>,
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
        writeln!(f, "  solid revision : {}", self.solid_index_revision)?;
        writeln!(
            f,
            "  rooms visited  : {}",
            if self.visited_rooms.is_empty() {
                "<none>".to_string()
            } else {
                self.visited_rooms.join(", ")
            }
        )?;
        if self.quest_log.is_empty() {
            write!(f, "  quest log      : <empty>")?;
        } else {
            writeln!(f, "  quest log      :")?;
            for (i, line) in self.quest_log.iter().enumerate() {
                if i + 1 == self.quest_log.len() {
                    write!(f, "    {line}")?;
                } else {
                    writeln!(f, "    {line}")?;
                }
            }
        }
        Ok(())
    }
}

/// Run the sandbox simulation headless for `max_ticks` Bevy `Update` cycles.
///
/// Builds an `App` from `MinimalPlugins` plus the small set of Bevy
/// foundation plugins the sim's resources / assets / states / transforms
/// need, then composes `init_sandbox_resources` and
/// `init_sandbox_resources` / `add_simulation_plugins` from `ambition_app::app`. Calls `app.update()`
/// `max_ticks` times and returns a `HeadlessReport`.
///
/// Validation failures from the embedded LDtk project propagate as `Err`,
/// matching the production policy that an invalid LDtk file is a hard
/// error rather than a `.expect()` panic.
pub fn run_headless(max_ticks: u32) -> Result<HeadlessReport, String> {
    // Validate the embedded LDtk file up front so we can return Err with a
    // useful diagnostic. `init_sandbox_resources` does this too but exits
    // the process on failure; tests want a structured error instead.
    let project = ldtk_world::LdtkProject::load_default_for_dev()?;
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

    app.add_plugins(SandboxSimulationPlugin);

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
    // Quest log + visited rooms are optional — both are inserted by
    // sandbox startup, but a hypothetical caller that swaps the plugin
    // set might omit them. Use try_resource where it exists so the
    // headless report still produces a partial result rather than
    // panicking on a missing resource.
    let quest_log = world
        .get_resource::<ambition_content::quest::QuestRegistry>()
        .map(|r| r.quest_log_lines())
        .unwrap_or_default();
    let visited_rooms = world
        .get_resource::<ambition_gameplay_core::menu::map::MapMenuState>()
        .map(|m| m.visited.iter().cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    Ok(HeadlessReport {
        ticks_run: max_ticks,
        active_room: active_room_after,
        room_count,
        spawned_entities: stats.spawned_entities,
        spine_revision: spine_index.revision,
        solid_index_revision: solid_index.revision,
        quest_log,
        visited_rooms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_gameplay_core::audio::SfxMessage;
    use ambition_gameplay_core::input::ControlFrame;
    use bevy::ecs::message::Messages;

    fn sandbox_sim_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.add_plugins(ImagePlugin::default());
        app.add_plugins(TransformPlugin);
        app.add_plugins(StatesPlugin);
        app.init_state::<GameMode>();
        app.add_plugins(crate::app::SandboxSimulationPlugin);
        app
    }

    fn initialized_sandbox_sim_app() -> App {
        let mut app = sandbox_sim_app();
        app.update();
        app
    }

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
    /// the player tick end-to-end and observe `SfxMessage` flow? This
    /// proves the sim/presentation seam holds for the input + sfx
    /// channels. Reset is the cheapest path: pressing Reset emits
    /// `SfxMessage::Reset` synchronously, no spawn-position dependence.
    #[test]
    fn sim_emits_sfx_reset_when_control_frame_requests_reset() {
        let mut app = initialized_sandbox_sim_app();

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

    /// Sustained run check: drive the full sim for 60 ticks and
    /// verify the brain action counter is present and its
    /// `last_frame` is internally consistent (≤ total). Catches a
    /// future regression where the brain tick + resolver path
    /// starts panicking somewhere mid-room (player loading the wrong
    /// room and brain seeing inconsistent state) or where the
    /// counter resource gets reset/leaked between frames.
    #[test]
    fn sim_completes_60_ticks_with_counter_intact() {
        use ambition_gameplay_core::brain::BrainActionCounter;
        let mut app = sandbox_sim_app();
        // Run 60 ticks (1 sim second at 60Hz).
        for _ in 0..60 {
            app.update();
        }
        let counter = app.world().resource::<BrainActionCounter>();
        // Total is a running sum, last_frame is per-frame count;
        // last_frame must never exceed total (would indicate the
        // observer is double-counting or the reset got out of
        // order).
        assert!(
            counter.last_frame as u64 <= counter.total,
            "last_frame={} exceeds total={}",
            counter.last_frame,
            counter.total,
        );
    }

    /// Verify the BrainPlugin is installed by SandboxSimulationPlugin
    /// — adding the plugin should mean ActorActionMessage +
    /// BrainActionCounter are both registered. Catches a future
    /// app-plugin refactor that accidentally drops the
    /// `app.add_plugins(ambition_gameplay_core::brain::BrainPlugin)` call.
    #[test]
    fn sim_includes_brain_plugin_registration() {
        use ambition_gameplay_core::brain::{ActorActionMessage, BrainActionCounter};
        use bevy::ecs::message::Messages;
        let app = initialized_sandbox_sim_app();
        // Both resources should be present.
        assert!(
            app.world()
                .get_resource::<Messages<ActorActionMessage>>()
                .is_some(),
            "ActorActionMessage registered via BrainPlugin",
        );
        assert!(
            app.world().get_resource::<BrainActionCounter>().is_some(),
            "BrainActionCounter registered via BrainPlugin",
        );
    }

    /// Sustained run with multiple player attack presses: stamp
    /// attack on every other tick for 20 ticks and verify the
    /// counter accumulates at least 10 melee messages. Pins that
    /// the seam survives sustained brain-message production
    /// (not just single-tick poison).
    #[test]
    fn sim_accumulates_messages_across_repeated_attacks() {
        use ambition_gameplay_core::brain::BrainActionCounter;
        let mut app = initialized_sandbox_sim_app();
        for i in 0..20 {
            let attack = i % 2 == 0;
            *app.world_mut().resource_mut::<ControlFrame>() = ControlFrame {
                attack_pressed: attack,
                ..ControlFrame::default()
            };
            app.update();
        }
        let counter = app.world().resource::<BrainActionCounter>();
        // 10 attack-press ticks × 1 melee message each = 10 total.
        // Other ticks may emit zero or other actions; assert
        // floor.
        assert!(
            counter.total >= 10,
            "expected ≥ 10 ActorActionMessages over 20-tick mix; got {}",
            counter.total,
        );
    }

    /// Universal-brain integration check: spawning the
    /// SandboxSimulationPlugin yields a player entity carrying
    /// Brain::Player and an ActionSet — verifies the bundle
    /// path injects the components even when the spawn flow
    /// runs through the real Startup schedule.
    #[test]
    fn sim_spawns_player_with_brain_and_action_set() {
        use ambition_gameplay_core::brain::{ActionSet, ActorControl, Brain};
        use ambition_gameplay_core::player::PlayerEntity;
        let mut app = initialized_sandbox_sim_app();
        let mut q = app
            .world_mut()
            .query_filtered::<(&Brain, &ActionSet, &ActorControl), With<PlayerEntity>>();
        let count = q.iter(app.world()).count();
        assert_eq!(
            count, 1,
            "player should spawn with Brain + ActionSet + ActorControl"
        );
        let (brain, action_set, _control) = q.iter(app.world()).next().expect("player exists");
        assert!(brain.is_player(), "player carries Brain::Player");
        assert!(
            action_set.melee.is_some(),
            "player ActionSet has Swipe melee"
        );
    }

    /// Universal-brain integration check: with the full
    /// SandboxSimulationPlugin installed, the player carries a
    /// Brain + ActionSet + ActorControl, the brain ticks each
    /// frame, and the ActionSet resolver writes an
    /// ActorActionMessage when the input frame triggers attack.
    /// Validates the production wiring (vs the synthetic mini-app
    /// in `player/systems.rs` tests).
    #[test]
    fn sim_emits_action_messages_when_player_attacks() {
        use ambition_gameplay_core::brain::{ActorActionMessage, BrainActionCounter};
        let mut app = initialized_sandbox_sim_app();
        // Stamp an attack press into the control frame.
        *app.world_mut().resource_mut::<ControlFrame>() = ControlFrame {
            attack_pressed: true,
            ..ControlFrame::default()
        };
        app.update();
        let counter = app.world().resource::<BrainActionCounter>();
        let messages = app.world().resource::<Messages<ActorActionMessage>>();
        let melee_count = messages
            .iter_current_update_messages()
            .filter(|m| m.is_melee())
            .count();
        assert!(
            melee_count >= 1,
            "expected at least one Melee ActorActionMessage; counter.last_frame={}",
            counter.last_frame,
        );
    }
}
