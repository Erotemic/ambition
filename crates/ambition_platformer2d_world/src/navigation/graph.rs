//! The surface graph of a room for one body: the standing surfaces, and the
//! legs between them that the body can do.
//!
//! [`NavGraph::build`] proposes legs from the geometry and the body's
//! [`TraversalEnvelope`], and keeps a leg only when the body ARRIVES: each
//! leg is rolled out in the room, in the movement kernel, with the rule a
//! brain follows it by (`ae::navigation::follow_leg`). There is no second
//! physics, and no second follower.
//!
//! A full graph is a pure function of the room, the body's tuning and the
//! frame. It is the same on each peer and after each rewind. Autoplay can
//! request a local graph for two surfaces. Both builds complete the work
//! for their selected coverage before they publish a result.
//!
//! An air jump is modelled as one double hop: the second press at the top of
//! the first arc, proposed only where one jump does not reach. A wall verb is
//! modelled as one wall climb: a hop to the face under a higher surface, a
//! cling, a climb, the ledge and the pull-up, proposed only where nothing else
//! arrives and only for a body that can cling and climb. Flight is modelled as
//! one fly leg: toggle flight on, rise to the air above a higher surface, go
//! along to it, toggle flight off and fall onto it, proposed last and only for
//! a body that can toggle flight. A dash is modelled as one dash hop: a jump,
//! and a dash toward the landing at the top of the arc, proposed only where a
//! jump does not reach. A wall jump is modelled as one leg up a shaft of two
//! walls: a kick off each wall the body touches below the landing, proposed
//! after the climb and only where nothing cheaper arrives.
//!
//! A leg whose body the kernel resets (a hazard in its air, a fall out of the
//! world) fails its rollout, by the kernel's own reset flag. A leg whose body
//! enters an exit of [`NavGraph::build_avoiding`] fails too.
//!
//! A slope is modelled as one slope leg: an open surface chain whose two ends are
//! at the tops of two surfaces joins them, and the body walks from one to the
//! other along it, no jump. The chain itself is no standing surface, so a
//! route is planned from the surfaces at its ends.
//!
//! NOT MODELLED: a drop through a one-way surface, a second air jump, an air
//! jump in a drop, a dash on the ground or up, a wall jump off one wall onto
//! its own top, permanent flight, a pogo bounce (a combat effect, which the
//! kernel rollout does not run), a surface that moves, and standing on a
//! slope (a route from the middle of a slope finds no surface under it).

use std::collections::BTreeMap;
use std::sync::Arc;

use ambition_platformer2d_core as ae;
use ae::movement::{step_motion, ActionEdges, Edge, InputState, MotionStepContext, MovementAction};
use ae::navigation::{
    follow_leg, mix, FlightLaw, LegInput, LegFacts, LegPhase, LegProgress, NavLeg, NavLegKind, NavNext, ARRIVE_TOLERANCE,
    NAV_WAYPOINTS,
};
use ae::{BodyClusterScratch, LocalAxes, MotionFrame, Vec2, World};

use super::envelope::{EnvelopeProbe, TraversalEnvelope};
use super::flight::FlightMap;
use super::surfaces::{GroundMap, NavFrame, StandSurface, SLOPE_END_REACH};

/// The most kernel steps one leg may take in its rollout.
const MAX_LEG_STEPS: usize = 360;
/// Steps a body may stand still on the ground in `Commit` before its leg has
/// failed. It starts the leg at rest, and it moves on its first steps.
const STALL_STEPS: usize = 15;
/// How far back from an edge a running jump starts, when the surface has room.
const RUN_UP: f32 = 140.0;
/// The most a wall climb rises: a leg rollout has `MAX_LEG_STEPS`, and a
/// climb of this height at the kernel's climb speed fits in it.
const MAX_CLIMB: f32 = 600.0;
/// The most further than a jump a dash hop is proposed for. A filter of the
/// proposals only: the kernel rollout says whether the leg arrives.
const MAX_DASH_GAIN: f32 = 400.0;

type SettledStarts = BTreeMap<[u32; 2], Option<BodyClusterScratch>>;

/// A leg the body can do, from one surface to another.
#[derive(Clone, Debug, PartialEq)]
pub struct NavLink {
    pub from: usize,
    pub to: usize,
    pub leg: NavLeg,
    /// Seconds from `leg.start`, at rest, to the landing.
    pub cost: f32,
}

/// What one [`NavGraph::build`] cost, in kernel work.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BuildCost {
    /// Leg rollouts run.
    pub rollouts: usize,
    /// Kernel steps over all rollouts.
    pub steps: usize,
    /// The part of `steps` spent in rollouts that did not arrive.
    pub failed_steps: usize,
}

impl BuildCost {
    fn add(&mut self, other: Self) {
        self.rollouts += other.rollouts;
        self.steps += other.steps;
        self.failed_steps += other.failed_steps;
    }
}

/// The standing surfaces of a room and the legs between them, for one body.
#[derive(Clone, Debug)]
pub struct NavGraph {
    pub ground: Arc<GroundMap>,
    /// The body's top speed along a surface, for the cost of a walk.
    pub run_speed: f32,
    /// The body's braking on a surface with no stick (px/s²), measured in
    /// the kernel. A follower lets go of the stick when its slide would take
    /// it to the point it stops on (`LegFacts::brake`). Zero: not measured.
    pub braking: f32,
    /// The speed of the body's first step from rest at a full press (a dash
    /// is at speed on its first step).
    pub start_speed: f32,
    /// The highest the body's feet rise in a jump, and the farthest its
    /// leading edge goes in one. What an author sizes a room by.
    pub apex_rise: f32,
    pub jump_reach: f32,
    pub links: Vec<NavLink>,
    /// What the build cost.
    pub cost: BuildCost,
    /// The air of the room for this body, when it can fly: the routes of a
    /// flight that stays in the air ([`FlightMap::route`]).
    pub flight: Option<FlightMap>,
    local_only: bool,
    /// The links that leave each surface.
    out: Vec<Vec<usize>>,
    /// The surfaces each surface reaches ([`Self::reach_from`]), found the
    /// first time it is asked and kept: the graph does not change once built.
    reach: Vec<std::sync::OnceLock<Box<[usize]>>>,
    /// Each link's cost to a goal ([`Self::to_goal`]), for the goals asked
    /// for lately: a pure function of the graph and the goal.
    destinations: DestinationTables,
    /// The answers of [`Self::waypoints`] lately asked for, by the feet's
    /// surface and the seed: a pure function of the graph and those two.
    waypoint_sets: KeptWaypoints,
}

