//! Generated sandbox room data.
//!
//! The engine models room geometry as named blocks. The Bevy sandbox decides
//! how to draw each block; the engine only cares about collision semantics.

use crate::geometry::Aabb;
use crate::math::Vec2;

/// Upgrade tier required to blink through a blink wall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlinkWallTier {
    /// Intended to be passable by an early blink-phasing upgrade.
    Soft,
    /// Intended to remain blocked until a stronger blink-phasing upgrade.
    Hard,
}

/// Collision/gameplay meaning of a generated world block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlockKind {
    /// Full collision on both axes, and also a hard blocker for blink pathing.
    Solid,
    /// Full collision on both axes, but blink pathing may pass through it when
    /// the player has the matching blink-through upgrade. The destination still
    /// must be open space.
    BlinkWall { tier: BlinkWallTier },
    /// Landing platform: only solid when the player crosses from above.
    OneWay,
    /// Reset surface. Hitting this returns the player to spawn.
    Hazard,
    /// Pogo target that refreshes movement resources when struck downward.
    PogoOrb,
    /// Momentum-conversion surface. It applies a fixed impulse on touch.
    Rebound { impulse: Vec2 },
}

/// One piece of generated room geometry.
#[derive(Clone, Debug)]
pub struct Block {
    pub name: &'static str,
    pub aabb: Aabb,
    pub kind: BlockKind,
}

impl Block {
    pub fn solid(name: &'static str, min: Vec2, size: Vec2) -> Self {
        Self {
            name,
            aabb: Aabb::from_min_size(min, size),
            kind: BlockKind::Solid,
        }
    }

    pub fn blink_wall(name: &'static str, min: Vec2, size: Vec2, tier: BlinkWallTier) -> Self {
        Self {
            name,
            aabb: Aabb::from_min_size(min, size),
            kind: BlockKind::BlinkWall { tier },
        }
    }

    pub fn one_way(name: &'static str, min: Vec2, size: Vec2) -> Self {
        Self {
            name,
            aabb: Aabb::from_min_size(min, size),
            kind: BlockKind::OneWay,
        }
    }

    pub fn hazard(name: &'static str, min: Vec2, size: Vec2) -> Self {
        Self {
            name,
            aabb: Aabb::from_min_size(min, size),
            kind: BlockKind::Hazard,
        }
    }

    pub fn pogo_orb(name: &'static str, center: Vec2, radius: f32) -> Self {
        Self {
            name,
            aabb: Aabb::new(center, Vec2::new(radius, radius)),
            kind: BlockKind::PogoOrb,
        }
    }

    pub fn rebound(name: &'static str, min: Vec2, size: Vec2, impulse: Vec2) -> Self {
        Self {
            name,
            aabb: Aabb::from_min_size(min, size),
            kind: BlockKind::Rebound { impulse },
        }
    }
}

/// Complete generated room spec.
#[derive(Clone, Debug)]
pub struct World {
    pub name: &'static str,
    pub size: Vec2,
    pub spawn: Vec2,
    pub blocks: Vec<Block>,
}

