use super::*;
use crate::test_support::spawn_primary_player_holding;
use ambition_combat::hitbox::{Hitbox, HitboxAnchor};

fn test_app() -> App {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ambition_vfx::EffectRequest>();
    // fire_beam emits Effect::DamageBox; apply_effects spawns the hitbox.
    app.add_systems(
        Update,
        (fire_beam_system, ambition_combat::strike::apply_effects).chain(),
    );
    app
}

fn hitboxes(app: &mut App) -> Vec<Hitbox> {
    app.world_mut()
        .query::<&Hitbox>()
        .iter(app.world())
        .cloned()
        .collect()
}

#[test]
fn attack_with_the_beam_spawns_one_player_faction_line_hitbox() {
    let mut app = test_app();
    let player = spawn_primary_player_holding(&mut app, BEAM_ID);
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    let boxes = hitboxes(&mut app);
    assert_eq!(boxes.len(), 1, "one beam hitbox spawned");
    assert_eq!(
        boxes[0].source,
        ambition_vfx::HitSide::Player,
        "beam carries the player's side so it damages enemies, not the player"
    );
    assert_eq!(boxes[0].owner, player);
    // Default facing (+x), no directional hold → a forward HORIZONTAL lance:
    // long along x, thin along y, offset forward of the player.
    assert!(
        boxes[0].half_extent.x > boxes[0].half_extent.y,
        "horizontal beam is long along x; got {:?}",
        boxes[0].half_extent
    );
    if let HitboxAnchor::World { center } = boxes[0].anchor {
        assert!(center.x > 100.0, "beam reaches forward (+x) of the player");
    } else {
        panic!("beam should be world-anchored");
    }
}

#[test]
fn no_beam_without_attack_or_item() {
    let mut app = test_app();
    spawn_primary_player_holding(&mut app, BEAM_ID);
    app.update(); // no attack pressed
    assert_eq!(hitboxes(&mut app).len(), 0);
}

#[test]
fn beam_costs_mana_and_is_blocked_when_empty() {
    let mut app = test_app();
    let player = spawn_primary_player_holding(&mut app, BEAM_ID);
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
    assert_eq!(hitboxes(&mut app).len(), 0, "no beam when mana < cost");

    app.world_mut()
        .get_mut::<BodyMana>(player)
        .unwrap()
        .meter
        .current = 100.0;
    app.update();
    assert_eq!(hitboxes(&mut app).len(), 1, "fires once there's mana");
    let mana = app.world().get::<BodyMana>(player).unwrap().meter.current;
    assert!(
        (mana - (100.0 - BEAM_MANA_COST)).abs() < 0.01,
        "mana dropped by the cost: {mana}"
    );
}

#[test]
fn vertical_aim_makes_a_tall_thin_beam() {
    // Aim straight up → a tall vertical lance (long along y, thin along x),
    // offset above the player. Engine y grows downward, so "up" is -y.
    let (offset, half) = beam_geometry(ae::Vec2::new(0.0, -1.0), 1.0);
    assert!(
        half.y > half.x,
        "vertical beam is long along y; got {half:?}"
    );
    assert!(offset.y < 0.0, "an up-aimed beam reaches above the player");
}

#[test]
fn beam_geometry_is_c4_equivariant_for_local_aim() {
    let local_aim = ae::Vec2::new(0.0, -1.0);
    let (offset_local, half_local) = beam_geometry(local_aim, 1.0);
    for gravity_dir in [
        ae::Vec2::new(0.0, 1.0),
        ae::Vec2::new(1.0, 0.0),
        ae::Vec2::new(0.0, -1.0),
        ae::Vec2::new(-1.0, 0.0),
    ] {
        let frame = ae::AccelerationFrame::new(gravity_dir);
        let offset_world = frame.to_world(offset_local);
        let half_world = frame.to_world_half(half_local);
        assert!(
            (frame.to_local(offset_world) - offset_local).length() < 0.001,
            "beam offset should round-trip through gravity {gravity_dir:?}"
        );
        if gravity_dir.x.abs() > gravity_dir.y.abs() {
            assert!(
                half_world.x > half_world.y,
                "local vertical beam should become world-horizontal under sideways gravity; got {half_world:?}"
            );
        } else {
            assert!(
                half_world.y > half_world.x,
                "local vertical beam should stay world-vertical under vertical gravity; got {half_world:?}"
            );
        }
    }
}

/// ⭐⭐ A SECOND DRIVEN BODY FIRES ITS OWN BEAM, anchored to ITS position.
/// Same singular-`ControlledSubject` defect as the volley.
#[test]
fn two_driven_bodies_each_fire_their_own_beam() {
    use crate::test_support::spawn_seated_body_holding;
    let mut app = test_app();
    app.insert_resource(ambition_platformer2d_shared_tangle::markers::ControlledSubject(None));
    let a = spawn_seated_body_holding(&mut app, BEAM_ID, 0, "seat_a", ae::Vec2::new(100.0, 100.0));
    let b = spawn_seated_body_holding(&mut app, BEAM_ID, 1, "seat_b", ae::Vec2::new(900.0, 100.0));
    for body in [a, b] {
        app.world_mut()
            .get_mut::<ActorControl>(body)
            .unwrap()
            .0
            .melee_pressed = true;
    }
    app.update();

    let boxes = hitboxes(&mut app);
    let owners: Vec<_> = boxes.iter().map(|hitbox| hitbox.owner).collect();
    assert!(owners.contains(&a), "seat a fired no beam: {owners:?}");
    assert!(owners.contains(&b), "seat b fired no beam: {owners:?}");
    // ⭐ AND EACH LANCE IS WHERE ITS OWN BODY IS. One seat firing twice would
    // satisfy a count; two lances at 100 and 900 could not come from one body.
    let centers: Vec<f32> = boxes
        .iter()
        .filter_map(|hitbox| match hitbox.anchor {
            HitboxAnchor::World { center } => Some(center.x),
            _ => None,
        })
        .collect();
    assert!(
        centers.iter().any(|&x| x > 100.0 && x < 500.0) && centers.iter().any(|&x| x > 900.0),
        "each beam should reach forward of its OWN body; got {centers:?}"
    );
}
