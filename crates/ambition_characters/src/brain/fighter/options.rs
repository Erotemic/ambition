//! Pure fighter option generation and utility scoring.
//!
//! Inputs are the allowed [`Perceived`] view, the body's own moveset/capabilities, and
//! difficulty-specific [`UtilityWeights`]. Movement options come from body capabilities; attack
//! options come from frame data. Utility features cover 2-D reach fit, frame advantage, victim
//! damage/kill potential, stage risk, and normalized expected payoff.
//!
//! Decision cadence and calibration live outside this module. The weights here are starting
//! values; ladder evaluation owns tuning.

use crate::brain::attack_kit::{ActionLegality, AttackBinding, AttackCandidate};
use ambition_entity_catalog::MoveFrameData;
use ambition_platformer2d_core as ae;


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
    /// The evade — a ground roll when the feet are down, an air dodge when
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
    /// `0..=1`. 1 when I am against a blastzone. Costed, not rewarded — its
    /// weight is negative in [`UtilityWeights::v1`].
    pub stage_risk: f32,
    /// `0..=1`. The move's power (its `max_damage` over the kit's strongest),
    /// gated by the positive part of `frame_advantage` — payoff only counts
    /// when the move plausibly lands. Zero across the board in neutral, so the
    /// original four features decide there (FB6a).
    pub expected_payoff: f32,
    /// `0..=1`. What HOLDING this opponent is worth right now — zero for
    /// every move that is not a capture. See [`capture_value`].
    ///
    /// A shielding opponent therefore reports zero commitment, every startup is a gamble,
    /// `frame_advantage` clamps to `-1`, and the gate multiplies by zero. Routing a hold's worth
    /// through that gate would have deleted it in exactly the situation a grab exists to answer.
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
    /// Prices what a HOLD is worth ( policy half). Positive.
    ///
    /// `serde(default)` because the authored profiles are RON-in-Rust
    /// literals (`brain_builders.rs`, the fighter content schema's tests) that
    /// spell all five of the older weights and cannot spell this one. Without a
    /// default they would fail to parse; with it they keep their meaning and
    /// take the tuned value. that is also the hazard — a literal that MEANT
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
    /// v1 starting values, not tuned. FB4's ladder self-play monotonicity gate
    /// is the calibration instrument (§FB6). Reach dominates because an attack
    /// that cannot touch the opponent has no other virtue.
    pub fn v1() -> Self {
        Self {
            reach_fit: 1.0,
            frame_advantage: 0.6,
            kill_potential: 0.4,
            stage_risk: -0.8,
            expected_payoff: 0.5,
            // a v1 starting value like its neighbours, not a tuned one.
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

/// Every move in this kit that OFFERS A WAY HOME, in a deterministic order.
///
/// ⭐⭐ THE FILTER IS THE ROUTE, NOT THE LIFT. This asked `lift_speed > 0.0`,
/// which is the shape of exactly one route kind — the genre's ordinary up-B —
/// and so it could not see the pirate's shark (seconds of movement authority)
/// or the Author's teleport (a discontinuity). Both author a real way home and
/// both read `0.0` here, so the CPU saw a fighter with no recovery at all
/// (D250). `MoveFrameData::recovery_route` is the resolved answer and this asks
/// it.
///
/// ⛔ LEGALITY STILL APPLIES, and it is the same rule as before: a body past the
/// blastzone has one problem, and a route it cannot BEGIN does not solve it.
///
/// ⛔ THE ORDER IS NOT A RANKING. It is a deterministic prefix for bounded
/// probing (ADR 0023) and the LENS decides usefulness from the current world —
/// which is the whole reason it is not sorted by "how much lift". Bursts come
/// first by lift because that is the order this list has always had and the
/// existing seats depend on nothing else; the carrying routes follow, longest
/// carry first, then move id.
pub fn lifting_candidates(kit: &[AttackCandidate]) -> Vec<&AttackCandidate> {
    let mut lifts: Vec<&AttackCandidate> = kit
        .iter()
        .filter(|c| {
            c.legality == ActionLegality::Now && c.frames.recovery_route.offers_a_way_home()
        })
        .collect();
    let key = |c: &AttackCandidate| match c.frames.recovery_route {
        ambition_entity_catalog::RecoveryRoute::Burst { speed, .. } => (0u8, speed),
        other => (1u8, other.carry()),
    };
    lifts.sort_by(|a, b| {
        let (a_kind, a_size) = key(a);
        let (b_kind, b_size) = key(b);
        a_kind
            .cmp(&b_kind)
            .then_with(|| {
                b_size
                    .partial_cmp(&a_size)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.move_id.cmp(&b.move_id))
    });
    lifts
}

/// L2. Enumerate and score every legal option for this tick.
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

    // Nothing here knows whose body it is.
    let lifts = lifting_candidates(kit);

    // Movement first: it is the only thing `Recovery` has.
    let mut movement = movement_options(&view, situation, !lifts.is_empty());
    sort_by_score_then_name(&mut movement, |m| (m.score, verb_order(m.verb)));

    if situation == Situation::Recovery {
        // Recovery attacks are candidates that lift the body, ordered only by lift.
        // The recovery lens decides whether any candidate actually reaches safety;
        // callers must not treat the first candidate as an endorsed route.
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

    // Preserve relative direction because attack coverage is directional.
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
    // ⭐ THE KIT'S SLOWEST STARTUP, which is what `frame_advantage` must be
    // normalised by for the RANKING. See the two call sites below: they ask
    // different questions and so want different scales, and the one that asks
    // "how exposed does this leave me" was being normalised by the move's own
    // startup — which divides the speed out of the feature that PRICES speed.
    // Against an uncommitted opponent, which is most of neutral, every attack in
    // the kit then reported exactly `-1.0` and the term cancelled out of the
    // ranking entirely, leaving reach as the only discriminator.
    let kit_slowest_startup = kit
        .iter()
        .map(|c| c.frames.startup_s)
        .fold(0.0f32, f32::max);
    // And the kit's SLOWEST startup, which is the scale frame advantage is
    // measured on.
    //
    // ⛔⛔ IT USED TO DIVIDE BY THE MOVE'S OWN STARTUP, so against an
    // uncommitted opponent — `their_commitment` zero, which is most of neutral —
    // every attack in the kit reported exactly `-1.0`. The feature that exists
    // to price SPEED normalised the speed away, a three-frame jab and a
    // twenty-frame smash scored identically, and the constant cancelled out of
    // the ranking entirely. Measured 2026-08-23: two CPUs threw dash attacks,
    // specials, grabs, throws, tilts and aerials over ninety seconds and NOT ONE
    // JAB, because with the speed term dead the only thing separating attacks in
    // neutral was reach — and a jab has the least of it.
    //
    // ⚠ the test that should have caught it asserted `slower <= faster` and both
    // sides were `-1.0`, so it passed while its own comment said *"a slower move
    // is a worse one"*. It is a strict `<` now.
    // ⛔⛔ AND FIXING IT ALONE IS NOT SHIPPABLE, which is why this is still the
    // move's own startup. Measured 2026-08-23: scaling by the kit's slowest
    // startup instead does exactly what it should to the ranking — jab 0 -> 7 in
    // thirty seconds, `smash_up` back on the board, and George vs George gains
    // damage 292-389-402 -> 369-418-498, tumbling 98-210-589 -> 358-572-1006,
    // techs 36-111-258 -> 110-226-343 across five 90s streams. And then
    // `npc_pirate_admiral` vs itself falls from taking 169% of its pool in a
    // minute to 49%, because preferring speed over reach makes that kit whiff.
    //
    // The weights were fitted while this feature was CONSTANT. Making it vary
    // re-prices every attack in every kit at once, and a change that doubles one
    // matchup while thirding another needs the ladder rig
    // (`brain::fighter::evaluation` + `scenarios`), not a coordinator's
    // judgement. Tracked as D188.
    // ⇒ the scale to pass, when the weights are refitted, is
    // `kit.iter().map(|c| c.frames.startup_s).fold(0.0, f32::max)`.
    let mut attacks: Vec<AttackOption> = kit
        .iter()
        // AN ATTACK THE BODY CANNOT BEGIN IS NOT AN OPTION. (measured
        // ) Sibling to the "cannot reach" filter below, and the other
        // half of the same sentence: that one refuses a move that cannot touch
        // the opponent, this refuses one the BODY cannot start. `capture_probe`
        // measured 33 of 54 CPU grab presses issued while a smash already owned
        // the body, every one dropped by `trigger_moveset_moves`.
        //
        // filtered here rather than scored low, and rather than filtered
        // after scoring: `attacks.first()` always answers, so an impossible move
        // priced low still wins whenever the rest of the kit prices worse — and
        // an option that cannot happen should never have become an option.
        //
        // the legality is the CALLER's answer, from the same `cancel_permits`
        // question the trigger system asks. A brain guessing from its own phase
        // would be answering a different question than the one that drops the
        // press.
        .filter(|c| c.legality == ActionLegality::Now)
        .map(|c| {
            use crate::brain::attack_kit::AttackVerb;
            let fa = frame_advantage(c.frames.startup_s, their_commitment, kit_slowest_startup);
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
                // TWO DIFFERENT QUESTIONS, so two different scales. The
                // ranking's `fa` asks *how exposed does this leave me* and is
                // measured against the kit's slowest move, so a jab and a smash
                // differ. The payoff gate asks *does this move FIT the opening*,
                // which is a comparison between one move's startup and one
                // window and is normalised by that move's own startup — the
                // original reading, kept exactly where it was right.
                //
                // Collapsing them cost the demo its smashes: with one shared
                // scale a slow move's negative `fa` zeroed its payoff in every
                // situation, and the CPU stopped charging entirely.
                expected_payoff: power
                    * frame_advantage(c.frames.startup_s, their_commitment, c.frames.startup_s)
                        .max(0.0),
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
    // AN ATTACK THAT CANNOT REACH IS NOT AN OPTION.
    //
    // `reach_fit` priced a hopeless swing at zero and left it in the list, and
    // the consumer takes `attacks.first()` whenever L3 names nothing — so a
    // fighter with its foe 300px away still pressed a 40px jab, every decision.
    // Each press costs `SLASH_RECOIL` (110 px/s) BACKWARDS along its facing, and
    // in the air almost nothing bleeds that off, so the presses ratchet: the
    // `ladder_probe` trace reads 200, 310, 420, 530 px/s in exactly 110 steps
    // while the brain's own emitted input points the other way. The fighter
    // swung itself off the stage, backwards, one whiff at a time.
    //
    // Scoring it low was never going to be enough: the list is never empty, so
    // `first()` always answers. Not offering it is the fix — and it is what the
    // feature's own doc already says, that a miss by a mile and a miss by two are
    // equally useless.
    //
    // a move that lands NO volume is NOT filtered. `coverage_fit` returns 0
    // for a buff or a summon because hitting is not its question; dropping those
    // would delete a whole class of move from every kit that has one. Only a move
    // that HAS a hittable region and cannot cover where the foe is goes.
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

/// Score how well a move's authored hittable region covers the opponent.
///
/// `foe_local` and `foe_extent` are evaluated in the body's facing-relative 2-D
/// frame, so vertical, rearward, and forward coverage are distinguished. Moves
/// with no hittable volume return zero and rely on their other option features.
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
/// 1-D on purpose — it is the shape of the judgement, and
/// [`coverage_fit`] owns which direction it is applied along.
pub fn reach_fit(reach: f32, gap: f32) -> f32 {
    if reach <= 0.0 {
        return 0.0;
    }
    let miss = (gap - reach).abs();
    (1.0 - miss / (reach * REACH_TOLERANCE)).clamp(0.0, 1.0)
}

/// Context value of acquiring a capture on `foe`, normalized to `[0, 1]`.
///
/// Captures have no damage payoff of their own, so their value comes from current
/// opponent state. Dead, invulnerable, airborne, or already-hitstunned targets are not
/// valuable/eligible; guarding and vulnerable commitments increase value. Reach is scored
/// separately by [`coverage_fit`].
pub fn capture_value(foe: &PerceivedActor) -> f32 {
    // Nothing to hold, or nothing that can be held.
    if !foe.alive || foe.invulnerable {
        return 0.0;
    }
    // a body already reeling is the WRONG grab. It is in hitstun, so it
    // is about to be hit again by anything at all; spending the grab's startup
    // to catch it trades a live combo for a hold. This is also the case where a
    // naive "they cannot answer, so grab" rule would score highest, which is
    // why it is refused explicitly rather than left to the weights.
    if matches!(foe.phase, BodyPhase::Hitstun) {
        return 0.0;
    }
    // AN AIRBORNE BODY CANNOT BE HELD AT ALL, so a hold on one is worth
    // exactly nothing. This is not a preference: `acquire_captures` skips any
    // victim whose `ground.on_ground` is false, so a grab thrown at a body in
    // the air plays its animation, costs its recovery and catches nobody.  the
    // brain was buying an outcome the rules refuse to sell.
    //
    // stated here rather than as a filter on the candidate, because "can
    // this land" is already `reach_fit`'s job for geometry and this is not
    // geometry: the body is inside the box and still cannot be caught. It is a
    // fact about what a hold is WORTH, which is this function's whole subject.
    if !foe.on_ground {
        return 0.0;
    }
    // THE GUARD. A raised shield makes every damaging option worth nothing
    // and a grab worth everything — the one answer the genre has. Grounded,
    // because a shield is a grounded posture and an airborne body's guard is not
    // the thing this beats.
    let guard = if foe.shield_raised && foe.on_ground {
        GRAB_BEATS_GUARD
    } else {
        0.0
    };
    // THE CONVERSION. A throw off a hold sends them further the higher they
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

/// `+1` when the attack lands a full startup before the opponent can answer; `-1` when it is a full
/// startup too slow.
pub fn frame_advantage(startup_s: f32, their_commitment_s: f32, slowest_startup_s: f32) -> f32 {
    let scale = slowest_startup_s.max(0.01);
    ((their_commitment_s - startup_s) / scale).clamp(-1.0, 1.0)
}

/// Movement verbs permitted by the body's capability mask, with coarse scores
/// for the situation's immediate obligation: recover, evade, or approach.
///
/// `walks_off` protects approach choices on open stages. Its margin is one body
/// width so a fighter does not deliberately stop on the ledge boundary.
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
    // Does the body's own kit contain a move that lifts it? A fact about the
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
        // The ledge penalty is applied HERE, at the one place every verb is
        // scored, rather than at each `push` site. A per-situation penalty is
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

    // JUMP IS A CAPABILITY LIKE THE OTHERS. Every verb below asks whether
    // the body can do it — `can_blink`, `can_shield`, `can_dash` — and Jump
    // alone was offered unconditionally, so a body airborne with an empty jump
    // budget was handed an option pressing does nothing for. That is worse than
    // a wasted press: L3 rolls the verb, the shadow's `Jump` is gated on the
    // same budget so the line goes nowhere, and "nowhere" scores as safe.
    let can_jump = me.on_ground || me.air_jumps_left > 0;
    // ONE BUTTON, AND THE BODY DECIDES WHAT IT MEANS.
    //
    // this asked `me.can_dash` and named the verb `Dash`, for every body. But
    // `apply_dodge` claims the dash buffer BEFORE `apply_dash` can see it, so a
    // body owning the dodge ability performs a ROLL — different speed, different
    // commitment, its own cooldown — and never dashes at all. The Smash fighters
    // author `dash: true` and `dodge: true` together (P4.30), which means every
    // burst this brain has ever chosen on that stage came out as a roll while
    // the shadow rollout scored it as a dash. The brain named one maneuver, the
    // model judged a second, the body performed a third.
    //
    // AND THE FIRST REPAIR ASKED THE WRONG QUESTION TOO. It read
    // `can_dodge` / `can_dash`, which are CAPABILITIES — what the body owns, not
    // what a press produces *now*. A dodge on cooldown declines without
    // consuming the buffered press and `apply_dash` takes it, so the brain went
    // on saying "Dodge" while the body dashed. Both repairs were duplicating the
    // movement kernel's precedence rules from the outside, which is the thing
    // that keeps going wrong.
    //
    //  the body RESOLVES the press and perception carries the answer
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
            // A BODY WITH A REAL RECOVERY MOVE DOES NOT BLINK HOME.
            //
            // Blink is a TRAVERSAL verb — a general-purpose way of being
            // somewhere else — and using it as a recovery is the placeholder a
            // fighter reaches for when its repertoire has no answer. Once the
            // kit contains a move that lifts the body, the answer is that move,
            // pressed on the ordinary attack seam like any other.
            //
            // derived, not decided. `kit_lifts` is a fact about the
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
            // A SHIELD IS A REACTION, NOT A STANCE — and scoring it as a
            // stance produced a match of two statues.
            //
            // `Disadvantage` covers "in hitstun" AND "cornered", and on a small stage two fighters
            // who open near the edges are BOTH cornered on the first tick. Shield outscored
            // Retreat, shielding does not un-corner anybody, and the situation that selected it
            // therefore never changes: an absorbing state, one per fighter, reached in the opening
            // second and held for the rest of the match.
            //
            // the genre's own answer is the fix: you shield an ATTACK. A
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
            // a roll is a real answer to a swing, and a dash is not. The
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
