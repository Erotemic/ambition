//! Unified, frame-aware movement-kernel facade.
//!
//! Every movement policy consumes the same body clusters, typed local-input
//! artifact, world, current [`MotionFrame`](crate::MotionFrame), and timestep.
//! The environment resolves that frame once from an explicit reference basis
//! plus the complete world-space acceleration for the body tick. It never lives
//! inside a model spec and is never rebuilt by an individual solver.

use crate::collision_semantics::{supporting_block, Contact, ContactKind};
use crate::{BodyClustersMut, MotionFrame, SweepSample, Vec2, World};

use super::adhesive_crawler;
use super::model::MotionModel;
use super::surface_momentum::{self, SurfaceBody, SurfaceInputs};
use super::{
    hazard_contact_on_path, touching_hazard_aabb, touching_rebound_aabb, FrameEvents,
    GroundContactTransition, InputState, ResetCause,
};

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
    /// THE OTHER BODIES THIS STEP MAY NOT MOVE FREELY THROUGH.
    ///
    /// `BodyContactField::NONE` is the default and the identity — every
    /// composition that has not granted the capability resolves exactly as it
    /// did. See [`super::body_contact`] for why an acceleration term cannot do
    /// this job and why the field is a snapshot rather than a query.
    pub contact: super::body_contact::BodyContactField<'a>,
    /// ANOTHER AUTHORITY OWNS THIS BODY'S POSE THIS TICK — a saddle, a capture,
    /// any constraint that writes position and velocity after the kernel runs.
    ///
    /// ⛔⛔ IT GATES THE LAUNCH DRAIN, and that is the whole reason it exists.
    /// A pending launch is INVOLUNTARY DISPLACEMENT, and displacement is exactly
    /// what the external owner has taken over. Draining it here would apply
    /// knockback to `vel` that the constraint then overwrites with zero on the
    /// same tick — the hit is spent, the body never moves, and every downstream
    /// reader sees a fighter that was launched and went nowhere. Measured on the
    /// pirate's shark: a launch strong enough to end the ride put him off the
    /// saddle at ZERO velocity, because `sync_riders_to_mounts` pins the rider
    /// after `step_motion` has already consumed the launch.
    ///
    /// ⭐ SO THE LAUNCH STAYS STAGED. `PendingLaunch` survives until a tick on
    /// which nobody else owns the pose, and the kernel spends it then — which is
    /// the tick the body is actually free to fly. Nothing needs to know WHY the
    /// pose was held or WHO released it.
    ///
    /// ⚠ A REQUIRED FIELD, not a default. "Who owns this body's displacement" is
    /// a question a caller building a motion step should have to answer; a
    /// defaulted `false` is how a constrained body silently gets two authorities
    /// again.
    pub pose_owned_externally: bool,
    /// THIS BODY HAS COMMITTED A RECOVERY THAT IS STILL HAPPENING.
    ///
    /// ⭐⭐ IT GATES THE GROUNDED REFRESH, and only that. A recovery is the one
    /// movement resource a fighter spends while still standing on the floor, and
    /// the landing-class refresh runs on every grounded TICK rather than on the
    /// landing EVENT — so the charge went back the frame after it was spent, for
    /// every fighter, contradicting the rule `start_move` states in its own
    /// comment. It is invisible for an ordinary up-B, which leaves the ground
    /// immediately; it is the whole story for one that hands its owner a vehicle,
    /// because that fighter is still grounded when the mount takes over.
    ///
    /// ⛔ DERIVED, NOT STORED. The authority is the `MovePlayback` the body is
    /// running — already rollback state — so asking it every tick cannot desync
    /// the way a second copy on `BodyJumpState` would. See
    /// `ambition_combat::moveset::recovery_commitment_outstanding`, which is the
    /// one place the question is answered.
    ///
    /// ⚠ A REQUIRED FIELD for the same reason as the one above it.
    pub recovery_commitment_outstanding: bool,
}

