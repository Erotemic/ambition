//! Ledge-grab runtime: probing for a grabbable ledge, ticking the active
//! climb/roll/getup clusters, grab classification, and the launch-boost math.

use super::*;
use crate::geometry::{Aabb, AabbExt};

fn launch_away_from_feet(
    frame: crate::MotionFrame,
    tuning: AxisSweptParams,
    platform_side_axis: f32,
    platform_speed: f32,
) -> Vec2 {
    frame.side() * (platform_side_axis * platform_speed)
        - frame.down() * tuning.locomotion.jump_speed
}

fn point_from_frame_coords(frame: crate::AccelerationFrame, side: f32, down: f32) -> Vec2 {
    frame.side * side + frame.down * down
}

fn half_along_axis(half: Vec2, axis: Vec2) -> f32 {
    half.x * axis.x.abs() + half.y * axis.y.abs()
}

fn inside_world_bounds(center: Vec2, half: Vec2, world: &World) -> bool {
    center.x - half.x >= 0.0
        && center.y - half.y >= 0.0
        && center.x + half.x <= world.size.x
        && center.y + half.y <= world.size.y
}

/// How much slack the latch-matching below allows between a stored contact and
/// the contact this block WOULD produce. One discrete tick of platform travel
/// plus the probe's own `-1.0`/`-4.0` face offsets already put the two several
/// pixels apart, so the tolerance is the matcher's, not a fudge factor.
const LEDGE_LATCH_TOL: f32 = 8.0;

/// Is `contact` the ledge of THIS block, under this frame?
///
/// Same file, same constants, one place to change.
///
/// The final range test rejects a block that merely shares a lip coordinate
/// with the real one — the climb target must be inboard of THIS block.
pub fn ledge_contact_is_on_block(
    contact: LedgeContact,
    body_size: Vec2,
    block: Aabb,
    gravity_dir: Vec2,
) -> bool {
    if contact.wall_normal_x.abs() < 0.5 {
        return false;
    }
    let frame = crate::AccelerationFrame::new(gravity_dir);
    let half = body_size * 0.5;
    let side_normal = contact.wall_normal_x.signum();
    let block_half = block.half_size();
    let block_center = block.center();
    let block_side = block_center.dot(frame.side);
    let block_down = block_center.dot(frame.down);
    let block_side_half = half_along_axis(block_half, frame.side);
    let block_down_half = half_along_axis(block_half, frame.down);
    let lip_down = block_down - block_down_half;
    let wall_side = block_side + side_normal * block_side_half;

    // The inverses of `hang_center` and `climb_target` in the probe above.
    let expected_anchor_side = wall_side + side_normal * (half.x - 1.0);
    let expected_anchor_down = lip_down + half.y - 4.0;
    let expected_climb_side = wall_side - side_normal * (half.x + 4.0);
    let expected_climb_down = lip_down - half.y - 1.0;

    let anchor_side = contact.anchor.dot(frame.side);
    let anchor_down = contact.anchor.dot(frame.down);
    let climb_side = contact.climb_target.dot(frame.side);
    let climb_down = contact.climb_target.dot(frame.down);

    if (anchor_side - expected_anchor_side).abs() > LEDGE_LATCH_TOL
        || (anchor_down - expected_anchor_down).abs() > LEDGE_LATCH_TOL
        || (climb_side - expected_climb_side).abs() > LEDGE_LATCH_TOL
        || (climb_down - expected_climb_down).abs() > LEDGE_LATCH_TOL
    {
        return false;
    }

    climb_side >= block_side - block_side_half - half.x - 12.0
        && climb_side <= block_side + block_side_half + half.x + 12.0
}

/// What a moving solid does to a body hanging on its ledge this step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LedgeCarry {
    /// Nothing is carrying this hang — no moving solid owns the latched ledge.
    /// The overwhelmingly common case, including every static ledge.
    Stay,
    /// Ride the solid by this displacement.
    Carry(Vec2),
    /// The carry would push the body into a DIFFERENT solid, so the hang breaks
    /// instead of the body being shoved through a wall (#126).
    KnockOff,
}

