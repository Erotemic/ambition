//! Adhesive-crawler movement policy for floors, walls, and ceilings.
//!
//! [`super::step_motion`] drives this [`super::MotionModel`] with authored
//! [`CrawlerParams`] and private [`CrawlerState`]. While attached, support is the
//! clung surface and is independent of gravity. While detached, the body falls
//! along the resolved [`MotionFrame`](crate::MotionFrame) acceleration and
//! reattaches to the surface it lands on.

use serde::{Deserialize, Serialize};

use crate::body_clusters::BodyClustersMut;
use crate::collision_semantics::{
    Axis, AxisConstraintConflict, Contact, ContactKind, ContactSource,
};
use crate::geometry::AabbExt;
use crate::world::{Block, BlockKind, World};
use crate::{Aabb, MotionFrame, Vec2};

/// Authored parameters of the adhesive-crawler policy. Like every policy
/// parameter type, this contains no live-environment fields: detached-fall
/// acceleration comes from the per-tick frame.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CrawlerParams {
    /// Tangential crawl speed along the clung surface (px/s).
    pub crawl_speed: f32,
    /// Detached-fall terminal speed along the frame's down axis (px/s).
    pub max_fall_speed: f32,
}

impl Default for CrawlerParams {
    fn default() -> Self {
        Self {
            crawl_speed: 40.0,
            max_fall_speed: super::MAX_FALL_SPEED,
        }
    }
}

/// What the crawler is glued to.
///
/// The two variants mirror the world's two surface representations: cardinal
/// AABB block faces (probe-based crawl, corner transit via probes) and
/// [`SurfaceChain`](crate::world::SurfaceChain) polylines — arbitrary-angle
/// surfaces whose crawl follows the chain's own local frame, so corner transit
/// is the GEOMETRY (`frame_at` walks the polyline), never a world-axis case.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CrawlAttachment {
    /// Clung to a block face whose outward unit normal is `normal`.
    Block { normal: Vec2 },
    /// Clung to `World::chains[chain]` at arc length `s` (rideable side).
    Chain { chain: u32, s: f32 },
}

/// Crawler-private persistent state: the attachment. `None` = detached and
/// falling. Nothing outside the kernel may author this except through the
/// typed [`AdhesiveCrawlerMotion::detach`] operation (the cling-break hit
/// reaction); the published support fact is the kernel result's
/// `surface_normal`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CrawlerState {
    attachment: Option<CrawlAttachment>,
}

impl CrawlerState {
    pub const DETACHED: Self = Self { attachment: None };

    /// Clung to a block face with outward normal `normal`.
    pub fn attached(normal: Vec2) -> Self {
        Self {
            attachment: Some(CrawlAttachment::Block { normal }),
        }
    }

    /// Clung to a surface chain at arc length `s`.
    pub fn attached_to_chain(chain: u32, s: f32) -> Self {
        Self {
            attachment: Some(CrawlAttachment::Chain { chain, s }),
        }
    }

    pub const fn attachment(self) -> Option<CrawlAttachment> {
        self.attachment
    }

    /// The clung surface's outward normal, resolved against the live world
    /// (a chain attachment's normal is the geometry's frame at `s`).
    pub fn attached_normal(self, world: &World) -> Option<Vec2> {
        match self.attachment? {
            CrawlAttachment::Block { normal } => Some(normal),
            CrawlAttachment::Chain { chain, s } => world
                .chains
                .get(chain as usize)
                .map(|surface| surface.frame_at(s).normal),
        }
    }

    pub const fn is_attached(self) -> bool {
        self.attachment.is_some()
    }
}

/// The adhesive-crawler policy's model-owned data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdhesiveCrawlerMotion {
    pub params: CrawlerParams,
    pub state: CrawlerState,
}

impl AdhesiveCrawlerMotion {
    /// A fresh crawler begins DETACHED on the unchanged pose; it may acquire a
    /// support only through its normal same-tick contact rule (no
    /// nearest-surface snapping during policy initialization).
    pub fn new(params: CrawlerParams) -> Self {
        Self {
            params,
            state: CrawlerState::DETACHED,
        }
    }

