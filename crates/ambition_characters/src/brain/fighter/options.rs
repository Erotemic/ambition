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

/// A kit move whose only effect is to MOVE THIS BODY, scored by whether that
/// motion serves the movement objective.
///
/// ⛔⛤ **A MOVE IS NOT AN ATTACK BECAUSE IT HAPPENS TO BE ENCODED AS ONE.** The
/// medic's `medic_rescue_lift` lands no volume and throws her 900 units
/// straight up. Priced on the attack menu it was ranked by frame advantage
/// against an opponent it cannot touch, admitted at every range because there
/// was nothing it could miss, and therefore whatever remained once the gap grew
/// past the kit's reach — thrown 48 times in 91 starts for 39% damage, each
/// press widening the gap so the next decision met the same world.
///
/// ⭐⭐ **THE QUESTION A SELF-MOTION MOVE ANSWERS IS A MOVEMENT QUESTION:** does
/// this displacement take me where I am trying to go. That is what
/// [`MotionOption::score`] measures, and it is why the same move shape can be
/// the medic's trap (the foe is level with her, so a vertical launch goes
/// nowhere useful) and Emmy's approach (it closes a gap her kit cannot
/// otherwise reach) without either being a special case.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionOption {
    pub move_id: String,
    pub frames: MoveFrameData,
    /// The press that reaches [`Self::move_id`] — a motion is still performed
    /// on the attack seam, because that is the button the move is bound to.
    pub binding: AttackBinding,
    /// `0..=1`. How much of the way to the objective this motion actually
    /// travels — a tent over `travelled / gap`, peaking at 1 where the motion
    /// arrives and falling to 0 as it overshoots to twice the gap. See
    /// [`motion_options`].
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
    /// `0..=1`. The victim's accumulated damage fraction, scaled by how hard
    /// THIS move launches relative to the kit's best percent-scaling launch.
    /// A move that cannot KO harder for the damage taken scores zero here
    /// however hurt the opponent is.
    pub kill_potential: f32,
    /// `0..=1`. What THIS ACTION exposes me to: how far its self-motion
    /// carries me toward a blast line (or how near one I already stand),
    /// scaled by how long I am committed to having gone there. Costed, not
    /// rewarded — its weight is negative in [`UtilityWeights::v1`].
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
    /// `0..=1`. What SHOVING this opponent is worth right now — zero for every
    /// move that pushes nothing. See [`displacement_value`].
    ///
    /// ⛔⛤ **WITHOUT IT A PURE SHOVE HAD NO VIRTUE AT ALL.** Splitting hit
    /// coverage from push coverage stopped a waked kick being priced by its
    /// dust, and left the other side of the same split unanswered: a volume
    /// that only pushes has `coverage: None`, so `reach_fit` is zero, and
    /// `damage: 0`, so `expected_payoff` is zero. It was admitted where it can
    /// shove and then scored as worth nothing — offered and never chosen. The
    /// Officer's neutral special `the_order_to_disperse` is exactly that move:
    /// a sustained zero-damage windbox whose whole reason to exist is to make
    /// space and push somebody off a ledge.
    ///
    /// ⚠ NOT `reach_fit` UNDER ANOTHER NAME. Feeding push coverage back into
    /// `reach_fit` would re-merge the two regions the split exists to keep
    /// apart, and price a gust as though it were a hit. This is a separate
    /// feature with its own weight because a shove is worth a DIFFERENT amount
    /// than a hit, and the amount depends on where the foe is standing.
    pub displacement_value: f32,
}

