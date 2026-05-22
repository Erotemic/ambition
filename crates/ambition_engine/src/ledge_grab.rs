//! Ledge grab probe, state, and movement-pipeline tick helpers.
//!
//! The probe answers: "is there a ledge corner I can snap onto, and
//! where is the hang / pull-up path?" The state machine is engine-owned so
//! ledge grab participates in the same movement tick as gravity, collision,
//! water, and wall state instead of running as a post-update sandbox mutator.

use crate::geometry::{Aabb, AabbExt};
use crate::movement::{InputState, MovementOp, MovementTuning, Player};
use crate::world::{BlockKind, World};
use crate::Vec2;

/// Duration of the standard ledge pull-up transition.
pub const LEDGE_CLIMB_TIME: f32 = 0.24;

/// Duration of a Smash-Bros-style ledge ROLL: shorter than the climb
/// because the player commits horizontally and lands further inboard.
/// The whole window grants invulnerability via the existing dodge-roll
/// timer.
pub const LEDGE_ROLL_TIME: f32 = 0.30;

/// How much further inboard the roll lands than the climb. The roll
/// target is `climb_target + into_axis * LEDGE_ROLL_OVERSHOOT`,
/// chosen so the roll covers ~1 player width past the platform edge
/// — enough that the player visibly tumbles past the lip, like the
/// "ledge roll" option in Smash Bros.
pub const LEDGE_ROLL_OVERSHOOT: f32 = 36.0;

/// Require a tiny hang beat before held horizontal input into the platform
/// auto-starts the climb.
pub const LEDGE_TOWARD_CLIMB_DELAY: f32 = 0.045;

/// Minimum hang time before any climb/roll input can fire. Tightened
/// from 0.16 to 0.06 (≈ one frame at 60Hz of debounce) for Smash-Bros
/// snap-and-act feel — the original 160 ms felt mushy because most of
/// the time the player is grabbing INTENTIONALLY and wants to act
/// immediately.
pub const LEDGE_MIN_CLIMB_DELAY: f32 = 0.06;

/// Intangibility window granted at the moment the player grabs a
/// ledge. Mirrors Smash's "ledge intangibility" so a grab can't be
/// punished by edge-guards on contact. Plumbed through
/// `Player::dodge_roll_timer` because that field already powers the
/// engine's "invuln while rolling" gate; reusing it keeps the damage
/// pipeline single-source.
pub const LEDGE_GRAB_INVULN_TIME: f32 = 0.50;

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

/// Which getup the player chose when leaving the hang. The state
/// machine interpolates position differently for each variant, and
/// the sandbox HUD reads this to label the action ("Climb" / "Roll")
/// at the bottom of the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgeGetupKind {
    /// Standard pull-up: short arc from anchor to `climb_target`.
    Climb,
    /// Smash-Bros style ledge roll: faster, covers more ground past
    /// the platform edge, and grants invulnerability for the whole
    /// duration via `Player::dodge_roll_timer`.
    Roll,
}

/// Engine-owned ledge-grab state for the player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LedgeGrabState {
    pub contact: LedgeContact,
    /// Seconds since the cling-snap fired. Used for input affordances such as
    /// giving held-into-wall input a tiny beat before it auto-starts the climb.
    pub elapsed: f32,
    /// True once a getup (climb or roll) has been requested. While
    /// true, the movement tick interpolates the player along the
    /// chosen getup curve.
    pub climbing: bool,
    /// Which getup is in progress. Only meaningful while
    /// ``climbing``; ignored while hanging.
    pub getup_kind: LedgeGetupKind,
    /// Seconds spent in the pull-up / roll transition.
    pub climb_elapsed: f32,
}

impl LedgeGrabState {
    pub fn hanging(contact: LedgeContact) -> Self {
        Self {
            contact,
            elapsed: 0.0,
            climbing: false,
            getup_kind: LedgeGetupKind::Climb,
            climb_elapsed: 0.0,
        }
    }

