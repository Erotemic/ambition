//! The in-flight [`PortalProjectile`]: firing a shot and stepping it until it
//! lands on a solid (opening a portal) or fizzles.

use bevy::prelude::*;

use crate::input::ControlFrame;
use crate::platformer_runtime::collision::raycast_solids;
use crate::platformer_runtime::prelude::SpawnScopedExt;
use crate::player::{PlayerEntity, PlayerKinematics, PrimaryPlayer};
use crate::GameWorld;

use super::color::PortalColor;
use super::gun::PortalGun;
use super::placement::pick_aim;
use super::types::{portal_half_extent, Portal, PORTAL_MAX_RANGE, PORTAL_SHOT_SPEED};

/// An in-flight portal shot streaking toward a surface. On contact with a
/// solid it opens a portal of `color`; if it travels too far / leaves the
/// world it fizzles.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalProjectile {
    pub color: PortalColor,
    pub pos: Vec2,
    pub vel: Vec2,
    pub traveled: f32,
}

/// `Attack` fires a portal *shot* of the gun's current color along the aim
/// direction. The shot travels (see `portal_projectile_step`) so the player
/// sees its path before it lands and opens a portal.
pub fn portal_fire_system(
    control: Res<ControlFrame>,
    players: Query<(&PlayerKinematics, &PortalGun), (With<PlayerEntity>, With<PrimaryPlayer>)>,
    mut commands: Commands,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    // Shield+Attack is the "drop the gun" gesture — don't fire on it.
    if !control.attack_pressed || control.shield_held {
        return;
    }
    let Ok((kin, gun)) = players.single() else {
        return;
    };
    if !gun.active {
        return;
    }
    let aim = pick_aim(&control, kin.facing).normalize_or_zero();
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
        PortalProjectile {
            color: gun.next_color,
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
    mut projectiles: Query<(Entity, &mut PortalProjectile)>,
    portals: Query<(Entity, &Portal)>,
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
                if portal.color == proj.color {
                    commands.entity(entity).despawn();
                    sfx.write(crate::audio::SfxMessage::Play {
                        id: ambition_sfx::ids::PORTAL_CLOSE,
                        pos: hit,
                    });
                }
            }
            commands.spawn_room_scoped((
                Portal {
                    color: proj.color,
                    pos: hit + normal * 2.0,
                    normal,
                    half_extent: portal_half_extent(normal),
                },
                Name::new(format!("Portal: {}", proj.color.name())),
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
