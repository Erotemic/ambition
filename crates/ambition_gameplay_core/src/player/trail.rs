//! Player-emitted **trail** mechanics.
//!
//! The trail is not a grapple/pull rope. It is the topological breadcrumb the
//! player chooses to emit: while emission is ON, the trail samples the player's
//! path in world space and grows behind them. Existing samples remain where they
//! were placed instead of being dragged around by the player. When emission stops,
//! a trail whose endpoint returns near its start closes into a cycle; otherwise it
//! collapses away as an unfinished chain.
//!
//! This module keeps the simulation deterministic and mostly presentation-free:
//! the pure helpers decide how a path is sampled, chunked across topological
//! continuity breaks, closed, or collapsed, while the Bevy systems only attach it
//! to the primary player and draw the resulting debug polylines. Future
//! homology/cohomology spells should consume [`PlayerTrail`] cycles rather than
//! raw input gestures.

use crate::engine_core as ae;
use bevy::prelude::*;

/// Minimum distance the player must move before the trail records another fixed
/// sample. The live endpoint still follows the player every frame while emitting;
/// this only controls how dense the preserved breadcrumb polyline is.
pub const TRAIL_SAMPLE_SPACING: f32 = 10.0;

/// Endpoint distance from the start point required to close an emitted trail into
/// a cycle when the player stops emitting.
pub const TRAIL_CLOSE_RADIUS: f32 = 24.0;

/// A tiny wiggle near the start point should not count as a spell cycle even if
/// its endpoint is technically close to its start.
pub const TRAIL_MIN_CYCLE_LENGTH: f32 = 72.0;

/// Minimum number of recorded samples, including the final endpoint, before a
/// loop can close into a cycle.
pub const TRAIL_MIN_CYCLE_POINTS: usize = 6;

/// Safety cap for runaway trails. At the default sample spacing this is still a
/// very long line for a platformer room, but it prevents accidental unbounded
/// memory growth during long dev sessions.
pub const TRAIL_MAX_POINTS: usize = 4096;

/// Duration of the collapse animation for unfinished trails.
pub const TRAIL_COLLAPSE_SECONDS: f32 = 0.22;

/// Duration of the local shrink animation when a self-crossing cuts off a
/// currently-trivial loop.
pub const TRAIL_SELF_LOOP_COLLAPSE_SECONDS: f32 = 0.30;

const TRAIL_MIN_SELF_LOOP_LENGTH: f32 = 18.0;
const TRAIL_INTERSECTION_EPSILON: f32 = 0.001;

/// Current global emission switch for the primary player.
///
/// `enabled == true` means the player is currently emitting a trail. It does not
/// mean an already-closed trail should be hidden: closed cycles remain visible
/// after emission is turned off.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct PlayerTrailEnabled {
    pub enabled: bool,
}

/// Neutral path-continuity message for systems that move a body through a
/// discontinuity in ordinary world coordinates while preserving continuity in the
/// game's topological space.
///
/// Portal adapters, room-wrap mechanics, scripted quotient seams, or debug
/// topological teleports can all emit this without making the trail module depend
/// on any one mechanic. The trail responds by ending the current Euclidean chunk
/// and starting a new chunk at `resume_at`; renderers must not draw a straight
/// world-space segment across the break.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct TrailContinuityBreak {
    pub body: Entity,
    pub resume_at: ae::Vec2,
}

/// One sampled point on the emitted trail.
///
/// `prev` is kept for compatibility with earlier rope/debug code and future
/// smoothing/collision work, but emitted trail samples are intentionally pinned in
/// world space, so it is normally equal to `pos`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrailPoint {
    pub pos: ae::Vec2,
    pub prev: ae::Vec2,
}

impl TrailPoint {
    fn pinned(pos: ae::Vec2) -> Self {
        Self { pos, prev: pos }
    }
}

/// One Euclidean chart segment of the emitted trail.
///
/// A trail can span multiple chunks when some other mechanic reports a
/// [`TrailContinuityBreak`]. Chunks are connected in the topological path, but
/// not by ordinary world-space line segments.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrailChunk {
    pub points: Vec<TrailPoint>,
}

impl TrailChunk {
    fn starting_at(pos: ae::Vec2) -> Self {
        Self {
            points: vec![TrailPoint::pinned(pos)],
        }
    }