/// The one contact rule for a ledge hang on moving geometry, phrased exactly
/// like the grounded ride: *a body attached to a moving solid is carried by that
/// solid's [`crate::world::Block::velocity`].* Static geometry carries `ZERO`, so
/// a hang on an ordinary wall is the degenerate case rather than a separate road.
///
/// Reading `Block::velocity` off the collision world instead removes the parameter, and with it the
/// reason the rule could only run on one body.
///
/// the carrier is excluded from the wall test by IDENTITY, not by kind. The
/// old site passed the *base* world, relying on the platform being composited in
/// separately; that is an accident of composition order, and it would have
/// started knocking every rider off the first time a moving solid was authored
/// into the base. The block we are latched to is the one block that cannot be
/// the wall we are carried into.
pub fn ledge_carry_for_frame(
    world: &World,
    contact: LedgeContact,
    body_aabb: Aabb,
    body_size: Vec2,
    gravity_dir: Vec2,
) -> LedgeCarry {
    // Only a MOVING solid can carry anything, and the zero-velocity test is a
    // cheap `Vec2` compare that skips the matcher for every static block — which
    // in a room with no moving platforms is all of them.
    let Some(carrier) = world.blocks.iter().find(|block| {
        block.velocity != Vec2::ZERO
            && ledge_surface_kind(block.kind)
            && (ledge_contact_is_on_block(contact, body_size, block.aabb, gravity_dir)
                // The solid advanced before the body simulation, so a contact
                // stored last tick still describes where it WAS. Matching both
                // poses keeps the hang glued across the advance instead of
                // losing it in the one frame between the two.
                || ledge_contact_is_on_block(
                    contact,
                    body_size,
                    block.aabb.translated(-block.velocity),
                    gravity_dir,
                ))
    }) else {
        return LedgeCarry::Stay;
    };

    let delta = carrier.velocity;
    let carried = body_aabb.translated(delta);
    // `Solid` only, exactly as before: a one-way is not a wall you can be shoved
    // through, and a `BlinkWall` is what a moving platform itself is composited
    // as — widening this would have one platform knock riders off another.
    let into_wall = world.blocks.iter().any(|block| {
        !std::ptr::eq(block, carrier)
            && matches!(block.kind, BlockKind::Solid)
            && carried.strict_intersects(block.aabb)
    });
    if into_wall {
        LedgeCarry::KnockOff
    } else {
        LedgeCarry::Carry(delta)
    }
}

