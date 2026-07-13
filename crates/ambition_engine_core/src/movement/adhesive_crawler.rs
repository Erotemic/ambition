//! Adhesive-crawler movement policy — a body GLUED to the surface it stands on,
//! crawling floors, walls, and ceilings by following the surface around convex
//! and concave corners (the PuppySlug crawl).
//!
//! This is the third sibling policy behind [`super::step_motion`]. It was
//! historically a hidden actor-only integrator (`step_surface_walker`) that
//! wrote body pose outside the kernel; it is now an explicit
//! [`super::MotionModel`] variant with its own authored [`CrawlerParams`] and
//! private [`CrawlerState`], stepped through the same frame-aware entry every
//! other body uses.
//!
//! Frame law: the crawler receives the environment-resolved
//! [`MotionFrame`](crate::MotionFrame) like every policy. While ATTACHED its
//! support direction is the clung surface (policy-private state, deliberately
//! independent of gravity — that is what "adhesive" means); while DETACHED it
//! free-falls along the frame's acceleration and re-attaches on the surface it
//! lands on, oriented by the frame's down axis.

use serde::{Deserialize, Serialize};

use crate::body_clusters::BodyClustersMut;
use crate::collision_semantics::{Axis, Contact, ContactKind, ContactSource};
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

/// Crawler-private persistent state: the attachment. `None` = detached and
/// falling; `Some(normal)` = clung to a surface whose outward unit normal is
/// `normal`. Nothing outside the kernel may author this except through the
/// typed [`AdhesiveCrawlerMotion::detach`] operation (the cling-break hit
/// reaction); the published support fact is the kernel result's
/// `surface_normal`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CrawlerState {
    attachment: Option<Vec2>,
}

impl CrawlerState {
    pub const DETACHED: Self = Self { attachment: None };

    pub fn attached(normal: Vec2) -> Self {
        Self {
            attachment: Some(normal),
        }
    }

    pub const fn attachment(self) -> Option<Vec2> {
        self.attachment
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
pub(super) fn step_crawler(
    motion: &mut AdhesiveCrawlerMotion,
    world: &World,
    clusters: &mut BodyClustersMut<'_>,
    frame: MotionFrame,
    facing_intent: f32,
    dt: f32,
    contacts: &mut Vec<Contact>,
) {
    if facing_intent.abs() > 0.001 {
        clusters.kinematics.facing = facing_intent.signum();
    }

    let Some(normal) = motion.state.attachment() else {
        fall_step(motion, world, clusters, frame, dt, contacts);
        publish_attachment_contact(motion, world, clusters, contacts);
        return;
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
    let body_long = clusters.kinematics.size.x * 0.5;
    let body_thick = clusters.kinematics.size.y * 0.5;

    // Concave corner: a wall dead ahead becomes the new floor.
    if wall_ahead(
        world,
        clusters.kinematics.pos,
        tangent,
        body_long,
        body_thick,
    ) {
        if let Some(pos) = snapped_to_surface(
            world,
            clusters.kinematics.pos,
            -tangent,
            body_long,
            body_thick,
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
    let around_corner = original_pos + tangent * body_long + (-n) * body_long;
    if let Some(pos) = snapped_to_surface(world, around_corner, tangent, body_long, body_thick) {
        clusters.kinematics.pos = pos;
        clusters.kinematics.vel = Vec2::ZERO;
        motion.state = CrawlerState::attached(tangent);
        finish_attached(clusters);
        publish_attachment_contact(motion, world, clusters, contacts);
        return;
    }

    // Reverse-side reattach (the surface curled back under the body).
    if let Some(pos) = snapped_to_surface(world, original_pos, -tangent, body_long, body_thick) {
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
    fall_step(motion, world, clusters, frame, dt, contacts);
    publish_attachment_contact(motion, world, clusters, contacts);
}

fn finish_attached(clusters: &mut BodyClustersMut<'_>) {
    clusters.ground.on_ground = true;
}

/// Push the tick's [`ContactKind::Attachment`] contact while attached — the
/// crawler's semantic support fact. The clung block supplies the source kind
/// and its frame motion; if the probe unexpectedly finds nothing the contact
/// still records the attachment (static, unknown-solid), never silence.
fn publish_attachment_contact(
    motion: &AdhesiveCrawlerMotion,
    world: &World,
    clusters: &BodyClustersMut<'_>,
    contacts: &mut Vec<Contact>,
) {
    let Some(normal) = motion.state.attachment() else {
        return;
    };
    let body_thick = clusters.kinematics.size.y * 0.5;
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
        source: ContactSource::Block {
            kind: clung.map_or(crate::world::BlockKind::Solid, |b| b.kind),
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
        super::collision::sweep_player_axis_clusters(
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
        );
    };
    sweep(clusters, side_axis);
    clusters.ground.on_ground = false;
    sweep(clusters, gravity_axis);

    if clusters.ground.on_ground {
        // Attach to the LANDED surface's true outward normal (the semantic
        // Support contact) — under an oblique frame the surface's normal and
        // the frame's anti-down differ, and adhesion is about the surface.
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

fn wall_ahead(world: &World, pos: Vec2, tangent: Vec2, body_long: f32, body_thick: f32) -> bool {
    let probe_center = pos + tangent * (body_long + 3.0);
    let normal = Vec2::new(-tangent.y, tangent.x);
    let half = surface_probe_half(tangent, normal, 2.0, body_thick * 0.7);
    let probe = Aabb::new(probe_center, half);
    world.body_overlaps_any(probe, wall_pred)
}

/// March a probe from `pos` toward the surface opposite `normal`; when it finds
/// cling geometry, return the position seated `body_thick` off that surface.
/// `None` when no surface is within reach.
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
        if world.body_overlaps_any(probe, cling_pred) {
            return Some(pos + normal * (body_thick - (d - 0.5)));
        }
    }
    None
}
