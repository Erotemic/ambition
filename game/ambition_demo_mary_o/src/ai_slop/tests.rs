//! Tests for the AI Slop head-stomp. The rule is pure ECS (no split-out
//! choreography function like the snake's), so it is checked on a minimal app: what
//! headless cannot check is how the squash LOOKS.

use super::*;
use ambition_platformer2d::characters::actor::{BodyHealth, Health};

/// Build an app with the message seams the stomp writes into, and the rule wired.
fn app() -> App {
    let mut app = App::new();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_systems(Update, bounce_squash_ai_slop);
    app
}

/// A player falling onto a body at (100, 100): feet land squarely on its head,
/// overlapping in x.
fn player(app: &mut App, vel_y: f32) -> Entity {
    app.world_mut()
        .spawn((
            PrimaryPlayer,
            ae::BodyKinematics {
                pos: ae::Vec2::new(100.0, 72.0),
                vel: ae::Vec2::new(0.0, vel_y),
                size: ae::Vec2::new(28.0, 40.0),
                facing: 1.0,
            },
        ))
        .id()
}

fn body_at_head(app: &mut App, ai_slop: bool) -> Entity {
    let mut e = app.world_mut().spawn((
        ae::BodyKinematics {
            pos: ae::Vec2::new(100.0, 100.0),
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(28.0, 28.0),
            facing: 1.0,
        },
        BodyHealth::new(Health::new(1)),
    ));
    if ai_slop {
        e.insert(AiSlop);
    }
    e.id()
}

/// A falling player whose feet are on an AI Slop's head squashes it — the body is
/// despawned and the player is bounced up (gravity is +y, so "up" is `vel.y < 0`).
#[test]
fn a_falling_stomp_squashes_an_ai_slop_and_bounces_the_player() {
    let mut app = app();
    let p = player(&mut app, 120.0);
    let mob = body_at_head(&mut app, true);

    app.update();

    assert!(
        app.world().get_entity(mob).is_err(),
        "the stomped AI Slop is despawned — it just dies, no shell"
    );
    let vel_y = app
        .world()
        .entity(p)
        .get::<ae::BodyKinematics>()
        .unwrap()
        .vel
        .y;
    assert!(
        vel_y < 0.0,
        "the stomp bounces her up (was falling, now {vel_y})"
    );
}

/// A RISING player that overlaps an AI Slop is taking a side/underside hit, not a
/// stomp: the mob survives (the shared contact pass hurts her instead).
#[test]
fn a_rising_player_does_not_squash_an_ai_slop() {
    let mut app = app();
    let _p = player(&mut app, -80.0);
    let mob = body_at_head(&mut app, true);

    app.update();

    assert!(
        app.world().get_entity(mob).is_ok(),
        "an AI Slop is only squashed by a DESCENDING stomp"
    );
    assert!(
        app.world().entity(mob).get::<BodyHealth>().unwrap().alive(),
        "and it is unharmed"
    );
}

/// The stomp only ever touches an AI Slop. An identical stomp onto a body WITHOUT the
/// `AiSlop` marker (a snake, which owns its own shell rule) leaves it untouched — the
/// two enemies never share a code path.
#[test]
fn the_stomp_never_squashes_a_non_ai_slop() {
    let mut app = app();
    let _p = player(&mut app, 120.0);
    let snake_like = body_at_head(&mut app, false);

    app.update();

    assert!(
        app.world().get_entity(snake_like).is_ok(),
        "a non-AI-Slop body is never despawned by the AI Slop stomp"
    );
    assert!(
        app.world()
            .entity(snake_like)
            .get::<BodyHealth>()
            .unwrap()
            .alive(),
        "and never harmed — the snake's shell rule is the only thing that stomps it"
    );
}

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
/// `AI_SLOP_WAKE_RADIUS` was chosen to clear the half-width of the widest view a
/// PLAYER can select, so an actor is never popped into frame already moving. That
/// reasoning is only true while the presets say what they said — so read them
/// rather than restate them. If someone widens `Cinematic`, this fails and names
/// the number to move, instead of the radius quietly becoming too small.
#[test]
fn the_wake_radius_clears_the_widest_playable_view() {
    use ambition_platformer2d::persistence::settings::video::CameraZoomPreset;

    let (widest_play_width, _) = CameraZoomPreset::Cinematic.base_view();
    let half_width = widest_play_width * 0.5;
    assert!(
        AI_SLOP_WAKE_RADIUS > half_width,
        "the wake radius ({AI_SLOP_WAKE_RADIUS}) must exceed the half-width of the \
         widest PLAYABLE view ({half_width}), or an actor wakes inside the frame"
    );

    // And the margin is the point, not an accident: five tiles of settling room.
    assert!(
        AI_SLOP_WAKE_RADIUS - half_width >= 5.0 * crate::T,
        "the margin ({}) is what lets a slop settle onto its column before it is \
         seen; below five tiles it wakes mid-fall in view",
        AI_SLOP_WAKE_RADIUS - half_width
    );

    // Debug's view is deliberately NOT cleared — see the constant's doc.
    let (debug_width, _) = CameraZoomPreset::Debug.base_view();
    assert!(
        AI_SLOP_WAKE_RADIUS < debug_width * 0.5,
        "if content ever grows to cover the DEBUG zoom, the policy culls nothing \
         and this test should be deleted along with the reason for the radius"
    );
}
