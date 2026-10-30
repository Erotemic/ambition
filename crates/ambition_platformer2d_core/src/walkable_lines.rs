//! A body that walks stands on a slope: the room's surface lines are ground
//! for the ordinary (axis-swept) body too, as one-way floors.
//!
//! A [`SurfaceChain`](crate::world::SurfaceChain) is a line of any angle. The
//! surface-momentum rider has always ridden it; this is how a WALKER stands
//! on one (`docs/planning/engine/slopes-feet-ride-surfaces.md`, phase B):
//!
//! - A segment whose outward normal is within [`WALK_SLOPE_LIMIT_DEG`] of the
//!   body's up is ground for it. A steeper one is not (it is passed, as a
//!   one-way floor is passed from the side).
//! - The FOOT SPAN is the width of the box: the body stands on the highest
//!   point of the line under the bottom of its box, so no part of the box is
//!   in the line, and a flat box top beside it is the same rule.
//! - A line is one-way. A body lands on it from above and passes it from
//!   below and from the side; a drop through passes it from above.
//! - A body that stood and did not jump is held on the line where the line
//!   falls away under it within its slope limit, so it walks down a ramp, a
//!   stair or over a crest with no tick in the air and no landing. Walking
//!   up, it is put on the line where the line rises under it within the same
//!   limit. Its speed across the floor is its speed on the flat.
//! - Where lines fork (a stair off a street, the landing of a switchback),
//!   a body keeps to the line it stands on while that line goes on. It steps
//!   from other ground onto a line ahead of it, never onto one behind it, and
//!   onto a line whose [`Boarding`] is `Up` only while it holds up.
//!
//! It is stateless: what a body stands on is found again each tick from where
//! it is, as for a box top. Nothing here is rollback state.

use crate::collision_semantics::ONE_WAY_CROSSING_SLOP;
use crate::world::{BlockKind, Boarding, World};
use crate::{Aabb, AabbExt, Vec2};

/// The steepest line a walker stands on, from the body's floor.
pub const WALK_SLOPE_LIMIT_DEG: f32 = 50.0;

/// How far up the stick is held to step onto a line whose boarding is `Up`.
pub const BOARDING_UP: f32 = 0.5;

/// How much of a line must be under the foot span for the body to stand on
/// it (the box-top rule's `EDGE_OVERLAP_SLOP`).
const SPAN_OVERLAP_SLOP: f32 = crate::collision_semantics::EDGE_OVERLAP_SLOP;

/// The line a body stands on, under its foot span.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineSupport {
    /// The chain (an index into [`World::chains`]) and its segment.
    pub chain: usize,
    pub segment: usize,
    /// Where the feet go: the line's highest point under the span, as a
    /// coordinate along gravity (`p · down`; smaller is higher).
    pub feet_coord: f32,
    /// The point under the span the feet rest on.
    pub point: Vec2,
    /// The segment's outward normal.
    pub normal: Vec2,
    /// The two ends of the part of the segment under the span.
    pub ends: [Vec2; 2],
    /// The chain's own per-frame motion.
    pub velocity: Vec2,
}

/// The axis across the floor for gravity `down`.
fn across(down: Vec2) -> Vec2 {
    Vec2::new(-down.y, down.x)
}

/// Each walkable line under `body`'s foot span whose highest point there is
/// within `[feet - above, feet + below]` of the body's feet coordinate: one
/// for each chain, at its highest point under the span.
pub fn lines_under_feet(world: &World, body: Aabb, down: Vec2, above: f32, below: f32) -> Vec<LineSupport> {
    lines_under(world, body, down, above, below, false)
}

