//! **FB2 — L2, the option generator and utility scorer.**
//!
//! `docs/planning/engine/fighter-brain.md` §1:
//!
//! > *"L2 — Option generator + utility scorer: per state, enumerate legal options
//! > from DATA — movement verbs from the body's capability mask, and attacks from
//! > the frame-data table (CM7 …) — the brain knows its moveset the way a player
//! > who read the frame data does, and automatically understands any NEW character
//! > it's put in. Score = authored utility features (range vs. option reach, frame
//! > advantage, kill potential at victim's damage meter, stage position risk) with
//! > per-difficulty weights."*
//!
//! Pure. Every input is the [`Perceived`] view the no-cheat contract allows, plus the
//! body's own kit and its difficulty's [`UtilityWeights`].
//!
//! ## The four features, and why each is a fact about the VIEW
//!
//! - **`reach_fit`** — can this attack's hittable REGION cover where the
//!   opponent actually is? A jab at three body-lengths scores nothing, and
//!   neither does an up-tilt against someone standing in front of you. This is
//!   what makes the brain *understand a new character*: the region comes from
//!   CM7's frame data (`MoveFrameData::coverage`), not from a table someone
//!   typed. ⚠ the WEIGHT keeps the name `reach_fit` because ladders author it by
//!   that name; what changed is that both terms are 2-D — see [`coverage_fit`].
//! - **`frame_advantage`** — will this attack's `startup_s` beat what the opponent
//!   is already committed to (`phase_remaining`)? Positive means it lands first.
//!   A player who read the frame data knows this number; so does the brain.
//! - **`kill_potential`** — the victim's `damage_frac`. In a smash-percent game a
//!   move's value is not its damage but who it can end.
//! - **`stage_risk`** — how little stage is behind ME. Committing to a long
//!   recovery near a blastzone is how a level-9 CPU dies to a level-3 one.
//!
//! ## What is NOT here
//!
//! **The weights are not tuned.** §FB6: *"Scoring weights are NOT divined up
//! front: v1 weights are authored starting values, then FB4's ladder self-play
//! monotonicity gate is the calibration instrument (adjust until levels order
//! correctly)."* [`UtilityWeights::v1`] is that starting value, and it is a
//! starting value, not a claim.
//!
//! **The decision cadence is not here.** §5: *"rebuilt per decision tick (not per
//! frame — decide at ~10–20 Hz gated by reaction latency, hold intents between
//! decisions)."* The latency lives on `FighterBrainProfile.reaction_ms`, which is
//! FB4's; L2 is a pure function that a decision tick calls.
//!
//! ## The gap in §1's four features — found by FB2, CLOSED by FB6a
//!
//! FB2 found that **none of the original four features read a move's POWER**:
//! `kill_potential` is the *victim's* meter; `reach_fit` and `frame_advantage`
//! are geometry and timing; `stage_risk` is about me. At any weights, given a
//! punish window both a jab and a smash fit, the jab won — faster, so more
//! frame advantage, and nothing priced the smash's payoff. A level-9 CPU that
//! always jabs its punishes is not a level-9 CPU. It was recorded rather than
//! patched, because §FB6 makes FB4's ladder the calibration instrument.
//!
//! FB6a took the recorded route (1): [`MoveFrameData`] now carries
//! `max_damage`/`max_knockback` (derived over the Active volumes exactly like
//! `reach`), and the fifth feature is
//! **`expected_payoff` = (this move's `max_damage` ÷ the kit's strongest
//! `max_damage`) × landing chance**, where the landing chance is the positive
//! part of `frame_advantage`. In neutral (nobody committed) every payoff is
//! zero and the original four decide; in a punish window the smash finally
//! outbids the jab it out-damages — which is the exact scenario FB2 recorded.
//! Power is normalized within the KIT so the feature is scale-free across
//! characters, per the same reasoning that lets the brain understand a
//! character nobody wrote a table for. The WEIGHT remains a v1 starting value;
//! the ladder still calibrates it.

use ambition_entity_catalog::MoveFrameData;
use ambition_platformer2d_core as ae;

use crate::actor::attack_gesture::AttackDir;

use super::situation::{is_punishable, Situation};
use crate::perception::{BodyPhase, Perceived, PerceivedActor};

/// One movement verb the body can attempt. Derived from `SelfView`'s capability
/// mask — the body-enforced floor (invariant I3), so the brain can only propose
/// what the body could accept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MovementVerb {
    /// Close the gap on the ground.
    Approach,
    /// Open the gap on the ground.
    Retreat,
    Jump,
    Dash,
    /// **The evade** — a ground roll when the feet are down, an air dodge when
    /// they are not. Shares the dash BUTTON with [`Self::Dash`] and is never
    /// offered beside it: the body resolves one press to one maneuver, and which
    /// one is settled by whether it owns the dodge ability.
    Dodge,
    Shield,
    Blink,
    /// Toward the stage's center. The only verb `Recovery` cares about.
    Recover,
}

/// A scored movement option.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveOption {
    pub verb: MovementVerb,
    pub score: f32,
}

/// One attack the body's kit can throw, with the frame data a player who read the
/// tables would know.
#[derive(Clone, Debug, PartialEq)]
pub struct AttackOption {
    pub move_id: String,
    pub frames: MoveFrameData,
    /// The press that reaches [`Self::move_id`]. Carried from the candidate so
    /// the decision that WINS can be executed as the move it scored.
    pub binding: AttackBinding,
    pub score: f32,
    /// The features that produced `score`, so a failing ladder run can be read
    /// rather than guessed at. `Σ weight_i · feature_i` is `score` by construction.
    pub features: Features,
}

