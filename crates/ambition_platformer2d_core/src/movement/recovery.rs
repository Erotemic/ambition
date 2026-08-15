//! **Can this body still get back to something to stand on?**
//!
//! The sibling of [`containment`](crate::movement::containment), and the same
//! shape: put a body in a world, drive the REAL kernel, report what happened.
//! Containment asks whether a movement POLICY stays in a room. This asks
//! whether a particular BODY, from where it is right now and with the verbs and
//! numbers it actually carries, can regain support.
//!
//! ## Why this is a measurement and not a rule
//!
//! The rule was tried. A fighter-brain rollout priced a KO from *"airborne,
//! below the platform top, outside the ground span ⇒ already dead"*, measured a
//! sixfold survival improvement, and the rule was **removed anyway** (Jon,
//! 2026-08-14) because it is not body-generic: air movement, an unspent jump,
//! flight, a wall, a ledge grab, a recovery attack, an impulse, a portal or a
//! grapple each falsify it. It happened to hold for one stage and one fighter.
//!
//! ⭐ **so this module states no rule about bodies at all.** It clones the body,
//! hands it to [`step_motion`], and watches. Every capability the kernel
//! implements — air jumps, glide, fast fall, wall cling and wall jump, dash,
//! blink, ledge grab, swim, flight, one-way platforms, moving-platform carry,
//! hazards, the blast margin — is honoured because the kernel honours it, gated
//! by the body's own [`AbilitySet`] and its own
//! [`AxisSweptParams`](crate::AxisSweptParams). There is no
//! list of capabilities here to fall out of date, which is precisely the failure
//! mode the deleted rule had.
//!
//! ⛔ **and it does not decide what the answer MEANS.** Like `FrameEvents`, this
//! reports what physically happened; a brain, an authoring validator or an LLM
//! decides whether "no support found" is a death, a level-design bug, or a
//! reason to turn around. Nothing in the engine reads it today, deliberately.
//!
//! ## What it does NOT cover, and these are gaps of the KERNEL, not assumptions
//! ## of this query
//!
//! `step_motion` is the whole world here, so anything that moves a body from
//! OUTSIDE it is invisible: portal transit, a grapple (which is a held item, not
//! an ability flag), a recovery attack, knockback, and any launch a game writes
//! into `BodyFlightState::pending_launch` after the probe was taken. Geometry is
//! whatever `world` contains at the instant of the call — a moving platform is
//! frozen where it stands, so a route that only exists while the platform is
//! elsewhere will not be found. State this to a caller in a world that has those
//! things; do not silently treat [`RecoveryOutlook::NoSupportFound`] as final.
//!
//! ## Cost, and rollback
//!
//! Three efforts times [`RecoveryProbe::steps`] kernel steps, on a CLONE. It
//! mutates nothing, caches nothing, and latches nothing across frames, so it is
//! **not rollback state** and owes no registration — recompute it whenever the
//! answer is wanted. It is far too expensive to run per body per tick; it is
//! sized for analysis, authoring validation, and offline reasoning.

use crate::abilities::{AbilityGrant, AbilitySet};
use crate::body_clusters::BodyClusterScratch;
use crate::movement::kernel::{step_motion, MotionStepContext};
use crate::movement::{
    ActionEdges, Edge, InputState, MotionModel, MotionModelSpec, MovementAction, ResetCause,
};
use crate::{LocalAxes, MotionFrame, World};

/// How long to watch, and at what timestep.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecoveryProbe {
    /// Kernel steps per effort. A probe that stops before the body would have
    /// landed reports "no support" for the wrong reason — the exact arithmetic
    /// that broke the fighter's rollout (its horizon was 12 ticks and the fall
    /// took 24), so the default is deliberately far longer than any plausible
    /// fall.
    pub steps: usize,
    pub dt: f32,
}

impl RecoveryProbe {
    /// Four seconds at 60Hz. A 480px fall under the default gravity takes
    /// 0.65s; terminal velocity crosses any authored room well inside this.
    pub const DEFAULT_STEPS: usize = 240;

    /// Watch for `seconds` at the given fixed timestep.
    pub fn seconds(seconds: f32, dt: f32) -> Self {
        Self {
            steps: (seconds / dt.max(f32::EPSILON)).ceil().max(0.0) as usize,
            dt,
        }
    }
}

impl Default for RecoveryProbe {
    fn default() -> Self {
        Self {
            steps: Self::DEFAULT_STEPS,
            dt: 1.0 / 60.0,
        }
    }
}

/// The steering efforts tried, in this order.
///
/// Body-LOCAL side, so it rotates with gravity. Standing still first, because a
/// body that already has support should report that without a story about which
/// way it ran. Fixed and ordered because two efforts can both succeed and the
/// answer must not depend on iteration luck (ADR 0023).
const EFFORTS: [f32; 3] = [0.0, -1.0, 1.0];

/// What the probe saw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RecoveryOutlook {
    /// The body came to rest on, rode, clung to or caught hold of something,
    /// after `steps` kernel steps while holding `side`.
    Regained { steps: usize, side: f32 },
    /// No effort regained support inside the probe's horizon.
    ///
    /// `reset` is `Some` only when EVERY effort ended in a world reset — the
    /// world killed the body whichever way it steered, which is a stronger and
    /// different fact from "still falling when I stopped watching". `None`
    /// means at least one effort was still going, so the horizon, not the
    /// world, ended it.
    NoSupportFound { reset: Option<ResetCause> },
}