    fn from_positions(positions: impl IntoIterator<Item = ae::Vec2>) -> Self {
        let mut points = Vec::new();
        for pos in positions {
            if points
                .last()
                .map_or(true, |last: &TrailPoint| last.pos.distance(pos) > 0.5)
            {
                points.push(TrailPoint::pinned(pos));
            }
        }
        Self { points }
    }
}

/// Local loop being erased after the active trail self-intersected.
#[derive(Clone, Debug, PartialEq)]
pub struct CollapsingTrailLoop {
    pub points: Vec<TrailPoint>,
    pub target: ae::Vec2,
    pub elapsed: f32,
}

impl CollapsingTrailLoop {
    fn new(loop_positions: Vec<ae::Vec2>) -> Self {
        let target = centroid_positions(&loop_positions).unwrap_or(ae::Vec2::ZERO);
        Self {
            points: loop_positions.into_iter().map(TrailPoint::pinned).collect(),
            target,
            elapsed: 0.0,
        }
    }

    fn step(&mut self, dt: f32) -> bool {
        let next_elapsed = self.elapsed + dt.max(0.0);
        let finished = next_elapsed >= TRAIL_SELF_LOOP_COLLAPSE_SECONDS;
        let t = if TRAIL_SELF_LOOP_COLLAPSE_SECONDS <= 0.0 {
            1.0
        } else {
            (dt.max(0.0) / TRAIL_SELF_LOOP_COLLAPSE_SECONDS).clamp(0.0, 1.0)
        };
        for p in &mut self.points {
            p.pos = p.pos.lerp(self.target, t);
            p.prev = p.pos;
        }
        self.elapsed = next_elapsed;
        finished
    }

    fn render_points(&self) -> Vec<ae::Vec2> {
        self.points.iter().map(|p| p.pos).collect()
    }
}

/// Lifecycle state of the player's current trail.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrailStatus {
    /// Emission is active. `chunks` are fixed samples; `live_end` follows the
    /// player until the next sample is committed.
    Emitting,
    /// Emission stopped near the start point, so the trail has become a closed
    /// cycle. This is the state future homology spell code should inspect.
    Closed,
    /// Emission stopped with loose endpoints. The open chain has no stable loop
    /// charge, so it is shrinking to a point and will be removed.
    Collapsing { elapsed: f32 },
}

impl Default for TrailStatus {
    fn default() -> Self {
        Self::Emitting
    }
}

/// The player's emitted trail.
///
/// `chunks` stores fixed world-space samples. While [`TrailStatus::Emitting`],
/// `live_end` is the current unsampled endpoint attached to the player's body.
#[derive(Component, Clone, Debug)]
pub struct PlayerTrail {
    pub chunks: Vec<TrailChunk>,
    pub live_end: ae::Vec2,
    pub status: TrailStatus,
    pub collapsing_loops: Vec<CollapsingTrailLoop>,
}

impl PlayerTrail {
    /// Start a fresh emitted trail at `anchor`.
    pub fn emitting_from(anchor: ae::Vec2) -> Self {
        Self {
            chunks: vec![TrailChunk::starting_at(anchor)],
            live_end: anchor,
            status: TrailStatus::Emitting,
            collapsing_loops: Vec::new(),
        }
    }

    /// Whether this trail has closed into a stable cycle.
    pub fn is_closed_cycle(&self) -> bool {
        matches!(self.status, TrailStatus::Closed)
    }

    /// Append fixed samples between the previous fixed sample and `anchor` while
    /// keeping the live endpoint on the player. Existing samples are not moved.
    pub fn emit_to(&mut self, anchor: ae::Vec2) {
        if !matches!(self.status, TrailStatus::Emitting) {
            return;
        }
        self.ensure_current_chunk(anchor);

        let old_live_end = self.live_end;
        self.try_extract_trivial_self_crossing(old_live_end, anchor);
        self.append_fixed_samples_to(anchor);
        self.live_end = anchor;
    }

    /// Continue the topological path across a coordinate discontinuity without
    /// drawing a fake Euclidean segment between the two sides.
    pub fn continue_after_break(&mut self, resume_at: ae::Vec2) {
        if !matches!(self.status, TrailStatus::Emitting) {
            return;
        }
        self.ensure_current_chunk(self.live_end);
        self.push_endpoint_if_distinct(self.live_end);
        self.chunks.push(TrailChunk::starting_at(resume_at));
        self.live_end = resume_at;
    }

