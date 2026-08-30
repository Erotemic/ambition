//! ⭐⭐ PHOTOGRAPH AN EXACT SIMULATION TICK, with the GPU's latency held outside
//! the clock.
//!
//! The hard problem this solves is not readback; it is that **a slow adapter must
//! not change WHICH tick a picture belongs to**. Under Lavapipe a readback takes
//! several passes of the render service loop, and an ordinary `App::run()` spends
//! real time on each of them — so a capture requested on tick N lands somewhere
//! after it, and a manifest that says "tick N" is guessing.
//!
//! ⭐ SO SIMULATION TIME AND GPU TIME ARE SEPARATED. The sim advances only at the
//! caller's canonical manual period; a pending readback is serviced with
//! `ManualDuration(ZERO)`, which runs every schedule and moves no clock. The
//! session then CHECKS that the tick did not move and refuses the frame if it
//! did, rather than writing provenance it cannot stand behind.
//!
//! ⛔⛔ EXTRACTED FROM `moveset_render` 2026-08-29, WHERE IT WAS THE WHOLE VALUE
//! AND NONE OF THE REUSE. A 2026-08-29 review named it: room capture, match
//! screenshots, character previews and future visual-regression tools all want
//! this property, and it was sitting in a `bin/`.
//!
//! ⛔ COMPOSITION STAYS WITH THE CALLER. This adopts an App that is already built
//! in an offscreen-GPU mode and already finalized; it does not know about the
//! product shell, which is what keeps the harness below it.

use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::prelude::*;

use ambition_platformer2d::capture::{
    CaptureProgress, CaptureSettings, CaptureTarget,
};

/// How many zero-duration pumps a single readback may take before the session
/// calls it a failure.
///
/// ⛔ A BUDGET, NOT A TIMEOUT. Every pump is free in simulation time, so the only
/// thing this bounds is a readback that will never complete — a driver that spun
/// forever would look like a hung renderer with no message.
pub const MAX_PUMPS_PER_FRAME: usize = 600;

/// What went wrong, in the caller's vocabulary rather than the renderer's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureError {
    /// ⛔⛔ THE ONE THAT MUST NEVER BE SWALLOWED. A zero-duration pump moved the
    /// fixed clock, so the picture belongs to no stated tick. A frame with false
    /// provenance is worse than no frame, because a viewer synchronises on it.
    SimulationAdvanced {
        expected: u64,
        observed: u64,
        pumps: usize,
    },
    /// The readback did not finish inside [`MAX_PUMPS_PER_FRAME`].
    ReadbackNeverCompleted { pumps: usize },
    /// The app was not composed with a capture target — the caller skipped
    /// [`DeterministicCaptureSession::install`] or ran it in a windowless mode
    /// that has no render app at all.
    NoCaptureTarget,
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SimulationAdvanced {
                expected,
                observed,
                pumps,
            } => write!(
                f,
                "a zero-duration pump advanced the simulation from tick {expected} to \
                 {observed} (pump {pumps}); the frame cannot name the tick it shows"
            ),
            Self::ReadbackNeverCompleted { pumps } => write!(
                f,
                "the readback never completed in {pumps} zero-duration pump(s)"
            ),
            Self::NoCaptureTarget => write!(
                f,
                "no CaptureTarget resource — the app was not composed for offscreen \
                 capture (see DeterministicCaptureSession::install)"
            ),
        }
    }
}

/// One picture, and the tick it belongs to.
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub path: PathBuf,
    /// The absolute `SimTick` the picture shows.
    pub sim_tick: u64,
    /// How many zero-duration pumps this readback took. ⭐ Reported because it is
    /// the evidence that the scheme WORKS — a nonzero count with an unchanged
    /// tick is the property this type exists for.
    pub pumps: usize,
}

/// Drives offscreen captures without letting GPU latency move the clock.
pub struct DeterministicCaptureSession {
    canonical: Duration,
    size: UVec2,
}

impl DeterministicCaptureSession {
    /// Install everything the capture road needs EXCEPT the target's own
    /// `Startup` ordering, which the caller owns.
    ///
    /// ⛔⛔ THE ORDERING IS THE CALLER'S BECAUSE THE SET IS THE SHELL'S.
    /// `setup_capture_target` must run after whatever built the cameras, and in
    /// Ambition that is `ambition_app::app::PresentationSetupSet` — a product
    /// shell this crate deliberately sits below. A no-op local anchor would
    /// compile and order against NOTHING, which is the silent version of getting
    /// this wrong, so the caller schedules that one system itself with
    /// `ambition_platformer2d::capture::setup_capture_target`.
    ///
    /// ⛔ AND CALL ALL OF THIS BEFORE `finalize`, finalize before stepping: Bevy
    /// builds the render device in plugin `finish()`, which `App::run()` performs
    /// and a hand-driven loop never does. Without it the first frame panics
    /// inside `bevy_pbr`'s skin batching with *"Res<RenderDevice> failed
    /// validation"*.
    pub fn install(app: &mut App, size: UVec2, first_output: impl AsRef<Path>) {
        app.insert_resource(CaptureSettings {
            output: first_output.as_ref().to_path_buf(),
            size,
            include_ui: false,
        });
        app.init_resource::<CaptureProgress>();
        app.add_systems(
            Update,
            ambition_platformer2d::capture::adopt_cameras_into_capture_target,
        );
    }


