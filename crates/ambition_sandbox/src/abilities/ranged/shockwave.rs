//! Shockwave Slam — a boss-style ground-slam AOE the **player** can wield.
//!
//! The first "player wields a boss attack" slice, now expressed on the effect
//! seam: `Attack` while holding the shockwave gauntlet emits a generic
//! [`crate::effects::EffectRequest`] carrying a `DamageBox` effect anchored at
//! the emitter. The generic [`crate::effects::apply_effects`] consumer spawns
//! the World-anchored, faction-tagged AOE — so the SAME path serves the player
//! (Player faction → damages enemies) and a boss (Boss faction → damages the
//! player, see `boss_encounter::systems` phase-transition slam). No bespoke
//! per-attack consumer: the technique just emits an effect.

use bevy::prelude::*;

use crate::engine_core as ae;
use crate::features::HeldItem;
use crate::input::ControlFrame;
use crate::player::{BodyKinematics, PlayerEntity, PlayerMana, PrimaryPlayer};

/// Held-item id of the shockwave gauntlet.
pub const SHOCKWAVE_ID: &str = "shockwave";

/// Mana the shockwave slam spends per use (out of 100). With the sandbox's fast
/// regen this is feedback (the bar visibly drops), not a hard gate — feel-tune.
const SHOCKWAVE_MANA_COST: f32 = 25.0;

/// Player-wielded shockwave tunings. (A boss authors its own `DamageBox` values
/// at its emit site; these are the player gauntlet's.)
const SHOCKWAVE_HALF: ae::Vec2 = ae::Vec2::new(120.0, 52.0);
const SHOCKWAVE_DAMAGE: i32 = 4;
const SHOCKWAVE_LIFETIME_S: f32 = 0.18;
const SHOCKWAVE_KNOCKBACK: f32 = 1.3;

/// `Attack` while holding the shockwave gauntlet emits a `DamageBox` effect from
/// the **player**. Plain Attack only — `Shield + Attack` is the throw/drop
/// gesture (handled by `item_pickup::throw_held_item_system`, which excludes
/// this id from throw-on-plain-Attack).
pub fn fire_shockwave_system(
    control: Res<ControlFrame>,
    mut players: Query<
        (Entity, &HeldItem, &BodyKinematics, &mut PlayerMana),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    mut effects: MessageWriter<crate::effects::EffectRequest>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    if !control.attack_pressed || control.shield_held {
        return;
    }
    let Ok((entity, held, kin, mut mana)) = players.single_mut() else {
        return;
    };
    if held.spec.id != SHOCKWAVE_ID {
        return;
    }
    // Costs mana — out of mana, no slam (the sandbox's fast regen tops it back up).
    if !mana.meter.try_spend(SHOCKWAVE_MANA_COST) {
        return;
    }
    effects.write(crate::effects::EffectRequest {
        owner: entity,
        effect: crate::effects::Effect::DamageBox(crate::effects::DamageBoxEffect {
            center: kin.pos,
            faction: crate::features::ActorFaction::Player,
            half_extent: SHOCKWAVE_HALF,
            damage: SHOCKWAVE_DAMAGE,
            knockback: SHOCKWAVE_KNOCKBACK,
            lifetime_s: SHOCKWAVE_LIFETIME_S,
            name: Some("Shockwave AOE"),
        }),
    });
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_ROCK_HIT,
        pos: kin.pos,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::test_support::spawn_primary_player_holding;
    use crate::features::{ActorFaction, FeatureAabb, FeatureSimEntity, Hitbox, HitboxAnchor};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.add_message::<crate::effects::EffectRequest>();
        app.insert_resource(ControlFrame::default());
        app.add_systems(
            Update,
            (fire_shockwave_system, crate::effects::apply_effects).chain(),
        );
        app
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
        app.world_mut()
            .resource_mut::<ControlFrame>()
            .attack_pressed = true;
        app.update();
        // Exactly one AOE hitbox, owned by the player and Player-faction so it
        // damages enemies (not the player) through apply_hitbox_damage.
        let mut q = app.world_mut().query::<&Hitbox>();
        let boxes: Vec<&Hitbox> = q.iter(app.world()).collect();
        assert_eq!(boxes.len(), 1, "one shockwave AOE spawned");
        assert_eq!(
            boxes[0].source,
            ActorFaction::Player,
            "AOE carries the player's faction"
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
            .get_mut::<PlayerMana>(player)
            .unwrap()
            .meter
            .current = 5.0;
        app.world_mut()
            .resource_mut::<ControlFrame>()
            .attack_pressed = true;
        app.update();
        assert_eq!(shockwave_count(&mut app), 0, "no slam when mana < cost");

        // Refill and fire → one slam, and mana drops by exactly the cost.
        app.world_mut()
            .get_mut::<PlayerMana>(player)
            .unwrap()
            .meter
            .current = 100.0;
        app.update();
        assert_eq!(shockwave_count(&mut app), 1, "fires once there's mana");
        let mana = app.world().get::<PlayerMana>(player).unwrap().meter.current;
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
        app.add_message::<crate::effects::EffectRequest>();
        app.add_systems(Update, crate::effects::apply_effects);
        let enemy = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                FeatureAabb::new(ae::Vec2::new(300.0, 80.0), ae::Vec2::new(12.0, 16.0)),
                ActorFaction::Enemy,
            ))
            .id();
        app.world_mut()
            .write_message(crate::effects::EffectRequest {
                owner: enemy,
                effect: crate::effects::Effect::DamageBox(crate::effects::DamageBoxEffect {
                    center: ae::Vec2::new(300.0, 80.0),
                    faction: ActorFaction::Enemy,
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
            ActorFaction::Enemy,
            "AOE carries the enemy's faction"
        );
        assert_eq!(boxes[0].owner, enemy);
    }
}
