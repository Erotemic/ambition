//! **Can THIS body still get back — asked of the real movement kernel.**
//!
//! `ambition_platformer2d_core::movement::recovery` answers *"is this position
//! recoverable by this body under its own capabilities"* by cloning the body,
//! handing it to `step_motion` and watching. It states no rule about bodies at
//! all, which is exactly why it can answer for a flyer, a wall-clinger, a
//! ledge-grabber and a fighter with one unspent air jump without a list here to
//! fall out of date.
//!
//! This module is the LOWERING that lets the fighter brain ask it: a
//! [`Perceived`](crate::perception::Perceived) view becomes an
//! [`ae::World`], the body's own kit becomes an [`ae::BodyClusterScratch`], and
//! the answer comes back as an [`ae::movement::recovery::RecoveryOutlook`].
//!
//! ⭐ **and it stops there.** The query reports what physically happened; what a
//! `NoSupportFoundBy` MEANS is the brain's business, and the brain spends it in
//! exactly one place ([`super::rollout::refine_by_rollout`]) as a veto on
//! movement lines. There is no fighting-game rule in here and no capability list
//! in the rollout — the split `FrameEvents` already draws for contacts.
//!
//! ⛔⛔ **AND THE VETO IS BOUNDED BY A SEARCH POLICY — say it out loud, because it
//! used to be implicit.** The outcome is `NoSupportFoundBy { search, .. }`, and
//! this lens probes under `RecoveryPolicy::DRIFT_AND_JUMP`, which presses
//! **only** `side ∈ {0, -1, +1}` plus jump. A body that recovers by
//! dash, blink, flight, wall verb or ledge grab is **not**
//! explored, so a negative here means *"this steering policy found nothing
//! within this horizon"* — ⛔ never *"this body cannot get back."*
//!
//! ⭐ **the recovery ATTACK is the one that is no longer missing, and the
//! header's own warning is why.** It said this bound *"is the first thing to
//! re-check the day a fighter gains one"*, and that day arrived: a body whose
//! repertoire commands a displacement hands it to the probe as a
//! [`RecoveryLift`], the policy becomes `drift+jump+burst`, and the veto is
//! taken against the body that will actually be throwing its recovery. Without
//! it the rollout condemned exactly the lines a real recovery saves. Read the
//! bound off the value with `RecoveryOutlook::bounded_by()` rather than assuming
//! it; a positive returns `None`, because finding a route proves one exists
//! while failing to find one is only ever a claim about the searcher.
//!
//! ⭐⭐ **AND THE ROUTES ARE SEARCHED, NOT RANKED — this is the module's
//! architectural claim.** The first version took ONE route: whichever authored
//! move advertised the largest against-gravity speed, picked before anything
//! knew where the body was. That is a static property standing in for a question
//! about the current state, and it fails in exactly the way a static property
//! fails — a fighter whose way home is a grapple that trades its energy for
//! lateral distance advertises a small rise, so a tiny rising aerial outranks it
//! and becomes "the recovery" for every layer downstream. [`RecoveryLens::best_route`]
//! runs the buttons-only baseline and then each route the body owns, and reports
//! the FIRST that gets home. The kernel decides which action is useful; the
//! repertoire only proposes.
//!
//! ## What the lowering claims, and what it therefore cannot see
//!
//! * **the terrain is the PERCEIVED terrain**, viewport-clipped like everything
//!   else a brain knows. A platform the body cannot see is a platform it cannot
//!   plan to land on, which is the honest answer rather than a cheat.
//! * **the envelope is the stage box, with every blast margin at zero.** That is
//!   the SAME death line [`crate::perception::StageView::offstage`] draws and the
//!   same one `shadow_step`'s KO fires on — one model of dying, not two. A stage
//!   that authors a generous margin is therefore judged conservatively (the probe
//!   believes it dies sooner than it does), which is the safe direction: a
//!   rollout that overestimates recovery certifies dives that will not come back.
//! * **nothing that moves the body from OUTSIDE the kernel is modelled** —
//!   portals, grapples, a launch that has not happened yet. `probe_recovery`'s
//!   own header says so; it is a gap of the kernel, not an assumption of this
//!   lowering. (A recovery move used to be on that list. It came off it: an
//!   authored move STATES the speed it commands, so it is expressible as a
//!   `RecoveryBurst` and no longer has to be invisible.)

use ambition_platformer2d_core as ae;

use crate::perception::{SolidKind, WorldView};

