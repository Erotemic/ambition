//! A CHARACTER'S SHEET MUST NOT BE FILED UNDER A NAME THAT MEANS EIGHT
//! THINGS.
//!
//! `SheetRegistry::from_baked_table` keys sheets by file root so
//! the player's `player_robot_v3` stays distinct from the enemy `robot`. A file
//! root only identifies a sheet while the file holds ONE record; `creator_lab_props`
//! packs 8 props into one, so that root names all eight and therefore none.
//!
//! THIS IS THE HALF THE SPRITE CRATE CANNOT WRITE. It can see that a root
//! is ambiguous; it cannot see whether anything resolves art by that root,
//! because it has no catalog — the same split `shadowed_targets` documents, and
//! the same reason `report_shadowed_character_sheets` lives here. A refusal that
//! hit a real character would cost that character its authored attack box
//! silently, falling back to the shared hardcoded volume.

use ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog;

#[test]
fn no_composed_character_resolves_its_sheet_by_a_refused_file_root() {
    let refused = ambition_platformer2d::character_sprites::refused_file_roots();
    // LOUD: with nothing refused this test passes over an empty set forever,
    // including on the day the refusal is accidentally deleted. The baked table
    // is known to contain a packed prop atlas, so zero means the mechanism
    // stopped working, not that the tree got cleaner.
    assert!(
        !refused.is_empty(),
        "no file root was refused at all — the baked table contains a packed \
         prop atlas, so this means the refusal itself stopped running and every \
         assertion below is vacuous"
    );

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    // The barrier: the cast is PUBLISHED at `Plugin::finish`, so a freshly built
    // App carries a partial catalog.
    ambition_platformer2d::platformer::app_finalization::finalize(&mut app);
    let catalog = app.world().resource::<CharacterCatalog>();
    assert!(
        !catalog.is_empty(),
        "the composed catalog is empty, so sweeping it proves nothing"
    );

    // POSITIVE CONTROL: the two names must live in the same namespace. The sweep below compares
    // `entry.manifest_target()` against a refused file root. If those were different spellings — a
    // path against a stem, say — nothing could ever match and this test would pass forever while
    // blind.
    let resolvable = catalog
        .iter()
        .filter(|(_, entry)| {
            entry
                .manifest_target()
                .is_some_and(ambition_platformer2d::character_sprites::resolves_by_file_root)
        })
        .count();
    assert!(
        resolvable > 0,
        "not one composed character's `manifest_target` resolves in the \
         file-root index, so the comparison below is between two namespaces \
         that never meet and cannot fail"
    );

    let collisions: Vec<String> = catalog
        .iter()
        .filter_map(|(id, entry)| {
            let target = entry.manifest_target()?;
            refused.iter().find(|r| r.file_root == target).map(|r| {
                format!(
                    "  {id} resolves art by `{target}`, which names {:?}",
                    r.targets
                )
            })
        })
        .collect();

    assert!(
        collisions.is_empty(),
        "{} character(s) resolve their sheet by a file root that names several \
         records, so the sheet they get depends on the packer's emission \
         order:\n{}\n\nEither give the character its own manifest, or look it \
         up by `record.target` instead of by file root.",
        collisions.len(),
        collisions.join("\n"),
    );
}