/// [`lines_under_feet`], each line measured against `body` moved by the
/// displacement that line made this tick: which line the body stood on as
/// the tick began, when the lines that move have already moved.
fn lines_under(world: &World, body: Aabb, down: Vec2, above: f32, below: f32, with_motion: bool) -> Vec<LineSupport> {
    let mut found: Vec<LineSupport> = Vec::new();
    if world.chains.is_empty() || down == Vec2::ZERO {
        return found;
    }
    let side = across(down);
    let up = -down;
    let min_up = WALK_SLOPE_LIMIT_DEG.to_radians().cos();
    let half = body.half_size();
    // The half width of the box across the floor.
    let half_span = half.x * side.x.abs() + half.y * side.y.abs();
    for (ci, chain) in world.chains.iter().enumerate() {
        // Where the body would be had it moved with this line: the motion of
        // the line at its feet.
        let centre = if with_motion && chain.moves() {
            let feet_point = body.center() + down * (body.feet_coord(down) - body.center().dot(down));
            body.center() + (chain.carried(feet_point) - feet_point)
        } else {
            body.center()
        };
        let (lo, hi) = (centre.dot(side) - half_span, centre.dot(side) + half_span);
        let feet = Aabb::new(centre, half).feet_coord(down);
        let mut best: Option<LineSupport> = None;
        for si in 0..chain.segment_count() {
            let (a, b) = chain.segment(si);
            let t = b - a;
            let len = t.length();
            if len <= 1.0e-4 {
                continue;
            }
            let normal = Vec2::new(t.y, -t.x) / len;
            if normal.dot(up) < min_up {
                continue;
            }
            let (sa, sb) = (a.dot(side), b.dot(side));
            let (u0, u1) = (sa.min(sb).max(lo), sa.max(sb).min(hi));
            if u1 - u0 <= SPAN_OVERLAP_SLOP {
                continue;
            }
            // The line's feet coordinate at each end of the overlap; the
            // highest point is at one of them.
            let at = |u: f32| {
                let k = if (sb - sa).abs() > 1.0e-6 { (u - sa) / (sb - sa) } else { 0.0 };
                a + t * k
            };
            let (p0, p1) = (at(u0), at(u1));
            let point = if p0.dot(down) <= p1.dot(down) { p0 } else { p1 };
            let coord = point.dot(down);
            // A line that turned moved its points across the body's width by
            // different amounts: the body, carried by the motion at the middle
            // of its feet, is off its support by up to that much.
            let turned = if with_motion { chain.turn.abs() * half_span * 2.0 } else { 0.0 };
            if coord < feet - above - turned || coord > feet + below + turned {
                continue;
            }
            if best.is_none_or(|known| coord < known.feet_coord) {
                best = Some(LineSupport {
                    chain: ci,
                    segment: si,
                    feet_coord: coord,
                    point,
                    normal,
                    ends: [p0, p1],
                    velocity: chain.velocity,
                });
            }
        }
        found.extend(best);
    }
    found
}

/// The highest walkable line under `body`'s foot span within
/// `[feet - above, feet + below]` (see [`lines_under_feet`]).
pub fn line_under_feet(world: &World, body: Aabb, down: Vec2, above: f32, below: f32) -> Option<LineSupport> {
    lines_under_feet(world, body, down, above, below)
        .into_iter()
        .min_by(|a, b| a.feet_coord.total_cmp(&b.feet_coord))
}

/// Is `body` standing on a walkable line where it is: its feet on the line,
/// or over it within the contact slop? A line above the feet (the foot of a
/// ramp the body stands beside) is not under them.
pub fn standing_on_line(world: &World, body: Aabb, down: Vec2) -> Option<LineSupport> {
    line_under_feet(world, body, down, ON_LINE_EPSILON, crate::collision_semantics::CONTACT_SLOP)
}

/// [`standing_on_line`] as a tick begins: a line that moved this tick (a
/// ridden mount's slope) is where the body stood on it once the body is
/// moved with it.
pub fn stood_on_line(world: &World, body: Aabb, down: Vec2) -> Option<LineSupport> {
    lines_under(world, body, down, ON_LINE_EPSILON, crate::collision_semantics::CONTACT_SLOP, true)
        .into_iter()
        .min_by(|a, b| a.feet_coord.total_cmp(&b.feet_coord))
}

/// The extent across the floor (coordinates along `side`) of each walkable
/// segment `body` stands on: a segment under its foot span whose highest
/// point there is at its feet, as [`standing_on_line`] finds them. The whole
/// segment, not only the part under the box: the support of a body goes on
/// past its box (`collision_semantics::footing`).
pub fn spans_underfoot(world: &World, body: Aabb, down: Vec2, side: Vec2) -> Vec<(f32, f32)> {
    let mut spans = Vec::new();
    if world.chains.is_empty() || down == Vec2::ZERO {
        return spans;
    }
    let up = -down;
    let min_up = WALK_SLOPE_LIMIT_DEG.to_radians().cos();
    let half = body.half_size();
    let half_span = half.x * side.x.abs() + half.y * side.y.abs();
    let centre = body.center().dot(side);
    let (lo, hi) = (centre - half_span, centre + half_span);
    let feet = body.feet_coord(down);
    for chain in world.chains.iter() {
        for si in 0..chain.segment_count() {
            let (a, b) = chain.segment(si);
            let t = b - a;
            let len = t.length();
            if len <= 1.0e-4 || Vec2::new(t.y, -t.x).dot(up) / len < min_up {
                continue;
            }
            let (sa, sb) = (a.dot(side), b.dot(side));
            let (u0, u1) = (sa.min(sb).max(lo), sa.max(sb).min(hi));
            if u1 - u0 <= SPAN_OVERLAP_SLOP {
                continue;
            }
            let at = |u: f32| {
                let k = if (sb - sa).abs() > 1.0e-6 { (u - sa) / (sb - sa) } else { 0.0 };
                a + t * k
            };
            let highest = at(u0).dot(down).min(at(u1).dot(down));
            if highest < feet - ON_LINE_EPSILON || highest > feet + crate::collision_semantics::CONTACT_SLOP {
                continue;
            }
            spans.push((sa.min(sb), sa.max(sb)));
        }
    }
    spans
}

