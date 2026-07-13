//! Unified, frame-aware movement-kernel facade.
//!
//! Every movement policy consumes the same body clusters, typed local-input
//! artifact, world, current [`MotionFrame`](crate::MotionFrame), and timestep.
//! The environment resolves that frame once from an explicit reference basis
//! plus the complete world-space acceleration for the body tick. It never lives
//! inside a model spec and is never rebuilt by an individual solver.

use crate::collision_semantics::{Contact, ContactKind};
use crate::{BodyClustersMut, MotionFrame, SweepSample, Vec2, World};

use super::adhesive_crawler;
use super::model::MotionModel;
use super::surface_momentum::{self, SurfaceBody, SurfaceInputs};
use super::{touching_hazard_aabb, FrameEvents, InputState};

/// One deterministic movement tick's complete external context.
#[derive(Clone, Copy)]
pub struct MotionStepContext<'a> {
    pub world: &'a World,
    /// The typed, already-frame-resolved motion intent (see [`InputState`]).
    pub input: InputState,
    /// The body's current acceleration/reference frame. Every policy arm
    /// receives this exact value; none is permitted to derive a private
    /// gravity frame.
    pub frame: MotionFrame,
    pub facing_intent: f32,
    pub dt: f32,
}

/// The tick's SEMANTIC support fact, selected from contact KINDS — never from
/// contact-list ordering. A lateral graze can no longer masquerade as support.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SupportFact {
    /// No support or attachment this tick.
    Airborne,
    /// Resting on / riding a surface holding the body against its frame's pull.
    Supported(Contact),
    /// Adhesively attached (policy-owned cling); the normal is the attachment,
    /// deliberately independent of the frame's pull.
    Attached(Contact),
}

impl SupportFact {
    /// The outward support/attachment normal, if any surface holds the body.
    pub fn normal(&self) -> Option<Vec2> {
        match self {
            SupportFact::Airborne => None,
            SupportFact::Supported(contact) | SupportFact::Attached(contact) => {
                Some(contact.normal)
            }
        }
    }

    /// The supporting/attached contact, if any.
    pub fn contact(&self) -> Option<&Contact> {
        match self {
            SupportFact::Airborne => None,
            SupportFact::Supported(contact) | SupportFact::Attached(contact) => Some(contact),
        }
    }

    pub fn is_held(&self) -> bool {
        !matches!(self, SupportFact::Airborne)
    }
}

/// Common observations produced by every movement policy.
#[derive(Clone, Debug)]
pub struct MotionStepResult {
    pub events: FrameEvents,
    /// The tick's semantic support fact (see [`SupportFact`]).
    pub support: SupportFact,
    /// Outward support normal for publishers that need a direction every tick:
    /// the support/attachment normal while held, opposite the resolved frame's
    /// down axis otherwise. Always derived from [`Self::support`].
    pub surface_normal: Vec2,
}

impl MotionStepResult {
    fn from_events(events: FrameEvents, frame: MotionFrame) -> Self {
        let support = support_fact(&events.contacts);
        Self {
            surface_normal: support.normal().unwrap_or(-frame.down()),
            support,
            events,
        }
    }
}

/// Step one body through its selected movement policy.
///
/// This is the only public movement-kernel gateway.  Model dispatch happens
/// inside the trusted kernel, while body/controller identity remains outside.
pub fn step_motion(
    model: &mut MotionModel,
    clusters: &mut BodyClustersMut<'_>,
    ctx: MotionStepContext<'_>,
) -> MotionStepResult {
    match model {
        MotionModel::AxisSwept(axis) => {
            let events = super::update_body_with_frame_clusters(
                ctx.world,
                clusters,
                ctx.input,
                ctx.frame,
                ctx.dt,
                axis.params,
            );
            MotionStepResult::from_events(events, ctx.frame)
        }
        MotionModel::SurfaceMomentum(momentum) => step_surface_momentum(momentum, clusters, ctx),
        MotionModel::AdhesiveCrawler(crawler) => step_adhesive_crawler(crawler, clusters, ctx),
    }
}

