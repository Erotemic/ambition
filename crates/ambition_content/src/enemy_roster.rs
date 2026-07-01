//! THE Ambition enemy roster — the named, authored enemy DATA.
//!
//! The machinery lib (`ambition_gameplay_core`) owns the generic spawn pipeline and
//! the `CharacterArchetypeSpec` schema, but the actual roster (which enemies exist,
//! their HP / speeds / brain template / weapons) is named game content and
//! lives here. The brain-keyed RON is installed into the lib's `CharacterRoster`
//! at content-plugin build time via [`install`], so every spawn resolves
//! against this table — never a lib-embedded default.
//!
//! Install happens at plugin **build** time (before any spawn system runs), so
//! the ordering is structural: resolution can never observe the lib's
//! standalone fallback in a content build.

use ambition_gameplay_core::features::{install_enemy_roster, CharacterRoster};

/// The authored enemy roster, embedded at compile time. Top-level keys are the
/// spawn brain keys a `LoadingZone` / encounter authors as `Brain::Custom("…")`;
/// `"combatant"` is the reserved fallback row.
pub const CHARACTER_ROSTER_RON: &str = include_str!("../assets/data/character_archetypes.ron");

/// Install the Ambition enemy roster into the machinery lib. Called once from
/// [`crate::AmbitionContentPlugin`].
pub fn install() {
    install_enemy_roster(CharacterRoster::from_ron(CHARACTER_ROSTER_RON));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The embedded roster parses and carries the reserved `combatant`
    /// fallback row (the `from_ron` invariant). A parse error or a missing
    /// fallback would panic here rather than at first spawn.
    #[test]
    fn embedded_roster_parses_and_installs() {
        // `from_ron` panics on a parse error or a missing `combatant` row.
        let _ = CharacterRoster::from_ron(CHARACTER_ROSTER_RON);
    }

    /// A practice-target ("sandbag") archetype is a PASSIVE dummy — it must not
    /// carry a melee attack (it never strikes back). Pins the authored roster
    /// against re-adding a counter-attack.
    #[test]
    fn sandbag_archetypes_are_passive() {
        let roster = CharacterRoster::from_ron(CHARACTER_ROSTER_RON);
        assert!(
            roster.sandbags_are_passive(),
            "a sandbag/training-dummy archetype carries a melee attack — passive \
             targets must have `melee: None`"
        );
    }
}
