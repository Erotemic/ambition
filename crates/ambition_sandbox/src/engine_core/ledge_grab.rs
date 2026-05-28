//! Ledge grab probe, state, and movement-pipeline tick helpers.
//!
//! The probe answers: "is there a ledge corner I can snap onto, and
//! where is the hang / pull-up path?" The state machine is engine-owned so
//! ledge grab participates in the same movement tick as gravity, collision,
//! water, and wall state instead of running as a post-update sandbox mutator.

use crate::engine_core::geometry::{Aabb, AabbExt};
use crate::engine_core::movement::{InputState, MovementOp, MovementTuning};
use crate::engine_core::world::{BlockKind, World};
use crate::engine_core::Vec2;

/// Duration of the standard ledge pull-up transition.
pub const LEDGE_CLIMB_TIME: f32 = 0.24;

/// Duration of a Smash-Bros-style ledge ROLL: shorter than the climb
/// because the player commits horizontally and lands further inboard.
/// The whole window grants invulnerability via the existing dodge-roll
/// timer.
pub const LEDGE_ROLL_TIME: f32 = 0.30;

/// Duration of a getup-attack (Smash-style "ledge attack"). The
/// player lifts to the platform on the same curve as the climb but
/// swings during the lift; the active hitbox fires at the start and
/// the player has invuln frames via `Player::dodge_roll_timer` for
/// the duration. Tuned slightly longer than a plain climb to give the
/// swing time to read.
pub const LEDGE_GETUP_ATTACK_TIME: f32 = 0.30;

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

/// Cooldown blocking a fresh ledge grab right after the player
/// voluntarily released a ledge (drop / ledge-jump / ledge-release).
/// At typical gravity (~1500 px/s²) a player accelerating from rest
/// clears the chin-band (≈30 px tall) in about 200 ms; pad to 250 ms
/// so the same lip can't re-snap on the very next fall sample, and
/// also so the player gets a clear "I'm dropping" beat before any
/// auto-snap can re-engage. Tune up for stickier feel, down for
/// snappier recovery.
pub const LEDGE_REGRAB_COOLDOWN: f32 = 0.25;

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
    /// Smash-Bros style ledge getup attack: the player swings onto the
    /// platform, attacking on the way up. Movement follows the same
    /// curve as `Climb`; the slash hitbox is fired at the start and
    /// the player is invulnerable for the duration. Reuses the
    /// regular player attack animation for now — TODO: author a
    /// dedicated getup-attack sprite/animation so the swing reads
    /// distinctly from a normal slash.
    Attack,
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
    /// Velocity the player carried into the ledge at the moment of
    /// grab. Used to grant a momentum-carry boost to early getup
    /// options (climb / roll / attack / vertical jump) per the
    /// `MovementTuning::ledge_momentum` parameters. Capped + decayed
    /// by [`ledge_boost`]; pure data, no behavior change unless that
    /// helper actually consumes it.
    pub momentum_at_grab: Vec2,
}

impl LedgeGrabState {
    pub fn hanging(contact: LedgeContact) -> Self {
        Self {
            contact,
            elapsed: 0.0,
            climbing: false,
            getup_kind: LedgeGetupKind::Climb,
            climb_elapsed: 0.0,
            momentum_at_grab: Vec2::ZERO,
        }
    }

    /// Convenience constructor for tests: hanging-on-ledge state with
    /// a specific incoming-momentum vector, so boost-eligible getup
    /// paths can be exercised without spelling out every field.
    #[cfg(test)]
    pub fn hanging_with_momentum(contact: LedgeContact, momentum: Vec2) -> Self {
        Self {
            momentum_at_grab: momentum,
            ..Self::hanging(contact)
        }
    }

