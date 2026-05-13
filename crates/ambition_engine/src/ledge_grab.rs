//! Ledge grab probe, state, and movement-pipeline tick helpers.
//!
//! The probe answers: "is there a ledge corner I can snap onto, and
//! where is the hang / pull-up path?" The state machine is engine-owned so
//! ledge grab participates in the same movement tick as gravity, collision,
//! water, and wall state instead of running as a post-update sandbox mutator.

use crate::geometry::{Aabb, AabbExt};
use crate::movement::{InputState, MovementOp, Player};
use crate::world::{BlockKind, World};
use crate::Vec2;

/// Duration of the ledge pull-up transition.
pub const LEDGE_CLIMB_TIME: f32 = 0.24;

/// Require a tiny hang beat before held horizontal input into the platform
/// auto-starts the climb.
pub const LEDGE_TOWARD_CLIMB_DELAY: f32 = 0.045;

/// Minimum hang time before any climb input can start the pull-up.
pub const LEDGE_MIN_CLIMB_DELAY: f32 = 0.16;

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

/// Engine-owned ledge-grab state for the player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LedgeGrabState {
    pub contact: LedgeContact,
    /// Seconds since the cling-snap fired. Used for input affordances such as
    /// giving held-into-wall input a tiny beat before it auto-starts the climb.
    pub elapsed: f32,
    /// True once the climb has been requested. While true, the movement tick
    /// interpolates the player from `contact.anchor` to `contact.climb_target`.
    pub climbing: bool,
    /// Seconds spent in the pull-up transition.
    pub climb_elapsed: f32,
}

impl LedgeGrabState {
    pub fn hanging(contact: LedgeContact) -> Self {
        Self {
            contact,
            elapsed: 0.0,
            climbing: false,
            climb_elapsed: 0.0,
        }
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn climb_position(contact: LedgeContact, progress: f32) -> Vec2 {
    let t = smoothstep(progress);
    contact.anchor + (contact.climb_target - contact.anchor) * t
}

pub fn into_platform_axis(contact: LedgeContact) -> f32 {
    -contact.wall_normal_x
}

pub fn away_from_platform_axis(contact: LedgeContact) -> f32 {
    contact.wall_normal_x
}

fn ledge_surface_kind(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Solid | BlockKind::BlinkWall { .. } | BlockKind::OneWay
    )
}

fn ledge_clearance_blocker_kind(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Solid | BlockKind::BlinkWall { .. } | BlockKind::OneWay
    )
}