/// **How long the probe watches.**
///
/// Sized to the QUESTION, not to a stage: an air jump's rise plus the fall that
/// follows it. At the engine baseline a jump takes ~0.29 s to its apex and a
/// body crosses a 480 px room in ~0.65 s, so two seconds contains a full
/// recovery attempt with room to spare — and a probe that stops before the body
/// would have landed reports "no support" for the wrong reason, which is the
/// exact arithmetic that broke this rollout in the first place.
pub const RECOVERY_PROBE_SECONDS: f32 = 2.0;

/// **The body's own kit, in the movement kernel's vocabulary.**
///
/// Both halves are body-derived truth arriving through the world-in port, beside
/// each other because the kernel needs both: the [`ae::AbilitySet`] says which
/// verbs exist and the [`ae::MovementTuning`] says what they are worth. A
/// heavier fighter's gravity and a lighter one's extra jump are the same kind of
/// fact and neither is guessed here.
///
/// ⚠ **this is NOT the brain reading capability flags to re-derive a rule** —
/// the failure mode [`crate::perception::SelfView::burst`] records. Nothing here
/// interprets the set; it is handed straight to the kernel, which is the
/// authority on what a body owning these verbs can do.
///
/// ⚠ **the ROUTES a body could press are NOT in here, and that split is the
/// point.** A kit is what a body IS; a route is one thing it could DO from a
/// particular place. Keeping them apart is what stopped the lens from having to
/// pick a body's recovery move before it knew where the body was.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyKit {
    pub abilities: ae::AbilitySet,
    pub movement: ae::MovementTuning,
}

/// **One displacement a body's repertoire can command, in the terms a
/// [`RecoveryProbe`](ae::movement::recovery::RecoveryProbe) needs.**
///
/// Derived from a move's frame data by
/// [`lifting_candidates`](super::options::lifting_candidates); nothing here
/// names a move, a verb or a character.
///
/// ⛔ **a route is a PROPOSAL, never a claim.** Holding one says the body can
/// throw this displacement, and says nothing about whether throwing it helps —
/// that question belongs to [`RecoveryLens::best_route`], which asks the kernel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecoveryLift {
    /// Engine units per second, against gravity.
    pub speed: f32,
    /// **Engine units per second along the body's FACING**, the other half of
    /// the same commanded velocity (`MoveFrameData::lift_side`). Negative for a
    /// move that hauls its owner backwards — the recoil of firing forwards is a
    /// real authored shape.
    ///
    /// ⭐ zero for a straight-up recovery, which is why every seat that had one
    /// is unaffected. It was hardcoded to zero here for one slice, with the
    /// reason *"the move commands a rise; where the body goes sideways is the
    /// drift's business"* — true of a vertical Up-B and false of every recovery
    /// that gets home by crossing rather than climbing. A `Set` impulse
    /// overrides the drift outright, so a probe that dropped this half searched
    /// a move the body cannot throw.
    pub side: f32,
    /// Proper-time seconds from the press to the burst — the windup the body has
    /// to survive first. A probe that ignored it would certify recoveries the
    /// real move is too slow to make.
    pub after_s: f32,
}

/// Where a rolled line left the ground, in the terms the kernel needs to take it
/// from there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecoveryQuery {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    /// Mid-air jumps this line has NOT spent. A line that burned its jump
    /// getting into trouble is asked about the body it left behind, not the one
    /// it started as.
    pub air_jumps_left: u8,
}

/// **How many authored routes one lens will spend kernel time on.**
///
/// ⚠ a bound, not a taste. Each route is a whole `probe_recovery` — three
/// efforts of up to [`RECOVERY_PROBE_SECONDS`] — and the lens is queried once
/// per rolled movement line, so the cost is linear in this number. Three plus
/// the buttons-only baseline is four searches per query, which is what a body
/// with a real repertoire needs (a recovery, a stall, and one more) and far
/// short of the dozen a full kit would offer.
///
/// ⛔ the routes are ordered by [`super::options::lifting_candidates`] and the
/// cut is a prefix of that order, so which three get probed never depends on
/// iteration luck (ADR 0023).
pub const MAX_PROBED_ROUTES: usize = 3;

/// **Which route got the body home, and what the search saw.**
///
/// ⭐ `route` is the whole product: `None` with a positive outlook means *"you
/// are getting back without throwing anything"*, `Some(i)` names the route that
/// worked, and `None` with a negative means no search this lens ran found
/// anything. Those are three different instructions to a caller and the old
/// boolean could express one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RouteVerdict {
    pub outlook: ae::movement::recovery::RecoveryOutlook,
    /// Index into the routes this lens was built with.
    pub route: Option<usize>,
}