    /// Typed detach operation — the cling-break hit reaction. The body falls
    /// under the live frame next tick; the caller applies any peel impulse to
    /// shared velocity itself.
    pub fn detach(&mut self) {
        self.state = CrawlerState::DETACHED;
    }
}

impl Default for AdhesiveCrawlerMotion {
    fn default() -> Self {
        Self::new(CrawlerParams::default())
    }
}

/// Predicate matching any tile a crawler can CLING TO — both solid blocks and
/// one-way platforms count, mirroring what the axis sweep treats as "ground"
/// for grounded bodies.
fn cling_pred(b: &Block) -> bool {
    matches!(
        b.kind,
        BlockKind::Solid | BlockKind::OneWay | BlockKind::BlinkWall { .. }
    )
}

/// Predicate matching tiles a crawler treats as "walls in the way" — strictly
/// solid, NOT one-way. A one-way platform sitting in the crawl path along a
/// wall must not register as a concave corner since the crawler would never
/// collide with its side anyway.
fn wall_pred(b: &Block) -> bool {
    matches!(b.kind, BlockKind::Solid | BlockKind::BlinkWall { .. })
}

/// One crawler tick. Kernel-private: reached only through
/// [`super::step_motion`]'s dispatch.
///
/// While attached it pushes one [`ContactKind::Attachment`] contact per tick —
/// the crawler's semantic support fact — so the kernel result derives the
/// published normal from the SAME contact vocabulary every policy speaks.
#[allow(clippy::too_many_arguments)]
pub(super) fn step_crawler(
    motion: &mut AdhesiveCrawlerMotion,
    world: &World,
    clusters: &mut BodyClustersMut<'_>,
    frame: MotionFrame,
    facing_intent: f32,
    dt: f32,
    contacts: &mut Vec<Contact>,
    conflicts: &mut Vec<AxisConstraintConflict>,
) {
    if facing_intent.abs() > 0.001 {
        clusters.kinematics.facing = facing_intent.signum();
    }

    let attachment = match motion.state.attachment() {
        None => {
            fall_step(motion, world, clusters, frame, dt, contacts, conflicts);
            publish_attachment_contact(motion, world, clusters, contacts);
            return;
        }
        Some(attachment) => attachment,
    };
    let normal = match attachment {
        CrawlAttachment::Chain { chain, s } => {
            crawl_chain(
                motion, world, clusters, frame, dt, contacts, conflicts, chain, s,
            );
            return;
        }
        CrawlAttachment::Block { normal } => normal,
    };

    // Emergent riding for a crawler: it is GLUED to its surface (it crawls
    // floors, walls, ceilings), so a MOVING surface carries it by the FULL
    // velocity — both axes, not just the gravity-perpendicular component a
    // gravity-resting body gets. Probe toward the surface it's clinging to.
    {
        let toward_surface = -normal;
        let probe = Aabb::new(
            clusters.kinematics.pos + toward_surface * 2.0,
            clusters.kinematics.size * 0.5,
        );
        if let Some(block) = world.first_overlapping_block(probe, cling_pred) {
            clusters.kinematics.pos += block.velocity;
        }
    }

    let n = normal;
    let facing = clusters.kinematics.facing;
    let speed = motion.params.crawl_speed;
    let step_len = speed * dt;
    let tangent = Vec2::new(-n.y * facing, n.x * facing);
    let half = clusters.kinematics.size * 0.5;
    // Extents in the CLUNG surface's frame. Every probe below takes the extents
    // of the surface IT is about, never these — a corner transition asks about a
    // face at 90° to this one, where `long` and `thick` trade places.
    let (body_long, body_thick) = body_extents_for(half, n);

    // Concave corner: a wall dead ahead becomes the new floor.
    if wall_ahead(
        world,
        clusters.kinematics.pos,
        tangent,
        body_long,
        body_thick,
        step_len,
    ) {
        let (turn_long, turn_thick) = body_extents_for(half, -tangent);
        if let Some(pos) = snapped_to_surface(
            world,
            clusters.kinematics.pos,
            -tangent,
            turn_long,
            turn_thick,
        ) {
            clusters.kinematics.pos = pos;
            clusters.kinematics.vel = Vec2::ZERO;
            motion.state = CrawlerState::attached(-tangent);
            finish_attached(clusters);
            publish_attachment_contact(motion, world, clusters, contacts);
            return;
        }
    }

    // Ordinary crawl along the tangent.
    let original_pos = clusters.kinematics.pos;
    clusters.kinematics.pos += tangent * step_len;
    clusters.kinematics.vel = tangent * speed;
    if let Some(pos) = snapped_to_surface(world, clusters.kinematics.pos, n, body_long, body_thick)
    {
        clusters.kinematics.pos = pos;
        finish_attached(clusters);
        publish_attachment_contact(motion, world, clusters, contacts);
        return;
    }

    // Convex corner: wrap around the block edge; the old tangent becomes the
    // new outward normal.
    //
    // The probe origin steps past the edge along the tangent and drops toward
    // the new face. Both offsets are stated in the NEW surface's frame — the
    // face being sought runs along `-n`, so `wrap_long` is the reach along it
    // and `wrap_thick` the standoff from it.
    let (wrap_long, wrap_thick) = body_extents_for(half, tangent);
    let around_corner = original_pos + tangent * wrap_thick + (-n) * wrap_long;
    if let Some(pos) = snapped_to_surface(world, around_corner, tangent, wrap_long, wrap_thick) {
        clusters.kinematics.pos = pos;
        clusters.kinematics.vel = Vec2::ZERO;
        motion.state = CrawlerState::attached(tangent);
        finish_attached(clusters);
        publish_attachment_contact(motion, world, clusters, contacts);
        return;
    }

    // Reverse-side reattach (the surface curled back under the body).
    let (back_long, back_thick) = body_extents_for(half, -tangent);
    if let Some(pos) = snapped_to_surface(world, original_pos, -tangent, back_long, back_thick) {
        clusters.kinematics.pos = pos;
        clusters.kinematics.vel = Vec2::ZERO;
        motion.state = CrawlerState::attached(-tangent);
        finish_attached(clusters);
        publish_attachment_contact(motion, world, clusters, contacts);
        return;
    }

    // Nothing to cling to: detach and free-fall under the live frame.
    clusters.kinematics.pos = original_pos;
    motion.detach();
    fall_step(motion, world, clusters, frame, dt, contacts, conflicts);
    publish_attachment_contact(motion, world, clusters, contacts);
}

