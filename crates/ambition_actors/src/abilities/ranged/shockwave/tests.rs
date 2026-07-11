//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::abilities::test_support::spawn_primary_player_holding;
use crate::features::{ActorFaction, CenteredAabb, FeatureSimEntity, Hitbox, HitboxAnchor};

fn test_app() -> App {
    let mut app = App::new();
    app.add_message::<ambition_sfx::SfxMessage>();
    app.add_message::<ambition_vfx::EffectRequest>();
    app.add_systems(
        Update,
        (fire_shockwave_system, ambition_vfx::apply_effects).chain(),
    );
    app
}

/// Stamp `melee_pressed` onto the body's resolved intent (the shockwave
/// system reads the body-generic `ActorControl`, not `PlayerInputFrame` or
/// `Res<ControlFrame>`).
fn press_attack(app: &mut App, player: Entity) {
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
}

fn shockwave_count(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<&Hitbox, ()>()
        .iter(app.world())
        .count()
}

#[test]
fn player_attack_with_shockwave_spawns_a_player_faction_aoe() {
    let mut app = test_app();
    let player = spawn_primary_player_holding(&mut app, SHOCKWAVE_ID);
    press_attack(&mut app, player);
    app.update();
    // Exactly one AOE hitbox, owned by the player and Player-faction so it
    // damages enemies (not the player) through apply_hitbox_damage.
    let mut q = app.world_mut().query::<&Hitbox>();
    let boxes: Vec<&Hitbox> = q.iter(app.world()).collect();
    assert_eq!(boxes.len(), 1, "one shockwave AOE spawned");
    assert_eq!(
        boxes[0].source,
        ambition_vfx::HitSide::Player,
        "AOE carries the player's side"
    );
    assert_eq!(boxes[0].owner, player);
    assert!(
        matches!(boxes[0].anchor, HitboxAnchor::World { .. }),
        "world-anchored AOE"
    );
}

#[test]
fn no_shockwave_without_attack_or_without_the_item() {
    // Holding the item but not attacking → no AOE.
    let mut app = test_app();
    spawn_primary_player_holding(&mut app, SHOCKWAVE_ID);
    app.update();
    assert_eq!(shockwave_count(&mut app), 0);
}

#[test]
fn shockwave_costs_mana_and_is_blocked_when_empty() {
    let mut app = test_app();
    let player = spawn_primary_player_holding(&mut app, SHOCKWAVE_ID);
    // Mana below the cost → the slam is blocked.
    app.world_mut()
        .get_mut::<BodyMana>(player)
        .unwrap()
        .meter
        .current = 5.0;
    press_attack(&mut app, player);
    app.update();
    assert_eq!(shockwave_count(&mut app), 0, "no slam when mana < cost");

    // Refill and fire → one slam, and mana drops by exactly the cost.
    app.world_mut()
        .get_mut::<BodyMana>(player)
        .unwrap()
        .meter
        .current = 100.0;
    app.update();
    assert_eq!(shockwave_count(&mut app), 1, "fires once there's mana");
    let mana = app.world().get::<BodyMana>(player).unwrap().meter.current;
    assert!(
        (mana - (100.0 - SHOCKWAVE_MANA_COST)).abs() < 0.01,
        "mana dropped by the cost: {mana}"
    );
}

#[test]
fn an_actor_emitting_shockwave_gets_an_aoe_of_its_own_faction() {
    // The effect path is actor-generic: a non-player actor (an enemy)
    // emitting the SAME DamageBox effect gets an Enemy-faction AOE at its
    // own position — proving player and bosses/enemies share one path.
    let mut app = App::new();
    app.add_message::<ambition_vfx::EffectRequest>();
    app.add_systems(Update, ambition_vfx::apply_effects);
    let enemy = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            CenteredAabb::new(ae::Vec2::new(300.0, 80.0), ae::Vec2::new(12.0, 16.0)),
            ActorFaction::Enemy,
        ))
        .id();
    app.world_mut().write_message(ambition_vfx::EffectRequest {
        owner: enemy,
        effect: ambition_vfx::Effect::DamageBox(ambition_vfx::DamageBoxEffect {
            center: ae::Vec2::new(300.0, 80.0),
            faction: ambition_vfx::HitSide::Enemy,
            half_extent: ae::Vec2::new(60.0, 30.0),
            damage: 3,
            knockback: 1.0,
            lifetime_s: 0.2,
            name: Some("Shockwave AOE"),
        }),
    });
    app.update();
    let mut q = app.world_mut().query::<&Hitbox>();
    let boxes: Vec<&Hitbox> = q.iter(app.world()).collect();
    assert_eq!(boxes.len(), 1, "the enemy's slam spawns one AOE");
    assert_eq!(
        boxes[0].source,
        ambition_vfx::HitSide::Enemy,
        "AOE carries the enemy's side"
    );
    assert_eq!(boxes[0].owner, enemy);
}
