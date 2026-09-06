use crate::world::World;

use super::events::FrameEvents;
use super::input::InputState;
use super::ops::MovementOp;
use super::tuning::{AxisJumpLaw, AxisSweptParams, ONE_WAY_DROP_THROUGH_GRACE};
use crate::player_state::BodyMode;
use crate::MotionFrame;

const LADDER_JUMP_BOOST_TIME: f32 = 0.10;

/// The ground leap itself, factored out because a body with a jump-squat pays
/// it a few ticks after the press and a body without one pays it on the press
/// tick — the SAME leap either way. do not inline a second copy into the
/// squat-expiry branch; the launch band, the air-jump refill and the `Jump` op
/// are one rule.
fn launch_ground_jump(
    state: &mut crate::movement::AxisManeuverState,
    abilities: &crate::body_clusters::BodyAbilities,
    kinematics: &mut crate::body_clusters::BodyKinematics,
    ground: &mut crate::body_clusters::BodyGroundState,
    jump_state: &mut crate::body_clusters::BodyJumpState,
    combo_trace: &mut crate::body_clusters::BodyComboTrace,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    events: &mut FrameEvents,
) {
    let side_speed = kinematics.vel.dot(frame.side()).abs();
    let (launch_speed, launch_band) = tuning
        .locomotion
        .jump_law
        .ground_launch(tuning.locomotion.jump_speed, side_speed);
    super::integration::set_jump_velocity(&mut kinematics.vel, frame.down(), launch_speed);
    match launch_band {
        Some(band) => state.phased_jump.begin(band),
        None => state.phased_jump.clear(),
    }
    ground.on_ground = false;
    state.buffer_jump = 0.0;
    state.coyote_timer = 0.0;
    jump_state.air_jumps_available = abilities
        .abilities
        .air_jump_count(tuning.locomotion.air_jumps);
    events.op_clusters(combo_trace, MovementOp::Jump);
}

/// Age one frame of a committed jump-squat and, if the crouch has finished,
/// take off. Called both on the tick the squat STARTS (so the press frame is the
/// first crouch frame) and on every tick after it.
#[allow(clippy::too_many_arguments)]
fn tick_jump_squat(
    state: &mut crate::movement::AxisManeuverState,
    abilities: &crate::body_clusters::BodyAbilities,
    kinematics: &mut crate::body_clusters::BodyKinematics,
    ground: &mut crate::body_clusters::BodyGroundState,
    jump_state: &mut crate::body_clusters::BodyJumpState,
    combo_trace: &mut crate::body_clusters::BodyComboTrace,
    input: InputState,
    dt: f32,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    events: &mut FrameEvents,
) {
    state.jump_squat_timer = (state.jump_squat_timer - dt).max(0.0);
    // an authored squat is a WHOLE NUMBER OF FRAMES times `dt`, and f32
    // subtraction does not land on zero: a 3-frame squat leaves ~3e-9s behind
    // and the body crouches forever. A remainder far below a tick is not a
    // crouch frame.
    if state.jump_squat_timer > dt * 1e-3 {
        return;
    }
    state.jump_squat_timer = 0.0;
    // a squat is a COMMITMENT, and the thing you can be knocked out of.
    // Losing the floor mid-crouch (struck, platform gone) voids the leap rather
    // than owing it in the air; that is the whole point of the startup existing.
    if !ground.on_ground {
        return;
    }
    launch_ground_jump(
        state,
        abilities,
        kinematics,
        ground,
        jump_state,
        combo_trace,
        frame,
        tuning,
        events,
    );
    // Honour it now through the body's own variable-jump law instead of authoring a second "short
    // hop" number.
    if !input.jump_held() {
        super::abilities::cut_ascent_now(kinematics, state, abilities, frame, tuning);
    }
}

/// Consume the buffered jump (if any) and emit the right verb:
/// swim stroke while submerged + swim ability, drop-through gate
/// while standing on a one-way + drop_through_pressed, wall-jump,
/// regular jump, or double-jump. Each branch zeroes the buffer +
/// coyote timer so the same press can't re-fire.
#[allow(clippy::too_many_arguments)]
/// LEAVE THE SURFACE DOWNWARD — the outcome both drop gestures share.
///
/// One function so guard+down and jump+down cannot leave the body in two
/// different states; the grace timer is what keeps the surface passable for
/// long enough to actually get clear of it.
fn begin_drop_through(
    state: &mut crate::movement::AxisManeuverState,
    ground: &mut crate::body_clusters::BodyGroundState,
) {
    state.buffer_jump = 0.0;
    ground.on_ground = false;
    state.coyote_timer = 0.0;
    state.drop_through_timer = ONE_WAY_DROP_THROUGH_GRACE;
}

