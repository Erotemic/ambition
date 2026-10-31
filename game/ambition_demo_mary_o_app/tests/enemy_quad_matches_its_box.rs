//! Check that authored body geometry reaches live actors.

use ambition_platformer2d::character_sprites::posed_body_geometry;
use ambition_platformer2d::sprite_sheet::character::CharacterAnim;

/// The AI Slop uses the collision shape and width from its authored row.
#[test]
fn the_ai_slops_box_has_the_shape_its_sheet_publishes() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
    let sheet = posed_body_geometry(&sprite_content,
        ambition_demo_mary_o::ai_slop::AI_SLOP_SHEET_TARGET,
        CharacterAnim::Idle,
        1.0,
    )
    .expect("the AI Slop's sheet publishes body metrics");
    // The body is the heap where it is high. The skirt of its puddle and its
    // sparkles are in the picture and not in the box: the box is the middle
    // part of the frame (Jon, 2026-10-12: the box went past what he could
    // see; it was 161 of the 192 px of the frame, and the heap is 101).
    assert!(
        sheet.collision.x < sheet.render.x * 0.6,
        "the box of the slop is {} px of a frame of {} px: the skirt of its puddle is in it",
        sheet.collision.x,
        sheet.render.x
    );

    // Exercise the production sizing function rather than recomputing its
    // arithmetic in the test, and compare it with the width the row authors.
    let half = ambition_demo_mary_o::ai_slop::ai_slop_half_size(&sprite_content);
    let (width, height) = (half.x * 2.0, half.y * 2.0);
    let authored = match ambition_demo_mary_o::mary_o_character_catalog()
        .get(ambition_demo_mary_o::ai_slop::AI_SLOP_SHEET_TARGET)
        .and_then(|row| row.posed_body.clone())
    {
        Some(ambition_platformer2d::characters::actor::character_catalog::PosedBodyScale::Width(
            width,
        )) => width,
        other => panic!("the ai_slop row authors its width, not {other:?}"),
    };
    assert!(
        (width - authored).abs() < 0.01,
        "the derived body is {width:.1} wide, not the {authored:.1} its row authors — \
         width is the anchor and every level is placed against it"
    );
    // The shape is the shape of the sheet: the scale is one number for the
    // width and for the height.
    let sheet_aspect = sheet.collision.x / sheet.collision.y;
    assert!(
        (width / height - sheet_aspect).abs() < 0.01,
        "the derived body is {width:.1} x {height:.1}, and the sheet's is {sheet_aspect:.2}:1"
    );
}

/// A LIVE slop is the size its character authors — asked of the BODY, not of
/// the arithmetic.
///
/// Asking the function proves the arithmetic. The authored value reached the mirror for two ticks
/// and never reached the body.
///
///  the same lesson the human-grab defect taught one layer up: a test that
/// starts downstream of the wiring cannot see the wiring. This one starts at
/// a booted app.
#[test]
fn a_live_ai_slop_wears_the_size_its_character_authors() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    for _ in 0..400 {
        app.update();
    }

    let authored = ambition_demo_mary_o::ai_slop::ai_slop_half_size(&sprite_content);
    let mut q = app.world_mut().query::<(
        &ambition_platformer2d::engine_core::BodyKinematics,
        &ambition_platformer2d::combat::components::CenteredAabb,
        &ambition_platformer2d::combat::actor_tuning::ActorConfig,
    )>();
    let live: Vec<(bevy::prelude::Vec2, bevy::prelude::Vec2)> = q
        .iter(app.world())
        .filter(|(_, _, config)| ambition_demo_mary_o::ai_slop::is_ai_slop_brain(&config.brain))
        .map(|(kin, aabb, _)| (kin.size, aabb.half_size))
        .collect();

    // the zero floor. A run that tagged no slop at all would otherwise
    // agree with every assertion below by reading no subjects.
    assert!(
        !live.is_empty(),
        "no AI Slop is alive after 400 ticks, so this measured nothing"
    );

    for (kin_size, half) in &live {
        assert!(
            (kin_size.x - authored.x * 2.0).abs() < 0.5
                && (kin_size.y - authored.y * 2.0).abs() < 0.5,
            "a live slop's BODY is {:.1} x {:.1} but its character authors \
             {:.1} x {:.1} — the authored size is reaching a mirror and not the \
             authority (`BodyKinematics.size`), which is what every reset and \
             re-derivation reads",
            kin_size.x,
            kin_size.y,
            authored.x * 2.0,
            authored.y * 2.0
        );
        assert!(
            (half.x - authored.x).abs() < 0.5 && (half.y - authored.y).abs() < 0.5,
            "a live slop's COLLISION BOX is {:.1} x {:.1} (half-extents) but its \
             character authors {:.1} x {:.1}",
            half.x,
            half.y,
            authored.x,
            authored.y
        );
    }
}
