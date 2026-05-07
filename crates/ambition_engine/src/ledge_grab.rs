//! Ledge grab probe and contact data.
//!
//! Pure (Bevy-free) primitives the sandbox calls each frame when the
//! player has the `ledge_grab` ability and is wall-clinging without a
//! ledge above. The probe answers "is there a ledge corner I can snap
//! onto, and where is it?" — the sandbox owns the actual snap, input
//! gating, and climb animation.
//!
//! Why this lives outside `movement.rs`: that module is dense and
//! hostile to incremental changes. The ledge grab integration is
//! deliberately layered on top via a separate sandbox-side step so
//! fragmenting the player simulation isn't a precondition for the
//! mechanic shipping.

use crate::geometry::{Aabb, AabbExt};
use crate::world::{BlockKind, World};
use crate::Vec2;

/// What surface, and where, does the probe accept a ledge grab?
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LedgeContact {
    /// X-direction the wall pushes the player toward (+1 = wall on
    /// player's left, ‑1 = wall on player's right).
    pub wall_normal_x: f32,
    /// World position the player should snap to (their center while
    /// hanging on the ledge). The vertical pos is the top of the
    /// ledge; the horizontal pos hugs the wall edge.
    pub anchor: Vec2,
    /// Top-left of the platform the player would climb up onto. Used
    /// by the climb animation so the player can ease toward this
    /// point on Up + Jump.
    pub climb_target: Vec2,
}

