//! Reachability probe for whether a body can regain support.
//!
//! The probe clones the body and drives the real movement kernel. A negative
//! result is bounded by the chosen [`RecoveryPolicy`] and horizon; it means that
//! search found no support, not that the body has no possible recovery.
//!
//! Movement performed outside the kernel, such as portal transit, grapples, or
//! later external impulses, is outside the probe. Moving geometry is sampled at
//! its current pose.
//!
//! The probe mutates no world state and carries no rollback state. It is intended
//! for analysis and authoring checks, not per-body per-tick simulation.

use crate::abilities::{AbilityGrant, AbilitySet};
use crate::body_clusters::BodyClusterScratch;
use crate::movement::kernel::{step_motion, MotionStepContext};
use crate::movement::{
    ActionEdges, Edge, InputState, MotionModel, MotionModelSpec, MovementAction, ResetCause,
};
use crate::{LocalAxes, MotionFrame, World};

/// What the probe is about to try: the input policy, the horizon, the
/// timestep. The whole SEARCH, in one value — which is why a negative can carry
/// it and a consumer can read what was actually covered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecoveryProbe {
    /// Kernel steps per effort.
    pub steps: usize,
    pub dt: f32,
    ///  the buttons this search presses. Not a property of the body and
    /// not a property of the kernel: a heuristic, and the cheapest one, so a
    /// negative under it is a much weaker claim than it looks.
    pub policy: RecoveryPolicy,
}

impl RecoveryProbe {
    /// Four seconds at 60Hz. A 480px fall under the default gravity takes
    /// 0.65s; terminal velocity crosses any authored room well inside this.
    pub const DEFAULT_STEPS: usize = 240;

    /// Watch for `seconds` at the given fixed timestep, under the default policy.
    pub fn seconds(seconds: f32, dt: f32) -> Self {
        Self {
            steps: (seconds / dt.max(f32::EPSILON)).ceil().max(0.0) as usize,
            dt,
            policy: RecoveryPolicy::DRIFT_AND_JUMP,
        }
    }

    /// Search the same horizon with different buttons.
    pub fn with_policy(mut self, policy: RecoveryPolicy) -> Self {
        self.policy = policy;
        self
    }
}

impl Default for RecoveryProbe {
    fn default() -> Self {
        Self {
            steps: Self::DEFAULT_STEPS,
            dt: 1.0 / 60.0,
            policy: RecoveryPolicy::DRIFT_AND_JUMP,
        }
    }
}

/// Where one effort has got to, when the policy is asked what to press.
///
/// Deliberately tiny: a policy may react to the body's motion (that is what
/// makes "press jump at the apex" expressible without a plan) but it may not
/// read the world, so it cannot smuggle in a rule about level geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecoveryStep {
    /// Which effort of [`RecoveryPolicy::efforts`] this is, counting from 0.
    pub effort: usize,
    /// Kernel steps already taken in THIS effort.
    pub step: usize,
    /// Is the body moving against gravity right now?
    pub rising: bool,
    /// Does the body have its feet down right now?
    pub on_ground: bool,
}

/// The input policy a [`RecoveryProbe`] searches with — a heuristic, never a
/// statement about the body.
///
///  this exists so that a negative result can name what it tried. It is NOT an enumeration of
/// what a body can do: the engine ships exactly one policy ([`Self::DRIFT_AND_JUMP`]) and a
/// caller that wants more supplies its own.
///
/// Determinism (ADR 0023): `input` must be a pure function of its
/// [`RecoveryStep`] — same step, same buttons — and the efforts are an ordered
/// range, so two probes of the same body agree and the winner never depends on
/// iteration luck.
#[derive(Clone, Copy)]
pub struct RecoveryPolicy {
    /// Stable identity, carried into every negative this policy produces.
    pub name: &'static str,
    /// How many ordered efforts to run. Effort `i` gets `effort: i`.
    pub efforts: usize,
    /// The buttons for one step of one effort.
    pub input: fn(RecoveryStep) -> InputState,
    ///  the one thing a search can press that is not a button.
    ///
    /// See [`RecoveryBurst`]. `None` — the default and
    /// [`Self::DRIFT_AND_JUMP`]'s value — is a policy that presses nothing but
    /// its buttons, which is what this module shipped with.
    pub burst: Option<RecoveryBurst>,
}

/// A DISPLACEMENT the search is allowed to spend, once, at a known moment.
///
///  this is the module header's own gap, closed on purpose and only this
/// far. The header says a recovery ATTACK is invisible here because
/// `step_motion` is the whole world and nothing that moves a body from outside
/// it can be seen. That is still true of knockback, of a portal and of a
/// grapple. What changed is that one of those outside movers is not outside at
/// all: an authored recovery move states the speed it commands and when
/// `MoveFrameData::lift_speed`),
/// so a caller that HOLDS such a move can hand the probe that number and get a
/// verdict about the body it actually has.
///
/// [`RecoveryProbe`] carries the whole policy into every negative precisely so a consumer can
/// read which it got.
///
///  it is not a MOVE and knows nothing about attacks. It is a velocity and
/// a step count. The kernel has no idea whether the thing that produced it was a
/// special, a jetpack, an authored knockback or a scripted cutscene shove — which
/// is what stops this from being a fighting-game rule in a movement module.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecoveryBurst {
    /// Body-LOCAL, so it rotates with gravity like everything else here:
    /// `+x` = the side the effort is steering toward, `+y` = toward the feet.
    /// An against-gravity burst is negative `y`.
    pub local: crate::Vec2,
    pub at_step: usize,
}

impl RecoveryPolicy {
    /// The cheap default: hold a side, hold jump, re-press jump at the apex.
    ///
    ///  the press rule is what makes this "full effort" without becoming a
    /// SEARCH. Pressing every tick would spend a whole air-jump budget in
    /// consecutive frames and climb less than one jump; pressing at the top of
    /// the arc is what a human does and what chains the most height out of the
    /// budget, and it is reactive rather than a plan the caller had to supply.
    /// Holding the button between presses is load-bearing too: a held jump is
    /// what opens a cape/glide and what stops a variable-jump law from cutting
    /// the arc short.
    ///
    ///  and it presses nothing else — no dash, no blink, no flight toggle,
    /// no fast fall. That is the whole reason [`RecoveryOutlook`]'s negative is
    /// bounded by the policy that produced it.
    pub const DRIFT_AND_JUMP: Self = Self {
        name: "drift+jump",
        efforts: DRIFT_SIDES.len(),
        input: drift_and_jump,
        burst: None,
    };

