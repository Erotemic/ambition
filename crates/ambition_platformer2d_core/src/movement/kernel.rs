//! Unified, frame-aware movement-kernel facade.
//!
//! Every movement policy consumes the same body clusters, typed local-input
//! artifact, world, current [`MotionFrame`](crate::MotionFrame), and timestep.
//! The environment resolves that frame once from an explicit reference basis
//! plus the complete world-space acceleration for the body tick. It never lives
//! inside a model spec and is never rebuilt by an individual solver.

use crate::collision_semantics::{Contact, ContactKind};
use crate::frame::thread::MotionAdmission;
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
    pub admission: Option<crate::frame::thread::MotionAdmission<'a>>,
    /// The typed, already-frame-resolved motion intent (see [`InputState`]).
    pub input: InputState,
    /// The body's current acceleration/reference frame. Every policy arm
    /// receives this exact value; none is permitted to derive a private
    /// gravity frame.
    pub frame: MotionFrame,
    /// Desired facing in control axes. A crawler uses its oriented route.
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
    /// A surface carries the body with its local orientation.
    Riding(Contact),
    /// Adhesively attached (policy-owned cling); the normal is the attachment,
    /// deliberately independent of the frame's pull.
    Attached(Contact),
}

impl SupportFact {
    /// The outward support/attachment normal, if any surface holds the body.
    pub fn normal(&self) -> Option<Vec2> {
        match self {
            SupportFact::Airborne => None,
            SupportFact::Supported(contact) | SupportFact::Riding(contact) | SupportFact::Attached(contact) => {
                Some(contact.normal)
            }
        }
    }

