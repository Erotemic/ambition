//! Deterministic cast-camera scenarios: 2, 3 and 4 bodies on a Smash-sized
//! stage, moved on scripted paths through the real resolver at 60 Hz, and
//! measured in screen-normalized terms.
//!
//! ```bash
//! cargo test -p ambition_sim_view --lib camera_scenarios -- --ignored --nocapture
//! ```
//!
//! The table is the instrument for camera tuning: it states what the frame
//! does to the same fight at each roster size, with a human seat or none.
//! The asserted tests below it hold the properties the tuning must keep.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::markers::{
    CastCameraBounds, CastFramingTuning, ControlledSubject, FramedCast,
};

use crate::camera_snapshot::{CameraObservationPlugin, ResolvedCameraSnapshot};

/// The tick length every scenario steps at.
const DT: f32 = 1.0 / 60.0;
/// The stage's floor: a standing body's centre is half its height above it.
const FLOOR_Y: f32 = 300.0;
/// A polygon fighter's body box.
const BODY: ae::Vec2 = ae::Vec2::new(40.0, 69.0);
/// A standing body's centre height.
const STAND_Y: f32 = FLOOR_Y - BODY.y * 0.5;

/// Smash's authored cast framing, as `match_rules.ron` states it.
pub(crate) fn smash_framing() -> CastFramingTuning {
    CastFramingTuning {
        min_view_scale: 0.45,
        margin: 40.0,
        floor_margin: 40.0,
        open_hz: Some(4.0),
        close_hz: 4.0,
        bounds: Some(CastCameraBounds { side: 280.0, rise: 170.0, fall: 100.0 }),
        bubble: None,
    }
}

fn smash_app(framing: CastFramingTuning) -> App {
    let mut app = App::new();
    app.add_plugins(bevy::time::TimePlugin);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(DT),
    ));
    app.init_resource::<ambition_dev_tools::dev_tools::DeveloperTools>();
    app.init_resource::<ambition_persistence::settings::UserSettings>();
    app.init_resource::<ambition_platformer2d_shared_tangle::camera_ease::CameraEaseTuning>();
    app.init_resource::<ControlledSubject>();
    app.init_resource::<FramedCast>();
    app.insert_resource(framing);
    app.add_plugins(CameraObservationPlugin);
    let world = ae::World::new("stage", ae::Vec2::new(640.0, 480.0), ae::Vec2::ZERO, Vec::new());
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        ae::RoomGeometry(world.clone()),
    );
    ambition_platformer2d_world::rooms::insert_room_set(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts_or_panic(
            "stage",
            vec![ambition_platformer2d_world::rooms::RoomSpec::new("stage", world)],
            Vec::new(),
        ),
    );
    // One update so the clock starts; the first scripted tick follows.
    app.update();
    app
}

/// A scripted fight: where each body is at time `t`.
pub(crate) struct Scenario {
    pub name: &'static str,
    pub bodies: usize,
    pub seconds: f32,
    pub path: fn(usize, f32) -> ae::Vec2,
    /// From this time on, the measure of how long the frame took to settle.
    pub settle_from: Option<f32>,
}

/// What one run of a scenario measured. Lengths are fractions of the view
/// (width for x, height for y), so a wide and a tight frame compare.
#[derive(Debug, Default, Clone)]
pub(crate) struct Measured {
    pub view_w_end: f32,
    /// Median body height / view height.
    pub occupancy: f32,
    /// Median distance from the view's centre to the centre of the bodies'
    /// box, in view widths.
    pub centre_error: f32,
    /// Seconds from `settle_from` until the centre is within 2% of a view
    /// and the width within 2% of its final value, for good.
    pub settle_s: Option<f32>,
    /// Body-ticks with any part of a body outside the view while the body is
    /// inside the camera box (a fault), and while it is past the box (by
    /// design: the camera stops at the box and the body leaves the frame).
    pub cutoffs: usize,
    pub past_box: usize,
    /// The largest one-tick change of the centre (view widths) and of the
    /// width (fraction of the width).
    pub max_centre_step: f32,
    pub max_width_step: f32,
    /// Reversals of the zoom's direction (oscillation), counting only
    /// changes faster than 1% of the width per second.
    pub zoom_reversals: usize,
}

fn median(mut values: Vec<f32>) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f32::total_cmp);
    values[values.len() / 2]
}