impl Features {
    fn dot(&self, w: &UtilityWeights) -> f32 {
        self.reach_fit * w.reach_fit
            + self.frame_advantage * w.frame_advantage
            + self.kill_potential * w.kill_potential
            + self.stage_risk * w.stage_risk
            + self.expected_payoff * w.expected_payoff
            + self.capture_value * w.capture_value
            + self.displacement_value * w.displacement_value
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
    /// Prices what a SHOVE is worth. Positive.
    ///
    /// `serde(default)` for the same reason as `capture_value` above, and with
    /// the same hazard: an authored profile that MEANT to zero this reads
    /// identically to one written before the feature existed.
    #[serde(default = "default_displacement_value_weight")]
    pub displacement_value: f32,
}

/// The `capture_value` weight an authored profile gets when it does not name one.
fn default_capture_value_weight() -> f32 {
    UtilityWeights::v1().capture_value
}

/// The `displacement_value` weight an authored profile gets when it does not
/// name one.
fn default_displacement_value_weight() -> f32 {
    UtilityWeights::v1().displacement_value
}

impl UtilityWeights {
    /// v1 starting values, not tuned. FB4's ladder self-play monotonicity gate
    /// is the calibration instrument (§FB6). Reach dominates because an attack
    /// that cannot touch the opponent has no other virtue.
    pub fn v1() -> Self {
        Self {
            reach_fit: 1.0,
            frame_advantage: 0.6,
            // ⛔⛤ **0.4 WAS BELOW THE BAND, SO A CORRECT FEATURE STILL COULD
            // NOT MOVE A DECISION.** Making `kill_potential` move-relative gave
            // it the power to rank; it did not give it the SIZE to win. Swept
            // 2026-09-20 against the real Officer kit at one gap, reading which
            // move he opens with as the opponent's meter climbs:
            //
            // ```text
            //   w        0%      40%      80%     120%     150%
            //   0.4-0.6  jab      jab      jab      jab      jab
            //   0.7      jab      jab      jab   smash_up smash_up
            //   0.8-1.5  jab      jab   smash_up smash_up smash_up
            //   1.6+     jab  smash_down smash_up smash_up smash_up
            // ```
            //
            // ⇒ Below 0.7 he pokes a dying opponent; at 1.6 he starts winding
            // up a smash against a nearly fresh one. The band is `0.7..1.6` and
            // 1.1 is the middle of the plateau inside it.
            //
            // ⚠ AND THE AUTHORED LADDER HAS TO MOVE WITH IT: its rungs ran
            // `0.00..0.40`, entirely under the floor of this band, so every
            // shipped CPU was spending a dead weight. See
            // `game/ambition_content/assets/data/fighter_brain_ladder.ron`.
            kill_potential: 1.1,
            stage_risk: -0.8,
            expected_payoff: 0.5,
            // a v1 starting value like its neighbours, not a tuned one.
            // Sized deliberately BELOW `reach_fit`'s 1.0 so that no amount of
            // hold value can buy a grab thrown from outside its own reach —
            // which is the exact failure the reverted "a grab is worth its
            // forward throw's damage" experiment produced.
            capture_value: 0.5,
            // ⭐ A SHOVE AT THE BLAST LINE IS WORTH MORE THAN A HIT, because
            // the hit is what the rest of the kit already does better. The
            // feature is scaled by how close the foe is to going off, so the
            // weight prices the BEST case and the position does the
            // discriminating.
            //
            // ⛔⛤ **1.0 WAS BELOW THE BAND AND THE FEATURE WAS INERT ON THE
            // SHIPPED ROSTER.** The first value was fitted against a two-move
            // fixture -- a 40px jab and a 60px gust -- where the rival was a
            // jab and the recorded band was `0.95..2.1`. Measured 2026-09-19
            // against the REAL Officer kit (eleven candidates, the road
            // `attack_kit_of` builds), the rival at the ledge is
            // `officer_tilt_forward`, not a jab, and it wins at every weight up
            // to 1.2: the gust placed FOURTH beside the blast line. A fixture
            // with two moves cannot price a move against a kit.
            //
            // ⇒ The real band is `1.25..2.6` -- below 1.25 the gust never
            // wins anywhere, above 2.6 it wins at centre stage, which is the
            // pushy CPU nobody asked for. 1.8 is the middle of it.
            //
            // ⚠ POPULATION: two moves in the whole roster author a push region
            // (`officer_disperse`, and the goblin's `dirt_kick`, which also
            // hits). `dirt_kick` never wins at any weight up to 3.0 -- it
            // competes on `reach_fit` like an ordinary move and its own
            // `tilt_forward` outscores it -- so the entire behavioural delta of
            // this number is ONE move on ONE character. See
            // `officer_moveset.rs::he_shoves_at_the_ledge_and_punches_at_centre`.
            displacement_value: 1.8,
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
    /// Scored pure-motion moves, best first. See [`MotionOption`]. Empty for
    /// the kits that author none, which is most of them.
    pub motions: Vec<MotionOption>,
}

impl OptionSet {
    pub fn best_attack(&self) -> Option<&AttackOption> {
        self.attacks.first()
    }

    pub fn best_movement(&self) -> Option<MoveOption> {
        self.movement.first().copied()
    }

    /// The motion worth performing, or `None` when none of them goes anywhere
    /// this body wants to be.
    ///
    /// ⚠ **A THRESHOLD, NOT `first()`.** Every other list here is consumed by
    /// taking the head, because an attack list is a RANKING among things worth
    /// doing. A motion list is not: a kit that authors a vertical launch always
    /// offers it, and pressing it because it is the only entry is exactly the
    /// loop this type exists to end. The head is worth pressing only when the
    /// motion actually carries the body most of the way toward the objective.
    pub fn best_motion(&self) -> Option<&MotionOption> {
        self.motions
            .first()
            .filter(|motion| motion.score >= MOTION_WORTH_PRESSING)
    }
}

/// How well a pure-motion move has to serve the objective before pressing it
/// beats simply walking.
///
/// ⭐ **HALF, AND THE HALF IS THE ARGUMENT.** [`MotionOption`]'s score is
/// alignment times speed share, so `0.5` is "either it points almost exactly
/// where I want to go at moderate speed, or it is the fastest thing I own and
/// points broadly the right way". Below that the body is better off walking,
/// which costs nothing and can be changed its mind about next tick; a move
/// cannot.
pub const MOTION_WORTH_PRESSING: f32 = 0.5;

/// How far past the point where a move's own region touches the opponent it is
/// still ADMITTED, in world pixels.
///
/// ⛔⛤ **ADMISSION USED TO BE `reach_fit > 0.0`, WHICH IS A TOLERANCE
/// MEASURED IN UNITS OF THE MOVE'S OWN REACH — SO THE LONGEST MOVES GOT THE
/// MOST SLACK.** [`REACH_TOLERANCE`] is `2.0`, so the score stays positive
/// while `|gap - reach| < 2 · reach`, i.e. out to THREE TIMES reach: a 40px
/// jab was on the menu at 119px and a 200px sword at 600px. A move that cannot
/// touch them is not more nearly an option because it is a big move.
///
/// ⭐ **AN ABSOLUTE SLACK INSTEAD, THE SAME FOR EVERY MOVE.** `reach` here is
/// already `extent_toward` inflated by the foe's hurtbox, so `gap <= reach` is
/// exactly *"this move's region touches them"*. The slack covers the one thing
/// a static comparison cannot: both bodies are still moving for the interval
/// until the next decision.
///
/// ⚠ **A CONSTANT AND NOT THE FOE'S HALF-WIDTH, though that was the first
/// version and it reads better.** A perceived body's `half_extent` defaults to
/// ZERO, which no real body has and every hand-built fixture does — so a rule
/// scaled by it is razor-tight in exactly the tests that would catch it being
/// wrong. The same trap as `SelfView::facing` defaulting to `0.0`. 24px is
/// about a body half-width on this roster and about a decision interval of
/// walking.
///
/// ⚠ **SCORING IS UNCHANGED.** `reach_fit` keeps its falloff — a poke thrown
/// from too far still scores badly, and a long move at point-blank still scores
/// badly — because that shape is a judgement about VALUE. This constant is a
/// judgement about what belongs on the menu at all, and the two were one number
/// only because one number was cheaper to write.
const ADMISSION_SLACK_PX: f32 = 24.0;

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
/// or the Director's teleport (a discontinuity). Both author a real way home and
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
        // ⚠ NO MOTION LIST IN RECOVERY, and that is not an omission: a body
        // past the blastzone has one objective and `lifting_candidates` above
        // already IS the list of moves that serve it, endorsed by the recovery
        // lens rather than by a gap-closing score.
        return OptionSet {
            movement,
            attacks,
            motions: Vec::new(),
        };
    }
    if foe.is_none() {
        // A motion is scored against where the FOE is. With nobody to go to
        // there is no objective, which is the same reason the foe-relative
        // movement verbs are withheld above.
        return OptionSet {
            movement,
            attacks: Vec::new(),
            motions: Vec::new(),
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
    // ⭐ ONE FORMULA, TWO SUBJECTS. Edge proximity costs ME (`stage_risk`) and
    // pays when it is THEM (`displacement_value`), and reading it the same way
    // for both is what keeps "near the edge" one fact.
    let edge_proximity = |pos| {
        let half_stage = (view.stage.bounds.max - view.stage.bounds.min).length() * 0.5;
        if half_stage <= 0.0 {
            1.0
        } else {
            (1.0 - view.stage.distance_to_edge(pos) / half_stage).clamp(0.0, 1.0)
        }
    };
    let stage_risk_here = edge_proximity(me.pos);
    // ⭐⛤ WHERE THE FOE STANDS, IN WORLD SPACE, so a body-local push direction
    // can be rotated into it and asked how much stage is left THAT WAY. The
    // pusher's own frame and facing travel with it because the authored
    // direction is in the PUSHER's axes. See [`displacement_value`].
    let push_world = PushGeometry {
        basis,
        facing: me.facing,
        stage: &view.stage,
        at: foe.pos,
    };
    // ⭐ THE SAME GEOMETRY READ ON MYSELF. Edge pressure costs me and pays when
    // it is them, and it was already one formula asked of two subjects; this
    // keeps that true now that the question has a DIRECTION in it.
    let my_world = PushGeometry {
        basis,
        facing: me.facing,
        stage: &view.stage,
        at: me.pos,
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
    // ⭐⭐ **THE LAUNCH LAW THIS BODY IS ACTUALLY FIGHTING UNDER, ASSEMBLED
    // ONCE.** It used to be `base + growth × victim_damage` and nothing else,
    // on the reasoning that the ruleset's percent scale, the per-`base`
    // steepening, the victim's weight and rage are common to every candidate.
    // They multiply the PERCENT TERM and not `base`, so they move the CROSSOVER
    // between two candidates rather than scaling both — see
    // `ambition_entity_catalog::launch`, which is the one copy of the law now.
    let launch_conditions = view.launch_law.against(foe);
    // ⛔⛤ **AGAINST THIS OPPONENT, NOT AGAINST A FRESH ONE.** The percent term
    // rides the victim's meter, so which of my moves finishes hardest is a
    // question whose ANSWER MOVES as the opponent wears down — the Pugnacious
    // Polygon's forward and up smashes swap places at about 2 damage. Evaluating
    // the kit under the conditions above is the only way the share below ranks
    // the same order the game's own arithmetic would.
    let kit_max_launch = kit
        .iter()
        .map(|c| c.frames.launch.at(launch_conditions))
        .fold(0.0_f32, f32::max);
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
            let reach_fit = coverage_fit(c.frames.coverage.as_ref(), foe_local, foe_extent);
            // ⚠ BOTH AUTHORED HALVES OF THE SELF-MOTION, because a move can
            // lunge (`start_impulse`) or command a burst (`lift_*`) and either
            // is what puts the body somewhere else. A crude integration over
            // the move's own length is the honest resolution available to a
            // static table: it is the distance the body would cover if nothing
            // bled the impulse off, which is the WORST case, and a risk term
            // is the one place to take the worst case.
            //
            // ⛔⛤ **AND A MOVE THAT OFFERS A WAY HOME IS NOT COSTED FOR MOVING
            // THE BODY, WHICH IS THE WHOLE OF ITS JOB.** Charging every metre
            // travelled against the nearest blast line prices a recovery at
            // maximum — a 900px/s rise over most of a second leaves any stage
            // this game has — so the CPU stopped throwing one at all: measured
            // over 3600 ticks, the two Georges threw nineteen different moves
            // and never his up-B
            // (`the_repertoire_gets_used::the_cpu_throws_its_authored_recovery_during_a_match`).
            // The feature is about being carried somewhere you did not mean to
            // go; a route home is the one motion that is meant.
            let travel = if c.frames.recovery_route.offers_a_way_home() {
                0.0
            } else {
                let (ix, iy) = c.frames.start_impulse;
                let (mx, my) = motion_of(&c.frames);
                let (lx, ly) = (ix + mx, iy + my);
                let speed = (lx * lx + ly * ly).sqrt();
                my_world.travel_share(lx, ly, speed * c.frames.total_s)
            };
            let features = Features {
                reach_fit,
                frame_advantage: fa,
                // ⛔⛤ **THIS WAS THE SAME NUMBER FOR EVERY CANDIDATE, AND AN
                // ATTACK'S SCORE IS ONLY EVER COMPARED WITH ANOTHER ATTACK'S.**
                // `foe.damage_frac()` alone is a fact about the OPPONENT, so it
                // added a constant to every option and could not change a
                // single ranking — measured 2026-09-19: `options.attacks` is
                // read through `first()` and through a lookup by move id, never
                // against a threshold and never against a movement score. Every
                // rung of the authored ladder varies `kill_potential` from 0.0
                // to 0.4 and none of it reached a decision.
                //
                // ⇒ A kill is the foe's percent AND THE LAUNCH THIS MOVE
                // CARRIES. Sharing against the kit's best is the same shape
                // `expected_payoff` uses for damage, so the feature stays
                // `0..=1` and the authored weights keep their scale.
                // ⛔⛤ **AND IT IS GATED ON LANDING, WHICH THE FIRST LIVE VERSION
                // WAS NOT — MEASURED, AND THE MEASUREMENT IS A WHOLE MATCH.**
                // While the feature was inert the smash demo's CPUs knocked
                // each other off the stage; on the first tick it could actually
                // rank with, they stopped, and every fighter stayed inside the
                // room for the entire bout
                // (`the_stage_kills::every_live_fighter_stays_inside_the_frame`,
                // green at `faf775f9e` and red from `1b7ec5ce2`). ⇒ Paying a
                // finisher its full worth from ACROSS THE STAGE buys a smash
                // that whiffs, and a bout of whiffed smashes accumulates no
                // damage and therefore no kills.
                //
                // ⚠ **THE GATE IS `reach_fit` OR A COMMITTED OPPONENT, AND BOTH
                // HALVES ARE THERE BECAUSE A MATCH REDDENED WITHOUT THEM.**
                //
                //  * `expected_payoff`'s frame gate ALONE is zero against an
                //    uncommitted opponent, which is most of neutral — it would
                //    have traded one dead weight for another.
                //  * `reach_fit` ALONE deleted the charge. A smash is chosen
                //    from OUTSIDE its own reach, held while the gap closes, and
                //    released fat; gating its worth on the reach it has RIGHT
                //    NOW means it is only ever worth throwing point-blank,
                //    where its startup loses to every jab in the kit. Measured:
                //    *"no CPU held a smash in any of 3 matches of 5400 ticks"*
                //    (`the_repertoire_gets_used::the_cpu_charges_a_smash_and_techs_a_landing_in_some_match`).
                //
                // ⇒ A finisher is worth something when it can TOUCH them, and
                // also when they cannot ANSWER — which is the window a charge
                // exists for, and the reason the two are a `max` rather than a
                // product.
                kill_potential: foe.damage_frac()
                    * if kit_max_launch > 0.0 {
                        c.frames.launch.at(launch_conditions) / kit_max_launch
                    } else {
                        0.0
                    }
                    * reach_fit.max(fa.max(0.0)),

                // ⛔⛤ **THIS WAS THE SAME NUMBER FOR EVERY CANDIDATE TOO, AND
                // FOR THE SAME REASON `kill_potential` WAS.** `edge_proximity`
                // of where this body is STANDING is a fact about the body, not
                // about the action, so it added a constant to every option and
                // could not rank. The test that showed the same jab scoring
                // lower near a ledge was proving arithmetic.
                //
                // ⇒ A risk is what THIS ACTION exposes me to: how far its own
                // self-motion carries me toward a blast line. `max` with the
                // standing reading rather than replacing it, because the two
                // are the same hazard stated twice — a body already against
                // the line is at risk without moving, and a body mid-stage is
                // at risk if the move throws it out.
                //
                // ⚠ **AND NOT SCALED BY COMMITMENT, THOUGH THE FIRST VERSION
                // WAS.** Multiplying by the move's share of the kit's longest
                // duration penalises every slow move a second time —
                // `frame_advantage` already prices startup — and the smash
                // demo said so immediately: no CPU charged a smash, threw a
                // recovery, or pressed an authored route in 3600 ticks. The
                // duration is in this feature only through the distance the
                // self-motion covers.
                //
                // ⭐⭐ **AND THE SWEEP SAYS WHO THIS IS FOR: ONE ROW OF 21, AND
                // IT IS THE OILER.** Grid sweep, 2026-09-20, mirror matches,
                // 3600 ticks, the other twenty bit-identical:
                //
                // ```text
                //               took0% took1% hitstun moves used/seen/kit
                //   constant      160%   172%     768    70   10/  0/0
                //   per-action    230%   290%    1309   125   18/ 16/32
                // ```
                //
                // ⇒ He throws nearly twice as many moves and almost twice as
                // much of his kit. A constant cost cannot say which of his
                // swings walks him off, so it was paid on all of them equally
                // and he simply swung less.
                stage_risk: stage_risk_here.max(travel),
                displacement_value: displacement_value(
                    c.frames.push_coverage.as_ref(),
                    c.frames.push_dir,
                    foe_local,
                    foe_extent,
                    push_world,
                ),
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
    //
    // ⛔⛤ AND A PURE SHOVE IS THE THIRD CASE, WHICH THE TWO ABOVE SILENTLY
    // MERGED. `MoveFrameData::coverage` used to be the union of the hit volumes
    // AND the windboxes, so a gust read as a hittable move and a waked kick read
    // as reaching as far as its dust. Splitting the datum fixes the kick — its
    // boot is now what `reach_fit` scores — and would have made the gust
    // `coverage: None`, i.e. offered at any range at all, which is the same
    // defect on the other foot. A move that can only PUSH is gated on where it
    // can push.
    //
    // ⛔⛤ **AND THE HIT ARM ASKED A QUESTION SCALED BY THE MOVE, WHICH GAVE THE
    // LONGEST MOVES THE MOST ROPE.** It was `reach_fit > 0.0`, and `reach_fit`
    // stays positive out to THREE TIMES reach ([`REACH_TOLERANCE`]) — so a 40px
    // jab was admitted at 119px and a 200px sword at 600px, each scoring low
    // and each still winning whenever the rest of the kit scored lower. The
    // admission question is now absolute and the scoring question is left
    // alone: see [`ADMISSION_SLACK_PX`].
    // ⛔⛤ **AND IT ASKED ITS QUESTION OF A WORLD THAT HAD ALREADY MOVED ON.**
    // Admission is the one judgement here about a moment in the FUTURE — the
    // tick this move's hitbox opens — and it was being made against the
    // opponent's position in a view that is `reaction_ms` old, plus the whole
    // of the move's startup still to come. At rung 5 that is 300ms of
    // staleness and up to 180ms of windup: about 100px of walking, against an
    // `ADMISSION_SLACK_PX` of 24.
    //
    // ⭐ MEASURED, and the number is unambiguous because it contradicts the
    // rule's own ceiling: a `medic` mirror started `medic_tourniquet` — 80px
    // of reach, so admitted only out to 104px — at a real gap of **153.6px**,
    // 177 times in 3600 ticks, with `LandedBodyHit` at ZERO for the bout. A
    // move cannot be admitted past its ceiling; what was admitted was a
    // remembered opponent.
    //
    // ⇒ **LEAD THE AIM.** Carry the foe forward at the relative velocity the
    // view reports, over the time between the world the brain SAW and the
    // tick the hitbox OPENS. Scoring is deliberately left on the observed
    // position: `reach_fit` is a judgement about VALUE and the difficulty
    // ladder is built on the brain being late. This says only that a swing is
    // aimed where somebody is going, which is what a person does and what the
    // 24px constant was a stand-in for.
    // ⛔⛤ **AND IT LEADS BY WHEN THE THREAT GOES LIVE, NOT BY `startup_s` —
    // REVIEWED 2026-09-20.** `startup_s` is *"time until the first Active
    // window"* and falls back to the WHOLE MOVE DURATION when there is none,
    // which is the shape of every projectile move in the game. Leading by it
    // aims past the opponent on exactly the moves thrown at somebody who is
    // moving: `polygon_projectile_charge_shot` fires at 0.26s and reports
    // `startup_s` 0.58s, so at 200px/s of closing speed the aim is 64px long —
    // wider than `ADMISSION_SLACK_PX`. `MoveFrameData::threat_live_at_s` asks
    // the mechanic instead. Equal to `startup_s` for an ordinary strike.
    let threat_at = |frames: &ambition_entity_catalog::MoveFrameData| {
        frames.threat_live_at_s.unwrap_or(frames.startup_s)
    };
    let lead_of = |startup_s: f32| {
        let dt = view.staleness_s() + startup_s;
        if dt <= 0.0 {
            return foe_local;
        }
        let rel = foe.vel - me.vel;
        let facing = if me.facing < 0.0 { -1.0 } else { 1.0 };
        let rel_local = (rel.dot(basis.side) * facing, rel.dot(basis.down));
        (
            foe_local.0 + rel_local.0 * dt,
            foe_local.1 + rel_local.1 * dt,
        )
    };
    // ⛔⛤ **A HAZARD IS NOT A THREAT WHERE IT IS THROWN, AND THE BRAIN AIMED
    // AS IF IT WERE.** `threat_at` says when the bolt LEAVES; a bolt that
    // leaves is still seconds from arriving. Measured on
    // `director_train_of_thought`, the roster's one steered bolt: it crosses
    // 671px at 300px/s, so against somebody 400px away the shot lands about
    // 1.3s after the throw and the aim was short by the whole of the
    // opponent's walk. Replacing the old over-lead (`startup_s`, i.e. the
    // whole move) with an under-lead (zero flight) was an improvement and not
    // the answer; this is the answer.
    //
    // ⚠ **ONE FIXED-POINT PASS, STATED RATHER THAN ITERATED.** The flight
    // time depends on the gap at arrival, which depends on the flight time. A
    // single pass — measure the gap at the throw, fly for that long — is
    // exact for a stationary opponent and errs toward UNDER-leading a
    // retreating one, which is the direction that refuses a shot rather than
    // throwing one that cannot land.
    //
    // ⚠ AND THE FLIGHT IS CAPPED BY THE HAZARD'S OWN REACH. A hazard cannot
    // fly further than it reaches, so `gap` past that would credit a shot
    // with travel it never makes — and the admission test below is about to
    // refuse that gap anyway.
    let arrival_of = |frames: &ambition_entity_catalog::MoveFrameData,
                      hazard: ambition_entity_catalog::MoveHazard| {
        let thrown_at = threat_at(frames);
        let speed = hazard.speed();
        if speed <= 0.0 {
            return thrown_at;
        }
        let at_throw = lead_of(thrown_at);
        let gap = (at_throw.0 * at_throw.0 + at_throw.1 * at_throw.1).sqrt();
        thrown_at + gap.min(hazard.reach()) / speed
    };
    attacks.retain(|attack| match (&attack.frames.coverage, &attack.frames.push_coverage) {
        // Hits somewhere: the hit is the question, and the shove it may also
        // carry is not a reason to swing at nobody.
        (Some(coverage), _) => {
            // ⚠ ALREADY INFLATED BY THEIR HURTBOX, so this is *"the move's
            // region touches them"* and not *"the move's region reaches their
            // centre"*.
            //
            // ⛔ **AND IT ASKED ONLY THE FAR SIDE.** An authored strike is a
            // box hung OUT from the body — `pointed_polygon`'s thrust spans
            // x ∈ [20, 76] — so `gap <= far` admitted it against somebody
            // standing in the hole in the middle of it. Both sides now, with
            // the same slack on each: the near side is forgiven by a decision
            // interval of walking for exactly the reason the far side is.
            //
            // ⚠ **IT REFUSES NOTHING ON TODAY'S ROSTER AND THAT IS MEASURED,
            // NOT ASSUMED.** `probe_how_far_out_an_authored_region_begins`:
            // the deepest authored region begins at **24.0px**, which is
            // `ADMISSION_SLACK_PX` exactly, before the foe's half-extent is
            // added on top. The whole grid came back bit-identical in all 21
            // rows. ⇒ This is the code agreeing with its own specification
            // and nothing more; it did NOT fix the fighter that prompted it,
            // whose hitboxes find no body at all — see the BRAIN row.
            let aim = lead_of(threat_at(&attack.frames));
            let Some((near, far)) = coverage.span_toward(aim, foe_extent) else {
                return false;
            };
            let gap = (aim.0 * aim.0 + aim.1 * aim.1).sqrt();
            gap >= near - ADMISSION_SLACK_PX && gap <= far + ADMISSION_SLACK_PX
        }
        // Only shoves: offered exactly where the shove lands.
        (None, Some(push)) => {
            coverage_fit(Some(push), lead_of(threat_at(&attack.frames)), foe_extent) > 0.0
        }
        // ⛔⛤ **TOUCHES NOTHING — AND THAT WAS FOUR DIFFERENT MOVES WEARING
        // ONE ANSWER.** This arm admitted every move that lands no volume and
        // shoves nobody, on the reasoning that a buff or a summon has no reach
        // question to fail. It has no reach question to PASS either: with no
        // `reach_fit`, no `expected_payoff` and no `kill_potential`, such a
        // move scores `frame_advantage` minus `stage_risk` — so the moment the
        // admission rule above started refusing swings that cannot land, the
        // fast safe move that does nothing became what a fighter pressed
        // whenever the opponent was out of range.
        //
        // ⭐ MEASURED, AND THE POPULATION IS SMALL ENOUGH TO NAME.
        // `authored_movesets::offer_census`, 2026-09-20: **123 of 470**
        // authored moves land here, and all but twelve are taunts, throws and
        // pummels — legal only while a capture is held, so they reach this
        // menu never. Of the twelve: five are `smash_counter::counter_move`,
        // three are `smash_vitality` buffs, three carry the body (two
        // teleports and the admiral's ridable shark — all three now on the
        // MOTION list, see the tail of this arm), and one is the
        // Performer's flyline. ⇒ The question is not
        // *"does it hit"*, it is **does it offer the opponent anything at
        // all**, and only two shapes of that answer are yes.
        //
        // ⛔ THE ROAD THAT NEARLY TOOK A WHOLE FIGHTER OFF THE MENU was the
        // one that carries no effect key: an ordinary ranged move pulls the
        // BODY's trigger through `MoveEventKind::Ranged`, and both of
        // projectile_polygon's neutral options — her boomerang and her charge
        // shot — author no Active volume at all, because *"the projectile IS
        // the damage, as it is for every ranged move"*. A rule built from the
        // effect-key table alone would have deleted her entire game. The
        // catalog answers them `RANGED_ACTION_REACH`.
        //
        // ⚠ A COUNTER AND A BUFF ARE NOT DELETED FROM THE GAME, they are off
        // the ATTACK ranking — which is the only list that was ever choosing
        // them, and it chose them by having nothing else left. Pricing them
        // needs a defensive feature (*"is the opponent committed to a
        // swing"*), and inventing one to keep them on a list they never won on
        // merit would be the wrong order.
        (None, None) => {
            // Reaches through something it puts in the world: a bolt, a
            // bomb, the body's own shot. Priced against its own reach, the
            // same absolute question the hit arm asks.
            if let Some(hazard) = attack.frames.hazard {
                let aim = lead_of(arrival_of(&attack.frames, hazard));
                let gap = (aim.0 * aim.0 + aim.1 * aim.1).sqrt();
                return gap <= hazard.reach() + ADMISSION_SLACK_PX;
            }
            // ⛔⛤ **AND A ROUTE THAT CARRIES THE BODY IS NOT AN ATTACK,
            // THOUGH THIS ARM ADMITTED ONE FOR A WHILE.** A teleport lands no
            // volume and crosses 210px; the admiral's shark is a ridable
            // summon with 650px of authority. Neither authors a `lift_*`
            // burst, so `motion_of` answered `(0, 0)` and they were on NO
            // list — and the reflex is to put them here, because here is a
            // list that exists.
            //
            // ⭐ MEASURED, 21 mirror matches of 3600 ticks, 2026-09-20. It
            // cost `player_robot_v3` the entire match: `phase_shift×157` —
            // one teleport every 23 ticks, which is its whole duration — at a
            // mean gap of **223px**, for **0% damage**, against 103% with an
            // ordinary menu. A move on the attack list is PRESSED, a pressed
            // move owns the body through its startup and recovery, and a body
            // in a move does not walk. So admitting a travel move as an
            // attack does not give the fighter a way to close: it takes away
            // the only one it had. `pointed_polygon` and `medic` failed the
            // same way, all three at 0%.
            //
            // ⛔ **AND FOLDING THE SUMMON INTO `hazard_reach` SO IT LANDED ON
            // THE ARM ABOVE WAS THE SAME MISTAKE WEARING THE OTHER HAT.** The
            // argument was that a summon holding ground for `seconds` makes an
            // opponent the same offer a bolt does. `call_the_shark`'s own
            // authoring refutes it — *"there is no hurtbox on this up-b, it's
            // purely a mobility special"*, the summoned body is `Neutral` and
            // deals no contact damage, and the `reach` is authored as half the
            // RIDE's straight-line distance. A number describing how far I can
            // GO is not a number describing how far I can HURT.
            //
            // ⇒ Both shapes belong to [`motion_options`], and they are there
            // now — the SUMMON is, at least: [`travel_of`] reads the ride's
            // own reach, and the score is a ratio against the gap rather than
            // a share of the kit's fastest SPEED, which is what made a
            // distance and a velocity incomparable and held this slice.
            //
            // ⛔ THE TELEPORT IS STILL ON NEITHER LIST, and that is now a
            // MEASURED refusal rather than an unfinished one: it goes where the
            // move AIMS, which for an up-special is straight up, so pricing it
            // as travel toward the opponent cost `player_robot_v3` his match at
            // two different prices. See [`travel_of`].
            //
            // What is left on this arm is what genuinely offers nobody
            // anything: counters, buffs, and the moves that are legal only
            // inside a hold.
            false
        }
    });

    // ⛔⛤ **AND IT DOES NOT COME BACK AS A LAST RESORT, WHICH IS WHAT THE
    // FIRST REPAIR DID.** That version re-admitted the hitless launcher
    // whenever `attacks` came out empty, on the reasoning that the alternative
    // was *"a body that cannot act at all"*. It is not: movement and attack are
    // chosen separately, so an empty attack menu at long range means WALK
    // TOWARD THEM, which is usually the right answer and always a cheaper
    // mistake than arming a vertical launch.
    //
    // ⇒ The move is not gone, it is FILED CORRECTLY: `motion_options` below
    // offers it as what it is, scored by whether its displacement goes where
    // this body is trying to go. That is what separates the two fighters who
    // own one — grid sweep, 2026-09-19, 19 of 21 bouts bit-identical either
    // way:
    //
    // ```text
    // medic  rescue_lift x48/91   gap 91   39%   → tourniquet x40, gap 63, 262%
    // emmy   smash_fwd   x52/95   gap 82  247%   → smash_fwd  x65, gap 123,  67%
    // ```
    //
    // The medic's lift was a TRAP: she had `tourniquet` at 90px and threw the
    // lift anyway, widening the gap so the next decision met the same world.
    // Emmy's was her APPROACH — so without it she ends up at a range her smash
    // cannot reach. An alignment score tells those apart by reading where the
    // opponent is, which is the fact that actually differs.
    //
    // ⭐⭐ **AND THE SWEEP SAYS SO: EXACTLY ONE OF 21 BOUTS MOVES, AND IT IS
    // EMMY.** Grid sweep, 2026-09-20, mirror matches, 3600 ticks, the other
    // twenty rows bit-identical to the printed digit:
    //
    // ```text
    //                took0% took1% hitstun moves used  gap  most thrown
    //   last resort    29%    29%      64    146   8   389  invariant_field x86
    //   motion list    73%   110%     307     89  12    88  smash_forward   x49
    // ```
    //
    // ⇒ She stops standing 389px away spamming the one move that reaches from
    // there and starts closing to 88 and swinging, and her situation mix goes
    // from `Disadvantage 51%` to `Advantage 83%`. That is the whole behavioural
    // delta of this change, which is what a one-row sweep is for.

    // Ties break on the move id, so the best option is a function of the world and
    // not of the kit's declaration order (ADR 0023: no order-dependent decisions).
    attacks.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.move_id.cmp(&b.move_id))
    });

    let motions = motion_options(kit, foe_local, basis);

    OptionSet {
        movement,
        attacks,
        motions,
    }
}

/// Every kit move whose only effect is to move this body, scored by whether
/// that motion goes where the body is trying to go.
///
/// ⛔⛤ **THE OBJECTIVE IS THE FOE, AND THAT IS WHAT MAKES THIS ONE RULE RATHER
/// THAN A TABLE OF CHARACTER CASES.** The medic's vertical launch and Emmy's
/// are the same move shape doing opposite jobs — measured on the grid sweep,
/// 2026-09-19: the medic threw hers 48 times in 91 starts for 39% damage while
/// `medic_tourniquet` sat unused at 90px, and removing Emmy's cost her 180
/// points of damage and grew her gap from 82 to 123. What separates them is
/// not the move, it is WHERE THE OPPONENT IS: the medic's foe is level with
/// her, so a launch straight up carries her away from the only thing she wants
/// to reach; Emmy's is above and across, so the same launch is her approach.
/// Asking for the alignment answers both without naming either.
///
/// ⛔⛤ **AND IT IS SCORED IN PIXELS AGAINST THE GAP, NOT AS A SHARE OF THE
/// KIT'S FASTEST MOTION — REVIEWED 2026-09-20.** The share was chosen so that
/// *"the weights stay comparable across bodies whose numbers are on different
/// scales"*, and it bought that by throwing away the only thing that makes a
/// displacement useful or useless. `alignment × speed/fastest` has no length in
/// it, so the gap magnitude cancels: a full-strength recovery scored exactly the
/// same against an opponent 5px away as against one 280px away, and
/// `medic_rescue_lift` — which applies about `(34, -905)` — was worth the same
/// whether it arrived or flew past.
///
/// ⭐ **THE SCORE IS A SYMMETRIC TENT OVER `travelled / gap`**, peaking at 1
/// where the motion arrives and reaching 0 at nothing and at twice the gap
/// alike. Comparability across bodies is preserved by the RATIO, which is what
/// the share was reaching for — a ratio of two lengths is dimensionless
/// whatever scale the body is on. See [`travel_of`] for where the distance
/// comes from.
fn motion_options(
    kit: &[AttackCandidate],
    foe_local: (f32, f32),
    basis: ae::AccelerationFrame,
) -> Vec<MotionOption> {
    let _ = basis;
    // ⛔ THE SAME THREE-WAY SPLIT THE ATTACK RETAIN MAKES, read from the other
    // side: a move that hits is an attack, a move that shoves is a shove, and
    // what is left over is only motion. A move that touches nothing and moves
    // nothing is a buff and belongs to neither list.
    let is_pure_motion = |c: &AttackCandidate| {
        c.legality == ActionLegality::Now
            && c.frames.coverage.is_none()
            && c.frames.push_coverage.is_none()
            && travel_of(&c.frames).is_some()
    };
    // ⚠ NOTHING TO GO TOWARD IS NOT "ANYWHERE IS FINE". With the foe on top of
    // this body there is no direction that serves the objective, and this is
    // exactly the state a body that has just launched itself is NOT in — so
    // scoring every motion 1 here would re-open the loop this list exists to
    // close.
    let gap = (foe_local.0 * foe_local.0 + foe_local.1 * foe_local.1).sqrt();
    if gap <= 0.0 {
        return Vec::new();
    }
    let mut motions: Vec<MotionOption> = kit
        .iter()
        .filter(|c| is_pure_motion(c))
        .filter_map(|c| {
            let travelled = match travel_of(&c.frames)? {
                Travel::Thrown { dir, distance } => {
                    let alignment =
                        ((dir.0 * foe_local.0 + dir.1 * foe_local.1) / gap).clamp(0.0, 1.0);
                    alignment * distance
                }
                // ⭐ A STEERED RIDE GOES WHERE IT IS POINTED, so there is no
                // alignment to lose: the whole distance counts toward whatever
                // the objective is. That is the mechanic — the admiral's shark
                // is flown with the control stick — and it is why a vehicle is
                // worth more than a burst of the same length.
                Travel::Steered { distance } => distance,
            };
            let fraction = travelled / gap;
            // ⛔⛤ **SYMMETRIC, AND THE ASYMMETRIC VERSION WAS MEASURED AND
            // REJECTED THE SAME DAY.** The first tent forgave overshoot at half
            // the rate it punished falling short, reasoning that *"arriving
            // long is a fighter who is now past the opponent, which in a
            // platform game is a position rather than a loss"*. That is a
            // statement about recovering to the STAGE, and this score's
            // objective is the FOE. Overshooting them does not close the gap —
            // it puts the same gap on the other side, so the next decision
            // meets the same world and presses again.
            //
            // ⭐ MEASURED on the grid sweep: at the forgiving falloff
            // `player_robot_v3` threw `phase_shift` **186 times** at a mean
            // gap of 129px for **27%/22%** on 9 distinct moves — 210px of
            // teleport into a 129px gap is 1.6×, which the forgiving rule
            // scored 0.69 and the symmetric one scores 0.37. That is the same
            // limit cycle the move caused when it was on the ATTACK list, and
            // it found the other list.
            //
            // ⇒ Worth nothing at twice the gap in either direction.
            let score = (1.0 - (fraction - 1.0).abs()).max(0.0);
            Some(MotionOption {
                move_id: c.move_id.clone(),
                frames: c.frames.clone(),
                binding: c.binding,
                score,
            })
        })
        .collect();
    motions.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.move_id.cmp(&b.move_id))
    });
    motions
}

