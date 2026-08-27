//! How a puppy slug's crawl READS, measured — the adhesive crawler judged by
//! the shape of its trajectory rather than only by where it ends up.
//!
//! # The situations are the level's, not invented
//!
//! * 48 px side walls (3 cells) the full height, and a full-height 48 px pillar
//!   at x 464–512 — the shaft's two sources of CONCAVE corners where the floor
//!   meets a vertical face;
//! * 32 px thick ledges (2 cells) free at both ends — 12 CONVEX corners the crawl
//!   has to wrap;
//! * `MovingPlatform` 160 x 18, `speed: 130`, `sweep_dx: 240`;
//! * `BreakablePlatform` 112 x 24 with `collision: OneWayUp` — the one surface the
//!   crawler's two predicates DISAGREE about (`cling_pred` accepts it, `wall_pred`
//!   rejects it), which makes it the interesting case rather than a duplicate;
//! * a 48 x 22 slug body (the authored `EnemySpawn` size).
//!
//! Each scenario is stated as "a slug crawling <direction> from <here>", stepped
//! through the same [`step_motion`] entry production uses, and its per-tick
//! positions measured. Set `AMBITION_MOTION_REPORT=1` to print every scenario's
//! numbers instead of only a failure's.

use crate::body_clusters::BodyClusterScratch;
use crate::motion_quality::{measure_motion, MotionBudget, MotionQuality};
use crate::movement::{
    step_motion, switch_motion_model, CrawlerParams, InputState, MotionModelSpec, MotionStepContext,
};
use crate::world::{Block, BlockKind};
use crate::{Vec2, World};

use super::TEST_TUNING;

/// The authored `EnemySpawn` footprint of a puppy slug in the shaft.
const SLUG_SIZE: Vec2 = Vec2::new(48.0, 22.0);
/// The shaft's cell size, wall thickness, and ledge thickness.
const CELL: f32 = 16.0;
const WALL: f32 = 3.0 * CELL;
const LEDGE: f32 = 2.0 * CELL;
/// Enough ticks for the crawl (40 px/s) to cross a ledge and turn a corner.
const TICKS: usize = 240;
const DT: f32 = 1.0 / 60.0;

/// A shaft-shaped world: side walls and a floor, both 48 px, sized like the real
/// level. Scenario geometry is added on top.
fn shaft(extra: Vec<Block>) -> World {
    let size = Vec2::new(1008.0, 2400.0);
    let mut blocks = vec![
        Block::solid("left wall", Vec2::ZERO, Vec2::new(WALL, size.y)),
        Block::solid(
            "right wall",
            Vec2::new(size.x - WALL, 0.0),
            Vec2::new(WALL, size.y),
        ),
        Block::solid(
            "floor",
            Vec2::new(0.0, size.y - WALL),
            Vec2::new(size.x, WALL),
        ),
    ];
    blocks.extend(extra);
    World {
        name: "vertical_shaft (measured)".to_string(),
        size,
        spawn: Vec2::new(502.0, size.y - WALL - 24.0),
        blocks,
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
        chains: Vec::new(),
        edges: Default::default(),
    }
}

/// A 2-cell-thick ledge, free at both ends, like every platform in the shaft.
fn ledge(name: &str, left: f32, top: f32, width: f32) -> Block {
    Block::solid(name, Vec2::new(left, top), Vec2::new(width, LEDGE))
}

/// The shaft's full-height central pillar (x 464–512 in the real level).
fn pillar() -> Block {
    Block::solid(
        "central pillar",
        Vec2::new(464.0, 0.0),
        Vec2::new(WALL, 2400.0),
    )
}

