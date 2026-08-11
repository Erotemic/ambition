//! **Every registered character's declared art actually resolves.**
//!
//! `CharacterDefinition::with_sheet` names a sheet TARGET, and preparation checks
//! it against the engine's baked manifest index. The check was already there and
//! already ran — it just could not fail anything. An unresolved reference is
//! logged and the registration publishes anyway, deliberately: a character that
//! draws a placeholder and says why beats a session that refuses to boot.
//!
//! The gap that left is that `checked_namespaces()` reports a resolver RAN, not
//! that it agreed. So four shipped characters — `sanic`, `super_sanic`, `mary_o`,
//! `mary_o_tall` — declared `<name>_spritesheet` (the sheet FILE stem) where the
//! registry is keyed by `<name>` (the sheet's `target:`), drew placeholders in the
//! running game, printed four ERROR lines listing all 400-odd available ids on
//! every boot, and no test anywhere went red.
//!
//! This is that test. It is not the same check as the resolver — the resolver
//! proves content agrees with content. This proves the SHIPPED composition
//! contains no such disagreement, which is a claim only the assembled app can
//! make.

use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry;

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
            // The resolver's own message already names the id, who declared it,
            // and the closest match. Reproducing it whole keeps the fix in the
            // failure output instead of one indirection away.
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

    // **EVERY SHIPPED PROVIDER'S STARTING CHARACTER IS ACTUALLY REGISTERED.**
    //
    // The check above only inspects characters that ARE in the registry, so a
    // provider that never registers passes it by being absent. The comment at the
    // top of this file has named Pocket as covered since it was written, and
    // Pocket registered nothing at all — it declared a catalog fragment and
    // stopped. Its runner was `UnknownCharacter` to the art pipeline and drew the
    // marked placeholder: picking "Pocket" from the launcher showed a blue box
    // standing on the platform, with this guard green (found by capturing the
    // route, 2026-07-29).
    //
    // A guard that inspects a population cannot notice something missing FROM the
    // population. This names the population instead.
    for (provider, character) in [
        ("Sanic", ambition_demo_sanic::SANIC_CHARACTER_ID),
        (
            "Mary-O",
            ambition_demo_mary_o::provider::MARY_O_CHARACTER_ID,
        ),
        ("Pocket", ambition_demo_pocket::POCKET_CHARACTER_ID),
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

/// **The two declaration authorities agree, in the SHIPPED composition.**
///
/// `audit_character_authority_parity` has existed since 2026-07-26 and reports
/// through `error!`. Nothing has ever asserted on it, so a conflict in the
/// shipped cast is a log line among hundreds and a green suite
/// [[feedback-a-green-guardrail-proves-nothing]]. This is the assertion.
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

/// **The migrated mites reach the SHIPPED composition with their bodies.**
///
/// D73 phase 2 moved `explodes_on_death` and `divides_on_death` off
/// `character_archetypes.ron` and onto the two mite CHARACTERS. That leg —
/// authored in `ambition_content`, registered through `BUILDABLE_ONLY_CAST`,
/// prepared into the registry the spawn path reads — is only true of the
/// assembled app, so only the assembled app can assert it.
///
/// ⛔ **the failure this exists for is silent.** Delete either mite from the
/// build-only cast, or empty its arm of `authored_intrinsics`, and nothing
/// crashes: the placement still names a character, `plan.definition()` reports
/// it missing or bodiless, the body keeps its archetype — which no longer says
/// anything about death — and a sandbox mite quietly stops exploding. Nobody
/// finds that until they stand next to one.
#[test]
fn the_migrated_mites_reach_the_prepared_registry_with_their_death_traits() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    ambition_platformer2d::runtime::finalize(&mut app);
    let registry = app
        .world()
        .get_resource::<PreparedCharacterRegistry>()
        .expect("the shipped composition registers characters through the one seam");

    for (id, explodes, divides, health) in [
        ("npc_exploding_mite", true, false, 2),
        ("npc_dividing_mite", false, true, 4),
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
        assert_eq!(traits.divides_on_death, divides, "{id}");
        assert_eq!(
            prepared.vitals.max_health,
            Some(health),
            "`{id}` must carry the pool its archetype row used to give it"
        );
    }
}