    /// Stop emission. Near-returning paths become closed cycles; open paths start
    /// collapsing toward their centroid.
    pub fn finish_emission(&mut self, final_anchor: ae::Vec2) {
        if !matches!(self.status, TrailStatus::Emitting) {
            return;
        }
        self.live_end = final_anchor;
        self.push_endpoint_if_distinct(final_anchor);

        if self.can_close(final_anchor) {
            let start = self.start_point().expect("can_close requires a start point");
            let last_pos = self
                .chunks
                .last()
                .and_then(|chunk| chunk.points.last())
                .map(|p| p.pos);
            match last_pos {
                Some(pos) if pos.distance(start) <= 1.0 => {
                    if let Some(last) = self
                        .current_chunk_mut()
                        .and_then(|chunk| chunk.points.last_mut())
                    {
                        *last = TrailPoint::pinned(start);
                    }
                }
                Some(_) if self.total_point_count() < TRAIL_MAX_POINTS => {
                    self.current_chunk_mut()
                        .expect("current chunk exists")
                        .points
                        .push(TrailPoint::pinned(start));
                }
                _ => {}
            }
            self.live_end = start;
            self.status = TrailStatus::Closed;
        } else {
            self.status = TrailStatus::Collapsing { elapsed: 0.0 };
        }
    }

    /// Advance any local self-loop erasures currently animating.
    pub fn collapse_self_loops_step(&mut self, dt: f32) {
        for loop_ in &mut self.collapsing_loops {
            loop_.step(dt);
        }
        self.collapsing_loops
            .retain(|loop_| loop_.elapsed < TRAIL_SELF_LOOP_COLLAPSE_SECONDS);
    }

    /// Advance the collapse animation for the whole unfinished trail. Returns
    /// `true` once the component can be removed.
    pub fn collapse_step(&mut self, dt: f32) -> bool {
        let TrailStatus::Collapsing { elapsed } = self.status else {
            return false;
        };
        let next_elapsed = elapsed + dt.max(0.0);
        let finished = next_elapsed >= TRAIL_COLLAPSE_SECONDS;
        let center = self.centroid();
        let t = if TRAIL_COLLAPSE_SECONDS <= 0.0 {
            1.0
        } else {
            (dt.max(0.0) / TRAIL_COLLAPSE_SECONDS).clamp(0.0, 1.0)
        };
        for chunk in &mut self.chunks {
            for p in &mut chunk.points {
                p.pos = p.pos.lerp(center, t);
                p.prev = p.pos;
            }
        }
        self.live_end = self.live_end.lerp(center, t);
        self.status = TrailStatus::Collapsing {
            elapsed: next_elapsed,
        };
        finished
    }

    /// Active chunk polylines to draw this frame, including the live endpoint
    /// while emitting. Continuity breaks produce multiple polylines so renderers
    /// do not draw fake teleport segments.
    pub fn render_polylines(&self) -> Vec<Vec<ae::Vec2>> {
        let mut out: Vec<Vec<ae::Vec2>> = self
            .chunks
            .iter()
            .map(|chunk| chunk.points.iter().map(|p| p.pos).collect())
            .collect();
        if matches!(self.status, TrailStatus::Emitting) {
            if let Some(last) = out.last_mut() {
                if last
                    .last()
                    .map_or(true, |last_pos| last_pos.distance(self.live_end) > 0.5)
                {
                    last.push(self.live_end);
                }
            }
        }
        out
    }

    /// Legacy convenience for tests/debuggers that only need a flattened view of
    /// fixed points. Prefer [`PlayerTrail::render_polylines`] when drawing.
    pub fn render_points(&self) -> Vec<ae::Vec2> {
        self.render_polylines().into_iter().flatten().collect()
    }

    /// Local self-crossing loops that are currently shrinking away.
    pub fn collapsing_loop_polylines(&self) -> Vec<Vec<ae::Vec2>> {
        self.collapsing_loops
            .iter()
            .map(CollapsingTrailLoop::render_points)
            .collect()
    }

