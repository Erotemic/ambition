//! Which hits land HEAVY: a rule that each game states for its own rooms.
//!
//! A heavy hit and a light hit are the same strike at two strengths, and only
//! a game can say where the line is, because only the game knows what its
//! damage means. Ambition's damage is health, where 3 is a big hit. A platform
//! fighter's damage is percent, where 3 is a jab. So the engine has no
//! threshold: a room whose game states none has no heavy hits.
//!
//! Declared with [`DeclareRulesExt`](crate::scoped_rules::DeclareRulesExt) like
//! every other per-game rule, and projected each tick into
//! [`ResolvedCombatTuning::strike_weight`](crate::rules::ResolvedCombatTuning::strike_weight).
//!
//! ⛔ NOT A FIELD OF [`CombatRules`](crate::rules::CombatRules). A game that
//! declares no combat ruleset (every Ambition room) keeps the engine's
//! undeclared combat baseline and still decides which of its hits are heavy.

/// A game's line between its light hits and its heavy hits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrikeWeightRules {
    /// A hit that deals at least this much damage lands heavy. The damage is
    /// the game's own, after the charge and the staling of the hit.
    pub heavy_at_damage: i32,
}

impl StrikeWeightRules {
    /// The line at `damage`.
    pub const fn heavy_at(damage: i32) -> Self {
        Self {
            heavy_at_damage: damage,
        }
    }

    /// Does a hit dealing `damage` land heavy under `rules`? Always `false`
    /// where no game stated a line.
    pub fn is_heavy(rules: Option<Self>, damage: i32) -> bool {
        rules.is_some_and(|rules| damage >= rules.heavy_at_damage)
    }
}