/// A request that can use a walk or a direct link between two surfaces.
#[derive(Clone, Copy, Debug)]
pub struct DirectWalk {
    pub center: Vec2,
    pub destination: Vec2,
}

/// How many (surface, seed) waypoint answers a graph keeps: one or two per
/// navigating body, whose seed holds while its errand does.
const WAYPOINT_SETS_KEPT: usize = 32;

/// The answers [`NavGraph::waypoints`] keeps, newest last. Like
/// [`DestinationTables`], not part of the graph's value.
#[derive(Debug, Default)]
struct KeptWaypoints(std::sync::Mutex<Vec<((usize, u64), ([Vec2; NAV_WAYPOINTS], u8))>>);

impl Clone for KeptWaypoints {
    fn clone(&self) -> Self {
        Self::default()
    }
}

/// How many goals' tables a graph keeps. A body walking to a waypoint asks
/// for one goal tick after tick; a few bodies with a few goals share a room.
const DESTINATIONS_KEPT: usize = 8;

/// The tables [`NavGraph::to_goal`] keeps, newest last. Not part of the
/// graph's value: a clone starts empty, and every answer is the same with or
/// without a kept table.
#[derive(Debug, Default)]
struct DestinationTables(std::sync::Mutex<Vec<((usize, u32), std::sync::Arc<[f32]>)>>);

impl Clone for DestinationTables {
    fn clone(&self) -> Self {
        Self::default()
    }
}

/// What a body's movement does in empty space: its traversal envelope and,
/// for a body that flies, its flight law. Each is measured in a probe world
/// of its own, so it depends on the body and its motion frame and on no
/// room: a graph built in a second room need not measure it again.
#[derive(Clone, Debug, PartialEq)]
pub struct NavMotion {
    pub envelope: TraversalEnvelope,
    pub flight: Option<ae::navigation::FlightLaw>,
}

impl NavMotion {
    /// Measure `body` under `frame`. `None` when the envelope cannot be
    /// measured (see [`TraversalEnvelope::measure`]): the body has no graph.
    pub fn measure(body: &BodyClusterScratch, frame: MotionFrame) -> Option<Self> {
        let envelope = TraversalEnvelope::measure(body, frame, EnvelopeProbe::default())?;
        let flight = body.abilities.abilities.fly.then(|| measure_flight(body, frame)).flatten();
        Some(Self { envelope, flight })
    }
}

/// An arrival waiting in [`NavGraph::route`]'s search: the cheapest first,
/// and of two as cheap the lower state index, so ties never depend on the
/// order they were pushed.
#[derive(Clone, Copy)]
struct RouteOpen {
    cost: f32,
    state: usize,
}

impl PartialEq for RouteOpen {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl Eq for RouteOpen {}

impl PartialOrd for RouteOpen {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RouteOpen {
    // Reversed: `BinaryHeap` pops its greatest.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.cost.total_cmp(&self.cost).then(other.state.cmp(&self.state))
    }
}

impl NavGraph {
    /// The graph of `world` for `body` under `frame`.
    ///
    /// `None` when the frame is not axis-aligned, or the body does not run:
    /// the envelope does not measure such a body.
    pub fn build(world: &World, body: &BodyClusterScratch, frame: MotionFrame) -> Option<Self> {
        Self::build_avoiding(world, &[], body, frame)
    }

    /// [`Self::build`] for a body that leaves the room when it enters one of
    /// `exits`: a leg whose body enters one fails, because the body is in
    /// another room then. For a body a player drives, whose room transition an
    /// overlap fires (an edge exit, a walk zone). A body no slot drives does
    /// not cross by a zone, and its graph has no exits.
    pub fn build_avoiding(
        world: &World,
        exits: &[ae::Aabb],
        body: &BodyClusterScratch,
        frame: MotionFrame,
    ) -> Option<Self> {
        Self::build_on(world, exits, body, frame, bevy_tasks::ComputeTaskPool::try_get().map(|pool| &**pool))
    }

    /// [`Self::build_avoiding`] on `pool`, or on this thread with none.
    ///
    /// A build that runs in the background passes the pool it runs on: on the
    /// compute pool its surfaces queue ahead of the frame's systems, and the
    /// frame waits for the whole build (1.2 s a broken block in Mary-O 1-2 on
    /// the phone: MEASURED, `android-all-mary_o-auto-20261022T205211Z`).
    pub fn build_on(
        world: &World,
        exits: &[ae::Aabb],
        body: &BodyClusterScratch,
        frame: MotionFrame,
        pool: Option<&bevy_tasks::TaskPool>,
    ) -> Option<Self> {
        Self::build_measured(world, exits, body, frame, &NavMotion::measure(body, frame)?, pool)
    }

    /// [`Self::build_on`] with the body's [`NavMotion`] already measured: a
    /// caller that builds one body's graph in several rooms measures its
    /// motion once. `motion` must be `NavMotion::measure(body, frame)`.
    pub fn build_measured(
        world: &World,
        exits: &[ae::Aabb],
        body: &BodyClusterScratch,
        frame: MotionFrame,
        motion: &NavMotion,
        pool: Option<&bevy_tasks::TaskPool>,
    ) -> Option<Self> {
        Self::build_measured_for(world, exits, body, frame, motion, pool, None)
    }