    fn ensure_current_chunk(&mut self, anchor: ae::Vec2) {
        if self.chunks.is_empty() {
            self.chunks.push(TrailChunk::starting_at(anchor));
            self.live_end = anchor;
            return;
        }
        if self.chunks.last().is_some_and(|chunk| chunk.points.is_empty()) {
            self.current_chunk_mut()
                .expect("current chunk exists")
                .points
                .push(TrailPoint::pinned(anchor));
            self.live_end = anchor;
        }
    }

    fn append_fixed_samples_to(&mut self, anchor: ae::Vec2) {
        let Some(last_pos) = self
            .chunks
            .last()
            .and_then(|chunk| chunk.points.last())
            .map(|p| p.pos)
        else {
            self.chunks.push(TrailChunk::starting_at(anchor));
            return;
        };

        let mut total_points = self.total_point_count();
        let mut last = last_pos;
        let mut delta = anchor - last;
        let mut dist = delta.length();
        let mut new_points = Vec::new();
        while dist >= TRAIL_SAMPLE_SPACING && total_points < TRAIL_MAX_POINTS {
            let dir = delta / dist;
            last += dir * TRAIL_SAMPLE_SPACING;
            new_points.push(TrailPoint::pinned(last));
            total_points += 1;
            delta = anchor - last;
            dist = delta.length();
        }
        if let Some(chunk) = self.current_chunk_mut() {
            chunk.points.extend(new_points);
        }
    }

    fn push_endpoint_if_distinct(&mut self, endpoint: ae::Vec2) {
        self.ensure_current_chunk(endpoint);
        if self
            .chunks
            .last()
            .and_then(|chunk| chunk.points.last())
            .is_some_and(|last| last.pos.distance(endpoint) > 0.5)
            && self.total_point_count() < TRAIL_MAX_POINTS
        {
            self.current_chunk_mut()
                .expect("current chunk exists")
                .points
                .push(TrailPoint::pinned(endpoint));
        }
    }

    fn can_close(&self, endpoint: ae::Vec2) -> bool {
        let Some(start) = self.start_point() else {
            return false;
        };
        self.total_point_count() >= TRAIL_MIN_CYCLE_POINTS
            && self.path_length() >= TRAIL_MIN_CYCLE_LENGTH
            && endpoint.distance(start) <= TRAIL_CLOSE_RADIUS
    }

    fn path_length(&self) -> f32 {
        self.chunks
            .iter()
            .flat_map(|chunk| chunk.points.windows(2))
            .map(|w| w[0].pos.distance(w[1].pos))
            .sum()
    }

    fn centroid(&self) -> ae::Vec2 {
        let positions: Vec<ae::Vec2> = self
            .chunks
            .iter()
            .flat_map(|chunk| chunk.points.iter().map(|p| p.pos))
            .chain(std::iter::once(self.live_end))
            .collect();
        centroid_positions(&positions).unwrap_or(self.live_end)
    }

    fn start_point(&self) -> Option<ae::Vec2> {
        self.chunks
            .iter()
            .find_map(|chunk| chunk.points.first().map(|p| p.pos))
    }

    fn total_point_count(&self) -> usize {
        self.chunks.iter().map(|chunk| chunk.points.len()).sum()
    }

    fn current_chunk_mut(&mut self) -> Option<&mut TrailChunk> {
        self.chunks.last_mut()
    }

