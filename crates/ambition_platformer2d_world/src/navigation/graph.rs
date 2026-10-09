//! The surface graph of a room for one body: the standing surfaces, and the
//! legs between them that the body can do.
//!
//! [`NavGraph::build`] proposes legs from the geometry and the body's
//! [`TraversalEnvelope`], and keeps a leg only when the body ARRIVES: each
//! leg is rolled out in the room, in the movement kernel, with the rule a
//! brain follows it by (`ae::navigation::follow_leg`). There is no second
//! physics, and no second follower.
//!
//! The graph is a pure function of the room, the body's tuning and the frame.
//! It is the same on each peer and after each rewind, so it is a derived
//! cache and no part of a snapshot. It must be built whole: a graph that is
//! built a part at a time gives an answer that depends on when it was asked.
//!
//! NOT MODELLED: a drop through a one-way surface, an air jump, a dash, a wall
//! verb, flight, a surface that moves, a slope, and a hazard in the air of a
//! leg (a hazard on a surface takes that stretch out).

use ambition_platformer2d_core as ae;
use ae::movement::{step_motion, ActionEdges, Edge, InputState, MotionStepContext, MovementAction};
use ae::navigation::{
    follow_leg, mix, LegFacts, LegPhase, LegProgress, NavLeg, NavLegKind, NavNext, ARRIVE_TOLERANCE,
    LAND_TOLERANCE, NAV_WAYPOINTS,
};
use ae::{BodyClusterScratch, LocalAxes, MotionFrame, Vec2, World};

use super::envelope::{EnvelopeProbe, TraversalEnvelope};
use super::surfaces::{standing_surfaces, NavFrame, StandSurface};

/// The most kernel steps one leg may take in its rollout.
const MAX_LEG_STEPS: usize = 360;
/// How far back from an edge a running jump starts, when the surface has room.
const RUN_UP: f32 = 140.0;

/// A leg the body can do, from one surface to another.
#[derive(Clone, Debug, PartialEq)]
pub struct NavLink {
    pub from: usize,
    pub to: usize,
    pub leg: NavLeg,
    /// Seconds from `leg.start`, at rest, to the landing.
    pub cost: f32,
}

/// The standing surfaces of a room and the legs between them, for one body.
#[derive(Clone, Debug)]
pub struct NavGraph {
    pub frame: NavFrame,
    /// Half the body along the side axis and against the fall.
    pub half: Vec2,
    /// The body's top speed along a surface, for the cost of a walk.
    pub run_speed: f32,
    /// The highest the body's feet rise in a jump, and the farthest its
    /// leading edge goes in one. What an author sizes a room by.
    pub apex_rise: f32,
    pub jump_reach: f32,
    pub surfaces: Vec<StandSurface>,
    pub links: Vec<NavLink>,
    /// The links that leave each surface.
    out: Vec<Vec<usize>>,
}

