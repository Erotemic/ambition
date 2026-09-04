//! Developer frame/time control for the real Ambition shell.
//!
//! The capture tools boot purpose-built scenes, which is exactly the wrong
//! instrument for launch defects that appear only after the title-shell route.
//! This panel lives on the ordinary F3 developer surface and controls the live
//! product composition while leaving shell input/routing, asset loading, Egui,
//! and rendering serviced.
//!
//! Manual mode freezes gameplay simulation plus the presentation projections
//! that can make an actor/camera "settle" after a step. A developer can advance
//! one or N canonical simulation frames, optionally request a primary-window
//! screenshot after every advanced frame, and then inspect the frozen result.
//!
//! This is deliberately HOST developer state, not simulation state: nothing
//! here is rollback-registered and none of it may affect a peer.

use std::path::PathBuf;
use std::time::Duration;
#[cfg(all(feature = "visible", not(target_arch = "wasm32")))]
use std::time::{SystemTime, UNIX_EPOCH};

use ambition_platformer2d::dev_tools::{dev_tools::DeveloperTools, DeveloperRuntimeState};
use ambition_platformer2d::platformer::developer_hotkeys::{
    DeveloperAction, DeveloperHotkeyBindings,
};
use ambition_platformer2d::platformer::schedule::{
    GameplayGated, GameplaySimulationRoot, Platformer2dSimulationPhaseMonolith,
    SimScheduleExt as _,
};
use ambition_platformer2d::runtime::SimulationHost;
use ambition_platformer2d::sim::SimTick;
use bevy::prelude::*;
#[cfg(all(feature = "visible", not(target_arch = "wasm32")))]
use bevy::render::view::window::screenshot::{save_to_disk, Screenshot};
use bevy::time::{TimeSystems, Virtual};
use bevy_inspector_egui::bevy_egui::{
    egui, EguiContext, EguiPrimaryContextPass, PrimaryEguiContext,
};

const MIN_SLOWMO_SCALE: f32 = 0.01;
const MAX_SLOWMO_SCALE: f32 = 1.0;
const MAX_BATCH_FRAMES: u32 = 600;
const SCREENSHOT_DIR: &str = "screenshots/frame-step";

/// Host-side state for the developer frame/time panel.
///
/// `step_armed` means THIS rendered update is allowed to advance the gameplay
/// world. `queued_steps` names additional frames still owed after the armed
/// one. A batch therefore advances one world/render frame per app update and
/// freezes again after exactly N advances.
#[derive(Resource, Debug)]
pub(crate) struct FrameStepControl {
    active: bool,
    enter_requested: bool,
    exit_requested: bool,
    step_armed: bool,
    queued_steps: u32,
    batch_size: u32,
    step_period: Option<Duration>,
    virtual_time_was_paused: bool,
    completed_steps: u64,
    step_started_at_tick: Option<u64>,
    last_step_ticks: Option<(u64, u64)>,
    capture_each_step: bool,
    screenshot_requested: bool,
    screenshot_counter: u64,
    last_screenshot_path: Option<PathBuf>,
    last_screenshot_error: Option<String>,
}

impl FrameStepControl {
    fn new(step_period: Option<Duration>) -> Self {
        Self {
            active: false,
            enter_requested: false,
            exit_requested: false,
            step_armed: false,
            queued_steps: 0,
            batch_size: 1,
            step_period,
            virtual_time_was_paused: false,
            completed_steps: 0,
            step_started_at_tick: None,
            last_step_ticks: None,
            capture_each_step: false,
            screenshot_requested: false,
            screenshot_counter: 0,
            last_screenshot_path: None,
            last_screenshot_error: None,
        }
    }

    fn can_queue_steps(&self) -> bool {
        self.active && self.step_period.is_some() && !self.step_armed && self.queued_steps == 0
    }

    fn queue_steps(&mut self, count: u32) {
        if self.can_queue_steps() {
            self.queued_steps = count.clamp(1, MAX_BATCH_FRAMES);
        }
    }

    fn remaining_steps(&self) -> u32 {
        self.queued_steps + u32::from(self.step_armed)
    }

    fn arm_next_step(&mut self, tick: u64) {
        if !self.active || self.step_armed || self.queued_steps == 0 {
            return;
        }
        self.queued_steps -= 1;
        self.step_armed = true;
        self.step_started_at_tick = Some(tick);
    }
}

/// Installs the product-shell frame/time debugger. Only compiled with
/// `dev_tools`; regular F3 controls its visibility through `DeveloperTools`.
pub(crate) struct FrameStepPanelPlugin;

