//! A character definition's provider and its catalog fragment's provider are
//! ONE identifier.
//!
//! Two registries name providers. `CharacterCatalogRegistry` keys fragments by
//! provider and namespaces every brain preset as `provider::name`;
//! `PreparedCharacterRegistry` holds definitions, each carrying a `provider`
//! string. Nothing made them agree, and the difference is not cosmetic: a
//! character's own default autonomous profile is authored as a provider-relative
//! reference (`"fighter"`), and preparation now qualifies it with the
//! DEFINITION's provider rather than borrowing a namespace off the character's
//! catalog row. That is only correct if the two id spaces are the same one.
//!
//! This test is what replaces the assumption, so the fixture repair rests on something.
//!
//! It is a claim only the assembled app can make: every provider plugin in the
//! shipped composition registers both authorities, and this asks whether they
//! used the same word.

use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::characters::actor::character_catalog::CharacterCatalogRegistry;
use ambition_platformer2d::characters::prepared::PreparedCharacterRegistry;

#[test]
fn character_definitions_and_catalog_fragments_share_one_provider_namespace() {
    // SHELL-hosted, for the same reason the art guard next door is: the plugins
    // that register characters (Sanic, Mary-O, Pocket) only join in that
    // composition, and they are the ones with a provider id to disagree about.
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    // Preparation publishes at the plugin-composition barrier, which building
    // alone does not close.
    ambition_platformer2d::runtime::finalize(&mut app);

    let catalog_providers: std::collections::BTreeSet<String> = app
        .world()
        .get_resource::<CharacterCatalogRegistry>()
        .expect("the shipped composition registers catalog fragments")
        .providers()
        .map(str::to_string)
        .collect();
    let prepared = app
        .world()
        .get_resource::<PreparedCharacterRegistry>()
        .expect("the shipped composition registers characters through the one seam");

    assert!(
        !catalog_providers.is_empty() && !prepared.is_empty(),
        "one of the two registries is empty, so this guard would pass vacuously \
         ({} catalog provider(s), {} prepared character(s))",
        catalog_providers.len(),
        prepared.len(),
    );

    // the membership test must be able to say NO. Without this the
    // assertion below is only as strong as the set being non-empty, and a set
    // that answered `true` for everything would look identical.
    assert!(
        !catalog_providers.contains("no_such_provider_authored_this"),
        "the provider set answers yes to a provider nobody registered, so \
         membership proves nothing"
    );

    let mut strangers = Vec::new();
    let mut checked = 0usize;
    for (id, definition) in prepared.iter() {
        checked += 1;
        if !catalog_providers.contains(&definition.provider) {
            strangers.push(format!(
                "  {id}: definition says provider `{}`, and the catalog registry \
                 knows only {catalog_providers:?}",
                definition.provider,
            ));
        }
    }

    assert!(
        checked > 0,
        "no definition was examined, so nothing was proved"
    );
    assert!(
        strangers.is_empty(),
        "{} registered character(s) name a provider the catalog registry never \
         assembled under. Preset keys are namespaced `provider::name` by the \
         CATALOG's id, and a character's own default profile is qualified by the \
         DEFINITION's id — so a disagreement here silently resolves a profile \
         reference to a key that does not exist:\n{}",
        strangers.len(),
        strangers.join("\n"),
    );
}