impl NavGraph {
    /// The graph of `world` for `body` under `frame`.
    ///
    /// `None` when the frame is not axis-aligned, or the body does not run:
    /// the envelope does not measure such a body.
    pub fn build(world: &World, body: &BodyClusterScratch, frame: MotionFrame) -> Option<Self> {
        let envelope = TraversalEnvelope::measure(body, frame, EnvelopeProbe::default())?;
        let nav = NavFrame { side: frame.side(), down: frame.down() };
        let half = Vec2::new(envelope.body_width * 0.5, envelope.body_height * 0.5);
        let surfaces = standing_surfaces(world, nav, half * 2.0);
        let jump_reach = envelope.jump.iter().map(|sample| sample.lead).fold(0.0, f32::max);
        let drop_reach = envelope.drop.iter().map(|sample| sample.lead).fold(0.0, f32::max);
        let apex = envelope.apex_rise();
        let mut graph = Self {
            frame: nav,
            half,
            run_speed: envelope.takeoff_speed.max(1.0),
            apex_rise: apex,
            jump_reach,
            out: vec![Vec::new(); surfaces.len()],
            surfaces,
            links: Vec::new(),
        };
        for from in 0..graph.surfaces.len() {
            for to in 0..graph.surfaces.len() {
                if from == to {
                    continue;
                }
                let (a, b) = (&graph.surfaces[from], &graph.surfaces[to]);
                // Up is positive: how far the landing is above the take-off.
                let rise = a.top - b.top;
                let gap = (b.left - a.right).max(a.left - b.right).max(0.0);
                let slack = envelope.body_width + 16.0;
                let hop = rise < apex - 1.0 && gap <= jump_reach + slack;
                let drop = rise < -1.0 && gap <= drop_reach + slack;
                if rise < -envelope.probe.max_drop || !(hop || drop) {
                    continue;
                }
                let best = proposals(a, b, half.x, hop, drop)
                    .into_iter()
                    .filter_map(|leg| {
                        let leg = graph.in_world(leg);
                        graph.rollout(world, body, frame, &leg, to).map(|cost| (leg, cost))
                    })
                    .min_by(|x, y| x.1.total_cmp(&y.1));
                if let Some((leg, cost)) = best {
                    graph.out[from].push(graph.links.len());
                    graph.links.push(NavLink { from, to, leg, cost });
                }
            }
        }
        Some(graph)
    }

    /// A leg whose points are (along, below) pairs, as world points.
    fn in_world(&self, leg: NavLeg) -> NavLeg {
        let point = |p: Vec2| self.frame.point(p.x, p.y);
        NavLeg { kind: leg.kind, start: point(leg.start), takeoff: point(leg.takeoff), land: point(leg.land) }
    }

    /// The seconds `leg` takes in the kernel, when the body arrives on
    /// surface `to` from each place the follower can start it at.
    fn rollout(&self, world: &World, body: &BodyClusterScratch, frame: MotionFrame, leg: &NavLeg, to: usize) -> Option<f32> {
        let mut slowest = 0.0_f32;
        // The follower starts a leg in the tolerance of its start point, so
        // the leg must hold at both ends of the tolerance.
        for offset in [0.0, ARRIVE_TOLERANCE, -ARRIVE_TOLERANCE] {
            slowest = slowest.max(self.rollout_from(world, body, frame, leg, to, offset)?);
        }
        Some(slowest)
    }

    fn rollout_from(
        &self,
        world: &World,
        body: &BodyClusterScratch,
        frame: MotionFrame,
        leg: &NavLeg,
        to: usize,
        offset: f32,
    ) -> Option<f32> {
        let dt = EnvelopeProbe::default().dt;
        let mut body = body.clone();
        {
            let feet = leg.start + self.frame.side * offset;
            let (model, mut clusters) = body.parts();
            ae::movement::transit_body(
                model,
                &mut clusters,
                feet - self.frame.down * self.half.y,
                ae::movement::TransitVelocity::Zero,
            );
        }
        // The body finds its floor before the leg starts.
        for _ in 0..3 {
            step(&mut body, world, frame, dt, ae::navigation::LegInput::default());
        }
        if !body.ground.on_ground {
            return None;
        }
        let mut phase = LegPhase::Commit;
        for index in 0..MAX_LEG_STEPS {
            let feet = body.kinematics.pos + self.frame.down * self.half.y;
            let facts = LegFacts {
                feet,
                vel: body.kinematics.vel,
                on_ground: body.ground.on_ground,
                side: self.frame.side,
                down: self.frame.down,
                dt,
            };
            let (input, progress) = follow_leg(leg, phase, &facts);
            match progress {
                LegProgress::Going(next) => phase = next,
                LegProgress::Failed => return None,
                LegProgress::Arrived => {
                    // On the surface the leg names, not another at its height.
                    return (self.surface_at(feet) == Some(to)).then_some(index as f32 * dt);
                }
            }
            step(&mut body, world, frame, dt, input);
        }
        None
    }

