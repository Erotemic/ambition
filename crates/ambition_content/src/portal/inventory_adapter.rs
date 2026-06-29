//! Ambition inventory binding for the portal gun.
//!
//! Portal core owns the [`PortalGun`](ambition_gameplay_core::portal::PortalGun) component and the
//! [`PortalGunPickup`](ambition_gameplay_core::portal::PortalGunPickup) body, but the *policy* of
//! how the Ambition player acquires / relinquishes the gun is content-specific:
//!
//! - equipping replaces the player's melee `Attack` (the same
//!   [`StashedActionSet`] path the held axe / gun-sword use), so `Attack` fires
//!   portals instead of swinging;
//! - acquiring reflects ownership + equipped state into the 24-item
//!   [`OwnedItems`] roster so the OoT menu shows it.
//!
//! These translate the reusable [`PickUpPortalGun`] / [`DropPortalGun`] intents
//! (emitted by the input adapter) and the [`PortalGunEquipped`] outcome into
//! Ambition item state. The reusable portal core never imports `ambition_gameplay_core::items`,
//! `StashedActionSet`, or `HeldItem`.

use bevy::prelude::*;

use ambition_characters::brain::ActionSet;
use ambition_engine_core::{self as ae, AabbExt};
#[cfg(test)]
use ambition_gameplay_core::actor::BodyBaseSize;
use ambition_gameplay_core::actor::{BodyKinematics, PlayerEntity, PrimaryPlayer};
use ambition_gameplay_core::items::pickup::StashedActionSet;
use ambition_gameplay_core::items::{Item, OwnedItems};
use ambition_gameplay_core::platformer_runtime::prelude::SpawnScopedExt;
use ambition_gameplay_core::portal::{
    DropPortalGun, PickUpPortalGun, PortalGun, PortalGunEquipped, PortalGunPickup,
};

/// Facade: the menu-driven equip/unequip pair moved to
/// [`ambition_gameplay_core::items::pickup`] (their bodies are pure item-equip
/// machinery, the twins of `equip_held_spec` / `unequip_held`); the
/// content adapter keeps the roster-policy systems below.
pub use ambition_gameplay_core::items::pickup::{equip_portal_gun, unequip_portal_gun};

/// On a [`DropPortalGun`] intent, drop the held portal gun: remove the
/// `PortalGun` (so `Attack` stops firing portals), restore the stashed melee,
/// and leave a `PortalGunPickup` at the player's feet to grab again. Only when
/// not also holding a throwable item (that throw takes precedence — the gesture
/// recognition lives in the input adapter, but the held-item exclusion is an
/// Ambition inventory rule, so it stays here).
pub fn drop_portal_gun_system(
    mut drops: MessageReader<DropPortalGun>,
    mut commands: Commands,
    mut players: Query<
        (
            Entity,
            &BodyKinematics,
            &mut ActionSet,
            Option<&StashedActionSet>,
        ),
        (
            With<PlayerEntity>,
            With<PrimaryPlayer>,
            With<PortalGun>,
            Without<ambition_gameplay_core::features::HeldItem>,
        ),
    >,
    mut sfx: MessageWriter<ambition_gameplay_core::audio::SfxMessage>,
) {
    if drops.read().next().is_none() {
        return;
    }
    let Ok((player, kin, mut action_set, stashed)) = players.single_mut() else {
        return;
    };
    commands.entity(player).remove::<PortalGun>();
    // Restore the swing the gun replaced (same path the held items use).
    if let Some(stash) = stashed {
        *action_set = stash.0.clone();
    }
    commands.entity(player).remove::<StashedActionSet>();
    let facing = if kin.facing >= 0.0 { 1.0 } else { -1.0 };
    commands.spawn_room_scoped((
        PortalGunPickup {
            // Drop it a bit ahead and arm it after a short delay so this same
            // drop press (and the immediately-overlapping next frame) can't
            // re-grab it — that was the "can't drop the portal gun" bug.
            pos: kin.pos + Vec2::new(facing * 44.0, 0.0),
            half_extent: Vec2::splat(20.0),
            arm_timer: 0.35,
        },
        Name::new("Portal gun pickup"),
    ));
    sfx.write(ambition_gameplay_core::audio::SfxMessage::Play {
        id: ambition_sfx::ids::PORTAL_FIZZLE,
        pos: kin.pos,
    });
}

