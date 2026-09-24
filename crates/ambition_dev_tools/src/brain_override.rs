//! A measurement knob: replace the brain every authored actor spawns with.
//!
//! All of `hall_of_characters`'s NPCs are authored
//! `brain_override: "stand_still"`, and `tick_simple_state_machine`, which
//! answers that arm, takes no `WorldView`. So the room measures the supply of
//! cognition but never its demand. Pointing this knob at a tactical preset
//! makes the hall a cognition benchmark (see
//! `bounded-perception-and-attention.md`) without a parallel room whose
//! geometry, factions and assets would differ.
//!
//! A forced run is not the shipped hall. Its numbers are marked on the census
//! row and in the ledger's comparability key, like [`crate::population_cap`].
//!
//! Qualify the preset with its provider. A preset name resolves against each
//! character's own provider, and the hall mixes providers, so a bare name
//! fails on the first character from another provider:
//!
//! ```text
//! NPC spawn `npc_snakes_on_a_cartesian_plane`: brain_override names unknown
//! brain preset `melee_brute_striker` (resolved to `mary_o::melee_brute_striker`)
//! ```
//!
//! Use `AMBITION_ACTOR_BRAIN_OVERRIDE=ambition::melee_brute_striker`. An
//! unknown preset panics at spawn instead of silently falling back.
//!
//! Not a gameplay feature. Read from the environment once; absent means the
//! author decides, at zero cost.

use ambition_characters::brain::AuthoredBrainOverride;

/// Force every authored actor's brain preset. Unset means the placement decides.
pub const BRAIN_OVERRIDE_ENV: &str = "AMBITION_ACTOR_BRAIN_OVERRIDE";

/// Force every authored actor's autonomous profile. Unset means the author decides.
///
/// Presets cannot reach the brains that perceive: every catalog preset lowers
/// to a `tick_simple_state_machine` arm, which takes no `WorldView`. `Fighter`
/// is reachable only through a character's autonomous profile, so measuring
/// perception demand needs this knob, not [`BRAIN_OVERRIDE_ENV`].
pub const BRAIN_PROFILE_ENV: &str = "AMBITION_ACTOR_BRAIN_PROFILE";

/// What the environment asks for, read once at plugin build.
///
/// This crate writes the session resource ([`AuthoredBrainOverride`]) and the
/// sim reads it. Nothing else reads the environment. A plugin builds once, so
/// the read cannot happen during placement and add to what it measures.
pub fn from_env() -> AuthoredBrainOverride {
    let read = |name: &str| {
        std::env::var(name)
            .ok()
            .map(|raw| raw.trim().to_owned())
            .filter(|raw| !raw.is_empty())
    };
    AuthoredBrainOverride {
        preset: read(BRAIN_OVERRIDE_ENV),
        profile: read(BRAIN_PROFILE_ENV),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Premise guard for every run without the knob: the default must be "the
    /// author decides".
    #[test]
    fn no_environment_variable_means_the_author_decides() {
        assert!(
            std::env::var(BRAIN_OVERRIDE_ENV).is_err()
                && std::env::var(BRAIN_PROFILE_ENV).is_err(),
            "this test asserts the DEFAULT; setting either variable would test \
             something else"
        );
        assert_eq!(from_env(), AuthoredBrainOverride::default());
    }

    /// `Default` and "no environment" are the same value, so a composition without
    /// developer tools and one with the knobs unset behave the same.
    #[test]
    fn the_default_override_forces_neither_preset_nor_profile() {
        let quiet = AuthoredBrainOverride::default();
        assert_eq!(quiet.preset(), None);
        assert_eq!(quiet.profile(), None);
    }
}