/// Probe for a grabbable ledge while the player is wall-clinging.
///
/// Inputs:
/// - `player_pos` — center of the player AABB.
/// - `player_size` — full size of the player AABB.
/// - `wall_normal_x` — what `Player::wall_normal_x` reads (+/-1).
/// - `world` — the active collision world.
///
/// The probe scans for a standable ledge surface (`Solid`, `BlinkWall`, or
/// `OneWay`) whose top edge is within a shoulder-height band of the player and
/// whose vertical edge matches the side the player is reaching toward. If
/// found, returns the snap anchor and the climb target.
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
        if !ledge_surface_kind(block.kind) {
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
        // World-bounds check: the player body sitting on top of this
        // ledge must stay inside the playable rect. Engine uses
        // top-left coords with the world spanning [0, size]. Without
        // this guard, a block whose top is at y≈0 (e.g. a ceiling
        // tile) yields a climb_target above the world, the climb-up
        // teleports the player OOB, and the engine's
        // collision-correction yanks them back — producing the
        // teleport loop seen in the May 2026 mob_lab F8 trace.
        if probe_center.y - half.y < 0.0
            || probe_center.x - half.x < 0.0
            || probe_center.x + half.x > world.size.x
        {
            continue;
        }
        let blocked = world.body_overlaps_any(probe_aabb, |b| {
            ledge_clearance_blocker_kind(b.kind) && !std::ptr::eq(b, block)
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

/// If the player is currently hanging/climbing, advance that state and return
/// true to indicate that the normal movement integrator should not run this
/// frame.
pub fn tick_active_ledge_grab(
    player: &mut Player,
    input: InputState,
    dt: f32,
    events: &mut crate::movement::FrameEvents,
) -> bool {
    let Some(mut state) = player.ledge_grab else {
        return false;
    };
    if !player.abilities.ledge_grab {
        player.ledge_grab = None;
        return false;
    }

    state.elapsed += dt;
    player.facing = into_platform_axis(state.contact);

    if state.climbing {
        state.climb_elapsed += dt;
        let progress = (state.climb_elapsed / LEDGE_CLIMB_TIME).clamp(0.0, 1.0);
        player.pos = climb_position(state.contact, progress);
        player.vel = Vec2::ZERO;
        player.on_ground = false;
        player.wall_clinging = false;
        player.wall_climbing = false;
        player.on_wall = false;

        if progress >= 1.0 {
            player.pos = state.contact.climb_target;
            player.vel = Vec2::ZERO;
            player.on_ground = true;
            player.wall_clinging = false;
            player.wall_climbing = false;
            player.on_wall = false;
            player.ledge_grab = None;
            events.op(player, MovementOp::LedgeClimbFinish);
        } else {
            player.ledge_grab = Some(state);
        }
        return true;
    }

    let input_up = input.axis_y < -0.4;
    let input_down = input.axis_y > 0.4;
    let input_into_platform = input.axis_x * into_platform_axis(state.contact) > 0.4;
    let input_away_from_platform = input.axis_x * away_from_platform_axis(state.contact) > 0.4;
    let climb_unlocked = state.elapsed >= LEDGE_MIN_CLIMB_DELAY;
    let want_climb = climb_unlocked
        && (input_up
            || input.interact_pressed
            || input.jump_pressed
            || (state.elapsed >= LEDGE_TOWARD_CLIMB_DELAY && input_into_platform));
    let want_drop = input_down || input_away_from_platform;

    if want_drop && !want_climb {
        player.wall_clinging = false;
        player.wall_climbing = false;
        player.on_wall = false;
        player.ledge_grab = None;
        events.op(player, MovementOp::LedgeDrop);
        return true;
    }
    if want_climb {
        state.climbing = true;
        state.climb_elapsed = 0.0;
        player.pos = state.contact.anchor;
        player.vel = Vec2::ZERO;
        player.on_ground = false;
        player.wall_clinging = false;
        player.wall_climbing = false;
        player.on_wall = false;
        player.ledge_grab = Some(state);
        events.op(player, MovementOp::LedgeClimbStart);
        return true;
    }

    player.pos = state.contact.anchor;
    player.vel = Vec2::ZERO;
    player.wall_clinging = true;
    player.wall_climbing = false;
    player.on_wall = true;
    player.ledge_grab = Some(state);
    true
}

fn requested_wall_normal(player: &Player, input: InputState) -> Option<f32> {
    if player.wall_clinging && player.wall_normal_x.abs() >= 0.5 {
        return Some(player.wall_normal_x);
    }
    if !player.on_ground && input.axis_x.abs() > 0.4 {
        return Some(-input.axis_x.signum());
    }
    None
}

/// Probe for and start a new ledge grab after normal collision has established
/// this frame's wall/airborne state. Returns true when a new grab latched.
pub fn try_start_ledge_grab(
    world: &World,
    player: &mut Player,
    input: InputState,
    events: &mut crate::movement::FrameEvents,
) -> bool {
    if !player.abilities.ledge_grab || player.ledge_grab.is_some() || player.on_ground {
        return false;
    }
    let Some(wall_normal) = requested_wall_normal(player, input) else {
        return false;
    };
    let Some(contact) = probe_ledge_grab(player.pos, player.size, wall_normal, world) else {
        return false;
    };
    player.pos = contact.anchor;
    player.vel = Vec2::ZERO;
    player.facing = into_platform_axis(contact);
    player.wall_clinging = true;
    player.wall_climbing = false;
    player.on_wall = true;
    player.wall_normal_x = contact.wall_normal_x;
    player.ledge_grab = Some(LedgeGrabState::hanging(contact));
    events.op(player, MovementOp::LedgeGrab);
    true
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
            Block::solid("ledge", Vec2::new(100.0, 100.0), Vec2::new(200.0, 200.0)),
            Block::solid("low_ceiling", Vec2::new(60.0, 50.0), Vec2::new(100.0, 50.0)),
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
        let contact = probe_ledge_grab(Vec2::new(86.0, 110.0), Vec2::new(28.0, 46.0), 0.0, &world);
        assert!(contact.is_none());
    }

    /// Regression: a ledge whose top sits near the world's ceiling
    /// must be rejected — climbing onto it would put the player
    /// out of bounds. This was the May 2026 mob_lab teleport-loop
    /// bug: a ceiling tile near y=0 produced a climb_target above
    /// the world, the climb-up snapped the player OOB, and the
    /// engine's collision-correction yanked them back, looping.
    #[test]
    fn rejects_ledge_when_player_would_land_above_world_top() {
        // Ceiling block: top edge at y=1 (world ranges y=0..600).
        // Player half-height is 23, so a body sitting on this ledge
        // would have its top at y = 1 - 46 - 1 = -46 (above world).
        let world = world_with(vec![Block::solid(
            "ceiling",
            Vec2::new(100.0, 1.0),
            Vec2::new(200.0, 80.0),
        )]);
        // Player wall-clinging just below the ceiling block, with
        // their head right under the block's top.
        let player_pos = Vec2::new(86.0, 24.0);
        let player_size = Vec2::new(28.0, 46.0);
        let contact = probe_ledge_grab(player_pos, player_size, -1.0, &world);
        assert!(
            contact.is_none(),
            "ceiling-adjacent ledge must be rejected (climb_target would be OOB)"
        );
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
    #[test]
    fn finds_ledge_on_blink_wall() {
        let world = world_with(vec![Block::blink_wall(
            "blink_ledge",
            Vec2::new(100.0, 100.0),
            Vec2::new(200.0, 200.0),
            crate::world::BlinkWallTier::Soft,
        )]);
        let contact = probe_ledge_grab(Vec2::new(86.0, 110.0), Vec2::new(28.0, 46.0), -1.0, &world);
        assert!(
            contact.is_some(),
            "blink walls are standable ledge surfaces"
        );
    }

    #[test]
    fn finds_ledge_on_one_way_platform_edge() {
        let world = world_with(vec![Block::one_way(
            "thin_ledge",
            Vec2::new(100.0, 100.0),
            Vec2::new(200.0, 16.0),
        )]);
        let contact = probe_ledge_grab(Vec2::new(86.0, 110.0), Vec2::new(28.0, 46.0), -1.0, &world);
        assert!(contact.is_some(), "one-way platforms can be pulled up onto");
    }

    #[test]
    fn rejects_when_lock_door_blocks_pull_up_space() {
        let world = world_with(vec![
            Block::one_way("ledge", Vec2::new(100.0, 100.0), Vec2::new(200.0, 16.0)),
            Block::solid("lock_door", Vec2::new(104.0, 40.0), Vec2::new(48.0, 80.0)),
        ]);
        let contact = probe_ledge_grab(Vec2::new(86.0, 110.0), Vec2::new(28.0, 46.0), -1.0, &world);
        assert!(
            contact.is_none(),
            "a solid lock door in the climb target must block the grab"
        );
    }
}