    /// Try a continuous walk or a measured link between the requested
    /// surfaces. Build the full graph if neither is available.
    pub fn build_measured_for(
        world: &World,
        exits: &[ae::Aabb],
        body: &BodyClusterScratch,
        frame: MotionFrame,
        motion: &NavMotion,
        pool: Option<&bevy_tasks::TaskPool>,
        direct: Option<DirectWalk>,
    ) -> Option<Self> {
        let nav = NavFrame { side: frame.side(), down: frame.down() };
        let ground = Arc::new(GroundMap::build(world, nav, body.kinematics.size())?);
        Self::build_on_ground(world, exits, body, frame, motion, pool, direct, ground)
    }

    pub(super) fn build_on_ground(
        world: &World,
        exits: &[ae::Aabb],
        body: &BodyClusterScratch,
        frame: MotionFrame,
        motion: &NavMotion,
        pool: Option<&bevy_tasks::TaskPool>,
        direct: Option<DirectWalk>,
        ground: Arc<GroundMap>,
    ) -> Option<Self> {
        let envelope = &motion.envelope;
        let (nav, half, count) = (ground.frame, ground.half, ground.surfaces.len());
        let jump_reach = envelope.jump.iter().map(|sample| sample.lead).fold(0.0, f32::max);
        let drop_reach = envelope.drop.iter().map(|sample| sample.lead).fold(0.0, f32::max);
        let apex = envelope.apex_rise();
        let air_jump_reach = envelope.air_jump.iter().map(|sample| sample.lead).fold(0.0, f32::max);
        // A body with no air jump measures the jump arc again: no double hop.
        let air_jump_apex = envelope.air_jump_apex_rise();
        let verbs = body.abilities.abilities;
        let can_climb = verbs.wall_cling && verbs.wall_climb;
        let can_fly = verbs.fly && verbs.fly_toggle;
        let can_dash = verbs.dash;
        let can_wall_jump = verbs.wall_jump;
        let mut graph = Self {
            ground,
            run_speed: envelope.takeoff_speed.max(1.0),
            braking: envelope.braking(),
            start_speed: envelope.start_speed,
            apex_rise: apex,
            jump_reach,
            out: vec![Vec::new(); count],
            reach: (0..count).map(|_| std::sync::OnceLock::new()).collect(),
            destinations: DestinationTables::default(),
            waypoint_sets: KeptWaypoints::default(),
            links: Vec::new(),
            cost: BuildCost::default(),
            flight: None,
            local_only: false,
        };
        let endpoints = direct.and_then(|walk| graph.walk_endpoints(walk));
        if endpoints.is_some_and(|(from, to)| from == to) {
            graph.local_only = true;
            return Some(graph);
        }
        // The links that leave each surface are a pure function of the graph
        // so far, so the surfaces are taken on the compute pool, one to a task,
        // and their links put back in surface order: the graph is the one a
        // single thread builds, on each peer and after each rewind (the hub's
        // dog graph is 83 ms on one thread: MEASURED, `nav_graph_census`,
        // 2026-10-18).
        let links_from = |from: usize, direct_to: Option<usize>| -> (Vec<(usize, NavLeg, f32)>, BuildCost) {
            let mut found = Vec::new();
            let mut spent = BuildCost::default();
            let mut starts = SettledStarts::new();
            let targets = direct_to.map_or(0..graph.ground.surfaces.len(), |to| to..to + 1);
            for to in targets {
                if from == to {
                    continue;
                }
                let (a, b) = (&graph.ground.surfaces[from], &graph.ground.surfaces[to]);
                // Up is positive: how far the landing is above the take-off.
                let rise = a.top - b.top;
                let gap = (b.left - a.right).max(a.left - b.right).max(0.0);
                let slack = envelope.body_width + 16.0;
                let hop = rise < apex - 1.0 && gap <= jump_reach + slack;
                let drop = rise < -1.0 && gap <= drop_reach + slack;
                let double = air_jump_apex > apex + 1.0
                    && rise < air_jump_apex - 1.0
                    && gap <= air_jump_reach + slack;
                // A dash carries the body along at the height of the apex: no
                // higher than a jump, and further.
                let dash = can_dash && rise < apex - 1.0 && gap <= jump_reach + MAX_DASH_GAIN + slack;
                let climb = can_climb && rise > 1.0 && rise <= MAX_CLIMB && gap <= air_jump_reach.max(jump_reach) + slack;
                let wall_jump = can_wall_jump && rise > 1.0 && rise <= MAX_CLIMB && gap <= jump_reach + slack;
                // Flight is for going up: a drop gets down.
                let fly = can_fly && rise > 1.0;
                let slopes = slope_proposals(&graph.ground.slope_pieces, nav, a, b, half.x);
                if slopes.is_empty()
                    && (rise < -envelope.probe.max_drop || !(hop || drop || double || dash || climb || wall_jump || fly))
                {
                    continue;
                }
                let mut cost = BuildCost::default();
                let mut arrived = |legs: Vec<NavLeg>, cost: &mut BuildCost| {
                    legs.into_iter()
                        .filter_map(|leg| {
                            let leg = graph.in_world(leg);
                            graph.rollout(world, exits, body, frame, &leg, to, &mut starts, cost)
                                .map(|seconds| (leg, seconds))
                        })
                        .min_by(|x, y| x.1.total_cmp(&y.1))
                };
                // A double hop only where no hop or drop arrives: each
                // proposal costs rollouts, and the leg with one jump is the
                // cheaper one to follow.
                let as_kind = |kind: NavLegKind| {
                    proposals(a, b, half.x, true, false).into_iter().map(move |leg| NavLeg { kind, ..leg }).collect()
                };
                // The cheaper leg first: one jump, then the air jump, then a
                // dash, then a climb, then wall jumps, then flight, each only
                // where the one before it does not arrive.
                // A walk up or down a slope first: it needs no verb at all.
                let best = (!slopes.is_empty())
                    .then(|| arrived(slopes, &mut cost))
                    .flatten()
                    .or_else(|| arrived(proposals(a, b, half.x, hop, drop), &mut cost))
                    .or_else(|| double.then(|| arrived(as_kind(NavLegKind::DoubleHop), &mut cost)).flatten())
                    .or_else(|| dash.then(|| arrived(as_kind(NavLegKind::DashHop), &mut cost)).flatten())
                    .or_else(|| climb.then(|| arrived(as_kind(NavLegKind::WallClimb), &mut cost)).flatten())
                    .or_else(|| wall_jump.then(|| arrived(as_kind(NavLegKind::WallJump), &mut cost)).flatten())
                    .or_else(|| {
                        fly.then(|| {
                            let mut legs: Vec<NavLeg> = as_kind(NavLegKind::Fly);
                            legs.extend(graph.fly_proposals(world, a, b));
                            arrived(legs, &mut cost)
                        })
                        .flatten()
                    });
                spent.add(cost);
                if let Some((leg, cost)) = best {
                    found.push((to, leg, cost));
                }
            }
            (found, spent)
        };
        let local_cost = if let Some((from, to)) = endpoints {
            let (outbound, spent) = links_from(from, Some(to));
            if !outbound.is_empty() {
                let (returning, return_cost) = links_from(to, Some(from));
                graph.local_only = true;
                graph.add_links(from, outbound, spent);
                graph.add_links(to, returning, return_cost);
                return Some(graph);
            }
            spent
        } else {
            BuildCost::default()
        };
        let surfaces = graph.ground.surfaces.len();
        let rows: Vec<(Vec<(usize, NavLeg, f32)>, BuildCost)> = match pool {
            Some(pool) if pool.thread_num() > 1 && surfaces > 1 => {
                let links_from = &links_from;
                pool.scope(|scope| {
                    for from in 0..surfaces {
                        scope.spawn(async move { links_from(from, None) });
                    }
                })
            }
            _ => (0..surfaces).map(|from| links_from(from, None)).collect(),
        };
        graph.cost.add(local_cost);
        graph.flight = motion.flight.map(|law| FlightMap::build(world, exits, half, law));
        for (from, (found, spent)) in rows.into_iter().enumerate() {
            graph.add_links(from, found, spent);
        }
        Some(graph)
    }