/// Probe for a grabbable ledge while the player is wall-clinging.
///
/// Inputs:
/// - `player_pos` — center of the player AABB.
/// - `player_size` — full size of the player AABB.
/// - `wall_normal_x` — what `Player::wall_normal_x` reads (+/-1).
/// - `world` — the active collision world.
///
/// The probe scans for a Solid block whose top edge is within a
/// shoulder-height band of the player and whose vertical face matches
/// the wall the player is clinging to. If found, returns the snap
/// anchor and the climb target.
pub fn probe_ledge_grab(
    player_pos: Vec2,
    player_size: Vec2,
    wall_normal_x: f32,
    world: &World,
) -> Option<LedgeContact> {
    if wall_normal_x.abs() < 0.5 {
        return None;
    }
    let half = player_size * 0.5;
    // Window where the ledge top must sit — a band roughly between
    // the player's chin and the top of the head. Outside this band the
    // ledge isn't grabbable in the cling-snap idiom.
    let head_y = player_pos.y - half.y;
    let chin_band_min = head_y - 12.0;
    let chin_band_max = head_y + 18.0;
    // The player is "facing into" the wall whose normal points away
    // from the player. wall_normal_x = +1 means the wall is on the
    // player's left (the wall normal points right toward the player),
    // so the wall edge we want to hook is just left of the player.
    let cling_x = if wall_normal_x > 0.0 {
        player_pos.x - half.x
    } else {
        player_pos.x + half.x
    };
    let mut best: Option<LedgeContact> = None;
    for block in &world.blocks {
        if !matches!(block.kind, BlockKind::Solid) {
            continue;
        }
        let top = block.aabb.top();
        if top < chin_band_min || top > chin_band_max {
            continue;
        }
        // Pick the wall edge of this block matching the cling side.
        let block_wall_x = if wall_normal_x > 0.0 {
            block.aabb.right()
        } else {
            block.aabb.left()
        };
        // The player must be touching that face (within a small
        // tolerance — the wall-cling state already implies contact).
        if (block_wall_x - cling_x).abs() > 4.0 {
            continue;
        }
        // The space directly above the block must be clear, otherwise
        // there's no platform to climb onto. Probe a half-size body
        // sitting on top of the block to test for clearance.
        let probe_center = Vec2::new(
            block_wall_x - wall_normal_x * (half.x - 1.0),
            top - half.y - 1.0,
        );
        let probe_aabb = Aabb::new(probe_center, half - Vec2::new(2.0, 2.0));
        let blocked = world.body_overlaps_any(probe_aabb, |b| {
            matches!(b.kind, BlockKind::Solid) && !std::ptr::eq(b, block)
        });
        if blocked {
            continue;
        }
        // Anchor: player center hugs the wall on the same side the
        // player was clinging from, with the chest at the ledge top.
        // wall_normal_x = -1 (wall on player's right) → anchor.x is
        // just left of the wall's left face.
        let anchor = Vec2::new(
            block_wall_x + wall_normal_x * (half.x - 1.0),
            top + half.y - 4.0,
        );
        // Climb target: top of the block, just inboard of the edge.
        // (Inboard = the side away from the cling — opposite sign to
        // the anchor.)
        let climb_target = Vec2::new(
            block_wall_x - wall_normal_x * (half.x + 4.0),
            top - half.y - 1.0,
        );
        let candidate = LedgeContact {
            wall_normal_x,
            anchor,
            climb_target,
        };
        // Prefer the ledge whose top is closest to the player's head.
        let new_distance = (top - head_y).abs();
        let keep = match best {
            None => true,
            Some(prev) => {
                let prev_distance = (prev.anchor.y - half.y - head_y).abs();
                new_distance < prev_distance
            }
        };
        if keep {
            best = Some(candidate);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Block;

    fn world_with(blocks: Vec<Block>) -> World {
        World::new("ledge", Vec2::new(800.0, 600.0), Vec2::ZERO, blocks)
    }

    #[test]
    fn finds_ledge_when_clinging_to_a_wall_with_open_space_above() {
        let world = world_with(vec![Block::solid(
            "ledge",
            Vec2::new(100.0, 100.0),
            Vec2::new(200.0, 200.0),
        )]);
        // Player center to the left of the wall (player's right edge
        // touches the block's left face). wall_normal_x = -1 (wall on
        // the player's right pushes them left).
        let player_pos = Vec2::new(86.0, 110.0);
        let player_size = Vec2::new(28.0, 46.0);
        let contact = probe_ledge_grab(player_pos, player_size, -1.0, &world);
        assert!(contact.is_some(), "expected ledge contact");
        let contact = contact.unwrap();
        assert!(contact.wall_normal_x < 0.0);
        // Anchor hugs the wall edge (block.left = 100) just outboard
        // of the player (player half is 14 → anchor.x ≈ 87).
        assert!(
            (contact.anchor.x - 87.0).abs() < 4.0,
            "anchor.x = {}, expected ~87",
            contact.anchor.x
        );
        // Climb target is on top of the block, slightly inboard from
        // the edge.
        assert!(contact.climb_target.x > 100.0);
        assert!(contact.climb_target.y < contact.anchor.y);
    }

    #[test]
    fn rejects_when_above_is_blocked() {
        let world = world_with(vec![
            Block::solid(
                "ledge",
                Vec2::new(100.0, 100.0),
                Vec2::new(200.0, 200.0),
            ),
            Block::solid(
                "low_ceiling",
                Vec2::new(60.0, 50.0),
                Vec2::new(100.0, 50.0),
            ),
        ]);
        let player_pos = Vec2::new(86.0, 110.0);
        let player_size = Vec2::new(28.0, 46.0);
        let contact = probe_ledge_grab(player_pos, player_size, -1.0, &world);
        assert!(
            contact.is_none(),
            "should not return a ledge whose top has another block above"
        );
    }

    #[test]
    fn rejects_when_no_wall_present() {
        let world = world_with(vec![]);
        let player_pos = Vec2::new(50.0, 50.0);
        let player_size = Vec2::new(28.0, 46.0);
        let contact = probe_ledge_grab(player_pos, player_size, -1.0, &world);
        assert!(contact.is_none());
    }

    #[test]
    fn rejects_zero_wall_normal() {
        let world = world_with(vec![Block::solid(
            "ledge",
            Vec2::new(100.0, 100.0),
            Vec2::new(200.0, 200.0),
        )]);
        let contact = probe_ledge_grab(
            Vec2::new(86.0, 110.0),
            Vec2::new(28.0, 46.0),
            0.0,
            &world,
        );
        assert!(contact.is_none());
    }

    #[test]
    fn finds_ledge_on_left_facing_wall() {
        // Block from x=0 to x=100. Player to the right of the block
        // with `wall_normal_x = +1` (wall on player's left, normal
        // pushes player right).
        let world = world_with(vec![Block::solid(
            "left_wall",
            Vec2::new(0.0, 100.0),
            Vec2::new(100.0, 200.0),
        )]);
        let player_size = Vec2::new(28.0, 46.0);
        let player_pos = Vec2::new(114.0, 110.0); // hugging right edge of block
        let contact = probe_ledge_grab(player_pos, player_size, 1.0, &world);
        assert!(contact.is_some(), "should find ledge on the right face");
        let contact = contact.unwrap();
        assert!(contact.wall_normal_x > 0.0);
        // Climb target is to the left of the anchor (toward the
        // block's interior on top).
        assert!(contact.climb_target.x < contact.anchor.x);
    }
}