fn finish_attached(clusters: &mut BodyClustersMut<'_>) {
    clusters.ground.on_ground = true;
}

/// One attached-to-chain crawl tick: advance the arc-length cursor by the
/// crawl speed, seat the body one half-thickness off the chain's local frame,
/// and publish the Attachment contact from that frame. Corner/junction transit
/// IS the polyline walk — `frame_at` blends across segments of ANY angle, so
/// attached crawling is covariant by construction. An open chain's end
/// detaches the crawler (it falls under the live frame); a closed chain wraps.
#[allow(clippy::too_many_arguments)]
fn crawl_chain(
    motion: &mut AdhesiveCrawlerMotion,
    world: &World,
    clusters: &mut BodyClustersMut<'_>,
    frame: MotionFrame,
    dt: f32,
    contacts: &mut Vec<Contact>,
    conflicts: &mut Vec<AxisConstraintConflict>,
    chain: u32,
    s: f32,
) {
    let Some(surface) = world.chains.get(chain as usize) else {
        // The clung geometry is gone (room swap without a transit): fall.
        motion.detach();
        fall_step(motion, world, clusters, frame, dt, contacts, conflicts);
        return;
    };
    let facing = clusters.kinematics.facing;
    let speed = motion.params.crawl_speed;
    let next_s = s + facing * speed * dt;
    let total = surface.total_length();
    if !surface.closed && !(0.0..=total).contains(&next_s) {
        // Crawled off an open end: detach and free-fall under the live frame.
        motion.detach();
        fall_step(motion, world, clusters, frame, dt, contacts, conflicts);
        return;
    }
    let f = surface.frame_at(next_s);
    let body_thick = clusters.kinematics.size.y * 0.5;
    clusters.kinematics.pos = f.point + f.normal * body_thick;
    clusters.kinematics.vel = f.tangent * facing * speed;
    motion.state = CrawlerState::attached_to_chain(
        chain,
        if surface.closed {
            next_s.rem_euclid(total)
        } else {
            next_s
        },
    );
    finish_attached(clusters);
    contacts.push(Contact {
        kind: ContactKind::Attachment,
        point: f.point,
        normal: f.normal,
        toi: 0.0,
        surface_velocity: surface.velocity,
        // A crawler ATTACHES; it does not arrive. Its closing speed along the
        // surface it is already stuck to is what it is.
        impact_speed: crate::collision_semantics::closing_speed(clusters.kinematics.vel, f.normal),
        involuntary: false,
        source: crate::collision_semantics::ContactSource::Chain {
            chain,
            segment: f.segment as u32,
        },
    });
}