    /// [`Self::DRIFT_AND_JUMP`], plus one displacement the body can actually
    /// command.
    ///
    /// The steering and the jump are unchanged, so a body that gets home without the burst
    /// still gets home the same way and by the same effort — the burst only ever adds routes.
    pub const fn drift_jump_and_burst(burst: RecoveryBurst) -> Self {
        Self {
            name: "drift+jump+burst",
            efforts: DRIFT_SIDES.len(),
            input: drift_and_jump,
            burst: Some(burst),
        }
    }
}

impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self::DRIFT_AND_JUMP
    }
}

impl PartialEq for RecoveryPolicy {
    /// Identity is the NAME, the effort count and the BURST.  never the
    /// function pointer: comparing fn pointers is unspecified (a linker may merge
    /// two identical bodies or duplicate one), so a value equality that depended
    /// on it would not be reproducible.
    ///
    ///  the burst IS part of identity, unlike the buttons: two probes named
    /// `drift+jump+burst` that spend different displacements searched different
    /// spaces, and a negative that could not tell them apart would let a
    /// consumer compare two incomparable bounds.
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.efforts == other.efforts && self.burst == other.burst
    }
}

impl core::fmt::Debug for RecoveryPolicy {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.burst {
            Some(burst) => write!(
                f,
                "RecoveryPolicy({:?}, {} efforts, burst {:?} at step {})",
                self.name, self.efforts, burst.local, burst.at_step
            ),
            None => write!(
                f,
                "RecoveryPolicy({:?}, {} efforts)",
                self.name, self.efforts
            ),
        }
    }
}

/// The steering efforts [`RecoveryPolicy::DRIFT_AND_JUMP`] tries, in this order.
///
/// Body-LOCAL side, so it rotates with gravity. Standing still first, because a
/// body that already has support should report that without a story about which
/// way it ran.
const DRIFT_SIDES: [f32; 3] = [0.0, -1.0, 1.0];

/// [`RecoveryPolicy::DRIFT_AND_JUMP`]'s per-step buttons.
fn drift_and_jump(at: RecoveryStep) -> InputState {
    let side = DRIFT_SIDES.get(at.effort).copied().unwrap_or(0.0);
    let jump = Edge {
        pressed: !at.on_ground && !at.rising,
        held: true,
        released: false,
    };
    InputState {
        axes: LocalAxes::new(side, 0.0),
        movement: ActionEdges::<MovementAction>::EMPTY.with(MovementAction::Jump, jump),
        ..Default::default()
    }
}

/// What the probe saw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RecoveryOutlook {
    /// The body came to rest on, rode, clung to or caught hold of something,
    /// after `steps` kernel steps of effort `effort`.
    ///
    /// a positive needs no bound.
    Regained { steps: usize, effort: usize },
    ///  NOT "this body cannot recover." `search` regained no support: the
    /// buttons [`RecoveryProbe::policy`] presses, run for
    /// [`RecoveryProbe::steps`] steps, reached nothing to stand on. A body that
    /// gets home only by a verb the policy never presses lands here, and this
    /// variant is named so a consumer cannot read it as more than it is.
    ///
    /// `reset` is `Some` only when EVERY effort ended in a world reset — the
    /// world killed the body whichever way this policy steered, which is a
    /// stronger and different fact from "still falling when I stopped watching".
    /// `None` means at least one effort was still going, so the horizon, not the
    /// world, ended it.  even `Some` is still policy-bounded: it says every
    /// effort THIS policy made died, not that every effort would.
    NoSupportFoundBy {
        search: RecoveryProbe,
        reset: Option<ResetCause>,
    },
}

impl RecoveryOutlook {
    pub fn regained(self) -> bool {
        matches!(self, Self::Regained { .. })
    }

    /// The search a negative is a fact about; `None` for a positive.
    pub fn bounded_by(self) -> Option<RecoveryProbe> {
        match self {
            Self::Regained { .. } => None,
            Self::NoSupportFoundBy { search, .. } => Some(search),
        }
    }
}

/// Drive this body's own kernel at full recovery effort and report whether it
/// gets back to support.
///
/// `body` is an ordinary [`BodyClusterScratch`] — the same 18 clusters plus
/// [`MotionModel`] a live entity carries — so its position, velocity, spent air
/// jumps, dash charges, ability set and authored tuning all come along. There is
/// no second body vocabulary to keep in step with the first.
///
/// `frame` carries gravity's direction AND magnitude, so the answer is
/// gravity-generic: rotate the world and the body and the verdict rotates with
/// them.
///
///  read the negative as what it is. A [`RecoveryOutlook::Regained`] is a
/// fact about the body — the real kernel got it home. A
/// [`RecoveryOutlook::NoSupportFoundBy`] is a fact about `probe`, which is why it
/// carries it.
pub fn probe_recovery(
    world: &World,
    body: &BodyClusterScratch,
    frame: MotionFrame,
    probe: RecoveryProbe,
) -> RecoveryOutlook {
    let mut every_effort_was_reset = true;
    let mut cause = None;
    for effort in 0..probe.policy.efforts {
        match run_effort(world, body, frame, probe, effort) {
            EffortOutcome::Regained(steps) => return RecoveryOutlook::Regained { steps, effort },
            EffortOutcome::Reset(reset) => cause = cause.or(Some(reset)),
            EffortOutcome::StillFalling => every_effort_was_reset = false,
        }
    }
    RecoveryOutlook::NoSupportFoundBy {
        search: probe,
        reset: if every_effort_was_reset { cause } else { None },
    }
}

/// The grants tried by [`recovery_capability_gap`], in this order.
const GAP_CANDIDATES: [AbilityGrant; 3] = [
    AbilityGrant::RunJump,
    AbilityGrant::AirJump,
    AbilityGrant::WallMobility,
];