/// `OneWay`) whose anti-gravity face is within a shoulder-height band of the
/// controlled body and whose side face matches the side the body is reaching
/// toward. If found, returns the snap anchor and the climb target.
///
/// `wall_normal_x` is historical naming: it is now the side-face normal expressed
/// in the controlled body's local side axis (`+1` = platform on local-left,
/// `-1` = platform on local-right). The public wrapper below keeps the old
/// down-gravity signature for tests and legacy callers.
pub fn probe_ledge_grab_in_frame(
    player_pos: Vec2,
    player_size: Vec2,
    wall_normal_x: f32,
    world: &World,
    gravity_dir: Vec2,
) -> Option<LedgeContact> {
    // AMBITION_REVIEW(discrete_ok): CC2 §3.3 ledge audit — this probe fires
    // OFF a resolved wall contact (`wall_normal_x`, a Class-A kernel output;
    // the body is already clung/resting against the wall this frame), not off a
    // path-dependent trigger overlap. There is no endpoint sample a fast body
    // could tunnel: you cannot reach the probe without the swept resolve first
    // planting you on the wall. Swept by construction; nothing to convert.
    if wall_normal_x.abs() < 0.5 {
        return None;
    }
    let side_normal = wall_normal_x.signum();
    let frame = crate::AccelerationFrame::new(gravity_dir);
    let player_half_local = player_size * 0.5;
    let player_half_world = frame.to_world_half(player_half_local);
    let player_side_half = player_half_local.x;
    let player_down_half = player_half_local.y;

    let player_side = player_pos.dot(frame.side);
    let player_down = player_pos.dot(frame.down);

    // Window where the ledge anti-gravity face must sit. Under normal gravity
    // this is the old world-Y top-face / head-Y band.
    let head_down = player_down - player_down_half;
    let reach_min_down = head_down - LEDGE_REACH_UP;
    let reach_max_down = head_down + LEDGE_REACH_DOWN;

    // The controlled body's side that is reaching toward the wall, expressed in
    // local side coordinates. side_normal = +1 means the block face normal points
    // toward local-right, so the actor reaches with its local-left side.
    let cling_side = player_side - side_normal * player_side_half;

    let mut best: Option<LedgeContact> = None;
    for block in &world.blocks {
        if !ledge_surface_kind(block.kind) {
            continue;
        }
        let block_half = block.aabb.half_size();
        let block_center = block.aabb.center();
        let block_side_half = half_along_axis(block_half, frame.side);
        let block_down_half = half_along_axis(block_half, frame.down);
        let block_side = block_center.dot(frame.side);
        let block_down = block_center.dot(frame.down);
        let lip_down = block_down - block_down_half;
        if lip_down < reach_min_down || lip_down > reach_max_down {
            continue;
        }

        // Pick the platform side face matching the controlled body's reach side.
        let block_wall_side = block_side + side_normal * block_side_half;
        if (block_wall_side - cling_side).abs() > LEDGE_HORIZONTAL_REACH {
            continue;
        }

        // The space directly above the platform (away from the feet) must be clear.
        let probe_center = point_from_frame_coords(
            frame,
            block_wall_side - side_normal * (player_side_half - 1.0),
            lip_down - player_down_half - 1.0,
        );
        if !inside_world_bounds(probe_center, player_half_world, world) {
            continue;
        }
        let probe_aabb = Aabb::new(probe_center, player_half_world - Vec2::new(2.0, 2.0));
        let blocked = world.body_overlaps_any(probe_aabb, |b| {
            ledge_clearance_blocker_kind(b.kind) && !std::ptr::eq(b, block)
        });
        if blocked {
            continue;
        }

        let hang_center = point_from_frame_coords(
            frame,
            block_wall_side + side_normal * (player_side_half - 1.0),
            lip_down + player_down_half - 4.0,
        );
        let hang_aabb = Aabb::new(hang_center, player_half_world - Vec2::new(2.0, 2.0));
        let hang_blocked = world.body_overlaps_any(hang_aabb, |b| {
            ledge_clearance_blocker_kind(b.kind) && !std::ptr::eq(b, block)
        });
        if hang_blocked {
            continue;
        }

        let climb_target = point_from_frame_coords(
            frame,
            block_wall_side - side_normal * (player_side_half + 4.0),
            lip_down - player_down_half - 1.0,
        );
        let candidate = LedgeContact {
            wall_normal_x: side_normal,
            anchor: hang_center,
            climb_target,
        };
        let new_distance = (lip_down - head_down).abs();
        let keep = match best {
            None => true,
            Some(prev) => {
                let prev_lip_down = prev.anchor.dot(frame.down) - player_down_half + 4.0;
                let prev_distance = (prev_lip_down - head_down).abs();
                new_distance < prev_distance
            }
        };
        if keep {
            best = Some(candidate);
        }
    }
    best
}

/// TODO(compat-remove): migrate callers to [`probe_ledge_grab_in_frame`] and delete this
/// down-gravity wrapper.
pub fn probe_ledge_grab(
    player_pos: Vec2,
    player_size: Vec2,
    wall_normal_x: f32,
    world: &World,
) -> Option<LedgeContact> {
    probe_ledge_grab_in_frame(
        player_pos,
        player_size,
        wall_normal_x,
        world,
        Vec2::new(0.0, 1.0),
    )
}