impl Plugin for FrameStepPanelPlugin {
    fn build(&self, app: &mut App) {
        // ⭐ ONE PERIOD AUTHORITY. Do not restate 60Hz here: rollback and the
        // fixed host intentionally differ by one nanosecond, and the stepping
        // SDK already owns that distinction.
        let step_period = ambition_platformer2d::sim::manual_step_period(app);
        app.insert_resource(FrameStepControl::new(step_period));

        // The gameplay schedule may be FixedUpdate, Update, or GgrsSchedule.
        // Gate its umbrella sets rather than enumerating gameplay systems here.
        let sim = app.sim_schedule();
        app.configure_sets(
            sim,
            (
                GameplaySimulationRoot.run_if(frame_step_allows_world_frame),
                GameplayGated.run_if(frame_step_allows_world_frame),
            ),
        );

        // GGRS is the OUTER driver of the sim schedule. Gating only systems
        // inside GgrsSchedule would still let the backend advance/save its
        // timeline around an empty gameplay run.
        if app
            .world()
            .get_resource::<SimulationHost>()
            .copied()
            .is_some_and(SimulationHost::is_rollback)
        {
            app.configure_sets(
                PreUpdate,
                ambition_platformer2d::rollback::RunGgrsSystems
                    .run_if(frame_step_allows_world_frame),
            );
        }

        // A frozen simulation with presentation still settling is the wrong
        // instrument for the launch-time sprite pop. Freeze the published
        // projection phases that own actor visuals, presented poses, and camera
        // observation while shell/loading/debug UI continue to run.
        app.configure_sets(
            Update,
            ambition_platformer2d::sim_view::presented_pose::PresentedPoseSet
                .run_if(frame_step_allows_world_frame),
        );
        app.configure_sets(
            Update,
            ambition_platformer2d::sim_view::camera_snapshot::CameraObservationSet
                .run_if(frame_step_allows_world_frame),
        );
        app.configure_sets(
            Update,
            Platformer2dSimulationPhaseMonolith::PresentationVisualSync
                .run_if(frame_step_allows_world_frame),
        );

        // Bevy has already updated Time<Virtual> from wall time when this runs.
        // While paused that update is zero. On an armed update, inject the
        // canonical simulation period and publish the same generic Time value
        // ordinary Update systems read.
        app.add_systems(
            First,
            inject_one_frame_of_virtual_time.after(TimeSystems),
        )
        .add_systems(EguiPrimaryContextPass, frame_step_panel_ui)
        .add_systems(Last, settle_frame_step_requests);
    }
}

/// Shared run condition for simulation and presentation projection.
fn frame_step_state_allows_world_frame(control: &FrameStepControl) -> bool {
    !control.active || control.step_armed
}

fn frame_step_allows_world_frame(control: Option<Res<FrameStepControl>>) -> bool {
    control
        .as_deref()
        .is_none_or(frame_step_state_allows_world_frame)
}

/// Advance virtual game time by exactly one canonical simulation period on an
/// armed frame, while wall/real time keeps Egui and the host shell responsive.
fn inject_one_frame_of_virtual_time(
    control: Res<FrameStepControl>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut time: ResMut<Time>,
) {
    if !control.active || !control.step_armed {
        return;
    }
    let Some(period) = control.step_period else {
        return;
    };

    virtual_time.unpause();
    virtual_time.advance_by(period);
    *time = virtual_time.as_generic();
}

/// Queue a screenshot of the ACTUAL primary window. Unlike the offscreen
/// capture harness this asks Bevy's normal renderer for the product window and
/// does not pump extra app updates while GPU readback completes.
#[cfg(all(feature = "visible", not(target_arch = "wasm32")))]
fn request_primary_window_screenshot(
    commands: &mut Commands,
    control: &mut FrameStepControl,
    tick: u64,
    tag: &str,
) {
    if let Err(error) = std::fs::create_dir_all(SCREENSHOT_DIR) {
        control.last_screenshot_error = Some(format!(
            "could not create {SCREENSHOT_DIR}: {error}"
        ));
        return;
    }

    control.screenshot_counter = control.screenshot_counter.saturating_add(1);
    let epoch_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    let path = PathBuf::from(SCREENSHOT_DIR).join(format!(
        "{epoch_ms}-tick-{tick:08}-{:04}-{tag}.png",
        control.screenshot_counter
    ));
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path.clone()));
    control.last_screenshot_path = Some(path);
    control.last_screenshot_error = None;
}

#[cfg(not(all(feature = "visible", not(target_arch = "wasm32"))))]
fn request_primary_window_screenshot(
    _commands: &mut Commands,
    control: &mut FrameStepControl,
    _tick: u64,
    _tag: &str,
) {
    control.last_screenshot_error = Some(
        "primary-window PNG capture requires a native visible build".to_owned(),
    );
}

