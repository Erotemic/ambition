use super::*;
use ae::movement::{switch_motion_model, MotionModelSpec, DEFAULT_TUNING, GRAVITY};
use ae::AbilitySet;

const DT: f32 = 1.0 / 60.0;

fn normal_frame() -> MotionFrame {
    MotionFrame::from_acceleration(Vec2::new(0.0, GRAVITY)).expect("a non-zero gravity")
}

/// A walking, jumping body with no air jump, no dash and no wall verb: the
/// verbs the envelope models.
fn walker() -> BodyClusterScratch {
    let abilities = AbilitySet {
        move_horizontal: true,
        jump: true,
        ..AbilitySet::NONE
    };
    BodyClusterScratch::new_with_abilities(Vec2::ZERO, abilities)
}

/// The same body with a slower run and a lower jump, so the guard is not a
/// fact about one tuning.
fn slow_walker() -> BodyClusterScratch {
    let mut body = walker();
    let tuning = ae::movement::MovementTuning {
        max_run_speed: DEFAULT_TUNING.max_run_speed * 0.6,
        max_air_speed: DEFAULT_TUNING.max_air_speed * 0.6,
        jump_speed: DEFAULT_TUNING.jump_speed * 0.75,
        ..DEFAULT_TUNING
    };
    switch_motion_model(&mut body.model, MotionModelSpec::AxisSwept(tuning.axis_swept_params()));
    body
}

#[derive(Debug, PartialEq, Eq)]
enum Crossing {
    /// The body stands on the far surface.
    Landed,
    /// The body fell past the far surface's top, or stood somewhere else.
    Missed,
}

/// Run `body` at full throttle across a gap in the real kernel, as a commanded
/// body crosses it: the jump is pressed on the last grounded step before the
/// edge (when `jump`), then held. The near floor ends at x = 4000, its top at
/// y = 2000; the far floor starts `gap` later, its top `rise` higher.
fn cross(body: &BodyClusterScratch, gap: f32, rise: f32, jump: bool) -> Crossing {
    let edge = 4000.0;
    let top = 2000.0;
    let far_top = top - rise;
    let world = World::new(
        "gap",
        Vec2::new(12000.0, 6000.0),
        Vec2::ZERO,
        vec![
            ae::Block::solid("near", Vec2::new(0.0, top), Vec2::new(edge, 64.0)),
            ae::Block::solid("far", Vec2::new(edge + gap, far_top), Vec2::new(3000.0, 64.0)),
        ],
    );
    let mut body = body.clone();
    let half = body.kinematics.size * 0.5;
    {
        let (model, mut clusters) = body.parts();
        ae::movement::transit_body(
            model,
            &mut clusters,
            Vec2::new(edge - 900.0, top - half.y),
            ae::movement::TransitVelocity::Zero,
        );
    }
    let mut jumped = false;
    for _ in 0..1200 {
        let lead = body.kinematics.pos.x + half.x;
        let next_lead = lead + body.kinematics.vel.x.max(0.0) * DT;
        let press = jump && !jumped && body.ground.on_ground && lead < edge && next_lead >= edge;
        jumped |= press;
        step(&mut body, &world, normal_frame(), DT, run_input(press, jumped));
        let feet = body.kinematics.pos.y + half.y;
        let trailing = body.kinematics.pos.x - half.x;
        if body.ground.on_ground && trailing > edge && (feet - far_top).abs() < 1.0 {
            return Crossing::Landed;
        }
        if feet > far_top + 200.0 {
            return Crossing::Missed;
        }
    }
    Crossing::Missed
}

/// The widest gap the envelope says a rollout reaches at `rise`, to 0.25 px.
///
/// A higher surface is not reached at gap 0: the body would meet its side
/// while still rising. So the reachable gaps are an interval, and this is its
/// far end.
fn widest(envelope: &TraversalEnvelope, rise: f32, jump: bool) -> Option<f32> {
    let reaches = |gap: f32| {
        if jump {
            envelope.reaches(gap, rise)
        } else {
            envelope.drop_reaches(gap, rise)
        }
    };
    (0..16000).rev().map(|quarter| quarter as f32 * 0.25).find(|gap| reaches(*gap))
}

