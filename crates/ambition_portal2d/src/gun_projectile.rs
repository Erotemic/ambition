//! Compatibility projectile for portal-gun-style placement.
//!
//! This module is intentionally sequestered from the portal topology/transit
//! core: a game can open portals by authoring, scripting, moving emitters, or
//! a gun. The reusable mechanic consumes the generic [`PortalFireIntent`] and
//! [`step_portal_shot`] helper here only for Ambition's current gun workflow.
//!
//! World access is captured through the reusable
//! [`SolidWorldQuery`](ambition_platformer2d_core::cast::SolidWorldQuery)
//! seam — the pure [`step_portal_shot`] helper raycasts against it (plus a
//! world-bounds rectangle) and decides the outcome, so portal core never reads
//! the concrete `ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomGeometry>`. The Bevy adapter that owns `RoomGeometry` lives in
//! the host portal adapter and calls the helper.

use bevy::prelude::*;

use ambition_platformer2d_core::cast::{raycast_solids, SolidWorldQuery};
use ambition_platformer2d_shared_tangle::prelude::SpawnScopedExt;

use super::color::PortalChannel;
use super::messages::{PortalFireIntent, PortalShotFired};
use super::types::{PORTAL_MAX_RANGE, PORTAL_SHOT_SPEED};

/// An in-flight portal-opening shot streaking toward a surface. On contact
/// with a solid it opens a portal on `channel`; if it travels too far / leaves
/// the world it fizzles. Ambition currently emits these from a portal gun, but
/// the shot itself is just one possible portal opener.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalShot {
    pub channel: PortalChannel,
    pub pos: Vec2,
    pub vel: Vec2,
    pub traveled: f32,
}

/// On a generic [`PortalFireIntent`], fire a portal *shot* of the intent's `channel` from
/// `origin` along `dir`. Portal core no longer reaches for a primary actor or held gun — the
/// host resolver may produce the intent from a gun, replay, script, AI, or any future emitter.
/// ⛔⛔ EVERY INTENT, NOT THE LAST ONE. This read `fires.read().last()` and
/// dropped the rest of the tick on the floor — a silent global winner, and the
/// last thing that should be implicit in a channel whose whole contract says any
/// number of emitters may produce one. Two players firing in the same tick made
/// ONE shot; a script and an actor firing together made one; a four-seat couch
/// match made one. The singleton reading was a leftover from when the only
/// emitter was the primary player's gun, and it outlived the generalisation that
/// removed that assumption.
///
/// ⚠ ORDER IS THE WRITE ORDER, and that is a deliberate rollback property: the
/// message buffer is cleared on `LoadWorld::Mapping`, and a resimulated tick
/// re-writes the same intents from the same inputs in the same order, so two
/// shots on one channel resolve the same way on every peer. If a same-channel
/// same-tick winner is ever wanted it belongs here as a stated policy over
/// emitter identity — not as a reader method nobody reads as policy.
pub fn portal_fire_system(
    mut fires: MessageReader<PortalFireIntent>,
    mut commands: Commands,
    mut fired: MessageWriter<PortalShotFired>,
) {
    for fire in fires.read().copied() {
        let dir = fire.dir.normalize_or_zero();
        if dir == Vec2::ZERO {
            continue;
        }
        // The crate emits the fire signal; a host audio adapter plays any blast /
        // travel cues (the crate owns neither audio nor ids).
        fired.write(PortalShotFired {
            origin: fire.origin,
        });
        commands.spawn_room_scoped((
            PortalShot {
                channel: fire.channel,
                pos: fire.origin,
                vel: dir * PORTAL_SHOT_SPEED,
                traveled: 0.0,
            },
            Name::new("Portal shot"),
        ));
    }
}

