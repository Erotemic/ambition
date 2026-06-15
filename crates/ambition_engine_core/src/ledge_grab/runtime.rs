//! Ledge-grab runtime: probing for a grabbable ledge, ticking the active
//! climb/roll/getup clusters, grab classification, and the launch-boost math.
//!
//! Split out of the former 934-line `ledge_grab/mod.rs` (2026-06-15). The pure
//! position/curve helpers stay in the parent and are reached via `use super::*`.

use super::*;

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
    // Window where the ledge top must sit. Keep this intentionally
    // forgiving: a near-miss slightly below the lip should still grab
    // instead of requiring the player's head to be in a tight chin band.
    let head_y = player_pos.y - half.y;
    let reach_min_y = head_y - LEDGE_REACH_UP;
    let reach_max_y = head_y + LEDGE_REACH_DOWN;
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
        if top < reach_min_y || top > reach_max_y {
            continue;
        }
        // Pick the wall edge of this block matching the cling side.
        let block_wall_x = if wall_normal_x > 0.0 {
            block.aabb.right()
        } else {
            block.aabb.left()
        };
        // The player's reaching side must be close to that face. This
        // is deliberately a small magnet range, not exact contact: by
        // the time the ledge probe runs, horizontal collision or one
        // frame of falling can leave the player a few pixels off the
        // wall even though the input/read is clearly a ledge grab.
        if (block_wall_x - cling_x).abs() > LEDGE_HORIZONTAL_REACH {
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
        // The hanging body must also have open space on the outside of the
        // ledge. Without this check, the climb target can be clear while the
        // initial hang snap overlaps a neighboring wall in front of the ledge;
        // from there the getup interpolation can tunnel the player through that
        // wall. Exclude the grabbed block itself because the anchor intentionally
        // overlaps it by ~1 px to keep the visual cling tight.
        let hang_center = Vec2::new(
            block_wall_x + wall_normal_x * (half.x - 1.0),
            top + half.y - 4.0,
        );
        let hang_aabb = Aabb::new(hang_center, half - Vec2::new(2.0, 2.0));
        let hang_blocked = world.body_overlaps_any(hang_aabb, |b| {
            ledge_clearance_blocker_kind(b.kind) && !std::ptr::eq(b, block)
        });
        if hang_blocked {
            continue;
        }
        // Anchor: player center hugs the wall on the same side the
        // player was clinging from, with the chest at the ledge top.
        // wall_normal_x = -1 (wall on player's right) → anchor.x is
        // just left of the wall's left face.
        let anchor = hang_center;
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
pub fn tick_active_ledge_grab_clusters(
    clusters: &mut crate::player_clusters::PlayerClustersMut<'_>,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
    events: &mut crate::movement::FrameEvents,
) -> bool {
    let Some(mut state) = clusters.ledge.grab else {
        return false;
    };
    if !clusters.abilities.abilities.ledge_grab {
        clusters.ledge.grab = None;
        return false;
    }

    state.elapsed += dt;
    clusters.kinematics.facing = into_platform_axis(state.contact);

    if state.climbing {
        state.climb_elapsed += dt;
        let duration_scale = ledge_getup_duration_scale(state, &tuning);
        let duration = state.getup_duration() * duration_scale;
        let progress = (state.climb_elapsed / duration).clamp(0.0, 1.0);
        clusters.kinematics.pos = getup_position(state, progress);
        clusters.kinematics.vel = Vec2::ZERO;
        clusters.ground.on_ground = false;
        clusters.wall.wall_clinging = false;
        clusters.wall.wall_climbing = false;
        clusters.wall.on_wall = false;

        if progress >= 1.0 {
            clusters.kinematics.pos = getup_end_position(state);
            // Carry HORIZONTAL momentum into exit; drop the Y so the
            // player doesn't relaunch off the platform they just stood
            // on. (Ledge-jump path keeps Y because that's a hop.)
            let boost = ledge_boost_for_state(state, &tuning);
            clusters.kinematics.vel = Vec2::new(boost.x, 0.0);
            clusters.ground.on_ground = true;
            clusters.wall.wall_clinging = false;
            clusters.wall.wall_climbing = false;
            clusters.wall.on_wall = false;
            clusters.ledge.grab = None;
            events.op_clusters(clusters.combo_trace, MovementOp::LedgeClimbFinish);
        } else {
            clusters.ledge.grab = Some(state);
        }
        return true;
    }

    // Gravity-relative: "up" = away from gravity (climb up the ledge), "down" =
    // toward gravity (drop). Flips under inverted gravity.
    let descend = crate::movement::gravity_descend(input.axis_y, tuning.gravity_dir);
    let input_up = descend < -0.4;
    let input_down = descend > 0.4;
    let input_into_platform = input.axis_x * into_platform_axis(state.contact) > 0.4;
    let input_away_from_platform = input.axis_x * away_from_platform_axis(state.contact) > 0.4;
    let climb_unlocked = state.elapsed >= LEDGE_MIN_CLIMB_DELAY;

    let want_roll = climb_unlocked && input.shield_held && clusters.abilities.abilities.shield;
    let want_ledge_release =
        climb_unlocked && !want_roll && input.jump_pressed && input_away_from_platform;
    let want_ledge_jump = climb_unlocked && !want_roll && !want_ledge_release && input.jump_pressed;
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
        clusters.wall.wall_clinging = false;
        clusters.wall.wall_climbing = false;
        clusters.wall.on_wall = false;
        clusters.dodge.roll_timer = LEDGE_ROLL_TIME + 0.10;
        clusters.ledge.grab = Some(state);
        events.op_clusters(clusters.combo_trace, MovementOp::LedgeRoll);
        return true;
    }
    if want_ledge_release {
        let away_x = away_from_platform_axis(state.contact);
        clusters.wall.wall_clinging = false;
        clusters.wall.wall_climbing = false;
        clusters.wall.on_wall = false;
        clusters.ground.on_ground = false;
        clusters.ledge.grab = None;
        clusters.ledge.release_cooldown = LEDGE_REGRAB_COOLDOWN;
        clusters.kinematics.vel = Vec2::new(away_x * tuning.wall_jump_x, -tuning.jump_speed);
        crate::player_clusters::refresh_movement_resources_clusters(
            clusters.abilities,
            &mut *clusters.dash,
            &mut *clusters.jump,
            tuning,
        );
        events.op_clusters(clusters.combo_trace, MovementOp::LedgeJump);
        return true;
    }
    if want_ledge_jump {
        let into_x = into_platform_axis(state.contact);
        clusters.wall.wall_clinging = false;
        clusters.wall.wall_climbing = false;
        clusters.wall.on_wall = false;
        clusters.ground.on_ground = false;
        clusters.ledge.grab = None;
        clusters.ledge.release_cooldown = LEDGE_REGRAB_COOLDOWN;
        let mut launch = Vec2::new(into_x * tuning.jump_speed * 0.35, -tuning.jump_speed);
        launch += ledge_boost_for_state(state, &tuning);
        clusters.kinematics.vel = launch;
        crate::player_clusters::refresh_movement_resources_clusters(
            clusters.abilities,
            &mut *clusters.dash,
            &mut *clusters.jump,
            tuning,
        );
        events.op_clusters(clusters.combo_trace, MovementOp::LedgeJump);
        return true;
    }
    if want_drop && !want_climb && !want_getup_attack {
        clusters.wall.wall_clinging = false;
        clusters.wall.wall_climbing = false;
        clusters.wall.on_wall = false;
        clusters.ledge.grab = None;
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
        clusters.wall.wall_clinging = false;
        clusters.wall.wall_climbing = false;
        clusters.wall.on_wall = false;
        clusters.dodge.roll_timer = LEDGE_GETUP_ATTACK_TIME;
        clusters.ledge.grab = Some(state);
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
        clusters.wall.wall_clinging = false;
        clusters.wall.wall_climbing = false;
        clusters.wall.on_wall = false;
        clusters.ledge.grab = Some(state);
        events.op_clusters(clusters.combo_trace, MovementOp::LedgeClimbStart);
        return true;
    }

    // Default: stay in the hang. Re-pin pos to the anchor and zero vel
    // so gravity / wall-slide doesn't drift the player off the lip.
    clusters.kinematics.pos = state.contact.anchor;
    clusters.kinematics.vel = Vec2::ZERO;
    clusters.wall.wall_clinging = true;
    clusters.wall.wall_climbing = false;
    clusters.wall.on_wall = true;
    clusters.ledge.grab = Some(state);
    true
}

