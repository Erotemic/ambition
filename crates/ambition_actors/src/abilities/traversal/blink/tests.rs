//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::actor::BodyKinematics;

/// The shared teleport rule (used by both the player blink and any actor
/// body): full distance over open space, clamped a body-half short of a wall,
/// never embedding. This is the single invariant both controllers inherit.
#[test]
fn blink_target_travels_full_distance_then_clamps_at_a_wall() {
    let half = ae::Vec2::new(12.0, 20.0);
    // Open world (no blocks): blink the full distance to the right.
    let empty = ae::World::new("t", ae::Vec2::new(2000.0, 600.0), ae::Vec2::ZERO, vec![]);
    let from = ae::Vec2::new(0.0, 0.0);
    let open = blink_target(&empty, from, ae::Vec2::new(1.0, 0.0), 150.0, half);
    assert!(
        (open.x - 150.0).abs() < 1e-3,
        "open blink travels full distance: {open:?}"
    );

    // A wall whose left face is at x=100 (Block::solid takes the MIN corner):
    // the body stops a half-width (+margin) short of it, never crossing in.
    let walled = ae::World::new(
        "t",
        ae::Vec2::new(2000.0, 600.0),
        ae::Vec2::ZERO,
        vec![ae::Block::solid(
            "wall",
            ae::Vec2::new(100.0, -300.0),
            ae::Vec2::new(120.0, 600.0),
        )],
    );
    let clamped = blink_target(&walled, from, ae::Vec2::new(1.0, 0.0), 150.0, half);
    assert!(
        clamped.x + half.x <= 100.0 + 1e-3,
        "clamped blink must not cross the wall's left face at x=100: right edge={}",
        clamped.x + half.x
    );
    assert!(
        clamped.x > 0.0,
        "but it should still carry toward the wall: {clamped:?}"
    );
}

fn test_app() -> App {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.add_message::<crate::features::HitEvent>();
    app.add_systems(Update, blink_system);
    app
}

fn spawn_player_holding(app: &mut App, id: &str, facing: f32) -> Entity {
    crate::abilities::test_support::spawn_primary_player_holding_at(
        app,
        id,
        ae::Vec2::new(300.0, 300.0),
        facing,
    )
}

fn player_pos(app: &App, player: Entity) -> ae::Vec2 {
    app.world().get::<BodyKinematics>(player).unwrap().pos
}

#[derive(bevy::prelude::Resource, Default)]
struct CapturedHits(Vec<crate::features::HitEvent>);

fn capture_hits(
    mut reader: bevy::prelude::MessageReader<crate::features::HitEvent>,
    mut out: bevy::prelude::ResMut<CapturedHits>,
) {
    out.0.extend(reader.read().cloned());
}

#[test]
fn blink_emits_a_player_side_shockwave_at_arrival() {
    let mut app = test_app();
    app.init_resource::<CapturedHits>();
    app.add_systems(bevy::prelude::Update, capture_hits.after(blink_system));
    let player = spawn_player_holding(&mut app, BLINK_ID, 1.0);
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    let hits = &app.world().resource::<CapturedHits>().0;
    assert_eq!(hits.len(), 1, "one shockwave on arrival");
    // Centered at the arrival point (300 + BLINK_DISTANCE along facing).
    let center_x = (hits[0].volume.bounds().min.x + hits[0].volume.bounds().max.x) * 0.5;
    assert!(
        (center_x - (300.0 + BLINK_DISTANCE)).abs() < 1.0,
        "shockwave is at the arrival point",
    );
    assert_eq!(hits[0].damage, BLINK_SHOCKWAVE_DAMAGE);
    assert!(
        matches!(
            hits[0].source,
            crate::features::HitSource::PlayerSlash { .. }
        ),
        "player-side source so it spares the player",
    );
}

#[test]
fn attack_blinks_the_player_forward_along_facing() {
    // No RoomGeometry inserted → the no-clamp branch teleports the full distance.
    let mut app = test_app();
    let player = spawn_player_holding(&mut app, BLINK_ID, 1.0);
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    assert_eq!(
        player_pos(&app, player),
        ae::Vec2::new(300.0 + BLINK_DISTANCE, 300.0),
        "blink carried the player one BLINK_DISTANCE along facing",
    );
}