    /// Adopt an app that is composed, finalized and stepping manually.
    ///
    /// `canonical` is the manual period the caller advances the simulation by —
    /// the session restores it after every capture, so a pump can never leak
    /// into the next step.
    pub fn adopt(canonical: Duration, size: UVec2) -> Self {
        Self { canonical, size }
    }

    /// The absolute simulation tick, or 0 where nothing publishes one.
    pub fn sim_tick(app: &App) -> u64 {
        app.world()
            .get_resource::<ambition_platformer2d::sim::SimTick>()
            .map(|t| t.0)
            .unwrap_or_default()
    }

    /// Photograph the CURRENT tick into `output`.
    ///
    /// ⭐ THE TICK IS READ BEFORE THE SHUTTER AND CHECKED AFTER EVERY PUMP. Frozen
    /// time is not a frozen WORLD — the pump runs `Update`, and anything there
    /// that does not gate on the fixed clock keeps going — so a caller reading
    /// diagnostics for the manifest must read them BEFORE calling this, not
    /// after.
    pub fn capture(
        &self,
        app: &mut App,
        output: impl AsRef<Path>,
    ) -> Result<CapturedFrame, CaptureError> {
        let output = output.as_ref().to_path_buf();
        let tick = Self::sim_tick(app);

        {
            let world = app.world_mut();
            let Some(target) = world.remove_resource::<CaptureTarget>() else {
                return Err(CaptureError::NoCaptureTarget);
            };
            let mut progress = CaptureProgress::default();
            world.insert_resource(CaptureSettings {
                output: output.clone(),
                size: self.size,
                include_ui: false,
            });
            let mut commands = world.commands();
            ambition_platformer2d::capture::request_capture(
                &mut commands,
                &target,
                &mut progress,
            );
            world.insert_resource(target);
            world.insert_resource(progress);
            world.flush();
        }

        // ⭐⭐ SERVICE THE GPU AT ZERO COST. Every pump runs the schedules and
        // moves no clock, so the picture belongs to `tick` and to no other.
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::ZERO,
        ));
        let mut pumps = 0usize;
        let mut done = false;
        let mut drift: Option<u64> = None;
        while pumps < MAX_PUMPS_PER_FRAME {
            app.update();
            pumps += 1;
            // ⛔⛔ A HARD CHECK, NOT A `debug_assert!`. As one it COMPILED OUT OF
            // A RELEASE BUILD — a build the inspector's server will happily
            // select, because it prefers the newest renderer on disk regardless
            // of profile. The single check standing between "this PNG belongs to
            // tick N" and a silent lie was absent from exactly the binary most
            // likely to be running.
            let now = Self::sim_tick(app);
            if now != tick {
                drift = Some(now);
                break;
            }
            if app
                .world()
                .get_resource::<CaptureProgress>()
                .is_some_and(|p| p.completed)
            {
                done = true;
                break;
            }
        }
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            self.canonical,
        ));

        if let Some(observed) = drift {
            // ⛔ AND IT REFUSES RATHER THAN RECORDS. A half-written frame goes
            // with the error; nothing downstream should ever see it.
            let _ = std::fs::remove_file(&output);
            return Err(CaptureError::SimulationAdvanced {
                expected: tick,
                observed,
                pumps,
            });
        }
        if !done {
            return Err(CaptureError::ReadbackNeverCompleted { pumps });
        }
        Ok(CapturedFrame {
            path: output,
            sim_tick: tick,
            pumps,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⛔ EVERY ERROR SAYS WHICH TICK AND HOW MANY PUMPS. A capture failure that
    /// reads "readback failed" sends the reader to the driver; these send them
    /// to the clock, which is where the bug was both times it happened.
    #[test]
    fn a_drift_error_names_both_ticks_and_the_pump() {
        let err = CaptureError::SimulationAdvanced {
            expected: 31,
            observed: 32,
            pumps: 4,
        };
        let text = err.to_string();
        assert!(text.contains("31") && text.contains("32"), "{text}");
        assert!(text.contains("pump 4"), "{text}");
        assert!(
            text.contains("cannot name the tick"),
            "the message must say what is WRONG with the frame, not only that \
             something moved: {text}"
        );
    }

    #[test]
    fn a_missing_target_points_at_the_composition_not_the_gpu() {
        let text = CaptureError::NoCaptureTarget.to_string();
        assert!(
            text.contains("not composed for offscreen capture"),
            "a missing CaptureTarget is a composition mistake and must not read \
             as a driver failure: {text}"
        );
    }
}