/// How far a line may be above the feet of a body that stands on it: the
/// feet are put exactly on the line, so this is rounding.
const ON_LINE_EPSILON: f32 = 0.05;

/// How far a line may rise or fall under a body that moves `travel` across
/// the floor and still be walked: the slope limit's rise over that travel,
/// and the one-way surface's crossing slop.
pub fn walk_allowance(travel: f32) -> f32 {
    ONE_WAY_CROSSING_SLOP + travel.abs() * WALK_SLOPE_LIMIT_DEG.to_radians().tan()
}

/// What a step did on a line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineStep {
    /// The body came down onto the line, or walked up onto it.
    Landed(LineSupport),
    /// The body stood on the line and is still on it (it walked down it, or
    /// over a crest).
    Held(LineSupport),
    /// The body walked off the end of the line it stood on onto the top of
    /// this block (an index into [`World::blocks`]) a little below.
    OntoBlock(usize),
}

/// What a walking body did in the step that is about to be settled.
#[derive(Clone, Copy, Debug)]
pub struct WalkStep {
    /// Its velocity after the sweep.
    pub velocity: Vec2,
    /// Its feet coordinate before the gravity sweep.
    pub prev_feet: f32,
    /// How it moved before the gravity sweep (across the floor).
    pub moved: Vec2,
    /// It stood at the start of the step and did not leave the ground (a
    /// jump clears that before the sweep).
    pub was_grounded: bool,
    /// The line it stood on at the start of the step, if it stood on one.
    pub entry_line: Option<usize>,
    /// The sweep put it on a box top: a line then counts only if higher.
    pub supported_by_block: bool,
    /// It holds up: it steps onto a line whose boarding is `Up`.
    pub up_held: bool,
    pub drop_through: bool,
}

/// Put a body that has just been swept onto the walkable line under it, if
/// it reached one this step. Returns the delta to add to its position, and
/// what happened. A line that would put the box into a solid block is
/// refused.
pub fn settle_on_line(world: &World, body: Aabb, down: Vec2, step: WalkStep) -> Option<(Vec2, LineStep)> {
    if step.drop_through || world.chains.is_empty() {
        return None;
    }
    // Rising: a line is passed from below.
    if step.velocity.dot(down) < 0.0 {
        return None;
    }
    let feet = body.feet_coord(down);
    let side = across(down);
    let travel = step.moved.dot(side);
    let allowance = walk_allowance(travel);
    // Above the feet: the line rose under a body that walked up it, or the
    // body fell through it this step. Either way the feet were not under the
    // line by more than the allowance at the start of the step.
    let above = (feet - step.prev_feet).max(0.0) + allowance;
    // Below the feet: only a body that stood is held down onto it.
    let held = step.was_grounded && !step.supported_by_block;
    let below = if held { allowance } else { 0.0 };
    let candidates = lines_under_feet(world, body, down, above, below);
    let highest = |lines: &mut dyn Iterator<Item = LineSupport>| lines.min_by(|a, b| a.feet_coord.total_cmp(&b.feet_coord));
    // A line ahead: under the span it does not fall toward the way the body
    // goes (a ramp rising ahead, or flat), so the body walks onto it rather
    // than under a line that comes down behind it.
    let ahead = |line: &LineSupport| {
        let [a, b] = line.ends;
        let (lead, trail) = if (b - a).dot(side) * travel >= 0.0 { (b, a) } else { (a, b) };
        travel != 0.0 && lead.dot(down) <= trail.dot(down) + 1.0e-3
    };
    let boards = |line: &LineSupport| match world.chains[line.chain].boarding {
        Boarding::Auto => true,
        Boarding::Up => step.up_held,
    };
    let kept = step
        .entry_line
        .filter(|_| step.was_grounded)
        .and_then(|chain| candidates.iter().copied().find(|line| line.chain == chain));
    // The ground it stands on goes on under it: where another line meets it,
    // that is a fork, and the other line is taken only if it is boarded.
    let continues = step.supported_by_block || kept.is_some();
    let chosen = if !step.was_grounded {
        // A fall lands on the first line it crossed.
        highest(&mut candidates.iter().copied())
    } else {
        // Another line it may step onto: up onto one only ahead of it; at a
        // fork, only one it boards.
        let other = highest(
            &mut candidates
                .iter()
                .copied()
                .filter(|line| Some(line.chain) != kept.map(|kept| kept.chain))
                .filter(|line| line.feet_coord >= feet || ahead(line))
                .filter(|line| !continues || boards(line)),
        );
        match (kept, other) {
            // It keeps to its line, unless it holds up toward a higher one.
            (Some(kept), Some(other)) if step.up_held && other.feet_coord < kept.feet_coord => Some(other),
            (Some(kept), _) => Some(kept),
            (None, other) => other,
        }
    };
    let Some(line) = chosen else {
        return off_the_line(world, body, down, step, allowance);
    };
    if line.feet_coord > feet && !held {
        return None;
    }
    if step.supported_by_block && line.feet_coord >= feet {
        return None;
    }
    let delta = down * (line.feet_coord - feet);
    let moved = Aabb::new(body.center() + delta, body.half_size());
    let into_solid = world.blocks.iter().any(|block| {
        matches!(block.kind, BlockKind::Solid) && moved.strict_intersects(block.aabb) && !body.strict_intersects(block.aabb)
    });
    if into_solid {
        return None;
    }
    let outcome = if step.was_grounded { LineStep::Held(line) } else { LineStep::Landed(line) };
    Some((delta, outcome))
}