/// What a motion move actually MOVES this body — how far, and whether the
/// direction is the move's or the rider's.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Travel {
    /// A commanded velocity, in the direction the move throws, carried for as
    /// long as the move owns the body.
    Thrown { dir: (f32, f32), distance: f32 },
    /// A vehicle the rider STEERS while it carries them, so only the distance
    /// is fixed and the direction is whatever is asked for.
    Steered { distance: f32 },
}

/// The travel a move offers its owner, or `None` when it offers none.
///
/// ⛔⛤ **A MOVE CAN CARRY ITS OWNER WITHOUT COMMANDING A VELOCITY, AND READING
/// ONLY THE IMPULSE HID THE WHOLE CLASS.** `call_the_shark` commands no impulse
/// at all — the climb is the player's to steer — so `motion_of` answers
/// `(0, 0)` and the admiral's only recovery fell off this list. It had been
/// patched onto the ATTACK list instead, as 650px of `hazard_reach`, which is a
/// different thing entirely: its authoring says *"there is no hurtbox on this
/// up-b, it's purely a mobility special"* and its `reach` is authored as half
/// the RIDE's straight-line distance. ⇒ Recovery authority is read here, where
/// travel is what is being priced, and the ride's own `reach` is the number it
/// already published for exactly this question.
///
/// ⚠ **`total_s` IS AN UPPER BOUND ON A THROWN DISTANCE, AND IS USED BECAUSE IT
/// IS THE NON-ARBITRARY WINDOW.** A thrown body keeps travelling after the move
/// ends and is slowed by gravity and drag while it does, and neither is
/// knowable here — [`ae::AccelerationFrame`] carries the two axes' DIRECTIONS
/// and no magnitude. What the move does own is how long it owns the body, and
/// past that edge the brain is deciding again anyway.
fn travel_of(frames: &ambition_entity_catalog::MoveFrameData) -> Option<Travel> {
    let (mx, my) = motion_of(frames);
    let speed = (mx * mx + my * my).sqrt();
    if speed > 0.0 {
        return Some(Travel::Thrown {
            dir: (mx / speed, my / speed),
            distance: speed * frames.total_s,
        });
    }
    // ⛔⛤ **A SUSTAINED RIDE ONLY, NOT EVERY ROUTE THAT PUBLISHES A `carry` —
    // AND THE TELEPORT WAS TRIED AND MEASURED OUT.** `RecoveryRoute::carry`
    // answers the RECOVERY planner's question, *"this gets you home from
    // within this far"*, and for a ride that is also the answer to *"how far
    // can I travel toward anything"*, because the rider flies it with the
    // control stick. For a teleport it is not: `phase_shift` is authored
    // *"aimed, like every recovery: the stick, then straight up"*, so pressing
    // it as an approach moves the robot 210px UP, and the gap it was pressed
    // to close is still there.
    //
    // ⭐ MEASURED on the grid sweep, both prices. Admitting a `Teleport` here
    // cost `player_robot_v3` his match either way: **27%/22% on 9 distinct
    // moves with `phase_shift×186`** under a forgiving overshoot rule, and
    // **32%/39% on 11 with `phase_shift×54`** under the symmetric one, against
    // **225%/223% on 19** with the move on no list at all. Two prices, one
    // outcome ⇒ the defect is not the price.
    //
    // ⚠ **WHAT IS OWED, so the next attempt starts here:** a teleport's
    // destination is a fact about the MOVE — `TeleportParams` carries
    // `behind_nearest_foe`, `behind_gap` and an aim that falls back to
    // straight up — and none of it reaches `MoveFrameData`. That is the same
    // gap the resolved-action-offer slice on the BRAIN row names: the brain
    // should be handed what pressing this move DOES, not a scalar published
    // for a different question.
    match frames.recovery_route {
        ambition_entity_catalog::RecoveryRoute::SustainedAuthority { reach, .. }
            if reach > 0.0 =>
        {
            Some(Travel::Steered { distance: reach })
        }
        _ => None,
    }
}