/// The honest margin: the envelope takes off exactly at the edge, and a
/// commanded body takes off on its last grounded step, up to one step of travel
/// short of it.
fn margin(envelope: &TraversalEnvelope) -> f32 {
    envelope.takeoff_speed * DT + 2.0
}

/// ⭐ THE GUARD. For each body and each height, a gap just inside the
/// envelope is crossed in the real kernel and a gap just outside it is not.
#[test]
fn a_gap_inside_the_envelope_is_crossed_and_one_outside_is_not() {
    let mut wrong = Vec::new();
    let mut measured = 0;
    for (name, body) in [("walker", walker()), ("slow walker", slow_walker())] {
        let envelope = TraversalEnvelope::measure(&body, normal_frame(), EnvelopeProbe::default())
            .expect("the probe measures an upright body");
        let m = margin(&envelope);
        let apex = envelope.apex_rise();
        for (jump, rises) in [(true, [0.0, apex * 0.5, apex - 8.0, -96.0, -400.0]), (false, [-24.0, -96.0, -400.0, -800.0, -1200.0])] {
            for rise in rises {
                let Some(gap) = widest(&envelope, rise, jump) else {
                    wrong.push(format!("{name}: jump={jump} rise={rise}: the envelope reaches no gap at all"));
                    continue;
                };
                measured += 1;
                let inside = cross(&body, (gap - m).max(0.0), rise, jump);
                let outside = cross(&body, gap + m, rise, jump);
                if inside != Crossing::Landed || outside != Crossing::Missed {
                    wrong.push(format!(
                        "{name}: jump={jump} rise={rise:.1}: widest gap {gap:.2}, margin {m:.2}: inside {inside:?}, outside {outside:?}"
                    ));
                }
            }
        }
    }
    assert_eq!(measured, 20, "a case reached no gap: {wrong:#?}");
    assert!(wrong.is_empty(), "the envelope disagrees with the kernel:\n{}", wrong.join("\n"));
}

/// The envelope follows the body's tuning, not a constant: a slower, lower
/// body reaches less far and less high.
#[test]
fn a_slower_body_has_a_smaller_envelope() {
    let fast = TraversalEnvelope::measure(&walker(), normal_frame(), EnvelopeProbe::default()).expect("measured");
    let slow = TraversalEnvelope::measure(&slow_walker(), normal_frame(), EnvelopeProbe::default()).expect("measured");
    assert!(slow.takeoff_speed < fast.takeoff_speed, "{} vs {}", slow.takeoff_speed, fast.takeoff_speed);
    assert!(slow.apex_rise() < fast.apex_rise(), "{} vs {}", slow.apex_rise(), fast.apex_rise());
    let reach = |envelope: &TraversalEnvelope| widest(envelope, 0.0, true).expect("a level gap");
    assert!(reach(&slow) < reach(&fast), "{} vs {}", reach(&slow), reach(&fast));
}

/// A rise above the apex is never reached, at any gap; and the measurement is
/// the same each time.
#[test]
fn above_the_apex_is_out_of_reach_and_the_envelope_is_deterministic() {
    let envelope = TraversalEnvelope::measure(&walker(), normal_frame(), EnvelopeProbe::default()).expect("measured");
    let again = TraversalEnvelope::measure(&walker(), normal_frame(), EnvelopeProbe::default()).expect("measured");
    assert_eq!(envelope, again);
    let above = envelope.apex_rise() + 1.0;
    assert!(!envelope.reaches(0.0, above) && !envelope.reaches(40.0, above));
    assert!(envelope.landing_lead(0.0).is_some_and(|lead| lead > 0.0));
}
