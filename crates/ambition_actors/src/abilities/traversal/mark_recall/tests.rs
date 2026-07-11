//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

fn test_app() -> App {
    let mut app = App::new();
    app.add_message::<ambition_sfx::SfxMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.add_message::<crate::features::HitEvent>();
    app.add_systems(Update, mark_recall_system);
    app
}

fn spawn_player_holding(app: &mut App, id: &str, pos: ae::Vec2) -> Entity {
    crate::abilities::test_support::spawn_primary_player_holding_at(app, id, pos, 1.0)
}

fn press(app: &mut App, player: Entity, attack: bool, blink: bool) {
    let mut control = app.world_mut().get_mut::<ActorControl>(player).unwrap();
    control.0.melee_pressed = attack;
    control.0.blink_pressed = blink;
    control.0.shield_held = false;
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
fn recall_emits_a_player_side_shockwave_at_the_mark() {
    let mut app = test_app();
    app.init_resource::<CapturedHits>();
    app.add_systems(
        bevy::prelude::Update,
        capture_hits.after(mark_recall_system),
    );
    let player = spawn_player_holding(&mut app, MARK_RECALL_ID, ae::Vec2::new(200.0, 80.0));
    press(&mut app, player, true, false); // mark at (200,80) — no hit yet
    app.update();
    app.world_mut()
        .get_mut::<BodyKinematics>(player)
        .unwrap()
        .pos = ae::Vec2::new(900.0, 50.0);
    press(&mut app, player, false, true); // recall -> shockwave at the mark
    app.update();
    let hits = &app.world().resource::<CapturedHits>().0;
    assert_eq!(hits.len(), 1, "one shockwave on recall");
    let cx = (hits[0].volume.bounds().min.x + hits[0].volume.bounds().max.x) * 0.5;
    assert!((cx - 200.0).abs() < 1.0, "shockwave centered on the mark");
    assert!(
        matches!(
            hits[0].source,
            crate::features::HitSource::PlayerSlash { .. }
        ),
        "player-side so it spares the player",
    );
}

#[test]
fn attack_sets_a_mark_then_blink_recalls_to_it() {
    let mut app = test_app();
    let player = spawn_player_holding(&mut app, MARK_RECALL_ID, ae::Vec2::new(100.0, 100.0));
    // Drop a mark where we stand.
    press(&mut app, player, true, false);
    app.update();
    assert_eq!(
        app.world().get::<PlayerMark>(player).and_then(|m| m.pos),
        Some(ae::Vec2::new(100.0, 100.0)),
        "Attack stored a mark at the player's position",
    );
    // Wander far away, then recall.
    app.world_mut()
        .get_mut::<BodyKinematics>(player)
        .unwrap()
        .pos = ae::Vec2::new(900.0, 50.0);
    press(&mut app, player, false, true);
    app.update();
    assert_eq!(
        player_pos(&app, player),
        ae::Vec2::new(100.0, 100.0),
        "Blink recalled the player to the mark",
    );
}

#[test]
fn re_marking_moves_the_single_mark() {
    let mut app = test_app();
    let player = spawn_player_holding(&mut app, MARK_RECALL_ID, ae::Vec2::new(10.0, 10.0));
    press(&mut app, player, true, false);
    app.update(); // mark at (10,10)
    app.world_mut()
        .get_mut::<BodyKinematics>(player)
        .unwrap()
        .pos = ae::Vec2::new(400.0, 20.0);
    press(&mut app, player, true, false);
    app.update(); // re-mark at (400,20) — should replace, not add a second
    assert_eq!(
        app.world().get::<PlayerMark>(player).and_then(|m| m.pos),
        Some(ae::Vec2::new(400.0, 20.0)),
        "the single mark moved to the newest drop",
    );
}

#[test]
fn blink_without_a_mark_is_a_no_op() {
    let mut app = test_app();
    let player = spawn_player_holding(&mut app, MARK_RECALL_ID, ae::Vec2::new(900.0, 50.0));
    press(&mut app, player, false, true);
    app.update();
    assert_eq!(
        player_pos(&app, player),
        ae::Vec2::new(900.0, 50.0),
        "no mark set → Blink does not teleport",
    );
}

#[test]
fn a_different_held_item_never_marks() {
    // Holding the bomb (also a pure throwable) must not trip the mark logic.
    let mut app = test_app();
    let player = spawn_player_holding(&mut app, "bomb", ae::Vec2::new(100.0, 100.0));
    press(&mut app, player, true, false);
    app.update();
    assert!(
        app.world().get::<PlayerMark>(player).is_none(),
        "Attack while holding a different item sets no mark",
    );
}