/// The four features, unweighted, each in a bounded range so a weight is a
/// comparable number rather than a unit conversion.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Features {
    /// `0..=1`. 1 when the attack's hittable region already covers the
    /// opponent's hurtbox, falling off with the miss ([`coverage_fit`]).
    pub reach_fit: f32,
    /// `-1..=1`. Positive when `startup_s` beats the opponent's commitment.
    pub frame_advantage: f32,
    /// `0..=1`. The victim's accumulated damage fraction.
    pub kill_potential: f32,
    /// `0..=1`. 1 when I am against a blastzone. **Costed, not rewarded** — its
    /// weight is negative in [`UtilityWeights::v1`].
    pub stage_risk: f32,
    /// `0..=1`. The move's power (its `max_damage` over the kit's strongest),
    /// gated by the positive part of `frame_advantage` — payoff only counts
    /// when the move plausibly lands. Zero across the board in neutral, so the
    /// original four features decide there (FB6a).
    pub expected_payoff: f32,
    /// `0..=1`. **What HOLDING this opponent is worth right now** — zero for
    /// every move that is not a capture. See [`capture_value`].
    ///
    /// ⛔⛔ **it is a feature of its own rather than a value routed through
    /// [`Self::expected_payoff`], and the reason is a trap found before it was
    /// written.** `expected_payoff` is `power * frame_advantage.max(0.0)`, and
    /// `frame_advantage` is measured against `is_punishable(foe)` — which is
    /// `AttackStartup | AttackRecovery | Hitstun` and *not* `Shielding`. A
    /// shielding opponent therefore reports zero commitment, every startup is a
    /// gamble, `frame_advantage` clamps to `-1`, and the gate multiplies by
    /// zero. Routing a hold's worth through that gate would have deleted it in
    /// exactly the situation a grab exists to answer.
    pub capture_value: f32,
}

impl Features {
    fn dot(&self, w: &UtilityWeights) -> f32 {
        self.reach_fit * w.reach_fit
            + self.frame_advantage * w.frame_advantage
            + self.kill_potential * w.kill_potential
            + self.stage_risk * w.stage_risk
            + self.expected_payoff * w.expected_payoff
            + self.capture_value * w.capture_value
    }
}

/// Per-difficulty scoring weights. Content in the end (`FighterBrainProfile`'s
/// `utility_weights`); a struct here so L2 stays pure.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct UtilityWeights {
    pub reach_fit: f32,
    pub frame_advantage: f32,
    pub kill_potential: f32,
    /// Negative: stage risk is a cost.
    pub stage_risk: f32,
    /// Prices a move's POWER on a plausible landing (FB6a). Positive.
    pub expected_payoff: f32,
    /// Prices what a HOLD is worth (D166's policy half). Positive.
    ///
    /// ⚠ **`serde(default)` because the authored profiles are RON-in-Rust
    /// literals** (`brain_builders.rs`, the fighter content schema's tests) that
    /// spell all five of the older weights and cannot spell this one. Without a
    /// default they would fail to parse; with it they keep their meaning and
    /// take the tuned value. ⛔ that is also the hazard — a literal that MEANT
    /// to zero this reads identically to one that never heard of it, so a
    /// profile which wants no grabs must say so.
    #[serde(default = "default_capture_value_weight")]
    pub capture_value: f32,
}

/// The `capture_value` weight an authored profile gets when it does not name one.
fn default_capture_value_weight() -> f32 {
    UtilityWeights::v1().capture_value
}

impl UtilityWeights {
    /// **v1 starting values, not tuned.** FB4's ladder self-play monotonicity gate
    /// is the calibration instrument (§FB6). Reach dominates because an attack
    /// that cannot touch the opponent has no other virtue.
    pub fn v1() -> Self {
        Self {
            reach_fit: 1.0,
            frame_advantage: 0.6,
            kill_potential: 0.4,
            stage_risk: -0.8,
            expected_payoff: 0.5,
            // ⚠ **a v1 starting value like its neighbours, not a tuned one.**
            // Sized deliberately BELOW `reach_fit`'s 1.0 so that no amount of
            // hold value can buy a grab thrown from outside its own reach —
            // which is the exact failure the reverted "a grab is worth its
            // forward throw's damage" experiment produced.
            capture_value: 0.5,
        }
    }
}

impl Default for UtilityWeights {
    fn default() -> Self {
        Self::v1()
    }
}

/// **How a chosen attack is actually PRESSED.**
///
/// ⚠ **this is the half the brain used to score and then discard** (GPT 5.6,
/// 2026-07-31, finding 2). L2 scored every move in the kit, L3 refined the
/// choice, `RefinedChoice::move_id` named a concrete move — and the emission set
/// `melee_pressed = true` with a neutral axis, so `trigger_moveset_moves`
/// resolved whatever the DEFAULT gesture maps to. The brain decided whether to
/// attack and never which attack.
///
/// It is the ordinary gesture vocabulary, not a fighter-only bypass: a verb plus
/// a direction is exactly what a human's stick and button produce, and what
/// `move_for_directional_verb` consumes. The POSTURE is deliberately absent — the
/// body's real grounded state decides it at press time, and a brain that could
/// claim a posture it does not have would be reaching past the no-cheat contract
/// to pick a move its body cannot reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttackBinding {
    pub verb: AttackVerb,
    pub direction: AttackDir,
}

/// The three press KINDS a moveset distinguishes. Not the move — the button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AttackVerb {
    /// The plain attack button (`"attack"` and its directional variants).
    #[default]
    Basic,
    /// Attack with a smash/strong hint (`"smash_*"`, falling back to `attack_*`).
    Smash,
    /// The special button (`"special"` and its directional variants).
    Special,
    /// The GRAB button (`"grab"`).
    ///
    /// ⚠ **no directional variants, and a CENTRED stick.** A grab is a button,
    /// not a stick gesture — a deflection beside it would arm a flick the next
    /// ordinary attack would inherit as an accidental smash.
    Grab,
}