/// Return the authored grant that would make this position recoverable.
///
/// Each candidate is added to the body's ability set, its newly available budget
/// is topped up, and recovery is re-probed. Candidates are limited to grants that
/// can be represented on an already-built body and verbs exercised by the probe's
/// recovery policy; otherwise the diagnostic would claim capabilities the probe
/// cannot actually test.
pub fn recovery_capability_gap(
    world: &World,
    body: &BodyClusterScratch,
    frame: MotionFrame,
    probe: RecoveryProbe,
) -> Option<AbilityGrant> {
    if probe_recovery(world, body, frame, probe).regained() {
        return None;
    }
    let air_jumps = authored_air_jumps(&body.model);
    for grant in GAP_CANDIDATES {
        let granted = body.abilities.abilities.union(grant.to_set());
        if granted == body.abilities.abilities {
            // Grants nothing this body did not already have. Skipping matters:
            // re-granting `AirJump` to a body that HAS the verb and has SPENT
            // the charge would top its budget back up and report the verb as
            // missing when the charge was.
            continue;
        }
        let mut with_grant = body.clone();
        grant_and_top_up(&mut with_grant, granted, air_jumps);
        if probe_recovery(world, &with_grant, frame, probe).regained() {
            return Some(grant);
        }
    }
    None
}

enum EffortOutcome {
    Regained(usize),
    Reset(ResetCause),
    StillFalling,
}

/// The ROLLOUT half: drive the real kernel for one effort and report what
/// physically happened.
///
///  nothing here decides what to press — `probe.policy` does, and it is the
/// only thing that would have to change to search harder. Everything below the
/// input call is body-generic and stays true whatever the policy is.
fn run_effort(
    world: &World,
    body: &BodyClusterScratch,
    frame: MotionFrame,
    probe: RecoveryProbe,
    effort: usize,
) -> EffortOutcome {
    let mut scratch = body.clone();
    for step in 0..probe.steps {
        let rising = scratch.kinematics.vel.dot(frame.down()) < 0.0;
        let input = (probe.policy.input)(RecoveryStep {
            effort,
            step,
            rising,
            on_ground: scratch.ground.on_ground,
        });
        // The effort's own steering IS its facing intent: a body that cannot
        // turn in the air still points where it is trying to go.
        let facing_intent = input.local_axis().x;
        //  THE BURST, WRITTEN THE WAY THE THING IT MODELS WRITES IT.
        //
        // The search would then model a fighter that throws its recovery and loses control of it,
        // and report "no support" about a body the real runtime never tumbles. A predictor whose
        // one move behaves differently from the move is worse than no predictor.
        //
        //  so this mirrors the authored-impulse seam exactly: a
        // `MoveEventKind::Impulse` is a direct velocity write on the owner, and
        // so is this.  and it inherits that seam's own limit — a direct
        // `vel` write is authoritative for an axis-swept body and approximate for
        // a surface-momentum rider, whose `vel` is republished from `v_t` each
        // step. Both halves are wrong together or right together, which is the
        // property worth having; fixing it is one change in two places, not a
        // divergence to reconcile.
        //
        // The side component follows the effort's own steering, so effort 1
        // (drift left) bursts left and effort 2 bursts right; a policy whose
        // burst always pointed one way would search two of its three efforts
        // with a displacement fighting the drift.
        //
        //  so effort 0 is now the honest baseline — buttons only, *"do I even
        // need this?"* — and efforts 1 and 2 are the move thrown each way. A
        // burst with no side component (`local.x == 0.0`) is unaffected in every
        // effort, which is why this changes nothing for a straight-up recovery.
        if let Some(burst) = probe.policy.burst {
            let toward = if input.local_axis().x == 0.0 {
                0.0
            } else {
                input.local_axis().x.signum()
            };
            let expressible = burst.local.x == 0.0 || toward != 0.0;
            if step == burst.at_step && expressible {
                scratch.kinematics.vel =
                    frame.side() * (burst.local.x * toward) + frame.down() * burst.local.y;
            }
        }
        let result = {
            let (model, mut clusters) = scratch.parts();
            step_motion(
                model,
                &mut clusters,
                MotionStepContext {
                    world,
                    input,
                    frame,
                    facing_intent,
                    dt: probe.dt,
                    // A recovery probe, not this body's real motion.
                    contact: crate::movement::body_contact::BodyContactField::NONE,
                    pose_owned_externally: false,
                },
            )
        };
        // The out-of-bounds gate REPORTS; it does not move the body.
        if let Some(reset) = result.events.reset {
            return EffortOutcome::Reset(reset);
        }
        // Resting on, riding, or adhesively attached to a surface — plus the
        // ledge hang, which holds a body against gravity without producing a
        // contact and so is invisible to `SupportFact`.
        if result.support.is_held() || scratch.model.holds_a_ledge() {
            return EffortOutcome::Regained(step + 1);
        }
    }
    EffortOutcome::StillFalling
}

/// Install `granted` and add ONLY the budget the newly-granted verbs pay for.
///
/// Never a wholesale refresh: `refresh_movement_resources_clusters` is the
/// LANDING rule, and using it here would hand a mid-air body back the air jumps
/// it had already spent, so every probe would answer about a body that had not
/// done anything yet.
fn grant_and_top_up(body: &mut BodyClusterScratch, granted: AbilitySet, authored_air_jumps: u8) {
    let before = body.abilities.abilities;
    let extra_air_jumps = granted
        .air_jump_count(authored_air_jumps)
        .saturating_sub(before.air_jump_count(authored_air_jumps));
    let extra_dash_charges = granted
        .dash_charge_count()
        .saturating_sub(before.dash_charge_count());
    body.abilities.abilities = granted;
    body.jump.air_jumps_available = body
        .jump
        .air_jumps_available
        .saturating_add(extra_air_jumps);
    body.dash.charges_available = body
        .dash
        .charges_available
        .saturating_add(extra_dash_charges);
}

