//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::abilities::test_support::spawn_primary_player_holding;
use crate::actor::{BodyKinematics, BodyMana};

fn test_app() -> App {
    let mut app = App::new();
    app.add_message::<ambition_sfx::SfxMessage>();
    app.add_message::<crate::features::HitEvent>();
    app.add_systems(Update, fire_dive_system);
    app
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
fn dive_lunges_the_player_forward_and_cuts_a_corridor() {
    let mut app = test_app();
    app.init_resource::<CapturedHits>();
    app.add_systems(Update, capture_hits.after(fire_dive_system));
    let player = spawn_primary_player_holding(&mut app, DIVE_ID);
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    // No world → no walls → full lunge along facing (+x).
    let pos = app.world().get::<BodyKinematics>(player).unwrap().pos;
    assert!(
        (pos.x - (100.0 + DIVE_LUNGE)).abs() < 0.01,
        "player lunged a full DIVE_LUNGE forward: {pos:?}"
    );
    let hits = &app.world().resource::<CapturedHits>().0;
    assert_eq!(hits.len(), 1, "one corridor hit emitted");
    assert_eq!(hits[0].damage, DIVE_DAMAGE);
    assert!(
        matches!(
            hits[0].source,
            crate::features::HitSource::PlayerSlash { .. }
        ),
        "player-side source so it spares the player",
    );
    // The corridor spans the dash: from start (100) to landing (240) along x.
    assert!(
        hits[0].volume.bounds().min.x <= 100.0
            && hits[0].volume.bounds().max.x >= 100.0 + DIVE_LUNGE,
        "corridor covers start..landing: {:?}",
        hits[0].volume
    );
}

#[test]
fn downward_dive_does_not_embed_in_the_floor() {
    // Regression (same class as the blink fix): a vertical lunge must clamp by
    // the body's half-HEIGHT, not half-width, or a down dive embeds in the floor.
    let mut app = test_app();
    let player = spawn_primary_player_holding(&mut app, DIVE_ID); // (100,100), 24x40
    app.insert_resource(ambition_engine_core::RoomGeometry(ae::World::new(
        "test",
        ae::Vec2::new(600.0, 600.0),
        ae::Vec2::new(100.0, 100.0),
        vec![ae::Block::solid(
            "floor",
            ae::Vec2::new(0.0, 200.0),
            ae::Vec2::new(600.0, 400.0),
        )],
    )));
    {
        let mut control = app.world_mut().get_mut::<ActorControl>(player).unwrap();
        control.0.melee_pressed = true;
        control.0.aim = ae::Vec2::new(0.0, 1.0); // brain-resolved local aim: down
    }
    app.update();
    let pos = app.world().get::<BodyKinematics>(player).unwrap().pos;
    assert!(
        pos.y + 20.0 <= 200.0 + 1e-3,
        "downward dive embedded the body in the floor: bottom={}, floor top=200",
        pos.y + 20.0,
    );
    assert!(
        pos.y > 100.0,
        "the dive should still carry the player downward"
    );
}

#[test]
fn no_dive_without_attack_or_item() {
    let mut app = test_app();
    app.init_resource::<CapturedHits>();
    app.add_systems(Update, capture_hits.after(fire_dive_system));
    let player = spawn_primary_player_holding(&mut app, DIVE_ID);
    app.update(); // no attack pressed
    assert_eq!(app.world().resource::<CapturedHits>().0.len(), 0);
    assert_eq!(
        app.world().get::<BodyKinematics>(player).unwrap().pos.x,
        100.0,
        "no lunge without an attack press"
    );
}

#[test]
fn dive_costs_mana_and_is_blocked_when_empty() {
    let mut app = test_app();
    app.init_resource::<CapturedHits>();
    app.add_systems(Update, capture_hits.after(fire_dive_system));
    let player = spawn_primary_player_holding(&mut app, DIVE_ID);
    app.world_mut()
        .get_mut::<BodyMana>(player)
        .unwrap()
        .meter
        .current = 5.0;
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    assert_eq!(
        app.world().resource::<CapturedHits>().0.len(),
        0,
        "no dive when mana < cost"
    );
    assert_eq!(
        app.world().get::<BodyKinematics>(player).unwrap().pos.x,
        100.0,
        "and no lunge either"
    );

    app.world_mut()
        .get_mut::<BodyMana>(player)
        .unwrap()
        .meter
        .current = 100.0;
    app.update();
    assert_eq!(
        app.world().resource::<CapturedHits>().0.len(),
        1,
        "fires once there's mana"
    );
}

#[test]
fn dive_dir_snaps_to_the_dominant_axis() {
    // Engine y grows downward, so "up" is -y.
    assert_eq!(
        dive_dir(ae::Vec2::new(0.0, -1.0), 1.0),
        ae::Vec2::new(0.0, -1.0)
    );
    assert_eq!(
        dive_dir(ae::Vec2::new(1.0, 0.0), 1.0),
        ae::Vec2::new(1.0, 0.0)
    );
    // Null aim falls back to facing.
    assert_eq!(dive_dir(ae::Vec2::ZERO, -1.0), ae::Vec2::new(-1.0, 0.0));
    // Dominant axis wins on a diagonal.
    assert_eq!(
        dive_dir(ae::Vec2::new(0.3, -0.9), 1.0),
        ae::Vec2::new(0.0, -1.0)
    );
}

#[test]
fn dive_corridor_is_a_thin_rectangle_spanning_the_dash() {
    // A horizontal dash: long along x, thin along y.
    let c = dive_corridor(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(240.0, 100.0));
    assert!(c.min.x <= 100.0 && c.max.x >= 240.0, "spans the dash on x");
    let half_y = (c.max.y - c.min.y) * 0.5;
    let half_x = (c.max.x - c.min.x) * 0.5;
    assert!(
        half_x > half_y,
        "horizontal corridor is long along x: {c:?}"
    );
}