    /// The supporting/attached contact, if any.
    pub fn contact(&self) -> Option<&Contact> {
        match self {
            SupportFact::Airborne => None,
            SupportFact::Supported(contact) | SupportFact::Riding(contact) | SupportFact::Attached(contact) => Some(contact),
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
    let retained_down = SweepSample::down_or(clusters.sweep.as_deref(), ctx.frame.down());
    let refusal = match try_motion(model, clusters, ctx, retained_down) {
        Ok(result) => return result,
        Err(refusal) => refusal,
    };
    // ⭐ A REFUSAL LEAVES THE WORLD PLAYABLE (PSIM-5). A refused step used to
    // keep the old body whole, velocity and all, and every later tick asked
    // the same refused question again: the body hung in the air at full
    // speed while the clock ran on, paying the refused search each tick.
    // A refusal is about this tick's portal material, so the step is taken
    // again with every portal closed to this body: their hosts are solid, as
    // the design's "prevent the crossing at the solid portal boundary" asks,
    // so the body is stopped or carried by real geometry and never tunnels.
    // Closing is the previous admitted world only while no material is
    // already through a portal: a straddling body would sit inside the
    // closed hosts, and repair would throw it out of them.
    // The refusal is still reported on the step that recovered.
    //
    // A body already through a pair keeps that pair open and meets every
    // other one as its host's solid: a body in J's first cut, whose far side
    // runs into the second cut beyond its hop budget, stops at the second
    // cut's backing instead of being refused there every tick.
    if let Some(admission) = ctx.admission.filter(|admission| !admission.links.is_empty()) {
        let retry = match straddled_links(clusters.kinematics, admission) {
            Some(keys) if keys.is_empty() => Some(Vec::new()),
            Some(keys) => {
                let kept: Vec<_> = admission.links.iter()
                    .filter(|link| keys.contains(&link.key) || keys.contains(&(link.key ^ 1))).copied().collect();
                (kept.len() < admission.links.len()).then_some(kept)
            }
            None => None,
        };
        if let Some(kept) = retry {
            let narrowed = (!kept.is_empty()).then(|| MotionAdmission { links: &kept, first: admission.first.filter(|first| kept.iter().any(|link| link.key == *first)), ..admission });
            let retried = MotionStepContext { admission: narrowed, ..ctx };
            if let Ok(mut result) = try_motion(model, clusters, retried, retained_down) {
                result.events.motion_refusal = Some(refusal);
                return result;
            }
        }
    }
    // Nothing admits a motion: the body keeps its place and stops, so the
    // next tick asks a different question rather than the same one forever.
    halt(model, clusters);
    if let Some(sample) = clusters.sweep.as_deref_mut() {
        *sample = SweepSample::at_tick_end(*clusters.kinematics, retained_down);
    }
    let mut events = FrameEvents::default();
    events.motion_refusal = Some(refusal);
    MotionStepResult::from_events(events, ctx.frame)
}

/// One candidate step and its righting. Commits only when both are admitted.
fn try_motion(
    model: &mut MotionModel,
    clusters: &mut BodyClustersMut<'_>,
    ctx: MotionStepContext<'_>,
    retained_down: Vec2,
) -> Result<MotionStepResult, crate::frame::thread::Refusal> {
    let mut candidate = crate::body_clusters::BodyClusterScratch::candidate(model, clusters);
    let mut sweep = SweepSample::at_rest(*clusters.kinematics, retained_down);
    let result = {
        let (candidate_model, mut body) = candidate.parts();
        body.sweep = Some(&mut sweep);
        body.carrier = clusters.carrier;
        step_admitted_motion(candidate_model, &mut body, ctx)
    };
    if let Some(refusal) = result.events.motion_refusal.clone() {
        return Err(refusal);
    }
    let target = match &result.support {
        SupportFact::Riding(contact) | SupportFact::Attached(contact) => -contact.normal,
        _ => ctx.frame.down(),
    };
    let max_step = if ctx.pose_owned_externally { 0.0 }
        else if matches!(result.support, SupportFact::Riding(_) | SupportFact::Attached(_)) {
            std::f64::consts::PI
        } else { 8.0 * f64::from(ctx.dt.max(0.0)) };
    super::admit_body_righting(&mut candidate.kinematics, &mut sweep,
        target, max_step, f64::from(ctx.dt), ctx.world, ctx.admission,
        &[ctx.frame.carried_acceleration(), ctx.frame.basis().side * candidate.flight.carried_run], clusters.carrier)?;
    candidate.commit(model, clusters);
    if let Some(target) = clusters.sweep.as_deref_mut() { *target = sweep; }
    Ok(result)
}

/// The links the body's material is through now; empty when none is.
/// `None` when its present partition cannot be admitted: no narrower world
/// can be chosen for it.
fn straddled_links(body: &crate::BodyKinematics, admission: MotionAdmission<'_>) -> Option<Vec<u32>> {
    use crate::frame::thread::{MaterialPrimitive, partition};
    let (Ok(domain), Ok(to_world)) = ((*body).canonical_box(), (*body).material_map()) else { return None };
    let pieces = partition(MaterialPrimitive { domain: &domain, to_world }, body.pos, admission.links,
        admission.first, admission.budget, admission.convention).ok()?;
    let mut keys: Vec<u32> = pieces.iter().flat_map(|piece| piece.path.iter().copied()).collect();
    keys.sort_unstable();
    keys.dedup();
    Some(keys)
}

/// Stop a body where it is: its velocity, a rider's speed along its surface,
/// and a hang or climb that places it by its own clock. A climb that is
/// refused every tick never advances that clock and takes no input, so it is
/// let go and the next tick is the player's (the hang's own drop: the re-grab
/// cooldown keeps it from latching the same ledge at once).
fn halt(model: &mut MotionModel, clusters: &mut BodyClustersMut<'_>) {
    clusters.kinematics.vel = Vec2::ZERO;
    match model {
        MotionModel::SurfaceMomentum(momentum) => { momentum.set_tangential_speed(0.0); }
        MotionModel::AxisSwept(axis) if axis.state.ledge_grab.is_some() => {
            axis.state.ledge_grab = None;
            axis.state.wall_clinging = false;
            axis.state.wall_climbing = false;
            clusters.wall.on_wall = false;
            clusters.ledge.release_cooldown = crate::ledge_grab::LEDGE_REGRAB_COOLDOWN;
        }
        _ => {}
    }
}

fn step_admitted_motion(
    model: &mut MotionModel,
    clusters: &mut BodyClustersMut<'_>,
    ctx: MotionStepContext<'_>,
) -> MotionStepResult {
    let material = clusters.kinematics.material;
    if material.scale() == 1.0 { return step_world_motion(model, clusters, ctx); }
    enum Authored {
        Axis(crate::AxisSweptParams),
        Momentum(crate::MomentumParams),
        Crawler(crate::CrawlerParams),
    }
    let prepare = |model: &mut MotionModel| -> Result<Authored, crate::frame::NumericFault> {
        Ok(match model {
            MotionModel::AxisSwept(axis) => { let canonical = axis.params; axis.params = canonical.for_material(material)?; Authored::Axis(canonical) }
            MotionModel::SurfaceMomentum(body) => { let canonical = body.params; body.params = canonical.for_material(material)?; Authored::Momentum(canonical) }
            MotionModel::AdhesiveCrawler(body) => { let canonical = body.params; body.params = canonical.for_material(material)?; Authored::Crawler(canonical) }
        })
    };
    let authored = match prepare(model) {
        Ok(authored) => authored,
        Err(fault) => {
            let events = FrameEvents { motion_refusal: Some(crate::frame::thread::Refusal {
                path: Vec::new(), link: None, reason: crate::frame::thread::RefusalReason::Numeric(fault),
            }), ..Default::default() };
            return MotionStepResult::from_events(events, ctx.frame);
        }
    };
    let result = step_world_motion(model, clusters, ctx);
    match (authored, model) {
        (Authored::Axis(params), MotionModel::AxisSwept(axis)) => axis.params = params,
        (Authored::Momentum(params), MotionModel::SurfaceMomentum(body)) => body.params = params,
        (Authored::Crawler(params), MotionModel::AdhesiveCrawler(body)) => body.params = params,
        _ => unreachable!("a movement step keeps its selected policy"),
    }
    result
}

fn step_world_motion(
    model: &mut MotionModel,
    clusters: &mut BodyClustersMut<'_>,
    ctx: MotionStepContext<'_>,
) -> MotionStepResult {
    let facing_intent = model.project_facing(clusters.kinematics.material, ctx.facing_intent, ctx.frame.side());
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
    let mut result = match model {
        MotionModel::AxisSwept(axis) => {
            let events = super::update_body_with_frame_clusters(
                ctx.world,
                axis,
                clusters,
                ctx.input,
                facing_intent,
                ctx.frame,
                ctx.dt,
                ctx.contact,
                ctx.recovery_commitment_outstanding,
                ctx.pose_owned_externally,
                ctx.admission,
            );
            MotionStepResult::from_events(events, ctx.frame)
        }
        MotionModel::SurfaceMomentum(momentum) => {
            let baseline = establish_ground_contact_baseline_from_sample(
                clusters,
                matches!(momentum.state, super::SurfaceMotion::Riding { .. }),
                ctx.frame,
            );
            let mut result = step_surface_momentum(momentum, clusters, ctx, facing_intent);
            result.events.ground_contact = baseline.transition_to(clusters.ground.on_ground);
            result
        }
        MotionModel::AdhesiveCrawler(crawler) => {
            let baseline = establish_ground_contact_baseline_from_sample(
                clusters,
                crawler.state.is_attached(),
                ctx.frame,
            );
            let mut result = step_adhesive_crawler(crawler, clusters, ctx, facing_intent);
            result.events.ground_contact = baseline.transition_to(clusters.ground.on_ground);
            result
        }
    };
    // ⛔⛔ A HELD BODY DOES NOT LAND. Where it is was decided by its holder, so a
    // surface it meets there is not an arrival of its own. A saddle that pinned
    // its rider into a ceiling had the kernel eject her up onto the ceiling's
    // top face, invalidated the baseline (correctly), and put her back — a
    // `Landed` edge, and a `Land` sound, on every tick of the fight (MEASURED:
    // 1,739 in 30 s in `pirate_sky_lookout`). The contact FACT is kept; only the
    // edge that claims an arrival is withheld. A body landing after its holder
    // lets go lands on the first free tick, as it should.
    if ctx.pose_owned_externally
        && matches!(result.events.ground_contact, GroundContactTransition::Landed { .. })
    {
        result.events.ground_contact = GroundContactTransition::Unchanged;
    }
    result
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
    admission: Option<crate::frame::thread::MotionAdmission<'_>>,
) -> Result<GroundContactBaseline, crate::frame::thread::Refusal> {
    let sampled_grounded = if clusters.ground.contact_initialized { false }
        else { super::collision::material_support_at_rest(world, clusters, frame.down(), admission)? };
    Ok(establish_ground_contact_baseline_from_sample(clusters, sampled_grounded, frame))
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
            // A hit gives the ledge grabs back. A launch that is not
            // flinchless is the hit that charges hitstun (`hit_reaction`), so
            // this is the one site that sees every such hit. A push does not:
            // it leaves the body in control.
            if !flinchless {
                axis.state.ledge_grabs = 0;
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
                    launch,
                    clusters.kinematics.facing,
                    ctx.frame,
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
                radius: surface_momentum::body_radius(clusters.kinematics.size()),
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
    facing_intent: f32,
) -> MotionStepResult {
    let sweep_entry = (clusters.kinematics.pos, clusters.kinematics.vel, clusters.kinematics.material);
    let mut body = SurfaceBody {
        pos: clusters.kinematics.pos,
        vel: clusters.kinematics.vel,
        radius: surface_momentum::body_radius(clusters.kinematics.size()),
        depth_lane: motion.depth_lane,
        motion: motion.state,
        route_memory: motion.route_memory,
        occlusions: motion.occlusions,
    };
    let mut contacts = Vec::new();
    let params = motion.effective_params();
    let jump_pressed = ctx.input.jump_pressed();
    let was_riding = body.riding();
    surface_momentum::step_surface_body(
        &mut body,
        ctx.world,
        &params,
        ctx.frame,
        SurfaceInputs {
            local_axes: ctx.input.axes,
            jump_pressed,
        },
        ctx.dt,
        Some(&mut contacts),
    );
    // A rider that pressed jump and left the surface jumped: the riding step
    // takes the jump before anything else can detach it. Derived from the
    // transition so listeners (jump sound, dust, a ball-up) hear this model's
    // jumps as they hear the axis model's.
    let jumped = was_riding && jump_pressed && !body.riding();

    clusters.kinematics.pos = body.pos;
    // Rebound pads are a world gate, like hazards — not follower collision.
    // The axis arm drains the same lookup in its integration step. Drained on
    // the SurfaceBody itself so a spring pad can launch a rider airborne
    // (with the occlusion bookkeeping every launch records).
    //
    // The ride circle is the body's extent along the frame's DOWN, so the DOWN
    // of this body is the frame's. The pad and the hazard gate test that box.
    let body_down = ctx.frame.down();
    let step_half = clusters.kinematics.half_extent();
    if let Some(impulse) = touching_rebound_aabb(ctx.world, crate::Aabb::new(clusters.kinematics.pos, step_half)) {
        surface_momentum::apply_pad_impulse(ctx.world, &mut body, impulse, ctx.dt);
    }
    clusters.kinematics.vel = body.vel;
    if facing_intent.abs() > 0.001 {
        clusters.kinematics.facing = facing_intent.signum();
    }
    clusters.ground.on_ground = body.riding();
    // The wall FACT, from the same contacts, by the axis arm's rule: a rider
    // stopped by a block in its path is against a wall. Without it a walker on
    // this model never learned a wall was there and pressed into it forever.
    clusters.wall.on_wall = false;
    for contact in contacts
        .iter()
        .filter(|c| c.kind != crate::collision_semantics::ContactKind::Support)
    {
        super::collision::apply_side_contact(clusters.wall, contact.normal, ctx.frame.down());
    }
    motion.state = body.motion;
    motion.depth_lane = body.depth_lane;
    motion.route_memory = body.route_memory;
    motion.occlusions = body.occlusions;
    motion.spend_boost(ctx.dt);
    write_sweep_sample(clusters, sweep_entry, body_down);

    let mut events = FrameEvents {
        contacts,
        ..FrameEvents::default()
    };
    if jumped {
        events.operations.push(super::MovementOp::Jump);
    }
    apply_world_hazard_gate(ctx.world, clusters, ctx.frame, step_half, &mut events);

    let mut result = MotionStepResult::from_events(events, ctx.frame);
    if body.riding() {
        if let SupportFact::Supported(contact) = result.support {
            result.support = SupportFact::Riding(contact);
        }
    }
    result
}

fn step_adhesive_crawler(
    motion: &mut super::AdhesiveCrawlerMotion,
    clusters: &mut BodyClustersMut<'_>,
    ctx: MotionStepContext<'_>,
    facing_intent: f32,
) -> MotionStepResult {
    let sweep_entry = (clusters.kinematics.pos, clusters.kinematics.vel, clusters.kinematics.material);
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
        facing_intent,
        ctx.input.axes.x,
        ctx.dt,
        &mut events.contacts,
        &mut events.constraint_conflicts,
        ctx.admission,
        &mut events.motion_refusal,
    );
    match (was_attached, motion.state.is_attached()) {
        (false, true) => events.operations.push(super::MovementOp::CrawlAttach),
        (true, false) => events.operations.push(super::MovementOp::CrawlDetach),
        _ => {}
    }
    let step_half = write_sweep_sample(clusters, sweep_entry, motion.body_down(ctx.world, ctx.frame));
    apply_world_hazard_gate(ctx.world, clusters, ctx.frame, step_half, &mut events);

    MotionStepResult::from_events(events, ctx.frame)
}

/// §3.1 motion record of one step: both endpoints captured inside the kernel,
/// so position changes outside this window are excluded from the record by
/// construction. `entry` is the position and velocity at the start of the
/// step.
///
/// `body_down` is the DOWN of the body in this step. Each policy arm states
/// it. The box the step moved is the body's box turned to it, and this
/// returns that box: the arm gives the same value to
/// [`apply_world_hazard_gate`], so the record, the end test and the path test
/// are of one box. The record keeps the DOWN for a later transit.
pub(crate) fn write_sweep_sample(clusters: &mut BodyClustersMut<'_>, entry: (Vec2, Vec2, crate::frame::BodyMaterial), body_down: Vec2) -> Vec2 {
    let curr = clusters.kinematics.pos;
    let step_half = clusters.kinematics.half_extent();
    if let Some(sweep) = clusters.sweep.as_deref_mut() {
        *sweep = SweepSample { turn: None, travel: Default::default(), time: [0.0, 1.0],
            material: entry.2,
            prev: entry.0,
            curr,
            vel: entry.1,
            half: step_half,
            down: body_down,
        };
    }
    step_half
}

/// "ONE" is now true.
///
/// `step_half` is the box the step moved, the same value its policy arm gave
/// to [`write_sweep_sample`].
pub(crate) fn apply_world_hazard_gate(
    world: &World,
    clusters: &mut BodyClustersMut<'_>,
    frame: MotionFrame,
    step_half: Vec2,
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
    } else if touching_hazard(world, clusters, step_half) {
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
/// endpoint is the only thing actually known about it — and that is a
/// permanent shape, not a migration residue: the fighter brain's recovery
/// planner runs this kernel over a scratch body that is not an entity, so a
/// sampleless mover will always exist. The ECS hazard and room-transition
/// readers one layer up DID rebuild the segment this paragraph forbids; they
/// no longer do.
///
/// Teleports need no exclusion of their own: the sample is captured from
/// simulation-phase entry to phase exit, so a later system's blink or room
/// transfer is not inside `prev -> curr` in the first place.
///
/// PARITY WITH THE DISCRETE TEST LIVES HERE, in the pair rather than in either
/// half: the endpoint arm below is `strict_intersects` against the hazard's real
/// AABB and carries no epsilon, so every hit the old discrete gate found is
/// still a hit. `hazard_contact_on_path` then adds travelled path and nothing
/// else — it queries the hazard's interior precisely so that it cannot add
/// surface contact.
///
/// ONE BOX. The end test and the path test are of `step_half`, the box the
/// step moved. The level box (`BodyKinematics::aabb`) is a different box for a
/// body that lies along a wall: a slug 48 long on a wall was hit by a hazard
/// beside it, and was not hit by a hazard under its own end.
fn touching_hazard(world: &World, clusters: &BodyClustersMut<'_>, step_half: Vec2) -> bool {
    // The endpoint the body is at NOW is always tested — a teleport that lands
    // inside a hazard is standing in one, whatever path preceded it.
    if touching_hazard_aabb(world, crate::Aabb::new(clusters.kinematics.pos, step_half)) {
        return true;
    }
    clusters
        .sweep
        .as_ref()
        .is_some_and(|sweep| hazard_contact_on_path(world, sweep.curr, step_half, sweep.delta()))
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
