//! The Solid Snake's art: both forms answer to one scale, and every pose the
//! shell plays resolves. The shell's rule is the module's, and its cases are
//! there (`ambition_demo_mary_o_modules::shell`).

// (The top/side geometry itself is proven in `crate::stomp`, the ONE authority
// both enemies read.)

/// THE WALKING SNAKE AND THE BOXED SNAKE RESCALE TOGETHER.
///
/// in-the-box form are consistent if we ever change the scale of the snake."*
///
/// it asserts PROPORTIONALITY rather than the sizes themselves, on purpose:
/// the numbers are the art's to choose and change, and a test that pinned them
/// would fail on an ordinary redraw. What may not change is that the two forms
/// answer to the same scale.
#[test]
fn both_snake_forms_follow_one_scale() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
    use ambition_platformer2d::sprite_sheet::character::CharacterAnim;

    let at = |wpp: f32, anim: CharacterAnim| {
        ambition_platformer2d::character_sprites::posed_body_geometry(&sprite_content,
            super::SNAKE_SHEET_TARGET,
            anim,
            wpp,
        )
        .map(|g| g.collision)
    };

    let base = crate::pack::PACK.posed_body_world_per_pixel(&sprite_content, super::SNAKE_SHEET_TARGET);
    let (Some(walk), Some(boxed)) = (
        at(base, CharacterAnim::Idle),
        at(base, CharacterAnim::ShellIdle),
    ) else {
        // No baked art (`--no-assets`): there is no geometry to be consistent
        // about. Skipping is honest here BECAUSE the assertion below is about a
        // relationship between two measurements, not about their presence — and
        // a fixture with no sheet cannot produce either one.
        //
        // VERIFIED NOT TAKEN in this tree by making it panic and
        // watching the test still pass — the check below really runs, rather
        // than skipping the way three other checks in this session did.
        return;
    };
    let (Some(walk2), Some(boxed2)) = (
        at(base * 2.0, CharacterAnim::Idle),
        at(base * 2.0, CharacterAnim::ShellIdle),
    ) else {
        panic!("the sheet resolved at one scale and not at twice it");
    };

    // The premise: the two forms are actually different boxes, or "they agree"
    // is vacuous.
    assert!(
        (walk - boxed).length() > 1e-3,
        "the walking and boxed forms measure the same box ({walk:?}), so this \
         test would pass without them being related at all"
    );

    for (name, one, two) in [("walking", walk, walk2), ("boxed", boxed, boxed2)] {
        assert!(
            (two.x - one.x * 2.0).abs() < 1e-3 && (two.y - one.y * 2.0).abs() < 1e-3,
            "the {name} form does not follow the scale: {one:?} at 1x became \
             {two:?} at 2x. Both forms must read the SAME world-per-pixel, or a \
             rescale moves one and leaves the other."
        );
    }
}

/// ⛔⛔ THE SILENT FALLBACK IS A SECOND SIZE.
///
/// The scale resolver reads 1.0 when the baked sheet does not resolve, and the
/// art derives 0.365 (measured 2026-10-08, at two times Mary-O's width). A
/// snake sized by the fallback is almost three times the size of one sized by
/// the art, and nothing at runtime says which happened.
///
/// Jon, 2026-09-05: *"the size of the snake has seemed to vary depending on the
/// global game state"*. That is the shape of a fallback that fires in some
/// compositions and not others, so this pins that it does not fire where the
/// demo actually runs.
#[test]
fn the_snake_is_sized_by_its_art_and_not_by_the_no_sheet_fallback() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
    let geometry = ambition_platformer2d::character_sprites::posed_body_geometry(&sprite_content,
        super::SNAKE_SHEET_TARGET,
        ambition_platformer2d::sprite_sheet::character::CharacterAnim::Idle,
        1.0,
    );
    assert!(
        geometry.is_some(),
        "the `{}` sheet did not resolve, so the scale is the 1.0 fallback \
         instead of the 0.365 the art derives, with nothing at runtime saying so",
        super::SNAKE_SHEET_TARGET
    );
    let scale = crate::pack::PACK.posed_body_world_per_pixel(&sprite_content, super::SNAKE_SHEET_TARGET);
    assert!(
        (scale - 1.0).abs() > 1e-6,
        "the scale IS the fallback value; see this test's doc"
    );
    // ⚠ A range, not an equality: the number moves when the art is redrawn, and
    // pinning it exactly would make an art change look like a defect. What must
    // never happen is landing back near the fallback.
    // The row asks for two times Mary-O's width (2026-10-08), so the range is
    // two times the one that held while the snake was as wide as she is.
    assert!(
        (0.20..0.60).contains(&scale),
        "world_per_pixel {scale} is outside the range the authored art implies"
    );
}

/// ⛔⛔ EVERY POSE THE SNAKE PLAYS MUST RESOLVE, for the same reason Mary-O has
/// this test — and Jon's report is about the SNAKE specifically.
///
/// Jon, 2026-09-05: *"the size of the snake has seemed to vary depending on the
/// global game state, so maybe something else exposed it."*
///
/// `sync_sprite_posed_bodies` resolves geometry PER ANIMATION and, on `None`,
/// `continue`s — leaving the body whatever box it last had. ⇒ An animation whose
/// sheet publishes no body metrics does not fall back to a default; it FREEZES
/// the previous pose's box. The state that "varies" would then be WHICH
/// ANIMATION IS PLAYING, which from the outside looks exactly like a size that
/// depends on game state.
///
/// ⚠ The snake is the sharp case because its poses are DELIBERATELY different
/// sizes — `withdrawing_into_the_shell_shrinks_the_body` pins the boxed form as
/// far narrower than the sprawled one. A frozen box is therefore not a subtle
/// error here; it is one silhouette wearing another's collision.
#[test]
fn every_pose_the_snake_plays_resolves() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
    use ambition_platformer2d::sprite_sheet::character::CharacterAnim;

    for anim in [
        CharacterAnim::Idle,
        CharacterAnim::Emerge,
        CharacterAnim::Peek,
        CharacterAnim::Retreat,
        CharacterAnim::ShellIdle,
    ] {
        let resolved = ambition_platformer2d::character_sprites::posed_body_geometry(&sprite_content,
            super::SNAKE_SHEET_TARGET,
            anim,
            crate::pack::PACK.posed_body_world_per_pixel(&sprite_content, super::SNAKE_SHEET_TARGET),
        );
        assert!(
            resolved.is_some(),
            "{anim:?} publishes no body metrics, so `sync_sprite_posed_bodies` \
             skips the snake on every frame it plays that pose and the body keeps \
             the PREVIOUS pose's box — a size that changes with which animation \
             is playing."
        );
    }
}