/// Classify whether the grab geometry also satisfies the original,
/// tighter probe window. Widened ledge grabs are still valid catches,
/// but they intentionally do not earn ledge-momentum boost rewards.
pub fn classify_ledge_grab(
    player_pos: Vec2,
    player_size: Vec2,
    contact: LedgeContact,
) -> LedgeGrabQuality {
    let half = player_size * 0.5;
    let head_y = player_pos.y - half.y;
    let precise_min_y = head_y - LEDGE_PRECISE_REACH_UP;
    let precise_max_y = head_y + LEDGE_PRECISE_REACH_DOWN;

    // Invert the anchor formula from `probe_ledge_grab`:
    // anchor.x = block_wall_x + wall_normal_x * (half.x - 1.0)
    // anchor.y = top + half.y - 4.0
    let block_wall_x = contact.anchor.x - contact.wall_normal_x * (half.x - 1.0);
    let top = contact.anchor.y - half.y + 4.0;
    let cling_x = if contact.wall_normal_x > 0.0 {
        player_pos.x - half.x
    } else {
        player_pos.x + half.x
    };

    if top >= precise_min_y
        && top <= precise_max_y
        && (block_wall_x - cling_x).abs() <= LEDGE_PRECISE_HORIZONTAL_REACH
    {
        LedgeGrabQuality::Precise
    } else {
        LedgeGrabQuality::Forgiving
    }
}

