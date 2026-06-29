//! Gravity-zone lifecycle / room-reset policy and the ambient gravity-flip
//! switch. Extracted from `crate::portal::lifecycle` (Stage 6 follow-up): these
//! are a *gravity mechanic*, not portal behavior, so they own their state here
//! and must not depend on `crate::portal`.

use bevy::prelude::*;

use crate::actor::BodyKinematics;
use crate::actor::{PlayerEntity, PrimaryPlayer};
use crate::physics::GravityField;
use ambition_engine_core::{self as ae, AabbExt};

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
    players: Query<&BodyKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    mut switches: Query<&mut GravityFlipSwitch>,
    mut sfx: MessageWriter<ambition_sfx::SfxMessage>,
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
            sfx.write(ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_POWERUP,
                pos: kin.pos,
            });
            bevy::log::info!(target: "ambition::gravity", "ambient gravity flipped: dir = {:?}", base.dir);
        } else if !overlapping {
            sw.armed = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::BodyBaseSize;
    use crate::physics::{BaseGravity, GravityField};

    fn spawn_player(app: &mut App, pos: Vec2) -> Entity {
        app.world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                BodyKinematics {
                    pos,
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                BodyBaseSize {
                    base_size: Vec2::new(24.0, 40.0),
                },
            ))
            .id()
    }

    #[test]
    fn gravity_switch_flips_on_entry_and_rearms_on_exit() {
        let mut app = App::new();
        app.add_message::<ambition_sfx::SfxMessage>();
        app.init_resource::<GravityField>();
        app.init_resource::<BaseGravity>();
        app.add_systems(Update, gravity_flip_switch_system);
        let player = spawn_player(&mut app, Vec2::new(100.0, 100.0));
        app.world_mut().spawn(GravityFlipSwitch {
            pos: Vec2::new(400.0, 100.0),
            half_extent: Vec2::new(16.0, 220.0),
            armed: true,
        });

        // Not overlapping → gravity stays down.
        app.update();
        assert!(
            app.world().resource::<BaseGravity>().dir.y > 0.0,
            "starts down"
        );

        // Step onto the switch → flips up.
        app.world_mut()
            .get_mut::<BodyKinematics>(player)
            .unwrap()
            .pos = Vec2::new(400.0, 100.0);
        app.update();
        assert!(
            app.world().resource::<BaseGravity>().dir.y < 0.0,
            "stepping on the switch flips ambient gravity up"
        );
        // Staying on it does not re-flip (latched).
        app.update();
        assert!(
            app.world().resource::<BaseGravity>().dir.y < 0.0,
            "stays flipped while on it"
        );

        // Leave, then re-enter → flips back down.
        app.world_mut()
            .get_mut::<BodyKinematics>(player)
            .unwrap()
            .pos = Vec2::new(100.0, 100.0);
        app.update();
        app.world_mut()
            .get_mut::<BodyKinematics>(player)
            .unwrap()
            .pos = Vec2::new(400.0, 100.0);
        app.update();
        assert!(
            app.world().resource::<BaseGravity>().dir.y > 0.0,
            "re-entering flips ambient gravity back down"
        );
    }
}