/// If the player is currently hanging/climbing, advance that state and return
/// true to indicate that the normal movement integrator should not run this
/// frame. Frame-explicit: the caller supplies the environment-resolved frame.
/// `axis_state` is the axis policy's model-private maneuver state — the hang
/// itself (`ledge_grab`), the wall engagement flags, and the invuln roll
/// timer all live there; the shared clusters keep only contact facts and the
/// re-grab cooldown.
pub fn tick_active_ledge_grab_clusters_in_frame(
    clusters: &mut crate::body_clusters::BodyClustersMut<'_>,
    axis_state: &mut crate::movement::AxisManeuverState,
    input: InputState,
    dt: f32,
    frame: crate::MotionFrame,
    tuning: AxisSweptParams,
    events: &mut crate::movement::FrameEvents,
) -> bool {
    let Some(mut state) = axis_state.ledge_grab else {
        return false;
    };
    if !clusters.abilities.abilities.ledge_grab {
        axis_state.ledge_grab = None;
        return false;
    }

    state.elapsed += dt;
    clusters.kinematics.facing = into_platform_axis(state.contact);

    // ⭐ THE HANG IS ON A CLOCK. Checked BEFORE the getup branch so a body that
    // has already committed to a climb, roll or getup attack finishes it — the
    // limit ends a HANG, and cancelling an animation mid-way would read as the
    // ledge eating an input the player already spent.
    //
    // The body is dropped exactly as `want_drop` drops it, cooldown and all: the
    // genre forces you OFF the ledge, it does not put you on the stage, and a
    // drop that did not arm the re-grab lockout would re-latch on the next frame
    // and the limit would do nothing at all.
    if !state.climbing
        && super::LEDGE_HANG_MAX_TIME > 0.0
        && state.elapsed >= super::LEDGE_HANG_MAX_TIME
    {
        axis_state.wall_clinging = false;
        axis_state.wall_climbing = false;
        clusters.wall.on_wall = false;
        axis_state.ledge_grab = None;
        clusters.ledge.release_cooldown = LEDGE_REGRAB_COOLDOWN;
        events.op_clusters(clusters.combo_trace, MovementOp::LedgeDrop);
        return true;
    }

    if state.climbing {
        state.climb_elapsed += dt;
        let duration_scale = ledge_getup_duration_scale(state, &tuning);
        let duration = state.getup_duration() * duration_scale;
        let progress = (state.climb_elapsed / duration).clamp(0.0, 1.0);
        clusters.kinematics.pos = getup_position(state, progress, frame.down());
        clusters.kinematics.vel = Vec2::ZERO;
        clusters.ground.on_ground = false;
        axis_state.wall_clinging = false;
        axis_state.wall_climbing = false;
        clusters.wall.on_wall = false;

        if progress >= 1.0 {
            clusters.kinematics.pos = getup_end_position(state, frame.down());
            // Carry HORIZONTAL momentum into exit; drop the Y so the
            // player doesn't relaunch off the platform they just stood
            // on. (Ledge-jump path keeps Y because that's a hop.)
            let boost = ledge_boost_for_state_in_frame(state, frame, &tuning);
            clusters.kinematics.vel = boost - frame.down() * boost.dot(frame.down());
            clusters.ground.on_ground = true;
            axis_state.wall_clinging = false;
            axis_state.wall_climbing = false;
            clusters.wall.on_wall = false;
            axis_state.ledge_grab = None;
            events.op_clusters(clusters.combo_trace, MovementOp::LedgeClimbFinish);
        } else {
            axis_state.ledge_grab = Some(state);
        }
        return true;
    }

    // Player-frame "descend": "up" = away from the feet (climb up the ledge),
    // "down" = toward the feet (drop). Gravity- + input-mode-relative via the
    // resolved stick `y`.
    let local_stick = input.local_axis();
    let input_up = local_stick.y < -0.4;
    let input_down = local_stick.y > 0.4;
    let input_into_platform = local_stick.x * into_platform_axis(state.contact) > 0.4;
    let input_away_from_platform = local_stick.x * away_from_platform_axis(state.contact) > 0.4;
    let climb_unlocked = state.elapsed >= LEDGE_MIN_CLIMB_DELAY;

    let want_roll = climb_unlocked && input.shield_held && clusters.abilities.abilities.shield;
    let want_ledge_release =
        climb_unlocked && !want_roll && input.jump_pressed() && input_away_from_platform;
    let want_ledge_jump =
        climb_unlocked && !want_roll && !want_ledge_release && input.jump_pressed();
    let want_getup_attack = climb_unlocked
        && !want_roll
        && !want_ledge_release
        && !want_ledge_jump
        && input.attack_pressed;
    let want_climb = climb_unlocked
        && !want_roll
        && !want_ledge_release
        && !want_ledge_jump
        && !want_getup_attack
        && (input_up
            || input.interact_pressed
            || (state.elapsed >= LEDGE_TOWARD_CLIMB_DELAY && input_into_platform));
    let want_drop = !want_roll
        && !want_ledge_jump
        && !want_getup_attack
        && (input_down || (input_away_from_platform && !want_ledge_release));

    if want_roll {
        state.climbing = true;
        state.getup_kind = LedgeGetupKind::Roll;
        state.climb_elapsed = 0.0;
        clusters.kinematics.pos = state.contact.anchor;
        clusters.kinematics.vel = Vec2::ZERO;
        clusters.ground.on_ground = false;
        axis_state.wall_clinging = false;
        axis_state.wall_climbing = false;
        clusters.wall.on_wall = false;
        axis_state.ledge_invuln_timer = LEDGE_ROLL_TIME + 0.10;
        axis_state.ledge_grab = Some(state);
        events.op_clusters(clusters.combo_trace, MovementOp::LedgeRoll);
        return true;
    }
    if want_ledge_release {
        let away_x = away_from_platform_axis(state.contact);
        axis_state.wall_clinging = false;
        axis_state.wall_climbing = false;
        clusters.wall.on_wall = false;
        clusters.ground.on_ground = false;
        axis_state.ledge_grab = None;
        clusters.ledge.release_cooldown = LEDGE_REGRAB_COOLDOWN;
        clusters.kinematics.vel =
            launch_away_from_feet(frame, tuning, away_x, tuning.locomotion.wall_jump_x);
        events.op_clusters(clusters.combo_trace, MovementOp::LedgeJump);
        return true;
    }
    if want_ledge_jump {
        let into_x = into_platform_axis(state.contact);
        axis_state.wall_clinging = false;
        axis_state.wall_climbing = false;
        clusters.wall.on_wall = false;
        clusters.ground.on_ground = false;
        axis_state.ledge_grab = None;
        clusters.ledge.release_cooldown = LEDGE_REGRAB_COOLDOWN;
        let mut launch =
            launch_away_from_feet(frame, tuning, into_x, tuning.locomotion.jump_speed * 0.35);
        launch += ledge_boost_for_state_in_frame(state, frame, &tuning);
        clusters.kinematics.vel = launch;
        events.op_clusters(clusters.combo_trace, MovementOp::LedgeJump);
        return true;
    }
    if want_drop && !want_climb && !want_getup_attack {
        axis_state.wall_clinging = false;
        axis_state.wall_climbing = false;
        clusters.wall.on_wall = false;
        axis_state.ledge_grab = None;
        clusters.ledge.release_cooldown = LEDGE_REGRAB_COOLDOWN;
        events.op_clusters(clusters.combo_trace, MovementOp::LedgeDrop);
        return true;
    }
    if want_getup_attack {
        state.climbing = true;
        state.getup_kind = LedgeGetupKind::Attack;
        state.climb_elapsed = 0.0;
        clusters.kinematics.pos = state.contact.anchor;
        clusters.kinematics.vel = Vec2::ZERO;
        clusters.ground.on_ground = false;
        axis_state.wall_clinging = false;
        axis_state.wall_climbing = false;
        clusters.wall.on_wall = false;
        // ⚠ the whole attack is intangible, and that is a decision rather
        // than an inheritance now — see `LEDGE_GETUP_ATTACK_INVULN`.
        axis_state.ledge_invuln_timer = super::LEDGE_GETUP_ATTACK_INVULN;
        axis_state.ledge_grab = Some(state);
        events.op_clusters(clusters.combo_trace, MovementOp::LedgeGetupAttack);
        events.op_clusters(clusters.combo_trace, MovementOp::Slash);
        return true;
    }
    if want_climb {
        state.climbing = true;
        state.getup_kind = LedgeGetupKind::Climb;
        state.climb_elapsed = 0.0;
        clusters.kinematics.pos = state.contact.anchor;
        clusters.kinematics.vel = Vec2::ZERO;
        clusters.ground.on_ground = false;
        axis_state.wall_clinging = false;
        axis_state.wall_climbing = false;
        clusters.wall.on_wall = false;
        axis_state.ledge_grab = Some(state);
        events.op_clusters(clusters.combo_trace, MovementOp::LedgeClimbStart);
        return true;
    }

    // Default: stay in the hang. Re-pin pos to the anchor and zero vel
    // so gravity / wall-slide doesn't drift the player off the lip.
    clusters.kinematics.pos = state.contact.anchor;
    clusters.kinematics.vel = Vec2::ZERO;
    axis_state.wall_clinging = true;
    axis_state.wall_climbing = false;
    clusters.wall.on_wall = true;
    axis_state.ledge_grab = Some(state);
    true
}