/// Run `scenario` with a fresh camera; `human` seats a controlled body in 0.
pub(crate) fn run(scenario: &Scenario, framing: CastFramingTuning, human: bool) -> Measured {
    let mut app = smash_app(framing.clone());
    let mut last: Vec<ae::Vec2> = (0..scenario.bodies).map(|i| (scenario.path)(i, 0.0)).collect();
    let bodies: Vec<Entity> = last
        .iter()
        .map(|at| {
            let mut kin = ae::BodyKinematics::default();
            kin.pos = *at;
            kin.size = BODY;
            app.world_mut().spawn(kin).id()
        })
        .collect();
    app.world_mut().resource_mut::<FramedCast>().0 = bodies.clone();
    if human {
        app.world_mut().resource_mut::<ControlledSubject>().0 = Some(bodies[0]);
    }
    let ticks = (scenario.seconds / DT).round() as usize;
    let mut rows: Vec<(f32, ae::Vec2, ae::Vec2, ae::Vec2, Vec<ae::Vec2>)> = Vec::new();
    for tick in 1..=ticks {
        let t = tick as f32 * DT;
        for (i, body) in bodies.iter().enumerate() {
            let at = (scenario.path)(i, t);
            let mut kin = app.world_mut().get_mut::<ae::BodyKinematics>(*body).expect("a body");
            kin.vel = (at - last[i]) / DT;
            kin.pos = at;
            last[i] = at;
        }
        app.update();
        let frame = app
            .world_mut()
            .query::<&ResolvedCameraSnapshot>()
            .iter(app.world())
            .find_map(|resolved| resolved.frame().cloned())
            .expect("a cast resolves a frame");
        let (lo, hi) = last.iter().fold((ae::Vec2::splat(f32::MAX), ae::Vec2::splat(f32::MIN)), |(lo, hi), at| {
            (lo.min(*at - BODY * 0.5), hi.max(*at + BODY * 0.5))
        });
        rows.push((t, frame.snapshot.center_world, frame.snapshot.visible_view, (lo + hi) * 0.5, last.clone()));
    }
    let mut measured = Measured::default();
    let (_, end_centre, end_view, ..) = rows.last().cloned().expect("ticks");
    measured.view_w_end = end_view.x;
    measured.occupancy = median(rows.iter().map(|(.., view, _, _)| BODY.y / view.y).collect());
    measured.centre_error = median(rows.iter().map(|(_, c, v, ideal, _)| (*c - *ideal).length() / v.x).collect());
    let bounds = framing.bounds.expect("Smash states a camera box");
    let in_box = |at: ae::Vec2| {
        at.x - BODY.x * 0.5 >= -bounds.side
            && at.x + BODY.x * 0.5 <= 640.0 + bounds.side
            && at.y - BODY.y * 0.5 >= -bounds.rise
            && at.y + BODY.y * 0.5 <= 480.0 + bounds.fall
    };
    for (_, centre, view, _, bodies) in &rows {
        let half = *view * 0.5;
        for at in bodies {
            let d = (*at - *centre).abs() + BODY * 0.5;
            if d.x > half.x + 0.5 || d.y > half.y + 0.5 {
                if in_box(*at) {
                    measured.cutoffs += 1;
                } else {
                    measured.past_box += 1;
                }
            }
        }
    }
    let mut last_rate = 0.0f32;
    for pair in rows.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        measured.max_centre_step = measured.max_centre_step.max((b.1 - a.1).length() / a.2.x);
        let width_rate = (b.2.x - a.2.x) / a.2.x / DT;
        measured.max_width_step = measured.max_width_step.max(((b.2.x - a.2.x) / a.2.x).abs());
        if width_rate.abs() > 0.01 {
            if last_rate != 0.0 && width_rate.signum() != last_rate.signum() {
                measured.zoom_reversals += 1;
            }
            last_rate = width_rate;
        }
    }
    if let Some(from) = scenario.settle_from {
        let settled = |row: &(f32, ae::Vec2, ae::Vec2, ae::Vec2, Vec<ae::Vec2>)| {
            (row.1 - end_centre).length() / row.2.x < 0.02 && ((row.2.x - end_view.x) / end_view.x).abs() < 0.02
        };
        let last_unsettled = rows.iter().rposition(|row| row.0 >= from && !settled(row));
        measured.settle_s = Some(match last_unsettled {
            Some(index) => rows.get(index + 1).map_or(scenario.seconds, |row| row.0) - from,
            None => 0.0,
        });
    }
    measured
}

/// Bodies spread evenly over `[left, right]`, standing.
fn spread(i: usize, n: usize, left: f32, right: f32) -> ae::Vec2 {
    let x = if n <= 1 { (left + right) * 0.5 } else { left + (right - left) * i as f32 / (n - 1) as f32 };
    ae::Vec2::new(x, STAND_Y)
}

macro_rules! by_count {
    ($name:ident, $n:expr, $body:expr) => {
        fn $name(i: usize, t: f32) -> ae::Vec2 {
            let f: fn(usize, usize, f32) -> ae::Vec2 = $body;
            f(i, $n, t)
        }
    };
}

