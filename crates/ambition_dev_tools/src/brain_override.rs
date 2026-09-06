//! A measurement knob: replace the brain every authored actor spawns with.
//!
//! ⭐ THIS EXISTS BECAUSE THE HALL CANNOT ANSWER THE QUESTION IT IS USED FOR.
//! All 129 of `hall_of_characters`'s NPCs are authored
//! `brain_override: "stand_still"`, and `tick_simple_state_machine` — which
//! answers that arm — takes NO `WorldView` argument at all. So every peer list,
//! world view and memory update the decision pipeline builds for that room is
//! supplied to a brain that by construction cannot read it. The room measures
//! the SUPPLY of cognition and can never measure its DEMAND.
//!
//! Pointing this knob at a tactical preset turns the one room with a real cast
//! into the benchmark `bounded-perception-and-attention.md` asks for, without
//! authoring a parallel room whose geometry, factions and assets would differ
//! from the hall's and make the two incomparable.
//!
//! ⛔⛔ IT CHANGES THE ROOM, AND THAT IS THE POINT. A forced run is not the
//! shipped hall: the cast moves, fights, and takes paths the authored room never
//! sees. No number taken under it describes the shipped hall, so it is reported
//! on the census row and in the ledger's comparability key, beside
//! [`crate::population_cap`], which exists for the same reason.
//!
//! ⛔⛔ NAME THE PROVIDER. A preset name resolves against EACH CHARACTER'S OWN
//! provider, and the hall is a cross-provider gallery, so a bare
//! `melee_brute_striker` dies on the first `mary_o` character:
//!
//! ```text
//! NPC spawn `npc_snakes_on_a_cartesian_plane`: brain_override names unknown
//! brain preset `melee_brute_striker` (resolved to `mary_o::melee_brute_striker`)
//! ```
//!
//! Pass it qualified — `AMBITION_ACTOR_BRAIN_OVERRIDE=ambition::melee_brute_striker`.
//! A preset no provider registers panics at spawn rather than falling back, which
//! is the honest failure: a silent fallback would measure a cast nobody chose.
//!
//! ⚠ NOT A GAMEPLAY FEATURE. Read from the environment exactly once; absent
//! means "author decides" at zero cost. A game that wants a different cast
//! authors one.

use ambition_characters::brain::AuthoredBrainOverride;

/// Force every authored actor's brain preset. Unset means the placement decides.
pub const BRAIN_OVERRIDE_ENV: &str = "AMBITION_ACTOR_BRAIN_OVERRIDE";

/// Force every authored actor's autonomous PROFILE. Unset means the author decides.
///
/// ⭐ THE PRESET ROAD CANNOT REACH THE BRAINS THAT PERCEIVE. Every preset the
/// catalog names lowers to an arm `tick_simple_state_machine` answers, and that
/// function takes no `WorldView`. `Fighter` — one of the only two brains that
/// consume one — is reachable ONLY through a character's autonomous profile, so
/// pricing perception demand needs this knob and not [`BRAIN_OVERRIDE_ENV`].
pub const BRAIN_PROFILE_ENV: &str = "AMBITION_ACTOR_BRAIN_PROFILE";

/// What the environment asks for, read ONCE at plugin build.
///
/// ⭐⭐ THIS IS THE WHOLE OF THE DEV CRATE'S JOB NOW. It used to be two
/// `OnceLock`s and two public readers, and the ACTOR KERNEL called them while
/// building a live brain — the simulation reading developer state to decide what
/// the world contains. The value is a session resource
/// ([`AuthoredBrainOverride`]) that this crate WRITES and the sim reads; nothing
/// outside this function touches the environment.
///
/// ⛔ READ ONCE, and now structurally rather than by a lock: a plugin builds
/// once, so `std::env::var` cannot land on a placement and make the knob's own
/// cost part of what it is measuring.
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

    /// ⛔ THE PREMISE GUARD FOR EVERY RUN TAKEN WITHOUT THE KNOB. If the default
    /// were anything but "the author decides", every measurement in the journal
    /// would describe a cast nobody authored.
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

    /// ⭐ AND THE DEFAULT IS READABLE AS "NOBODY IS STEERING", which is the
    /// question every consumer asks of it. `Default` and "no environment" are
    /// the same value, so a composition that installs no developer tools and one
    /// that installs them with the knobs unset behave identically.
    #[test]
    fn the_default_override_forces_neither_preset_nor_profile() {
        let quiet = AuthoredBrainOverride::default();
        assert_eq!(quiet.preset(), None);
        assert_eq!(quiet.profile(), None);
    }
}
