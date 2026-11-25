//! Estimates for ordinary jumps and drops from the measured motion envelope.

use ambition_platformer2d_core as ae;
use ae::collision_semantics::{body_on_support_side, is_full_collision_surface};
use ae::layered::Layered;
use ae::navigation::{NavLeg, NavLegKind};
use ae::{Aabb, AabbExt, AbilitySet, BlockKind, Vec2, World};

use super::envelope::{descending_crossing, ArcSample, TraversalEnvelope};
use super::graph::{BuildCost, RUN_UP};
use super::surfaces::{GroundMap, StandSurface};

const LANDING_MARGIN: f32 = 12.0;
const START_MARGIN: f32 = 4.0;
const CLEARANCE_STEPS: usize = 12;
const MAX_NEAR_SURFACES: usize = 64;

/// The room's supports and the body's empty-room movement samples.
pub(super) struct Approximate<'a> {
    world: &'a World,
    exits: &'a [Aabb],
    ground: &'a GroundMap,
    envelope: &'a TraversalEnvelope,
    abilities: AbilitySet,
    supports: Layered<Aabb>,
    reach: f32,
    apex: f32,
    depth: f32,
    moving_blocks: bool,
}

impl<'a> Approximate<'a> {
    pub fn new(
        world: &'a World,
        exits: &'a [Aabb],
        ground: &'a GroundMap,
        envelope: &'a TraversalEnvelope,
        abilities: AbilitySet,
    ) -> Self {
        let supports = ground.surfaces.iter().map(|span| Aabb {
            min: Vec2::new(span.left, span.top),
            max: Vec2::new(span.right, span.top),
        }).collect();
        let arcs = envelope.jump.iter().chain(envelope.drop.iter());
        let reach = arcs.clone().map(|sample| sample.lead.abs()).fold(0.0, f32::max);
        let apex = envelope.apex_rise();
        let depth = envelope.probe.max_drop;
        let moving_blocks = world.blocks.iter().any(|block| block.velocity != Vec2::ZERO);
        Self { world, exits, ground, envelope, abilities, supports, reach, apex, depth, moving_blocks }
    }

    pub fn links(&self, from: usize, work: &mut BuildCost) -> Vec<(usize, NavLeg, f32)> {
        if !self.abilities.move_horizontal || self.envelope.takeoff_speed <= 0.0 {
            return Vec::new();
        }
        let a = &self.ground.surfaces[from];
        let ranges = Aabb {
            min: Vec2::new(a.left - self.reach, a.top - self.apex),
            max: Vec2::new(a.right + self.reach, a.top + self.depth),
        };
        let mut nearby = self.supports.indexed_near(ranges);
        let metric = |span: &Aabb| (span.min.x - a.right).max(a.left - span.max.x).max(0.0) + (span.min.y - a.top).abs();
        nearby.retain(|(to, _)| *to != from);
        nearby.sort_by(|a, b| metric(a.1).total_cmp(&metric(b.1)).then(a.0.cmp(&b.0)));
        let mut found = Vec::new();
        for (to, _) in nearby.into_iter().take(MAX_NEAR_SURFACES) {
            let b = &self.ground.surfaces[to];
            work.candidate_pairs += 1;
            for direction in [-1.0, 1.0] {
                if self.abilities.jump {
                    self.jumps(a, b, to, direction, work, &mut found);
                }
                if b.top > a.top + 1.0 {
                    if let Some((leg, cost)) = self.drop(a, b, direction, work) {
                        found.push((to, leg, cost));
                    }
                }
            }
        }
        found
    }

    fn jumps(
        &self,
        a: &StandSurface,
        b: &StandSurface,
        to: usize,
        direction: f32,
        work: &mut BuildCost,
        found: &mut Vec<(usize, NavLeg, f32)>,
    ) {
        let rise = a.top - b.top;
        let Some(landing) = descending_crossing(&self.envelope.jump, rise) else {
            return;
        };
        let offset = direction * landing.lead;
        let lead_in = direction * RUN_UP;
        let margin = LANDING_MARGIN + self.envelope.takeoff_speed * self.envelope.probe.dt;
        let lo = (a.left + START_MARGIN + lead_in.max(0.0)).max(b.left + margin - offset);
        let hi = (a.right - START_MARGIN + lead_in.min(0.0)).min(b.right - margin - offset);
        if lo > hi {
            return;
        }
        let middle = (lo + hi) * 0.5;
        // Each takeoff has its own landing. Search can choose a leg by its arrival cost.
        let starts = if hi - lo > 24.0 { [Some(middle), Some(lo), Some(hi)] } else { [Some(middle), None, None] };
        for takeoff in starts.into_iter().flatten() {
            if self.clear_arc(takeoff - lead_in, takeoff, a.top, direction, &self.envelope.jump, landing, work) {
                let leg = self.leg(NavLegKind::Hop, a.top, b, takeoff - lead_in, takeoff, takeoff + offset);
                found.push((to, leg, self.time(landing, RUN_UP)));
            }
        }
    }

    fn drop(
        &self,
        a: &StandSurface,
        b: &StandSurface,
        direction: f32,
        work: &mut BuildCost,
    ) -> Option<(NavLeg, f32)> {
        let landing = descending_crossing(&self.envelope.drop, a.top - b.top)?;
        let lip = self.static_lip(a, direction)?;
        let takeoff = lip - direction * self.ground.half.x;
        let start = takeoff - direction * RUN_UP;
        let at = takeoff + direction * landing.lead;
        let margin = LANDING_MARGIN + self.envelope.takeoff_speed * self.envelope.probe.dt;
        if start < a.left || start > a.right || at < b.left + margin || at > b.right - margin {
            return None;
        }
        if !self.clear_arc(start, takeoff, a.top, direction, &self.envelope.drop, landing, work) {
            return None;
        }
        Some((self.leg(NavLegKind::Drop, a.top, b, start, takeoff, at), self.time(landing, RUN_UP)))
    }