/// World access for the pure portal-shot step: the solid surfaces the shot's
/// ray can hit, plus the world bounds it fizzles past. The host supplies a
/// concrete value (for Ambition, `RoomGeometry`) via a host adapter;
/// [`step_portal_shot`] reasons about it through this seam, never the host's
/// concrete world type.
///
/// `solids` is the reusable
/// [`SolidWorldQuery`](ambition_platformer2d_core::cast::SolidWorldQuery)
/// surface (Stage 16); `size` is the world rectangle (origin at `(0,0)`) the
/// shot fizzles 64px outside of.
pub struct PortalShotWorld<'a, W: SolidWorldQuery + ?Sized> {
    /// The solid surfaces the shot's raycast adheres to (one-way platforms
    /// included — portal placement sticks to them).
    pub solids: &'a W,
    /// World extent (max corner; min is `(0,0)`). The shot fizzles 64px outside.
    pub size: Vec2,
}

/// Whether a surface the shot hit accepts a portal. The world seam distinguishes
/// "blocks the ray" (every [`SolidWorldQuery`] surface) from "accepts a portal":
/// a surface can stop the shot yet reject a portal. Default: every solid
/// surface accepts portals — so this is a no-op hook today. A future LDtk
/// no-portal tile will refine it (a data change, not an API change); its exact
/// representation is deferred until a concrete solid-but-no-portal surface
/// exists. `hit` is the contact point, `normal` the surface outward normal.
#[inline]
pub fn is_portal_placeable(_hit: Vec2, _normal: Vec2) -> bool {
    true
}

/// Outcome of advancing one [`PortalShot`] by `dt` against the world seam. The
/// pure decision; the Bevy adapter applies it (spawns/despawns entities, plays
/// sfx). Keeps portal core's shot logic free of `ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomGeometry>` and of ECS
/// entity bookkeeping.
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

/// Advance one portal shot one tick against the world seam and decide its
/// outcome — the pure heart of `portal_projectile_step`, free of ECS and of the
/// concrete `RoomGeometry`. A solid contact on a [`is_portal_placeable`] surface
/// places the portal; a contact on a non-placeable surface fizzles; otherwise
/// the shot travels until it passes max range or leaves the world bounds.
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
        }
    }

    fn shots(app: &mut App) -> Vec<PortalShot> {
        let world = app.world_mut();
        let mut query = world.query::<&PortalShot>();
        query.iter(world).copied().collect()
    }

    /// ⛔⛔ THE DEFECT. `PortalFireIntent`'s own doc says the host may lower an
    /// intent from a "gun, replay, script, AI, or any future emitter" — and the
    /// implementation kept only `read().last()`, so every other emitter in the
    /// tick was discarded. Two players firing on the same frame produced one
    /// shot.
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

    /// The channel each shot opens on is its OWN intent's, not the last one's.
    /// Dropping all but the last intent also silently re-coloured the survivor.
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

    /// ⚠ A ZERO AIM SKIPS ITS OWN INTENT AND NOTHING ELSE. The old `return`
    /// meant one degenerate aim cancelled the whole tick for every other
    /// emitter; iterating turns that into a `continue`, which is the only
    /// reading that matches "each emitter fires its own shot".
    #[test]
    fn a_zero_aim_cancels_only_its_own_shot() {
        let mut app = app_with_the_fire_system();
        app.world_mut()
            .write_message(intent(20.0, PortalChannel::Gun(PortalGunColor::ORANGE)));
        // ⭐ THE DEGENERATE ONE GOES LAST ON PURPOSE. With it first, a
        // `read().last()` implementation still finds the good intent and this
        // arm passes for the wrong reason — it has to be the intent the broken
        // reading would have kept.
        app.world_mut().write_message(PortalFireIntent {
            origin: Vec2::new(10.0, 0.0),
            dir: Vec2::ZERO,
            channel: PortalChannel::Gun(PortalGunColor::BLUE),
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

    /// One signal per shot: a host audio adapter plays a blast cue for each, and
    /// the emitter count is what a versus HUD would read.
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