/// A body that stood on a line and walked off its end is held onto the top
/// of a block a little below (the floor at the foot of a ramp), as it is
/// held onto a line: no tick in the air. A body on a block keeps the rule of
/// blocks: static geometry does not take it down a step.
fn off_the_line(world: &World, body: Aabb, down: Vec2, step: WalkStep, allowance: f32) -> Option<(Vec2, LineStep)> {
    if step.entry_line.is_none() || !step.was_grounded || step.supported_by_block {
        return None;
    }
    let feet = body.feet_coord(down);
    let (index, top) = world
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| matches!(block.kind, BlockKind::Solid | BlockKind::OneWay))
        .filter(|(_, block)| crate::collision_semantics::perpendicular_overlap(body, block.aabb, down))
        .map(|(index, block)| (index, block.aabb.head_coord(down)))
        .filter(|(_, top)| *top >= feet && *top <= feet + allowance)
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
    Some((down * (top - feet), LineStep::OntoBlock(index)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::SurfaceChain;

    const DOWN: Vec2 = Vec2::new(0.0, 1.0);

    fn world_with(points: Vec<Vec2>) -> World {
        let mut world = World::new("t", Vec2::new(2000.0, 1000.0), Vec2::ZERO, Vec::new());
        world.chains.push(SurfaceChain::open("ramp", points));
        world
    }

    fn step(velocity: Vec2, prev_feet: f32, travel: f32, was_grounded: bool) -> WalkStep {
        WalkStep {
            velocity,
            prev_feet,
            moved: Vec2::new(travel, 0.0),
            was_grounded,
            entry_line: None,
            supported_by_block: false,
            up_held: false,
            drop_through: false,
        }
    }

    fn body_at(feet: Vec2) -> Aabb {
        Aabb::new(feet - Vec2::new(0.0, 23.0), Vec2::new(14.0, 23.0))
    }

    /// A ramp up to the right at 27 degrees (rise 1 in 2).
    fn ramp() -> World {
        world_with(vec![Vec2::new(0.0, 200.0), Vec2::new(100.0, 200.0), Vec2::new(300.0, 100.0), Vec2::new(400.0, 100.0)])
    }

    #[test]
    fn the_feet_rest_on_the_highest_point_of_the_line_under_the_box() {
        let world = ramp();
        // Centred at x 200 the box spans 186..214; the line is highest at 214.
        let line = standing_on_line(&world, body_at(Vec2::new(200.0, 143.0)), DOWN).expect("on the ramp");
        assert!((line.feet_coord - 143.0).abs() < 1.0e-3, "{line:?}");
        assert!((line.point.x - 214.0).abs() < 1.0e-3);
    }

    #[test]
    fn a_line_is_passed_from_below_and_a_steep_one_is_not_ground() {
        let world = ramp();
        let under = body_at(Vec2::new(200.0, 170.0));
        let rising = settle_on_line(&world, under, DOWN, step(Vec2::new(0.0, -100.0), 175.0, 0.0, false));
        assert!(rising.is_none(), "a rising body passes the line");
        let wall = world_with(vec![Vec2::new(0.0, 300.0), Vec2::new(20.0, 100.0)]);
        assert!(line_under_feet(&wall, body_at(Vec2::new(10.0, 200.0)), DOWN, 500.0, 500.0).is_none());
    }

    #[test]
    fn a_body_that_stood_walks_down_the_ramp_with_no_tick_in_the_air() {
        let world = ramp();
        // Stood at x 250 (feet 143 at its uphill edge 264 -> 118); it walks 6
        // to the left, downhill: the line under it falls 3.
        let x = 244.0;
        let line_feet = 200.0 - (x + 14.0 - 100.0) / 2.0;
        let body = body_at(Vec2::new(x, line_feet - 3.0 + 0.6));
        let walked = WalkStep { entry_line: Some(0), ..step(Vec2::new(-360.0, 36.0), line_feet - 3.0, -6.0, true) };
        let (delta, outcome) = settle_on_line(&world, body, DOWN, walked).expect("held on the line");
        assert!(matches!(outcome, LineStep::Held(_)), "{outcome:?}");
        assert!((body.feet_coord(DOWN) + delta.y - line_feet).abs() < 1.0e-3);
    }

    #[test]
    fn a_falling_body_lands_on_the_line_however_fast_it_falls() {
        let world = ramp();
        let body = body_at(Vec2::new(200.0, 400.0));
        let (delta, outcome) = settle_on_line(&world, body, DOWN, step(Vec2::new(0.0, 3000.0), 40.0, 0.0, false)).expect("landed");
        assert!(matches!(outcome, LineStep::Landed(_)));
        assert!((body.feet_coord(DOWN) + delta.y - 143.0).abs() < 1.0e-3);
    }

    #[test]
    fn a_drop_through_passes_the_line() {
        let world = ramp();
        let body = body_at(Vec2::new(200.0, 146.0));
        let drop = WalkStep { drop_through: true, entry_line: Some(0), ..step(Vec2::new(0.0, 30.0), 143.0, 0.0, true) };
        assert!(settle_on_line(&world, body, DOWN, drop).is_none());
    }
}

#[cfg(test)]
mod portal_cut_tests {
    use crate::frame::FaceRect;
    use crate::world::SurfaceChain;
    use crate::Vec2;

    /// A portal's opening on a line cuts it out: the two pieces either side
    /// are left, and the line is whole where it is not cut.
    #[test]
    fn a_portal_on_a_line_cuts_its_opening_out_of_it() {
        let ramp = SurfaceChain::open("ramp", vec![Vec2::new(0.0, 200.0), Vec2::new(400.0, 0.0)]);
        let t = (Vec2::new(400.0, 0.0) - Vec2::new(0.0, 200.0)).normalize();
        let normal = Vec2::new(t.y, -t.x);
        let centre = Vec2::new(200.0, 100.0);
        let hole = FaceRect { center: centre, normal, half_along: 46.0, half_out: 30.0 };
        let pieces = ramp.without(&hole);
        assert_eq!(pieces.len(), 2);
        let gap = (pieces[1].points[0] - pieces[0].points[pieces[0].points.len() - 1]).length();
        assert!((gap - 92.0).abs() < 1.0e-2, "the gap is the opening: {gap}");
        let far = FaceRect { center: Vec2::new(900.0, 900.0), ..hole };
        assert_eq!(ramp.without(&far).len(), 1);
    }

    /// For a level face the rectangle is a box: its bounds and its overlap
    /// test are the box's.
    #[test]
    fn a_level_face_rect_is_a_box() {
        use ambition_geometry::geometry::{Aabb, AabbExt};
        let r = FaceRect { center: Vec2::new(10.0, 20.0), normal: Vec2::new(0.0, -1.0), half_along: 46.0, half_out: 9.0 };
        let b = r.bounds();
        assert_eq!(b, Aabb::new(Vec2::new(10.0, 20.0), Vec2::new(46.0, 9.0)));
        for (c, h) in [(Vec2::new(60.0, 20.0), 5.0), (Vec2::new(52.0, 20.0), 5.0), (Vec2::new(10.0, 33.0), 4.0), (Vec2::new(10.0, 34.0), 4.0)] {
            let other = Aabb::new(c, Vec2::splat(h));
            assert_eq!(r.strict_intersects(other), b.strict_intersects(other), "{c:?}");
        }
    }
}