    fn static_lip(&self, support: &StandSurface, direction: f32) -> Option<f32> {
        let frame = self.ground.frame;
        let end = if direction > 0.0 { support.right } else { support.left };
        let foot = frame.point(end, support.top);
        let near = Aabb::new(foot, Vec2::splat(4.0));
        self.world.blocks.near(near).into_iter()
            .filter(|block| ae::collision_semantics::is_support_surface(block.kind) && block.velocity == Vec2::ZERO)
            .filter_map(|block| {
                let low = block.aabb.min;
                let high = block.aabb.max;
                let top = frame.below(low).min(frame.below(high));
                let edge = (frame.along(low) * direction).max(frame.along(high) * direction) * direction;
                ((top - support.top).abs() <= 0.5 && (edge - end).abs() <= 3.0).then_some(edge)
            })
            .min_by(|a, b| (a - end).abs().total_cmp(&(b - end).abs()))
    }

    fn clear_arc(
        &self,
        start: f32,
        takeoff: f32,
        top: f32,
        direction: f32,
        arc: &[ArcSample],
        landing: ArcSample,
        work: &mut BuildCost,
    ) -> bool {
        let last = usize::from(landing.step);
        let stride = last.div_ceil(CLEARANCE_STEPS).max(1);
        let before_landing = &arc[..last.saturating_sub(1)];
        // Keep the last level step of a walk-off. A chord that cuts this
        // corner can enter the floor before the body clears the edge.
        let crest = before_landing.iter().enumerate()
            .max_by(|a, b| a.1.rise.total_cmp(&b.1.rise))
            .map(|(index, _)| index);
        let frame = self.ground.frame;
        let points = before_landing.iter().enumerate()
            .filter(|(index, _)| (index + 1) % stride == 0
                || crest.is_some_and(|crest| crest == *index || crest + 1 == *index))
            .map(|(_, sample)| *sample)
            .chain(std::iter::once(landing))
            .map(|sample| frame.point(takeoff + direction * sample.lead, top - sample.rise));
        let first = frame.point(start, top);
        let points = std::iter::once(frame.point(takeoff, top)).chain(points);
        let start_box = self.body_at(first);
        let extent = points.clone().fold(start_box, |bounds, feet| {
            let body = self.body_at(feet);
            Aabb { min: bounds.min.min(body.min), max: bounds.max.max(body.max) }
        });
        if extent.min.cmplt(Vec2::ZERO).any() || extent.max.cmpgt(self.world.size).any() {
            return false;
        }
        // Query the room index once for the runway and all chords.
        let broad = Aabb { min: extent.min - Vec2::ONE, max: extent.max + Vec2::ONE };
        work.clearance_queries += 1;
        let blocks = self.world.blocks.near(broad);
        let chains_near = !self.world.chains.swept_near(extent).is_empty();
        let moving_blocks_near = self.moving_blocks
            && self.world.blocks.swept_near(extent).iter().any(|block| block.velocity != Vec2::ZERO);
        let mut from = first;
        for to in points {
            work.clearance_sweeps += 1;
            if !self.clear_segment(from, to, &blocks, chains_near, moving_blocks_near) {
                return false;
            }
            from = to;
        }
        true
    }

    fn body_at(&self, feet: Vec2) -> Aabb {
        let frame = self.ground.frame;
        let half = self.ground.half;
        let axes = frame.side.abs() * half.x + frame.down.abs() * half.y;
        // Contact at the start and at the landing is permitted.
        let relief = 0.5_f32.min(half.y * 0.125);
        Aabb::new(feet - frame.down * (half.y + relief * 0.5), axes - frame.down.abs() * relief)
    }

    fn clear_segment(
        &self,
        from: Vec2,
        to: Vec2,
        blocks: &[&ae::Block],
        chains_near: bool,
        moving_blocks_near: bool,
    ) -> bool {
        let down = self.ground.frame.down;
        let body = self.body_at(from);
        let delta = to - from;
        let swept = Aabb { min: body.min.min(body.min + delta), max: body.max.max(body.max + delta) };
        if (chains_near && !self.world.chains.swept_near(swept).is_empty())
            || (moving_blocks_near
                && self.world.blocks.swept_near(swept).iter().any(|block| block.velocity != Vec2::ZERO))
            || self.world.water_regions.iter().any(|water| swept.strict_intersects(water.aabb))
            || self.exits.iter().any(|exit| swept.strict_intersects(*exit))
        {
            return false;
        }
        let min = swept.min - Vec2::ONE;
        let max = swept.max + Vec2::ONE;
        !blocks.iter().any(|block| {
            !min.cmpgt(block.aabb.max).any() && !max.cmplt(block.aabb.min).any()
                && (is_full_collision_surface(block.kind)
                    || block.kind == BlockKind::Hazard
                    || (block.kind == BlockKind::OneWay && delta.dot(down) > 0.0
                        && body_on_support_side(body, block.aabb, down)))
                && body.sweep_hit(delta, block.aabb).is_some()
        })
    }

    fn leg(&self, kind: NavLegKind, top: f32, b: &StandSurface, start: f32, takeoff: f32, at: f32) -> NavLeg {
        let frame = self.ground.frame;
        NavLeg {
            kind,
            start: frame.point(start, top),
            takeoff: frame.point(takeoff, top),
            land: frame.point(at, b.top),
            land_span: [b.left, b.right],
        }
    }

    fn time(&self, landing: ArcSample, run: f32) -> f32 {
        f32::from(landing.step) * self.envelope.probe.dt + run / self.envelope.takeoff_speed.max(1.0)
    }
}