    /// Duration of the active getup at this state's `getup_kind`.
    /// Returns 0 if not currently in a getup.
    pub fn getup_duration(self) -> f32 {
        match self.getup_kind {
            LedgeGetupKind::Climb => LEDGE_CLIMB_TIME,
            LedgeGetupKind::Roll => LEDGE_ROLL_TIME,
            LedgeGetupKind::Attack => LEDGE_GETUP_ATTACK_TIME,
        }
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn climb_position(contact: LedgeContact, progress: f32) -> Vec2 {
    // Smash-style curved climb: the player rises up the wall FIRST
    // and arcs over onto the platform second, instead of moving in
    // a straight diagonal from anchor → climb_target. Implemented
    // as a quadratic Bezier with control point at (anchor.x,
    // climb_target.y) — same x as the anchor (so the early curve
    // is purely vertical along the wall), same y as the target (so
    // the late curve is purely horizontal across the platform top).
    let t = smoothstep(progress);
    let a = contact.anchor;
    let b = contact.climb_target;
    let control = Vec2::new(a.x, b.y);
    let one_t = 1.0 - t;
    a * (one_t * one_t) + control * (2.0 * one_t * t) + b * (t * t)
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
        // Attack uses the same arc as Climb — only the timing,
        // invuln, and triggered slash differ.
        LedgeGetupKind::Attack => climb_position(state.contact, progress),
    }
}

fn getup_end_position(state: LedgeGrabState) -> Vec2 {
    match state.getup_kind {
        LedgeGetupKind::Climb => state.contact.climb_target,
        LedgeGetupKind::Roll => roll_target(state.contact),
        LedgeGetupKind::Attack => state.contact.climb_target,
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

/// Cluster-native variant of [`tick_active_ledge_grab`]. Drops the
/// internal `to_player`/`write_from_player` scratchpad in
/// `update_player_simulation_with_clusters` for the active-ledge tick.
///
/// Behavior parity is preserved field-for-field; the only divergence
/// is that `events.op_clusters` is used in place of
/// `events.op(player, …)` (cluster combo trace instead of
/// `Player::record`).
pub fn tick_active_ledge_grab_clusters(
    clusters: &mut crate::engine_core::player_clusters::PlayerClustersMut<'_>,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
    events: &mut crate::engine_core::movement::FrameEvents,
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

    let input_up = input.axis_y < -0.4;
    let input_down = input.axis_y > 0.4;
    let input_into_platform = input.axis_x * into_platform_axis(state.contact) > 0.4;
    let input_away_from_platform = input.axis_x * away_from_platform_axis(state.contact) > 0.4;
    let climb_unlocked = state.elapsed >= LEDGE_MIN_CLIMB_DELAY;

    let want_roll = climb_unlocked && input.shield_held && clusters.abilities.abilities.shield;
    let want_ledge_release =
        climb_unlocked && !want_roll && input.jump_pressed && input_away_from_platform;
    let want_ledge_jump =
        climb_unlocked && !want_roll && !want_ledge_release && input.jump_pressed;
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
        crate::engine_core::player_clusters::refresh_movement_resources_clusters(
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
        let mut launch =
            Vec2::new(into_x * tuning.jump_speed * 0.35, -tuning.jump_speed);
        launch += ledge_boost_for_state(state, &tuning);
        clusters.kinematics.vel = launch;
        crate::engine_core::player_clusters::refresh_movement_resources_clusters(
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


/// Cluster-native variant of [`requested_wall_normal`]. Reads
/// `wall_clinging` / `wall_normal_x` / `on_ground` from cluster
/// components so callers don't need to materialize an `ae::Player`.
fn requested_wall_normal_clusters(
    wall: &crate::engine_core::player_clusters::PlayerWallState,
    ground: &crate::engine_core::player_clusters::PlayerGroundState,
    input: InputState,
) -> Option<f32> {
    if wall.wall_clinging && wall.wall_normal_x.abs() >= 0.5 {
        return Some(wall.wall_normal_x);
    }
    if !ground.on_ground && input.axis_x.abs() > 0.4 {
        return Some(-input.axis_x.signum());
    }
    None
}

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

/// trigger. Set just above terminal "drifting" speed so a player
/// who is loitering near a ledge with no stick input doesn't get
/// snagged on it by accident.
const FALL_SNAP_MIN_VY: f32 = 80.0;

/// Cluster-native variant of [`try_start_ledge_grab`]. Reads /
/// writes the same player state via the cluster components on the
/// player entity so the engine's `update_player_simulation_with_clusters`
/// can drop one of its internal `to_player`/`write_from_player`
/// round-trips.
pub fn try_start_ledge_grab_clusters(
    world: &World,
    clusters: &mut crate::engine_core::player_clusters::PlayerClustersMut<'_>,
    input: InputState,
    events: &mut crate::engine_core::movement::FrameEvents,
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

    let pre_wall_fresh = clusters.wall.pre_wall_vel_age <= LEDGE_REGRAB_COOLDOWN;
    let momentum_at_grab = if pre_wall_fresh
        && clusters.wall.pre_wall_vel.length_squared()
            > clusters.kinematics.vel.length_squared()
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
        ..LedgeGrabState::hanging(contact)
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
    scratch: &mut crate::engine_core::player_clusters::PlayerClusterScratch,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
    events: &mut crate::engine_core::movement::FrameEvents,
) -> bool {
    let mut clusters = scratch.as_mut();
    tick_active_ledge_grab_clusters(&mut clusters, input, dt, tuning, events)
}

/// Scratch-based wrapper around [`try_start_ledge_grab_clusters`].
pub fn try_start_ledge_grab_scratch(
    world: &World,
    scratch: &mut crate::engine_core::player_clusters::PlayerClusterScratch,
    input: InputState,
    events: &mut crate::engine_core::movement::FrameEvents,
) -> bool {
    let mut clusters = scratch.as_mut();
    try_start_ledge_grab_clusters(world, &mut clusters, input, events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_core::world::Block;

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
            crate::engine_core::world::BlinkWallTier::Soft,
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

    use crate::engine_core::player_clusters::PlayerClusterScratch;

    fn make_hanging_player(contact: LedgeContact) -> PlayerClusterScratch {
        make_hanging_player_with_momentum(contact, Vec2::ZERO)
    }

    fn make_hanging_player_with_momentum(
        contact: LedgeContact,
        momentum: Vec2,
    ) -> PlayerClusterScratch {
        let mut scratch = scratch_at(Vec2::ZERO);
        scratch.abilities.abilities.ledge_grab = true;
        scratch.abilities.abilities.shield = true;
        scratch.ledge.grab = Some(LedgeGrabState {
            contact,
            elapsed: LEDGE_MIN_CLIMB_DELAY + 0.01,
            climbing: false,
            getup_kind: LedgeGetupKind::Climb,
            climb_elapsed: 0.0,
            momentum_at_grab: momentum,
        });
        scratch.wall.wall_clinging = true;
        scratch.wall.on_wall = true;
        scratch
    }

    fn scratch_at(pos: Vec2) -> PlayerClusterScratch {
        PlayerClusterScratch::new_with_abilities(pos, crate::engine_core::AbilitySet::sandbox_all())
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
        let mut scratch = make_hanging_player(contact);
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let tuning = crate::engine_core::movement::MovementTuning::default();
        let input = InputState {
            jump_pressed: true,
            axis_x: -1.0, // pressing away from the platform (away = wall_normal direction = -1)
            ..InputState::default()
        };
        let consumed = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
        assert!(consumed, "tick should consume the frame");
        assert!(scratch.ledge.grab.is_none(), "ledge should be released");
        // Player should move left (away from the right-side wall).
        assert!(
            scratch.kinematics.vel.x < -100.0,
            "should have leftward velocity, got {}",
            scratch.kinematics.vel.x
        );
        assert!(
            scratch.kinematics.vel.y < -100.0,
            "should have upward velocity, got {}",
            scratch.kinematics.vel.y
        );
        assert!(!scratch.wall.on_wall, "should not be on wall");
    }

    #[test]
    fn jump_toward_platform_now_hops_up_not_climbs() {
        // Smash-style split: pressing Jump from a ledge is the
        // "ledge jump" option (vertical hop with control). It used
        // to trigger a climb instead. The climb is now reserved
        // for Up / Into / Interact.
        let contact = LedgeContact {
            wall_normal_x: -1.0,
            anchor: Vec2::new(86.0, 110.0),
            climb_target: Vec2::new(115.0, 77.0),
        };
        let mut scratch = make_hanging_player(contact);
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let tuning = crate::engine_core::movement::MovementTuning::default();
        let input = InputState {
            jump_pressed: true,
            axis_x: 1.0, // pressing into the platform (into = -wall_normal = +1)
            ..InputState::default()
        };
        let consumed = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
        assert!(consumed);
        assert!(
            scratch.ledge.grab.is_none(),
            "ledge should be released by the hop"
        );
        assert!(
            scratch.kinematics.vel.y < -100.0,
            "ledge jump should fling upward, got vy={}",
            scratch.kinematics.vel.y
        );
        // Inboard drift: for a -1 wall_normal, into_x = +1, so vx > 0.
        assert!(
            scratch.kinematics.vel.x > 0.0,
            "ledge jump should drift inboard, got vx={}",
            scratch.kinematics.vel.x
        );
        assert!(!scratch.wall.on_wall);
    }

    /// Pure jump from the ledge with NO horizontal input also hops
    /// UP. This was the case the old code mapped to "climb" because
    /// jump was a confirm cue; now the player gets a vertical hop
    /// they can air-control.
    #[test]
    fn jump_with_no_horizontal_input_hops_up() {
        let contact = LedgeContact {
            wall_normal_x: -1.0,
            anchor: Vec2::new(86.0, 110.0),
            climb_target: Vec2::new(115.0, 77.0),
        };
        let mut scratch = make_hanging_player(contact);
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let tuning = crate::engine_core::movement::MovementTuning::default();
        let input = InputState {
            jump_pressed: true,
            ..InputState::default()
        };
        let consumed = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
        assert!(consumed);
        assert!(scratch.ledge.grab.is_none());
        assert!(
            scratch.kinematics.vel.y < -100.0,
            "pure jump should still go up, got vy={}",
            scratch.kinematics.vel.y
        );
    }

    /// `Up` (without jump) is still the slow climb path. The split
    /// must NOT have broken the regular pull-up.
    #[test]
    fn up_alone_still_starts_a_climb() {
        let contact = LedgeContact {
            wall_normal_x: -1.0,
            anchor: Vec2::new(86.0, 110.0),
            climb_target: Vec2::new(115.0, 77.0),
        };
        let mut scratch = make_hanging_player(contact);
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let tuning = crate::engine_core::movement::MovementTuning::default();
        let input = InputState {
            axis_y: -1.0, // up
            ..InputState::default()
        };
        let _ = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
        let state = scratch
            .ledge.grab
            .expect("climb should leave transitioning state");
        assert!(state.climbing, "Up should start a climb");
        assert_eq!(state.getup_kind, LedgeGetupKind::Climb);
    }

    /// The climb path is a quadratic Bezier whose control point sits
    /// at `(anchor.x, climb_target.y)` — so the player goes UP the
    /// wall first and ACROSS onto the platform second. At t=0.5 the
    /// player's position should be much closer to the bend (above
    /// the anchor on the wall) than to the midpoint of a straight
    /// line between anchor and climb_target. This is the curved-feel
    /// Jon asked for; the straight-diagonal was the old behavior.
    #[test]
    fn climb_path_curves_up_before_going_over() {
        let contact = LedgeContact {
            wall_normal_x: -1.0,
            anchor: Vec2::new(86.0, 110.0),
            climb_target: Vec2::new(115.0, 77.0),
        };
        let mid_curved = climb_position(contact, 0.5);
        let mid_straight = (contact.anchor + contact.climb_target) * 0.5;
        // The curved midpoint should be closer to `(anchor.x, target.y)`
        // — the control point — than the straight-line midpoint is.
        let control = Vec2::new(contact.anchor.x, contact.climb_target.y);
        let curved_to_ctrl = (mid_curved - control).length();
        let straight_to_ctrl = (mid_straight - control).length();
        assert!(
            curved_to_ctrl < straight_to_ctrl,
            "curved midpoint should bias toward the bend; got {:.2} vs straight {:.2}",
            curved_to_ctrl,
            straight_to_ctrl,
        );
    }

    /// Falling fast past a ledge should auto-snap to it, even with
    /// no stick input — the Smash recovery snap. Without this you
    /// have to hold a stick INTO the wall to grab; in practice
    /// players want a near-miss snap.
    #[test]
    fn falling_player_auto_snaps_to_nearby_ledge() {
        let world = world_with(vec![Block::solid(
            "ledge",
            Vec2::new(100.0, 100.0),
            Vec2::new(200.0, 200.0),
        )]);
        let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
        scratch.abilities.abilities.ledge_grab = true;
        scratch.kinematics.vel = Vec2::new(0.0, 150.0); // falling fast, no horizontal input
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let latched = try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
        assert!(latched, "fast-falling near a ledge should auto-snap");
        assert!(scratch.ledge.grab.is_some());
    }

    /// A loitering player (slow descent, no stick input) should NOT
    /// auto-snap — only an active recovery does. Keeps the snap
    /// from feeling like sticky-wall.
    #[test]
    fn drifting_player_does_not_auto_snap() {
        let world = world_with(vec![Block::solid(
            "ledge",
            Vec2::new(100.0, 100.0),
            Vec2::new(200.0, 200.0),
        )]);
        let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
        scratch.abilities.abilities.ledge_grab = true;
        scratch.kinematics.vel = Vec2::new(0.0, 20.0); // gentle drift, well below FALL_SNAP_MIN_VY
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let latched = try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
        assert!(!latched, "slow drift must not auto-snap");
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
        let mut scratch = make_hanging_player(contact);
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let tuning = crate::engine_core::movement::MovementTuning::default();
        let input = InputState {
            shield_held: true,
            ..InputState::default()
        };
        let consumed = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
        assert!(consumed);
        let state = scratch
            .ledge.grab
            .expect("roll should leave a transitioning state");
        assert!(state.climbing, "roll must enter the climbing state");
        assert_eq!(state.getup_kind, LedgeGetupKind::Roll);
        assert!(
            scratch.dodge.roll_timer > 0.0,
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
        let mut scratch = make_hanging_player(contact);
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let tuning = crate::engine_core::movement::MovementTuning::default();
        let input = InputState {
            shield_held: true,
            axis_y: -1.0, // up — would otherwise climb
            ..InputState::default()
        };
        let _ = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
        let state = scratch.ledge.grab.expect("active transition");
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
        let mut scratch = make_hanging_player(contact);
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let tuning = crate::engine_core::movement::MovementTuning::default();
        // Start the roll.
        let _ = tick_active_ledge_grab_scratch(
            &mut scratch,
            InputState {
                shield_held: true,
                ..InputState::default()
            },
            0.001,
            tuning,
            &mut events,
        );
        // Run the full roll duration in one big tick to land.
        let _ = tick_active_ledge_grab_scratch(
            &mut scratch,
            InputState::default(),
            LEDGE_ROLL_TIME + 0.05,
            tuning,
            &mut events,
        );
        // After the roll finishes, the player should be at the roll
        // landing position which is past climb_target along the
        // into-platform axis. For a -1 wall normal that's +x.
        assert!(scratch.ledge.grab.is_none(), "roll should have finished");
        assert!(scratch.ground.on_ground, "roll lands the player on the platform");
        let expected = roll_target(contact);
        assert!(
            (scratch.kinematics.pos.x - expected.x).abs() < 0.5,
            "expected roll landing x ≈ {}, got {}",
            expected.x,
            scratch.kinematics.pos.x,
        );
        assert!(
            (expected.x - contact.climb_target.x).abs() >= LEDGE_ROLL_OVERSHOOT - 0.01,
            "roll target must overshoot the climb target by ~{}px",
            LEDGE_ROLL_OVERSHOOT,
        );
    }

    /// Smash-Bros regrab guard: after the player voluntarily drops
    /// from a ledge, `try_start_ledge_grab` must not re-snap the same
    /// lip while the cooldown is still ticking. Without this the
    /// auto-snap-on-fall path fires the moment gravity pushes
    /// `vel.y` past `FALL_SNAP_MIN_VY` — roughly two frames after
    /// release, while the player is still inside the chin-band.
    #[test]
    fn voluntary_drop_arms_a_regrab_cooldown() {
        let contact = LedgeContact {
            wall_normal_x: -1.0,
            anchor: Vec2::new(86.0, 110.0),
            climb_target: Vec2::new(115.0, 77.0),
        };
        let mut scratch = make_hanging_player(contact);
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let tuning = crate::engine_core::movement::MovementTuning::default();
        let input = InputState {
            axis_y: 1.0, // down
            ..InputState::default()
        };
        let consumed = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
        assert!(consumed);
        assert!(scratch.ledge.grab.is_none(), "drop should release the ledge");
        assert!(
            scratch.ledge.release_cooldown >= LEDGE_REGRAB_COOLDOWN - 0.001,
            "drop should arm the regrab cooldown, got {}",
            scratch.ledge.release_cooldown,
        );
    }

    /// While the regrab cooldown is live, `try_start_ledge_grab` must
    /// return false even if the player is falling fast past the same
    /// ledge. This is the actual fix for Jon's 2026-05-23 instant-
    /// regrab bug.
    #[test]
    fn regrab_cooldown_blocks_auto_snap_on_fall() {
        let world = world_with(vec![Block::solid(
            "ledge",
            Vec2::new(100.0, 100.0),
            Vec2::new(200.0, 200.0),
        )]);
        let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
        scratch.abilities.abilities.ledge_grab = true;
        // Player is falling fast — would normally trigger the
        // Smash-style auto-snap path.
        scratch.kinematics.vel = Vec2::new(0.0, 200.0);
        scratch.ledge.release_cooldown = LEDGE_REGRAB_COOLDOWN;
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let latched = try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
        assert!(
            !latched,
            "regrab cooldown should block the auto-snap-on-fall path"
        );
        assert!(scratch.ledge.grab.is_none());
    }

    /// After the cooldown expires the player can grab again normally.
    /// Guards against the cooldown being permanent / never decaying.
    #[test]
    fn regrab_cooldown_expires_and_allows_fresh_grab() {
        let world = world_with(vec![Block::solid(
            "ledge",
            Vec2::new(100.0, 100.0),
            Vec2::new(200.0, 200.0),
        )]);
        let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
        scratch.abilities.abilities.ledge_grab = true;
        scratch.kinematics.vel = Vec2::new(0.0, 200.0);
        // Cooldown has already expired (e.g. simulation_timers ticked
        // it down past zero between frames). Auto-snap should be free
        // to fire again.
        scratch.ledge.release_cooldown = 0.0;
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let latched = try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
        assert!(
            latched,
            "with cooldown cleared, the same fall trajectory should re-grab"
        );
    }

    /// Both the outward ledge-release-jump and the vertical ledge-jump
    /// also arm the cooldown — any voluntary release should prevent
    /// instant regrab.
    #[test]
    fn ledge_jump_options_also_arm_regrab_cooldown() {
        let contact = LedgeContact {
            wall_normal_x: -1.0,
            anchor: Vec2::new(86.0, 110.0),
            climb_target: Vec2::new(115.0, 77.0),
        };
        // Outward release (jump + away).
        {
            let mut scratch = make_hanging_player(contact);
            let mut events = crate::engine_core::movement::FrameEvents::default();
            let tuning = crate::engine_core::movement::MovementTuning::default();
            let input = InputState {
                jump_pressed: true,
                axis_x: -1.0, // away from a -1 wall_normal
                ..InputState::default()
            };
            let _ = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
            assert!(scratch.ledge.grab.is_none());
            assert!(
                scratch.ledge.release_cooldown >= LEDGE_REGRAB_COOLDOWN - 0.001,
                "ledge release should arm cooldown",
            );
        }
        // Vertical hop (jump alone).
        {
            let mut scratch = make_hanging_player(contact);
            let mut events = crate::engine_core::movement::FrameEvents::default();
            let tuning = crate::engine_core::movement::MovementTuning::default();
            let input = InputState {
                jump_pressed: true,
                ..InputState::default()
            };
            let _ = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
            assert!(scratch.ledge.grab.is_none());
            assert!(
                scratch.ledge.release_cooldown >= LEDGE_REGRAB_COOLDOWN - 0.001,
                "ledge jump should arm cooldown",
            );
        }
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
        let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
        scratch.abilities.abilities.ledge_grab = true;
        scratch.wall.wall_clinging = true;
        scratch.wall.wall_normal_x = -1.0;
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let latched = try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
        assert!(latched, "expected ledge grab to latch");
        assert!(
            scratch.dodge.roll_timer >= LEDGE_GRAB_INVULN_TIME - 0.001,
            "grab should arm at least {}s of invuln, got {}",
            LEDGE_GRAB_INVULN_TIME,
            scratch.dodge.roll_timer,
        );
    }

    // ---- Momentum-carry boost tests (Jon 2026-05-23 feature) ----
    //
    // Invariants under test:
    // 1. The boost is captured at grab time and rides the LedgeGrabState.
    // 2. Eligible getup options (climb, roll, attack, ledge_jump) get
    //    the boost folded into their exit velocity.
    // 3. The DROP and outward LEDGE-RELEASE options DO NOT get the
    //    boost — those are deliberate disengage actions.
    // 4. The boost decays linearly across the configured window and
    //    fires zero once the window has elapsed.
    // 5. Setting `LedgeMomentumTuning::OFF` (or window=0.0) fully
    //    disables the mechanic — restores the original "vel zeroed
    //    on grab" feel.
    // 6. Only INTO-platform horizontal and UPWARD vertical momentum
    //    is counted; reverse / downward components are discarded.

    fn into_platform_for(contact: LedgeContact) -> f32 {
        // Helper so tests don't have to memoize the sign convention.
        into_platform_axis(contact)
    }

    fn rightward_ledge_contact() -> LedgeContact {
        // Wall on player's RIGHT (wall_normal_x = -1). Platform is to
        // the right of the player, so into_platform_axis = +1.
        LedgeContact {
            wall_normal_x: -1.0,
            anchor: Vec2::new(86.0, 110.0),
            climb_target: Vec2::new(115.0, 77.0),
        }
    }

    #[test]
    fn try_start_ledge_grab_captures_incoming_velocity() {
        let world = world_with(vec![Block::solid(
            "ledge",
            Vec2::new(100.0, 100.0),
            Vec2::new(200.0, 200.0),
        )]);
        let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
        scratch.abilities.abilities.ledge_grab = true;
        // Player arriving at the ledge with rightward + upward momentum
        // (i.e. running up against the wall during a jump).
        scratch.kinematics.vel = Vec2::new(180.0, -240.0);
        scratch.wall.wall_clinging = true;
        scratch.wall.wall_normal_x = -1.0;
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let latched = try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
        assert!(latched);
        // After grab, vel is zeroed for the hang animation...
        assert_eq!(scratch.kinematics.vel, Vec2::ZERO);
        // ...but the state retains the pre-grab velocity for the
        // boost path.
        let state = scratch.ledge.grab.unwrap();
        assert!(
            (state.momentum_at_grab - Vec2::new(180.0, -240.0)).length() < 0.01,
            "momentum_at_grab should mirror pre-grab vel, got {:?}",
            state.momentum_at_grab,
        );
    }

    #[test]
    fn ledge_boost_decays_linearly_across_window() {
        let tuning = MovementTuning::default();
        let contact = rightward_ledge_contact();
        let momentum = Vec2::new(200.0 * into_platform_for(contact), 0.0);
        // t=0: full boost.
        let early = ledge_boost(momentum, contact, 0.0, &tuning);
        // t=window/2: roughly half.
        let mid = ledge_boost(
            momentum,
            contact,
            tuning.ledge_momentum.window * 0.5,
            &tuning,
        );
        // t=window: zero (or right at zero per the linear weight).
        let late = ledge_boost(momentum, contact, tuning.ledge_momentum.window, &tuning);
        // t>window: zero.
        let past = ledge_boost(
            momentum,
            contact,
            tuning.ledge_momentum.window * 2.0,
            &tuning,
        );
        assert!(
            early.x.abs() > mid.x.abs(),
            "early > mid: {} > {}",
            early.x,
            mid.x
        );
        assert!(
            mid.x.abs() > late.x.abs(),
            "mid > late: {} > {}",
            mid.x,
            late.x
        );
        assert!(late.x.abs() < 0.01, "late ≈ 0, got {}", late.x);
        assert_eq!(past, Vec2::ZERO);
    }

    #[test]
    fn ledge_boost_off_disables_mechanic() {
        let mut tuning = MovementTuning::default();
        tuning.ledge_momentum = crate::engine_core::movement::LedgeMomentumTuning::OFF;
        let contact = rightward_ledge_contact();
        let momentum = Vec2::new(300.0 * into_platform_for(contact), -300.0);
        let boost = ledge_boost(momentum, contact, 0.0, &tuning);
        assert_eq!(boost, Vec2::ZERO, "OFF tuning must produce zero boost");
    }

    #[test]
    fn ledge_boost_ignores_reverse_horizontal_momentum() {
        let tuning = MovementTuning::default();
        let contact = rightward_ledge_contact();
        // Momentum AWAY from the platform — into_platform is +1 here,
        // so a leftward (negative) vel doesn't earn a boost.
        let momentum = Vec2::new(-200.0, -200.0);
        let boost = ledge_boost(momentum, contact, 0.0, &tuning);
        assert_eq!(boost.x, 0.0, "reverse momentum should produce zero x boost");
        // Upward momentum is still rewarded though.
        assert!(
            boost.y < 0.0,
            "upward momentum should still produce a y boost"
        );
    }

    #[test]
    fn ledge_boost_ignores_downward_vertical_momentum() {
        let tuning = MovementTuning::default();
        let contact = rightward_ledge_contact();
        let momentum = Vec2::new(200.0 * into_platform_for(contact), 300.0); // falling
        let boost = ledge_boost(momentum, contact, 0.0, &tuning);
        assert_eq!(
            boost.y, 0.0,
            "downward (falling) y momentum should not boost"
        );
        // Forward horizontal momentum still counts.
        assert!(boost.x.abs() > 0.0);
    }

    #[test]
    fn ledge_boost_clamps_at_caps() {
        let tuning = MovementTuning::default();
        let contact = rightward_ledge_contact();
        // Extreme incoming momentum (e.g. dash + air jump combo).
        let momentum = Vec2::new(2_000.0 * into_platform_for(contact), -2_000.0);
        let boost = ledge_boost(momentum, contact, 0.0, &tuning);
        assert!(
            boost.x.abs() <= tuning.ledge_momentum.x_cap + 0.01,
            "x boost should clamp to x_cap"
        );
        assert!(
            boost.y.abs() <= tuning.ledge_momentum.y_cap + 0.01,
            "y boost should clamp to y_cap"
        );
    }

    #[test]
    fn ledge_jump_with_quick_action_carries_momentum() {
        let contact = rightward_ledge_contact();
        // Player came in with strong rightward (into-platform) momentum
        // before grabbing.
        let momentum = Vec2::new(220.0, -100.0);
        let mut scratch = make_hanging_player_with_momentum(contact, momentum);
        let baseline_player = make_hanging_player_with_momentum(contact, Vec2::ZERO);
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let tuning = MovementTuning::default();
        let input = InputState {
            jump_pressed: true,
            ..InputState::default()
        };
        let _ = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
        // Replay the same input on the zero-momentum baseline so we
        // can compare "with boost" vs "without boost" exit velocities.
        let mut baseline = baseline_player;
        let mut baseline_events = crate::engine_core::movement::FrameEvents::default();
        let _ = tick_active_ledge_grab_scratch(&mut baseline, input, 0.016, tuning, &mut baseline_events);
        // The boosted exit velocity should be larger in magnitude
        // along the carried axes than the unboosted one.
        assert!(
            scratch.kinematics.vel.x.abs() > baseline.kinematics.vel.x.abs(),
            "expected boosted ledge-jump to exceed baseline x: {} vs {}",
            scratch.kinematics.vel.x,
            baseline.kinematics.vel.x,
        );
        assert!(
            scratch.kinematics.vel.y < baseline.kinematics.vel.y,
            "expected boosted ledge-jump to exceed baseline upward (more negative): {} vs {}",
            scratch.kinematics.vel.y,
            baseline.kinematics.vel.y,
        );
    }

    #[test]
    fn drop_does_not_apply_boost() {
        let contact = rightward_ledge_contact();
        let momentum = Vec2::new(220.0, -120.0);
        let mut scratch = make_hanging_player_with_momentum(contact, momentum);
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let tuning = MovementTuning::default();
        let input = InputState {
            axis_y: 1.0, // down
            ..InputState::default()
        };
        let _ = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
        assert!(scratch.ledge.grab.is_none());
        // After a drop the player has no claimed launch velocity —
        // the existing behavior is that vel is whatever it was when
        // the ledge released, and for `want_drop` we leave it
        // untouched (vel was ZERO from the hang). Importantly, we
        // do NOT add any boost.
        assert_eq!(
            scratch.kinematics.vel,
            Vec2::ZERO,
            "drop must not pick up momentum boost"
        );
    }

    #[test]
    fn outward_ledge_release_does_not_apply_boost() {
        let contact = rightward_ledge_contact();
        let momentum = Vec2::new(220.0, -120.0);
        let mut scratch = make_hanging_player_with_momentum(contact, momentum);
        let baseline_player = make_hanging_player_with_momentum(contact, Vec2::ZERO);
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let tuning = MovementTuning::default();
        // jump + AWAY from platform. away_from_platform here is -1
        // (left) since wall_normal_x is -1.
        let away = away_from_platform_axis(contact);
        let input = InputState {
            jump_pressed: true,
            axis_x: away,
            ..InputState::default()
        };
        let _ = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
        // Replay with zero momentum to confirm both produce the SAME
        // exit vel (i.e. no boost applied).
        let mut baseline = baseline_player;
        let mut baseline_events = crate::engine_core::movement::FrameEvents::default();
        let _ = tick_active_ledge_grab_scratch(&mut baseline, input, 0.016, tuning, &mut baseline_events);
        assert!(
            (scratch.kinematics.vel - baseline.kinematics.vel).length() < 0.5,
            "outward release must produce identical vel with and without momentum, \
             got boosted={:?} baseline={:?}",
            scratch.kinematics.vel,
            baseline.kinematics.vel,
        );
    }

    #[test]
    fn climb_finish_carries_momentum_when_grabbed_with_speed() {
        let contact = rightward_ledge_contact();
        let momentum = Vec2::new(220.0, -120.0);
        let mut scratch = make_hanging_player_with_momentum(contact, momentum);
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let tuning = MovementTuning::default();
        // Start the climb (Up).
        let input = InputState {
            axis_y: -1.0,
            ..InputState::default()
        };
        let _ = tick_active_ledge_grab_scratch(&mut scratch, input, 0.001, tuning, &mut events);
        // Run the full climb in one big tick.
        let _ = tick_active_ledge_grab_scratch(
            &mut scratch,
            InputState::default(),
            LEDGE_CLIMB_TIME + 0.05,
            tuning,
            &mut events,
        );
        assert!(scratch.ledge.grab.is_none(), "climb should have finished");
        // Carried x velocity into platform. The forward-into is +x
        // here (right-side wall_normal, into = +1).
        assert!(
            scratch.kinematics.vel.x > 0.0,
            "expected positive x exit velocity from carry, got {}",
            scratch.kinematics.vel.x,
        );
    }

    /// Regression for Jon's "horizontal getup shouldn't be adding
    /// vertical boost" — the player just got placed standing on the
    /// platform; a residual upward vel.y would relaunch them off
    /// it. Climb / roll / attack finish must zero the Y component
    /// of the boost; ledge-jump (a vertical hop) still keeps both.
    #[test]
    fn climb_finish_does_not_carry_vertical_boost() {
        let contact = rightward_ledge_contact();
        // Strong upward incoming momentum (e.g. recovery via double-jump).
        let momentum = Vec2::new(220.0, -500.0);
        let mut scratch = make_hanging_player_with_momentum(contact, momentum);
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let tuning = MovementTuning::default();
        // Start + complete the climb in two big ticks.
        let _ = tick_active_ledge_grab_scratch(
            &mut scratch,
            InputState {
                axis_y: -1.0,
                ..InputState::default()
            },
            0.001,
            tuning,
            &mut events,
        );
        let _ = tick_active_ledge_grab_scratch(
            &mut scratch,
            InputState::default(),
            LEDGE_CLIMB_TIME + 0.05,
            tuning,
            &mut events,
        );
        assert!(scratch.ledge.grab.is_none());
        assert!(
            scratch.kinematics.vel.x > 0.0,
            "horizontal carry should still apply, got vx={}",
            scratch.kinematics.vel.x,
        );
        assert_eq!(
            scratch.kinematics.vel.y, 0.0,
            "climb-finish must NOT launch the player upward off the platform; got vy={}",
            scratch.kinematics.vel.y,
        );
    }

    /// The boost mechanic now ALSO shortens the getup transition
    /// when momentum was carried. Without this, a 0.24-s climb of
    /// dead-zero velocity feels sluggish — the post-transition kick
    /// can't compensate. Tests that a fresh-momentum getup completes
    /// in noticeably less time than a baseline getup with the
    /// speedup disabled.
    #[test]
    fn getup_transition_completes_faster_with_momentum_carry() {
        let contact = rightward_ledge_contact();
        let momentum = Vec2::new(220.0, -120.0);
        let mut boosted = make_hanging_player_with_momentum(contact, momentum);
        let mut baseline = make_hanging_player_with_momentum(contact, momentum);
        let tuning = MovementTuning::default();
        // Disable just the speedup on the baseline so the comparison
        // isolates THIS knob.
        let mut baseline_tuning = tuning;
        baseline_tuning.ledge_momentum.getup_speedup_gain = 0.0;
        let input = InputState {
            axis_y: -1.0,
            ..InputState::default()
        };
        // Start both climbs.
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let _ = tick_active_ledge_grab_scratch(&mut boosted, input, 0.001, tuning, &mut events);
        let _ = tick_active_ledge_grab_scratch(&mut baseline, input, 0.001, baseline_tuning, &mut events);
        // Step both forward by exactly the BASELINE climb time. The
        // baseline should be ~done; the boosted player should be
        // OFF the ledge already (we're past their shortened duration).
        let _ = tick_active_ledge_grab_scratch(
            &mut boosted,
            InputState::default(),
            LEDGE_CLIMB_TIME * 0.6,
            tuning,
            &mut events,
        );
        let _ = tick_active_ledge_grab_scratch(
            &mut baseline,
            InputState::default(),
            LEDGE_CLIMB_TIME * 0.6,
            baseline_tuning,
            &mut events,
        );
        assert!(
            boosted.ledge.grab.is_none(),
            "boosted climb should have completed by 60% of base duration"
        );
        assert!(
            baseline.ledge.grab.is_some(),
            "baseline climb should still be in progress at 60% of base duration"
        );
    }

    /// `try_start_ledge_grab` now prefers `pre_wall_vel` over
    /// `scratch.kinematics.vel` when the snapshot is fresh — because wall-cling
    /// and wall-collision shred the actual approach velocity by the
    /// time the grab fires. This was Jon's "I don't feel the boost
    /// on jump option at all, even with gain > 1 and caps in
    /// thousands" bug: vel was 0 at capture time, so any gain
    /// multiplied to zero.
    #[test]
    fn grab_prefers_pre_wall_vel_when_fresh() {
        let world = world_with(vec![Block::solid(
            "ledge",
            Vec2::new(100.0, 100.0),
            Vec2::new(200.0, 200.0),
        )]);
        let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
        scratch.abilities.abilities.ledge_grab = true;
        // Simulate "wall-cling killed our approach velocity": current
        // vel.x is zero (collision zeroed it), but pre_wall_vel
        // still has the approach momentum from a frame ago.
        scratch.kinematics.vel = Vec2::new(0.0, 50.0);
        scratch.wall.pre_wall_vel = Vec2::new(260.0, -180.0);
        scratch.wall.pre_wall_vel_age = 0.05; // fresh
        scratch.wall.wall_clinging = true;
        scratch.wall.wall_normal_x = -1.0;
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let latched = try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
        assert!(latched);
        let state = scratch.ledge.grab.unwrap();
        assert!(
            (state.momentum_at_grab - Vec2::new(260.0, -180.0)).length() < 0.01,
            "grab should snapshot the pre-wall vel, got {:?}",
            state.momentum_at_grab,
        );
    }

    /// Once `pre_wall_vel_age` exceeds the freshness threshold, the
    /// grab falls back to `scratch.kinematics.vel` so a player who clung the
    /// wall for ages can't claim a fossil approach.
    #[test]
    fn grab_falls_back_to_current_vel_when_pre_wall_stale() {
        let world = world_with(vec![Block::solid(
            "ledge",
            Vec2::new(100.0, 100.0),
            Vec2::new(200.0, 200.0),
        )]);
        let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
        scratch.abilities.abilities.ledge_grab = true;
        scratch.kinematics.vel = Vec2::new(0.0, 50.0);
        scratch.wall.pre_wall_vel = Vec2::new(260.0, -180.0);
        scratch.wall.pre_wall_vel_age = LEDGE_REGRAB_COOLDOWN * 4.0; // very stale
        scratch.wall.wall_clinging = true;
        scratch.wall.wall_normal_x = -1.0;
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let latched = try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
        assert!(latched);
        let state = scratch.ledge.grab.unwrap();
        // Should fall back to current vel, NOT the stale pre_wall.
        assert!(
            (state.momentum_at_grab - Vec2::new(0.0, 50.0)).length() < 0.01,
            "stale pre_wall must be discarded; got {:?}",
            state.momentum_at_grab,
        );
    }

    #[test]
    fn boost_decays_to_zero_outside_window() {
        // If the player lingers on the ledge past the boost window
        // and THEN climbs, the carry should be zero. Verifies the
        // window gate uses the grab-to-action time, not zero.
        let contact = rightward_ledge_contact();
        let momentum = Vec2::new(220.0, -120.0);
        let mut scratch = make_hanging_player_with_momentum(contact, momentum);
        let mut events = crate::engine_core::movement::FrameEvents::default();
        let tuning = MovementTuning::default();
        // Sit on the ledge for longer than the boost window with no
        // input (so we don't auto-climb).
        let dt = tuning.ledge_momentum.window + 0.05;
        let _ = tick_active_ledge_grab_scratch(&mut scratch, InputState::default(), dt, tuning, &mut events);
        // Now climb.
        let _ = tick_active_ledge_grab_scratch(
            &mut scratch,
            InputState {
                axis_y: -1.0,
                ..InputState::default()
            },
            0.001,
            tuning,
            &mut events,
        );
        let _ = tick_active_ledge_grab_scratch(
            &mut scratch,
            InputState::default(),
            LEDGE_CLIMB_TIME + 0.05,
            tuning,
            &mut events,
        );
        assert!(scratch.ledge.grab.is_none(), "climb should have finished");
        assert_eq!(
            scratch.kinematics.vel,
            Vec2::ZERO,
            "post-window climb should NOT carry momentum, got {:?}",
            scratch.kinematics.vel,
        );
    }
}
