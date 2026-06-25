//! The in-flight [`PortalShot`]: firing a shot and stepping it until it
//! lands on a solid (opening a portal) or fizzles.
//!
//! World access is captured through the reusable
//! [`SolidWorldQuery`](ambition_platformer_primitives::world_query::SolidWorldQuery)
//! seam — the pure [`step_portal_shot`] helper raycasts against it (plus a
//! world-bounds rectangle) and decides the outcome, so portal core never reads
//! the concrete `Res<RoomGeometry>`. The Bevy adapter that owns `RoomGeometry` lives in
//! the host portal adapter and calls the helper.

use bevy::prelude::*;

use ambition_platformer_primitives::prelude::SpawnScopedExt;
use ambition_platformer_primitives::world_query::{raycast_solids, SolidWorldQuery};

use super::color::PortalChannel;
use super::messages::{PortalFireIntent, PortalShotFired};
use super::types::{PORTAL_MAX_RANGE, PORTAL_SHOT_SPEED};

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

/// On a generic [`PortalFireIntent`], fire a portal *shot* of the intent's
/// `channel` from `origin` along `dir`. The shot travels (see
/// `portal_projectile_step`) so the player sees its path before it lands and
/// opens a portal. Portal core no longer reaches for the primary player or the
/// held gun — the Ambition resolver
/// (the host portal adapter) produces the
/// intent from the player's body + the gun's color, so a replay or AI can place
/// a portal by emitting the same intent.
pub fn portal_fire_system(
    mut fires: MessageReader<PortalFireIntent>,
    mut commands: Commands,
    mut fired: MessageWriter<PortalShotFired>,
) {
    let Some(fire) = fires.read().last().copied() else {
        return;
    };
    let dir = fire.dir.normalize_or_zero();
    if dir == Vec2::ZERO {
        return;
    }
    // The crate emits the fire signal; an Ambition audio adapter plays the
    // punchy blast + airy travel whizz (the crate owns neither audio nor ids).
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

/// World access for the pure portal-shot step: the solid surfaces the shot's
/// ray can hit, plus the world bounds it fizzles past. The host supplies a
/// concrete value (its `RoomGeometry`) via the Ambition adapter; portal core's
/// [`step_portal_shot`] reasons about it through this seam, never `RoomGeometry`.
///
/// `solids` is the reusable
/// [`SolidWorldQuery`](ambition_platformer_primitives::world_query::SolidWorldQuery)
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
/// a surface can stop the shot yet reject a portal. **Default: every solid
/// surface accepts portals** — so this is a no-op hook today. A future LDtk
/// no-portal tile will refine it (a data change, not an API change); its exact
/// representation is deferred until a concrete solid-but-no-portal surface
/// exists. `hit` is the contact point, `normal` the surface outward normal.
#[inline]
pub fn is_portal_placeable(_hit: Vec2, _normal: Vec2) -> bool {
    true
}

/// Outcome of advancing one [`PortalShot`] by `dt` against the world seam. The
/// pure decision; the Bevy adapter applies it (spawns/despawns entities, plays
/// sfx). Keeps portal core's shot logic free of `Res<RoomGeometry>` and of ECS
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
