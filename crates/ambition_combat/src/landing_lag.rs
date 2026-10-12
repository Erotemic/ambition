//! How much of a move's landing lag a game charges: a rule that each game
//! states for its own rooms.
//!
//! A move states the lag it owes when its owner touches down in it
//! (`MoveSpec::landing_lag_s`): a fact of the swing, written for a platform
//! fighter, where that cost is what makes an aerial a decision. A game whose
//! feel is not a platform fighter's (Ambition: a body is not to stand locked
//! after it lands) keeps the moves and says how much of that cost it charges.
//!
//! Declared with [`DeclareRulesExt`](crate::scoped_rules::DeclareRulesExt) like
//! every other per-game rule, and projected each tick into
//! [`ResolvedCombatTuning::landing_lag`](crate::rules::ResolvedCombatTuning::landing_lag).
//!
//! ⛔ NOT A FIELD OF [`CombatRules`](crate::rules::CombatRules), for the reason
//! [`crate::strike_weight`] gives: a game that declares no combat ruleset still
//! states this.

/// A game's share of each move's landing lag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LandingLagRules {
    /// What a move's authored landing lag is multiplied by. 1 is the move's
    /// own; 0 is none.
    pub scale: f32,
}

impl LandingLagRules {
    /// Each landing lag times `scale`.
    pub const fn scaled_by(scale: f32) -> Self {
        Self { scale }
    }

    /// The lag a landing charges under `rules` for a move that authors
    /// `seconds`. Where no game stated a share, the move's own.
    pub fn charged(rules: Option<Self>, seconds: f32) -> f32 {
        (seconds * rules.map_or(1.0, |rules| rules.scale.max(0.0))).max(0.0)
    }
}
