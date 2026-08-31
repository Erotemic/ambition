//! Does the SHIPPED demo actually have what the transformation beat asks for?
//!
//! ⭐⭐ THIS IS THE PRICE OF AN `Option<Res<_>>`. `sync_grown_form` reads the
//! form's sheet through the character catalog rather than through a
//! `match character_id` of its own (queue row D166), and it takes both the
//! catalog and the authored sheets as OPTIONS so a narrow unit fixture can swap
//! her form without staging a roster. That absence means one thing —
//! *"there is no sheet to read"*, the case the beat has always answered with its
//! fallback — but an option nobody checks in the real app is a silent veto, and
//! this repo keeps finding those.
//!
//! ⛔ SO THE UNIT TEST IS NOT ENOUGH. `a_forms_beat_is_as_long_as_the_sheet_its_
//! catalog_row_names` proves the join answers when it is handed a catalog. Only
//! a booted demo can say the demo HANDS IT ONE.

use ambition_demo_mary_o_app::build_demo_app;
use ambition_platformer2d::character::{AuthoredSheets, CharacterCatalog};

/// Every form Mary-O can wear, by the id `WornCharacter` carries.
const FORMS: [&str; 3] = ["mary_o", "mary_o_tall", "mary_o_fire"];

#[test]
fn every_mary_o_form_resolves_a_real_sheet_in_the_shipped_demo() {
    let mut app = build_demo_app();
    for _ in 0..120 {
        app.update();
    }

    let catalog = app.world().get_resource::<CharacterCatalog>().expect(
        "the demo installs no `CharacterCatalog`, so `sync_grown_form` takes the \
             fallback beat for every transformation and no unit test can see it",
    );
    let authored = app
        .world()
        .get_resource::<AuthoredSheets>()
        .expect("the demo installs no `AuthoredSheets`");

    for form in FORMS {
        let spec =
            ambition_platformer2d::sprite_sheet::character::catalog_join::sheet_for_character_id_from_data(
                authored,
                catalog.data(),
                form,
            );
        assert!(
            spec.is_some(),
            "`{form}` resolves no sheet from the demo's own catalog row, so its \
             transformation beat is the unreadable-art fallback rather than the \
             length of the clip a player watches"
        );
    }
}