/// One attack the caller's kit offers. The caller resolves these from the body's
/// moveset; L2 never queries anything.
///
/// The caller enumerates BINDINGS and asks the moveset what each one reaches, so
/// a candidate is a move the body can actually be made to perform — a move with
/// no binding (a buff, a summon, an on-hit technique) never enters the kit, and
/// a scored choice is executable by construction.
#[derive(Clone, Debug, PartialEq)]
pub struct AttackCandidate {
    pub move_id: String,
    pub frames: MoveFrameData,
    pub binding: AttackBinding,
    /// **Whether the BODY can begin this move on this tick** — see
    /// [`ActionLegality`]. Supplied by the caller, which is the only layer that
    /// can see the running `MovePlayback`.
    pub legality: ActionLegality,
}

/// **CAN this action begin right now?** — a question about the BODY's state,
/// kept deliberately separate from *how useful would it be*, which is the
/// scorer's ([`Features`]) subject.
///
/// ⛔⛔ **the two were conflated and it was measured.** `capture_probe`,
/// 2026-08-19: of 54 CPU grab presses in a sixty-second match, **33 were issued
/// while a smash already owned the body** and were dropped by
/// `trigger_moveset_moves` before they did anything. The brain was scoring an
/// action it could not perform, every tick, and no feature could express that
/// because it is not a question about the opponent or the geometry at all.
///
/// ⭐ **it is a FILTER, never a weight.** A cheap move that cannot be started is
/// not a slightly worse option than one that can — it is not an option. Pricing
/// it low would leave it winning whenever the kit is bad, which is exactly how
/// the "an attack that cannot REACH is not an option" filter came to exist one
/// class over.
///
/// ⚠ **the third state is deliberately absent and named here so it is not
/// invented twice.** Once `BodyActionBuffer` is actually fed, a press that
/// cannot execute *now* but would be consumed on the first actionable frame
/// becomes `BufferableSoon`, and the brain may legitimately issue it. Until
/// then, "legal eventually" must not read as "press now".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActionLegality {
    /// Nothing owns the body, or the running move's cancel windows admit this
    /// one under its hit-state condition.
    #[default]
    Now,
    /// Another move owns the body and its cancel windows do not admit this one.
    /// The press would be discarded, so the brain does not spend it.
    BlockedByPlayback,
}

/// L2's working set for one decision tick.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OptionSet {
    /// Scored movement verbs, best first.
    pub movement: Vec<MoveOption>,
    /// Scored attacks, best first. In [`Situation::Recovery`] this holds ONLY
    /// the kit's lifting moves ([`lifting_candidates`]) — a body past the
    /// blastzone has exactly one problem, and a move that solves that problem is
    /// not an offensive option, it is the answer to it.
    pub attacks: Vec<AttackOption>,
}

impl OptionSet {
    pub fn best_attack(&self) -> Option<&AttackOption> {
        self.attacks.first()
    }

    pub fn best_movement(&self) -> Option<MoveOption> {
        self.movement.first().copied()
    }
}

/// How far past its own reach an attack is still worth considering. Beyond this
/// the fit is zero rather than negative — an attack that misses by a mile and one
/// that misses by two are equally useless, and letting the feature go negative
/// would let a big negative reach_fit be bought back by kill potential.
const REACH_TOLERANCE: f32 = 2.0;

/// **Which of these moves COMMAND A DISPLACEMENT with an against-gravity
/// component**, strongest rise first, ties on the move id.
///
/// ⭐ **a list of CANDIDATE ROUTES — proposals — and nothing more.** Each entry
/// is a move whose authored `Set` impulse would move its owner, in the catalog's
/// derived terms (`lift_speed` up, `lift_side` along facing). So a route is
/// recognised by what the move does to the BODY, exactly the way `reach`
/// recognises a poke and `max_damage` recognises a kill move.
///
/// ⛔⛔ **THE ORDER IS NOT A RANKING OF USEFULNESS, and reading it as one is the
/// defect this doc used to describe as a feature.** It said *"this is the whole
/// of 'the brain understands recovery moves', and it is one number"* — and one
/// number is precisely what it cannot be. A fighter whose way home is a grapple
/// that trades its energy for lateral distance advertises a SMALL rise, so any
/// stall-and-juggle aerial in the same kit sorts above it here. Taking
/// `.first()` as "the recovery" then hands the search a move that goes nowhere
/// and never explores the one that works. Which route is useful depends on where
/// the body IS, and the only authority on that is the movement kernel —
/// [`RecoveryLens::best_route`](super::recovery::RecoveryLens::best_route) asks
/// it, over this whole list, in this order.
///
/// ⚠ so the sort exists for DETERMINISM (ADR 0023) and for the search's cut at
/// [`MAX_PROBED_ROUTES`](super::recovery::MAX_PROBED_ROUTES) — a stable prefix,
/// never a claim.
///
/// ⛔⛔ **A NAMED, OPEN GAP: A PURELY HORIZONTAL DISPLACEMENT IS INVISIBLE HERE.**
/// The filter is `lift_speed > 0`, and a move that commands `Set (760, 0)` has
/// none — so a body whose way home is a flat charge is never proposed as having
/// one. This is not hypothetical: `smash_george_booul`'s `modus_ponens` is
/// described in its own authoring comment as *"a real horizontal recovery"* and
/// this function cannot see it.
///
/// ⛔ **and the fix is NOT to widen the derivation.** `lift_side` already carries
/// the number; what is missing is a reason to propose a move with no
/// against-gravity component at all, which means proposing EVERY displacing move
/// and paying for the probes. That is a decision about search cost — it belongs
/// with [`MAX_PROBED_ROUTES`] and wants a measurement, not another predicate.
/// Recorded rather than half-built.
///
/// ⛔ **no character conditional, and no role taxonomy.** There is no list of
/// which body's special is the Up-B, and there is deliberately no `MoveRole`
/// enum: the day a second body authors a displacing move it is understood here
/// for free, and the day a body authors none this returns empty and the brain
/// plays exactly as it did before.
///
/// ⚠ **the POSTURE filter is upstream and load-bearing.** The kit is built for
/// the body's real grounded state, so an airborne body's kit already contains
/// only airborne-legal moves — which is why nothing here has to ask whether a
/// grounded-only move could be pressed off the stage.
pub fn lifting_candidates(kit: &[AttackCandidate]) -> Vec<&AttackCandidate> {
    // ⚠ **legality applies here too, and it is the same rule.** A body past the
    // blastzone has one problem, but a lifting move it cannot BEGIN does not
    // solve it — offering one would make `Recovery` name a route the press
    // cannot take, which is the failure this filter exists to stop one layer up.
    let mut lifts: Vec<&AttackCandidate> = kit
        .iter()
        .filter(|c| c.legality == ActionLegality::Now && c.frames.lift_speed > 0.0)
        .collect();
    lifts.sort_by(|a, b| {
        b.frames
            .lift_speed
            .partial_cmp(&a.frames.lift_speed)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.move_id.cmp(&b.move_id))
    });
    lifts
}