/// How many air jumps this body's own policy authors. Zero for a policy that
/// has no such thing — a surface-momentum rider and an adhesive crawler do not
/// jump in the air, and answering with the engine default would invent a verb.
fn authored_air_jumps(model: &MotionModel) -> u8 {
    match model.spec() {
        MotionModelSpec::AxisSwept(params) => params.locomotion.air_jumps,
        MotionModelSpec::SurfaceMomentum(_) | MotionModelSpec::AdhesiveCrawler(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Block;
    use crate::Vec2;

    /// One shelf in a tall empty room: `x` in `300..660`, top face at `y = 400`.
    /// Everything else is void, so a body that misses it leaves the world.
    fn shelf_world() -> World {
        World::new(
            "recovery shelf",
            Vec2::new(960.0, 540.0),
            Vec2::new(480.0, 376.0),
            vec![Block::solid(
                "shelf",
                Vec2::new(300.0, 400.0),
                Vec2::new(360.0, 32.0),
            )],
        )
    }

    /// The same room transposed — `(x, y)` swapped — so gravity can point along
    /// `+x` and every geometric relationship is preserved.
    fn transposed_shelf_world() -> World {
        World::new(
            "recovery shelf (sideways gravity)",
            Vec2::new(540.0, 960.0),
            Vec2::new(376.0, 480.0),
            vec![Block::solid(
                "shelf",
                Vec2::new(400.0, 300.0),
                Vec2::new(32.0, 360.0),
            )],
        )
    }

    fn falling_body(abilities: AbilitySet, pos: Vec2) -> BodyClusterScratch {
        let mut body = BodyClusterScratch::new_with_abilities(pos, abilities);
        body.ground.on_ground = false;
        body
    }

    fn frame_pulling(down: Vec2) -> MotionFrame {
        MotionFrame::from_acceleration(down * crate::movement::GRAVITY)
            .expect("the probe frames are built from a non-zero gravity")
    }

    fn cannot_steer() -> AbilitySet {
        AbilitySet {
            move_horizontal: false,
            ..AbilitySet::basic()
        }
    }

    /// A body whose only way home is a BLINK: it can steer and it can blink, and
    /// [`AbilitySet::NONE`] underneath means nothing else — no jump, no dash, no
    /// wall verb — can be doing the work.
    fn blinker() -> AbilitySet {
        AbilitySet {
            move_horizontal: true,
            blink: true,
            ..AbilitySet::NONE
        }
    }

    /// The same three ordered drift efforts, plus a blink press/release
    /// alternation. Blink completes on RELEASE (`handle_blink_clusters`), so one
    /// blink costs two kernel steps and the cooldown swallows the presses in
    /// between. Pure in its [`RecoveryStep`], like any policy must be.
    fn drift_and_blink(at: RecoveryStep) -> InputState {
        let side = [0.0_f32, -1.0, 1.0].get(at.effort).copied().unwrap_or(0.0);
        let blink = if at.step % 2 == 0 {
            Edge {
                pressed: true,
                held: true,
                released: false,
            }
        } else {
            Edge {
                pressed: false,
                held: false,
                released: true,
            }
        };
        InputState {
            axes: LocalAxes::new(side, 0.0),
            movement: ActionEdges::<MovementAction>::EMPTY.with(MovementAction::Blink, blink),
            ..Default::default()
        }
    }

    /// A tall empty room with ONE high shelf: `x` in `300..660`, top face at
    /// `y = 300`, and nothing else at all. A body that starts BELOW that face
    /// cannot reach it by falling, whatever it steers.
    fn high_shelf_world() -> World {
        World::new(
            "recovery high shelf",
            Vec2::new(960.0, 1000.0),
            Vec2::new(480.0, 260.0),
            vec![Block::solid(
                "shelf",
                Vec2::new(300.0, 300.0),
                Vec2::new(360.0, 32.0),
            )],
        )
    }

    /// Steers, and nothing else — no jump, no dash, no wall verb. Whatever gets
    /// this body home was not a button.
    fn drifter() -> AbilitySet {
        AbilitySet {
            move_horizontal: true,
            ..AbilitySet::NONE
        }
    }

    /// A COMMANDED DISPLACEMENT IS A ROUTE THE SAME SEARCH DID NOT HAVE.
    ///
    /// The body is below the only surface in the world and owns no verb that
    /// climbs: drift is the whole of its kit, and drift never gains height. So
    /// `DRIFT_AND_JUMP` is right to report nothing — and the identical body, in
    /// the identical world, from the identical position, gets home the moment
    /// the search is allowed to spend the displacement the body can actually
    /// command.
    ///
    ///  both terms are observed, which is what stops this passing
    /// vacuously: a permissive probe would regain under both policies and a
    /// broken burst under neither.
    ///
    ///  nothing here is a fighting game. The burst is a velocity and a step
    /// count; that an authored recovery special is where a caller gets those
    /// numbers is the caller's business, and this test never mentions one.
    #[test]
    fn a_commanded_burst_finds_a_route_the_buttons_could_not() {
        let world = high_shelf_world();
        let frame = frame_pulling(Vec2::new(0.0, 1.0));
        // Below the shelf's top face (y = 300) and well to the left of its
        // span (x in 300..660), so the climb has to clear the lip BEFORE the
        // drift carries the body over it. Starting nearer would put the rise
        // through the block's side.
        let body = falling_body(drifter(), Vec2::new(150.0, 500.0));

        let buttons_only = RecoveryProbe::default();
        let without = probe_recovery(&world, &body, frame, buttons_only);
        assert!(
            !without.regained(),
            "a body that can only drift cannot climb 220px onto a shelf, but \
             the probe reported {without:?}"
        );

        // Rise 1400px/s: 1400² / (2 · 2250) = 435px of climb against the ~220px
        // back up to the shelf's face, so the body is over the lip long before
        // the drift (capped at 270px/s) carries it into the span.
        let with_burst =
            buttons_only.with_policy(RecoveryPolicy::drift_jump_and_burst(RecoveryBurst {
                local: Vec2::new(0.0, -1400.0),
                at_step: 8,
            }));
        let with = probe_recovery(&world, &body, frame, with_burst);
        assert!(
            with.regained(),
            "the same body, given the displacement it can command, reaches the \
             shelf — got {with:?}"
        );
    }

    /// THE SIGN IS DOING THE WORK, not the mere presence of a burst.
    ///
    /// A dive is the same primitive pointed the other way. If the test above
    /// passed because ANY launch shook something loose in the kernel, this one
    /// would pass too — and it must not.
    #[test]
    fn a_burst_pointed_at_the_floor_is_not_a_way_home() {
        let world = high_shelf_world();
        let frame = frame_pulling(Vec2::new(0.0, 1.0));
        let body = falling_body(drifter(), Vec2::new(150.0, 500.0));
        let diving = RecoveryProbe::default().with_policy(RecoveryPolicy::drift_jump_and_burst(
            RecoveryBurst {
                local: Vec2::new(0.0, 1400.0),
                at_step: 8,
            },
        ));
        assert!(!probe_recovery(&world, &body, frame, diving).regained());
    }

    /// A wide room whose ONLY surface is far off to the right: `x` in
    /// `900..1300`, top face at `y = 500`. A body starting high and far left is
    /// ABOVE that face and cannot reach it — the gap is horizontal, not
    /// vertical, so no amount of climbing helps.
    fn distant_shelf_world() -> World {
        World::new(
            "recovery distant shelf",
            Vec2::new(1400.0, 900.0),
            Vec2::new(700.0, 400.0),
            vec![Block::solid(
                "shelf",
                Vec2::new(900.0, 500.0),
                Vec2::new(400.0, 32.0),
            )],
        )
    }

    /// THE BURST IS A VECTOR, AND THE SIDE HALF IS THE HALF THAT CROSSES A
    /// GAP.
    ///
    ///  [`RecoveryBurst::local`] always had two components; every caller in
    /// the tree passed zero for the first, so nothing had ever demonstrated that
    /// the side half does any work. A recovery whose whole job is lateral
    /// distance — a grapple line, a boarding charge, a slingshot — is
    /// unrepresentable by the vertical half alone, and a search that kept only
    /// that half would condemn the exact lines such a move saves.
    ///
    /// So this cannot pass because the burst is strong, or because the probe is permissive.
    #[test]
    fn the_side_half_of_a_burst_crosses_a_gap_the_rise_alone_cannot() {
        let world = distant_shelf_world();
        let frame = frame_pulling(Vec2::new(0.0, 1.0));
        // Above the shelf's face and 750px to its left, falling from rest. Drift
        // is capped at 270px/s and the body is airborne for well under a second,
        // so steering alone covers a third of the gap at best.
        let body = falling_body(drifter(), Vec2::new(150.0, 430.0));

        let bare = probe_recovery(&world, &body, frame, RecoveryProbe::default());
        assert!(
            !bare.regained(),
            "drift alone cannot cross 750px in the time this fall lasts, but the \
             probe reported {bare:?}"
        );

        // Rise 900px/s: 900² / (2 · 2250) = 180px of climb and ~0.85s of
        // airtime. Drift covers ~230px of the 750 in that time, so the rise on
        // its OWN buys altitude over an empty room.
        let rise_only = RecoveryProbe::default().with_policy(RecoveryPolicy::drift_jump_and_burst(
            RecoveryBurst {
                local: Vec2::new(0.0, -900.0),
                at_step: 8,
            },
        ));
        let without_side = probe_recovery(&world, &body, frame, rise_only);
        assert!(
            !without_side.regained(),
            "poison: the same rise with no side component still lands nowhere \
             near the shelf — got {without_side:?}"
        );

        // The same rise, now carrying the 1000px/s the move actually commands
        // along the body's facing. `.max(along)` in the air law conserves a
        // burst above the drift cap while the effort keeps steering that way, so
        // ~0.85s of airtime covers ~1000px and puts the body over the span.
        let grapple = RecoveryProbe::default().with_policy(RecoveryPolicy::drift_jump_and_burst(
            RecoveryBurst {
                local: Vec2::new(1000.0, -900.0),
                at_step: 8,
            },
        ));
        let with_side = probe_recovery(&world, &body, frame, grapple);
        assert!(
            with_side.regained(),
            "the same body, given the whole displacement it commands, reaches \
             the shelf — got {with_side:?}"
        );
        // And it got there by STEERING into it. Effort 0 does not steer, so a
        // route found there would be one this move cannot express.
        assert!(
            matches!(with_side, RecoveryOutlook::Regained { effort, .. } if effort != 0),
            "a side-carrying burst can only pay off on a steered effort — got \
             {with_side:?}"
        );
    }

    /// A perch a body can rise THROUGH and land on top of: one-way, `x` in
    /// `60..400`, face at `y = 300`. The only way onto it is from below, which
    /// is what makes "did the search delete the side component?" observable.
    fn one_way_perch_world() -> World {
        World::new(
            "recovery one-way perch",
            Vec2::new(1200.0, 900.0),
            Vec2::new(600.0, 400.0),
            vec![Block::one_way(
                "perch",
                Vec2::new(60.0, 300.0),
                Vec2::new(340.0, 32.0),
            )],
        )
    }

    /// AN UNSTEERED EFFORT DOES NOT THROW A DE-SIDED BURST — because there is
    /// no such move.
    ///
    ///  the search multiplied the burst's side component by the effort's
    /// steering, and `DRIFT_SIDES[0]` is zero: effort 0 therefore fired a
    /// displacement with its lateral half deleted. That is not a conservative
    /// approximation, it is a DIFFERENT MOVE — and this world is one where the
    /// different move is the better one. A body that rises straight up here
    /// passes through the one-way perch and settles on it; the same body
    /// throwing the move it actually owns is carried a thousand pixels past it
    /// and dies.
    ///
    ///  so the two halves are: the de-sided burst really does get home (or the
    /// world proves nothing), and the real one really does not.
    #[test]
    fn an_unsteered_effort_does_not_throw_a_de_sided_burst() {
        let world = one_way_perch_world();
        let frame = frame_pulling(Vec2::new(0.0, 1.0));
        let body = falling_body(drifter(), Vec2::new(150.0, 430.0));

        // 1100px/s is 269px of climb — well over the ~130px back up to the
        // perch's face, and the body is under its span the whole way.
        let straight_up = RecoveryProbe::default().with_policy(
            RecoveryPolicy::drift_jump_and_burst(RecoveryBurst {
                local: Vec2::new(0.0, -1100.0),
                at_step: 8,
            }),
        );
        let vertical = probe_recovery(&world, &body, frame, straight_up);
        assert!(
            vertical.regained(),
            "a body that rises straight up here clears the one-way perch and \
             lands on it — if it does not, this world cannot tell the two \
             bursts apart. Got {vertical:?}"
        );

        // The SAME rise, at the SAME step, now carrying the side the move
        // commands. Steered either way it overshoots the perch entirely, and
        // unsteered it is not thrown at all.
        let diagonal = RecoveryProbe::default().with_policy(RecoveryPolicy::drift_jump_and_burst(
            RecoveryBurst {
                local: Vec2::new(1000.0, -1100.0),
                at_step: 8,
            },
        ));
        let diagonal_outlook = probe_recovery(&world, &body, frame, diagonal);
        assert!(
            !diagonal_outlook.regained(),
            "the search found a route by deleting half of the displacement — \
             that route belongs to a move this body does not have. Got \
             {diagonal_outlook:?}"
        );
    }

    /// A NEGATIVE STILL NAMES WHAT IT SPENT.
    ///
    /// The module's whole honesty contract is that `NoSupportFoundBy` carries
    /// the search that produced it, so *"my search found nothing"* can never be
    /// read as *"no way exists"*. A burst widens the search, so it has to widen
    /// the BOUND too — two negatives from differently-armed policies are not the
    /// same claim, and a consumer comparing them must be able to tell.
    #[test]
    fn a_negative_names_the_burst_it_spent() {
        let world = high_shelf_world();
        let frame = frame_pulling(Vec2::new(0.0, 1.0));
        let body = falling_body(drifter(), Vec2::new(150.0, 500.0));

        let feeble = RecoveryBurst {
            local: Vec2::new(0.0, -100.0),
            at_step: 8,
        };
        let outlook = probe_recovery(
            &world,
            &body,
            frame,
            RecoveryProbe::default().with_policy(RecoveryPolicy::drift_jump_and_burst(feeble)),
        );
        let bound = outlook
            .bounded_by()
            .expect("a body that found no support is bounded by its search");
        assert_eq!(bound.policy.burst, Some(feeble));
        assert_ne!(
            bound.policy,
            RecoveryPolicy::DRIFT_AND_JUMP,
            "an armed search that failed must not be reported as the bare one"
        );
        //  and two differently-armed searches are different bounds, even
        // though they share a name.
        assert_ne!(
            bound.policy,
            RecoveryPolicy::drift_jump_and_burst(RecoveryBurst {
                local: Vec2::new(0.0, -1400.0),
                at_step: 8,
            }),
        );
    }

    const DRIFT_AND_BLINK: RecoveryPolicy = RecoveryPolicy {
        name: "drift+blink (test)",
        efforts: 3,
        input: drift_and_blink,
        burst: None,
    };

    /// The verdict comes from the BODY's kit, not from where the body is.
    ///
    /// Same world, same position, same velocity; the only difference is whether
    /// the body owns the verb that carries it over the shelf. Both terms are
    /// observed, so neither half can pass vacuously — if the probe ignored
    /// capabilities the two would agree and the test fails.
    #[test]
    fn the_bodys_own_kit_decides_whether_a_fall_is_recoverable() {
        let world = shelf_world();
        let frame = frame_pulling(Vec2::new(0.0, 1.0));
        let probe = RecoveryProbe::default();
        // High above the void to the left of the shelf: the drift has to earn it.
        let start = Vec2::new(250.0, 50.0);

        let stuck = falling_body(cannot_steer(), start);
        let stuck_outlook = probe_recovery(&world, &stuck, frame, probe);
        assert!(
            !stuck_outlook.regained(),
            "a body with no horizontal verb falls straight past the shelf and \
             out of the world, but the probe reported {stuck_outlook:?}"
        );

        let steering = falling_body(AbilitySet::basic(), start);
        let steering_outlook = probe_recovery(&world, &steering, frame, probe);
        assert!(
            steering_outlook.regained(),
            "the same fall, by a body that can steer, reaches the shelf — got \
             {steering_outlook:?}"
        );

        // And the gap is reported in the engine's own authoring vocabulary.
        assert_eq!(
            recovery_capability_gap(&world, &stuck, frame, probe),
            Some(AbilityGrant::RunJump),
            "the grant that would have saved this body is the one that grants \
             horizontal movement"
        );
        assert_eq!(
            recovery_capability_gap(&world, &steering, frame, probe),
            None,
            "a body that already recovers has no capability gap"
        );
    }

    /// "Airborne, below the lip, outside the span" is not a verdict.
    #[test]
    fn below_the_lip_and_outside_the_span_is_answered_by_the_surfaces() {
        let frame = frame_pulling(Vec2::new(0.0, 1.0));
        let probe = RecoveryProbe::default();
        // Left of the shelf's span (300..660), below its top (400), falling,
        // and with no air jump left: exactly the deleted rule's `doomed`.
        let mut body = falling_body(AbilitySet::basic(), Vec2::new(200.0, 460.0));
        body.kinematics.vel = Vec2::new(0.0, 200.0);
        body.jump.air_jumps_available = 0;

        let mut caught = shelf_world();
        caught.blocks.push(Block::solid(
            "catch",
            Vec2::new(120.0, 500.0),
            Vec2::new(200.0, 32.0),
        ));
        let outlook = probe_recovery(&caught, &body, frame, probe);
        assert!(
            outlook.regained(),
            "a body below the lip and outside the span landed on the floor that \
             is right underneath it, but the probe reported {outlook:?}"
        );

        let bare = shelf_world();
        let without = probe_recovery(&bare, &body, frame, probe);
        assert!(
            !without.regained(),
            "poison: with that floor removed the identical body must NOT be \
             reported as recovering — got {without:?}"
        );
    }

    /// The answer rotates with gravity.
    ///
    /// The room and the body are transposed and gravity is pointed along `+x`.
    /// Nothing in the probe may assume screen-down: the steering body recovers
    /// and the one that cannot steer does not, exactly as under normal gravity.
    /// The second assertion is what stops this passing for the wrong reason —
    /// a probe that reported "recovered" for everything would satisfy the first.
    #[test]
    fn the_probe_is_gravity_generic() {
        let world = transposed_shelf_world();
        let frame = frame_pulling(Vec2::new(1.0, 0.0));
        let probe = RecoveryProbe::default();
        let start = Vec2::new(50.0, 250.0);

        let steering = falling_body(AbilitySet::basic(), start);
        let outlook = probe_recovery(&world, &steering, frame, probe);
        assert!(
            outlook.regained(),
            "under sideways gravity the same fall must still reach the same \
             shelf — got {outlook:?}"
        );

        let stuck = falling_body(cannot_steer(), start);
        let stuck_outlook = probe_recovery(&world, &stuck, frame, probe);
        assert!(
            !stuck_outlook.regained(),
            "and a body that cannot steer must still miss it — got \
             {stuck_outlook:?}"
        );
    }

    ///  A NEGATIVE IS A FACT ABOUT THE SEARCH, NOT ABOUT THE BODY.
    ///
    /// The default policy presses a side and a jump and nothing else, so a body
    /// that gets home only by blinking is reported as finding no support — a
    /// verdict that is right about the search and *wrong about the body*. This
    /// demonstrates the gap rather than asserting it from a doc comment: the
    /// world, the body, the position, the velocity and the horizon are IDENTICAL
    /// across the two probes and only the buttons differ, so a difference in the
    /// verdict can only have come from the policy.
    ///
    /// The arithmetic, so a failure can be told from a fixture mistake:
    /// - centre `(140, 300)`, body 30×48  right edge 155, feet 324;
    /// - the shelf spans `300..660` with its top face at 400, so the body must
    ///   cross 145px sideways while falling the 76px to that face;
    /// - 76px of fall is 0.26s and the body's top run speed is 270px/s, so
    ///   drifting covers at most ~70px — it misses by better than a factor of
    ///   two whichever way it steers, then leaves the world;
    /// - one blink is 190px: it puts the body 45px INSIDE the span, still 76px
    ///   above the face, and gravity does the rest.
    ///
    ///  if the first assertion fails, the default policy grew a verb. If the
    /// second fails, blink stopped carrying 190px or the shelf moved. If the
    /// poison fails, the policy is teleporting bodies that cannot blink.
    #[test]
    fn a_negative_is_a_fact_about_the_search_not_the_body() {
        let world = shelf_world();
        let frame = frame_pulling(Vec2::new(0.0, 1.0));
        let default_probe = RecoveryProbe::default();
        let start = Vec2::new(140.0, 300.0);
        let body = falling_body(blinker(), start);

        let bounded = probe_recovery(&world, &body, frame, default_probe);
        assert!(
            !bounded.regained(),
            "the default policy never presses blink, so it cannot find this \
             body's only way home — got {bounded:?}"
        );

        //  and the negative says WHOSE negative it is. Even the strongest
        // shape this type can take — every effort ended in a world reset, the
        // "it died whichever way it steered" case — is still bounded by the
        // buttons that were pressed and the steps they were pressed for.
        let RecoveryOutlook::NoSupportFoundBy { search, reset } = bounded else {
            unreachable!("a non-regained outlook is a NoSupportFoundBy");
        };
        assert_eq!(
            search.policy,
            RecoveryPolicy::DRIFT_AND_JUMP,
            "the negative must name the policy that produced it"
        );
        assert_eq!(
            search.steps,
            RecoveryProbe::DEFAULT_STEPS,
            "and the horizon it was produced within"
        );
        assert!(
            matches!(reset, Some(ResetCause::LeftTheWorld)),
            "every drift effort falls out of this room, so the negative wears \
             its strongest form — got {reset:?}"
        );

        // Same world, same body, same horizon. ONLY the buttons differ.
        let searched = probe_recovery(
            &world,
            &body,
            frame,
            default_probe.with_policy(DRIFT_AND_BLINK),
        );
        assert!(
            searched.regained(),
            "the identical body in the identical place DOES get back when the \
             search presses the verb it owns — so the negative above was about \
             the search. Got {searched:?}"
        );

        // Take blink off the same body, leave everything else including the policy alone, and
        // it must fail again.
        let no_blink = falling_body(
            AbilitySet {
                move_horizontal: true,
                ..AbilitySet::NONE
            },
            start,
        );
        let poisoned = probe_recovery(
            &world,
            &no_blink,
            frame,
            default_probe.with_policy(DRIFT_AND_BLINK),
        );
        assert!(
            !poisoned.regained(),
            "poison: pressing blink on a body that cannot blink must change \
             nothing — got {poisoned:?}"
        );
    }
}

#[cfg(test)]
mod dodge_shadows_the_dash {
    //! What a platform fighter LOSES when the dash leaves its kit: nothing.

    use super::*;
    use crate::abilities::AbilitySet;
    use crate::body_clusters::BodyClusterScratch;
    use crate::movement::input::{ActionEdges, Edge, MovementAction};
    use crate::movement::tuning::{AxisSweptParams, MovementTuning, DEFAULT_TUNING};
    use crate::reference_frame::LocalAxes;
    use crate::{Block, MotionFrame, Vec2, World};

    /// The shipped platform-fighter kit, with the traversal burst switched on or
    /// off. Everything else is held identical — the whole point is that ONE bit
    /// is the variable.
    fn kit(dash: bool) -> AbilitySet {
        AbilitySet {
            move_horizontal: true,
            jump: true,
            variable_jump: true,
            double_jump: true,
            fast_fall: true,
            dash,
            attack: true,
            pogo: true,
            directional_primary: true,
            shield: true,
            dodge: true,
            ledge_grab: true,
            ..AbilitySet::NONE
        }
    }

    /// The fighter's authored movement tuning — the jump squat and the air-dodge
    /// window a platform fighter authors on top of the engine defaults.
    fn fighter_tuning() -> AxisSweptParams {
        MovementTuning {
            jump_squat_time: 3.0 / 60.0,
            air_dodge_time: crate::movement::tuning::AIR_DODGE_TIME,
            air_dodge_speed: crate::movement::tuning::AIR_DODGE_SPEED,
            air_dodge_endlag: crate::movement::tuning::AIR_DODGE_ENDLAG,
            tumble_speed: 500.0,
            ..DEFAULT_TUNING
        }
        .axis_swept_params()
    }

    /// A stage shaped like the smash demo's: ONE contiguous 480x32 platform in a
    /// 640x480 room, its top face at y = 300 and its left ledge at x = 80.
    ///
    ///  there is no gap on it, and that is half the verdict. Crossing this
    /// stage is `move_horizontal` and nothing else; no jump, no burst and no
    /// ability bit is between a fighter and the far ledge. The only reachability
    /// question a fighter stage HAS is getting back to a ledge from off it, which
    /// is what the probe below measures.
    fn stage() -> World {
        World::new(
            "smash-shaped stage",
            Vec2::new(640.0, 480.0),
            Vec2::new(320.0, 204.0),
            vec![Block::solid(
                "platform",
                Vec2::new(80.0, 300.0),
                Vec2::new(480.0, 32.0),
            )],
        )
    }

    /// [`RecoveryPolicy::DRIFT_AND_JUMP`] plus ONE press of the shared burst
    /// button, a few steps in — what a player recovering offstage does.
    ///
    ///  the shipped policy deliberately presses nothing but drift and jump, so
    /// on its own it could never tell a dash-capable body from a dash-less one.
    /// A comparison run under it would have been a check that cannot fail.
    fn drift_jump_and_burst_press(at: RecoveryStep) -> InputState {
        let side = [0.0_f32, -1.0, 1.0].get(at.effort).copied().unwrap_or(0.0);
        let jump = Edge {
            pressed: !at.on_ground && !at.rising,
            held: true,
            released: false,
        };
        let burst = Edge {
            pressed: at.step == 6,
            held: at.step == 6,
            released: false,
        };
        InputState {
            axes: LocalAxes::new(side, 0.0),
            movement: ActionEdges::<MovementAction>::EMPTY
                .with(MovementAction::Jump, jump)
                .with(MovementAction::Burst, burst),
            ..Default::default()
        }
    }

    const DRIFT_JUMP_AND_BURST_PRESS: RecoveryPolicy = RecoveryPolicy {
        name: "drift+jump+burst-press",
        efforts: 3,
        input: drift_jump_and_burst_press,
        burst: None,
    };

    /// The furthest offstage x-offset from the left ledge (x = 80) this policy
    /// gets this body home from, starting `dy` below the platform's top face.
    /// `-1.0` means it never got home from any offset.
    fn furthest_recovered(abilities: AbilitySet, policy: RecoveryPolicy, dy: f32) -> f32 {
        let world = stage();
        let frame = MotionFrame::from_acceleration(Vec2::new(0.0, 1.0) * crate::movement::GRAVITY)
            .expect("gravity is non-zero");
        let mut best = -1.0_f32;
        let mut dx = 0.0_f32;
        while dx <= 700.0 {
            let mut body =
                BodyClusterScratch::new_with_abilities(Vec2::new(80.0 - dx, 300.0 + dy), abilities);
            body.model = crate::movement::MotionModel::axis_swept(fighter_tuning());
            body.ground.on_ground = false;
            let probe = RecoveryProbe::default().with_policy(policy);
            if probe_recovery(&world, &body, frame, probe).regained() {
                best = dx;
            }
            dx += 10.0;
        }
        best
    }

    /// A BODY THAT OWNS THE DODGE CAN NEVER SPEND ITS PRESS ON A DASH, SO
    /// TAKING THE DASH AWAY COSTS IT NOTHING.
    ///
    /// Dodge outranks dash on the shared burst press ([`super:super:abilities:
    /// BurstManeuver`]), and a platform fighter authors an air-dodge window — so airborne,
    /// every press it makes is already an air dodge.
    ///
    /// ```text
    ///   dy   drift+jump      +burst press      poison: dash, NO dodge
    ///  -40   170 / 170       180 / 180          370
    ///    0   150 / 150       150 / 150          330
    ///   40   140 / 140       130 / 130          280
    ///  120    -1 /  -1        -1 /  -1           -1
    /// ```
    ///
    ///  the poison column is the whole test. Strip the DODGE and the same
    /// press reaches `apply_dash`, and the body recovers from twice as far — so
    /// the instrument plainly CAN see a dash, and the identical columns are a
    /// fact about the kit rather than about a probe that never pressed anything.
    ///
    ///  the deep rows (`dy >= 120`) are where an up-B earns its keep; the
    /// buttons alone reach nothing from there WITH or WITHOUT the dash.
    #[test]
    fn removing_the_dash_from_a_dodging_kit_changes_no_reach() {
        let mut saw_a_reachable_row = false;
        let mut saw_the_poison_pay_off = false;
        for dy in [-40.0_f32, 0.0, 40.0] {
            for policy in [RecoveryPolicy::DRIFT_AND_JUMP, DRIFT_JUMP_AND_BURST_PRESS] {
                let without = furthest_recovered(kit(false), policy, dy);
                let with = furthest_recovered(kit(true), policy, dy);
                assert_eq!(
                    without, with,
                    "dy={dy}, policy {}: the fighter reaches {without}px without the \
                     dash and {with}px with it — the dash is doing work in this kit \
                     after all, and D146 removed a real option",
                    policy.name
                );
                if without > 0.0 {
                    saw_a_reachable_row = true;
                }
            }
            // POISON: the same press on a body that owns the dash and NOT the
            // dodge reaches `apply_dash`, and gets home from much further.
            let dash_only = AbilitySet {
                dodge: false,
                ..kit(true)
            };
            let poisoned = furthest_recovered(dash_only, DRIFT_JUMP_AND_BURST_PRESS, dy);
            let dodging = furthest_recovered(kit(true), DRIFT_JUMP_AND_BURST_PRESS, dy);
            if poisoned > dodging + 50.0 {
                saw_the_poison_pay_off = true;
            }
        }
        assert!(
            saw_a_reachable_row,
            "no row recovered from anywhere, so the equality above compared two \
             failures and proved nothing"
        );
        assert!(
            saw_the_poison_pay_off,
            "stripping the DODGE did not extend the reach on any row, so this \
             probe cannot see a dash and the equality it asserts is vacuous"
        );
    }
}
