//! Ambition inventory binding for the portal gun.
//!
//! Portal core owns the [`PortalGun`](ambition_portal2d::PortalGun) component and the
//! [`PortalGunPickup`](ambition_portal2d::PortalGunPickup) body, but the *policy* of
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
//! Ambition item state. The reusable portal core never imports `ambition_platformer2d_actor_monolith::items`,
//! `StashedActionSet`, or `HeldItem`.

use bevy::prelude::*;

use ambition_characters::brain::ActionSet;
use ambition_items::{Item, OwnedItems};
use ambition_platformer2d_actor_monolith::features::HeldItem;
use ambition_platformer2d_actor_monolith::items::pickup::StashedActionSet;
use ambition_platformer2d_actor_monolith::platformer_runtime::prelude::SpawnScopedExt;
#[cfg(test)]
use ambition_platformer2d_core::BodyBaseSize;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use ambition_portal2d::{
    DropPortalGun, PickUpPortalGun, PortalGun, PortalGunEquipped, PortalGunPickup,
};

/// Facade: the equip/unequip pair lives in
/// [`ambition_platformer2d_actor_monolith::items::pickup`] (their bodies are pure item-equip
/// machinery, the twins of `equip_held_spec` / `unequip_held`); the content
/// adapter keeps the world-side systems below — WHEN a body may take or release
/// the gun, and what it leaves on the floor.
///
/// Both call them now, so "release the gun" has one body and the roster cannot be left behind by
/// one caller.
pub use ambition_platformer2d_actor_monolith::items::pickup::{
    equip_portal_gun, unequip_portal_gun,
};

/// On a [`DropPortalGun`] intent, drop the held portal gun: remove the
/// `PortalGun` (so `Attack` stops firing portals), restore the stashed melee,
/// and leave a `PortalGunPickup` at the player's feet to grab again. Only when
/// not also holding a throwable item (that throw takes precedence — the gesture
/// recognition lives in the input adapter, but the held-item exclusion is an
/// Ambition inventory rule, so it stays here).
pub fn drop_portal_gun_system(
    mut drops: MessageReader<DropPortalGun>,
    mut commands: Commands,
    controlled: Option<Res<ControlledSubject>>,
    mut holders: Query<
        (
            &BodyKinematics,
            &mut ActionSet,
            Option<&StashedActionSet>,
            &mut ambition_characters::control::ActorControl,
        ),
        (With<PortalGun>, Without<HeldItem>),
    >,
    primary_fallback: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    mut owned: Option<ResMut<OwnedItems>>,
    mut sfx: ambition_sfx::SfxWriter,
) {
    if drops.read().next().is_none() {
        return;
    }
    let Some(player) = controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary_fallback.single().ok())
    else {
        return;
    };
    let Ok((kin, mut action_set, stashed, mut actor_control)) = holders.get_mut(player) else {
        return;
    };
    // Committed: this body IS dropping its gun, so the press is answered.
    actor_control.0.melee_pressed = false;
    // CUSTODY, one operation: detach the gun, restore the swing it replaced, and
    // clear the catalog slot. The same release the inventory menu performs.
    unequip_portal_gun(
        &mut commands,
        player,
        &mut action_set,
        stashed,
        owned.as_deref_mut(),
    );
    let facing = if kin.facing >= 0.0 { 1.0 } else { -1.0 };
    commands.spawn_room_scoped((
        PortalGunPickup {
            pos: kin.pos + Vec2::new(facing * 44.0, 0.0),
            half_extent: Vec2::splat(20.0),
            arm_timer: 0.35,
        },
        Name::new("Portal gun pickup"),
    ));
    sfx.write(ambition_sfx::SfxMessage::Play {
        id: ambition_sfx::ids::PORTAL_FIZZLE,
        pos: kin.pos,
    });
}

/// On a [`PickUpPortalGun`] intent, grant the player an (active) `PortalGun` if
/// they overlap an armed `PortalGunPickup`, consume the pickup, stash the melee
/// (so `Attack` fires portals), and reflect the grant into [`OwnedItems`]. The
/// gun is a single item: it doesn't exist until you pick it up — picking up
/// the one world item *is* getting the portal gun. Emits [`PortalGunEquipped`].
pub fn pickup_portal_gun_system(
    mut picks: MessageReader<PickUpPortalGun>,
    mut commands: Commands,
    controlled: Option<Res<ControlledSubject>>,
    // The controlled body attempts the pickup. `Has` flags gate on ITS state: it
    // can't grab a gun it already holds, nor while holding a ground item.
    mut bodies: Query<(
        &BodyKinematics,
        &mut ActionSet,
        Has<PortalGun>,
        Has<HeldItem>,
    )>,
    primary_fallback: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    pickups: Query<(Entity, &PortalGunPickup)>,
    mut owned: Option<ResMut<OwnedItems>>,
    mut equipped: MessageWriter<PortalGunEquipped>,
    mut sfx: ambition_sfx::SfxWriter,
) {
    if picks.read().next().is_none() {
        return;
    }
    let Some(player) = controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary_fallback.single().ok())
    else {
        return;
    };
    let Ok((kin, mut action_set, has_gun, has_held)) = bodies.get_mut(player) else {
        return;
    };
    // Already holding the gun, or holding a ground item → no pickup.
    if has_gun || has_held {
        return;
    }
    let player_aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
    for (entity, pickup) in &pickups {
        if pickup.arm_timer > 0.0 {
            continue;
        }
        if player_aabb.strict_intersects(ae::Aabb::new(pickup.pos, pickup.half_extent)) {
            // ACQUISITION: the gun does not exist until somebody picks it up, so
            // grabbing the one world item IS how you come to own it. Separate
            // from the custody transfer below — owning it and holding it are
            // different facts, and re-equipping from the menu must not mint a
            // second gun.
            if let Some(owned) = owned.as_deref_mut() {
                owned.grant(Item::PortalGun, 1);
            }
            // CUSTODY: the ONE take-custody operation, shared with the inventory menu.
            equip_portal_gun(&mut commands, player, &mut action_set, owned.as_deref_mut());
            commands.entity(entity).despawn();
            equipped.write(PortalGunEquipped { player });
            // Rising sci-fi charge-up as the device wakes.
            sfx.write(ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_POWERUP,
                pos: kin.pos,
            });
            bevy::log::info!(target: "ambition_platformer2d::portal", "picked up the portal gun");
            break;
        }
    }
}

#[cfg(test)]
mod tests;