/// **L2.** Enumerate and score every legal option for this tick.
///
/// `situation` is L1's answer, passed in rather than recomputed: the two layers
/// must agree about the tick, and a second `classify` call on a delayed view could
/// disagree with the first.
pub fn generate_options(
    view: Perceived<'_>,
    situation: Situation,
    kit: &[AttackCandidate],
    weights: &UtilityWeights,
) -> OptionSet {
    let me = &view.self_view;
    let foe = view.nearest_hostile();

    // ⭐ **DOES THIS BODY OWN A WAY UP?** Derived from the kit's own numbers —
    // see [`lifting_candidates`] — and handed to movement scoring so that a
    // fighter with a real recovery move stops being offered the traversal verb
    // it used to fall back on. Nothing here knows whose body it is.
    let lifts = lifting_candidates(kit);

    // Movement first: it is the only thing `Recovery` has.
    let mut movement = movement_options(&view, situation, !lifts.is_empty());
    sort_by_score_then_name(&mut movement, |m| (m.score, verb_order(m.verb)));

    if situation == Situation::Recovery {
        // **THE ONE ATTACK A RECOVERING BODY MAY THROW IS THE ONE THAT LIFTS
        // IT.**
        //
        // ⛔ this list used to be empty, unconditionally, and the reason given
        // was right about attacking and wrong about the repertoire: *"a body
        // past the blastzone has exactly one problem"*. It does — and a genre
        // fighter's answer to that problem IS a move, pressed on the ordinary
        // attack seam. Refusing to offer it left the brain drifting and jumping
        // at a stage it could not reach while the body carried the thing that
        // would have got it home.
        //
        // ⚠ scored on LIFT ALONE, deliberately. Reach, frame advantage and
        // payoff are questions about an opponent, and a recovering body is not
        // having a conversation with one — the whole utility vocabulary is the
        // wrong instrument here, and borrowing it would price a way home by how
        // hard it hits.
        //
        // ⛔⛔ **AND THIS ORDER IS A PROPOSAL, NOT THE ANSWER.** L2 is pure: it
        // has no world, no kernel and no idea where the body will be, so the
        // most it can say is *"these are the moves that displace me, biggest
        // rise first."* The DECISION overrides this with the route the recovery
        // lens actually got home on (`decide`'s `endorsed_recovery`), because a
        // move's usefulness from a particular place is a physics question and
        // this function cannot ask one. If a caller ever takes `.first()` here
        // as the recovery, the tiny-rising-aerial trap is back.
        //
        // ⭐ **the caller that did exactly that is GONE (2026-08-15).** `decide`
        // fell through to `options.attacks.first()` whenever the search endorsed
        // nothing, which on the smash stage was 97 of 100 recovery decisions; it
        // now presses NOTHING on that branch, so the kernel's answer stands in
        // both directions and this list is a proposal in fact and not only in
        // prose. ⚠ the warning above is kept because it is about the NEXT
        // caller, not the last one.
        //
        // ⚠ **and it is the ORDER the lens searches**, so the two layers agree
        // about which route index means which move.
        let mut attacks: Vec<AttackOption> = lifts
            .into_iter()
            .map(|c| AttackOption {
                move_id: c.move_id.clone(),
                frames: c.frames.clone(),
                binding: c.binding,
                score: c.frames.lift_speed,
                features: Features::default(),
            })
            .collect();
        // Ties on lift break on the move id, never on kit order (ADR 0023).
        attacks.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.move_id.cmp(&b.move_id))
        });
        return OptionSet { movement, attacks };
    }
    if foe.is_none() {
        return OptionSet {
            movement,
            attacks: Vec::new(),
        };
    }
    let foe = foe.expect("checked");

    // ⭐⭐ **WHERE THE OPPONENT IS, not merely how far away.** This was
    // `let gap = (foe.pos - me.pos).length()`, and throwing the direction away is
    // the whole of why no kit's vertical game was ever selected — see
    // [`coverage_fit`].
    //
    // Body-local and facing-relative, the frame the authored volumes are in:
    // `+x` toward this body's facing, `+y` toward its feet.
    let to_foe = foe.pos - me.pos;
    let basis = me.acceleration_frame();
    let foe_local = (
        to_foe.dot(basis.side) * if me.facing < 0.0 { -1.0 } else { 1.0 },
        to_foe.dot(basis.down),
    );
    // A hitbox catches a HURTBOX. Asking whether the foe's CENTRE is inside a
    // volume would refuse every move that clips a tall body's shoulder.
    let foe_extent = (foe.half_extent.x, foe.half_extent.y);
    let stage_risk = {
        let half_stage = (view.stage.bounds.max - view.stage.bounds.min).length() * 0.5;
        if half_stage <= 0.0 {
            1.0
        } else {
            (1.0 - view.stage.distance_to_edge(me.pos) / half_stage).clamp(0.0, 1.0)
        }
    };
    // A committed opponent cannot answer for `phase_remaining` seconds. An
    // uncommitted one answers immediately, so any startup at all is a gamble.
    let their_commitment = if is_punishable(foe, me.gravity_down) {
        foe.phase_remaining
    } else {
        0.0
    };

    // The kit's strongest hit, for scale-free power pricing (FB6a). Zero when
    // no candidate lands a volume, which zeroes every payoff below.
    let kit_max_damage = kit.iter().map(|c| c.frames.max_damage).max().unwrap_or(0);
    let mut attacks: Vec<AttackOption> = kit
        .iter()
        // **AN ATTACK THE BODY CANNOT BEGIN IS NOT AN OPTION.** (measured
        // 2026-08-19) Sibling to the "cannot reach" filter below, and the other
        // half of the same sentence: that one refuses a move that cannot touch
        // the opponent, this refuses one the BODY cannot start. `capture_probe`
        // measured 33 of 54 CPU grab presses issued while a smash already owned
        // the body, every one dropped by `trigger_moveset_moves`.
        //
        // ⭐ **filtered here rather than scored low**, and rather than filtered
        // after scoring: `attacks.first()` always answers, so an impossible move
        // priced low still wins whenever the rest of the kit prices worse — and
        // an option that cannot happen should never have become an option.
        //
        // ⚠ the legality is the CALLER's answer, from the same `cancel_permits`
        // question the trigger system asks. A brain guessing from its own phase
        // would be answering a different question than the one that drops the
        // press.
        .filter(|c| c.legality == ActionLegality::Now)
        .map(|c| {
            use super::options::AttackVerb;
            let fa = frame_advantage(c.frames.startup_s, their_commitment);
            let power = if kit_max_damage > 0 {
                c.frames.max_damage as f32 / kit_max_damage as f32
            } else {
                0.0
            };
            let features = Features {
                reach_fit: coverage_fit(c.frames.coverage.as_ref(), foe_local, foe_extent),
                frame_advantage: fa,
                kill_potential: foe.damage_frac(),
                stage_risk,
                expected_payoff: power * fa.max(0.0),
                // Only a capture asks this question, and `capture_value` answers
                // zero for everything else — stated at the call site so the
                // feature cannot quietly start pricing ordinary swings.
                capture_value: match c.binding.verb {
                    AttackVerb::Grab => capture_value(foe),
                    AttackVerb::Basic | AttackVerb::Smash | AttackVerb::Special => 0.0,
                },
            };
            AttackOption {
                move_id: c.move_id.clone(),
                frames: c.frames.clone(),
                binding: c.binding,
                score: features.dot(weights),
                features,
            }
        })
        .collect();
    // **AN ATTACK THAT CANNOT REACH IS NOT AN OPTION.** (traced 2026-07-31)
    //
    // `reach_fit` priced a hopeless swing at zero and left it in the list, and
    // the consumer takes `attacks.first()` whenever L3 names nothing — so a
    // fighter with its foe 300px away still pressed a 40px jab, every decision.
    // Each press costs `SLASH_RECOIL` (110 px/s) BACKWARDS along its facing, and
    // in the air almost nothing bleeds that off, so the presses ratchet: the
    // `ladder_probe` trace reads 200, 310, 420, 530 px/s in exactly 110 steps
    // while the brain's own emitted input points the other way. **The fighter
    // swung itself off the stage, backwards, one whiff at a time.**
    //
    // Scoring it low was never going to be enough: the list is never empty, so
    // `first()` always answers. Not offering it is the fix — and it is what the
    // feature's own doc already says, that a miss by a mile and a miss by two are
    // equally useless.
    //
    // ⚠ **a move that lands NO volume is NOT filtered.** `coverage_fit` returns 0
    // for a buff or a summon because hitting is not its question; dropping those
    // would delete a whole class of move from every kit that has one. Only a move
    // that HAS a hittable region and cannot cover where the foe is goes.
    //
    // ⛔ this asked `frames.reach <= 0.0`, and `reach` is the `+x` face alone —
    // so every move whose volume sits BEHIND the body (a back-air's does)
    // reported reach `0.0`, took this exemption, and rode into every ranking at
    // any distance with a fit of zero. Measured 2026-08-15: `air_back` was the
    // most-selected move in one seat's whole `Disadvantage` column while being
    // the one move the filter could not see.
    attacks.retain(|attack| attack.frames.coverage.is_none() || attack.features.reach_fit > 0.0);


    // Ties break on the move id, so the best option is a function of the world and
    // not of the kit's declaration order (ADR 0023: no order-dependent decisions).
    attacks.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.move_id.cmp(&b.move_id))
    });

    OptionSet { movement, attacks }
}

