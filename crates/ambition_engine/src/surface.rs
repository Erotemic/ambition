//! Typed runtime model for rectangular gameplay-geometry "Surfaces".
//!
//! A Surface is the engine-side abstraction for any rectangular
//! collision-like volume: solid walls, one-way platforms, blink walls,
//! breakable walls/platforms, hazards, pogo orbs, rebound pads. Authoring
//! tools (LDtk, RON manifests, hand-written tests) parse data and build a
//! [`SurfaceFixture`]; collision/contact systems then consume the typed
//! fixture rather than scattered identifier strings or per-feature blocks.
//!
//! The four orthogonal axes (collision, breakability, contact, respawn)
//! mirror the LDtk authoring fields exactly, so editor and runtime stay in
//! lockstep without a translation layer.

use crate::geometry::Aabb;
use crate::Vec2;

/// Hard collision behavior contributed by a Surface while it exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SurfaceCollision {
    /// Pure trigger volume; bodies pass through it.
    #[default]
    None,
    /// Full collision on both axes.
    Solid,
    /// One-way landing: solid only when crossed from above.
    OneWayUp,
    /// Soft blink wall: solid until the player has the matching blink upgrade.
    BlinkSoft,
    /// Hard blink wall: solid until the player has the stronger blink upgrade.
    BlinkHard,
}

impl SurfaceCollision {
    /// True if this collision blocks ordinary horizontal movement.
    pub fn blocks_horizontally(self) -> bool {
        matches!(
            self,
            SurfaceCollision::Solid | SurfaceCollision::BlinkSoft | SurfaceCollision::BlinkHard
        )
    }
}

/// Whether and how a Surface can be destroyed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SurfaceBreakability {
    #[default]
    Indestructible,
    BreakOnHit,
    BreakOnStand,
    BreakOnHitOrStand,
}

impl SurfaceBreakability {
    pub fn is_indestructible(self) -> bool {
        matches!(self, SurfaceBreakability::Indestructible)
    }
}

/// Side-effect applied to bodies that touch a Surface.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum SurfaceContact {
    #[default]
    None,
    /// Damage / hazard reset.
    Damage { amount: i32 },
    /// Refreshes pogo / movement resources.
    PogoRefresh,
    /// Applies a fixed impulse on contact.
    Rebound { impulse: Vec2 },
}

/// When a destroyed Surface returns.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum SurfaceRespawn {
    #[default]
    Never,
    OnRoomReload,
    AfterSeconds(f32),
}

/// Engine-side typed runtime IR for one rectangular Surface.
///
/// This is the durable game-logic representation of a Surface: position +
/// the four authoring axes. ECS components, indices, and gameplay systems
/// should hold and query [`SurfaceFixture`] rather than reparse LDtk JSON or
/// reach for legacy per-feature block kinds.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFixture {
    pub name: String,
    pub aabb: Aabb,
    pub collision: SurfaceCollision,
    pub breakability: SurfaceBreakability,
    pub contact: SurfaceContact,
    pub respawn: SurfaceRespawn,
    /// Hit points for breakable surfaces. Ignored when `Indestructible`.
    pub max_hp: i32,
}

impl SurfaceFixture {
    pub fn is_solid(&self) -> bool {
        matches!(self.collision, SurfaceCollision::Solid)
    }

    pub fn is_breakable(&self) -> bool {
        !self.breakability.is_indestructible()
    }

    pub fn is_one_way(&self) -> bool {
        matches!(self.collision, SurfaceCollision::OneWayUp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::aabb_from_min_size;

    #[test]
    fn defaults_describe_a_pure_trigger_surface() {
        let fixture = SurfaceFixture {
            name: "trigger".into(),
            aabb: aabb_from_min_size(Vec2::ZERO, Vec2::splat(16.0)),
            collision: SurfaceCollision::default(),
            breakability: SurfaceBreakability::default(),
            contact: SurfaceContact::default(),
            respawn: SurfaceRespawn::default(),
            max_hp: 0,
        };
        assert!(!fixture.is_solid());
        assert!(!fixture.is_breakable());
        assert!(!fixture.is_one_way());
    }

    #[test]
    fn solid_collision_blocks_horizontally() {
        assert!(SurfaceCollision::Solid.blocks_horizontally());
        assert!(!SurfaceCollision::OneWayUp.blocks_horizontally());
        assert!(SurfaceCollision::BlinkSoft.blocks_horizontally());
    }
}