#[test]
fn downward_blink_does_not_embed_in_the_floor() {
    // Regression: a vertical blink must pull back by the body's half-HEIGHT,
    // not half-width, or the 40-tall body embeds in the floor and trips the
    // inside-solid OOB detector (the fly + aim-down blink case).
    let mut app = test_app();
    let player = spawn_player_holding(&mut app, BLINK_ID, 1.0); // (300,300), 24x40
                                                                // Solid floor whose top edge is at y=350, just below the player.
    app.insert_resource(ambition_engine_core::RoomGeometry(ae::World::new(
        "test",
        ae::Vec2::new(600.0, 600.0),
        ae::Vec2::new(300.0, 300.0),
        vec![ae::Block::solid(
            "floor",
            ae::Vec2::new(0.0, 350.0),
            ae::Vec2::new(600.0, 250.0),
        )],
    )));
    {
        let mut control = app.world_mut().get_mut::<ActorControl>(player).unwrap();
        control.0.melee_pressed = true;
        control.0.aim = ae::Vec2::new(0.0, 1.0); // brain-resolved local aim: down
    }
    app.update();
    let pos = player_pos(&app, player);
    let half_h = 20.0;
    assert!(
        pos.y + half_h <= 350.0 + 1e-3,
        "downward blink embedded the body in the floor: bottom={}, floor top=350",
        pos.y + half_h,
    );
    assert!(
        pos.y > 300.0,
        "the blink should still carry the player toward the floor (got y={})",
        pos.y,
    );
}

#[test]
fn blink_follows_facing_left() {
    let mut app = test_app();
    let player = spawn_player_holding(&mut app, BLINK_ID, -1.0);
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    assert_eq!(
        player_pos(&app, player),
        ae::Vec2::new(300.0 - BLINK_DISTANCE, 300.0),
        "a left-facing blink goes left",
    );
}

/// ABILITY ORIGIN is subject-generic: blink executes on whatever body is the
/// `ControlledSubject`, even a NON-`PlayerEntity` actor (a possessed body),
/// and the home avatar (not the subject) does NOT blink. This is the exact
/// "blink no longer controls the original robot" invariant — proven headlessly
/// without a player-shaped query.
#[test]
fn blink_executes_on_the_controlled_actor_not_the_home_avatar() {
    use crate::actor::PlayerEntity;
    let mut app = test_app();
    // Home avatar (a PlayerEntity) — holds blink, but is NOT the controlled
    // subject this frame. It must stay put.
    let home_spec = ambition_characters::brain::held_item_by_id(BLINK_ID).unwrap();
    let home = app
        .world_mut()
        .spawn((
            PlayerEntity,
            BodyKinematics {
                pos: ae::Vec2::new(100.0, 100.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            crate::features::MotionModel::default(),
            HeldItem::new(home_spec),
            // Every body carries the per-tick resolved frame + full clusters
            // (ADR 0024; the transit authority reconciles through them) — the
            // ancillary bundle carries both.
            crate::actor::AncillaryMovementBundle::from_scratch(
                ae::BodyClusterScratch::new_with_abilities(
                    ae::Vec2::new(100.0, 100.0),
                    ae::AbilitySet::default(),
                ),
            ),
            {
                let mut c = ActorControl::default();
                c.0.melee_pressed = true; // even pressing attack, it must not blink
                c
            },
        ))
        .id();
    // A possessed ACTOR — NOT a PlayerEntity — holding blink, IS the controlled
    // subject, pressing attack. It must blink.
    let actor_spec = ambition_characters::brain::held_item_by_id(BLINK_ID).unwrap();
    let actor = app
        .world_mut()
        .spawn((
            BodyKinematics {
                pos: ae::Vec2::new(500.0, 500.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            crate::features::MotionModel::default(),
            HeldItem::new(actor_spec),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ae::BodyClusterScratch::new_with_abilities(
                    ae::Vec2::new(500.0, 500.0),
                    ae::AbilitySet::default(),
                ),
            ),
            {
                let mut c = ActorControl::default();
                c.0.melee_pressed = true;
                c.0.facing = 1.0;
                c
            },
        ))
        .id();
    app.insert_resource(ControlledSubject(Some(actor)));
    app.update();

    assert_eq!(
        player_pos(&app, home),
        ae::Vec2::new(100.0, 100.0),
        "the home avatar is NOT the controlled subject — it must not blink",
    );
    assert_eq!(
        player_pos(&app, actor),
        ae::Vec2::new(500.0 + BLINK_DISTANCE, 500.0),
        "the possessed actor (a non-PlayerEntity controlled body) blinks",
    );
}

#[test]
fn no_blink_without_attack_or_with_a_different_item() {
    // Holding blink but not attacking → stays put.
    let mut app = test_app();
    let player = spawn_player_holding(&mut app, BLINK_ID, 1.0);
    app.update();
    assert_eq!(player_pos(&app, player), ae::Vec2::new(300.0, 300.0));
    // Holding the bomb + attacking → blink_system ignores it.
    let mut app2 = test_app();
    let player2 = spawn_player_holding(&mut app2, "bomb", 1.0);
    app2.world_mut()
        .get_mut::<ActorControl>(player2)
        .unwrap()
        .0
        .melee_pressed = true;
    app2.update();
    assert_eq!(player_pos(&app2, player2), ae::Vec2::new(300.0, 300.0));
}