/// While falling, the first chain whose rideable side the body's underside
/// touches captures the crawler (adhesion): projection within one
/// half-thickness (+slop) on the `+normal` side.
fn chain_capture(world: &World, pos: Vec2, body_thick: f32) -> Option<(u32, f32)> {
    const CAPTURE_SLOP: f32 = 2.0;
    for (index, surface) in world.chains.iter().enumerate() {
        if surface.points.len() < 2 {
            continue;
        }
        let (s, signed) = surface.project(pos);
        if signed >= 0.0 && signed <= body_thick + CAPTURE_SLOP {
            return Some((index as u32, s));
        }
    }
    None
}

/// Push the tick's [`ContactKind::Attachment`] contact while attached — the
/// crawler's semantic support fact. The clung block supplies the source kind
/// and its frame motion; if the probe unexpectedly finds nothing the contact
/// still records the attachment (static, unknown-solid), never silence.
/// Publish the Attachment contact for a crawler stuck to a BLOCK.
///
/// ⭐⭐ THE INVARIANT THIS HALF-IMPLEMENTS, stated because reconstructing it costs
/// twenty minutes: **every exit of `step_crawler` publishes exactly ONE
/// `ContactKind::Attachment`, and it does so by TWO mechanisms.** Five paths call
/// this function; the sixth returns from `crawl_chain`, which pushes its own
/// contact inline. ⇒ a new exit must publish exactly once — and a `step_crawler`
/// that wrapped its body to publish "on every path" would DOUBLE-publish the
/// chain one.
///
/// ⚠ THE TWO ARE NOT A DUPLICATION AND SHOULD NOT BE MERGED. The chain path HAS
/// the geometry (`f.point`, `f.normal` come from the polyline frame) and the
/// surface it is riding; this path must PROBE the world for the block it is clung
/// to, and falls back to zero velocity and an anonymous `GeoId` when nothing
/// answers. Different information, therefore different construction.
/// ⭐ What they DO share is the rule that matters, and they already share it in
/// one place: `impact_speed` is `collision_semantics::closing_speed(vel, normal)`
/// on both roads, because a crawler ATTACHES rather than arrives.
fn publish_attachment_contact(
    motion: &AdhesiveCrawlerMotion,
    world: &World,
    clusters: &BodyClustersMut<'_>,
    contacts: &mut Vec<Contact>,
) {
    let Some(normal) = motion.state.attached_normal(world) else {
        return;
    };
    let (_, body_thick) = body_extents_for(clusters.kinematics.size * 0.5, normal);
    let probe = Aabb::new(
        clusters.kinematics.pos - normal * 2.0,
        clusters.kinematics.size * 0.5,
    );
    let clung = world.first_overlapping_block(probe, cling_pred);
    contacts.push(Contact {
        kind: ContactKind::Attachment,
        point: clusters.kinematics.pos - normal * body_thick,
        normal,
        toi: 0.0,
        surface_velocity: clung.map_or(Vec2::ZERO, |b| b.velocity),
        // Already clung: the crawler is not arriving at this surface.
        impact_speed: crate::collision_semantics::closing_speed(clusters.kinematics.vel, normal),
        involuntary: false,
        source: ContactSource::Block {
            kind: clung.map_or(crate::world::BlockKind::Solid, |b| b.kind),
            id: clung.map_or(crate::geo_id::GeoId::anon(), |b| b.id.clone()),
        },
    });
}