/// **`1` when this move's hittable region already covers the opponent**, falling
/// to `0` as the miss grows past [`REACH_TOLERANCE`] × the region's own span. A
/// move that lands no volume (a buff, a summon, a pure-motion recovery) has no
/// fit anywhere and must be priced by its other features alone.
///
/// ⛔⛔ **this replaces a 1-D `reach_fit(reach, gap)` and the difference is the
/// single largest thing measured wrong in the fighter brain** (2026-08-15,
/// CPU-versus-CPU). `reach` is only the `+x` face of the authored volumes and
/// `gap` was `(foe.pos - me.pos).length()` — a scalar against a scalar — so:
///
/// * an up-tilt whose volume sits above the shoulder projected onto `+x` as a
///   ~30px poke, scored identically to the jab, and lost the tie on startup.
///   **Every** anti-air, juggle and spike in **every** kit was therefore unpicked
///   for the reason it was authored, and the CPU played one plane of a two-plane
///   game;
/// * a move BEHIND the body reported `reach = 0` (the fold clamps at zero) and
///   was exempted from the "cannot reach" filter entirely;
/// * and the old shape scored a miss the same whether the opponent was that far
///   AWAY or that far ABOVE.
///
/// ⭐ `foe_local` is the opponent's centre in the body's own facing-relative
/// frame and `foe_extent` its half-extent, so the question asked is the one the
/// hitbox will actually answer: *would this volume overlap that hurtbox from
/// here?* Nothing here reads a move id, a character or a role.
///
/// ⭐ **the CURVE is the old one, unchanged, and that is deliberate.** What
/// changed is only which `reach` it is asked about: the move's extent along the
/// line to the opponent rather than along `+x`. So the spacing behaviour a match
/// already had — different moves winning at different distances, a lunge scoring
/// badly from touching range — survives intact, and gains the other axis.
///
/// ⛔⛔ **a flat "inside the box scores 1" was tried first and is WRONG, measured
/// the same afternoon.** With every covering move scoring 1.0, and
/// `frame_advantage` pinned at `-1` for every move in neutral (nobody is
/// committed, so nothing beats anything) and `expected_payoff` therefore zero,
/// EVERY grounded option tied on total score — and the ADR-0023 tiebreak, which
/// exists so a choice never depends on iteration luck, handed the match to
/// whichever move id sorts first. George Booul threw `bivalence` 18 times out of
/// 33 and never threw his recovery. A gate that does not discriminate does not
/// stop being the discriminator; it just delegates to the alphabet.
pub fn coverage_fit(
    coverage: Option<&ambition_entity_catalog::MoveCoverage>,
    foe_local: (f32, f32),
    foe_extent: (f32, f32),
) -> f32 {
    let Some(coverage) = coverage else {
        return 0.0;
    };
    // How far the move reaches THAT WAY, against how far away they are.
    let reach = coverage.extent_toward(foe_local, foe_extent);
    let gap = (foe_local.0 * foe_local.0 + foe_local.1 * foe_local.1).sqrt();
    reach_fit(reach, gap)
}