    fn add_links(&mut self, from: usize, links: Vec<(usize, NavLeg, f32)>, cost: BuildCost) {
        self.cost.add(cost);
        for (to, leg, cost) in links {
            self.out[from].push(self.links.len());
            self.links.push(NavLink { from, to, leg, cost });
        }
    }

    /// Flights from `a` up to `b` that start beside each end of a block the
    /// body rises past, with points as (along, below) pairs. A flight rises straight up
    /// from its start, so it gets through an opening in a ceiling (a grate)
    /// only from under it, and an opening starts beside the end of a block.
    fn fly_proposals(&self, world: &World, a: &StandSurface, b: &StandSurface) -> Vec<NavLeg> {
        // What the body passes through when it rises from `a` to above `b`.
        let band = (b.top - self.ground.half.y * 2.0 - ae::navigation::FLIGHT_CLEARANCE, a.top);
        let mut starts: Vec<f32> = Vec::new();
        for block in &world.blocks {
            if matches!(block.kind, ae::BlockKind::OneWay | ae::BlockKind::PogoOrb) {
                continue;
            }
            let along = [self.ground.frame.along(block.aabb.min), self.ground.frame.along(block.aabb.max)];
            let below = [self.ground.frame.below(block.aabb.min), self.ground.frame.below(block.aabb.max)];
            let (top, bottom) = (below[0].min(below[1]), below[0].max(below[1]));
            if bottom <= band.0 || top >= band.1 {
                continue;
            }
            let (left, right) = (along[0].min(along[1]), along[0].max(along[1]));
            // Clear of the end by the follower's tolerance too: it starts the
            // leg anywhere in it.
            let clear = self.ground.half.x + ARRIVE_TOLERANCE + 2.0;
            starts.push(left - clear);
            starts.push(right + clear);
        }
        starts.retain(|start| *start >= a.left && *start <= a.right);
        starts.sort_by(|x, y| x.total_cmp(y));
        starts.dedup_by(|x, y| (*x - *y).abs() < 1.0);
        starts
            .into_iter()
            .map(|start| NavLeg {
                kind: NavLegKind::Fly,
                start: Vec2::new(start, a.top),
                takeoff: Vec2::new(start, a.top),
                land: Vec2::new(b.clamp(start), b.top),
                land_span: [b.left - self.ground.half.x, b.right + self.ground.half.x],
            })
            .collect()
    }

    /// A leg whose points are (along, below) pairs, as world points.
    fn in_world(&self, leg: NavLeg) -> NavLeg {
        let point = |p: Vec2| self.ground.frame.point(p.x, p.y);
        // The span is places on the side axis, and they are the same in
        // each of the two forms.
        NavLeg { start: point(leg.start), takeoff: point(leg.takeoff), land: point(leg.land), ..leg }
    }

    /// The seconds `leg` takes in the kernel, when the body arrives on
    /// surface `to` from each place the follower can start it at.
    #[allow(clippy::too_many_arguments)]
    fn rollout(
        &self,
        world: &World,
        exits: &[ae::Aabb],
        body: &BodyClusterScratch,
        frame: MotionFrame,
        leg: &NavLeg,
        to: usize,
        starts: &mut SettledStarts,
        cost: &mut BuildCost,
    ) -> Option<f32> {
        let mut slowest = 0.0_f32;
        // The follower starts a leg in the tolerance of its start point, so
        // the leg must hold at both ends of the tolerance.
        for offset in [0.0, ARRIVE_TOLERANCE, -ARRIVE_TOLERANCE] {
            let feet = leg.start + self.ground.frame.side * offset;
            // Legs with the same start share the neutral steps before input.
            let mut settling_steps = 0;
            let settled = starts.entry(feet.to_array().map(f32::to_bits))
                .or_insert_with(|| self.settle_start(world, frame, body, feet, &mut settling_steps));
            let (seconds, steps) = settled.as_ref().map_or((None, 0), |body| {
                self.rollout_from(world, exits, body.clone(), frame, leg, to)
            });
            let steps = steps + settling_steps;
            cost.rollouts += 1;
            cost.steps += steps;
            if seconds.is_none() {
                cost.failed_steps += steps;
            }
            slowest = slowest.max(seconds?);
        }
        Some(slowest)
    }