/// Detached free-fall under the live frame, swept through the SAME axis
/// collision doctrine every policy shares. On landing the crawler re-attaches
/// with its normal opposite the frame's down axis.
fn fall_step(
    motion: &mut AdhesiveCrawlerMotion,
    world: &World,
    clusters: &mut BodyClustersMut<'_>,
    frame: MotionFrame,
    dt: f32,
    contacts: &mut Vec<Contact>,
    conflicts: &mut Vec<AxisConstraintConflict>,
) {
    clusters.ground.on_ground = false;
    let g = frame.down();

    // Terminal velocity is an equilibrium gravity accelerates up to, never a
    // brake on an over-cap fling (same rule as the axis spine).
    let fall_before = clusters.kinematics.vel.dot(g).max(0.0);
    let cap = motion.params.max_fall_speed.max(fall_before);
    clusters.kinematics.vel += frame.acceleration() * dt;
    let along = clusters.kinematics.vel.dot(g);
    if along > cap {
        clusters.kinematics.vel -= (along - cap) * g;
    }

    // Sweep the frame's SIDE-role world axis first, then its gravity-role
    // axis — the same role classification the shared collision doctrine uses
    // (dominant component decides; an oblique frame sweeps both axes with the
    // full per-axis velocity, classified frame-relatively inside the sweep).
    let gravity_axis = crate::collision_semantics::gravity_axis(g);
    let side_axis = gravity_axis.perpendicular();
    let mut sweep = |clusters: &mut BodyClustersMut<'_>, axis: Axis| {
        let prev_feet_coord = clusters.kinematics.aabb_oriented(g).feet_coord(g);
        let delta_along = match axis {
            Axis::X => clusters.kinematics.vel.x,
            Axis::Y => clusters.kinematics.vel.y,
        } * dt;
        // Same seam as the axis spine: a crush is REPORTED and left to the
        // body's owner to interpret.
        conflicts.extend(super::collision::sweep_player_axis_clusters(
            world,
            clusters.kinematics,
            clusters.ground,
            clusters.wall,
            clusters.body_mode,
            clusters.env_contact,
            axis,
            delta_along,
            prev_feet_coord,
            false,
            g,
            contacts,
        ));
    };
    sweep(clusters, side_axis);
    clusters.ground.on_ground = false;
    sweep(clusters, gravity_axis);

    // Adhesion: a chain surface the body's underside touches captures the
    // crawler mid-fall — arbitrary-angle attachment through the same geometry
    // the momentum policy rides.
    let body_thick = clusters.kinematics.size.y * 0.5;
    if let Some((chain, s)) = chain_capture(world, clusters.kinematics.pos, body_thick) {
        motion.state = CrawlerState::attached_to_chain(chain, s);
        clusters.kinematics.vel = Vec2::ZERO;
        clusters.ground.on_ground = true;
        return;
    }

    if clusters.ground.on_ground {
        let landed = contacts
            .iter()
            .rev()
            .find(|contact| contact.kind == ContactKind::Support)
            .map(|contact| contact.normal)
            .unwrap_or_else(|| -g.normalize_or(Vec2::new(0.0, 1.0)));
        motion.state = CrawlerState::attached(landed);
        clusters.kinematics.vel = Vec2::ZERO;
    }
}

/// World-AABB half-extent BOUNDING a box authored in a surface-local basis
/// (`along` on the tangent axis, `across` on the normal axis). Exact for
/// cardinal bases (90° swaps components); a conservative bound for oblique
/// ones — the crawler's probes are covariant constructions, never world-axis
/// cases.
fn surface_probe_half(tangent: Vec2, normal: Vec2, along: f32, across: f32) -> Vec2 {
    Vec2::new(
        (tangent.x * along).abs() + (normal.x * across).abs(),
        (tangent.y * along).abs() + (normal.y * across).abs(),
    )
}