fn step_surface_momentum(
    motion: &mut super::SurfaceMomentumMotion,
    clusters: &mut BodyClustersMut<'_>,
    ctx: MotionStepContext<'_>,
) -> MotionStepResult {
    let sweep_entry = (clusters.kinematics.pos, clusters.kinematics.vel);
    let mut body = SurfaceBody {
        pos: clusters.kinematics.pos,
        vel: clusters.kinematics.vel,
        radius: clusters.kinematics.size.min_element() * 0.5,
        depth_lane: motion.depth_lane,
        motion: motion.state,
    };
    let mut contacts = Vec::new();
    surface_momentum::step_surface_body(
        &mut body,
        ctx.world,
        &motion.params,
        ctx.frame,
        SurfaceInputs {
            local_axes: ctx.input.axes,
            jump_pressed: ctx.input.jump_pressed,
        },
        ctx.dt,
        Some(&mut contacts),
    );

    clusters.kinematics.pos = body.pos;
    clusters.kinematics.vel = body.vel;
    if ctx.facing_intent.abs() > 0.001 {
        clusters.kinematics.facing = ctx.facing_intent.signum();
    }
    clusters.ground.on_ground = body.riding();
    motion.state = body.motion;
    motion.depth_lane = body.depth_lane;
    write_sweep_sample(clusters, sweep_entry);

    let mut events = FrameEvents {
        contacts,
        ..FrameEvents::default()
    };
    apply_world_hazard_gate(ctx.world, clusters, ctx.frame, &mut events);

    MotionStepResult::from_events(events, ctx.frame)
}

fn step_adhesive_crawler(
    motion: &mut super::AdhesiveCrawlerMotion,
    clusters: &mut BodyClustersMut<'_>,
    ctx: MotionStepContext<'_>,
) -> MotionStepResult {
    let sweep_entry = (clusters.kinematics.pos, clusters.kinematics.vel);
    let mut events = FrameEvents::default();
    adhesive_crawler::step_crawler(
        motion,
        ctx.world,
        clusters,
        ctx.frame,
        ctx.facing_intent,
        ctx.dt,
        &mut events.contacts,
    );
    write_sweep_sample(clusters, sweep_entry);
    apply_world_hazard_gate(ctx.world, clusters, ctx.frame, &mut events);

    MotionStepResult::from_events(events, ctx.frame)
}

/// §3.1 motion record for the non-axis policy arms: both endpoints captured
/// inside the kernel, so position changes outside this window are excluded
/// from the record by construction. (The axis arm writes its own sample at
/// simulation-phase boundaries.)
fn write_sweep_sample(clusters: &mut BodyClustersMut<'_>, entry: (Vec2, Vec2)) {
    let curr = clusters.kinematics.pos;
    let half = clusters.kinematics.size * 0.5;
    if let Some(sweep) = clusters.sweep.as_deref_mut() {
        *sweep = SweepSample {
            prev: entry.0,
            curr,
            vel: entry.1,
            half,
        };
    }
}

/// The ONE hazard/out-of-bounds gate every policy publishes through: hazard
/// touch plus the frame-relative "fell out of the world" test (distance past
/// the world AABB measured ALONG the fall direction). Policies flag; the
/// body's owner applies its reset policy.
fn apply_world_hazard_gate(
    world: &World,
    clusters: &mut BodyClustersMut<'_>,
    frame: MotionFrame,
    events: &mut FrameEvents,
) {
    let pos = clusters.kinematics.pos;
    let clamped = Vec2::new(
        pos.x.clamp(0.0, world.size.x),
        pos.y.clamp(0.0, world.size.y),
    );
    let fell_out = (pos - clamped).dot(frame.down()) > 200.0;
    if touching_hazard_aabb(world, clusters.kinematics.aabb()) || fell_out {
        events.hazard = true;
        events.reset = true;
    }
}

/// Select the tick's semantic support fact from the contact kinds: the newest
/// Attachment wins (an adhesive policy's cling overrides gravity support),
/// else the newest Support contact, else airborne. Head and Side contacts can
/// NEVER become the published support.
fn support_fact(contacts: &[Contact]) -> SupportFact {
    if let Some(contact) = contacts
        .iter()
        .rev()
        .find(|contact| contact.kind == ContactKind::Attachment)
    {
        return SupportFact::Attached(*contact);
    }
    contacts
        .iter()
        .rev()
        .find(|contact| contact.kind == ContactKind::Support)
        .map(|contact| SupportFact::Supported(*contact))
        .unwrap_or(SupportFact::Airborne)
}

#[cfg(test)]
mod tests;
