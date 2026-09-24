//! Compatibility projectile for portal-gun-style placement.
//!
//! Kept apart from the transit core: portals can also be opened by
//! authoring, scripts, or moving emitters. This module consumes the generic
//! [`PortalFireIntent`].
//!
//! The pure [`step_portal_shot`] raycasts against a
//! [`SolidWorldQuery`](ambition_platformer2d_core::cast::SolidWorldQuery) and
//! the world bounds, so portal core never reads `RoomGeometry`. The host
//! portal adapter owns `RoomGeometry` and calls the helper.

use bevy::prelude::*;

use ambition_platformer2d_core::cast::{raycast_solids, SolidWorldQuery};
use ambition_platformer2d_shared_tangle::prelude::SpawnScopedExt;

use super::color::PortalChannel;
use super::messages::{PortalFireIntent, PortalShotFired};
use super::types::{PORTAL_MAX_RANGE, PORTAL_SHOT_SPEED};

/// An in-flight portal-opening shot. On contact with a solid it opens a portal
/// on `channel`; if it travels too far or leaves the world, it fizzles.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalShot {
    pub channel: PortalChannel,
    pub pos: Vec2,
    pub vel: Vec2,
    pub traveled: f32,
}

/// For each [`PortalFireIntent`], fire a portal shot of its `channel` from
/// `origin` along `dir`. Any emitter (gun, replay, script, AI) can write
/// intents. Every intent in the tick fires, not only the last one.
///
/// Shots fire in write order. The message buffer is cleared on
/// `LoadWorld::Mapping`, and a resimulated tick writes the same intents in the
/// same order, so results match on every peer. A same-channel, same-tick
/// winner rule, if needed, belongs here.
pub fn portal_fire_system(
    mut fires: MessageReader<PortalFireIntent>,
    mut commands: Commands,
    mut fired: MessageWriter<PortalShotFired>,
) {
    for fire in fires.read().cloned() {
        let dir = fire.dir.normalize_or_zero();
        if dir == Vec2::ZERO {
            continue;
        }
        // A host audio adapter plays the cues.
        fired.write(PortalShotFired {
            origin: fire.origin,
        });
        let mut shot = commands.spawn_room_scoped((
            PortalShot {
                channel: fire.channel,
                pos: fire.origin,
                vel: dir * PORTAL_SHOT_SPEED,
                traveled: 0.0,
            },
            Name::new("Portal shot"),
        ));
        // Only the emitter can derive an identity; see `PortalFireIntent::id`.
        if let Some(id) = fire.id.clone() {
            shot.insert(id);
        }
    }
}

/// World access for [`step_portal_shot`]: the solids the ray can hit and the
/// world bounds. A host adapter supplies it (for Ambition, from
/// `RoomGeometry`).
///
/// `solids` is a
/// [`SolidWorldQuery`](ambition_platformer2d_core::cast::SolidWorldQuery);
/// `size` is the world rectangle (origin `(0,0)`). The shot fizzles 64 px
/// outside it.
pub struct PortalShotWorld<'a, W: SolidWorldQuery + ?Sized> {
    /// The solids the shot's raycast hits, including one-way platforms.
    pub solids: &'a W,
    /// World extent (max corner; min is `(0,0)`). The shot fizzles 64px outside.
    pub size: Vec2,
}

/// Whether a surface the shot hit accepts a portal. A surface can block the
/// ray and still reject a portal. Today every solid accepts portals; a future
/// no-portal tile can change this without an API change. `hit` is the contact
/// point, `normal` the outward surface normal.
#[inline]
pub fn is_portal_placeable(_hit: Vec2, _normal: Vec2) -> bool {
    true
}

/// Outcome of advancing one [`PortalShot`] by `dt`. A pure decision; the Bevy
/// adapter applies it (spawns, despawns, sfx).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PortalShotStep {
    /// Still flying: advance to `pos` and add `traveled_delta` to the odometer.
    Travel { pos: Vec2, traveled_delta: f32 },
    /// Hit a portal-placeable surface: open (or replace) a portal of `channel`
    /// at `pos` with `normal`; despawn the shot.
    Place {
        channel: PortalChannel,
        pos: Vec2,
        normal: Vec2,
        /// The raw contact point (for the close/attach sfx position).
        hit: Vec2,
    },
    /// Fizzled (past max range / out of bounds, or hit a non-placeable surface):
    /// despawn the shot. `pos` is where the buzz plays.
    Fizzle { pos: Vec2 },
}