    /// Duration of the active getup at this state's `getup_kind`.
    /// Returns 0 if not currently in a getup.
    pub fn getup_duration(self) -> f32 {
        match self.getup_kind {
            LedgeGetupKind::Climb => LEDGE_CLIMB_TIME,
            LedgeGetupKind::Roll => LEDGE_ROLL_TIME,
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

/// Roll target: ``climb_target`` plus an extra ``LEDGE_ROLL_OVERSHOOT``
/// along the into-platform axis, so the player lands a body-width
/// past the lip rather than right at the edge.
fn roll_target(contact: LedgeContact) -> Vec2 {
    Vec2::new(
        contact.climb_target.x + into_platform_axis(contact) * LEDGE_ROLL_OVERSHOOT,
        contact.climb_target.y,
    )
}

fn roll_position(contact: LedgeContact, progress: f32) -> Vec2 {
    // Roll uses ease-out (quick start, settles into the ground) so
    // the player commits horizontally fast and decelerates smoothly.
    // Smoothstep starts slow; for the roll feel we want fast-start so
    // we mirror the curve: 1 - smoothstep(1 - t).
    let t = 1.0 - smoothstep(1.0 - progress.clamp(0.0, 1.0));
    let target = roll_target(contact);
    contact.anchor + (target - contact.anchor) * t
}

fn getup_position(state: LedgeGrabState, progress: f32) -> Vec2 {
    match state.getup_kind {
        LedgeGetupKind::Climb => climb_position(state.contact, progress),
        LedgeGetupKind::Roll => roll_position(state.contact, progress),
    }
}

fn getup_end_position(state: LedgeGrabState) -> Vec2 {
    match state.getup_kind {
        LedgeGetupKind::Climb => state.contact.climb_target,
        LedgeGetupKind::Roll => roll_target(state.contact),
    }
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
    tuning: MovementTuning,
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
        let duration = state.getup_duration();
        let progress = (state.climb_elapsed / duration).clamp(0.0, 1.0);
        player.pos = getup_position(state, progress);
        player.vel = Vec2::ZERO;
        player.on_ground = false;
        player.wall_clinging = false;
        player.wall_climbing = false;
        player.on_wall = false;

        if progress >= 1.0 {
            player.pos = getup_end_position(state);
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

    // Ledge ROLL (Smash-Bros style): shield held while hanging →
    // forward roll onto the platform with full invulnerability. The
    // shield ability gate matches the dodge/shield pattern elsewhere
    // — if the player doesn't have shield unlocked the roll branch
    // is dead. Roll wins over climb so a "shield + up" combo still
    // produces the roll the player asked for.
    let want_roll =
        climb_unlocked && input.shield_held && player.abilities.shield;
    // Ledge jump: jump pressed while holding away from the ledge launches
    // the player outward with a full wall-jump-style velocity. This is the
    // smash-like "ledge jump" option as opposed to the normal pull-up climb.
    let want_ledge_jump =
        climb_unlocked && !want_roll && input.jump_pressed && input_away_from_platform;
    let want_climb = climb_unlocked
        && !want_roll
        && !want_ledge_jump
        && (input_up
            || input.interact_pressed
            || (input.jump_pressed && !input_away_from_platform)
            || (state.elapsed >= LEDGE_TOWARD_CLIMB_DELAY && input_into_platform));
    let want_drop = !want_roll && (input_down || (input_away_from_platform && !want_ledge_jump));

    if want_roll {
        state.climbing = true;
        state.getup_kind = LedgeGetupKind::Roll;
        state.climb_elapsed = 0.0;
        player.pos = state.contact.anchor;
        player.vel = Vec2::ZERO;
        player.on_ground = false;
        player.wall_clinging = false;
        player.wall_climbing = false;
        player.on_wall = false;
        // Invuln for the duration of the roll, plus a small tail so
        // the player has a few extra frames as they stand up.
        player.dodge_roll_timer = LEDGE_ROLL_TIME + 0.10;
        player.ledge_grab = Some(state);
        events.op(player, MovementOp::LedgeRoll);
        return true;
    }
    if want_ledge_jump {
        let away_x = away_from_platform_axis(state.contact);
        player.wall_clinging = false;
        player.wall_climbing = false;
        player.on_wall = false;
        player.on_ground = false;
        player.ledge_grab = None;
        player.vel = Vec2::new(away_x * tuning.wall_jump_x, -tuning.jump_speed);
        player.refresh_movement_resources(tuning);
        events.op(player, MovementOp::LedgeJump);
        return true;
    }
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
        state.getup_kind = LedgeGetupKind::Climb;
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
    // Smash-Bros style ledge intangibility: a brief invuln window on
    // grab so the player can't be edge-guarded the instant they
    // touch a corner. Reuses `dodge_roll_timer` because that field
    // already gates damage (`PlayerBody::dodge_rolling`) — same
    // pipeline, single source of truth.
    if player.dodge_roll_timer < LEDGE_GRAB_INVULN_TIME {
        player.dodge_roll_timer = LEDGE_GRAB_INVULN_TIME;
    }
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

    fn make_hanging_player(contact: LedgeContact) -> crate::movement::Player {
        let mut player = crate::movement::Player::new(Vec2::ZERO);
        player.abilities.ledge_grab = true;
        player.abilities.shield = true;
        player.ledge_grab = Some(LedgeGrabState {
            contact,
            elapsed: LEDGE_MIN_CLIMB_DELAY + 0.01,
            climbing: false,
            getup_kind: LedgeGetupKind::Climb,
            climb_elapsed: 0.0,
        });
        player.wall_clinging = true;
        player.on_wall = true;
        player
    }

    #[test]
    fn ledge_jump_away_launches_player_outward() {
        // Wall on the right of the player (wall_normal_x = -1, pushes left).
        // away_from_platform = left = negative x.
        let contact = LedgeContact {
            wall_normal_x: -1.0,
            anchor: Vec2::new(86.0, 110.0),
            climb_target: Vec2::new(115.0, 77.0),
        };
        let mut player = make_hanging_player(contact);
        let mut events = crate::movement::FrameEvents::default();
        let tuning = crate::movement::MovementTuning::default();
        let input = InputState {
            jump_pressed: true,
            axis_x: -1.0, // pressing away from the platform (away = wall_normal direction = -1)
            ..InputState::default()
        };
        let consumed = tick_active_ledge_grab(&mut player, input, 0.016, tuning, &mut events);
        assert!(consumed, "tick should consume the frame");
        assert!(player.ledge_grab.is_none(), "ledge should be released");
        // Player should move left (away from the right-side wall).
        assert!(player.vel.x < -100.0, "should have leftward velocity, got {}", player.vel.x);
        assert!(player.vel.y < -100.0, "should have upward velocity, got {}", player.vel.y);
        assert!(!player.on_wall, "should not be on wall");
    }

    #[test]
    fn jump_toward_platform_still_climbs() {
        let contact = LedgeContact {
            wall_normal_x: -1.0,
            anchor: Vec2::new(86.0, 110.0),
            climb_target: Vec2::new(115.0, 77.0),
        };
        let mut player = make_hanging_player(contact);
        let mut events = crate::movement::FrameEvents::default();
        let tuning = crate::movement::MovementTuning::default();
        // Pressing toward the platform (right = into wall side) with jump.
        let input = InputState {
            jump_pressed: true,
            axis_x: 1.0, // pressing into the platform (into = -wall_normal = +1)
            ..InputState::default()
        };
        let consumed = tick_active_ledge_grab(&mut player, input, 0.016, tuning, &mut events);
        assert!(consumed);
        // Should start climbing, not a ledge jump.
        if let Some(state) = player.ledge_grab {
            assert!(state.climbing, "should be in climbing state");
        } else {
            // Also OK: climb finished immediately (degenerate case)
        }
    }

    /// Holding shield while hanging on a ledge triggers a Smash-Bros
    /// style roll: the getup_kind switches to Roll, the player starts
    /// climbing (interpolating along the roll trajectory), and
    /// `dodge_roll_timer` is set so the player is invulnerable for
    /// the duration of the roll.
    #[test]
    fn shield_held_starts_a_ledge_roll() {
        let contact = LedgeContact {
            wall_normal_x: -1.0,
            anchor: Vec2::new(86.0, 110.0),
            climb_target: Vec2::new(115.0, 77.0),
        };
        let mut player = make_hanging_player(contact);
        let mut events = crate::movement::FrameEvents::default();
        let tuning = crate::movement::MovementTuning::default();
        let input = InputState {
            shield_held: true,
            ..InputState::default()
        };
        let consumed = tick_active_ledge_grab(&mut player, input, 0.016, tuning, &mut events);
        assert!(consumed);
        let state = player.ledge_grab.expect("roll should leave a transitioning state");
        assert!(state.climbing, "roll must enter the climbing state");
        assert_eq!(state.getup_kind, LedgeGetupKind::Roll);
        assert!(
            player.dodge_roll_timer > 0.0,
            "ledge roll must arm dodge_roll_timer for invuln",
        );
    }

    /// Shield wins over climb when both inputs are present (e.g. Up
    /// + Shield). Matches Smash where shield-from-ledge is the
    /// universal roll cue regardless of stick direction.
    #[test]
    fn shield_overrides_climb_when_both_inputs_are_held() {
        let contact = LedgeContact {
            wall_normal_x: -1.0,
            anchor: Vec2::new(86.0, 110.0),
            climb_target: Vec2::new(115.0, 77.0),
        };
        let mut player = make_hanging_player(contact);
        let mut events = crate::movement::FrameEvents::default();
        let tuning = crate::movement::MovementTuning::default();
        let input = InputState {
            shield_held: true,
            axis_y: -1.0, // up — would otherwise climb
            ..InputState::default()
        };
        let _ = tick_active_ledge_grab(&mut player, input, 0.016, tuning, &mut events);
        let state = player.ledge_grab.expect("active transition");
        assert_eq!(state.getup_kind, LedgeGetupKind::Roll);
    }

    /// At the end of the roll the player lands FURTHER inboard than a
    /// climb would, by ``LEDGE_ROLL_OVERSHOOT`` along the into-platform
    /// axis. That overshoot is what makes the roll feel like a real
    /// commitment past the ledge edge instead of a snappier climb.
    #[test]
    fn ledge_roll_lands_further_inboard_than_climb() {
        let contact = LedgeContact {
            wall_normal_x: -1.0,
            anchor: Vec2::new(86.0, 110.0),
            climb_target: Vec2::new(115.0, 77.0),
        };
        let mut player = make_hanging_player(contact);
        let mut events = crate::movement::FrameEvents::default();
        let tuning = crate::movement::MovementTuning::default();
        // Start the roll.
        let _ = tick_active_ledge_grab(
            &mut player,
            InputState { shield_held: true, ..InputState::default() },
            0.001,
            tuning,
            &mut events,
        );
        // Run the full roll duration in one big tick to land.
        let _ = tick_active_ledge_grab(
            &mut player,
            InputState::default(),
            LEDGE_ROLL_TIME + 0.05,
            tuning,
            &mut events,
        );
        // After the roll finishes, the player should be at the roll
        // landing position which is past climb_target along the
        // into-platform axis. For a -1 wall normal that's +x.
        assert!(player.ledge_grab.is_none(), "roll should have finished");
        assert!(player.on_ground, "roll lands the player on the platform");
        let expected = roll_target(contact);
        assert!(
            (player.pos.x - expected.x).abs() < 0.5,
            "expected roll landing x ≈ {}, got {}",
            expected.x,
            player.pos.x,
        );
        assert!(
            (expected.x - contact.climb_target.x).abs() >= LEDGE_ROLL_OVERSHOOT - 0.01,
            "roll target must overshoot the climb target by ~{}px",
            LEDGE_ROLL_OVERSHOOT,
        );
    }

    /// Grabbing a ledge grants brief intangibility via
    /// ``Player::dodge_roll_timer`` so an edge-guarding hit can't
    /// punish the moment of contact.
    #[test]
    fn ledge_grab_arms_intangibility_window() {
        let world = world_with(vec![Block::solid(
            "ledge",
            Vec2::new(100.0, 100.0),
            Vec2::new(200.0, 200.0),
        )]);
        let mut player = crate::movement::Player::new(Vec2::new(86.0, 110.0));
        player.abilities.ledge_grab = true;
        player.wall_clinging = true;
        player.wall_normal_x = -1.0;
        let mut events = crate::movement::FrameEvents::default();
        let latched = try_start_ledge_grab(&world, &mut player, InputState::default(), &mut events);
        assert!(latched, "expected ledge grab to latch");
        assert!(
            player.dodge_roll_timer >= LEDGE_GRAB_INVULN_TIME - 0.001,
            "grab should arm at least {}s of invuln, got {}",
            LEDGE_GRAB_INVULN_TIME,
            player.dodge_roll_timer,
        );
    }
}