/// Classify whether the grab geometry also satisfies the original,
/// tighter probe window in the controlled body's local frame. Widened ledge
/// grabs are still valid catches, but they intentionally do not earn
/// ledge-momentum boost rewards.
pub fn classify_ledge_grab_in_frame(
    player_pos: Vec2,
    player_size: Vec2,
    contact: LedgeContact,
    gravity_dir: Vec2,
) -> LedgeGrabQuality {
    let frame = crate::AccelerationFrame::new(gravity_dir);
    let half = player_size * 0.5;
    let player_side = player_pos.dot(frame.side);
    let player_down = player_pos.dot(frame.down);
    let head_down = player_down - half.y;
    let precise_min_down = head_down - LEDGE_PRECISE_REACH_UP;
    let precise_max_down = head_down + LEDGE_PRECISE_REACH_DOWN;

    // Invert the local-frame anchor formula from `probe_ledge_grab_in_frame`:
    // anchor.side = block_wall_side + wall_normal * (half.x - 1)
    // anchor.down = lip_down + half.y - 4
    let anchor_side = contact.anchor.dot(frame.side);
    let anchor_down = contact.anchor.dot(frame.down);
    let block_wall_side = anchor_side - contact.wall_normal_x * (half.x - 1.0);
    let lip_down = anchor_down - half.y + 4.0;
    let cling_side = player_side - contact.wall_normal_x * half.x;

    if lip_down >= precise_min_down
        && lip_down <= precise_max_down
        && (block_wall_side - cling_side).abs() <= LEDGE_PRECISE_HORIZONTAL_REACH
    {
        LedgeGrabQuality::Precise
    } else {
        LedgeGrabQuality::Forgiving
    }
}

