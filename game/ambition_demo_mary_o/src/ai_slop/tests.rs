//! Tests for the AI Slop head-stomp. The rule is pure ECS (no split-out
//! choreography function like the snake's), so it is checked on a minimal app: what
//! headless cannot check is how the squash LOOKS.

use super::*;
use ambition::characters::actor::{BodyHealth, Health};

/// Build an app with the message seams the stomp writes into, and the rule wired.
fn app() -> App {
    let mut app = App::new();
    app.add_message::<ambition::vfx::VfxMessage>();
    app.add_message::<ambition::sfx::OwnedSfxMessage>();
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
