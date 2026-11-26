//! Portal M2: a step interrupted where the body's root crosses into another
//! chart, and continued there under the same movement policy.
//!
//! A whole step integrated in the chart the body started in, then mapped
//! through the pair, turns everything the policy did after the crossing
//! (gravity, drag, input, a support it met) with the map: through a quarter
//! turn the rest of the tick's fall comes out sideways. So the kernel runs
//! the step over the window up to the crossing, moves the root through the
//! pair at the crossing point, and runs the rest of the window in the exit's
//! chart, where the same frame now means the exit side's down. Each part is
//! an ordinary step over its own world-time window ([`StepSpan`]), so its
//! casts meet moving geometry and the pair's recorded motion at the right
//! time, and every movement model keeps its own law.

use super::*;
use super::super::ChartChange;
use crate::frame::thread::{MotionAdmission, PortalLink, Refusal, RefusalReason, Reroot, StepSpan};
use crate::frame::{NumericFault, RootCrossing, RootTransfer};

fn numeric(fault: NumericFault) -> Refusal {
    Refusal { path: Vec::new(), link: None, reason: RefusalReason::Numeric(fault) }
}

/// Step `clusters` over the window of `admission`, moving its root through
/// each admitted link it crosses, up to the budget's hops.
pub(super) fn step_through_crossings(
    model: &mut MotionModel,
    clusters: &mut BodyClustersMut<'_>,
    ctx: &MotionStepContext<'_>,
    retained_down: Vec2,
    admission: MotionAdmission<'_>,
    reroot: Reroot<'_>,
    hops: u8,
) -> Result<MotionStepResult, Refusal> {
    let [start, end] = admission.span.window;
    let (candidate, sweep, result) = candidate_motion(model, clusters, ctx, retained_down)?;
    let crossing = if hops < admission.budget.max_hops && end > start {
        let domain = clusters.kinematics.canonical_box().map_err(numeric)?;
        first_root_crossing(admission, reroot, &sweep, &domain)?
    } else {
        None
    };
    let Some((root, link)) = crossing else {
        commit_candidate(candidate, sweep, model, clusters);
        return Ok(result);
    };
    drop(candidate);
    let at = root.world_time.clamp(start, end);
    let share = ((at - start) / (end - start)) as f32;
    let within = |window: [f64; 2], first: Option<u32>| MotionAdmission {
        first, span: StepSpan { window, ..admission.span }, ..admission
    };

    // Up to the crossing, in the chart the body is in.
    let before_dt = ctx.dt * share;
    let (mut events, before) = if at > start {
        let before_ctx = MotionStepContext { dt: before_dt, admission: Some(within([start, at], admission.first)), ..*ctx };
        let (result, sweep) = try_motion_once(model, clusters, &before_ctx, retained_down)?;
        (result.events, sweep)
    } else {
        let mut sweep = SweepSample::at_rest(*clusters.kinematics, retained_down);
        sweep.time = [start, start];
        (FrameEvents::default(), sweep)
    };

    // Through the pair, from where the root crossed, at the velocity the
    // policy reached by then.
    let pose = link.snapshot_at(at).map_err(numeric)?;
    let body = *clusters.kinematics;
    let transfer = RootTransfer::through(&pose.enter, &pose.exit, admission.convention,
        root.point, body.vel, body.material, reroot.min_exit_speed).map_err(numeric)?;
    let side = ctx.frame.basis().side;
    clusters.flight.carried_run = transfer.carried_run(side, side, clusters.flight.carried_run).map_err(numeric)?;
    clusters.kinematics.pos = transfer.exit_pos;
    clusters.kinematics.vel = transfer.exit_vel;
    clusters.kinematics.material = body.material.carried(transfer.map).map_err(numeric)?;
    super::super::authority::release_departure(model, clusters);
    events.chart_changes.push(ChartChange {
        link, pose, world_time: at, point: root.point, transfer,
        before: SweepSample { curr: root.point, ..before },
    });

    // The rest of the window in the exit's chart: in at the exit, the body's
    // centre is on the front of the pair's other way.
    let reverse = admission.links.iter()
        .find(|other| other.enter == link.exit && other.exit == link.enter)
        .map(|other| other.key);
    if at < end {
        let mut after_ctx = MotionStepContext {
            dt: ctx.dt - before_dt, admission: Some(within([at, end], reverse)), ..*ctx
        };
        // A hold goes on the way it went in: through a pair that mirrors the
        // run, the held run and facing are mirrored for the rest of the step,
        // as the seat's held input is after a crossing (`PortalInputWarp`).
        // Unwarped, "hold right" turned a mirrored body round mid-step.
        if mirrors_the_run(transfer, ctx.frame) {
            let mirror = |axes: crate::LocalAxes| crate::LocalAxes::new(-axes.x, axes.y);
            after_ctx.input.axes = mirror(after_ctx.input.axes);
            after_ctx.input.undamped_axes = after_ctx.input.undamped_axes.map(mirror);
            after_ctx.facing_intent = -after_ctx.facing_intent;
        }
        let after = step_through_crossings(model, clusters, &after_ctx, retained_down,
            within([at, end], reverse), reroot, hops.saturating_add(1))?;
        events.extend(after.events);
        return Ok(MotionStepResult { events, support: after.support, surface_normal: after.surface_normal });
    }
    // Crossed as the window ends: the body is at the exit, touching nothing
    // in its new chart yet.
    if let Some(record) = clusters.sweep.as_deref_mut() {
        *record = SweepSample { time: [end, end], ..SweepSample::at_rest(*clusters.kinematics, retained_down) };
    }
    Ok(MotionStepResult { events, support: SupportFact::Airborne, surface_normal: -ctx.frame.down() })
}

/// Whether the pair's map takes the frame's run axis to its opposite (and
/// keeps it along the run), the rule the seat's input warp uses.
fn mirrors_the_run(transfer: RootTransfer, frame: MotionFrame) -> bool {
    let side = frame.basis().side;
    transfer.map.vector(side).ok().map(Vec2::normalize_or_zero)
        .is_some_and(|mapped| mapped.dot(side) < -0.5 && mapped.dot(frame.down()).abs() < 0.5)
}

/// The first admitted link the root of `sweep` crosses, from its front.
/// Ties go to the link listed first.
fn first_root_crossing(
    admission: MotionAdmission<'_>, reroot: Reroot<'_>, sweep: &SweepSample, domain: &crate::CombatVolume,
) -> Result<Option<(RootCrossing, PortalLink)>, Refusal> {
    let cast = sweep.material_cast(domain)?;
    let mut first: Option<(RootCrossing, PortalLink)> = None;
    for link in admission.links {
        if reroot.closed.contains(&link.key) { continue; }
        let refusal = |reason| Refusal { path: Vec::new(), link: Some(link.key), reason };
        let Some(root) = cast.crossing(link.enter_step, link.enter, 1.0).map_err(refusal)? else { continue; };
        if first.is_some_and(|(hit, _)| hit.toi <= root.toi) { continue; }
        first = Some((root, *link));
    }
    Ok(first)
}