/// `1` when the attack's reach exactly spans the gap, falling to `0` as the miss
/// grows past [`REACH_TOLERANCE`] × reach. A move that reaches nowhere in the
/// asked direction (a buff, a summon, an up-tilt against a foe on the floor
/// beside you) has no fit and must be priced by its other features alone.
///
/// ⚠ **1-D on purpose** — it is the shape of the judgement, and
/// [`coverage_fit`] owns which direction it is applied along.
pub fn reach_fit(reach: f32, gap: f32) -> f32 {
    if reach <= 0.0 {
        return 0.0;
    }
    let miss = (gap - reach).abs();
    (1.0 - miss / (reach * REACH_TOLERANCE)).clamp(0.0, 1.0)
}

/// **WHAT A HOLD IS WORTH — the platform-fighter policy term the damage road
/// cannot express (D166's open half, ruled 2026-08-19).**
///
/// A capture deals NO DAMAGE. `max_damage` is what a move does on contact, so a
/// grab's power is honestly zero and `expected_payoff` is honestly zero with it.
/// What a capture actually buys is that the opponent is HELD, and that is worth
/// a different amount on different ticks — which is why it is a function of the
/// opponent rather than a constant on the move.
///
/// ⛔⛔ **THE MEASURED MISTAKE THIS REPLACES: pricing the grab at its FOLLOW-UP
/// THROW's damage.** It reads as obviously right — what is catching somebody
/// worth, if not the throw? — and `capture_probe` measured what it bought on
/// 2026-08-18: the CPU grabbed from **110px with a 42px reach**, nine attempts
/// in sixty seconds, none of them inside its own range, zero holds. It was
/// reverted to the honest zero.
///
/// ⇒ **the fault was not the size of the number, it was that the number was
/// UNCONDITIONAL.** A constant payoff makes a grab the best answer to every
/// situation including the ones it cannot reach. Every term below is zero or
/// small unless a specific fact about the opponent is true right now.
///
/// ⭐ **the largest term is the guard, and the tree already said so.**
/// `rollout.rs` writes the triangle down — *"Attack beats grab, grab beats
/// shield, shield beats attack"* — and the L3 rollout has known it since a
/// shielding opponent made the whole kit worth zero and the CPU picked by
/// tie-break. L2 did not know it, and L2 is what `attacks.first()` falls back to
/// whenever L3 names nothing. This is L2 learning the same fact.
///
/// ⚠ **what is deliberately NOT here: escape risk.** What a hold is worth truly
/// depends on whether it can be KEPT, and nothing in the view reports how hard
/// this opponent mashes. Inventing a term for it would be modelling, not
/// measuring. Stated rather than guessed.
///
/// ⭐⭐ **THE CLAMP IS LOAD-BEARING, and a poison attempt is what showed it.**
/// The return is clamped to `0..=1`, so no value of [`GRAB_BEATS_GUARD`] or
/// [`THROW_CONVERSION`] can raise a hold's worth past 1 — raising both tenfold
/// leaves the score unchanged, which is what the first poison of
/// `a_hold_is_never_worth_a_grab_the_body_cannot_reach` measured when the test
/// stayed green. ⇒ **the ceiling on how far a hold can buy a grab is the WEIGHT
/// alone** (`UtilityWeights::capture_value`), and the guard bites when that is
/// raised. A future tuner should move the weight knowing it is the only knob
/// that can reproduce the reverted bug.
///
/// ⚠ v1 starting values, in the same sense as [`UtilityWeights::v1`] — chosen so
/// the relationships are right, not tuned. `capture_probe`'s move histogram is
/// the instrument.
pub fn capture_value(foe: &PerceivedActor) -> f32 {
    // Nothing to hold, or nothing that can be held.
    if !foe.alive || foe.invulnerable {
        return 0.0;
    }
    // ⛔ **a body already reeling is the WRONG grab.** It is in hitstun, so it
    // is about to be hit again by anything at all; spending the grab's startup
    // to catch it trades a live combo for a hold. This is also the case where a
    // naive "they cannot answer, so grab" rule would score highest, which is
    // why it is refused explicitly rather than left to the weights.
    if matches!(foe.phase, BodyPhase::Hitstun) {
        return 0.0;
    }
    // ⛔⛔ **AN AIRBORNE BODY CANNOT BE HELD AT ALL, so a hold on one is worth
    // exactly nothing.** This is not a preference: `acquire_captures` skips any
    // victim whose `ground.on_ground` is false, so a grab thrown at a body in
    // the air plays its animation, costs its recovery and catches nobody. ⇒ the
    // brain was buying an outcome the rules refuse to sell.
    //
    // ⭐ measured 2026-08-19, the run that first produced a hold: 66 capture
    // attempts yielded ONE, and a large share of them asked while the target was
    // airborne. Spacing was no longer the problem by then — the median press had
    // come in from ~110px to 48px — so what remained was throwing a correct grab
    // at an ineligible body.
    //
    // ⚠ **stated here rather than as a filter on the candidate**, because "can
    // this land" is already `reach_fit`'s job for geometry and this is not
    // geometry: the body is inside the box and still cannot be caught. It is a
    // fact about what a hold is WORTH, which is this function's whole subject.
    if !foe.on_ground {
        return 0.0;
    }
    // **THE GUARD.** A raised shield makes every damaging option worth nothing
    // and a grab worth everything — the one answer the genre has. Grounded,
    // because a shield is a grounded posture and an airborne body's guard is not
    // the thing this beats.
    let guard = if foe.shield_raised && foe.on_ground {
        GRAB_BEATS_GUARD
    } else {
        0.0
    };
    // **THE CONVERSION.** A throw off a hold sends them further the higher they
    // are, so the same hold is worth more at 120% than at 0%. Scales with the
    // percent axis the rest of the scorer already reads.
    let convert = foe.damage_frac() * THROW_CONVERSION;
    (guard + convert).clamp(0.0, 1.0)
}

