//! The Limit meter: what fills a fighter's meter, authored rather than assumed.
//!
//! ⭐⭐ JON, 2026-09-05, AND THE CONSTRAINT MATTERS MORE THAN THE NUMBERS:
//! *"It will depend on the mechanic. There could be a cloud like meter, where a
//! move fills it. Or a damage only meter, or whatever, **make sure the meter
//! doesn't push future uses of it into a box.**"* ⇒ So this is not a fill RULE.
//! It is a set of independent SOURCES, every one of which may be zero, and a
//! mechanic that wants only one of them authors only that one.
//!
//! The four obvious ways a meter fills, and all four are expressible here:
//!
//! | want | author |
//! |---|---|
//! | a slow inevitability | `per_second` alone |
//! | a reward for offence | `on_damage_dealt` / `per_damage_dealt` |
//! | a comeback mechanic | `on_damage_taken` / `per_damage_taken` |
//! | *"a cloud like meter, where a move fills it"* | `smash.fill_meter`, an ordinary technique any move may emit |
//!
//! ⛔ THE METER ITSELF IS NOT NEW AND MUST NOT BE. `ambition_platformer2d_core`'s
//! `BodyMana` is a per-body `ResourceMeter`, already rollback-canonical as
//! `body.mana`, already published to presentation through `sim_view::facts`, and
//! already the thing `MoveGates::meter_cost` spends. ⇒ What was missing was
//! anything that PUT SOMETHING IN IT — measured 2026-09-05, every `BodyMana` in
//! the workspace was `ResourceMeter::new(100.0, 0.0, 0.0)`, regen zero, refilled
//! only by a shrine no smash stage has.
//!
//! ⚠ AND A FULL METER IS NOT A NEW GATE EITHER. *"Give whoever gets the limit
//! meter some move they can use when it fills"* is `meter_cost == cap`: a move
//! that costs the whole meter is available exactly when the meter is full, and
//! `afford_meter` already refuses it otherwise. Nothing new decides that.

use serde::{Deserialize, Serialize};

/// The authored effect key for a move that fills its own owner's meter — the
/// *"cloud like meter"* case, expressed as an ordinary technique so a move can
/// charge the thing it later spends.
pub const FILL_METER: &str = "smash.fill_meter";

/// Authored parameters of one meter fill.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FillMeterParams {
    /// How much to add. Clamped at the meter's cap by `ResourceMeter::refill`.
    pub amount: f32,
}

/// What fills a fighter's Limit meter, per second and per hit.
///
/// ⛔⛔ **THIS IS NOT THE ONLY THING THAT FILLS `BodyMana`, AND THAT IS AN OPEN
/// DEFECT.** `ambition_platformer2d_actor_monolith::avatar::regen_player_mana`
/// refills every DRIVEN body at **14.0 per second**, unconditionally, from the
/// monolith's `FeatureCollection` phase — a platformer rule that exists so mana
/// is "a genuine spendable resource" for charge attacks. A composition carrying
/// both gets both, so a fighter accrues `14.0 + per_second` rather than
/// `per_second`. ⇒ Jon's 60-point baseline, authored to take **120 s** of clock,
/// reaches its cap in about **4.1 s**, and a locally-driven fighter has a
/// different Limit economy from an otherwise identical undriven one.
///
/// ⚠ THE TESTS IN `limit/tests.rs` CANNOT SEE IT: they install the Limit systems
/// directly and never compose the monolith's feature plugin, so the 14/s
/// producer does not exist in that world. A guard whose world lacks the thing it
/// is guarding against.
///
/// ⇒ The fix is not a gate on `DrivingParticipant` — that preserves the leak for
/// driven bodies and merely hides the asymmetry. **A meter's fill policy belongs
/// to the RULESET**, and two rulesets currently both write this one.
///
/// ⭐ EVERY FIELD DEFAULTS TO ZERO AND THAT IS THE POINT: a mechanic authors the
/// sources it wants and gets nothing it did not ask for. A damage-only meter is
/// this struct with `per_second: 0.0`; a pure clock is the reverse.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LimitMeterFill {
    /// The meter's ceiling.
    pub cap: f32,
    /// Added every second, regardless of what anybody does.
    pub per_second: f32,
    /// Added once per damage INSTANCE this fighter dealt, whatever its size.
    pub on_damage_dealt: f32,
    /// Added per point of damage dealt.
    pub per_damage_dealt: f32,
    /// Added once per damage instance this fighter TOOK.
    pub on_damage_taken: f32,
    /// Added per point of damage taken.
    pub per_damage_taken: f32,
    /// SUBTRACTED every second — the Limit that must be spent rather than banked.
    ///
    /// ⭐ THE TIMEOUT LEVER, and the fifth independent source. A meter that only
    /// ever rises rewards holding it: the correct play is to charge and wait. A
    /// decay makes the charge a WINDOW, which is the other obvious shape a Limit
    /// takes and one Jon's *"the meter doesn't push future uses of it into a
    /// box"* asks the baseline to leave available.
    ///
    /// ⛔⛔ IT LIVES HERE AND NOT ON `ResourceMeter::decay_rate`, which is inert
    /// for a body: `ResourceMeter` implements decay in `tick_decay`, and no
    /// system calls `tick_decay` on a `BodyMana`. ⇒ Setting `decay_rate` on a
    /// fighter authors a rule nothing reads.
    ///
    /// ⚠ **AND "NOTHING TICKS IT" IS NOT "NOTHING WRITES IT" — a distinction two
    /// of us got wrong in opposite directions on the same day.** `decay_rate` is
    /// applied only inside `ResourceMeter::tick`, and there are ZERO non-test
    /// callers of `.tick(` against a body's mana meter, so the sentence above
    /// stands and this rate belongs here. What DOES write a `BodyMana` is
    /// `avatar::regen_player_mana`, and it calls `meter.refill` directly — a
    /// competing FILL, not a competing decay. ⇒ It cost the authored Limit its
    /// meaning until `PlayerManaRegen` gave that rate an owner; see
    /// `LimitMeterFill`'s own doc. It does not bear on where decay lives.
    ///
    /// ⚠ ZERO IN JON'S BASELINE. He asked for four rising sources and no decay,
    /// so the shipped Limit still banks; this is the lever, not a change to what
    /// the demo does.
    #[serde(default)]
    pub decay_per_second: f32,
}