    fn settle_start(
        &self,
        world: &World,
        frame: MotionFrame,
        body: &BodyClusterScratch,
        feet: Vec2,
        steps: &mut usize,
    ) -> Option<BodyClusterScratch> {
        let mut body = body.clone();
        {
            let (model, mut clusters) = body.parts();
            ae::movement::transit_body(
                model,
                &mut clusters,
                feet - self.ground.frame.down * self.ground.half.y,
                ae::movement::TransitVelocity::Zero,
            );
        }
        for _ in 0..3 {
            *steps += 1;
            if step(&mut body, world, frame, EnvelopeProbe::default().dt, LegInput::default()) {
                return None;
            }
        }
        body.ground.on_ground.then_some(body)
    }

    fn rollout_from(
        &self,
        world: &World,
        exits: &[ae::Aabb],
        mut body: BodyClusterScratch,
        frame: MotionFrame,
        leg: &NavLeg,
        to: usize,
    ) -> (Option<f32>, usize) {
        let dt = EnvelopeProbe::default().dt;
        let mut phase = LegPhase::Commit;
        // Where the feet were when the body last moved on the ground.
        let mut moved_at = (0, self.ground.frame.along(body.kinematics.pos));
        // Where the feet were when the body last moved in flight.
        let mut flown_at = (0, body.kinematics.pos + self.ground.frame.down * self.ground.half.y);
        // A slope is walked, and a long one (a freight track down a hill)
        // takes longer than the step limit at a walk: it has a step per two
        // pixels of its length, at least the limit.
        let steps = match leg.kind {
            ae::navigation::NavLegKind::Slope => MAX_LEG_STEPS.max(((leg.land - leg.start).length() / 2.0) as usize),
            _ => MAX_LEG_STEPS,
        };
        for index in 0..steps {
            let feet = body.kinematics.pos + self.ground.frame.down * self.ground.half.y;
            // A body that runs into a wall in `Commit` stands there until the
            // step limit: a walk-off with a wall past the edge. Measured, that
            // was most of the steps of the failed rollouts. Stopped for
            // STALL_STEPS on the ground means it has failed.
            if phase == LegPhase::Commit && body.ground.on_ground {
                let along = self.ground.frame.along(feet);
                if (along - moved_at.1).abs() > 1.0 {
                    moved_at = (index, along);
                } else if index - moved_at.0 >= STALL_STEPS {
                    return (None, index);
                }
            }
            // A flight under a ceiling hovers there until the step limit.
            // Stopped in the air for STALL_STEPS means it has failed.
            if leg.kind == ae::navigation::NavLegKind::Fly && phase == LegPhase::Air {
                if (feet - flown_at.1).length() > 1.0 {
                    flown_at = (index, feet);
                } else if index - flown_at.0 >= STALL_STEPS {
                    return (None, index);
                }
            }
            let facts = LegFacts {
                feet,
                vel: body.kinematics.vel,
                on_ground: body.ground.on_ground,
                side: self.ground.frame.side,
                down: self.ground.frame.down,
                dt,
                wall: body.wall.on_wall.then_some(body.wall.wall_normal_x.signum()),
                approach: None,
            };
            let (input, progress) = follow_leg(leg, phase, &facts);
            match progress {
                LegProgress::Going(next) => phase = next,
                LegProgress::Failed => return (None, index),
                LegProgress::Arrived => {
                    // On the surface the leg names, not another at its height.
                    return ((self.surface_at(feet) == Some(to)).then_some(index as f32 * dt), index);
                }
            }
            // A leg that touches a hazard (or leaves the world) fails: the
            // body that does it is reset, and it does not arrive.
            if step(&mut body, world, frame, dt, input) {
                return (None, index + 1);
            }
            // A body in an exit is in another room: the leg did not arrive.
            let reach = body.kinematics.aabb();
            if exits.iter().any(|exit| ae::AabbExt::strict_intersects(reach, *exit)) {
                return (None, index + 1);
            }
        }
        (None, steps)
    }

    /// How this body stops on a point ([`ae::navigation::ApproachLaw`]):
    /// `None` when the body does not slow with no stick.
    pub fn approach_law(&self) -> Option<ae::navigation::ApproachLaw> {
        if self.braking <= 0.0 {
            return None;
        }
        // From rest, a full press moves the body one step at its start speed
        // and then slides it: nearer than that, it creeps.
        let dash = self.start_speed.abs();
        let creep_band = dash / 60.0 + dash * dash / (2.0 * self.braking);
        Some(ae::navigation::ApproachLaw { braking: self.braking, creep_band })
    }

    /// The surface the feet point `feet` stands on.
    pub fn surface_at(&self, feet: Vec2) -> Option<usize> {
        self.ground.surface_at(feet)
    }