/// What catching a GUARDING opponent is worth — the third leg of the triangle.
/// The dominant term on purpose: it is the situation in which every other option
/// in the kit is worth zero.
const GRAB_BEATS_GUARD: f32 = 0.8;

/// How much of a hold's worth comes from the throw it sets up, at 100%. Kept
/// well under [`GRAB_BEATS_GUARD`] so that percent alone never makes a grab the
/// answer to a neutral opponent standing out of reach.
const THROW_CONVERSION: f32 = 0.35;

/// `+1` when the attack lands a full startup before the opponent can answer; `-1`
/// when it is a full startup too slow. Normalized by the startup so a slow move's
/// disadvantage is measured against its own commitment, not against a wall clock.
pub fn frame_advantage(startup_s: f32, their_commitment_s: f32) -> f32 {
    let scale = startup_s.max(0.01);
    ((their_commitment_s - startup_s) / scale).clamp(-1.0, 1.0)
}

/// Movement verbs the body's capability mask permits, scored by the situation.
///
/// The scores are coarse ON PURPOSE. §1 puts the interesting judgement in the
/// attack scorer and in L3's rollouts; movement's job at L2 is to express the
/// situation's ONE obligation — get back, get out, get in — so that a brain with
/// no L3 still plays a recognizable game.
/// **Would moving `toward` walk this body off the floor it is standing on?**
///
/// ⛔ the defect this closes, measured 2026-07-31 in the smash demo: a fighter
/// lost all three of its stocks WITHOUT BEING HIT, by running past its opponent
/// and off the edge, repeatedly.
///
/// The brain was not wrong — the world changed. L1 has `Situation::Recovery` for
/// a body ALREADY offstage, and until the smash stage every room in this engine
/// was enclosed, so `Approach` was always safe and nothing had to score a ledge.
/// A platform-fighter stage is the first room you can walk out of.
///
/// The margin is a body-width rather than a tuned distance: a fighter that stops
/// exactly at the edge is standing on the one pixel a knockback removes.
fn walks_off(view: &crate::perception::WorldView, toward: f32) -> bool {
    // ONE authority: `WorldView::floor_ahead`, which L1 also asks to classify
    // `Disadvantage`. Two implementations of "where does the floor end" would
    // drift the moment one of them learned about one-way platforms.
    let Some(ahead) = view.floor_ahead(toward) else {
        return false;
    };
    ahead < view.self_view.half_extent.x * 2.0
}

