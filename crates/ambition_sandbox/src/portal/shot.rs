//! The in-flight [`PortalShot`]: firing a shot and stepping it until it
//! lands on a solid (opening a portal) or fizzles.

use bevy::prelude::*;

use crate::platformer_runtime::collision::raycast_solids;
use crate::platformer_runtime::prelude::SpawnScopedExt;
use crate::player::{BodyKinematics, PlayerEntity, PrimaryPlayer};
use crate::GameWorld;

use super::color::PortalChannel;
use super::gun::PortalGun;
use super::messages::FirePortalGun;
use super::types::{portal_half_extent, PlacedPortal, PORTAL_MAX_RANGE, PORTAL_SHOT_SPEED};

/// An in-flight portal shot streaking toward a surface. On contact with a
/// solid it opens a portal on `channel`; if it travels too far / leaves the
/// world it fizzles. The gun fires its `PortalGunColor` mapped to a channel.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalShot {
    pub channel: PortalChannel,
    pub pos: Vec2,
    pub vel: Vec2,
    pub traveled: f32,
}

/// On a [`FirePortalGun`] intent, fire a portal *shot* of the gun's current
/// color along the message's aim direction. The shot travels (see
/// `portal_projectile_step`) so the player sees its path before it lands and
/// opens a portal. The shield-gated "this is a drop, not a fire" decision is
/// made by the input adapter before it emits the intent.
pub fn portal_fire_system(
    mut fires: MessageReader<FirePortalGun>,
    players: Query<(&BodyKinematics, &PortalGun), (With<PlayerEntity>, With<PrimaryPlayer>)>,
    mut commands: Commands,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    let Some(fire) = fires.read().last().copied() else {
        return;
    };
    let Ok((kin, gun)) = players.single() else {
        return;
    };
    if !gun.active {
        return;
    }
    let aim = fire.aim.normalize_or_zero();
    if aim == Vec2::ZERO {
        return;
    }
    // Punchy fire blast + the airy travel whizz.
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::ids::PORTAL_FIRE,
        pos: kin.pos,
    });
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::ids::PORTAL_TRAVEL,
        pos: kin.pos,
    });
    commands.spawn_room_scoped((
        PortalShot {
            channel: gun.next_color.channel(),
            pos: kin.pos,
            vel: aim * PORTAL_SHOT_SPEED,
            traveled: 0.0,
        },
        Name::new("Portal shot"),
    ));
}

/// Advance portal shots; open a portal on solid contact (the bright warping
/// whoosh) or fizzle past max range / out of bounds (the rejection buzz).
pub fn portal_projectile_step(
    time: Res<crate::WorldTime>,
    world: Res<GameWorld>,
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut PortalShot)>,
    portals: Query<(Entity, &PlacedPortal)>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for (proj_entity, mut proj) in &mut projectiles {
        let step = (proj.vel * dt).length().max(1.0);
        if let Some((hit, normal)) = raycast_solids(&world.0, proj.pos, proj.vel, step, true) {
            // Hit a wall — open (or replace) the portal of this color.
            for (entity, portal) in &portals {
                if portal.channel == proj.channel {
                    commands.entity(entity).despawn();
                    sfx.write(crate::audio::SfxMessage::Play {
                        id: ambition_sfx::ids::PORTAL_CLOSE,
                        pos: hit,
                    });
                }
            }
            commands.spawn_room_scoped((
                PlacedPortal {
                    channel: proj.channel,
                    pos: hit + normal * 2.0,
                    normal,
                    half_extent: portal_half_extent(normal),
                },
                Name::new(format!("Portal: {}", proj.channel.name())),
                // Portals are per-room: a room transition despawns them, so they
                // don't linger and reappear when you leave and come back (#41).
            ));
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_ATTACH,
                pos: hit,
            });
            commands.entity(proj_entity).despawn();
            continue;
        }
        let delta = proj.vel * dt;
        proj.pos += delta;
        proj.traveled += step;
        let oob = proj.pos.x < -64.0
            || proj.pos.y < -64.0
            || proj.pos.x > world.0.size.x + 64.0
            || proj.pos.y > world.0.size.y + 64.0;
        if proj.traveled > PORTAL_MAX_RANGE || oob {
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_INVALID,
                pos: proj.pos,
            });
            commands.entity(proj_entity).despawn();
        }
    }
}