/// Crawl a slug and return its per-tick positions.
///
/// `facing` is the crawl intent the brain would supply (`-1` left, `+1` right);
/// the crawler reads it as a tangential direction along whatever it is clung to,
/// so it keeps its meaning on a wall and on a ceiling.
///
/// `advance_world` runs before each tick so a scenario can move its geometry —
/// the moving-platform case. It receives the tick index and must set each moving
/// block's `velocity` to that tick's DISPLACEMENT (`Block::velocity` is a
/// per-frame delta, not px/s — see its doc).
fn crawl(
    world: &mut World,
    start: Vec2,
    facing: f32,
    ticks: usize,
    mut advance_world: impl FnMut(&mut World, usize),
) -> Vec<Vec2> {
    let mut scratch =
        BodyClusterScratch::new_with_abilities(start, crate::abilities::AbilitySet::default());
    scratch.kinematics.size = SLUG_SIZE;
    scratch.base_size.base_size = SLUG_SIZE;
    let (model, mut clusters) = scratch.parts();
    switch_motion_model(
        model,
        MotionModelSpec::AdhesiveCrawler(CrawlerParams::default()),
    );
    let mut track = Vec::with_capacity(ticks);
    for tick in 0..ticks {
        advance_world(world, tick);
        step_motion(
            model,
            &mut clusters,
            MotionStepContext {
                world,
                input: InputState {
                    axes: crate::reference_frame::LocalAxes::new(facing, 0.0),
                    ..Default::default()
                },
                frame: TEST_TUNING.frame(),
                facing_intent: facing,
                dt: DT,
                contact: crate::movement::body_contact::BodyContactField::NONE,
                pose_owned_externally: false,
            },
        );
        track.push(clusters.kinematics.pos);
    }
    track
}

/// Crawl in a world nothing moves — every scenario but the platform one.
fn crawl_static(world: &World, start: Vec2, facing: f32, ticks: usize) -> Vec<Vec2> {
    let mut world = world.clone();
    crawl(&mut world, start, facing, ticks, |_, _| {})
}

/// Report a scenario, and fail with its full numbers if it breaks the budget.
///
/// The message carries the whole measurement, not just the broken clause: when a
/// crawl misbehaves, the neighbouring figures (did it stall? did it reverse? did
/// it get anywhere?) are what distinguish "snagged on one corner" from
/// "oscillating between two surfaces", and re-running to collect them is exactly
/// the friction that stops a probe from being used.
#[track_caller]
fn check(label: &str, track: &[Vec2], budget: MotionBudget) -> MotionQuality {
    let quality = measure_motion(track);
    if std::env::var_os("AMBITION_MOTION_REPORT").is_some() {
        eprintln!("[motion] {label}: {}", quality.summary());
        // The ticks around the worst jerk are where a diagnosis actually happens
        // — the summary says how bad, this says what it did.
        let from = quality.max_jerk_at.saturating_sub(4);
        for (offset, pos) in track[from..(from + 9).min(track.len())].iter().enumerate() {
            eprintln!("[motion]   tick {:4}  {pos:?}", from + offset);
        }
    }
    let broken = budget.violations(&quality);
    assert!(
        broken.is_empty(),
        "{label} moves badly:\n  - {}\n  full measurement: {}\n  first ticks: {:?}",
        broken.join("\n  - "),
        quality.summary(),
        &track[..track.len().min(8)],
    );
    quality
}

/// Baseline. A slug crawling the middle of a ledge, touching nothing else.
/// Any jerk here is the crawl integrator's own, with no corner to blame.
#[test]
fn a_slug_crawling_a_flat_ledge_moves_perfectly_evenly() {
    let world = shaft(vec![ledge("ledge", 144.0, 944.0, 256.0)]);
    // Seated on top of the ledge: centre is half a body above its surface.
    let start = Vec2::new(300.0, 944.0 - SLUG_SIZE.y * 0.5);
    let quality = check(
        "flat ledge",
        &crawl_static(&world, start, -1.0, 120),
        MotionBudget::CRAWLING,
    );
    assert!(
        quality.max_jerk < 0.05,
        "a flat crawl is the calibration case — it should be nearly perfect: {}",
        quality.summary()
    );
}