/// The tick's SEMANTIC support fact, selected from contact KINDS — never from
/// contact-list ordering. A lateral graze can no longer masquerade as support.
///
/// Holds a [`Contact`], which owns its block's `GeoId` and so is `Clone`, not
/// `Copy` — this fact is likewise `Clone`.
#[derive(Clone, Debug, PartialEq)]
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
    // THE launch drain, and it is here because here is the only gateway.
    //
    // An external reaction (knockback, a fling) writes a world-space launch into
    // `BodyFlightState::pending_launch` and cannot apply it itself: it holds a
    // `&mut Vec2`, not the model, and only the model knows what a launch MEANS to
    // it. Writing `kinematics.vel` directly is authoritative for an axis-swept
    // body and a LIE for a riding surface-momentum one, whose `vel` is derived
    // from `v_t` and republished every step — which is why Sanic took knockback
    // with every number non-zero and never moved.
    //
    // draining BEFORE the step, so the launch is honoured by this tick rather
    // than by the next one. That matches the jump path inside the surface kernel,
    // which sets the velocity, goes airborne, and then takes its substep.
    //
    // and it is drained in ONE place on purpose.
    // ⛔⛔ A HELD BODY TAKES THE HIT BUT NOT THE TRAVEL. See
    // `MotionStepContext::pose_owned_externally`. The launch's two halves are
    // separable and only one of them belongs to the external authority: deciding
    // that this hit sends the body tumbling is about the HIT, and it has to
    // happen now because the policy that ends the ride reads `tumbling`.
    // Assigning the velocity is about DISPLACEMENT, which is exactly what the
    // constraint has taken over — writing it here would hand knockback to a
    // `vel` the saddle overwrites with zero on this same tick.
    //
    // ⭐ SO THE VELOCITY STAYS STAGED while the floor-game answer lands
    // immediately. `PendingLaunch` survives to the first tick nobody owns the
    // pose, which is the tick the body is free to fly.
    if ctx.pose_owned_externally {
        let staged = clusters.flight.pending_launch_state();
        let tumbles = accept_external_launch(model, clusters, &ctx, staged, LaunchTravel::Deferred);
        // ⛔⛔ AND AN ABSORBED LAUNCH IS RETIRED RATHER THAN BANKED. Deferring
        // EVERY launch was too broad: a weak hit that flinches a rider and leaves
        // it aboard also staged its travel, and nothing spent it — so the jab sat
        // in the gateway until the ride ended for an unrelated reason (the lease
        // ran out, the player jumped off) and THEN threw the body across the
        // stage, seconds after the hit that caused it.
        //
        // ⭐ THE TWO CASES ARE TOLD APART BY THE FLOOR GAME, not by the
        // constraint's identity. A launch that tumbles is the one that will end
        // the constraint — its travel has to outlive the release. A launch that
        // does not tumble is one the constraint ABSORBED, and absorbing it is the
        // whole of what "the saddle held him through that hit" means.
        if !tumbles {
            let _ = clusters.flight.take_launch();
        }
    } else {
        let launch = clusters.flight.take_launch();
        accept_external_launch(model, clusters, &ctx, launch, LaunchTravel::Applied);
    }
    match model {
        MotionModel::AxisSwept(axis) => {
            let events = super::update_body_with_frame_clusters(
                ctx.world,
                axis,
                clusters,
                ctx.input,
                ctx.frame,
                ctx.dt,
                ctx.contact,
                ctx.recovery_commitment_outstanding,
                ctx.pose_owned_externally,
            );
            MotionStepResult::from_events(events, ctx.frame)
        }
        MotionModel::SurfaceMomentum(momentum) => {
            let baseline = establish_ground_contact_baseline_from_sample(
                clusters,
                matches!(momentum.state, super::SurfaceMotion::Riding { .. }),
                ctx.frame,
            );
            let mut result = step_surface_momentum(momentum, clusters, ctx);
            result.events.ground_contact = baseline.transition_to(clusters.ground.on_ground);
            result
        }
        MotionModel::AdhesiveCrawler(crawler) => {
            let baseline = establish_ground_contact_baseline_from_sample(
                clusters,
                crawler.state.is_attached(),
                ctx.frame,
            );
            let mut result = step_adhesive_crawler(crawler, clusters, ctx);
            result.events.ground_contact = baseline.transition_to(clusters.ground.on_ground);
            result
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct GroundContactBaseline {
    grounded: bool,
    initialized_now: bool,
    impact_speed: f32,
    /// Was this body still falling out of a launch on the way into the step?
    ///
    /// Captured with the baseline rather than read at the transition, because
    /// `tick_knockdown` CLEARS `tumble_until_landing` on the touchdown it
    /// resolves — a read taken afterwards is always false, which is the same
    /// phase trap that made a `MovementOp::Knockdown` splat read zero.
    involuntary: bool,
}

impl GroundContactBaseline {
    pub(super) fn with_impact_velocity(mut self, velocity: Vec2, frame: MotionFrame) -> Self {
        self.impact_speed = velocity.dot(frame.down()).max(0.0);
        self
    }

    /// Record that the body entered this step still tumbling. See the field.
    pub(super) fn falling_out_of_a_launch(mut self, tumbling: bool) -> Self {
        self.involuntary = tumbling;
        self
    }

    pub(super) fn transition_to(self, grounded_now: bool) -> GroundContactTransition {
        match (self.grounded, grounded_now) {
            (false, true) => GroundContactTransition::Landed {
                impact_speed: self.impact_speed,
                involuntary: self.involuntary,
            },
            (true, false) => GroundContactTransition::LeftGround,
            (true, true) if self.initialized_now => GroundContactTransition::InitializedGrounded,
            (false, false) if self.initialized_now => GroundContactTransition::InitializedAirborne,
            _ => GroundContactTransition::Unchanged,
        }
    }
}

/// Establish the contact state at the step's entry pose before control or
/// integration can interpret it. This is the distinction between
/// `unknown -> grounded` (construction baseline, no landing) and
/// `airborne -> grounded` (a real landing), including a body that spawns in
/// the air and touches down during its first tick.
fn establish_ground_contact_baseline_from_sample(
    clusters: &mut BodyClustersMut<'_>,
    sampled_grounded: bool,
    frame: MotionFrame,
) -> GroundContactBaseline {
    let initialized_now = !clusters.ground.contact_initialized;
    if initialized_now {
        clusters.ground.on_ground = sampled_grounded;
        clusters.ground.contact_initialized = true;
    }
    GroundContactBaseline {
        grounded: clusters.ground.on_ground,
        initialized_now,
        impact_speed: clusters.kinematics.vel.dot(frame.down()).max(0.0),
        // Neither of the models that reach this sample tumbles: a surface
        // rider and an adhesive crawler have no launch to fall out of. The
        // axis path sets it explicitly.
        involuntary: false,
    }
}

pub(super) fn establish_axis_ground_contact_baseline(
    world: &World,
    clusters: &mut BodyClustersMut<'_>,
    frame: MotionFrame,
) -> GroundContactBaseline {
    let sampled_grounded = supporting_block(
        world,
        clusters.kinematics.aabb_oriented(frame.down()),
        frame.down(),
        false,
    )
    .is_some();
    establish_ground_contact_baseline_from_sample(clusters, sampled_grounded, frame)
}

/// Hand a pending launch to whichever model owns this body's velocity.
///
/// Zero is the empty state (see [`BodyFlightState::pending_launch`]), so the
/// common path is one comparison.
/// Whether this acceptance may move the body, or only decide what the hit MEANT.
///
/// ⛔ `Deferred` exists for one case and it is named on `pose_owned_externally`:
/// a body another authority is posing takes the floor-game consequence now and
/// the travel later, because the authority that owns its position would erase
/// the travel this tick.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LaunchTravel {
    Applied,
    Deferred,
}

/// Put a staged launch through the one gateway, and report whether it STARTED A
/// TUMBLE.
///
/// ⭐⭐ THE ANSWER IS WHAT AN EXTERNAL POSE OWNER NEEDS. A launch that tumbles is
/// the one that will END the constraint, so its travel has to survive to the
/// tick the body is free; a launch that does not tumble is ABSORBED by the
/// constraint and must be retired, or it waits — forever, until something
/// unrelated releases the pose — and then fires. See the caller.
fn accept_external_launch(
    model: &mut MotionModel,
    clusters: &mut BodyClustersMut<'_>,
    ctx: &MotionStepContext<'_>,
    launch: crate::body_clusters::PendingLaunch,
    travel: LaunchTravel,
) -> bool {
    if launch.is_empty() {
        return false;
    }
    let flinchless = launch.flinchless;
    let launch = launch.velocity;
    match model {
        // Assigning again is deliberate rather than redundant: it makes the launch channel the
        // single story for every model, so a reader does not have to know which arm secretly relies
        // on a second write somewhere else.
        MotionModel::AxisSwept(axis) => {
            // ⛔ THE JAB LOCK IS ASKED FIRST, because a pin is the ABSENCE of a
            // launch: a body already prone that takes a weak hit is re-pinned
            // where it lies, and assigning the velocity before asking would
            // slide it away from the spot the attacker is standing over.
            // ⛔⛔ AND A PUSH DECIDES NO FLOOR GAME. Both questions below were
            // asked from SPEED ALONE, so a weak gust pinned a prone body where
            // it lay and a strong one sent it tumbling — against a volume whose
            // authored contract is *"moves you and leaves you in control"*. The
            // KIND is the only thing that separates them, and it had to be
            // carried here because a bare `Vec2` says nothing about it.
            if !flinchless
                && super::knockdown::jab_lock(&mut axis.state, axis.params, launch.length())
            {
                // Pinned where it lies: the launch is spent on the pin and there
                // is no travel left to defer.
                return false;
            }
            if travel == LaunchTravel::Applied {
                clusters.kinematics.vel = launch;
            }
            // and the floor game starts HERE, for the same reason the drain
            // is here. "Was this launch big enough to send the body tumbling"
            // is a question only the model can answer — the threshold is authored
            // per body, and maneuver state is model-private (ADR 0024) — and the
            // reaction that resolved the knockback holds neither. Asking it at
            // the one gateway every launch already passes through is what keeps
            // it from being a follow-up call some caller forgets.
            if !flinchless
                && super::knockdown::launch_into_tumble(
                    &mut axis.state,
                    axis.params,
                    launch.length(),
                )
            {
                // Without it a launched body carried its stale resting contact into the same
                // step's `tick_knockdown`, which read `on_ground == true`, called that a
                // landing *while still tumbling*, and resolved the whole thing to a KNOCKDOWN —
                // `kinematics.vel = ZERO` on the tick the launch was applied.
                //
                // gated on the tumble answer rather than on the launch's
                // direction on purpose: a shove that does not throw you leaves
                // you planted, and a body whose authored `tumble_speed` is `0.0`
                // — every body in Ambition today — is byte-identical to before.
                clusters.ground.on_ground = false;
                return true;
            }
            return false;
        }
        MotionModel::SurfaceMomentum(momentum) => {
            let mut body = SurfaceBody {
                pos: clusters.kinematics.pos,
                vel: clusters.kinematics.vel,
                radius: clusters.kinematics.size.min_element() * 0.5,
                depth_lane: momentum.depth_lane,
                motion: momentum.state,
                route_memory: momentum.route_memory,
                occlusions: momentum.occlusions,
            };
            surface_momentum::apply_external_launch(ctx.world, &mut body, launch, ctx.dt);
            if travel == LaunchTravel::Applied {
                clusters.kinematics.vel = body.vel;
                momentum.state = body.motion;
                momentum.occlusions = body.occlusions;
            }
        }
        MotionModel::AdhesiveCrawler(_) => {
            if travel == LaunchTravel::Applied {
                clusters.kinematics.vel = launch;
            }
        }
    }
    // ⛔ NEITHER OF THE OTHER TWO MODELS HAS A FLOOR GAME, so neither can start
    // a tumble, so neither has a launch worth banking past a constraint.
    false
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
        route_memory: motion.route_memory,
        occlusions: motion.occlusions,
    };
    let mut contacts = Vec::new();
    surface_momentum::step_surface_body(
        &mut body,
        ctx.world,
        &motion.params,
        ctx.frame,
        SurfaceInputs {
            local_axes: ctx.input.axes,
            jump_pressed: ctx.input.jump_pressed(),
        },
        ctx.dt,
        Some(&mut contacts),
    );

    clusters.kinematics.pos = body.pos;
    // Rebound pads are a world gate, like hazards — not follower collision.
    // The axis arm drains the same lookup in its integration step. Drained on
    // the SurfaceBody itself so a spring pad can launch a rider airborne
    // (with the occlusion bookkeeping every launch records).
    if let Some(impulse) = touching_rebound_aabb(ctx.world, clusters.kinematics.aabb()) {
        surface_momentum::apply_pad_impulse(ctx.world, &mut body, impulse, ctx.dt);
    }
    clusters.kinematics.vel = body.vel;
    if ctx.facing_intent.abs() > 0.001 {
        clusters.kinematics.facing = ctx.facing_intent.signum();
    }
    clusters.ground.on_ground = body.riding();
    motion.state = body.motion;
    motion.depth_lane = body.depth_lane;
    motion.route_memory = body.route_memory;
    motion.occlusions = body.occlusions;
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
    // the edge is derived HERE, from the attachment either side of the step,
    // rather than pushed at the eight places inside `step_crawler` that detach or
    // re-attach. Those eight are the "second step every call site has to
    // remember" shape this engine keeps paying for, and `step_crawler` has
    // several early returns, so an emit-at-the-end rule inside it would silently
    // skip exactly the paths that exit early. One derivation, no path to miss.
    let was_attached = motion.state.is_attached();
    adhesive_crawler::step_crawler(
        motion,
        ctx.world,
        clusters,
        ctx.frame,
        ctx.facing_intent,
        ctx.dt,
        &mut events.contacts,
        &mut events.constraint_conflicts,
    );
    match (was_attached, motion.state.is_attached()) {
        (false, true) => events.operations.push(super::MovementOp::CrawlAttach),
        (true, false) => events.operations.push(super::MovementOp::CrawlDetach),
        _ => {}
    }
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

/// "ONE" is now true.
pub(crate) fn apply_world_hazard_gate(
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
    // How far outside the world this body is, resolved in ITS OWN frame, so
    // "below" means "past the edge gravity pulls toward" under any gravity.
    let outside = pos - clamped;
    let past_fall = outside.dot(frame.down());
    let past_side = outside.dot(frame.side()).abs();

    // The fall direction always kills — that is a pit, and every room has one
    // whether or not it wanted one. The other two are OPT-IN, because they mean
    // opposite things in the two genres this engine has to serve: a platformer
    // walking off the left edge of a room is a ROOM TRANSITION, and killing
    // there would break every corridor in the game, while a platform fighter
    // thrown off the left edge has lost a stock. `None` is "this direction is
    // not a blast zone", and it is the default.
    // ⛔ EXHAUSTIVE on purpose. A fourth axis must be a compile error here rather
    // than a comparison somebody forgot to add — a missing margin is SILENT, and
    // the room looks fine right up until a body drifts out of it forever.
    let crate::world::WorldEdgeMargins { fall, side, rise } = &world.edges;
    let left_the_world = past_fall > *fall
        || side.is_some_and(|margin| past_side > margin)
        || rise.is_some_and(|margin| -past_fall > margin);

    // Order matters, and it is a design statement: a body that is BOTH past an
    // edge margin and overlapping a hazard left the world. The void is
    // further out than any authored volume, so it is the later, larger fact.
    if left_the_world {
        events.reset = events.reset.or(Some(ResetCause::LeftTheWorld));
    } else if touching_hazard(world, clusters) {
        events.reset = events.reset.or(Some(ResetCause::Hazard));
    }
}

/// Hazard contact for the shared gate: the tick's travelled path when the body
/// has a `SweepSample`, its endpoint AABB when it does not.
///
/// ⛔ THE NO-SAMPLE ARM IS THE DISCRETE TEST, DELIBERATELY, and it is the only
/// compatibility behaviour here. The alternative — rebuilding a segment from
/// `vel * dt` — would be a SECOND motion model living next to the kernel's, free
/// to disagree with it: velocity at phase entry does not describe a step that a
/// collision resolve shortened, so that arm would report a hazard on a path the
/// body never travelled. A body with no sample is a body nothing swept, and the
/// endpoint is the only thing actually known about it. `SweepSample`'s
/// `TODO(compat-remove)` is the plan to delete this arm outright.
///
/// Teleports need no exclusion of their own: the sample is captured from
/// simulation-phase entry to phase exit, so a later system's blink or room
/// transfer is not inside `prev -> curr` in the first place.
fn touching_hazard(world: &World, clusters: &BodyClustersMut<'_>) -> bool {
    // The endpoint the body is at NOW is always tested — a teleport that lands
    // inside a hazard is standing in one, whatever path preceded it.
    if touching_hazard_aabb(world, clusters.kinematics.aabb()) {
        return true;
    }
    clusters
        .sweep
        .as_ref()
        .is_some_and(|sweep| hazard_contact_on_path(world, sweep.curr, sweep.half, sweep.delta()))
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
        return SupportFact::Attached(contact.clone());
    }
    contacts
        .iter()
        .rev()
        .find(|contact| contact.kind == ContactKind::Support)
        .map(|contact| SupportFact::Supported(contact.clone()))
        .unwrap_or(SupportFact::Airborne)
}

#[cfg(test)]
mod tests;
