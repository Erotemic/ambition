//! Assembled-app guard that every registered character resolves its declared
//! art and that each shipped provider's starting character is actually registered.

use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::characters::prepared::PreparedCharacterRegistry;

#[test]
fn every_registered_character_resolves_the_art_it_declares() {
    // SHELL-hosted: the provider plugins that register characters (Sanic, Mary-O,
    // Pocket) join in that composition, and they are the ones this guard exists
    // for. The non-hosted build has no cast to check.
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    // BUILDING an app is not COMPOSING one. Character preparation publishes its
    // registry at the plugin-composition barrier, which Bevy's runners close and
    // `build_visible_app` alone does not — so a guard that inspects a built-but-
    // never-run app is inspecting the staged cast, not the published one.
    ambition_platformer2d::runtime::finalize(&mut app);
    let registry = app
        .world()
        .get_resource::<PreparedCharacterRegistry>()
        .expect("the shipped composition registers characters through the one seam");
    assert!(
        !registry.is_empty(),
        "no character reached the prepared registry, so this guard would pass vacuously"
    );

    let mut complaints = Vec::new();
    for (id, prepared) in registry.iter() {
        for unresolved in prepared.unresolved_references() {
            // The resolver's own message already names the id, who declared it, and the closest
            // match.
            complaints.push(format!("  {id}: {unresolved}"));
        }
    }
    assert!(
        complaints.is_empty(),
        "{} registered character reference(s) name something that does not exist. \
         Preparation logs these and publishes anyway, so the running game draws a \
         placeholder rather than crashing — which is why this has to be a test:\n{}",
        complaints.len(),
        complaints.join("\n"),
    );

    // Check the expected starting-character population explicitly so a missing
    // provider cannot make the resolver audit pass vacuously.
    for (provider, character) in [
        ("Sanic", ambition_demo_sanic::SANIC_CHARACTER_ID),
        (
            "Mary-O",
            ambition_demo_mary_o::provider::MARY_O_CHARACTER_ID,
        ),
    ] {
        assert!(
            registry.get(character).is_some(),
            "{provider}'s starting character `{character}` is not in the prepared \
             registry, so the art pipeline calls it UnknownCharacter and the player \
             body draws a placeholder rectangle. A catalog fragment declares what a \
             character IS; `register_character` is what makes it exist"
        );
    }
}

/// The assembled cast must have one effective authority per character.
#[test]
fn the_shipped_cast_has_one_authority_per_character() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    ambition_platformer2d::runtime::finalize(&mut app);
    let conflicts =
        ambition_platformer2d::actors::character_runtime::audit::audit_character_authority_parity(
            app.world(),
        );
    assert!(
        conflicts.is_empty(),
        "{} character(s) are declared by both the prepared registry and the \
         catalog with different content. Every resolver prefers the registry, so \
         the catalog's version is dead content that still reads as \
         authoritative:\n{}",
        conflicts.len(),
        conflicts
            .iter()
            .map(|conflict| format!("  {conflict}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// The assembled app must prepare the authored mite characters with their body traits.
#[test]
fn the_migrated_mites_reach_the_prepared_registry_with_their_death_traits() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    ambition_platformer2d::runtime::finalize(&mut app);
    let registry = app
        .world()
        .get_resource::<PreparedCharacterRegistry>()
        .expect("the shipped composition registers characters through the one seam");

    for (id, explodes, divides_into, health) in [
        ("npc_exploding_mite", true, None, 2),
        ("npc_dividing_mite", false, Some("npc_puppy_slug"), 4),
    ] {
        let prepared = registry.get(id).unwrap_or_else(|| {
            panic!(
                "`{id}` is migrated content: its death traits live on the CHARACTER \
                 and nowhere else, so a composition that does not prepare it ships \
                 a mite that cannot die properly"
            )
        });
        let traits = prepared
            .death_traits
            .as_ref()
            .unwrap_or_else(|| panic!("`{id}` prepared without the death traits it authors"));
        assert_eq!(traits.explodes_on_death, explodes, "{id}");
        assert_eq!(traits.divides_into.as_deref(), divides_into, "{id}");
        assert_eq!(
            prepared.vitals.max_health,
            Some(health),
            "`{id}` must carry the pool its archetype row used to give it"
        );
    }
}