impl LimitMeterFill {
    /// Jon's baseline, verbatim from his own numbers (2026-09-05): *"the meter
    /// has a cap of 60, add 1 tick every 2 seconds, add 1 tick on each damage
    /// instance plus 0.1x the damage dealt, and add 2 ticks for each instance of
    /// damage taken plus 0.2x the damage taken."*
    ///
    /// ⚠ HE CALLED IT AN EXAMPLE — *"this is just an example, you can tweak
    /// things"* — so these are a demonstration that all four sources express, not
    /// a balance ruling. ⭐ Taking damage fills roughly twice as fast as dealing
    /// it, which is what makes it a comeback meter rather than a snowball.
    pub const JONS_BASELINE: Self = Self {
        cap: 60.0,
        // "1 tick every 2 seconds".
        per_second: 0.5,
        on_damage_dealt: 1.0,
        per_damage_dealt: 0.1,
        on_damage_taken: 2.0,
        per_damage_taken: 0.2,
        // ⚠ HE ASKED FOR NO DECAY. The lever exists; the baseline does not pull
        // it, so the goblin's charge keeps until it is spent.
        decay_per_second: 0.0,
    };

    /// What one damage instance contributes to the fighter who DEALT it.
    pub fn dealt(&self, damage: i32) -> f32 {
        self.on_damage_dealt + self.per_damage_dealt * damage.max(0) as f32
    }

    /// What one damage instance contributes to the fighter who TOOK it.
    pub fn taken(&self, damage: i32) -> f32 {
        self.on_damage_taken + self.per_damage_taken * damage.max(0) as f32
    }

    /// Everything wrong with this fill, as sentences an author can act on.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.cap <= 0.0 {
            problems.push(format!(
                "cap {} gives a meter nothing can ever fill",
                self.cap
            ));
        }
        // ⛔ NOT "at least one source must be non-zero" — a meter filled ONLY by
        // an authored `smash.fill_meter` move is exactly the "cloud like meter"
        // Jon named, and every field here is legitimately zero for it.
        for (name, v) in [
            ("per_second", self.per_second),
            ("on_damage_dealt", self.on_damage_dealt),
            ("per_damage_dealt", self.per_damage_dealt),
            ("on_damage_taken", self.on_damage_taken),
            ("per_damage_taken", self.per_damage_taken),
            // ⛔ A NEGATIVE DECAY IS A FILL WEARING THE WRONG NAME, and it would
            // be the one source whose sign nobody reading the field name would
            // check. Same arm as the rest for that reason.
            ("decay_per_second", self.decay_per_second),
        ] {
            if v < 0.0 {
                problems.push(format!(
                    "{name} is {v}: a NEGATIVE rate here means the opposite of \
                     what the field is called. Drain is authored with \
                     `decay_per_second`, which is the only field that subtracts."
                ));
            }
        }
        // ⛔⛔ A DECAY THAT OUTRUNS EVERY SOURCE IS A METER NOBODY CAN FILL, and
        // it fails SILENTLY: the fighter charges, the number falls back, and the
        // priced move simply never becomes available. ⚠ Only decidable against
        // the CLOCK source — the damage sources depend on a match nobody can
        // predict — so this catches the unarguable case rather than every one.
        if self.decay_per_second > 0.0
            && self.per_second > 0.0
            && self.decay_per_second >= self.per_second
            && self.on_damage_dealt <= 0.0
            && self.per_damage_dealt <= 0.0
            && self.on_damage_taken <= 0.0
            && self.per_damage_taken <= 0.0
        {
            problems.push(format!(
                "decay_per_second {} is at least per_second {} and no damage \
                 source fills this meter, so it can never reach its cap",
                self.decay_per_second, self.per_second
            ));
        }
        problems
    }
}

impl Default for LimitMeterFill {
    /// ⛔ NOTHING FILLS ANYTHING. A game that does not declare a Limit gets a
    /// meter that never moves, which is what every match did before this existed.
    fn default() -> Self {
        Self {
            cap: 0.0,
            per_second: 0.0,
            on_damage_dealt: 0.0,
            per_damage_dealt: 0.0,
            on_damage_taken: 0.0,
            per_damage_taken: 0.0,
            decay_per_second: 0.0,
        }
    }
}