impl RecoveryOutlook {
    pub fn regained(self) -> bool {
        matches!(self, Self::Regained { .. })
    }
}

/// **Drive this body's own kernel at full recovery effort and report whether it
/// gets back to support.**
///
/// `body` is an ordinary [`BodyClusterScratch`] — the same 18 clusters plus
/// [`MotionModel`] a live entity carries — so its position, velocity, spent air
/// jumps, dash charges, ability set and authored tuning all come along. There is
/// no second body vocabulary to keep in step with the first.
///
/// `frame` carries gravity's direction AND magnitude, so the answer is
/// gravity-generic: rotate the world and the body and the verdict rotates with
/// them.
pub fn probe_recovery(
    world: &World,
    body: &BodyClusterScratch,
    frame: MotionFrame,
    probe: RecoveryProbe,
) -> RecoveryOutlook {
    let mut every_effort_was_reset = true;
    let mut cause = None;
    for side in EFFORTS {
        match run_effort(world, body, frame, probe, side) {
            EffortOutcome::Regained(steps) => return RecoveryOutlook::Regained { steps, side },
            EffortOutcome::Reset(reset) => cause = cause.or(Some(reset)),
            EffortOutcome::StillFalling => every_effort_was_reset = false,
        }
    }
    RecoveryOutlook::NoSupportFound {
        reset: if every_effort_was_reset { cause } else { None },
    }
}

/// The grants tried by [`recovery_capability_gap`], in this order.
const GAP_CANDIDATES: [AbilityGrant; 3] = [
    AbilityGrant::RunJump,
    AbilityGrant::AirJump,
    AbilityGrant::WallMobility,
];

/// **Which authored grant would have made this position recoverable?**
///
/// The plan's *"which capability blocks the route"*, answered in the engine's
/// own authoring vocabulary rather than a new one: union one [`AbilityGrant`]
/// onto the body's kit, top up only the budget that grant newly pays for, and
/// re-probe. `None` when the body already recovers, when nothing in the tried
/// list changes the answer, or when the grant it needs is not in the list.
///
/// ⚠ **the list is short on purpose.** A grant is tried only when granting it is
/// completely expressed by the [`AbilitySet`] plus a resource top-up.
/// [`AbilityGrant::FreeFlight`] and [`AbilityGrant::SandboxAll`] are excluded
/// because permanent flight is LATCHED into `BodyFlightState` when a body is
/// built (`fly && !fly_toggle`), not derived from the ability set — granting
/// `fly` to an already-built body would report a capability that does not
/// actually fly. [`AbilityGrant::FastFall`] is excluded because falling faster
/// never puts a surface in reach. Widening it is a real follow-up, not an
/// oversight; each addition owes the same "granting it is fully expressed here"
/// argument.
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

/// One effort: hold `side`, hold jump, and re-press jump the moment the body
/// stops rising.
///
/// ⭐ that press rule is what makes this "full effort" without becoming a
/// SEARCH. Pressing every tick would spend a whole air-jump budget in
/// consecutive frames and climb less than one jump; pressing at the top of the
/// arc is what a human does and what chains the most height out of the budget,
/// and it is a reactive rule rather than a plan the caller had to supply.
/// Holding the button between presses is load-bearing too: a held jump is what
/// opens a cape/glide and what stops a variable-jump law from cutting the arc
/// short.
fn run_effort(
    world: &World,
    body: &BodyClusterScratch,
    frame: MotionFrame,
    probe: RecoveryProbe,
    side: f32,
) -> EffortOutcome {
    let mut scratch = body.clone();
    for step in 0..probe.steps {
        let rising = scratch.kinematics.vel.dot(frame.down()) < 0.0;
        let jump = Edge {
            pressed: !scratch.ground.on_ground && !rising,
            held: true,
            released: false,
        };
        let result = {
            let (model, mut clusters) = scratch.parts();
            step_motion(
                model,
                &mut clusters,
                MotionStepContext {
                    world,
                    input: InputState {
                        axes: LocalAxes::new(side, 0.0),
                        movement: ActionEdges::<MovementAction>::EMPTY
                            .with(MovementAction::Jump, jump),
                        ..Default::default()
                    },
                    frame,
                    facing_intent: side,
                    dt: probe.dt,
                },
            )
        };
        // ⛔ BEFORE the support test, and that order is the whole reason this is
        // checked at all. The out-of-bounds gate REPORTS; it does not move the
        // body. A body whose owner would have respawned it keeps falling here,
        // and in a world with a floor under the blast zone it would eventually
        // land on it and be reported as having recovered from a position it had
        // already died in.
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

    /// **The verdict comes from the BODY's kit, not from where the body is.**
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

    /// **"Airborne, below the lip, outside the span" is not a verdict.**
    ///
    /// That predicate was implemented as a rollout terminal value, measured, and
    /// removed (Jon, 2026-08-14) because it is a claim about one stage wearing
    /// the clothes of a claim about bodies. This pins the replacement: the state
    /// it called dead is answered by the SURFACES, and the poison shows the
    /// answer really did come from the surface rather than from a permissive
    /// probe — take that one block away and the same body in the same place is
    /// reported unrecovered.
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

    /// **The answer rotates with gravity.**
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
}
