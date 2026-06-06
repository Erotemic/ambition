//! The world-resting [`PortalGunPickup`] and the pickup/drop systems that grant
//! or relinquish the player's [`super::gun::PortalGun`].

use bevy::prelude::*;

use crate::brain::ActionSet;
use crate::engine_core::{self as ae, AabbExt};
use crate::input::ControlFrame;
use crate::item_pickup::StashedActionSet;
use crate::platformer_runtime::prelude::SpawnScopedExt;
use crate::player::{PlayerEntity, PlayerKinematics, PrimaryPlayer};

use super::gun::PortalGun;

/// A portal gun resting in the world. Walking onto it and pressing `Attack`
/// activates the player's (inactive) portal gun — "pick up the portal gun in
/// a room". Kept distinct from `item_pickup::GroundItem` because the portal
/// gun's ability is the `PortalGun` component, not a `HeldItemSpec` verb.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalGunPickup {
    pub pos: Vec2,
    pub half_extent: Vec2,
    /// Seconds before this pickup can be grabbed. A *just-dropped* gun arms
    /// after a short delay so the same `Attack` press that dropped it (and the
    /// next overlapping frame) can't immediately re-grab it. World-placed
    /// pickups spawn already armed (`0.0`).
    pub arm_timer: f32,
}

// The portal gun is now an LDtk-authored `PortalGunSpawn` entity (spawned at
// room load via `spawn_room_feature_entities`); the old debug near-player
// spawner is retired.

/// Tick down each pickup's [`PortalGunPickup::arm_timer`] so a just-dropped gun
/// becomes grabbable after the short delay. Always runs (cheap; at most a
/// couple of pickups).
pub fn arm_portal_pickups(time: Res<crate::WorldTime>, mut pickups: Query<&mut PortalGunPickup>) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for mut pickup in &mut pickups {
        if pickup.arm_timer > 0.0 {
            pickup.arm_timer = (pickup.arm_timer - dt).max(0.0);
        }
    }
}

/// `Shield + Attack` drops the held portal gun: removes the `PortalGun` (so
/// `Attack` stops firing portals) and leaves a `PortalGunPickup` at the
/// player's feet to grab again. Only when not also holding a throwable item
/// (that throw takes precedence).
pub fn drop_portal_gun_system(
    control: Res<ControlFrame>,
    mut commands: Commands,
    mut players: Query<
        (
            Entity,
            &PlayerKinematics,
            &mut ActionSet,
            Option<&StashedActionSet>,
        ),
        (
            With<PlayerEntity>,
            With<PrimaryPlayer>,
            With<PortalGun>,
            Without<crate::features::HeldItem>,
        ),
    >,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    if !(control.shield_held && control.attack_pressed) {
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
            // Attack press (and the immediately-overlapping next frame) can't
            // re-grab it — that was the "can't drop the portal gun" bug.
            pos: kin.pos + Vec2::new(facing * 44.0, 0.0),
            half_extent: Vec2::splat(20.0),
            arm_timer: 0.35,
        },
        Name::new("Portal gun pickup"),
    ));
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::ids::PORTAL_FIZZLE,
        pos: kin.pos,
    });
}

/// `Attack` while overlapping the [`PortalGunPickup`] grants the player an
/// (active) `PortalGun` and consumes the pickup. The gun is a **single item**:
/// it doesn't exist until you pick it up (no separate granted-but-inactive
/// component) — picking up the one world item *is* getting the portal gun.
pub fn pickup_portal_gun_system(
    control: Res<ControlFrame>,
    mut commands: Commands,
    mut players: Query<
        (Entity, &PlayerKinematics, &mut ActionSet),
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
            With<crate::features::HeldItem>,
        ),
    >,
    pickups: Query<(Entity, &PortalGunPickup)>,
    mut owned: Option<ResMut<crate::items::OwnedItems>>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    if !control.attack_pressed || !already_have.is_empty() || !holding_item.is_empty() {
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
                owned.grant(crate::items::Item::PortalGun, 1);
                owned.set_equipped(Some(crate::items::Item::PortalGun));
            }
            commands.entity(entity).despawn();
            // Rising sci-fi charge-up as the device wakes.
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_POWERUP,
                pos: kin.pos,
            });
            bevy::log::info!(target: "ambition::portal", "picked up the portal gun");
            break;
        }
    }
}
