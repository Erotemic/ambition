//! Authored Limit-meter fill policy and the technique that fills the meter directly.
//! Each fill source is independent. The ruleset chooses which sources are active; the shared meter remains the canonical resource.

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

/// Independent sources that fill or drain a fighter's Limit meter.
/// All sources default to zero, so a ruleset opts into only the policies it needs.
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
    /// Amount added for each successfully blocked strike. This is per strike because a blocked hit carries no damage amount to scale by.
    pub on_block: f32,
    /// Amount subtracted per second. This policy lives here because `BodyMana` does not otherwise tick `ResourceMeter::decay_rate`.
    #[serde(default)]
    pub decay_per_second: f32,
}

impl LimitMeterFill {
    /// Baseline demo values. They exercise clock, dealt-damage, taken-damage, and block sources; decay is disabled.
    pub const JONS_BASELINE: Self = Self {
        cap: 60.0,
        // "1 tick every 2 seconds".
        per_second: 0.5,
        on_damage_dealt: 1.0,
        per_damage_dealt: 0.1,
        on_damage_taken: 2.0,
        per_damage_taken: 0.2,
        // Baseline block fill matches one landed-hit event. Zero disables this source.
        on_block: 1.0,
        // The baseline keeps earned meter until it is spent.
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

    /// Return the fill awarded for one blocked strike.
    pub fn blocked(&self) -> f32 {
        self.on_block
    }

    /// Return mechanical validation errors for this fill policy.
    /// This checks signs and impossible clock-only combinations. Relative source weights are ruleset balance policy, not type validity.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.cap <= 0.0 {
            problems.push(format!(
                "cap {} gives a meter nothing can ever fill",
                self.cap
            ));
        }
        // Relative source weights are balance policy. All automatic sources may be zero
        // when authored techniques provide the meter fill.
        for (name, v) in [
            ("per_second", self.per_second),
            ("on_damage_dealt", self.on_damage_dealt),
            ("per_damage_dealt", self.per_damage_dealt),
            ("on_damage_taken", self.on_damage_taken),
            ("per_damage_taken", self.per_damage_taken),
            // `decay_per_second` is a positive drain rate despite being checked with fill rates.
            ("decay_per_second", self.decay_per_second),
            // Keep every fill source in this sign check; this list is not compiler-exhaustive.
            ("on_block", self.on_block),
        ] {
            if v < 0.0 {
                problems.push(format!(
                    "{name} is {v}: a NEGATIVE rate here means the opposite of \
                     what the field is called. Drain is authored with \
                     `decay_per_second`, which is the only field that subtracts."
                ));
            }
        }
        // Reject only the statically provable no-fill case. Damage and block income
        // depend on match events and cannot be compared to decay here.
        if self.decay_per_second > 0.0
            && self.per_second > 0.0
            && self.decay_per_second >= self.per_second
            && self.on_damage_dealt <= 0.0
            && self.per_damage_dealt <= 0.0
            && self.on_damage_taken <= 0.0
            && self.per_damage_taken <= 0.0
            // Blocking is an independent source and prevents this static no-fill proof.
            && self.on_block <= 0.0
        {
            problems.push(format!(
                "decay_per_second {} is at least per_second {} and no damage or \
                 block source fills this meter, so it can never reach its cap",
                self.decay_per_second, self.per_second
            ));
        }
        problems
    }
}

impl Default for LimitMeterFill {
    /// Default to a meter with no automatic fill or decay.
    fn default() -> Self {
        Self {
            cap: 0.0,
            per_second: 0.0,
            on_damage_dealt: 0.0,
            per_damage_dealt: 0.0,
            on_damage_taken: 0.0,
            per_damage_taken: 0.0,
            on_block: 0.0,
            decay_per_second: 0.0,
        }
    }
}