fn movement_options(
    view: &crate::perception::WorldView,
    situation: Situation,
    // **Does the body's own kit contain a move that lifts it?** A fact about the
    // repertoire, derived by [`lifting_candidates`] and passed in rather than
    // re-derived, so movement scoring and the attack list cannot disagree about
    // it within one tick.
    kit_lifts: bool,
) -> Vec<MoveOption> {
    let me = &view.self_view;
    // Which way the foe is, so "approach" and "retreat" can be asked whether the
    // floor is still there. Zero when there is nobody to approach, and a zero
    // direction reads as "no ledge question", which is correct: a brain with no
    // foe is not closing on anything.
    let toward_foe = view
        .actors
        .iter()
        .find(|actor| actor.hostile_to_self && actor.alive)
        .map(|foe| (foe.pos.x - me.pos.x).signum())
        .unwrap_or(0.0);
    let approach_walks_off = toward_foe != 0.0 && walks_off(view, toward_foe);
    let retreat_walks_off = toward_foe != 0.0 && walks_off(view, -toward_foe);
    let mut out = Vec::new();
    let mut push = |verb: MovementVerb, score: f32| {
        // **The ledge penalty is applied HERE, at the one place every verb is
        // scored**, rather than at each `push` site. A per-situation penalty is
        // the kind that gets added to three arms and forgotten in the fourth —
        // and the forgotten arm is always the one a real match spends its time
        // in.
        let score = match verb {
            MovementVerb::Approach | MovementVerb::Dash if approach_walks_off => score - 1.0,
            MovementVerb::Retreat if retreat_walks_off => score - 1.0,
            _ => score,
        };
        out.push(MoveOption { verb, score })
    };

    // **JUMP IS A CAPABILITY LIKE THE OTHERS.** Every verb below asks whether
    // the body can do it — `can_blink`, `can_shield`, `can_dash` — and Jump
    // alone was offered unconditionally, so a body airborne with an empty jump
    // budget was handed an option pressing does nothing for. That is worse than
    // a wasted press: L3 rolls the verb, the shadow's `Jump` is gated on the
    // same budget so the line goes nowhere, and "nowhere" scores as safe.
    let can_jump = me.on_ground || me.air_jumps_left > 0;
    // ⭐⭐ **ONE BUTTON, AND THE BODY DECIDES WHAT IT MEANS.**
    //
    // ⛔ this asked `me.can_dash` and named the verb `Dash`, for every body. But
    // `apply_dodge` claims the dash buffer BEFORE `apply_dash` can see it, so a
    // body owning the dodge ability performs a ROLL — different speed, different
    // commitment, its own cooldown — and never dashes at all. The Smash fighters
    // author `dash: true` and `dodge: true` together (P4.30), which means every
    // burst this brain has ever chosen on that stage came out as a roll while
    // the shadow rollout scored it as a dash. The brain named one maneuver, the
    // model judged a second, the body performed a third.
    //
    // ⛔⛔ **AND THE FIRST REPAIR ASKED THE WRONG QUESTION TOO.** It read
    // `can_dodge` / `can_dash`, which are CAPABILITIES — what the body owns, not
    // what a press produces *now*. A dodge on cooldown declines without
    // consuming the buffered press and `apply_dash` takes it, so the brain went
    // on saying "Dodge" while the body dashed. Both repairs were duplicating the
    // movement kernel's precedence rules from the outside, which is the thing
    // that keeps going wrong.
    //
    // ⇒ the body RESOLVES the press and perception carries the answer
    // ([`BurstManeuver`]). The brain is handed a fact rather than a rule to
    // re-derive.
    let evade = match me.burst {
        ae::BurstManeuver::GroundDodge | ae::BurstManeuver::AirDodge => Some(MovementVerb::Dodge),
        ae::BurstManeuver::Dash => Some(MovementVerb::Dash),
        ae::BurstManeuver::None => None,
    };
    match situation {
        Situation::Recovery => {
            push(MovementVerb::Recover, 1.0);
            // ⭐ **A BODY WITH A REAL RECOVERY MOVE DOES NOT BLINK HOME.**
            //
            // Blink is a TRAVERSAL verb — a general-purpose way of being
            // somewhere else — and using it as a recovery is the placeholder a
            // fighter reaches for when its repertoire has no answer. Once the
            // kit contains a move that lifts the body, the answer is that move,
            // pressed on the ordinary attack seam like any other.
            //
            // ⛔ **derived, not decided.** `kit_lifts` is a fact about the
            // body's own authored numbers, so this rule reads "prefer the
            // authored recovery over the general traversal verb" for every body
            // that has one, and changes nothing at all for every body that does
            // not. A character conditional here would be the thing this brain's
            // whole no-cheat contract exists to forbid.
            if me.can_blink && !kit_lifts {
                push(MovementVerb::Blink, 0.9);
            }
            if can_jump {
                push(MovementVerb::Jump, 0.5);
            }
        }
        Situation::Disadvantage => {
            // **A SHIELD IS A REACTION, NOT A STANCE — and scoring it as a
            // stance produced a match of two statues.**
            //
            // ⛔ measured 2026-08-11, the day the Smash fighters were first
            // given the `shield` capability. `Disadvantage` covers "in hitstun"
            // AND "cornered", and on a small stage two fighters who open near
            // the edges are BOTH cornered on the first tick. Shield outscored
            // Retreat, shielding does not un-corner anybody, and the situation
            // that selected it therefore never changes: an absorbing state, one
            // per fighter, reached in the opening second and held for the rest
            // of the match. The stage was two bodies facing each other with
            // their guard up, forever, and the CPU-versus-CPU regression read
            // `travel: 0.0px`.
            //
            // ⭐ the genre's own answer is the fix: you shield an ATTACK. A
            // cornered player with nothing incoming retreats, rolls or jumps
            // out — pressing guard against nothing is how you get grabbed. So
            // the verb is offered only when a hostile is actually swinging, and
            // "cornered with nothing incoming" falls through to Retreat, which
            // solves the problem being cornered actually poses.
            let threatened = view
                .actors
                .iter()
                .any(|actor| actor.hostile_to_self && actor.alive && actor.phase.is_attacking());
            if me.can_shield && threatened {
                push(MovementVerb::Shield, 0.8);
            }
            push(MovementVerb::Retreat, 0.7);
            // ⭐ **a roll is a real answer to a swing, and a dash is not.** The
            // evade carries i-frames and gets the defensive score; a plain dash
            // is only travel, so a body whose button dashes keeps the lower one
            // it always had. Same slot, two bodies, two honest numbers.
            if let Some(evade) = evade {
                push(
                    evade,
                    if evade == MovementVerb::Dodge {
                        0.75
                    } else {
                        0.6
                    },
                );
            }
            if can_jump {
                push(MovementVerb::Jump, 0.4);
            }
        }
        Situation::EdgeGuard | Situation::Advantage => {
            push(MovementVerb::Approach, 0.8);
            // Rolling IN is an approach with i-frames — the genre's other use
            // for the button — so the offensive slot takes whichever maneuver
            // this body's press produces, at the score the slot always had.
            if let Some(evade) = evade {
                push(evade, 0.7);
            }
            if can_jump {
                push(MovementVerb::Jump, 0.3);
            }
        }
        Situation::Neutral => {
            push(MovementVerb::Approach, 0.5);
            push(MovementVerb::Retreat, 0.4);
            if can_jump {
                push(MovementVerb::Jump, 0.3);
            }
            if let Some(evade) = evade {
                push(evade, 0.3);
            }
        }
    }
    out
}

fn verb_order(v: MovementVerb) -> MovementVerb {
    v
}

fn sort_by_score_then_name<T, K: Ord>(items: &mut [T], key: impl Fn(&T) -> (f32, K)) {
    items.sort_by(|a, b| {
        let (sa, ka) = key(a);
        let (sb, kb) = key(b);
        sb.partial_cmp(&sa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| ka.cmp(&kb))
    });
}

#[cfg(test)]
mod tests;