/// The body-local displacement a move COMMANDS of its owner, `+x` toward
/// facing and `+y` toward its feet.
///
/// ⭐ Assembled from the two authored halves rather than from one: `lift_speed`
/// is the against-gravity component (so it enters as `-y`) and `lift_side` is
/// the sideways one. A grapple line that hauls its owner mostly sideways and a
/// genre up-B are the same question asked of the same pair.
fn motion_of(frames: &MoveFrameData) -> (f32, f32) {
    (frames.lift_side, -frames.lift_speed)
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

/// What SHOVING `foe` is worth right now, normalized to `[0, 1]`.
///
/// A push is worth WHERE IT PUSHES SOMEBODY. The same gust is a spacing tool
/// at centre stage and a kill at the ledge, so the value is the shove's
/// geometric fit against how close the foe already is to a blast line — the
/// mirror of [`Features::stage_risk`], read on the opponent instead of on
/// oneself.
///
/// ⛔ ZERO FOR A MOVE THAT PUSHES NOTHING, which is most of a kit. This is a
/// feature about the push region specifically: [`coverage_fit`] asks the same
/// geometric question of the HITTABLE region and the two must not be summed
/// into one number, because that is the merge the coverage split undid.
///
/// ⛔⛤ **AND IT IS DIRECTIONAL, WHICH THE FIRST VERSION WAS NOT.** Coverage says
/// the push REACHES them and edge proximity says they are near going off;
/// neither says the push sends them THAT WAY. Wind blows one way — the gust's
/// `push_dir` is authored, not derived from geometry — so a fighter who has
/// crossed to the OUTBOARD side of a cornered opponent shoves them back toward
/// centre with the same coverage and the same edge proximity, and the first
/// feature paid full ledge value for a rescue.
///
/// ⛔⛤ **AND THE SECOND VERSION ASKED THE RIGHT QUESTION OF THE WRONG
/// GEOMETRY, TWICE.** It multiplied a generic `distance_to_edge` — the minimum
/// over all FOUR sides — by a left/right sign taken from WORLD `x`. So a foe
/// standing mid-stage under a low ceiling collected almost full side-shove ledge
/// pressure with both side blast lines a stage away; and the sign was computed
/// in world space while `push_dir` is body-local, which are the same frame only
/// while gravity points down. Arbitrary gravity is a shipped mechanic here, not
/// a hypothetical.
///
/// ⭐⭐ **ONE QUESTION REPLACES BOTH TERMS: how far can they still travel the
/// way this push sends them.** [`StageView::exit_distance_along`] answers it in
/// world space, so rotating the authored direction through the body's
/// acceleration frame is the whole of the frame handling — and left/right
/// shoves, up/down shoves and sideways gravity all fall out of it instead of
/// each needing a rule. A shove that sends them INBOARD has a long exit
/// distance and is worth nothing, which is the rescue case arriving for free
/// rather than as a separate sign.
pub fn displacement_value(
    push_coverage: Option<&ambition_entity_catalog::MoveCoverage>,
    push_dir: Option<(f32, f32)>,
    foe_local: (f32, f32),
    foe_extent: (f32, f32),
    push_world: PushGeometry<'_>,
) -> f32 {
    let fit = coverage_fit(push_coverage, foe_local, foe_extent);
    if fit <= 0.0 {
        return 0.0;
    }
    // A move that shoves but authors no direction is not read as shoving
    // NOWHERE — it is read as shoving forward, which is what an unauthored
    // launch resolves to everywhere else.
    let (local_x, local_y) = push_dir.unwrap_or((1.0, 0.0));
    fit * push_world.pressure(local_x, local_y)
}

/// The world geometry a body-local push direction has to be asked against.
///
/// ⭐ A PARAMETER OBJECT because the three values only mean anything together:
/// the frame that rotates a body-local direction into the world, the stage that
/// owns the blast lines, and the point being pushed. Passing them singly is how
/// the previous version ended up comparing a body-local `x` against a world one.
#[derive(Clone, Copy)]
pub struct PushGeometry<'a> {
    /// The PUSHER's acceleration frame — the authored direction is in the
    /// pusher's body-local axes, not the victim's.
    pub basis: ae::AccelerationFrame,
    /// `-1.0` when the pusher faces world-left, so `+x` local is `-side` world.
    pub facing: f32,
    pub stage: &'a crate::perception::StageView,
    /// Where the victim is standing.
    pub at: ae::Vec2,
}

