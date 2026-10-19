//! A Smash seat wearing a skin is its fighter, drawn in that look.
//!
//! The skin is art only (`MatchParticipant::skin`): the body wears the
//! fighter's character, and the render index draws the skin's sheet. This
//! composition has no looks of its own, so the seat borrows the other
//! fighter's sheet: the road is the same for any registered row.

use ambition_demo_smash_app::build_demo_app;

#[test]
fn a_seat_wearing_a_skin_is_its_fighter_drawn_from_the_skins_sheet() {
    use ambition_demo_smash::{SMASH_CHARACTER_ID as FIGHTER, SMASH_OPPONENT_ID as LOOK};
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::characters::actor::{WornCharacter, WornSkin};

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    let mut roster = ambition_demo_smash::smash_roster([FIGHTER, LOOK]);
    roster.participants[0].skin = Some(LOOK.into());
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
        ));
    for _ in 0..120 {
        app.update();
    }

    let world = app.world_mut();
    let mut seats = world.query::<(
        &MatchSeat,
        &WornCharacter,
        Option<&WornSkin>,
        &ambition_platformer2d::combat::components::FeatureId,
    )>();
    let dressed: Vec<_> = seats
        .iter(world)
        .filter(|(_, worn, _, _)| worn.id() == FIGHTER)
        .map(|(_, _, skin, feature)| (skin.map(|skin| skin.id().to_string()), feature.0.clone()))
        .collect();
    assert_eq!(dressed.len(), 1, "one seat wears `{FIGHTER}`");
    let (skin, feature) = &dressed[0];
    assert_eq!(skin.as_deref(), Some(LOOK), "the seat wears the skin it was given, and is still `{FIGHTER}`");

    let index = world.resource::<ambition_platformer2d::sim_view::ActorRenderIndex>();
    let view = index
        .get(feature)
        .unwrap_or_else(|| panic!("the dressed seat `{feature}` has a render view"));
    assert_eq!(
        view.sprite_character_id.as_deref(),
        Some(LOOK),
        "the seat is drawn from the skin's sheet, not its fighter's"
    );
}