    /// Where a body in the air at `feet` should come down, when no leg
    /// steers it (a leg failed in the air: a foe struck it, a block broke
    /// under it). The surface point nearest the body that is below its feet,
    /// or at most `rise` above them (what an air jump the owner can still
    /// spend gains), with a lean toward `toward` (its goal). `None`: no
    /// surface is there.
    pub fn landing_in_reach(&self, feet: Vec2, rise: f32, toward: Vec2) -> Option<Vec2> {
        let (along, below) = (self.ground.frame.along(feet), self.ground.frame.below(feet));
        let goal_along = self.ground.frame.along(toward);
        (0..self.ground.surfaces.len())
            .filter(|index| self.ground.surfaces[*index].top >= below - rise)
            .map(|index| {
                let surface = &self.ground.surfaces[index];
                let at = surface.clamp(along);
                // The way along to the surface, the fall to it, and a lean
                // toward the goal: the nearest landing first, and of two as
                // near, the one on the way.
                let cost = (at - along).abs() + (surface.top - below).abs() * 0.25 + (at - goal_along).abs() * 0.05;
                (self.ground.frame.point(at, surface.top), cost)
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(point, _)| point)
    }

    /// The feet point at `along` on `surface`, kept on the surface.
    pub fn point_on(&self, surface: usize, along: f32) -> Vec2 {
        self.ground.point_on(surface, along)
    }

    /// The nearest standing point at or below the destination.
    pub fn standing_destination(&self, destination: Vec2) -> Option<Vec2> {
        self.ground.standing_destination(destination)
    }

    /// True if this build covers standing surfaces and selected direct links.
    pub fn local_only(&self) -> bool {
        self.local_only
    }

    /// This request has a continuous walk or a measured direct link.
    pub fn can_answer_walk(&self, walk: DirectWalk) -> bool {
        self.walk_endpoints(walk).is_some_and(|(from, to)| {
            from == to || self.direct_link(from, to).is_some()
        })
    }

    fn walk_endpoints(&self, walk: DirectWalk) -> Option<(usize, usize)> {
        let feet = walk.center + self.ground.frame.down * self.ground.half.y;
        let from = self.surface_at(feet)?;
        let to = self.surface_at(self.standing_destination(walk.destination)?)?;
        Some((from, to))
    }

    fn direct_link(&self, from: usize, to: usize) -> Option<NavLeg> {
        self.out[from].iter().map(|&link| &self.links[link])
            .find(|link| link.to == to).map(|link| link.leg)
    }

    /// The surfaces a body on `from` can get to, `from` first, in the order
    /// the search finds them.
    pub fn reachable_from(&self, from: usize) -> Vec<usize> {
        self.reach_from(from).to_vec()
    }

    /// [`Self::reachable_from`], searched once for each origin a caller asks
    /// about and kept with the graph: the advisor asks it for each waypoint,
    /// placement and errand of each body each tick.
    pub fn reach_from(&self, from: usize) -> &[usize] {
        self.reach[from].get_or_init(|| self.search_reach(from).into_boxed_slice())
    }

    /// Can a body on `from` get to `to`?
    pub fn reaches(&self, from: usize, to: usize) -> bool {
        self.reach_from(from).contains(&to)
    }

    fn search_reach(&self, from: usize) -> Vec<usize> {
        let mut seen = vec![false; self.ground.surfaces.len()];
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

    /// The links of the fastest route from `feet` to `goal`, both on
    /// surfaces, under the graph's cost model: each leg's measured cost, and
    /// a walk at run speed from where the body stands to the next takeoff and
    /// from the last landing to `goal`. Empty when both are on one surface
    /// (the body walks).
    ///
    /// A search state is an ARRIVAL (the start, or the landing of one link),
    /// not a surface: the next walk depends on where on the surface the body
    /// stands, so a cheaper arrival does not rule out a dearer one somewhere
    /// else on the same surface. This is the fastest route over the admitted
    /// legs, not the fastest physical motion: each leg is measured from rest,
    /// and the build keeps one preferred leg per surface pair.
    pub fn route(&self, feet: Vec2, goal: Vec2) -> Option<Vec<usize>> {
        let (from, to) = (self.surface_at(feet)?, self.surface_at(goal)?);
        if from == to {
            return Some(Vec::new());
        }
        let start = self.links.len();
        let goal_along = self.ground.frame.along(goal);
        let arrival = |state: usize| match state == start {
            true => (from, self.ground.frame.along(feet)),
            false => (self.links[state].to, self.ground.frame.along(self.links[state].leg.land)),
        };
        // Cost to each arrival and the arrival it came from.
        let mut best: Vec<Option<(f32, usize)>> = vec![None; start + 1];
        let mut done = vec![false; start + 1];
        let mut open = std::collections::BinaryHeap::new();
        best[start] = Some((0.0, start));
        open.push(RouteOpen { cost: 0.0, state: start });
        let mut finish: Option<(f32, usize)> = None;
        while let Some(RouteOpen { cost, state }) = open.pop() {
            if done[state] {
                continue;
            }
            if finish.is_some_and(|(total, _)| total <= cost) {
                break;
            }
            done[state] = true;
            let (surface, at) = arrival(state);
            if surface == to {
                let total = cost + (goal_along - at).abs() / self.run_speed;
                if finish.is_none_or(|(known, _)| total < known) {
                    finish = Some((total, state));
                }
            }
            for index in &self.out[surface] {
                let link = &self.links[*index];
                let total = cost + (self.ground.frame.along(link.leg.start) - at).abs() / self.run_speed + link.cost;
                if !done[*index] && best[*index].is_none_or(|(known, _)| total < known) {
                    best[*index] = Some((total, state));
                    open.push(RouteOpen { cost: total, state: *index });
                }
            }
        }
        let (_, mut state) = finish?;
        let mut route = Vec::new();
        while state != start {
            route.push(state);
            state = best[state]?.1;
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
            return if self.local_only { NavNext::Unknown } else { NavNext::Unreachable };
        };
        if from == to {
            return self.ground.walk_on(to, feet, goal);
        }
        if self.local_only {
            return self.direct_link(from, to).map_or(NavNext::Unknown, NavNext::Leg);
        }
        // The first leg of the cheapest route: of the links leaving the feet's
        // surface, the one whose walk to its takeoff, its own cost and its
        // cost on to the goal ([`Self::to_goal`]) are least; of two as cheap,
        // the lower link. Only the walk from the feet is asked each tick.
        let to_goal = self.to_goal(to, self.ground.frame.along(goal));
        let at = self.ground.frame.along(feet);
        let first = self.out[from]
            .iter()
            .filter(|link| to_goal[**link].is_finite())
            .map(|link| {
                let leg = &self.links[*link];
                ((self.ground.frame.along(leg.leg.start) - at).abs() / self.run_speed + leg.cost + to_goal[*link], *link)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        match first {
            Some((_, link)) => NavNext::Leg(self.links[link].leg),
            None => NavNext::Unreachable,
        }
    }

    /// Each link's cost from its landing to the goal `goal_along` on surface
    /// `to`: the walks to each next takeoff, the legs, and the walk from the
    /// last landing to the goal (`f32::INFINITY` when the goal is out of its
    /// reach). The lifted form of [`Self::route`]'s search: a link's landing
    /// is a fixed point, so every edge cost is fixed and one search from the
    /// goal answers every body and every start for that goal. Kept for the
    /// last [`DESTINATIONS_KEPT`] goals; a body walking to a still goal asks
    /// for the same one tick after tick.
    fn to_goal(&self, to: usize, goal_along: f32) -> std::sync::Arc<[f32]> {
        let key = (to, goal_along.to_bits());
        if let Ok(kept) = self.destinations.0.lock() {
            if let Some((_, table)) = kept.iter().find(|(known, _)| *known == key) {
                return table.clone();
            }
        }
        // Links arriving on each surface: the predecessors of a link are the
        // links that land where it leaves.
        let mut into = vec![Vec::new(); self.ground.surfaces.len()];
        for (index, link) in self.links.iter().enumerate() {
            into[link.to].push(index);
        }
        let mut cost = vec![f32::INFINITY; self.links.len()];
        let mut done = vec![false; self.links.len()];
        let mut open = std::collections::BinaryHeap::new();
        for &arrive in &into[to] {
            let walk = (goal_along - self.ground.frame.along(self.links[arrive].leg.land)).abs() / self.run_speed;
            if walk < cost[arrive] {
                cost[arrive] = walk;
                open.push(RouteOpen { cost: walk, state: arrive });
            }
        }
        while let Some(RouteOpen { cost: known, state }) = open.pop() {
            if done[state] {
                continue;
            }
            done[state] = true;
            let link = &self.links[state];
            let (start, onward) = (self.ground.frame.along(link.leg.start), known + link.cost);
            for &before in &into[link.from] {
                let total = (start - self.ground.frame.along(self.links[before].leg.land)).abs() / self.run_speed + onward;
                if !done[before] && total < cost[before] {
                    cost[before] = total;
                    open.push(RouteOpen { cost: total, state: before });
                }
            }
        }
        let table: std::sync::Arc<[f32]> = cost.into();
        if let Ok(mut kept) = self.destinations.0.lock() {
            if kept.len() >= DESTINATIONS_KEPT {
                kept.remove(0);
            }
            kept.push((key, table.clone()));
        }
        table
    }

    /// A place beside `point` that a body at `feet` can get to: on the nearest
    /// surface under `point` (a body's centre, or its feet), `beside` away
    /// from it along the surface, on the side the body comes from. `None`
    /// when no surface in reach is under the point within `depth`.
    pub fn place_beside(&self, feet: Vec2, point: Vec2, beside: f32, depth: f32) -> Option<Vec2> {
        let (from, to, place) = self.ground.approach(feet, point, beside, depth)?;
        self.reaches(from, to).then_some(place)
    }

    /// The nearest surface under `point` (a body's centre, or its feet),
    /// no more than `depth` below it.
    pub fn surface_under(&self, point: Vec2, depth: f32) -> Option<usize> {
        self.ground.surface_under(point, depth)
    }

    /// Up to [`NAV_WAYPOINTS`] points a body at `feet` can get to, one on each
    /// of some surfaces. `seed` chooses the surfaces and the place on each.
    ///
    /// The answer is a function of the feet's surface and the seed only, so
    /// it is chosen once for each pair and kept: a body asks every tick, and
    /// its seed holds while its errand does.
    pub fn waypoints(&self, feet: Vec2, seed: u64) -> ([Vec2; NAV_WAYPOINTS], u8) {
        let Some(from) = self.surface_at(feet) else {
            return ([Vec2::ZERO; NAV_WAYPOINTS], 0);
        };
        let key = (from, seed);
        if let Ok(kept) = self.waypoint_sets.0.lock() {
            if let Some((_, answer)) = kept.iter().find(|(known, _)| *known == key) {
                return *answer;
            }
        }
        let answer = self.choose_waypoints(from, seed);
        if let Ok(mut kept) = self.waypoint_sets.0.lock() {
            if kept.len() >= WAYPOINT_SETS_KEPT {
                kept.remove(0);
            }
            kept.push((key, answer));
        }
        answer
    }

    /// [`Self::waypoints`] for a body on surface `from`, chosen now.
    fn choose_waypoints(&self, from: usize, seed: u64) -> ([Vec2; NAV_WAYPOINTS], u8) {
        let mut points = [Vec2::ZERO; NAV_WAYPOINTS];
        let mut reachable = self.reach_from(from).to_vec();
        reachable.sort_by_key(|surface| mix(seed ^ (*surface as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)));
        reachable.truncate(NAV_WAYPOINTS);
        for (slot, surface) in reachable.iter().enumerate() {
            points[slot] = self.ground.waypoint(*surface, seed);
        }
        (points, reachable.len() as u8)
    }
}

/// The legs worth a rollout from surface `a` to surface `b`, as (along, below)
/// points. Few: each one costs a rollout.

/// Walks from `a` to `b` along a slope piece of a chain (the part between
/// two flats, or a flat and an end: [`chain_walks`]) with one end at the top
/// of each (points as (along, below) pairs, like [`proposals`]). An end
/// joins a surface when it is at the surface's height and over it, or past
/// one of its ends by no more than half a body: the bank path leaves the
/// lane's floor at its end, and the cart's ramp comes down onto the middle of
/// the workshop's floor.
fn slope_proposals(pieces: &[(Vec2, Vec2)], frame: NavFrame, a: &StandSurface, b: &StandSurface, half_width: f32) -> Vec<NavLeg> {
    let reach = half_width + SLOPE_END_REACH;
    let joins = |surface: &StandSurface, point: Vec2| {
        let (along, below) = (frame.along(point), frame.below(point));
        (below - surface.top).abs() <= SLOPE_END_REACH / 2.0
            && along >= surface.left - reach
            && along <= surface.right + reach
    };
    let mut legs = Vec::new();
    for &(first, last) in pieces {
        for (from, to) in [(first, last), (last, first)] {
            if !(joins(a, from) && joins(b, to)) {
                continue;
            }
            let direction = (frame.along(to) - frame.along(from)).signum();
            // A slope is walked onto from behind its start and off ahead of
            // its end: where it touches down inside a floor, the part of the
            // floor under it is not where it starts or lands
            // ([`split_at_touchdowns`]).
            let behind = |surface: &StandSurface, along: f32| {
                if direction > 0.0 { surface.left < along } else { surface.right > along }
            };
            let ahead = |surface: &StandSurface, along: f32| {
                if direction > 0.0 { surface.right > along } else { surface.left < along }
            };
            if !(behind(a, frame.along(from)) && ahead(b, frame.along(to))) {
                continue;
            }
            let start = Vec2::new(a.clamp(frame.along(from)), a.top);
            let onto = (half_width + 4.0).min(b.width() * 0.5);
            legs.push(NavLeg {
                kind: NavLegKind::Slope,
                start,
                takeoff: start,
                land: Vec2::new(b.clamp(frame.along(to) + direction * onto), b.top),
                land_span: [b.left - half_width, b.right + half_width],
            });
        }
    }
    legs
}

fn proposals(a: &StandSurface, b: &StandSurface, half_width: f32, hop: bool, drop: bool) -> Vec<NavLeg> {
    let mut legs = Vec::new();
    let leg = |kind, start: f32, takeoff: f32, land: f32| NavLeg {
        kind,
        start: Vec2::new(a.clamp(start), a.top),
        takeoff: Vec2::new(takeoff, a.top),
        land: Vec2::new(b.clamp(land), b.top),
        // What `surface_at` takes for `b`: its ends, each a half body wider.
        land_span: [b.left - half_width, b.right + half_width],
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
            // Down through a one-way `a`, where the two overlap.
            if a.one_way {
                let middle = (overlap.0 + overlap.1) * 0.5;
                legs.push(leg(NavLegKind::DropThrough, middle, middle, middle));
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

/// One kernel step of a rollout. `true` when the kernel asks for the body to
/// be reset: it touched a hazard or left the world.
/// Steps a flight probe may take to reach its top speed, and to stop.
const FLIGHT_PROBE_STEPS: usize = 240;

/// How `body` flies under `frame`, measured in the kernel in open air: its
/// speed at a full stick along the side axis, and how fast it sheds that speed
/// with the stick let go. `None` when the body does not take flight.
fn measure_flight(body: &BodyClusterScratch, frame: MotionFrame) -> Option<FlightLaw> {
    let dt = EnvelopeProbe::default().dt;
    let world = World::new("flight_probe", Vec2::splat(1.0e5), Vec2::ZERO, Vec::new());
    let mut body = body.clone();
    {
        let (model, mut clusters) = body.parts();
        ae::movement::transit_body(model, &mut clusters, Vec2::splat(5.0e4), ae::movement::TransitVelocity::Zero);
    }
    if !body.flight.fly_enabled {
        step(&mut body, &world, frame, dt, LegInput { fly_toggle: true, ..LegInput::default() });
    }
    if !body.flight.fly_enabled {
        return None;
    }
    let side = frame.side();
    let speed = |body: &BodyClusterScratch| body.kinematics.vel.dot(side);
    let mut top = 0.0_f32;
    for _ in 0..FLIGHT_PROBE_STEPS {
        step(&mut body, &world, frame, dt, LegInput { axis: 1.0, full_speed: true, ..LegInput::default() });
        let now = speed(&body);
        if now <= top + 0.01 && now > 0.0 {
            break;
        }
        top = top.max(now);
    }
    if top <= 1.0 {
        return None;
    }
    let mut stop_steps = FLIGHT_PROBE_STEPS;
    for index in 1..=FLIGHT_PROBE_STEPS {
        step(&mut body, &world, frame, dt, LegInput::default());
        if speed(&body) <= top * 0.05 {
            stop_steps = index;
            break;
        }
    }
    Some(FlightLaw { top_speed: top, braking: top * 0.95 / (stop_steps as f32 * dt) })
}

pub(super) fn step(body: &mut BodyClusterScratch, world: &World, frame: MotionFrame, dt: f32, input: ae::navigation::LegInput) -> bool {
    let input = InputState {
        // Up is toward -y in the body's local axes.
        axes: LocalAxes::new(input.axis, ae::navigation::leg_stick_y(input)),
        movement: ActionEdges::<MovementAction>::EMPTY
            .with(MovementAction::Jump, Edge { pressed: input.jump_pressed, held: input.jump_held, released: false })
            .with(MovementAction::FlyToggle, Edge { pressed: input.fly_toggle, held: input.fly_toggle, released: false })
            .with(MovementAction::Burst, Edge { pressed: input.dash_pressed, held: input.dash_pressed, released: false }),
        ..Default::default()
    };
    let facing_intent = input.local_axis().x;
    let (model, mut clusters) = body.parts();
    step_motion(
        model,
        &mut clusters,
        MotionStepContext {
            admission: None,
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
    )
    .events
    .reset
    .is_some()
}

#[cfg(test)]
mod tests;