    fn try_extract_trivial_self_crossing(&mut self, old_live_end: ae::Vec2, anchor: ae::Vec2) {
        if old_live_end.distance(anchor) <= TRAIL_INTERSECTION_EPSILON {
            return;
        }
        let Some(chunk) = self.chunks.last() else {
            return;
        };
        if chunk.points.len() < 3 {
            return;
        }

        let mut polyline: Vec<ae::Vec2> = chunk.points.iter().map(|p| p.pos).collect();
        if polyline
            .last()
            .map_or(true, |last| last.distance(old_live_end) > 0.5)
        {
            polyline.push(old_live_end);
        }
        if polyline.len() < 4 {
            return;
        }

        let mut best: Option<(usize, ae::Vec2, f32)> = None;
        // The final polyline segment ends at `old_live_end` and is adjacent to
        // the candidate segment; skip it so ordinary forward motion does not
        // count as self-intersection.
        for i in 0..polyline.len().saturating_sub(2) {
            let a = polyline[i];
            let b = polyline[i + 1];
            let Some((hit, _along_existing, along_candidate)) =
                segment_intersection(a, b, old_live_end, anchor)
            else {
                continue;
            };
            if along_candidate <= TRAIL_INTERSECTION_EPSILON {
                continue;
            }
            if best.map_or(true, |(_, _, best_along)| along_candidate < best_along) {
                best = Some((i, hit, along_candidate));
            }
        }

        let Some((hit_segment, hit, _along_candidate)) = best else {
            return;
        };

        let mut loop_positions = Vec::new();
        loop_positions.push(hit);
        loop_positions.extend(polyline.iter().skip(hit_segment + 1).copied());
        if loop_positions
            .last()
            .map_or(true, |last| last.distance(old_live_end) > 0.5)
        {
            loop_positions.push(old_live_end);
        }
        loop_positions.push(hit);
        if polyline_length(&loop_positions) < TRAIL_MIN_SELF_LOOP_LENGTH {
            return;
        }

        let mut prefix: Vec<ae::Vec2> = polyline.iter().take(hit_segment + 1).copied().collect();
        prefix.push(hit);
        if let Some(chunk) = self.current_chunk_mut() {
            *chunk = TrailChunk::from_positions(prefix);
        }
        self.live_end = hit;
        self.collapsing_loops
            .push(CollapsingTrailLoop::new(loop_positions));
    }
}

/// Give the primary player an emitting trail if emission is already enabled but
/// the player does not have a trail component yet. This covers spawn/reset cases
/// and headless tests that flip [`PlayerTrailEnabled`] directly.
pub fn ensure_player_trail(
    mut commands: Commands,
    enabled: Option<Res<PlayerTrailEnabled>>,
    players: Query<
        (Entity, &crate::player::BodyKinematics),
        (
            With<crate::player::PlayerEntity>,
            With<crate::player::PrimaryPlayer>,
            Without<PlayerTrail>,
        ),
    >,
) {
    let emission_enabled = enabled.as_ref().is_some_and(|enabled| enabled.enabled);
    if !emission_enabled {
        return;
    }
    for (entity, kin) in &players {
        commands
            .entity(entity)
            .insert(PlayerTrail::emitting_from(trail_anchor(kin)));
    }
}

/// Advance the player's emitted trail: append samples while emission is active,
/// split chunks across topological continuity breaks, or animate unfinished
/// trails as they collapse away.
pub fn update_player_trail(
    world_time: Res<crate::WorldTime>,
    enabled: Option<Res<PlayerTrailEnabled>>,
    mut continuity_breaks: MessageReader<TrailContinuityBreak>,
    mut commands: Commands,
    mut players: Query<
        (Entity, &crate::player::BodyKinematics, &mut PlayerTrail),
        With<crate::player::PlayerEntity>,
    >,
) {
    let dt = world_time.sim_dt();
    let emission_enabled = enabled.as_ref().is_some_and(|enabled| enabled.enabled);
    let breaks: Vec<TrailContinuityBreak> = continuity_breaks.read().copied().collect();
    for (entity, kin, mut trail) in &mut players {
        trail.collapse_self_loops_step(dt);
        let anchor = trail_anchor(kin);
        match trail.status {
            TrailStatus::Emitting => {
                if emission_enabled {
                    for ev in breaks.iter().filter(|ev| ev.body == entity) {
                        trail.continue_after_break(ev.resume_at);
                    }
                    trail.emit_to(anchor);
                } else {
                    trail.finish_emission(anchor);
                }
            }
            TrailStatus::Closed => {
                if emission_enabled {
                    *trail = PlayerTrail::emitting_from(anchor);
                }
            }
            TrailStatus::Collapsing { .. } => {
                if emission_enabled {
                    *trail = PlayerTrail::emitting_from(anchor);
                } else if trail.collapse_step(dt) {
                    commands.entity(entity).remove::<PlayerTrail>();
                }
            }
        }
    }
}

/// The trail's sampling point: the player's centre. Keeping this as a helper lets
/// future work move the emission point to a hand/hip socket without touching the
/// trail state machine.
fn trail_anchor(kin: &crate::player::BodyKinematics) -> ae::Vec2 {
    use crate::engine_core::AabbExt;
    kin.aabb().center()
}