/// TODO(compat-remove): migrate callers to [`classify_ledge_grab_in_frame`] and delete this
/// down-gravity wrapper.
pub fn classify_ledge_grab(
    player_pos: Vec2,
    player_size: Vec2,
    contact: LedgeContact,
) -> LedgeGrabQuality {
    classify_ledge_grab_in_frame(player_pos, player_size, contact, Vec2::new(0.0, 1.0))
}

/// Convenience predicate for call sites/tests that only care about the
/// precision reward gate.
pub fn is_precise_ledge_grab(player_pos: Vec2, player_size: Vec2, contact: LedgeContact) -> bool {
    classify_ledge_grab(player_pos, player_size, contact).is_precise()
}

/// Pick a side-face normal to probe for a ledge: the active wall-cling normal
/// first (engagement from the axis maneuver state, face from the shared wall
/// contact), else a local side-axis press while airborne.
fn requested_wall_normal_clusters(
    wall: &crate::body_clusters::BodyWallState,
    axis_state: &crate::movement::AxisManeuverState,
    ground: &crate::body_clusters::BodyGroundState,
    input: InputState,
) -> Option<f32> {
    if axis_state.wall_clinging && wall.wall_normal_x.abs() >= 0.5 {
        return Some(wall.wall_normal_x);
    }
    let local_stick = input.local_axis();
    if !ground.on_ground && local_stick.x.abs() > LEDGE_GRAB_INTENT_DEADZONE {
        return Some(-local_stick.x.signum());
    }
    None
}

/// Compute the momentum-carry boost vector for a getup option.
///
/// Returns a velocity to ADD to the launch / post-transition velocity
/// of an eligible getup (climb / roll / attack / vertical ledge-jump).
/// Returns zero when:
/// - The mechanic is disabled via `tuning.abilities.ledge_momentum.window == 0.0`.
/// - The window has elapsed (so a player who lingered on the ledge
///   doesn't claim a stale boost when they finally act).
/// - The carried component is in a direction that wouldn't count as
///   "moving toward the platform" (backward X) or "rising" (downward Y).
///
/// `elapsed_at_initiation` is the grab-to-action time at the moment
/// the getup was first committed to — for transitions, the state
/// machine ticks `climb_elapsed` after that point, so subtract it
/// from `state.elapsed` at the call site. (See [`ledge_boost_for_state`]
/// which does that subtraction for you.)
pub fn ledge_boost_in_frame(
    momentum_at_grab: Vec2,
    contact: LedgeContact,
    elapsed_at_initiation: f32,
    frame: crate::MotionFrame,
    tuning: &AxisSweptParams,
) -> Vec2 {
    let cfg = tuning.abilities.ledge_momentum;
    if cfg.window <= 0.0 || elapsed_at_initiation > cfg.window {
        return Vec2::ZERO;
    }
    // Linear decay across the window — full boost at t=0, zero at
    // t=window. Easier to reason about while tuning than smoothstep.
    let weight = 1.0 - (elapsed_at_initiation / cfg.window).clamp(0.0, 1.0);
    let m = momentum_at_grab;
    let basis = frame.basis();
    // Only count side-axis momentum that points INTO the platform. Reverse
    // momentum at grab time meant the actor was sliding off the lip — no reward.
    let side_speed = m.dot(basis.side);
    let into = into_platform_axis(contact);
    let forward_into = side_speed * into;
    let carried_side = if forward_into > 0.0 {
        basis.side * (side_speed * cfg.x_gain * weight).clamp(-cfg.x_cap, cfg.x_cap)
    } else {
        Vec2::ZERO
    };
    // Carry only momentum that is away from the feet. Under normal gravity this
    // is the old `m.y < 0` upward check; under flipped/sideways gravity it is the
    // same rule in the controlled body's acceleration frame.
    let carried_away = {
        let along_down = m.dot(basis.down);
        if along_down < 0.0 {
            basis.down * (along_down * cfg.y_gain * weight).clamp(-cfg.y_cap, cfg.y_cap)
        } else {
            Vec2::ZERO
        }
    };
    carried_side + carried_away
}

