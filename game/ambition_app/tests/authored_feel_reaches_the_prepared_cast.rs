//! A catalog row that authors a body's FEEL must be one the barrier prepared.
//!
//! ⭐⭐ WHY THIS IS THE INVARIANT AND NOT "THE TWO AUTHORITIES AGREE". `movement_tuning`
//! and `motion_model` are FOLDED at the preparation barrier — `ambition_characters/src/prepared.rs:1338`
//! is `movement_tuning.or_else(|| catalog?.axis_tuning(&id))` and `:1334` is the
//! same shape for the motion model — so a prepared character cannot disagree with
//! its catalog row about either. The catalog is the authority and the registry is
//! its fold. That is why `CharacterAuthorityConflict` audits `display_name`,
//! `sheet` and `provider` and not these two: those three are carried on both
//! sides, these two are reconciled.
//!
//! ⛔ WHAT WAS NOT RECONCILED WAS THE ROW NOBODY PREPARED. `avatar/starting_character.rs`
//! spelled the fold a second time at read time for the ids the registry did not
//! hold: 89 of 147 per boot (measured 2026-09-11), none of which authored a value.
//! Since AP30 the barrier prepares every row nobody authored, so the registry
//! holds every catalog id and the read-site fold is deleted. This test is the
//! check that a row authoring feel is in the prepared cast.

use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog;
use ambition_platformer2d::characters::prepared::PreparedCharacterRegistry;
use std::collections::BTreeSet;

#[test]
fn every_catalog_row_that_authors_feel_is_in_the_prepared_cast() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    // What `App::run` does before the first update. `combat_schedule.rs` notes
    // that `PreStartup` lives inside that update and that a guard driving
    // `update()` by hand otherwise takes a different barrier road than
    // production does.
    app.finish();
    app.cleanup();
    app.update();

    let world = app.world();
    let catalog = world.resource::<CharacterCatalog>();
    let registry = world.resource::<PreparedCharacterRegistry>();
    let prepared: BTreeSet<&str> = registry.ids().collect();
    let default_motion = CharacterCatalog::empty().motion_model_spec("an_id_no_catalog_knows");

    let mut rows = 0usize;
    let mut authored: Vec<&String> = Vec::new();
    let mut orphaned: Vec<String> = Vec::new();
    for (id, _) in catalog.iter() {
        rows += 1;
        let states_feel = catalog.axis_tuning(id).is_some()
            || catalog.motion_model_spec(id) != default_motion;
        if !states_feel {
            continue;
        }
        authored.push(id);
        if !prepared.contains(id.as_str()) {
            orphaned.push(id.clone());
        }
    }

    // ⛔ ANTI-VACUITY, AND THE SECOND FLOOR IS THE ONE THAT MATTERS. A guard over
    // "rows that author feel" passes trivially when no row authors any, which is
    // one content edit away and would look exactly like this test staying green.
    assert!(
        rows >= 100 && !prepared.is_empty(),
        "the shipped composition published {rows} catalog rows and {} prepared \
         characters; this is probing an empty population",
        prepared.len()
    );
    assert!(
        authored.len() >= 5,
        "only {} catalog row(s) author a tuning or a motion model, so this guard \
         is checking almost nothing. Measured 2026-09-11: 5 — mary_o, mary_o_fire, \
         mary_o_tall, sanic, super_sanic. If authored feel really did move out of \
         the catalog, this test has no subject left and should be deleted rather \
         than lowered.",
        authored.len()
    );

    assert!(
        orphaned.is_empty(),
        "{} catalog row(s) author a body's feel and are NOT in the prepared cast: \
         {orphaned:?}. Their tuning reaches the body only through the read-site \
         fallback in `avatar/starting_character.rs`, never through the barrier's \
         fold — so any consumer holding the prepared definition sees a body on the \
         shared editable tuning while the row says otherwise, and nothing reports \
         it. Either stage the character so the barrier folds its row, or delete the \
         row's authored feel.",
        orphaned.len()
    );
}
