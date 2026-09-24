//! Can two `App`s in one process select different packs? Fast-iteration I3
//! step 1's acceptance: *"Two Apps can select different packs without
//! contamination."*
//!
//! `pack::prepared()` is a process-wide `OnceLock`: the first caller compiles,
//! and every later caller in any App or test receives that value. So a
//! second App could not disagree, a reload had nowhere to put a new pack, and a
//! test could not give a composition its own content.
//!
//! The subject is the migrated family only. Move tables read this App's
//! selection; items, encounters, audio and boss profiles still read the boot
//! pack, as step 1 scopes ("for migrated families").

use super::*;

/// Every move in every move-table source, half a second longer.
///
/// A typed edit, not a text substitution: a regex bump would change anything
/// else in the file that matched.
///
/// Lengthening is safe: a window must lie inside `[0, duration_s]`, so a
/// longer move cannot invalidate a window that fit. Shortening could, and the
/// refusal would look like a selection failure.
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
    // The raw road, named. This fixture installs no technique handlers, so real
    // admission would withhold every character naming a native effect, which is
    // not what this test asks.
    ambition_characters::prepared::close_preparation_barrier_without_admission(app.world_mut());
    app
}

/// The subject: a character that is in a shipped move table and in the
/// buildable cast, derived, not named.
///
/// Derived, so a roster change does not break a test about pack selection.
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

/// The premise first: the two packs must differ, or "no contamination" is
/// satisfied by two Apps reading one pack.
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

/// Two Apps, two packs, no contamination: I3 step 1's acceptance.
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

/// Order does not decide it. With a `OnceLock`, whichever App ran first would
/// win for the whole process.
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

/// An App that selects nothing still gets the boot pack, which every
/// composition relies on.
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

/// The App tells the engine which pack. `install_selection` publishes an
/// `ambition_platformer2d_runtime::SelectedContentIdentity` beside the pack,
/// and `prepare_platformer_content` folds it into the prepared content's
/// fingerprint, so sessions prepared under different move tables are
/// different content generations.
///
/// The provider-side test proves the section reaches the fingerprint, using
/// hand-written identity strings, so it cannot see whether this crate puts
/// anything distinguishing into it: dropping `pack.fingerprint` from the
/// format string would pass there. This test catches that.
#[test]
fn two_different_packs_publish_two_different_content_identities() {
    use ambition_platformer2d_runtime::SelectedContentIdentity;

    let shipped = std::sync::Arc::new(compile_pack().expect("compiles"));
    let edited = std::sync::Arc::new(compile_pack_with(half_a_second_longer).expect("compiles"));
    assert_ne!(
        shipped.fingerprint, edited.fingerprint,
        "the premise: the two packs differ"
    );
    assert_eq!(
        (shipped.id.clone(), shipped.version.clone()),
        (edited.id.clone(), edited.version.clone()),
        "the two packs differ in id or version, so this arm would pass without \
         the fingerprint reaching the identity at all"
    );

    let identity_of = |pack: std::sync::Arc<PreparedContentPack>| {
        let mut app = bevy::prelude::App::new();
        select_pack(&mut app, pack);
        app.world().resource::<SelectedContentIdentity>().0.clone()
    };
    assert_ne!(
        identity_of(shipped),
        identity_of(edited),
        "two packs that differ only in their CONTENT publish the same identity, \
         so the engine cannot tell one generation from the other"
    );
}

/// An App that selects nothing publishes nothing. `None` is a real answer
/// ("this composition has no content pack"), and the provider's own test
/// asserts it is a third distinct generation.
#[test]
fn an_app_that_selects_nothing_publishes_no_content_identity() {
    let app = bevy::prelude::App::new();
    assert!(app
        .world()
        .get_resource::<ambition_platformer2d_runtime::SelectedContentIdentity>()
        .is_none());
}