    /// The surface the feet point `feet` stands on.
    pub fn surface_at(&self, feet: Vec2) -> Option<usize> {
        let (along, below) = (self.frame.along(feet), self.frame.below(feet));
        self.surfaces
            .iter()
            .enumerate()
            .filter(|(_, surface)| {
                (surface.top - below).abs() <= LAND_TOLERANCE
                    && along >= surface.left - self.half.x
                    && along <= surface.right + self.half.x
            })
            .min_by(|(_, a), (_, b)| (a.clamp(along) - along).abs().total_cmp(&(b.clamp(along) - along).abs()))
            .map(|(index, _)| index)
    }

    /// The feet point at `along` on `surface`, kept on the surface.
    pub fn point_on(&self, surface: usize, along: f32) -> Vec2 {
        let surface = &self.surfaces[surface];
        self.frame.point(surface.clamp(along), surface.top)
    }

    /// The surfaces a body on `from` can get to, `from` first, in the order
    /// the search finds them.
    pub fn reachable_from(&self, from: usize) -> Vec<usize> {
        let mut seen = vec![false; self.surfaces.len()];
        let mut order = vec![from];
        seen[from] = true;
        let mut next = 0;
        while next < order.len() {
            for link in &self.out[order[next]] {
                let to = self.links[*link].to;
                if !seen[to] {
                    seen[to] = true;
                    order.push(to);
                }
            }
            next += 1;
        }
        order
    }

    /// The links of the fastest route from `feet` to `goal`, both on surfaces.
    pub fn route(&self, feet: Vec2, goal: Vec2) -> Option<Vec<usize>> {
        let (from, to) = (self.surface_at(feet)?, self.surface_at(goal)?);
        // Cost to each surface, where the body stands on it then, and the
        // link it came by. The surfaces are few: no heap.
        let mut best: Vec<Option<(f32, f32, Option<usize>)>> = vec![None; self.surfaces.len()];
        let mut done = vec![false; self.surfaces.len()];
        best[from] = Some((0.0, self.frame.along(feet), None));
        loop {
            let current = (0..self.surfaces.len())
                .filter(|index| !done[*index])
                .filter_map(|index| best[index].map(|(cost, ..)| (index, cost)))
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
                .map(|(index, _)| index)?;
            if current == to {
                break;
            }
            done[current] = true;
            let (cost, at, _) = best[current]?;
            for index in &self.out[current] {
                let link = &self.links[*index];
                let walk = (self.frame.along(link.leg.start) - at).abs() / self.run_speed;
                let total = cost + walk + link.cost;
                if best[link.to].is_none_or(|(known, ..)| total < known) {
                    best[link.to] = Some((total, self.frame.along(link.leg.land), Some(*index)));
                }
            }
        }
        let mut route = Vec::new();
        let mut at = to;
        while let Some((_, _, Some(link))) = best[at] {
            route.push(link);
            at = self.links[link].from;
        }
        route.reverse();
        Some(route)
    }

    /// What a body at `feet` does first to get to `goal`.
    pub fn next(&self, feet: Vec2, goal: Vec2) -> NavNext {
        let Some(from) = self.surface_at(feet) else {
            return NavNext::Unknown;
        };
        let Some(to) = self.surface_at(goal) else {
            return NavNext::Unreachable;
        };
        if from == to {
            let land = self.point_on(to, self.frame.along(goal));
            if (self.frame.along(land) - self.frame.along(feet)).abs() <= ARRIVE_TOLERANCE {
                return NavNext::Arrived;
            }
            return NavNext::Leg(NavLeg { kind: NavLegKind::Walk, start: land, takeoff: land, land });
        }
        match self.route(feet, goal).and_then(|route| route.first().copied()) {
            Some(link) => NavNext::Leg(self.links[link].leg),
            None => NavNext::Unreachable,
        }
    }