/// Convenience predicate for call sites/tests that only care about the
/// precision reward gate.
pub fn is_precise_ledge_grab(player_pos: Vec2, player_size: Vec2, contact: LedgeContact) -> bool {
    classify_ledge_grab(player_pos, player_size, contact).is_precise()
}

/// Pick a wall normal to probe for a ledge: the active wall-cling
/// normal first, else a horizontal axis-press while airborne.
fn requested_wall_normal_clusters(
    wall: &crate::player_clusters::PlayerWallState,
    ground: &crate::player_clusters::PlayerGroundState,
    input: InputState,
) -> Option<f32> {
    if wall.wall_clinging && wall.wall_normal_x.abs() >= 0.5 {
        return Some(wall.wall_normal_x);
    }
    if !ground.on_ground && input.axis_x.abs() > LEDGE_GRAB_INTENT_DEADZONE {
        return Some(-input.axis_x.signum());
    }
    None
}

/// Compute the momentum-carry boost vector for a getup option.
///
/// Returns a velocity to ADD to the launch / post-transition velocity
/// of an eligible getup (climb / roll / attack / vertical ledge-jump).
/// Returns zero when:
/// - The mechanic is disabled via `tuning.ledge_momentum.window == 0.0`.
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
pub fn ledge_boost(
    momentum_at_grab: Vec2,
    contact: LedgeContact,
    elapsed_at_initiation: f32,
    tuning: &MovementTuning,
) -> Vec2 {
    let cfg = tuning.ledge_momentum;
    if cfg.window <= 0.0 || elapsed_at_initiation > cfg.window {
        return Vec2::ZERO;
    }
    // Linear decay across the window — full boost at t=0, zero at
    // t=window. Easier to reason about while tuning than smoothstep.
    let weight = 1.0 - (elapsed_at_initiation / cfg.window).clamp(0.0, 1.0);
    let m = momentum_at_grab;
    // Only count horizontal momentum that points INTO the platform.
    // Reverse momentum at grab time meant the player wasn't carrying
    // forward speed — they were sliding off the lip — no reward.
    let into = into_platform_axis(contact);
    let forward_into = m.x * into; // > 0 if pointing toward platform
    let carried_x = if forward_into > 0.0 {
        m.x * cfg.x_gain * weight
    } else {
        0.0
    };
    // Sim convention: +Y is down. Upward momentum is negative; only
    // count that. Downward momentum was "falling," no boost.
    let carried_y = if m.y < 0.0 {
        m.y * cfg.y_gain * weight
    } else {
        0.0
    };
    Vec2::new(
        carried_x.clamp(-cfg.x_cap, cfg.x_cap),
        carried_y.clamp(-cfg.y_cap, cfg.y_cap),
    )
}

/// Convenience: compute the boost from a [`LedgeGrabState`]. For
/// transitions that have already started ticking `climb_elapsed`,
/// subtracts that from `elapsed` to recover the grab-to-action time.
pub fn ledge_boost_for_state(state: LedgeGrabState, tuning: &MovementTuning) -> Vec2 {
    if !state.grab_quality.is_precise() {
        return Vec2::ZERO;
    }
    let elapsed_at_initiation = (state.elapsed - state.climb_elapsed).max(0.0);
    ledge_boost(
        state.momentum_at_grab,
        state.contact,
        elapsed_at_initiation,
        tuning,
    )
}

