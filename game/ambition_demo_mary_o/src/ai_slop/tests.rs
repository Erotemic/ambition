//! Tests of the AI Slop's content. Its squash is the module
//! `ambition_demo_mary_o_modules::squash` (tests there, and
//! `a_mob_is_squashed_by_each_player` in the Mary-O app).

// ── Dormancy ( content half) ───────────────────────────────────────────

// `a_staged_ai_slop_is_given_its_dormancy_policy` MOVED, it did not go
// away. It lives in `ambition_demo_mary_o_app`'s `one_placement_one_actor`
// now, and the move was forced rather than chosen: the tag pass reads
// `ActorConfig.brain`, and a game crate cannot BUILD an `ActorConfig`. It has no
// `Default`, its `tuning`/`brain_profile` come from an `ArchetypeSpec`, and
// `ArchetypeSpec` is not re-exported through the `ambition_platformer2d`
// umbrella — which is the E9 oracle, so reaching past it is not an option.
//
// The old fixture spawned a bare `FeatureId` on an `App::new()` and asserted the pass answered
// it: it could only ever prove the pass reacts to something a test invented. The integration
// version asserts every slop the REAL construction path builds, from the real authored level,
// gets its policy — and that nothing else does.

/// The radius is DERIVED, and this is what makes that claim checkable.
///
/// Her `hostile_wake_radius` (`room_rules.ron`) was chosen to clear the half-width of the widest view a
/// PLAYER can select, so an actor is never popped into frame already moving. That
/// reasoning is only true while the presets say what they said — so read them
/// rather than restate them. If someone widens `Cinematic`, this fails and names
/// the number to move, instead of the radius quietly becoming too small.
#[test]
fn the_wake_radius_clears_the_widest_playable_view() {
    use ambition_platformer2d::persistence::settings::video::CameraZoomPreset;

    let (widest_play_width, _) = CameraZoomPreset::Cinematic.base_view();
    let half_width = widest_play_width * 0.5;
    let radius = crate::pack::room_rules().dormancy.expect("her rooms state a wake radius").hostile_wake_radius;
    assert!(
        radius > half_width,
        "the wake radius ({}) must exceed the half-width of the \
         widest PLAYABLE view ({half_width}), or an actor wakes inside the frame",
        radius
    );

    // And the margin is the point, not an accident: five tiles of settling room.
    assert!(
        radius - half_width >= 5.0 * crate::T,
        "the margin ({}) is what lets a slop settle onto its column before it is \
         seen; below five tiles it wakes mid-fall in view",
        radius - half_width
    );

    // Debug's view is deliberately NOT cleared — see the file's comment.
    let (debug_width, _) = CameraZoomPreset::Debug.base_view();
    assert!(
        radius < debug_width * 0.5,
        "if content ever grows to cover the DEBUG zoom, the policy culls nothing \
         and this test should be deleted along with the reason for the radius"
    );
}