    /// Up to [`NAV_WAYPOINTS`] points a body at `feet` can get to, one on each
    /// of some surfaces. `seed` chooses the surfaces and the place on each.
    pub fn waypoints(&self, feet: Vec2, seed: u64) -> ([Vec2; NAV_WAYPOINTS], u8) {
        let mut points = [Vec2::ZERO; NAV_WAYPOINTS];
        let Some(from) = self.surface_at(feet) else {
            return (points, 0);
        };
        let mut reachable = self.reachable_from(from);
        reachable.sort_by_key(|surface| mix(seed ^ (*surface as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)));
        reachable.truncate(NAV_WAYPOINTS);
        for (slot, surface) in reachable.iter().enumerate() {
            let stretch = &self.surfaces[*surface];
            let fraction = (mix(seed.rotate_left(17) ^ *surface as u64) % 1000) as f32 / 999.0;
            points[slot] = self.frame.point(stretch.left + stretch.width() * fraction, stretch.top);
        }
        (points, reachable.len() as u8)
    }
}

/// The legs worth a rollout from surface `a` to surface `b`, as (along, below)
/// points. Few: each one costs a rollout.
fn proposals(a: &StandSurface, b: &StandSurface, half_width: f32, hop: bool, drop: bool) -> Vec<NavLeg> {
    let mut legs = Vec::new();
    let leg = |kind, start: f32, takeoff: f32, land: f32| NavLeg {
        kind,
        start: Vec2::new(a.clamp(start), a.top),
        takeoff: Vec2::new(takeoff, a.top),
        land: Vec2::new(b.clamp(land), b.top),
    };
    // How far onto a surface a landing aims, from the end the body comes by.
    let onto = (half_width + 4.0).min(b.width() * 0.5);
    let overlap = (a.left.max(b.left), a.right.min(b.right));
    if overlap.0 <= overlap.1 {
        if b.top < a.top && hop {
            // `b` is above `a`. Up through a one-way surface.
            if b.one_way {
                let middle = (overlap.0 + overlap.1) * 0.5;
                legs.push(leg(NavLegKind::Hop, middle, middle, middle));
            }
            // Up past an end of `b`, from next to it.
            for (end, direction) in [(b.left, 1.0), (b.right, -1.0)] {
                let beside = end - direction * (half_width + 3.0);
                if beside >= a.left && beside <= a.right {
                    legs.push(leg(NavLegKind::Hop, beside, beside, end + direction * onto));
                }
            }
        }
        if b.top > a.top && drop {
            // `b` is below `a`: off an end of `a`.
            for (end, direction) in [(a.left, -1.0), (a.right, 1.0)] {
                let past = end + direction * (half_width + 3.0);
                legs.push(leg(NavLegKind::Drop, end - direction * 16.0, past, past + direction * 2.0));
            }
        }
        return legs;
    }
    // `b` is to one side of `a`.
    let direction = if b.left > a.right { 1.0 } else { -1.0 };
    let (edge, near) = if direction > 0.0 { (a.right, b.left) } else { (a.left, b.right) };
    let land = near + direction * onto;
    if hop {
        legs.push(leg(NavLegKind::Hop, edge, edge, land));
        let start = a.clamp(edge - direction * RUN_UP);
        if (edge - start).abs() >= 24.0 {
            legs.push(leg(NavLegKind::Hop, start, edge, land));
        }
    }
    if drop {
        legs.push(leg(NavLegKind::Drop, edge - direction * 16.0, edge + direction * (half_width + 3.0), land));
    }
    legs
}

fn step(body: &mut BodyClusterScratch, world: &World, frame: MotionFrame, dt: f32, input: ae::navigation::LegInput) {
    let input = InputState {
        axes: LocalAxes::new(input.axis, 0.0),
        movement: ActionEdges::<MovementAction>::EMPTY.with(
            MovementAction::Jump,
            Edge { pressed: input.jump_pressed, held: input.jump_held, released: false },
        ),
        ..Default::default()
    };
    let facing_intent = input.local_axis().x;
    let (model, mut clusters) = body.parts();
    step_motion(
        model,
        &mut clusters,
        MotionStepContext {
            world,
            input,
            frame,
            facing_intent,
            dt,
            // A rollout of this body alone, not its real motion among others.
            contact: ae::movement::BodyContactField::NONE,
            pose_owned_externally: false,
            recovery_commitment_outstanding: false,
        },
    );
}

#[cfg(test)]
mod tests;