/// Apply panel commands at the end of the frame so a click never half-steps the
/// update that delivered it. Every state change takes effect on the NEXT frame.
fn settle_frame_step_requests(
    mut commands: Commands,
    mut control: ResMut<FrameStepControl>,
    mut virtual_time: ResMut<Time<Virtual>>,
    tick: Option<Res<SimTick>>,
) {
    let now = tick.as_deref().map_or(0, |tick| tick.get());

    // Finish the frame armed last update before considering new UI requests.
    if control.active && control.step_armed {
        if let Some(before) = control.step_started_at_tick.take() {
            control.last_step_ticks = Some((before, now));
        }
        control.completed_steps = control.completed_steps.saturating_add(1);
        control.step_armed = false;
        virtual_time.pause();

        if control.capture_each_step {
            request_primary_window_screenshot(&mut commands, &mut control, now, "step");
        }
    }

    if control.screenshot_requested {
        control.screenshot_requested = false;
        request_primary_window_screenshot(&mut commands, &mut control, now, "manual");
    }

    if control.exit_requested {
        control.exit_requested = false;
        control.enter_requested = false;
        control.queued_steps = 0;
        control.step_armed = false;
        control.step_started_at_tick = None;
        control.active = false;
        if !control.virtual_time_was_paused {
            virtual_time.unpause();
        } else {
            virtual_time.pause();
        }
        return;
    }

    if control.enter_requested {
        control.enter_requested = false;
        if control.step_period.is_some() && !control.active {
            control.virtual_time_was_paused = virtual_time.is_paused();
            control.active = true;
            control.queued_steps = 0;
            control.step_armed = false;
            control.step_started_at_tick = None;
            virtual_time.pause();
        }
    }

    if control.active {
        // Defensive reassertion: an unrelated developer control must not
        // accidentally unpause the clock underneath the world-frame gate.
        virtual_time.pause();
        control.arm_next_step(now);
    }
}