/// A CONVEX 90° transit is discontinuous by construction, and this budget
/// admits it rather than hiding it.
///
/// The crawler's AABB does not rotate with its attachment, so a 48 x 22 body lying along a ledge's
/// top cannot also lie along the ledge's END: the two placements share no position.
///
/// Removing the pop needs a decision this test cannot make: orient the crawler's
/// collision box by its attachment (the sprite already rotates — `rotation_rad`),
/// give crawlers a square-ish body, or spread the transit over several ticks as
/// an animation. Logged in `dev/journals/code_smells.md`.
const WRAPPING_A_CORNER: MotionBudget = MotionBudget {
    max_jerk: 26.0,
    max_jerk_ratio: 32.0,
    // A body circumnavigating a ledge legitimately ends up near where it began,
    // so neither straightness nor reversals mean anything here. Jerk does.
    max_reversal_rate: 0.10,
    min_straightness: 0.0,
};

/// Convex corner. The slug crawls off a ledge's free end and wraps under it.
/// Twelve of these exist in the shaft; every ledge is free at both ends.
///
/// What this pins is that the wrap COMPLETES and costs exactly one pivot: no
/// stall, no oscillation between the two faces, and no second lurch once it is
/// on the new surface.
#[test]
fn a_slug_wrapping_a_ledge_end_pivots_once_and_keeps_going() {
    let world = shaft(vec![ledge("ledge", 144.0, 944.0, 256.0)]);
    // Start close enough to the left end that the wrap happens inside the window.
    let start = Vec2::new(210.0, 944.0 - SLUG_SIZE.y * 0.5);
    let track = crawl_static(&world, start, -1.0, TICKS);
    let quality = check("convex corner (ledge end)", &track, WRAPPING_A_CORNER);
    assert_eq!(
        quality.stalled_ticks,
        0,
        "the slug must never stop at the corner: {}",
        quality.summary()
    );
    // ONE pivot in the window, not a repeated pop: the mean jerk is what tells
    // these apart — a single 25 px event over 240 ticks averages ~0.1 px, while
    // a corner the crawl keeps re-entering averages near the pop itself.
    assert!(
        quality.mean_jerk < 1.0,
        "the corner must be transited once, not repeatedly re-entered: {}",
        quality.summary()
    );
}

/// Concave corner, side wall. The shaft's floor meets its 48 px side wall;
/// the slug at `px(272, 2336)` in the real level crawls straight into it.
#[test]
fn a_slug_turning_the_floor_into_the_side_wall_does_not_stick() {
    let world = shaft(Vec::new());
    let floor_top = 2400.0 - WALL;
    // Close enough that the turn lands inside the window. The real slug spawns at
    // x 272 and takes ~5 s to crawl here; starting mid-approach measures the same
    // motion without 300 idle ticks — and the assertion below is what keeps this
    // honest, because a window that never reaches the corner would otherwise read
    // as a perfect score.
    let start = Vec2::new(140.0, floor_top - SLUG_SIZE.y * 0.5);
    let track = crawl_static(&world, start, -1.0, TICKS);
    let quality = check(
        "concave corner (floor into side wall)",
        &track,
        MotionBudget::CRAWLING,
    );
    // It must actually TURN: the wall's inner face is at x = 48, so a slug that
    // stayed on the floor would still be at floor level after 240 ticks.
    let ended = track.last().copied().expect("a track");
    assert!(
        ended.y < floor_top - SLUG_SIZE.y,
        "the slug must be climbing the wall by now, not still on the floor \
         (ended at {ended:?}): {}",
        quality.summary()
    );
}

/// Concave corner, central pillar. The same turn against the shaft's
/// full-height interior pillar — a wall the slug meets from the OTHER side, so a
/// rule that only works against the world's edge fails here.
#[test]
fn a_slug_turning_the_floor_into_the_central_pillar_does_not_stick() {
    let world = shaft(vec![pillar()]);
    let floor_top = 2400.0 - WALL;
    let start = Vec2::new(360.0, floor_top - SLUG_SIZE.y * 0.5);
    check(
        "concave corner (floor into pillar)",
        &crawl_static(&world, start, 1.0, TICKS),
        MotionBudget::CRAWLING,
    );
}

