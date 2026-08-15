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
//! repertoire commands an against-gravity speed hands it to the probe as a
//! [`BodyKit::lift`], the policy becomes `drift+jump+burst`, and the veto is
//! taken against the body that will actually be throwing its Up-B. Without it
//! the rollout condemned exactly the lines a real recovery saves. Read the bound
//! off the value with `RecoveryOutlook::bounded_by()` rather than assuming it; a
//! positive returns `None`, because finding a route proves one exists while
//! failing to find one is only ever a claim about the searcher.
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
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyKit {
    pub abilities: ae::AbilitySet,
    pub movement: ae::MovementTuning,
    /// ⭐ **the body's own way up, if it authored one.**
    ///
    /// The third body-derived fact, and the one that widens the SEARCH rather
    /// than the body: an authored move that commands an against-gravity speed
    /// (`MoveFrameData::lift_speed`) is a route the default drift-and-jump
    /// policy cannot press, so a negative taken without it is a verdict about a
    /// body the fighter does not have.
    ///
    /// `None` — every body that authors no such move — leaves the probe exactly
    /// as it was, which is what makes this safe for every other seat.
    ///
    /// ⛔ still not a rule about bodies. It is a velocity and a step count, and
    /// where those came from is this module's business and not the kernel's.
    pub lift: Option<RecoveryLift>,
}

/// **The against-gravity displacement a body's repertoire can command, in the
/// terms a [`RecoveryProbe`](ae::movement::recovery::RecoveryProbe) needs.**
///
/// Derived from a move's frame data by
/// [`lifting_candidates`](super::options::lifting_candidates); nothing here
/// names a move, a verb or a character.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecoveryLift {
    /// Engine units per second, against gravity.
    pub speed: f32,
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

/// A perceived stage plus a body's kit, ready to answer recovery questions.
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
    probe: ae::movement::recovery::RecoveryProbe,
}

impl RecoveryLens {
    /// Lower a view + a kit into something [`ae::movement::recovery::probe_recovery`]
    /// can be run against.
    ///
    /// `None` when the view names no stage (there is no envelope to fall out of),
    /// when the stage is degenerate, or when the body's gravity is zero — a
    /// free-mover has no fall to recover from and the frame cannot be built.
    pub fn from_view(view: &WorldView, kit: BodyKit, dt: f32) -> Option<Self> {
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
            probe: {
                let probe =
                    ae::movement::recovery::RecoveryProbe::seconds(RECOVERY_PROBE_SECONDS, dt);
                // ⭐ **THE VETO NOW CONSIDERS THE MOVE THE BODY WOULD ACTUALLY
                // THROW.** Without this the rollout condemned lines a real Up-B
                // saves, and the module header's own warning — *"the first thing
                // to re-check the day a fighter gains one"* — was that day.
                match kit.lift {
                    Some(lift) => probe.with_policy(
                        ae::movement::recovery::RecoveryPolicy::drift_jump_and_burst(
                            ae::movement::recovery::RecoveryBurst {
                                // Body-local: `+y` is toward the feet, so a lift
                                // is negative. The side component is zero — the
                                // move commands a rise; where the body goes
                                // sideways is the drift's business, and the
                                // probe steers that itself.
                                local: ae::Vec2::new(0.0, -lift.speed),
                                // The windup, in kernel steps of THIS probe.
                                at_step: (lift.after_s / dt.max(f32::EPSILON)).round().max(0.0)
                                    as usize,
                            },
                        ),
                    ),
                    None => probe,
                }
            },
        })
    }

    /// **Drive the real kernel from `at` and report what it found.**
    pub fn outlook(&self, at: RecoveryQuery) -> ae::movement::recovery::RecoveryOutlook {
        let mut body =
            ae::BodyClusterScratch::new_with_abilities(at.pos - self.origin, self.kit.abilities);
        // The body's OWN movement law, not the engine default a scratch body is
        // born with. A character that authors its own gravity, air accel or jump
        // is probed as itself.
        body.model = ae::MotionModel::axis_swept(self.kit.movement.axis_swept_params());
        body.kinematics.vel = at.vel;
        body.kinematics.size = self.body_size;
        body.base_size.base_size = self.body_size;
        body.ground.on_ground = false;
        // ⛔ NOT a refresh. `new_with_abilities` hands out a full air-jump budget
        // and the question is about the body as this line left it — a line that
        // already spent its jump must be probed without it.
        body.jump.air_jumps_available = at.air_jumps_left;
        ae::movement::recovery::probe_recovery(&self.world, &body, self.frame, self.probe)
    }

    /// [`Self::outlook`], reduced to the one bit the veto spends.
    pub fn regains_support(&self, at: RecoveryQuery) -> bool {
        self.outlook(at).regained()
    }
}

#[cfg(test)]
mod tests;