impl RouteVerdict {
    pub fn regained(self) -> bool {
        self.outlook.regained()
    }
}

/// A perceived stage plus a body's kit and the routes it could press, ready to
/// answer recovery questions.
///
/// Built ONCE per decision and queried per rolled line: the world lowering is
/// the expensive half (it allocates one [`ae::Block`] per perceived solid) and
/// it does not change between the lines of a single decision.
#[derive(Clone, Debug)]
pub struct RecoveryLens {
    world: ae::World,
    /// The stage box's own origin, subtracted from everything. `ae::World` is a
    /// `0..size` box while a [`crate::perception::StageView`] is an AABB
    /// anywhere, so the lowering TRANSLATES rather than assuming the two agree.
    /// Translation changes no distance, no velocity and no gravity direction, so
    /// the verdict is unchanged by it.
    origin: ae::Vec2,
    kit: BodyKit,
    frame: ae::MotionFrame,
    body_size: ae::Vec2,
    /// The buttons-only search. Every armed search is this one plus a burst, so
    /// the horizon and timestep can never differ between routes.
    probe: ae::movement::recovery::RecoveryProbe,
    /// ⭐ **the displacements this body could throw, in probe order.** Capped at
    /// [`MAX_PROBED_ROUTES`].
    routes: Vec<RecoveryLift>,
}

impl RecoveryLens {
    /// Lower a view, a kit and a set of routes into something
    /// [`ae::movement::recovery::probe_recovery`] can be run against.
    ///
    /// `None` when the view names no stage (there is no envelope to fall out of),
    /// when the stage is degenerate, or when the body's gravity is zero — a
    /// free-mover has no fall to recover from and the frame cannot be built.
    ///
    /// ⚠ **`routes` may be empty**, and for every seat that is not a platform
    /// fighter it is. An empty set leaves the lens exactly as it was before any
    /// of this existed: one buttons-only search, bounded by
    /// [`ae::movement::recovery::RecoveryPolicy::DRIFT_AND_JUMP`].
    pub fn from_view(
        view: &WorldView,
        kit: BodyKit,
        routes: &[RecoveryLift],
        dt: f32,
    ) -> Option<Self> {
        let stage = view.stage;
        if !stage.is_known() {
            return None;
        }
        let size = stage.bounds.max - stage.bounds.min;
        if !(size.x > 0.0 && size.y > 0.0) {
            return None;
        }
        let down = view.self_view.gravity_down.normalize_or_zero();
        let frame = ae::MotionFrame::from_acceleration(down * kit.movement.gravity)?;
        let origin = stage.bounds.min;
        // Ordered exactly as the view lists them, which is the order the room's
        // own `world.blocks` were in (ADR 0023: no iteration luck).
        let blocks = view
            .terrain
            .iter()
            .map(|solid| {
                let min = solid.aabb.min - origin;
                let extent = solid.aabb.max - solid.aabb.min;
                match solid.kind {
                    // A blink wall is full collision to a body that cannot blink
                    // through it, and whether it CAN is the kernel's call, not
                    // this lowering's — perception has already distilled the tier
                    // away, so the conservative solid is what is left.
                    SolidKind::Solid | SolidKind::BlinkWall => {
                        ae::Block::solid("perceived", min, extent)
                    }
                    SolidKind::OneWay => ae::Block::one_way("perceived", min, extent),
                    SolidKind::Hazard => ae::Block::hazard("perceived", min, extent),
                }
            })
            .collect();
        Some(Self {
            world: ae::World::new("perceived stage", size, ae::Vec2::ZERO, blocks)
                // ⭐ the envelope IS the death line — see the module header. One
                // model of dying, shared with `StageView::offstage`.
                .with_blast_margin(0.0)
                .with_side_blast_margin(0.0)
                .with_ceiling_blast_margin(0.0),
            origin,
            kit,
            frame,
            body_size: view.self_view.half_extent * 2.0,
            probe: ae::movement::recovery::RecoveryProbe::seconds(RECOVERY_PROBE_SECONDS, dt),
            routes: routes.iter().take(MAX_PROBED_ROUTES).copied().collect(),
        })
    }

