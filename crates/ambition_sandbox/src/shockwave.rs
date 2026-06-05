//! Shockwave Slam — a boss-style ground-slam AOE the **player** can wield.
//!
//! This is the first slice of "player wields boss attacks" / the Effect-primitive
//! ability vocabulary (TODO.md). A boss ground-slam and the player's wielded
//! shockwave are the **same primitive**: an
//! [`ActorActionMessage::Special`]`{ spec: ShockwaveSlam }` →
//! [`spawn_shockwave_from_special_messages`] → a World-anchored, faction-tagged
//! [`Hitbox`]. The consumer is **actor-generic**: it reads the emitting actor's
//! position and stamps the hitbox with the **emitter's** faction, so the SAME
//! system serves a boss (Boss faction → damages the player) and the player
//! (Player faction → damages enemies, via the player-faction branch added to
//! `apply_hitbox_damage`). No projectile-pool faction split is involved — the
//! `Hitbox` primitive already carries a faction.
//!
//! The older `spawn_*_from_special_messages` consumers are still boss-query
//! coupled; `ShockwaveSlam` is authored actor-generic from the start, and
//! migrating the rest onto this shape is the Effect-primitive vocabulary item.

use bevy::prelude::*;

use crate::brain::action_set::SpecialActionSpec;
use crate::brain::{ActionRequest, ActorActionMessage};
use crate::engine_core as ae;
use crate::features::{
    ActorFaction, FeatureAabb, FeatureSimEntity, HeldItem, Hitbox, HitboxAnchor, HitboxHits,
    HitboxLifetime,
};
use crate::input::ControlFrame;
use crate::player::{PlayerEntity, PlayerKinematics, PlayerMana, PrimaryPlayer};

/// Held-item id of the shockwave gauntlet.
pub const SHOCKWAVE_ID: &str = "shockwave";

/// Mana the shockwave slam spends per use (out of 100). With the sandbox's fast
/// regen this is feedback (the bar visibly drops), not a hard gate — feel-tune.
const SHOCKWAVE_MANA_COST: f32 = 25.0;

/// Player-wielded shockwave tunings. (A boss using `ShockwaveSlam` authors its
/// own values on the spec; these are the player gauntlet's.)
const SHOCKWAVE_HALF: ae::Vec2 = ae::Vec2::new(120.0, 52.0);
const SHOCKWAVE_DAMAGE: i32 = 4;
const SHOCKWAVE_LIFETIME_S: f32 = 0.18;
const SHOCKWAVE_KNOCKBACK: f32 = 1.3;

/// `Attack` while holding the shockwave gauntlet emits a `ShockwaveSlam`
/// Special so the shared [`spawn_shockwave_from_special_messages`] consumer
/// spawns the AOE from the **player** (player faction). Plain Attack only —
/// `Shield + Attack` is the throw/drop gesture (handled by
/// `item_pickup::throw_held_item_system`, which excludes this id from
/// throw-on-plain-Attack).
pub fn fire_shockwave_system(
    control: Res<ControlFrame>,
    mut players: Query<
        (Entity, &HeldItem, &PlayerKinematics, &mut PlayerMana),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    mut actions: MessageWriter<ActorActionMessage>,
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
    actions.write(ActorActionMessage {
        actor: entity,
        request: ActionRequest::Special {
            spec: SpecialActionSpec::ShockwaveSlam {
                half_extent_x: SHOCKWAVE_HALF.x,
                half_extent_y: SHOCKWAVE_HALF.y,
                damage: SHOCKWAVE_DAMAGE,
                lifetime_s: SHOCKWAVE_LIFETIME_S,
                knockback: SHOCKWAVE_KNOCKBACK,
            },
        },
    });
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_ROCK_HIT,
        pos: kin.pos,
    });
}

/// Actor-generic EFFECTS consumer: for every `ShockwaveSlam` Special, spawn a
/// World-anchored [`Hitbox`] AOE at the **emitting actor's** position, tagged
/// with that actor's faction. Resolves the emitter against the player first,
/// then the feature actors (enemies/bosses that carry an [`ActorFaction`]), so
/// one system serves player- and actor-sourced slams alike.
pub fn spawn_shockwave_from_special_messages(
    mut commands: Commands,
    mut messages: MessageReader<ActorActionMessage>,
    players: Query<&PlayerKinematics, With<PlayerEntity>>,
    features: Query<(&FeatureAabb, &ActorFaction), With<FeatureSimEntity>>,
) {
    for msg in messages.read() {
        let ActionRequest::Special {
            spec:
                SpecialActionSpec::ShockwaveSlam {
                    half_extent_x,
                    half_extent_y,
                    damage,
                    lifetime_s,
                    knockback,
                },
        } = msg.request
        else {
            continue;
        };
        // Resolve the emitter's position + faction generically.
        let (center, faction) = if let Ok(kin) = players.get(msg.actor) {
            (kin.pos, ActorFaction::Player)
        } else if let Ok((aabb, fac)) = features.get(msg.actor) {
            (aabb.center, *fac)
        } else {
            // Emitter has neither player kinematics nor a feature AABB+faction
            // — nothing to anchor the slam to.
            continue;
        };
        commands.spawn((
            Hitbox {
                owner: msg.actor,
                source: faction,
                anchor: HitboxAnchor::World { center },
                half_extent: ae::Vec2::new(half_extent_x, half_extent_y),
                damage,
                knockback_strength: knockback,
            },
            HitboxLifetime {
                remaining_s: lifetime_s,
            },
            HitboxHits::default(),
            Name::new("Shockwave AOE"),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::ActionSet;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.add_message::<ActorActionMessage>();
        app.insert_resource(ControlFrame::default());
        app.add_systems(
            Update,
            (fire_shockwave_system, spawn_shockwave_from_special_messages).chain(),
        );
        app
    }

    fn spawn_player_holding_shockwave(app: &mut App) -> Entity {
        let spec = crate::brain::held_item_by_id(SHOCKWAVE_ID).unwrap();
        app.world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                PlayerKinematics {
                    pos: ae::Vec2::new(100.0, 100.0),
                    vel: ae::Vec2::ZERO,
                    size: ae::Vec2::new(24.0, 40.0),
                    base_size: ae::Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                ActionSet::default(),
                HeldItem::new(spec),
                PlayerMana::default(),
            ))
            .id()
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
        let player = spawn_player_holding_shockwave(&mut app);
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
        spawn_player_holding_shockwave(&mut app);
        app.update();
        assert_eq!(shockwave_count(&mut app), 0);
    }

    #[test]
    fn shockwave_costs_mana_and_is_blocked_when_empty() {
        let mut app = test_app();
        let player = spawn_player_holding_shockwave(&mut app);
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
        // The consumer is actor-generic: a non-player actor (an enemy) emitting
        // the SAME ShockwaveSlam Special gets an Enemy-faction AOE at its own
        // position — proving the player and bosses/enemies share one system.
        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        app.add_systems(Update, spawn_shockwave_from_special_messages);
        let enemy = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                FeatureAabb::new(ae::Vec2::new(300.0, 80.0), ae::Vec2::new(12.0, 16.0)),
                ActorFaction::Enemy,
            ))
            .id();
        app.world_mut().write_message(ActorActionMessage {
            actor: enemy,
            request: ActionRequest::Special {
                spec: SpecialActionSpec::ShockwaveSlam {
                    half_extent_x: 60.0,
                    half_extent_y: 30.0,
                    damage: 3,
                    lifetime_s: 0.2,
                    knockback: 1.0,
                },
            },
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