/// Advance one portal shot by one tick and decide its outcome (the pure part
/// of `portal_projectile_step`). Contact with an [`is_portal_placeable`]
/// surface places the portal; other contact fizzles. Otherwise the shot moves
/// until max range or the world bounds.
pub fn step_portal_shot<W: SolidWorldQuery + ?Sized>(
    shot: &PortalShot,
    world: &PortalShotWorld<'_, W>,
    dt: f32,
) -> PortalShotStep {
    let step = (shot.vel * dt).length().max(1.0);
    if let Some((hit, normal)) = raycast_solids(world.solids, shot.pos, shot.vel, step, true) {
        if is_portal_placeable(hit, normal) {
            return PortalShotStep::Place {
                channel: shot.channel,
                pos: hit + normal * 2.0,
                normal,
                hit,
            };
        }
        // Hit a solid that rejects a portal — the shot dies on it (no portal).
        return PortalShotStep::Fizzle { pos: hit };
    }
    let pos = shot.pos + shot.vel * dt;
    let traveled = shot.traveled + step;
    let oob = pos.x < -64.0
        || pos.y < -64.0
        || pos.x > world.size.x + 64.0
        || pos.y > world.size.y + 64.0;
    if traveled > PORTAL_MAX_RANGE || oob {
        PortalShotStep::Fizzle { pos }
    } else {
        PortalShotStep::Travel {
            pos,
            traveled_delta: step,
        }
    }
}

#[cfg(test)]
mod fire_intent_tests {
    use super::*;
    use crate::color::{PortalChannel, PortalGunColor};

    fn app_with_the_fire_system() -> App {
        let mut app = App::new();
        app.add_message::<PortalFireIntent>();
        app.add_message::<PortalShotFired>();
        app.add_systems(Update, portal_fire_system);
        app
    }

    fn intent(origin_x: f32, channel: PortalChannel) -> PortalFireIntent {
        PortalFireIntent {
            origin: Vec2::new(origin_x, 0.0),
            dir: Vec2::new(1.0, 0.0),
            channel,
            // These tests cover motion; `rollback_populated_timeline` covers
            // identity.
            id: None,
        }
    }

    fn shots(app: &mut App) -> Vec<PortalShot> {
        let world = app.world_mut();
        let mut query = world.query::<&PortalShot>();
        query.iter(world).copied().collect()
    }

    /// Two emitters firing in the same tick give two shots.
    #[test]
    fn two_emitters_firing_in_one_tick_each_get_their_shot() {
        let mut app = app_with_the_fire_system();
        app.world_mut()
            .write_message(intent(10.0, PortalChannel::Gun(PortalGunColor::BLUE)));
        app.world_mut()
            .write_message(intent(20.0, PortalChannel::Gun(PortalGunColor::ORANGE)));
        app.update();

        let mut origins: Vec<f32> = shots(&mut app).iter().map(|shot| shot.pos.x).collect();
        origins.sort_by(f32::total_cmp);
        assert_eq!(
            origins,
            vec![10.0, 20.0],
            "one of two same-tick fire intents was dropped, so a second player, \
             a script, or any non-gun emitter cannot fire on a frame the gun did"
        );
    }

    /// Each shot uses its own intent's channel.
    #[test]
    fn each_shot_keeps_the_channel_of_the_intent_that_made_it() {
        let mut app = app_with_the_fire_system();
        app.world_mut()
            .write_message(intent(10.0, PortalChannel::Gun(PortalGunColor::BLUE)));
        app.world_mut()
            .write_message(intent(20.0, PortalChannel::Gun(PortalGunColor::ORANGE)));
        app.update();

        let mut pairs: Vec<(i32, PortalChannel)> = shots(&mut app)
            .iter()
            .map(|shot| (shot.pos.x as i32, shot.channel))
            .collect();
        pairs.sort_by_key(|(x, _)| *x);
        assert_eq!(
            pairs,
            vec![
                (10, PortalChannel::Gun(PortalGunColor::BLUE)),
                (20, PortalChannel::Gun(PortalGunColor::ORANGE)),
            ]
        );
    }

    /// A zero aim skips only its own intent.
    #[test]
    fn a_zero_aim_cancels_only_its_own_shot() {
        let mut app = app_with_the_fire_system();
        app.world_mut()
            .write_message(intent(20.0, PortalChannel::Gun(PortalGunColor::ORANGE)));
        // The zero-aim intent goes last, so a "last intent only" reader fails.
        app.world_mut().write_message(PortalFireIntent {
            origin: Vec2::new(10.0, 0.0),
            dir: Vec2::ZERO,
            channel: PortalChannel::Gun(PortalGunColor::BLUE),
            id: None,
        });
        app.update();

        let origins: Vec<f32> = shots(&mut app).iter().map(|shot| shot.pos.x).collect();
        assert_eq!(
            origins,
            vec![20.0],
            "a degenerate aim from one emitter must not cancel another emitter's \
             shot in the same tick"
        );
    }

    /// One signal per shot.
    #[test]
    fn every_shot_emits_its_own_fired_signal() {
        let mut app = app_with_the_fire_system();
        app.world_mut()
            .write_message(intent(10.0, PortalChannel::Gun(PortalGunColor::BLUE)));
        app.world_mut()
            .write_message(intent(20.0, PortalChannel::Gun(PortalGunColor::ORANGE)));
        app.update();

        let world = app.world_mut();
        let messages = world.resource::<Messages<PortalShotFired>>();
        let mut cursor = messages.get_cursor();
        assert_eq!(cursor.read(messages).count(), 2);
    }
}