/// The boost weight (0..1) at the time a getup was initiated. Used
/// to scale both the launch velocity AND the transition duration —
/// so a high-momentum getup runs faster AND exits faster, rather
/// than just teleporting fast at the end of a frozen animation.
pub fn ledge_boost_weight_for_state(state: LedgeGrabState, tuning: &MovementTuning) -> f32 {
    if !state.grab_quality.is_precise() {
        return 0.0;
    }
    let cfg = tuning.ledge_momentum;
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
pub fn ledge_getup_duration_scale(state: LedgeGrabState, tuning: &MovementTuning) -> f32 {
    let weight = ledge_boost_weight_for_state(state, tuning);
    1.0 / (1.0 + weight * tuning.ledge_momentum.getup_speedup_gain)
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
/// - **Intentional snap**: the player is wall-clinging or actively
///   moving toward a wall (input.axis_x non-zero while airborne).
///   `requested_wall_normal` returns the side to probe.
/// - **Falling-into-ledge snap**: the player is falling fast and a
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
pub fn try_start_ledge_grab_clusters(
    world: &World,
    clusters: &mut crate::player_clusters::PlayerClustersMut<'_>,
    input: InputState,
    events: &mut crate::movement::FrameEvents,
) -> bool {
    if !clusters.abilities.abilities.ledge_grab
        || clusters.ledge.grab.is_some()
        || clusters.ground.on_ground
    {
        return false;
    }
    if clusters.ledge.release_cooldown > 0.0 {
        return false;
    }

    let mut contact: Option<LedgeContact> = None;
    if let Some(wall_normal) = requested_wall_normal_clusters(clusters.wall, clusters.ground, input)
    {
        contact = probe_ledge_grab(
            clusters.kinematics.pos,
            clusters.kinematics.size,
            wall_normal,
            world,
        );
    }
    if contact.is_none() && clusters.kinematics.vel.y > FALL_SNAP_MIN_VY {
        // Smash-style auto-snap during a falling recovery: try BOTH
        // sides and snap to whichever has a grabbable lip in the chin
        // band.
        for trial_normal in [-1.0_f32, 1.0_f32] {
            if let Some(found) = probe_ledge_grab(
                clusters.kinematics.pos,
                clusters.kinematics.size,
                trial_normal,
                world,
            ) {
                contact = Some(found);
                break;
            }
        }
    }
    let Some(contact) = contact else {
        return false;
    };

    let grab_quality =
        classify_ledge_grab(clusters.kinematics.pos, clusters.kinematics.size, contact);

    let pre_wall_fresh = clusters.wall.pre_wall_vel_age <= LEDGE_REGRAB_COOLDOWN;
    let momentum_at_grab = if pre_wall_fresh
        && clusters.wall.pre_wall_vel.length_squared() > clusters.kinematics.vel.length_squared()
    {
        clusters.wall.pre_wall_vel
    } else {
        clusters.kinematics.vel
    };

    clusters.kinematics.pos = contact.anchor;
    clusters.kinematics.vel = Vec2::ZERO;
    clusters.kinematics.facing = into_platform_axis(contact);
    clusters.wall.wall_clinging = true;
    clusters.wall.wall_climbing = false;
    clusters.wall.on_wall = true;
    clusters.wall.wall_normal_x = contact.wall_normal_x;
    clusters.ledge.grab = Some(LedgeGrabState {
        momentum_at_grab,
        ..LedgeGrabState::hanging_with_quality(contact, grab_quality)
    });
    // Smash-Bros style ledge intangibility: a brief invuln window on
    // grab. Reuses `dodge_roll_timer` because that field already gates
    // damage — same pipeline, single source of truth.
    if clusters.dodge.roll_timer < LEDGE_GRAB_INVULN_TIME {
        clusters.dodge.roll_timer = LEDGE_GRAB_INVULN_TIME;
    }
    events.op_clusters(clusters.combo_trace, MovementOp::LedgeGrab);
    true
}

/// Scratch-based wrapper around [`tick_active_ledge_grab_clusters`]. The
/// engine ledge_grab tests use this so they can keep the
/// "build a Player → assert against it" pattern.
pub fn tick_active_ledge_grab_scratch(
    scratch: &mut crate::player_clusters::PlayerClusterScratch,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
    events: &mut crate::movement::FrameEvents,
) -> bool {
    let mut clusters = scratch.as_mut();
    tick_active_ledge_grab_clusters(&mut clusters, input, dt, tuning, events)
}

/// Scratch-based wrapper around [`try_start_ledge_grab_clusters`].
pub fn try_start_ledge_grab_scratch(
    world: &World,
    scratch: &mut crate::player_clusters::PlayerClusterScratch,
    input: InputState,
    events: &mut crate::movement::FrameEvents,
) -> bool {
    let mut clusters = scratch.as_mut();
    try_start_ledge_grab_clusters(world, &mut clusters, input, events)
}