/// One-way platform. `cling_pred` accepts a one-way surface and `wall_pred`
/// rejects it, deliberately — a crawler would never collide with a one-way
/// platform's side, so it must not read as a concave corner. This is the crawl
/// that proves the two predicates disagree in the intended direction.
#[test]
fn a_slug_crawling_a_one_way_platform_treats_it_as_ground_not_wall() {
    let mut world = shaft(vec![ledge("ledge", 144.0, 944.0, 256.0)]);
    world.blocks.push(Block {
        kind: BlockKind::OneWay,
        ..ledge("breakable one-way", 400.0, 944.0, 112.0)
    });
    // Start on the solid ledge and crawl RIGHT onto the abutting one-way slab:
    // the seam between them must not register as a wall.
    let start = Vec2::new(340.0, 944.0 - SLUG_SIZE.y * 0.5);
    check(
        "one-way platform seam",
        &crawl_static(&world, start, 1.0, TICKS),
        MotionBudget::CRAWLING,
    );
}

/// Moving platform. The shaft's two `MovingPlatform`s sweep 240 px at
/// 130 px/s. A crawler is GLUED to its surface, so it is carried by the platform's
/// FULL delta (both axes, unlike a gravity-resting body) — and that carry must not
/// read as a lurch on top of the crawl.
///
/// It does include the platform's turnaround, which is where a rider would be dropped or
/// double-counted.
#[test]
fn a_slug_riding_a_moving_platform_is_carried_smoothly() {
    const PLATFORM_TOP: f32 = 1792.0;
    const PLATFORM_SIZE: Vec2 = Vec2::new(160.0, 18.0);
    const PLATFORM_LEFT: f32 = 96.0;
    /// The authored `MovingPlatform` fields, verbatim.
    const PLATFORM_SPEED: f32 = 130.0;
    const PLATFORM_SWEEP: f32 = 240.0;
    /// Long enough to include the platform's turnaround
    /// (240 px / 130 px/s ≈ 111 ticks), short enough that the slug — crawling
    /// 0.67 px/tick across 112 px of free deck — never reaches an end.
    const RIDE_TICKS: usize = 150;

    let mut world = shaft(vec![Block::solid(
        "moving platform",
        Vec2::new(PLATFORM_LEFT, PLATFORM_TOP),
        PLATFORM_SIZE,
    )]);
    let platform = world.blocks.len() - 1;
    // Seated at the deck's right end, crawling inward.
    let start = Vec2::new(
        PLATFORM_LEFT + PLATFORM_SIZE.x - SLUG_SIZE.x * 0.5,
        PLATFORM_TOP - SLUG_SIZE.y * 0.5,
    );
    // The platform sweeps right then back, exactly as the authored feature does,
    // and publishes each tick's DISPLACEMENT as its velocity (`Block::velocity`
    // is a per-frame delta, not px/s).
    let mut travelled = 0.0f32;
    let mut direction = 1.0f32;
    let track = crawl(&mut world, start, -1.0, RIDE_TICKS, move |world, _| {
        let step = PLATFORM_SPEED * DT * direction;
        travelled += step;
        if travelled > PLATFORM_SWEEP {
            direction = -1.0;
        } else if travelled < 0.0 {
            direction = 1.0;
        }
        let block = &mut world.blocks[platform];
        block.aabb = crate::geometry::aabb_from_min_size(
            Vec2::new(PLATFORM_LEFT + travelled, PLATFORM_TOP),
            PLATFORM_SIZE,
        );
        block.velocity = Vec2::new(step, 0.0);
    });
    let quality = check(
        "riding a moving platform",
        &track,
        MotionBudget {
            // The PLATFORM's own turnaround is an instantaneous direction flip (`sweep_dx` reverses
            // in one tick), so its rider inherits a jerk of twice the platform's per-tick step.
            max_jerk: 2.0 * PLATFORM_SPEED * DT + 40.0 * DT + 0.01,
            max_jerk_ratio: 4.0,
            // One turnaround in the window, and the ride is a there-and-back — so
            // neither of these describes anything here.
            max_reversal_rate: 0.05,
            min_straightness: 0.0,
        },
    );
    assert_eq!(
        quality.reversals,
        1,
        "exactly the platform's one turnaround should reverse the rider — more \
         means the carry is fighting the crawl: {}",
        quality.summary()
    );
}