const TRAIL_EMITTING_COLOR: Color = Color::srgb(0.62, 0.47, 0.30);
const TRAIL_CLOSED_COLOR: Color = Color::srgb(0.95, 0.78, 0.32);
const TRAIL_COLLAPSING_COLOR: Color = Color::srgb(0.40, 0.32, 0.26);
const TRAIL_SELF_LOOP_COLLAPSING_COLOR: Color = Color::srgb(0.80, 0.56, 0.36);

/// Draw each trail as gizmo linestrips in Bevy world space. Closed cycles remain
/// visible even after emission has been turned off. Chunks separated by
/// continuity breaks are drawn independently, so a portal transit never appears
/// as a long straight line across the room.
pub fn render_player_trail(
    world: Option<Res<crate::GameWorld>>,
    ropes: Query<&PlayerTrail, With<crate::player::PlayerEntity>>,
    mut gizmos: Gizmos,
) {
    let Some(world) = world.as_deref() else {
        return;
    };
    let z = crate::config::WORLD_Z_PLAYER - 0.1;
    for rope in &ropes {
        let color = match rope.status {
            TrailStatus::Emitting => TRAIL_EMITTING_COLOR,
            TrailStatus::Closed => TRAIL_CLOSED_COLOR,
            TrailStatus::Collapsing { .. } => TRAIL_COLLAPSING_COLOR,
        };
        for pts in rope.render_polylines() {
            if pts.len() < 2 {
                continue;
            }
            let bevy_pts = pts
                .into_iter()
                .map(|p| crate::config::world_to_bevy(&world.0, p, z).truncate());
            gizmos.linestrip_2d(bevy_pts, color);
        }
        for pts in rope.collapsing_loop_polylines() {
            if pts.len() < 2 {
                continue;
            }
            let bevy_pts = pts
                .into_iter()
                .map(|p| crate::config::world_to_bevy(&world.0, p, z).truncate());
            gizmos.linestrip_2d(bevy_pts, TRAIL_SELF_LOOP_COLLAPSING_COLOR);
        }
    }
}

/// Trail plugin: registers the emission state, continuity-break message,
/// sampling/update, and debug-line rendering. Input systems may toggle
/// [`PlayerTrailEnabled`], but this module does not know which physical key or
/// device action caused that change; portal adapters may emit
/// [`TrailContinuityBreak`], but this module does not depend on the portal crate.
pub struct PlayerTrailPlugin;

impl Plugin for PlayerTrailPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerTrailEnabled>();
        app.add_message::<TrailContinuityBreak>();
        app.add_systems(
            Update,
            (ensure_player_trail, update_player_trail)
                .chain()
                .in_set(crate::schedule::SandboxSet::PresentationSync),
        );
        app.add_systems(
            Update,
            render_player_trail.run_if(resource_exists::<bevy::gizmos::config::GizmoConfigStore>),
        );
    }
}

fn centroid_positions(points: &[ae::Vec2]) -> Option<ae::Vec2> {
    if points.is_empty() {
        return None;
    }
    let sum = points
        .iter()
        .copied()
        .fold(ae::Vec2::ZERO, |acc, p| acc + p);
    Some(sum / points.len() as f32)
}

fn polyline_length(points: &[ae::Vec2]) -> f32 {
    points.windows(2).map(|w| w[0].distance(w[1])).sum()
}

fn segment_intersection(
    a: ae::Vec2,
    b: ae::Vec2,
    c: ae::Vec2,
    d: ae::Vec2,
) -> Option<(ae::Vec2, f32, f32)> {
    let r = b - a;
    let s = d - c;
    let denom = cross(r, s);
    if denom.abs() <= TRAIL_INTERSECTION_EPSILON {
        return None;
    }
    let c_minus_a = c - a;
    let along_existing = cross(c_minus_a, s) / denom;
    let along_candidate = cross(c_minus_a, r) / denom;
    if !(TRAIL_INTERSECTION_EPSILON..=(1.0 - TRAIL_INTERSECTION_EPSILON))
        .contains(&along_existing)
    {
        return None;
    }
    if !(TRAIL_INTERSECTION_EPSILON..=1.0).contains(&along_candidate) {
        return None;
    }
    Some((a + r * along_existing, along_existing, along_candidate))
}

