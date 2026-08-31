//! What the game does when the GPU fails under it.
//!
//! ⛔⛔ THE DEFAULT IS "QUIT", AND INHERITING IT WAS A DECISION NOBODY MADE.
//! Bevy 0.19's `RenderErrorHandler` defaults to writing `AppExit::error()` for
//! EVERY `RenderError` — including `DeviceLost`, which on a desktop is routinely
//! a driver update, a GPU reset or a laptop waking from sleep, and is the one
//! category a game can genuinely come back from. Upstream says as much in its
//! own comment ("This is overzealous at the moment"). A player who alt-tabs
//! through a driver reset should not lose their session.
//!
//! ⭐ AND THE OPPOSITE MISTAKE IS WORSE. `RenderErrorPolicy::Ignore` re-runs the
//! frame that just failed; if the cause is still there the result is a hard loop
//! that upstream warns can produce **hazardous rapid flashing**. So recovery is
//! COUNTED, and the count escalates to a stop rather than trying forever.
//!
//! This is presentation/platform state. ⛔ It must never reach rollback
//! simulation: none of it is a fact the two peers of a match agree on, and a
//! host that lost its device has no frame number to blame.

use bevy::prelude::*;
use bevy::render::error_handler::{ErrorType, RenderError, RenderErrorHandler, RenderErrorPolicy};
use bevy::render::settings::RenderCreation;

/// How many device-loss recoveries one run is allowed before the policy stops
/// trying.
///
/// ⭐ TWO, NOT ONE AND NOT MANY. One is indistinguishable from "never recovers"
/// to a player who hit a single driver reset; many is the flashing loop. Two
/// covers the realistic case (a reset, and a second one while the first was
/// still settling) and refuses the pathological one.
pub const MAX_DEVICE_LOST_RECOVERIES: u32 = 2;

/// The host's running tally of render failures.
///
/// ⛔ A RESOURCE, NOT A LOCAL: the handler is a plain `fn` pointer with no state
/// of its own, so the count has to live in the world it is handed. It is
/// deliberately in the MAIN world — the render world is torn down and rebuilt by
/// the very recovery this counts, which would reset the tally that exists to
/// bound it.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RenderRecoveryLedger {
    /// Device-loss recoveries attempted so far in this run.
    pub device_lost_recoveries: u32,
}

/// Install Ambition's render-error policy.
///
/// ⛔ THE VISIBLE WINDOWED HOST ONLY. `NoWindow` has no render app to lose, and
/// `OffscreenGpu` is a capture tool whose right answer to a dead device is to
/// fail the run loudly rather than quietly re-create a device and hand back a
/// picture of something else.
pub fn install_render_recovery(app: &mut App) {
    app.init_resource::<RenderRecoveryLedger>()
        .insert_resource(RenderErrorHandler(decide));
}

/// The policy itself, as a pure decision over the error category.
///
/// Split from [`decide`] so it can be tested without a GPU, a render world or a
/// `RenderError` — the campaign asks for a policy test, and a policy test should
/// not need a device to fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderResponse {
    /// Rebuild the renderer and carry on.
    Recover,
    /// Keep the app alive with rendering stopped, and say why.
    StopRendering,
    /// Leave, deliberately.
    Quit,
}

/// What Ambition does about each category of render failure.
///
/// | category | response | why |
/// |---|---|---|
/// | `DeviceLost` | recover, up to [`MAX_DEVICE_LOST_RECOVERIES`], then stop | a driver reset or a wake from sleep is survivable; an endless one is a flashing loop |
/// | `OutOfMemory` | stop rendering | ⛔ RECOVERING WOULD RE-ALLOCATE what just failed to fit. The frame that ran out is the frame recovery would run again |
/// | `Validation` | quit | wgpu says the engine used it wrongly. That is OUR defect, it will not fix itself, and a player watching it repeat learns nothing |
/// | `Internal` | quit | the driver reported a fault it could not attribute. Fail loudly |
///
/// ⭐ `Ignore` APPEARS NOWHERE. It is the one policy that re-runs the failing
/// frame unchanged, which is exactly the shape upstream warns can strobe.
pub fn response_for(error: ErrorType, recoveries_so_far: u32) -> RenderResponse {
    match error {
        ErrorType::DeviceLost if recoveries_so_far < MAX_DEVICE_LOST_RECOVERIES => {
            RenderResponse::Recover
        }
        ErrorType::DeviceLost => RenderResponse::StopRendering,
        ErrorType::OutOfMemory => RenderResponse::StopRendering,
        ErrorType::Validation | ErrorType::Internal => RenderResponse::Quit,
        // ⛔⛔ NO CATCH-ALL, DELIBERATELY. The first draft had `_ => Quit` with a
        // comment arguing that a wgpu upgrade could add a category — and the
        // compiler answered it: the arm was unreachable, because `ErrorType` is
        // NOT `#[non_exhaustive]`. That makes exhaustiveness the STRONGER guard.
        // A new wgpu category now breaks this build, which is a person reading
        // the new variant and deciding what it means; a catch-all would have
        // given it a default silently, which is the same as never noticing.
    }
}