// Standing together near the stage's middle.
by_count!(cluster2, 2, |i, n, _| spread(i, n, 280.0, 360.0));
by_count!(cluster3, 3, |i, n, _| spread(i, n, 270.0, 370.0));
by_count!(cluster4, 4, |i, n, _| spread(i, n, 260.0, 380.0));
// The same outer pair at every roster size: what the count alone does.
by_count!(wide2, 2, |i, n, _| spread(i, n, 120.0, 520.0));
by_count!(wide3, 3, |i, n, _| spread(i, n, 120.0, 520.0));
by_count!(wide4, 4, |i, n, _| spread(i, n, 120.0, 520.0));
// Everybody runs right at 300 u/s for 2/3 s, then stops.
fn travel(i: usize, n: usize, t: f32) -> ae::Vec2 {
    spread(i, n, 140.0, 300.0) + ae::Vec2::new(300.0 * t.min(2.0 / 3.0), 0.0)
}
by_count!(travel2, 2, travel);
by_count!(travel4, 4, travel);
// Body 0 is launched up and out (1000 u/s up, 220 u/s out, Smash gravity);
// the rest stand.
fn launch(i: usize, n: usize, t: f32) -> ae::Vec2 {
    let at = spread(i, n, 240.0, 400.0);
    if i != 0 {
        return at;
    }
    let y = (at.y - 1000.0 * t + 0.5 * 1150.0 * t * t).min(STAND_Y);
    ae::Vec2::new(at.x - 220.0 * t, y)
}
by_count!(launch2, 2, launch);
by_count!(launch4, 4, launch);
// Four bodies close from the stage's width to its middle in half a second.
fn converge(i: usize, n: usize, t: f32) -> ae::Vec2 {
    let k = (t / 0.5).min(1.0);
    spread(i, n, 120.0, 520.0).lerp(spread(i, n, 280.0, 360.0), k)
}
by_count!(converge4, 4, converge);
// Body 1 flies off the right side past the camera box, then is put back on
// the respawn point above the stage at 1.0 s.
fn knockout(i: usize, n: usize, t: f32) -> ae::Vec2 {
    let at = spread(i, n, 240.0, 400.0);
    if i != 1 {
        return at;
    }
    if t < 1.0 {
        at + ae::Vec2::new(900.0 * t, -300.0 * t)
    } else {
        ae::Vec2::new(320.0, 140.0)
    }
}
by_count!(knockout2, 2, knockout);
by_count!(knockout4, 4, knockout);

pub(crate) fn scenarios() -> Vec<Scenario> {
    let s = |name, bodies, seconds, path, settle_from| Scenario { name, bodies, seconds, path, settle_from };
    vec![
        s("cluster", 2, 2.0, cluster2 as fn(usize, f32) -> ae::Vec2, Some(0.0)),
        s("cluster", 3, 2.0, cluster3, Some(0.0)),
        s("cluster", 4, 2.0, cluster4, Some(0.0)),
        s("wide", 2, 2.0, wide2, Some(0.0)),
        s("wide", 3, 2.0, wide3, Some(0.0)),
        s("wide", 4, 2.0, wide4, Some(0.0)),
        s("travel", 2, 2.5, travel2, Some(2.0 / 3.0)),
        s("travel", 4, 2.5, travel4, Some(2.0 / 3.0)),
        s("launch", 2, 2.5, launch2, None),
        s("launch", 4, 2.5, launch4, None),
        s("converge", 4, 2.5, converge4, Some(0.5)),
        s("knockout", 2, 2.5, knockout2, Some(1.0)),
        s("knockout", 4, 2.5, knockout4, Some(1.0)),
    ]
}

#[test]
#[ignore = "an instrument: prints the scenario table"]
fn cast_camera_scenario_table() {
    println!(
        "{:<9} {:>2} {:>5} | {:>6} {:>6} {:>8} {:>6} {:>7} {:>5} {:>7} {:>4}",
        "scenario", "n", "seat", "view_w", "occup", "ctr_err", "settle", "cutoff", "past", "dctr", "rev"
    );
    for scenario in scenarios() {
        for human in [false, true] {
            if human && !matches!(scenario.name, "travel" | "launch") {
                continue;
            }
            let m = run(&scenario, smash_framing(), human);
            println!(
                "{:<9} {:>2} {:>5} | {:>6.0} {:>6.3} {:>8.3} {:>6} {:>7} {:>5} {:>7.3} {:>4}",
                scenario.name,
                scenario.bodies,
                if human { "human" } else { "cpu" },
                m.view_w_end,
                m.occupancy,
                m.centre_error,
                m.settle_s.map_or("-".to_string(), |s| format!("{s:.2}")),
                m.cutoffs,
                m.past_box,
                m.max_centre_step,
                m.zoom_reversals,
            );
        }
    }
}
