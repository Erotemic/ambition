//! THE Ambition enemy roster — the named, authored enemy DATA.
//!
//! The machinery lib (`ambition_sandbox`) owns the generic spawn pipeline and
//! the `EnemyArchetypeSpec` schema, but the actual roster (which enemies exist,
//! their HP / speeds / brain template / weapons) is named game content and
//! lives here. The brain-keyed RON is installed into the lib's `EnemyRoster`
//! at content-plugin build time via [`install`], so every spawn resolves
//! against this table — never a lib-embedded default.
//!
//! Install happens at plugin **build** time (before any spawn system runs), so
//! the ordering is structural: resolution can never observe the lib's
//! standalone fallback in a content build.

use ambition_sandbox::features::{install_enemy_roster, EnemyRoster};

/// The authored enemy roster, embedded at compile time. Top-level keys are the
/// spawn brain keys a `LoadingZone` / encounter authors as `Brain::Custom("…")`;
/// `"combatant"` is the reserved fallback row.
pub const ENEMY_ROSTER_RON: &str = include_str!("../assets/data/enemy_archetypes.ron");

/// Install the Ambition enemy roster into the machinery lib. Called once from
/// [`crate::AmbitionContentPlugin`].
pub fn install() {
    install_enemy_roster(EnemyRoster::from_ron(ENEMY_ROSTER_RON));
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
        let _ = EnemyRoster::from_ron(ENEMY_ROSTER_RON);
    }

    /// Drift guard: the machinery lib keeps a `#[cfg(test)]` COPY of this RON
    /// so its own unit tests resolve enemies standalone (without this plugin).
    /// Content is the source of truth — fail loudly if the copy drifts, so a
    /// roster edit can't make the lib's tests assert stale data while
    /// production (this file) behaves differently.
    #[test]
    fn lib_test_fixture_matches_the_authored_roster() {
        let lib_fixture = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../ambition_sandbox/assets/data/enemy_archetypes.ron"
        ))
        .expect("read the lib's enemy_archetypes.ron test fixture");
        assert_eq!(
            lib_fixture, ENEMY_ROSTER_RON,
            "the lib's #[cfg(test)] enemy_archetypes.ron fixture has drifted from \
             the authored content roster — re-copy this file into \
             crates/ambition_sandbox/assets/data/enemy_archetypes.ron"
        );
    }
}