/// Bridge the pure policy to Bevy's handler signature.
fn decide(
    error: &RenderError,
    main_world: &mut World,
    _render_world: &mut World,
) -> RenderErrorPolicy {
    let ledger = main_world.get_resource::<RenderRecoveryLedger>().copied();
    let recoveries = ledger.map(|l| l.device_lost_recoveries).unwrap_or(0);

    match response_for(error.ty, recoveries) {
        RenderResponse::Recover => {
            let attempt = recoveries + 1;
            if let Some(mut ledger) = main_world.get_resource_mut::<RenderRecoveryLedger>() {
                ledger.device_lost_recoveries = attempt;
            }
            error!(
                target: "ambition_app::render_recovery",
                "render device lost ({}); rebuilding the renderer, attempt {attempt} of {MAX_DEVICE_LOST_RECOVERIES}",
                error.description,
            );
            // ⭐ THE DEFAULT `RenderCreation` IS AMBITION'S, and that is checked
            // rather than assumed: the `VisibleRenderMode::Windowed` arm in
            // `app::cli` adds the plugin group WITHOUT `.set(RenderPlugin { .. })`,
            // so the shipped windowed composition IS Bevy's default. The two
            // arms that do override it (`NoWindow`, `OffscreenGpu`) never
            // install this policy.
            RenderErrorPolicy::Recover(RenderCreation::default())
        }
        RenderResponse::StopRendering => {
            error!(
                target: "ambition_app::render_recovery",
                "rendering stopped after a {:?} render error ({}); the app is still running, \
                 and this is deliberate — retrying would re-run the frame that failed",
                error.ty, error.description,
            );
            RenderErrorPolicy::StopRendering
        }
        RenderResponse::Quit => {
            error!(
                target: "ambition_app::render_recovery",
                "quitting after a {:?} render error ({}); this category means the engine used \
                 the GPU incorrectly, so it will not resolve by trying again",
                error.ty, error.description,
            );
            main_world.write_message(AppExit::error());
            RenderErrorPolicy::StopRendering
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every category Bevy can report has a decided answer.
    ///
    /// ⭐ THIS IS THE TEST THE CAMPAIGN ASKED FOR, and it is a PURE one on
    /// purpose: forcing a real device loss needs a backend that cooperates, and
    /// a policy that cannot be checked without one is a policy nobody checks.
    #[test]
    fn every_render_error_category_has_a_decided_response() {
        assert_eq!(
            response_for(ErrorType::DeviceLost, 0),
            RenderResponse::Recover,
            "a lost device is the one category a game can come back from"
        );
        assert_eq!(
            response_for(ErrorType::OutOfMemory, 0),
            RenderResponse::StopRendering,
            "recovering from OOM would re-allocate exactly what did not fit"
        );
        assert_eq!(response_for(ErrorType::Validation, 0), RenderResponse::Quit);
        assert_eq!(response_for(ErrorType::Internal, 0), RenderResponse::Quit);
    }

    /// Recovery is BOUNDED, and the boundary is the whole point.
    ///
    /// ⛔⛔ AN UNBOUNDED RECOVER IS A FLASHING LOOP. Upstream's own warning is
    /// that re-running a frame whose cause is unaddressed can strobe; a policy
    /// that recovers forever is that warning implemented.
    #[test]
    fn device_loss_recovery_escalates_to_a_stop() {
        for attempt in 0..MAX_DEVICE_LOST_RECOVERIES {
            assert_eq!(
                response_for(ErrorType::DeviceLost, attempt),
                RenderResponse::Recover,
                "attempt {attempt} is within the budget and must still try"
            );
        }
        assert_eq!(
            response_for(ErrorType::DeviceLost, MAX_DEVICE_LOST_RECOVERIES),
            RenderResponse::StopRendering,
            "past the budget the policy must stop rather than keep rebuilding"
        );
        assert_eq!(
            response_for(ErrorType::DeviceLost, MAX_DEVICE_LOST_RECOVERIES + 9),
            RenderResponse::StopRendering,
            "and it must stay stopped"
        );
    }

    /// The ledger counts recoveries, not errors.
    ///
    /// Without this the budget could be spent by categories that never recover:
    /// a run that hit nine validation errors would refuse the first device loss.
    #[test]
    fn the_budget_is_spent_only_by_recoveries() {
        assert_eq!(
            response_for(ErrorType::Validation, MAX_DEVICE_LOST_RECOVERIES),
            RenderResponse::Quit,
            "a spent budget must not change what a validation error means"
        );
        assert_eq!(
            RenderRecoveryLedger::default().device_lost_recoveries,
            0,
            "a run starts with its whole budget"
        );
    }
}