pub fn handle_jump_buffer_clusters(
    world: &World,
    // THE PLATFORM DROP, decided by the kernel against the surface under the
    // body (see `wants_platform_drop`). A second road to the SAME
    // drop-through this function already performs — not a second mechanic.
    platform_drop: bool,
    state: &mut crate::movement::AxisManeuverState,
    env_contact: &crate::body_clusters::BodyEnvironmentContact,
    abilities: &crate::body_clusters::BodyAbilities,
    body_mode: BodyMode,
    // Movement-verb taxonomy: `jump`/`double-jump` are GROUNDED-mode verbs. A body
    // currently in FLIGHT mode steers vertically through the flight limb
    // (ascend/descend), so the buffered jump must NOT become a grounded leap — else
    // a possessed flyer "jumps straight up". Grounded/hybrid bodies (fly off) are
    // unaffected. Wall/ladder/swim keep their own context gates below.
    flying: bool,
    kinematics: &mut crate::body_clusters::BodyKinematics,
    ground: &mut crate::body_clusters::BodyGroundState,
    wall: &mut crate::body_clusters::BodyWallState,
    jump_state: &mut crate::body_clusters::BodyJumpState,
    combo_trace: &mut crate::body_clusters::BodyComboTrace,
    input: InputState,
    dt: f32,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    events: &mut FrameEvents,
) {
    // A committed jump-squat outranks the buffer, because the press that
    // started it is ALREADY SPENT: the only question left is whether the crouch
    // has finished. Resolving it first is also what stops a mash from
    // re-entering the squat and holding the body on the floor forever.
    if state.jump_squat_timer > 0.0 {
        tick_jump_squat(
            state,
            abilities,
            kinematics,
            ground,
            jump_state,
            combo_trace,
            input,
            dt,
            frame,
            tuning,
            events,
        );
        return;
    }

    // ⛔⛔ THE PLATFORM DROP IS NOT A JUMP, AND IT IS ANSWERED BEFORE THE JUMP
    // GATE BELOW. The drop-through this function performs has always been
    // reachable only through that gate — a body with no jump press and no
    // buffered one returns before ever reaching it — so guard + down would have
    // been silently swallowed no matter how the gesture was declared. It shares
    // the OUTCOME with the jump road, not the entry.
    if platform_drop {
        begin_drop_through(state, ground);
        return;
    }

    // A zero-duration buffer still means "honor the press on this tick". The
    // timer extends that edge into later ticks; it is not the authority for the
    // edge that is already present in this input frame.
    let current_press = input.jump_pressed() && abilities.abilities.jump;
    if !current_press && state.buffer_jump <= 0.0 {
        return;
    }

    let basis = frame.basis();

    if let Some(contact) = env_contact.water {
        if abilities.abilities.swim {
            let impulse = contact.spec.swim_up_impulse;
            let ascend_target = -impulse;
            let descend = kinematics.vel.dot(basis.down);
            if descend > ascend_target {
                kinematics.vel += basis.down * (ascend_target - descend);
            }
            state.buffer_jump = 0.0;
            state.coyote_timer = 0.0;
            events.op_clusters(combo_trace, MovementOp::SwimStroke);
            return;
        }
    }

    let on_ladder = env_contact.climbable.is_some();

    if super::integration::wants_drop_through(input.local_axis().y, input.jump_pressed())
        && on_ladder
    {
        jump_state.ladder_drop_through_timer = ONE_WAY_DROP_THROUGH_GRACE;
        jump_state.ladder_drop_through_hold_lock = true;
        jump_state.ladder_jump_boost = 0.0;
        let descend = kinematics.vel.dot(basis.down);
        if descend < 0.0 {
            kinematics.vel -= basis.down * descend;
        }
        state.buffer_jump = 0.0;
        state.coyote_timer = 0.0;
        return;
    }

    if body_mode == BodyMode::Climbing && on_ladder {
        // "Press away from the feet + jump" boosts off the ladder (gravity- +
        // input-mode-relative via the resolved descend).
        if abilities.abilities.jump && input.local_axis().y < -0.1 {
            jump_state.ladder_jump_boost = LADDER_JUMP_BOOST_TIME;
            events.op_clusters(combo_trace, MovementOp::Jump);
        }
        state.buffer_jump = 0.0;
        state.coyote_timer = 0.0;
        return;
    }

    let can_ladder_jump = on_ladder && !ground.on_ground;
    if super::integration::wants_drop_through(input.local_axis().y, input.jump_pressed())
        && ground.on_ground
        && crate::movement::collision::standing_on_one_way_aabb(
            world,
            kinematics.aabb_oriented(frame.down()),
            frame.down(),
        )
    {
        begin_drop_through(state, ground);
        return;
    }

    if abilities.abilities.wall_jump && wall.on_wall && !ground.on_ground {
        let basis = frame.basis();
        let target_side = wall.wall_normal_x * tuning.locomotion.wall_jump_x;
        let cur_side = kinematics.vel.dot(basis.side);
        kinematics.vel += basis.side * (target_side - cur_side);
        super::integration::set_jump_velocity(
            &mut kinematics.vel,
            frame.down(),
            tuning.locomotion.jump_speed * 0.94,
        );
        state.phased_jump.clear();
        wall.on_wall = false;
        state.wall_clinging = false;
        state.wall_climbing = false;
        state.buffer_jump = 0.0;
        state.coyote_timer = 0.0;
        events.op_clusters(combo_trace, MovementOp::WallJump);
    } else if abilities.abilities.jump
        && !flying
        && (ground.on_ground || state.coyote_timer > 0.0 || can_ladder_jump)
    {
        // A squat is a GROUNDED crouch, so a coyote-grace or ladder jump — where
        // the floor is already gone — takes off immediately. There is nothing
        // left to crouch on.
        if tuning.locomotion.jump_squat_time > 0.0 && ground.on_ground {
            state.jump_squat_timer = tuning.locomotion.jump_squat_time;
            state.buffer_jump = 0.0;
            state.coyote_timer = 0.0;
            // the press tick is the FIRST crouch frame, not a free one before
            // it. Charging the whole squat and waiting for the next tick would
            // make an N-frame authored squat cost N+1, and a squat shorter than
            // one tick cost a whole one instead of nothing.
            tick_jump_squat(
                state,
                abilities,
                kinematics,
                ground,
                jump_state,
                combo_trace,
                input,
                dt,
                frame,
                tuning,
                events,
            );
        } else {
            launch_ground_jump(
                state,
                abilities,
                kinematics,
                ground,
                jump_state,
                combo_trace,
                frame,
                tuning,
                events,
            );
        }
    } else if jump_state.footstool_claimed && tuning.abilities.footstool.is_enabled() {
        // AHEAD OF THE AIR JUMP, and that ordering IS the mechanic. The
        // press resolves as a footstool and costs nothing: a body that has spent
        // every midair jump can still bounce off a head, which is the genre's
        // rule. When this was applied AFTER the kernel by overwriting velocity,
        // the same footstool cost an air jump when you had one and nothing when
        // you did not — one input edge with two meanings.
        //
        // the claim is spent here, so one press is one footstool however many
        // heads are under the feet. The pair pass owns WHICH head; this owns
        // what the press means.
        jump_state.footstool_claimed = false;
        super::integration::set_jump_velocity(
            &mut kinematics.vel,
            frame.down(),
            tuning.abilities.footstool.rise_speed,
        );
        match tuning.locomotion.jump_law {
            AxisJumpLaw::VelocityCut => state.phased_jump.clear(),
            AxisJumpLaw::PhasedGravity(params) => {
                let band = params.band_for_side_speed(kinematics.vel.dot(frame.side()).abs());
                state.phased_jump.begin(band);
            }
        }
        ground.on_ground = false;
        wall.on_wall = false;
        state.wall_clinging = false;
        state.wall_climbing = false;
        state.buffer_jump = 0.0;
        events.op_clusters(combo_trace, MovementOp::Footstool);
    } else if abilities.abilities.double_jump && !flying && jump_state.air_jumps_available > 0 {
        super::integration::set_jump_velocity(
            &mut kinematics.vel,
            frame.down(),
            tuning.locomotion.double_jump_speed,
        );
        // ⭐⭐ THE ONLY PLACE THAT EVER GRANTS OWNED RISE. `set_jump_velocity`
        // SETS the rise component, so what the jump put in is exactly this
        // number; from here `integrate` can only shrink it. A double-jump
        // cancel may take back at most this much, which is what stops it from
        // eating an opponent's launch.
        state.air_jump_rise_owned = tuning.locomotion.double_jump_speed;
        // Air jumps are independent ability impulses. Do not accidentally
        // inherit a ground jump's weak-gravity phase.
        match tuning.locomotion.jump_law {
            AxisJumpLaw::VelocityCut => state.phased_jump.clear(),
            AxisJumpLaw::PhasedGravity(params) => {
                let band = params.band_for_side_speed(kinematics.vel.dot(frame.side()).abs());
                state.phased_jump.begin(band);
            }
        }
        ground.on_ground = false;
        wall.on_wall = false;
        state.wall_clinging = false;
        state.wall_climbing = false;
        state.buffer_jump = 0.0;
        jump_state.air_jumps_available -= 1;
        events.op_clusters(combo_trace, MovementOp::DoubleJump);
    }
}