    /// The body this lens probes, as this line left it.
    fn scratch(&self, at: RecoveryQuery) -> ae::BodyClusterScratch {
        // ⚠ the velocity is said at CONSTRUCTION, not patched on after: the whole
        // point of this scratch is "the body as this line left it", and its speed
        // is part of that state rather than a change to it.
        let mut body =
            ae::BodyClusterScratch::new_with_abilities(at.pos - self.origin, self.kit.abilities)
                .with_velocity(at.vel);
        // The body's OWN movement law, not the engine default a scratch body is
        // born with. A character that authors its own gravity, air accel or jump
        // is probed as itself.
        body.model = ae::MotionModel::axis_swept(self.kit.movement.axis_swept_params());
        body.kinematics.size = self.body_size;
        body.base_size.base_size = self.body_size;
        body.ground.on_ground = false;
        // ⛔ NOT a refresh. `new_with_abilities` hands out a full air-jump budget
        // and the question is about the body as this line left it — a line that
        // already spent its jump must be probed without it.
        body.jump.air_jumps_available = at.air_jumps_left;
        body
    }

    /// This lens's search, armed with one route's displacement.
    fn armed(&self, route: RecoveryLift) -> ae::movement::recovery::RecoveryProbe {
        self.probe.with_policy(
            ae::movement::recovery::RecoveryPolicy::drift_jump_and_burst(
                ae::movement::recovery::RecoveryBurst {
                    // Body-local: `+y` is toward the feet, so a lift is negative,
                    // and `+x` is toward the facing the effort is steering.
                    local: ae::Vec2::new(route.side, -route.speed),
                    // The windup, in kernel steps of THIS probe.
                    at_step: (route.after_s / self.probe.dt.max(f32::EPSILON))
                        .round()
                        .max(0.0) as usize,
                },
            ),
        )
    }

    /// ⭐⭐ **ASK THE KERNEL WHICH OF THIS BODY'S ACTIONS IS USEFUL FROM HERE.**
    ///
    /// The buttons-only search first, then each route in order, stopping at the
    /// first one that regains support.
    ///
    /// ⛔⛔ **this replaces ranking the repertoire by a number, and that is the
    /// whole architectural point of the change.** The lens used to be handed ONE
    /// route — whichever authored move advertised the largest against-gravity
    /// speed — chosen before anything knew where the body was. That makes a
    /// scalar into a recovery ontology: any move with a positive lift is *the
    /// way home*, a tiny rising aerial outranks a grapple that trades its energy
    /// for distance, and the route that would actually have worked is never
    /// explored. Usefulness is a question about the CURRENT STATE and the only
    /// authority on it is the movement kernel, so the kernel is asked.
    ///
    /// ⭐ **the buttons-only baseline goes first on purpose.** A body that is
    /// already getting home does not need to throw anything, and a caller told
    /// so can save the move — which is a real fighting-game fact (spending your
    /// recovery early is how you die to an edgeguard) that fell out of the
    /// ordering rather than being encoded.
    ///
    /// ⚠ **the negative belongs to the LAST search run.** When nothing regains,
    /// the outlook returned is the widest search's, and its
    /// `NoSupportFoundBy { search, .. }` names that policy — so a consumer
    /// reading `bounded_by()` learns what the final attempt spent, not what
    /// every attempt spent. Still a bounded claim, and now bounded by a strictly
    /// wider search than before.
    pub fn best_route(&self, at: RecoveryQuery) -> RouteVerdict {
        let body = self.scratch(at);
        let mut last =
            ae::movement::recovery::probe_recovery(&self.world, &body, self.frame, self.probe);
        if last.regained() {
            return RouteVerdict {
                outlook: last,
                route: None,
            };
        }
        for (index, route) in self.routes.iter().enumerate() {
            last = ae::movement::recovery::probe_recovery(
                &self.world,
                &body,
                self.frame,
                self.armed(*route),
            );
            if last.regained() {
                return RouteVerdict {
                    outlook: last,
                    route: Some(index),
                };
            }
        }
        RouteVerdict {
            outlook: last,
            route: None,
        }
    }

    /// **Drive the real kernel from `at` and report what it found**, over every
    /// route this body owns.
    pub fn outlook(&self, at: RecoveryQuery) -> ae::movement::recovery::RecoveryOutlook {
        self.best_route(at).outlook
    }

    /// [`Self::outlook`], reduced to the one bit the veto spends.
    pub fn regains_support(&self, at: RecoveryQuery) -> bool {
        self.best_route(at).regained()
    }
}

#[cfg(test)]
mod tests;