/// Build the first Ambition endgame lab.
///
/// All geometry is procedural/code data; there are no textures, sprites, maps,
/// sounds, or imported assets. This function is intentionally explicit for now:
/// the room itself is a design document for what mechanics the sandbox should
/// test. Later this can become a `RoomSpec` DSL or generated layout grammar.
pub fn build_endgame_sandbox() -> World {
    let mut blocks = Vec::new();
    let w = 1600.0;
    let h = 900.0;

    // Shell. The world is intentionally larger than the first pass so the same
    // screen has more world-space resolution for movement tuning.
    blocks.push(Block::solid("floor", Vec2::new(0.0, h - 48.0), Vec2::new(w, 48.0)));
    blocks.push(Block::solid("left wall", Vec2::new(0.0, 0.0), Vec2::new(36.0, h)));
    blocks.push(Block::solid("right wall", Vec2::new(w - 36.0, 0.0), Vec2::new(36.0, h)));
    blocks.push(Block::solid("ceiling lip", Vec2::new(0.0, 0.0), Vec2::new(w, 24.0)));

    // Reachable enemy test lane near spawn. This is deliberately simple:
    // immediate attack/pogo testing should not require completing the loop.
    blocks.push(Block::blink_wall("dummy approach step", Vec2::new(120.0, 760.0), Vec2::new(170.0, 28.0), BlinkWallTier::Soft));
    blocks.push(Block::one_way("dummy upper tap platform", Vec2::new(315.0, 704.0), Vec2::new(250.0, 16.0)));

    // A larger clockwise flow loop for endgame movement experiments.
    blocks.push(Block::blink_wall("left wall kick column", Vec2::new(100.0, 520.0), Vec2::new(58.0, 220.0), BlinkWallTier::Soft));
    blocks.push(Block::one_way("middle shelf", Vec2::new(430.0, 610.0), Vec2::new(265.0, 18.0)));
    blocks.push(Block::blink_wall("upper left shelf", Vec2::new(260.0, 410.0), Vec2::new(240.0, 24.0), BlinkWallTier::Soft));
    blocks.push(Block::solid("needle pillar", Vec2::new(730.0, 515.0), Vec2::new(58.0, 285.0)));
    blocks.push(Block::one_way("high bridge", Vec2::new(850.0, 330.0), Vec2::new(320.0, 18.0)));
    blocks.push(Block::blink_wall("right catch wall", Vec2::new(1320.0, 430.0), Vec2::new(56.0, 330.0), BlinkWallTier::Soft));
    blocks.push(Block::blink_wall("right return shelf", Vec2::new(1080.0, 675.0), Vec2::new(235.0, 24.0), BlinkWallTier::Soft));
    blocks.push(Block::one_way("ceiling practice shelf", Vec2::new(610.0, 230.0), Vec2::new(190.0, 16.0)));

    // Blink walls are solid to normal movement, but sandbox blink pathing can
    // pass through them. For this iteration, every interior wall-like blocker is
    // blink-passable except the central needle pillar. The outer shell remains
    // `Solid`, so the player cannot blink out of the room.
    blocks.push(Block::blink_wall(
        "soft blink membrane",
        Vec2::new(980.0, 676.0),
        Vec2::new(20.0, 155.0),
        BlinkWallTier::Soft,
    ));
    blocks.push(Block::blink_wall(
        "hard blink lock",
        Vec2::new(1185.0, 500.0),
        Vec2::new(22.0, 150.0),
        BlinkWallTier::Hard,
    ));

    // Intentional danger/rest/reset surfaces: recoverable if you are stylish.
    blocks.push(Block::hazard("central spike channel", Vec2::new(570.0, 830.0), Vec2::new(260.0, 22.0)));
    blocks.push(Block::hazard("right spike channel", Vec2::new(930.0, 830.0), Vec2::new(230.0, 22.0)));
    blocks.push(Block::hazard("high tooth", Vec2::new(1225.0, 650.0), Vec2::new(70.0, 24.0)));

    // Pogo orbs act as refresh notes in the movement instrument.
    blocks.push(Block::pogo_orb("pogo alpha", Vec2::new(555.0, 505.0), 19.0));
    blocks.push(Block::pogo_orb("pogo beta", Vec2::new(850.0, 470.0), 19.0));
    blocks.push(Block::pogo_orb("pogo gamma", Vec2::new(1135.0, 555.0), 19.0));
    blocks.push(Block::pogo_orb("pogo ceiling note", Vec2::new(700.0, 300.0), 17.0));

    // Rebound pads are explicit momentum converters.
    blocks.push(Block::rebound(
        "left launcher",
        Vec2::new(72.0, 812.0),
        Vec2::new(86.0, 22.0),
        Vec2::new(570.0, -810.0),
    ));
    blocks.push(Block::rebound(
        "right return launcher",
        Vec2::new(1390.0, 795.0),
        Vec2::new(100.0, 24.0),
        Vec2::new(-720.0, -680.0),
    ));
    blocks.push(Block::rebound(
        "ceiling redirect",
        Vec2::new(650.0, 84.0),
        Vec2::new(180.0, 18.0),
        Vec2::new(760.0, 240.0),
    ));

    World {
        name: "Ambition: Tangent Space v0.2",
        size: Vec2::new(w, h),
        spawn: Vec2::new(210.0, h - 95.0),
        blocks,
    }
}