/// The body's half-extents expressed in a SURFACE's frame: `(along the tangent,
/// across the normal)`. The inverse of [`surface_probe_half`], and the reason a
/// crawl on a wall is the same code as a crawl on a floor.
///
/// A crawler's AABB does not rotate when it changes surface, so "how long is the
/// body" and "how thick is it" are questions about the SURFACE, not about the
/// body: on a floor the answers are `size.x/2` and `size.y/2`, and on a wall they
/// are exactly swapped. Reading them off the world axes once — which is what this
/// module did — is right on floors and ceilings and wrong on every vertical face,
/// where it seated the body a half-width INSIDE the wall it had just grabbed and
/// made the centre leap to get there. That was the visible half of "slugs get
/// stuck on corners": the shaft's ledges and its full-height pillar are all
/// vertical faces.
fn body_extents_for(half: Vec2, normal: Vec2) -> (f32, f32) {
    let tangent = Vec2::new(-normal.y, normal.x);
    let along = (half.x * tangent.x).abs() + (half.y * tangent.y).abs();
    let across = (half.x * normal.x).abs() + (half.y * normal.y).abs();
    (along, across)
}

/// Is there a wall within the NEXT CRAWL STEP of the body's leading edge?
fn wall_ahead(
    world: &World,
    pos: Vec2,
    tangent: Vec2,
    body_long: f32,
    body_thick: f32,
    reach: f32,
) -> bool {
    let reach = reach.max(1.0);
    let probe_center = pos + tangent * (body_long + reach * 0.5);
    let normal = Vec2::new(-tangent.y, tangent.x);
    let half = surface_probe_half(tangent, normal, reach * 0.5, body_thick * 0.7);
    let probe = Aabb::new(probe_center, half);
    world.body_overlaps_any(probe, wall_pred)
}

/// March a probe from `pos` toward the surface opposite `normal`; when it finds
/// cling geometry, return the position seated `body_thick` off that surface.
/// `None` when no surface is within reach.
///
/// The march only has to FIND the surface — the seat then comes from the block's own face
/// ([`seated_on`]), so it is exact regardless of how coarsely the probe stepped. Deriving the
/// seat from the march distance instead is what produced a permanent 30 Hz shimmer in every
/// crawl: the seat was `body_thick - (d - 0.5)` off the probe, and settling required `d ==
/// body_thick + 0.5` — a half-integer the integer march can never take.
fn snapped_to_surface(
    world: &World,
    pos: Vec2,
    normal: Vec2,
    body_long: f32,
    body_thick: f32,
) -> Option<Vec2> {
    let down = -normal;
    let max_d = (body_thick + body_long + 4.0) as i32;
    let tangent = Vec2::new(-normal.y, normal.x);
    let half = surface_probe_half(tangent, normal, body_long * 0.35, 0.75);
    for i in 0..=max_d {
        let d = i as f32;
        let probe = Aabb::new(pos + down * d, half);
        if let Some(block) = world.first_overlapping_block(probe, cling_pred) {
            return Some(seated_on(block.aabb, pos, normal, body_thick));
        }
    }
    None
}

/// The pose seated exactly `body_thick` off `surface`'s `normal`-side face,
/// leaving the TANGENTIAL coordinate alone — seating is a statement about the
/// distance to the surface, and must never undo the crawl's own progress along
/// it.
///
/// Cardinal normals only, which is the block-attachment vocabulary: arbitrary
/// angles are the [`CrawlAttachment::Chain`] case and seat from the polyline's
/// own frame instead.
fn seated_on(surface: Aabb, pos: Vec2, normal: Vec2, body_thick: f32) -> Vec2 {
    let face = Vec2::new(
        if normal.x >= 0.0 {
            surface.max.x
        } else {
            surface.min.x
        },
        if normal.y >= 0.0 {
            surface.max.y
        } else {
            surface.min.y
        },
    );
    // `normal.abs()` is a basis selector for a cardinal normal: 1 on the axis
    // being seated, 0 on the axis being preserved.
    let seated_axis = normal.abs();
    pos * (Vec2::ONE - seated_axis) + (face + normal * body_thick) * seated_axis
}