fn frame_step_panel_ui(world: &mut World) {
    let inspector_visible = world
        .get_resource::<DeveloperTools>()
        .is_some_and(|tools| tools.inspector_visible);
    if !inspector_visible {
        return;
    }

    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
        .single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    let Some(control) = world.get_resource::<FrameStepControl>() else {
        return;
    };

    let host = world
        .get_resource::<SimulationHost>()
        .copied()
        .unwrap_or_default();
    let tick = world.get_resource::<SimTick>().map_or(0, |tick| tick.get());
    let hotkey = world
        .get_resource::<DeveloperHotkeyBindings>()
        .and_then(|bindings| bindings.label_for(DeveloperAction::ToggleInspector))
        .unwrap_or_else(|| "F3".to_owned());

    let active = control.active;
    let supported = control.step_period.is_some();
    let can_queue = control.can_queue_steps();
    let completed_steps = control.completed_steps;
    let last_step_ticks = control.last_step_ticks;
    let remaining_steps = control.remaining_steps();
    let period = control.step_period;
    let mut batch_size = control.batch_size;
    let mut capture_each_step = control.capture_each_step;
    let last_screenshot_path = control.last_screenshot_path.clone();
    let last_screenshot_error = control.last_screenshot_error.clone();

    let (mut slowmo, mut slowmo_scale) = world
        .get_resource::<DeveloperRuntimeState>()
        .map(|state| (state.slowmo, state.slowmo_scale))
        .unwrap_or((false, 0.25));
    slowmo_scale = slowmo_scale.clamp(MIN_SLOWMO_SCALE, MAX_SLOWMO_SCALE);

    let mut enter = false;
    let mut exit = false;
    let mut step_one = false;
    let mut step_batch = false;
    let mut screenshot = false;

    egui::Window::new("Frame / Time Control")
        .default_width(370.0)
        .resizable(false)
        .show(egui_context.get_mut(), |ui| {
            ui.label(format!("Host: {host:?}   Sim tick: {tick}"));
            if let Some(period) = period {
                ui.small(format!(
                    "One frame = {:.3} ms of canonical simulation time",
                    period.as_secs_f64() * 1000.0
                ));
            } else {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    "This composition uses RenderFrame simulation; exact fixed-step mode is unavailable.",
                );
            }

            ui.separator();
            ui.strong("Realtime speed");
            ui.horizontal(|ui| {
                ui.checkbox(&mut slowmo, "Developer slow motion");
                ui.label(format!("{slowmo_scale:.3}x"));
            });
            ui.add(
                egui::Slider::new(&mut slowmo_scale, MIN_SLOWMO_SCALE..=MAX_SLOWMO_SCALE)
                    .logarithmic(true)
                    .text("speed multiplier"),
            );
            ui.small(
                "This edits the existing developer clock request used by F2; 1.0x is realtime and values below 1 slow gameplay.",
            );

            ui.separator();
            ui.strong("Manual frame advance");
            if active {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::LIGHT_GREEN, "FRAME-STEP MODE");
                    ui.label("world frozen between advances");
                });

                ui.horizontal(|ui| {
                    ui.add_enabled_ui(can_queue, |ui| {
                        if ui.button("Advance 1 frame").clicked() {
                            step_one = true;
                        }
                    });
                    ui.add(
                        egui::DragValue::new(&mut batch_size)
                            .speed(1.0)
                            .prefix("N = "),
                    );
                    ui.add_enabled_ui(can_queue, |ui| {
                        if ui.button("Advance N frames").clicked() {
                            step_batch = true;
                        }
                    });
                });

                if remaining_steps > 0 {
                    ui.small(format!("{remaining_steps} frame(s) queued / executing."));
                }
                if ui.button("Resume realtime").clicked() {
                    exit = true;
                }
            } else if supported {
                if ui
                    .add_sized([210.0, 32.0], egui::Button::new("Enter frame-step mode"))
                    .clicked()
                {
                    enter = true;
                }
                ui.small(
                    "For title-shell launch bugs, enter frame-step mode here, then start Ambition normally. The menu remains interactive because it is shell/UI work, not gameplay simulation.",
                );
            }

            ui.separator();
            ui.strong("Screenshots");
            let screenshot_supported = cfg!(all(feature = "visible", not(target_arch = "wasm32")));
            ui.add_enabled_ui(screenshot_supported, |ui| {
                if ui.button("Capture primary window now").clicked() {
                    screenshot = true;
                }
                ui.checkbox(
                    &mut capture_each_step,
                    "Capture after every manually advanced frame",
                );
            });
            ui.small(format!("Output: {SCREENSHOT_DIR}/"));
            if let Some(path) = last_screenshot_path.as_ref() {
                ui.small(format!("Last requested: {}", path.display()));
            }
            if let Some(error) = last_screenshot_error.as_ref() {
                ui.colored_label(egui::Color32::YELLOW, error);
            }
            ui.small(
                "These are Bevy primary-window screenshots of the real running shell; they do not use capture_scene or pump extra app updates.",
            );

            ui.separator();
            ui.label(format!("Completed manual frames: {completed_steps}"));
            if let Some((before, after)) = last_step_ticks {
                let delta = after.saturating_sub(before);
                let text = format!("Last advance SimTick: {before} -> {after} (delta {delta})");
                if delta == 1 {
                    ui.label(text);
                } else {
                    ui.colored_label(egui::Color32::YELLOW, text);
                    ui.small(
                        "A delta other than 1 means that rendered frame was shell/loading work or the host advanced differently than expected.",
                    );
                }
            }

            ui.separator();
            ui.small(
                "Shell input/routing, asset loading, Egui and rendering keep running. Gameplay simulation, actor visual projection, presented poses and camera observation only advance on the manual-frame gate.",
            );
            ui.small(format!(
                "{hotkey} hides/shows the developer tools without resuming frame-step mode."
            ));
        });

    if let Some(mut state) = world.get_resource_mut::<DeveloperRuntimeState>() {
        state.slowmo = slowmo;
        state.slowmo_scale = slowmo_scale.clamp(MIN_SLOWMO_SCALE, MAX_SLOWMO_SCALE);
    }

    if let Some(mut control) = world.get_resource_mut::<FrameStepControl>() {
        control.batch_size = batch_size.clamp(1, MAX_BATCH_FRAMES);
        control.capture_each_step = capture_each_step;
        if enter {
            control.enter_requested = true;
        }
        if exit {
            control.exit_requested = true;
        }
        if step_one {
            control.queue_steps(1);
        } else if step_batch {
            let batch_size = control.batch_size;
            control.queue_steps(batch_size);
        }
        if screenshot {
            control.screenshot_requested = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_paused_stepper_only_opens_the_world_for_an_armed_frame() {
        let mut control = FrameStepControl::new(Some(Duration::from_millis(16)));
        assert!(!control.active);

        control.active = true;
        assert!(!frame_step_state_allows_world_frame(&control));

        control.queue_steps(1);
        control.arm_next_step(9);
        assert!(frame_step_state_allows_world_frame(&control));
        assert_eq!(control.remaining_steps(), 1);
    }

    #[test]
    fn a_batch_arms_one_frame_at_a_time() {
        let mut control = FrameStepControl::new(Some(Duration::from_millis(16)));
        control.active = true;
        control.queue_steps(3);
        assert_eq!(control.remaining_steps(), 3);

        control.arm_next_step(10);
        assert!(control.step_armed);
        assert_eq!(control.queued_steps, 2);
        assert_eq!(control.remaining_steps(), 3);

        control.step_armed = false;
        control.arm_next_step(11);
        assert_eq!(control.queued_steps, 1);
        assert_eq!(control.step_started_at_tick, Some(11));
    }
}