/// On a [`PickUpPortalGun`] intent, grant the player an (active) `PortalGun` if
/// they overlap an armed `PortalGunPickup`, consume the pickup, stash the melee
/// (so `Attack` fires portals), and reflect the grant into [`OwnedItems`]. The
/// gun is a **single item**: it doesn't exist until you pick it up — picking up
/// the one world item *is* getting the portal gun. Emits [`PortalGunEquipped`].
pub fn pickup_portal_gun_system(
    mut picks: MessageReader<PickUpPortalGun>,
    mut commands: Commands,
    mut players: Query<
        (Entity, &BodyKinematics, &mut ActionSet),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    already_have: Query<(), (With<PlayerEntity>, With<PrimaryPlayer>, With<PortalGun>)>,
    // One item at a time (Smash-style): can't grab the portal gun while holding
    // a ground item (axe / gun-sword / javelin).
    holding_item: Query<
        (),
        (
            With<PlayerEntity>,
            With<PrimaryPlayer>,
            With<ambition_gameplay_core::features::HeldItem>,
        ),
    >,
    pickups: Query<(Entity, &PortalGunPickup)>,
    mut owned: Option<ResMut<OwnedItems>>,
    mut equipped: MessageWriter<PortalGunEquipped>,
    mut sfx: MessageWriter<ambition_gameplay_core::audio::SfxMessage>,
) {
    if picks.read().next().is_none() || !already_have.is_empty() || !holding_item.is_empty() {
        return;
    }
    let Ok((player, kin, mut action_set)) = players.single_mut() else {
        return;
    };
    let player_aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
    for (entity, pickup) in &pickups {
        if pickup.arm_timer > 0.0 {
            continue;
        }
        if player_aabb.strict_intersects(ae::Aabb::new(pickup.pos, pickup.half_extent)) {
            commands.entity(player).insert(PortalGun {
                active: true,
                ..PortalGun::default()
            });
            // Equipping the portal gun REPLACES the attack: stash the player's
            // ActionSet and clear the melee swing so Attack fires portals
            // instead of swinging (same StashedActionSet path the held axe /
            // gun-sword use — unified held-item attack replacement).
            commands
                .entity(player)
                .insert(StashedActionSet(action_set.clone()));
            action_set.melee = None;
            // Reflect the portal gun into the 24-item catalog so the OoT menu
            // shows it as owned + equipped.
            if let Some(owned) = owned.as_deref_mut() {
                owned.grant(Item::PortalGun, 1);
                owned.set_equipped(Some(Item::PortalGun));
            }
            commands.entity(entity).despawn();
            equipped.write(PortalGunEquipped { player });
            // Rising sci-fi charge-up as the device wakes.
            sfx.write(ambition_gameplay_core::audio::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_POWERUP,
                pos: kin.pos,
            });
            bevy::log::info!(target: "ambition::portal", "picked up the portal gun");
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_gameplay_core::portal::arm_portal_pickups;

    fn spawn_player(app: &mut App, pos: Vec2, facing: f32) -> Entity {
        app.world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                BodyKinematics {
                    pos,
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    facing,
                },
                BodyBaseSize {
                    base_size: Vec2::new(24.0, 40.0),
                },
                PortalGun::default(),
                ActionSet::default(),
            ))
            .id()
    }

    #[test]
    fn picking_up_the_portal_gun_activates_it() {
        let mut app = App::new();
        app.add_message::<ambition_gameplay_core::audio::SfxMessage>();
        app.add_message::<PickUpPortalGun>();
        app.add_message::<PortalGunEquipped>();
        app.add_systems(Update, pickup_portal_gun_system);
        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                BodyKinematics {
                    pos: Vec2::new(50.0, 50.0),
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                BodyBaseSize {
                    base_size: Vec2::new(24.0, 40.0),
                },
                ActionSet::default(),
                // No PortalGun yet — the single pickup item grants it.
            ))
            .id();
        app.world_mut().spawn(PortalGunPickup {
            pos: Vec2::new(50.0, 50.0),
            half_extent: Vec2::splat(20.0),
            arm_timer: 0.0,
        });
        assert!(app.world().get::<PortalGun>(player).is_none());

        app.world_mut().write_message(PickUpPortalGun);
        app.update();
        assert!(
            app.world()
                .get::<PortalGun>(player)
                .is_some_and(|g| g.active),
            "a pickup intent while overlapping grants the active gun"
        );
        let remaining = {
            let mut q = app.world_mut().query::<&PortalGunPickup>();
            q.iter(app.world()).count()
        };
        assert_eq!(remaining, 0, "the pickup is consumed");
    }

    #[test]
    fn dropped_portal_gun_arms_before_it_can_be_regrabbed() {
        let mut app = App::new();
        app.add_message::<ambition_gameplay_core::audio::SfxMessage>();
        app.add_message::<DropPortalGun>();
        app.add_message::<PickUpPortalGun>();
        app.add_message::<PortalGunEquipped>();
        app.insert_resource(ambition_platformer_primitives::time::SimDt { dt: 1.0 / 60.0 });
        app.add_systems(
            Update,
            (
                drop_portal_gun_system,
                arm_portal_pickups,
                pickup_portal_gun_system,
            )
                .chain(),
        );
        let player = spawn_player(&mut app, Vec2::new(100.0, 100.0), 1.0);

        // Drop intent drops the gun.
        app.world_mut().write_message(DropPortalGun);
        app.update();
        assert!(
            app.world().get::<PortalGun>(player).is_none(),
            "a drop intent should drop the portal gun"
        );

        // Move the player directly onto the dropped pickup so only the arm
        // timer (not distance) guards against a re-grab.
        let pickup_pos = {
            let mut q = app.world_mut().query::<&PortalGunPickup>();
            q.iter(app.world())
                .next()
                .expect("a pickup was dropped")
                .pos
        };
        app.world_mut()
            .get_mut::<BodyKinematics>(player)
            .unwrap()
            .pos = pickup_pos;

        // Immediately a pickup intent while overlapping — the freshly-dropped
        // pickup is still arming, so it must NOT be re-grabbed (the bug).
        app.world_mut().write_message(PickUpPortalGun);
        app.update();
        assert!(
            app.world().get::<PortalGun>(player).is_none(),
            "an armed (just-dropped) pickup can't be re-grabbed on the next intent"
        );

        // Let it disarm, then a pickup intent picks it back up.
        for _ in 0..30 {
            app.update();
        }
        app.world_mut().write_message(PickUpPortalGun);
        app.update();
        assert!(
            app.world().get::<PortalGun>(player).is_some(),
            "once disarmed, a pickup intent while overlapping re-grabs the gun"
        );
    }
}
