//! Tests for the prepared half that need no `App`.
//!
//! The activation tests (seating bodies, channels, refusals) stay in the
//! kernel's `character_runtime/match_activation/tests.rs`. This one belongs
//! here because it tests [`crate::prepared::seat_brain_profile`]: a CPU seat's
//! policy comes only from its provider's published registry.

/// A CPU seat's policy comes from the published registry and from nowhere
/// else. A reference resolves in a provider.
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

// A policy published by a different provider must not answer this seat;
// otherwise one game's `duelist` could drive another's fighter.
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
