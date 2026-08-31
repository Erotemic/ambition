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
    /// The category this host has already declared TERMINAL and said so about.
    ///
    /// ⛔⛔ WITHOUT THIS, A TERMINAL ERROR IS AN INFINITE LOG. Bevy's
    /// `RenderErrorPolicy::StopRendering` is documented to KEEP THE ERROR STATE
    /// and "continue polling the `RenderErrorHandler` every frame until some
    /// other policy is returned" — and a terminal decision by definition never
    /// returns another policy. So `decide` is called again on the next frame,
    /// and the next, with the same preserved error; every side effect it
    /// performs (an `error!` line, an `AppExit` message) repeats at frame rate
    /// for as long as the process lives.
    ///
    /// ⭐ KEYED ON THE CATEGORY, NOT A BARE `bool`. Being re-polled with the
    /// SAME error is the noise; a genuinely DIFFERENT category arriving after a
    /// stop is news, and gets its one line.
    pub terminal_reported: Option<ErrorType>,
}

impl RenderRecoveryLedger {
    /// Is this terminal error NEW news, or the same one being re-polled?
    ///
    /// Pure, and separate from [`decide`], for the same reason [`response_for`]
    /// is: the defect it guards is about being called twice, and a test should
    /// be able to call it twice without a GPU.
    fn take_terminal_report(&mut self, error: ErrorType) -> bool {
        if self.terminal_reported == Some(error) {
            return false;
        }
        self.terminal_reported = Some(error);
        true
    }
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

/// Is this the FIRST poll that has reported this terminal category?
///
/// ⛔ A HOST WITH NO LEDGER STILL REPORTS. `install_render_recovery` always
/// inserts one, so this is unreachable in the shipped app; if it ever is
/// reachable, a terminal failure that says nothing is the worse of the two
/// failures, so the missing-ledger case is loud rather than silent.
fn report_once(world: &mut World, error: ErrorType) -> bool {
    world
        .get_resource_mut::<RenderRecoveryLedger>()
        .is_none_or(|mut ledger| ledger.take_terminal_report(error))
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
            if report_once(main_world, error.ty) {
                error!(
                    target: "ambition_app::render_recovery",
                    "rendering stopped after a {:?} render error ({}); the app is still running, \
                     and this is deliberate — retrying would re-run the frame that failed. \
                     Bevy will re-poll this handler every frame; it will stay quiet",
                    error.ty, error.description,
                );
            }
            RenderErrorPolicy::StopRendering
        }
        RenderResponse::Quit => {
            if report_once(main_world, error.ty) {
                error!(
                    target: "ambition_app::render_recovery",
                    "quitting after a {:?} render error ({}); this category means the engine used \
                     the GPU incorrectly, so it will not resolve by trying again",
                    error.ty, error.description,
                );
                // ⛔ ONE EXIT MESSAGE. Bevy re-polls this handler on every frame
                // between the decision and the actual shutdown, and each of
                // those polls would otherwise queue another `AppExit`.
                main_world.write_message(AppExit::error());
            }
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

    /// A terminal error re-polled every frame reports ONCE.
    ///
    /// ⛔⛔ THIS IS THE ARM `response_for` CANNOT PROVIDE, AND THE REASON THE
    /// DEFECT SURVIVED A GREEN POLICY TEST. `response_for` is a pure function of
    /// (category, count): calling it a thousand times is calling it once. The
    /// defect lives in `decide`'s SIDE EFFECTS under Bevy's documented contract
    /// that `StopRendering` keeps the error and re-polls the handler every
    /// frame — so the test has to be about the second call, not the first.
    ///
    /// `AppExit` is the observable half: the `Quit` path used to queue one exit
    /// message per frame for the whole interval between the decision and the
    /// shutdown.
    #[test]
    fn a_terminal_error_repolled_every_frame_is_reported_once() {
        fn poll(main: &mut World, render: &mut World, ty: ErrorType) -> RenderErrorPolicy {
            decide(
                &RenderError {
                    ty,
                    description: "a test".to_string(),
                    source: None,
                },
                main,
                render,
            )
        }
        fn exits(world: &mut World) -> usize {
            world
                .get_resource::<Messages<AppExit>>()
                .map(|messages| messages.iter_current_update_messages().count())
                .unwrap_or(0)
        }

        let mut main = World::new();
        let mut render = World::new();
        main.init_resource::<RenderRecoveryLedger>();
        main.init_resource::<Messages<AppExit>>();

        // A category that QUITS, polled the way Bevy polls it.
        assert!(matches!(
            poll(&mut main, &mut render, ErrorType::Validation),
            RenderErrorPolicy::StopRendering
        ));
        assert_eq!(
            exits(&mut main),
            1,
            "premise: the first poll of a quitting category must ask to exit"
        );
        for _ in 0..30 {
            poll(&mut main, &mut render, ErrorType::Validation);
        }
        assert_eq!(
            exits(&mut main),
            1,
            "thirty more polls of the SAME preserved error must add no exits"
        );
        assert_eq!(
            main.resource::<RenderRecoveryLedger>().terminal_reported,
            Some(ErrorType::Validation),
            "the ledger is what remembers that it was already said"
        );

        // A DIFFERENT category after a stop is news, and gets its one report.
        poll(&mut main, &mut render, ErrorType::Internal);
        assert_eq!(
            exits(&mut main),
            2,
            "a different terminal category is new information, not the same \
             error being re-polled"
        );
    }

    /// Exhausting the recovery budget is terminal, and terminal means quiet.
    ///
    /// The device-lost road reaches `StopRendering` by a different route than
    /// `OutOfMemory` does — through the budget rather than straight off the
    /// category — so it gets its own arm.
    #[test]
    fn an_exhausted_device_lost_budget_stops_reporting_too() {
        let mut ledger = RenderRecoveryLedger {
            device_lost_recoveries: MAX_DEVICE_LOST_RECOVERIES,
            terminal_reported: None,
        };
        assert_eq!(
            response_for(ErrorType::DeviceLost, ledger.device_lost_recoveries),
            RenderResponse::StopRendering,
            "premise: the budget must actually be exhausted, or the arms below \
             are testing the recovery road"
        );
        assert!(
            ledger.take_terminal_report(ErrorType::DeviceLost),
            "the first poll after the budget runs out must say so"
        );
        for _ in 0..10 {
            assert!(
                !ledger.take_terminal_report(ErrorType::DeviceLost),
                "every later poll of the same preserved device loss must be quiet"
            );
        }
    }
}