impl PushGeometry<'_> {
    /// How close this victim is to leaving the stage along a body-local push.
    ///
    /// `1.0` at the blast line, falling to `0.0` at a stage half-span away —
    /// the same normalisation [`Features::stage_risk`] uses, so "near the edge"
    /// stays one fact read on two subjects.
    /// How much of the stage left in this direction a body travelling
    /// `distance` along it spends.
    ///
    /// ⭐ The self-facing half of [`Self::pressure`]: that one asks *how near
    /// the line are they*, this one asks *how much nearer does this carry me*.
    /// `1.0` when the travel reaches the blast line, `0.0` for a body that
    /// commits to no self-motion at all.
    pub fn travel_share(&self, local_x: f32, local_y: f32, distance: f32) -> f32 {
        if distance <= 0.0 {
            return 0.0;
        }
        let facing = if self.facing < 0.0 { -1.0 } else { 1.0 };
        let world = self.basis.side * (local_x * facing) + self.basis.down * local_y;
        if world.length_squared() <= 0.0 {
            return 0.0;
        }
        let exit = self.stage.exit_distance_along(self.at, world.normalize());
        if !exit.is_finite() || exit <= 0.0 {
            return 1.0;
        }
        (distance / exit).clamp(0.0, 1.0)
    }

    pub fn pressure(&self, local_x: f32, local_y: f32) -> f32 {
        let facing = if self.facing < 0.0 { -1.0 } else { 1.0 };
        let world = self.basis.side * (local_x * facing) + self.basis.down * local_y;
        if world.length_squared() <= 0.0 {
            return 0.0;
        }
        let half_stage = (self.stage.bounds.max - self.stage.bounds.min).length() * 0.5;
        if half_stage <= 0.0 {
            return 1.0;
        }
        let exit = self.stage.exit_distance_along(self.at, world.normalize());
        (1.0 - exit / half_stage).clamp(0.0, 1.0)
    }
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
    // ⛔⛔ A FOE-RELATIVE VERB WITH NO FOE IS AN OPTION THE EMITTER CANNOT ACT
    // ON. `apply_movement` builds Approach/Retreat/Dash from the direction to
    // `nearest_hostile()` and emits a ZERO stick when there is none — so the
    // brain chose "approach", pressed nothing, and the trace read
    // `chose=Some(Approach) emit_x=0.0` for as long as the body stood there.
    //
    // ⭐ THE ARGUMENT IS ALREADY IN THIS FILE, one verb over: Jump was offered
    // unconditionally until a body with an empty jump budget was "handed an
    // option pressing does nothing for", and the note there says why that is
    // worse than a wasted press — L3 rolls the verb, the line goes nowhere, and
    // *nowhere scores as safe*. Same rule, the verb it was not applied to.
    //
    // ⭐ MEASURED, seed 0 of `ladder_rig --sweep-below`, the l5 partner after its
    // opponent died: `offered=[Approach, Retreat, Jump] chose=Some(Approach)
    // emit_x=0.0`, unchanged for the rest of the bout.
    let has_foe = toward_foe != 0.0;
    let mut out = Vec::new();
    let mut push = |verb: MovementVerb, score: f32| {
        if !has_foe
            && matches!(
                verb,
                MovementVerb::Approach | MovementVerb::Retreat | MovementVerb::Dash
            )
        {
            return;
        }
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
