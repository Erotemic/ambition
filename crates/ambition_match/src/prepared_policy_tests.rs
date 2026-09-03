//! Tests for the prepared half that need no `App`.
//!
//! ⭐⭐ ONE TEST, AND THE MEASUREMENT IS WHY. The kernel's
//! `character_runtime/match_activation/tests.rs` was handed over as "the pure
//! half — the tests that call `prepare_match` and never build an App", 3,397
//! lines. Measured before moving anything: **51 tests, of which exactly ONE
//! builds no `App`, and `prepare_match` is called ZERO times in the file.** The
//! other fifty test ACTIVATION — seating bodies, channels, refusals — which is
//! the kernel's job and stays there.
//!
//! ⇒ So "tests move with what they test" moved 70 lines rather than half a file.
//! This one belongs here because its subject is
//! [`crate::prepared::seat_brain_profile`]: it proves a CPU seat's policy comes
//! from the published registry of its provider and from nowhere else, which is
//! this crate's rule and not the kernel's.

/// A CPU SEAT'S POLICY COMES FROM THE PUBLISHED REGISTRY, AND FROM NOWHERE
/// ELSE.
///
/// this was `a_cpu_seat_prefers_a_published_policy_over_an_archetype_of_the_same_name`,
/// and its subject — the PREFERENCE between two authorities — is gone with the
/// second authority. Its middle clause asserted
/// *"the legacy road is still open, which is what makes the preference above a
/// preference rather than a replacement"*; it is a replacement now.
///
/// what survives is the part that was always the real claim, and it is stronger for having one
/// authority: a reference resolves in a PROVIDER.
#[test]
fn a_cpu_seats_policy_resolves_in_a_provider_or_not_at_all() {
use ambition_characters::actor::character_catalog::{BrainProfileRegistry, CharacterCatalog};

const CATALOG: &str = r#"(
    autonomous_profiles: {
        "medium_striker": (
            template: StandStill,
            aggro_radius: 1.0,
            attack_range: 2.0,
        ),
    },
    brain_presets: {},
    action_set_presets: {},
    characters: {},
)"#;
const PROVIDER: &str = "fixture_game";
let profiles = BrainProfileRegistry::from_catalog_for_test(
    PROVIDER,
    &CharacterCatalog::from_data(
        ambition_characters::actor::character_catalog::parse_catalog(CATALOG),
    ),
);

let published = crate::prepared::seat_brain_profile(
    "medium_striker",
    None,
    PROVIDER,
    Some(&profiles),
)
.expect(
    "a published policy of that name resolves — a BARE key reached a registry \
         that holds provider::name, which is the production shape",
);
assert_eq!(published.aggro_radius, 1.0);

// THE POISON. A policy published by a DIFFERENT provider must not
// answer this seat: that is the bare-key match that made this arm vacuous,
// and it would also let one game's `duelist` silently drive another's
// fighter.
assert!(
    crate::prepared::seat_brain_profile(
        "medium_striker",
        None,
        "some_other_game",
        Some(&profiles)
    )
    .is_none(),
    "another provider's policy answered this seat, so the reference is not \
     being resolved in a provider at all"
);

assert!(
    crate::prepared::seat_brain_profile("combatant", None, PROVIDER, Some(&profiles))
        .is_none(),
    "an enemy archetype key answered a controller question, so the archetype \
     table is still a policy authority"
);
}
