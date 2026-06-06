//! Portal lifecycle / persistence policy: clear portals on room reset, despawn
//! gun-orphaned portals, reset gravity on room reset, and the gravity-flip
//! switch that toggles ambient gravity.

use bevy::prelude::*;

use crate::engine_core::{self as ae, AabbExt};
use crate::physics::GravityField;
use crate::player::{PlayerEntity, PlayerKinematics, PrimaryPlayer};

use super::gun::PortalGun;
use super::pickup::PortalGunPickup;
use super::shot::PortalShot;
use super::types::{PlacedPortal, PortalTransitCooldown};

/// Despawn all portals when the room resets / transitions, and clear any body's
/// transit cooldown — portals are per-room, so stale ones from a previous room
/// must not linger and teleport the player unexpectedly.
pub fn clear_portals_on_reset(
    mut commands: Commands,
    mut resets: MessageReader<crate::features::ResetRoomFeaturesEvent>,
    portals: Query<Entity, With<PlacedPortal>>,
    cooldowns: Query<Entity, With<PortalTransitCooldown>>,
) {
    if resets.read().next().is_none() {
        return;
    }
    for entity in &portals {
        commands.entity(entity).despawn();
    }
    for entity in &cooldowns {
        commands.entity(entity).remove::<PortalTransitCooldown>();
    }
}

/// The GUN's portals must not outlive the gun that made them: despawn the
/// gun-pair portals (blue/orange) + in-flight shots when **no** portal gun is
/// present in the room — neither held (`PortalGun`) nor lying as a
/// `PortalGunPickup`. This is the "gun is destroyed" case. Authored pairs (other
/// colors, e.g. a test room's portals) are NOT gun-owned, so they persist even
/// with no gun around. A merely *dropped* gun still exists as a pickup, so its
/// portals persist; leaving the room is handled by [`clear_portals_on_reset`].
pub fn despawn_orphaned_portals(
    mut commands: Commands,
    guns: Query<(), With<PortalGun>>,
    pickups: Query<(), With<PortalGunPickup>>,
    portals: Query<(Entity, &PlacedPortal)>,
    shots: Query<Entity, With<PortalShot>>,
) {
    if !guns.is_empty() || !pickups.is_empty() {
        return;
    }
    for (entity, portal) in &portals {
        if portal.channel.is_gun_pair() {
            commands.entity(entity).despawn();
        }
    }
    for entity in &shots {
        commands.entity(entity).despawn();
    }
}

/// Reset gravity to the default (down) when the room resets, so a flipped /
/// zoned room doesn't carry over.
pub fn reset_gravity_on_room_reset(
    mut resets: MessageReader<crate::features::ResetRoomFeaturesEvent>,
    mut gravity: ResMut<GravityField>,
    mut base: ResMut<crate::physics::BaseGravity>,
) {
    if resets.read().next().is_none() {
        return;
    }
    *gravity = GravityField::default();
    *base = crate::physics::BaseGravity::default();
}

/// A sandbox gravity-flip switch: a tall pressure-plate column the player steps
/// into to flip [`GravityField`] up↔down. Tall so it's reachable from both the
/// floor and the ceiling (after a flip you're on the ceiling — walk back into
/// the column to flip again). `armed` latches so one entry = one flip.
#[derive(Component, Clone, Copy, Debug)]
pub struct GravityFlipSwitch {
    pub pos: Vec2,
    pub half_extent: Vec2,
    /// True when the player is clear of the plate, so the next entry flips.
    pub armed: bool,
}

// The hub gravity flip is now an LDtk-authored `Switch` whose `action` is
// "FlipGravity" (handled in `encounter::systems::update_encounters_from_world`),
// so the old debug-spawned overlap column is gone. The `GravityFlipSwitch`
// component + `gravity_flip_switch_system` below remain only for the unit test
// + any future overlap-style gravity plate; nothing spawns one in-game.

/// Flip the room's **ambient** gravity ([`crate::physics::BaseGravity`]) up↔down
/// when the player steps into a [`GravityFlipSwitch`] (rising-edge latched by
/// `armed`). Flipping the ambient (not the live `GravityField` directly) lets
/// gravity zones override locally while the switch sets the room default.
pub fn gravity_flip_switch_system(
    mut base: ResMut<crate::physics::BaseGravity>,
    players: Query<&PlayerKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    mut switches: Query<&mut GravityFlipSwitch>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    let Ok(kin) = players.single() else {
        return;
    };
    let player_aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
    for mut sw in &mut switches {
        let overlapping = player_aabb.strict_intersects(ae::Aabb::new(sw.pos, sw.half_extent));
        if overlapping && sw.armed {
            // Flip the vertical component of the ambient gravity.
            base.dir = Vec2::new(base.dir.x, -base.dir.y);
            sw.armed = false;
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_POWERUP,
                pos: kin.pos,
            });
            bevy::log::info!(target: "ambition::portal", "ambient gravity flipped: dir = {:?}", base.dir);
        } else if !overlapping {
            sw.armed = true;
        }
    }
}