/// Convenience: compute the boost from a [`LedgeGrabState`]. For
/// transitions that have already started ticking `climb_elapsed`,
/// subtracts that from `elapsed` to recover the grab-to-action time.
pub fn ledge_boost_for_state_in_frame(
    state: LedgeGrabState,
    frame: crate::MotionFrame,
    tuning: &AxisSweptParams,
) -> Vec2 {
    if !state.grab_quality.is_precise() {
        return Vec2::ZERO;
    }
    let elapsed_at_initiation = (state.elapsed - state.climb_elapsed).max(0.0);
    ledge_boost_in_frame(
        state.momentum_at_grab,
        state.contact,
        elapsed_at_initiation,
        frame,
        tuning,
    )
}

pub fn ledge_boost_weight_for_state(state: LedgeGrabState, tuning: &AxisSweptParams) -> f32 {
    if !state.grab_quality.is_precise() {
        return 0.0;
    }
    let cfg = tuning.abilities.ledge_momentum;
    if cfg.window <= 0.0 {
        return 0.0;
    }
    let elapsed_at_initiation = (state.elapsed - state.climb_elapsed).max(0.0);
    (1.0 - (elapsed_at_initiation / cfg.window).clamp(0.0, 1.0)).max(0.0)
}

/// Scale a getup transition duration by the carried-momentum weight.
/// `duration_scale = 1.0 / (1.0 + weight * gain)`. With `gain = 1.0`
/// and full weight, a 0.24-s climb becomes ~0.12 s — exactly the
/// "no stop-and-go" feel a quick getup should have.
pub fn ledge_getup_duration_scale(state: LedgeGrabState, tuning: &AxisSweptParams) -> f32 {
    let weight = ledge_boost_weight_for_state(state, tuning);
    1.0 / (1.0 + weight * tuning.abilities.ledge_momentum.getup_speedup_gain)
}

/// Minimum downward velocity for the no-input falling auto-snap
/// trigger. Kept above gentle drift so a player who is loitering near
/// a ledge with no stick input doesn't get snagged by accident, but
/// low enough that normal falling recovery catches near-miss lips.
const FALL_SNAP_MIN_VY: f32 = 45.0;