fn cross(a: ae::Vec2, b: ae::Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f32, y: f32) -> ae::Vec2 {
        ae::Vec2::new(x, y)
    }

    #[test]
    fn trail_emission_is_disabled_by_default() {
        assert!(
            !PlayerTrailEnabled::default().enabled,
            "the trail should start inactive until the player toggles emission",
        );
    }

    #[test]
    fn emitting_trail_samples_movement_but_keeps_old_points_fixed() {
        let mut trail = PlayerTrail::emitting_from(v(0.0, 0.0));
        trail.emit_to(v(TRAIL_SAMPLE_SPACING * 2.5, 0.0));

        assert_eq!(trail.chunks[0].points[0].pos, v(0.0, 0.0));
        assert!(
            trail.chunks[0].points.len() >= 3,
            "moving several sample spacings should commit breadcrumb samples",
        );
        assert_eq!(trail.live_end, v(TRAIL_SAMPLE_SPACING * 2.5, 0.0));
    }

    #[test]
    fn near_returning_trail_finishes_as_closed_cycle() {
        let mut trail = PlayerTrail::emitting_from(v(0.0, 0.0));
        for p in [
            v(30.0, 0.0),
            v(60.0, 0.0),
            v(60.0, 30.0),
            v(30.0, 60.0),
            v(0.0, 60.0),
            v(0.0, 20.0),
            v(3.0, 2.0),
        ] {
            trail.emit_to(p);
        }
        trail.finish_emission(v(3.0, 2.0));

        assert!(trail.is_closed_cycle());
        let pts = trail.render_points();
        assert_eq!(pts.first().copied().unwrap(), pts.last().copied().unwrap());
    }

    #[test]
    fn far_endpoint_finishes_as_collapsing_open_chain() {
        let mut trail = PlayerTrail::emitting_from(v(0.0, 0.0));
        for p in [v(40.0, 0.0), v(80.0, 0.0), v(120.0, 0.0)] {
            trail.emit_to(p);
        }
        trail.finish_emission(v(120.0, 0.0));

        assert!(matches!(trail.status, TrailStatus::Collapsing { .. }));
    }

    #[test]
    fn tiny_near_start_wiggle_does_not_form_a_cycle() {
        let mut trail = PlayerTrail::emitting_from(v(0.0, 0.0));
        trail.emit_to(v(5.0, 0.0));
        trail.emit_to(v(2.0, 1.0));
        trail.finish_emission(v(2.0, 1.0));

        assert!(matches!(trail.status, TrailStatus::Collapsing { .. }));
    }

    #[test]
    fn collapse_step_eventually_removes_open_chain() {
        let mut trail = PlayerTrail::emitting_from(v(0.0, 0.0));
        trail.emit_to(v(100.0, 0.0));
        trail.finish_emission(v(100.0, 0.0));

        assert!(!trail.collapse_step(TRAIL_COLLAPSE_SECONDS * 0.5));
        assert!(trail.collapse_step(TRAIL_COLLAPSE_SECONDS));
    }

    #[test]
    fn continuity_break_starts_a_new_render_chunk() {
        let mut trail = PlayerTrail::emitting_from(v(0.0, 0.0));
        trail.emit_to(v(40.0, 0.0));
        trail.continue_after_break(v(200.0, 100.0));
        trail.emit_to(v(230.0, 100.0));

        let polylines = trail.render_polylines();
        assert_eq!(polylines.len(), 2, "portal-like breaks must not draw one line");
        assert_eq!(polylines[0].last().copied().unwrap(), v(40.0, 0.0));
        assert_eq!(polylines[1].first().copied().unwrap(), v(200.0, 100.0));
    }

    #[test]
    fn self_crossing_cuts_off_a_trivial_loop_to_collapse() {
        let mut trail = PlayerTrail::emitting_from(v(0.0, 0.0));
        trail.emit_to(v(50.0, 0.0));
        trail.emit_to(v(50.0, 50.0));
        trail.emit_to(v(0.0, 50.0));
        trail.emit_to(v(20.0, -10.0));

        assert_eq!(trail.collapsing_loops.len(), 1);
        assert!(
            trail.chunks[0]
                .points
                .iter()
                .all(|p| p.pos.y <= 1.0 || p.pos.x <= 1.0),
            "the square-like trivial lobe should be removed from the active trail",
        );
        assert!(!trail.collapsing_loops[0].step(TRAIL_SELF_LOOP_COLLAPSE_SECONDS * 0.5));
        assert!(trail.collapsing_loops[0].step(TRAIL_SELF_LOOP_COLLAPSE_SECONDS));
    }
}
