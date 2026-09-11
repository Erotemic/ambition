//! ⭐⭐ **CAN TWO `App`s IN ONE PROCESS SELECT DIFFERENT PACKS?** That is
//! fast-iteration I3's acceptance for step 1, in its own words: *"Two Apps can
//! select different packs without contamination."*
//!
//! ⛔⛔ **THE ANSWER WAS STRUCTURALLY NO.** `pack::prepared()` is a process-wide
//! `OnceLock`: the first caller compiles and every later caller — in any App, in
//! any test, forever — receives that value. A second App could not disagree, a
//! reload had nowhere to put a new pack, and a test could not hand a composition
//! content of its own. Nothing was wrong with the pack; the SHAPE could not
//! express a second one.
//!
//! ⚠ THE SUBJECT IS THE MIGRATED FAMILY ONLY. Move tables read this App's
//! selection; items, encounters, audio and boss profiles still read the boot
//! pack, which is what step 1 scopes ("for migrated families"). A test that
//! claimed otherwise would be claiming a migration that has not happened.

use super::*;

/// Every move in every move-table source, half a second longer.
///
/// ⛔ A TYPED EDIT, NOT A TEXT SUBSTITUTION. Bumping a number by regex would
/// change whatever else in the file happened to match, and this repository has
/// already shipped one fixture edit that landed in two places.
///
/// ⭐ LENGTHENING IS SAFE BY CONSTRUCTION: a window must lie inside
/// `[0, duration_s]`, so a longer move cannot invalidate a window that already
/// fit. Shortening could, and the refusal would look like a selection failure.
fn half_a_second_longer(path: &str, text: String) -> String {
    if !path.starts_with("data/movesets/") {
        return text;
    }
    let mut doc = ambition_entity_catalog::EntityCatalogDoc::parse(&text)
        .expect("a shipped move table parses");
    for entity in &mut doc.entities {
        if let Some(moveset) = entity.contracts.moveset.as_mut() {
            for spec in &mut moveset.moves {
                spec.duration_s += 0.5;
            }
        }
    }
    doc.to_ron().expect("and serializes back")
}

/// A composition that registered its declared cast against `pack`.
fn app_selecting(pack: std::sync::Arc<PreparedContentPack>) -> bevy::prelude::App {
    let mut app = bevy::prelude::App::new();
    select_pack(&mut app, pack);
    crate::character_catalog::register(&mut app);
    crate::player_robot_lineage::register_declared_cast(&mut app);
    // ⛔ THE RAW ROAD, NAMED. This fixture installs no technique handlers, so
    // real admission would correctly withhold every character naming a native
    // effect — the right answer to a question this test is not asking.
    ambition_characters::prepared::close_preparation_barrier_without_admission(app.world_mut());
    app
}

/// The subject: a character that is BOTH in a shipped move table and in the
/// buildable cast, derived rather than named.
///
/// ⛔ DERIVED, because naming one hard-codes a roster decision into a test about
/// pack selection — and the first id to leave the roster would redden this file
/// for a reason that has nothing to do with it.
fn a_character_in_both(pack: &PreparedContentPack) -> String {
    let table = ambition_characters::moveset_content_schema::lowered_movesets(pack)
        .expect("the shipped pack carries a move section");
    let buildable: std::collections::BTreeSet<&str> =
        crate::character_catalog::buildable_cast().collect();
    table
        .keys()
        .find(|id| buildable.contains(id.as_str()))
        .expect(
            "no shipped move table names a buildable character, so every arm \
             below would be about a cast that plays nothing",
        )
        .clone()
}

fn duration_of(app: &bevy::prelude::App, id: &str) -> f32 {
    app.world()
        .resource::<ambition_characters::prepared::PreparedCharacterRegistry>()
        .get(id)
        .unwrap_or_else(|| panic!("`{id}` is published"))
        .kit
        .projectable_moveset()
        .unwrap_or_else(|| panic!("`{id}` carries a moveset"))
        .moves[0]
        .duration_s
}

/// ⭐ THE PREMISE, FIRST. The two packs must actually differ, or "no
/// contamination" is satisfied by two Apps reading one pack.
#[test]
fn the_two_packs_really_do_disagree() {
    let shipped = compile_pack().expect("the shipped pack compiles");
    let edited = compile_pack_with(half_a_second_longer).expect("the edited pack compiles");
    let who = a_character_in_both(&shipped);

    let read = |pack: &PreparedContentPack| {
        ambition_characters::moveset_content_schema::lowered_movesets(pack).expect("a section")
            [&who]
            .moves[0]
            .duration_s
    };
    assert!(
        (read(&edited) - read(&shipped) - 0.5).abs() < 1e-6,
        "the edit did not reach the compiled pack: {} vs {}",
        read(&shipped),
        read(&edited)
    );
}

/// ⛔⛔ **TWO APPs, TWO PACKS, NO CONTAMINATION** — I3 step 1's acceptance.
#[test]
fn two_apps_select_different_packs_without_contamination() {
    let shipped = std::sync::Arc::new(compile_pack().expect("compiles"));
    let edited = std::sync::Arc::new(compile_pack_with(half_a_second_longer).expect("compiles"));
    let who = a_character_in_both(&shipped);

    let first = app_selecting(std::sync::Arc::clone(&shipped));
    let second = app_selecting(std::sync::Arc::clone(&edited));

    assert!(
        (duration_of(&second, &who) - duration_of(&first, &who) - 0.5).abs() < 1e-6,
        "`{who}` plays {} in the App that selected the shipped pack and {} in the \
         App that selected the edited one — the two Apps are reading one pack",
        duration_of(&first, &who),
        duration_of(&second, &who)
    );
}

/// ⛔ AND ORDER DOES NOT DECIDE IT, which is the specific failure a `OnceLock`
/// produces: whichever App ran FIRST would have won for the whole process.
#[test]
fn the_app_that_selects_second_still_gets_its_own_pack() {
    let shipped = std::sync::Arc::new(compile_pack().expect("compiles"));
    let edited = std::sync::Arc::new(compile_pack_with(half_a_second_longer).expect("compiles"));
    let who = a_character_in_both(&shipped);

    let first = app_selecting(std::sync::Arc::clone(&edited));
    let second = app_selecting(std::sync::Arc::clone(&shipped));
    assert!(
        (duration_of(&first, &who) - duration_of(&second, &who) - 0.5).abs() < 1e-6,
        "the App that selected FIRST decided what the second one plays"
    );
}

/// ⚠ AND AN APP THAT SELECTS NOTHING STILL GETS THE BOOT PACK — the behaviour
/// every composition in the tree relies on, which this change must not have
/// taken away.
#[test]
fn an_app_that_selects_nothing_reads_the_boot_pack() {
    let shipped = compile_pack().expect("compiles");
    let who = a_character_in_both(&shipped);
    let mut app = bevy::prelude::App::new();
    crate::character_catalog::register(&mut app);
    crate::player_robot_lineage::register_declared_cast(&mut app);
    ambition_characters::prepared::close_preparation_barrier_without_admission(app.world_mut());

    let expected = ambition_characters::moveset_content_schema::lowered_movesets(&shipped)
        .expect("a section")[&who]
        .moves[0]
        .duration_s;
    assert!(
        (duration_of(&app, &who) - expected).abs() < 1e-6,
        "an App that selected nothing did not get the shipped pack"
    );
}