/// Probe for and start a new ledge grab after normal collision has established
/// this frame's wall/airborne state. Returns true when a new grab latched.
///
/// Two snap paths:
///
/// - Intentional snap: the player is wall-clinging or actively
///   moving toward a wall (local side input non-zero while airborne).
///   `requested_wall_normal` returns the side to probe.
/// - Falling-into-ledge snap: the player is falling fast and a
///   grabbable ledge sits within reach on either side. Mirrors
///   Smash's auto-snap on a descending recovery — you don't have
///   to hold a stick into the wall to catch the lip you're already
///   trying to grab.
///
/// Attempt to latch a ledge grab this frame: requires the
/// `ledge_grab` ability, no current grab, an airborne body, no
/// release cooldown, and either a wall-cling axis press or a fast
/// falling auto-snap into a grabbable lip. On a successful latch,
/// captures the pre-wall momentum + arms the post-grab invuln.
/// Frame-explicit: the caller supplies the environment-resolved frame.
pub fn try_start_ledge_grab_clusters_in_frame(
    world: &World,
    clusters: &mut crate::body_clusters::BodyClustersMut<'_>,
    axis_state: &mut crate::movement::AxisManeuverState,
    input: InputState,
    frame: crate::MotionFrame,
    tuning: &AxisSweptParams,
    events: &mut crate::movement::FrameEvents,
) -> bool {
    if !clusters.abilities.abilities.ledge_grab
        || axis_state.ledge_grab.is_some()
        || clusters.ground.on_ground
    {
        return false;
    }
    if clusters.ledge.release_cooldown > 0.0 {
        return false;
    }

    let mut contact: Option<LedgeContact> = None;
    if let Some(wall_normal) =
        requested_wall_normal_clusters(clusters.wall, axis_state, clusters.ground, input)
    {
        contact = probe_ledge_grab_in_frame(
            clusters.kinematics.pos,
            clusters.kinematics.size,
            wall_normal,
            world,
            frame.down(),
        );
    }
    if contact.is_none() && clusters.kinematics.vel.dot(frame.down()) > FALL_SNAP_MIN_VY {
        // Smash-style auto-snap during a falling recovery: try BOTH
        // sides and snap to whichever has a grabbable lip in the chin
        // band.
        for trial_normal in [-1.0_f32, 1.0_f32] {
            if let Some(found) = probe_ledge_grab_in_frame(
                clusters.kinematics.pos,
                clusters.kinematics.size,
                trial_normal,
                world,
                frame.down(),
            ) {
                contact = Some(found);
                break;
            }
        }
    }
    let Some(contact) = contact else {
        return false;
    };

    let grab_quality = classify_ledge_grab_in_frame(
        clusters.kinematics.pos,
        clusters.kinematics.size,
        contact,
        frame.down(),
    );

    let pre_wall_fresh = axis_state.pre_wall_vel_age <= LEDGE_REGRAB_COOLDOWN;
    let momentum_at_grab = if pre_wall_fresh
        && axis_state.pre_wall_vel.length_squared() > clusters.kinematics.vel.length_squared()
    {
        axis_state.pre_wall_vel
    } else {
        clusters.kinematics.vel
    };

    clusters.kinematics.pos = contact.anchor;
    clusters.kinematics.vel = Vec2::ZERO;
    clusters.kinematics.facing = into_platform_axis(contact);
    axis_state.wall_clinging = true;
    axis_state.wall_climbing = false;
    clusters.wall.on_wall = true;
    clusters.wall.wall_normal_x = contact.wall_normal_x;
    axis_state.ledge_grab = Some(LedgeGrabState {
        momentum_at_grab,
        ..LedgeGrabState::hanging_with_quality(contact, grab_quality)
    });
    // Smash-Bros style ledge intangibility: a brief invuln window on grab.
    // It is the LEDGE's own timer, joined to the damage rule at
    // `BodyMotionFacts::evading` — which is the pipeline the old comment here
    // meant, and the right place to join it. Sharing the dodge roll's field
    // instead made a body hanging on an edge read as one mid-evade.
    //
    // and it is EARNED, not flat. The window is bought with the time this
    // body spent off a ledge, so a fighter that was knocked away and recovered
    // gets all of it and one that drops and instantly re-catches gets the floor.
    // A flat grant made the edge a free reset you could hold forever.
    let earned = super::ledge_grab_invuln_earned(axis_state.time_off_ledge);
    if axis_state.ledge_invuln_timer < earned {
        axis_state.ledge_invuln_timer = earned;
    }
    // ⭐⭐ AND IT DOES NOT START YET. A sliver of exposure at the catch is what
    // makes contesting a ledge possible at all — without it the edge is an
    // unconditional safe point and the recovery is decided entirely off-stage.
    // See `LEDGE_GRAB_VULNERABLE_TIME`; the earned window above is HELD, not
    // spent, while this runs.
    axis_state.ledge_vulnerable_timer = super::LEDGE_GRAB_VULNERABLE_TIME;
    axis_state.time_off_ledge = 0.0;
    // CATCHING THE LEDGE IS THE RESET, and it happens HERE rather than at each
    // way out of the hang. The genre's rule is that the edge gives you your
    // options back the moment you take it — jumps, dash charges, the air dodge
    // — which is what makes a recovery that reaches the ledge a recovery at
    // all. Two of the five exits refreshed (ledge jump, ledge release) and
    // three did not, so a fighter that caught the lip with an empty budget and
    // DROPPED off it to reposition fell with nothing, having just touched the
    // thing that was supposed to have restored it.
    //
    // ⇒ one site, at the latch. The exits that used to refresh no longer need
    // to: nothing between the grab and the exit can spend any of it.
    crate::body_clusters::refresh_movement_resources_clusters(
        clusters.abilities,
        clusters.dash,
        clusters.jump,
        clusters.dodge,
        tuning.locomotion.air_jumps,
        // Catching the lip IS the re-seating — the whole reason this call moved
        // to the latch. It answers for the recovery outright.
        crate::body_clusters::RecoveryRefresh::Answered,
    );
    events.op_clusters(clusters.combo_trace, MovementOp::LedgeGrab);
    true
}
