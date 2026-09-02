use crate::geometry::AabbExt;
use crate::world::World;
use crate::Vec2;

/// Move `value` toward `target` by at most `delta`. Inlined from the
/// removed `ae::scalar::approach`.
fn approach(value: f32, target: f32, delta: f32) -> f32 {
    if value < target {
        (value + delta).min(target)
    } else {
        (value - delta).max(target)
    }
}

/// Clamp the velocity component ALONG `gravity_dir` (the fall direction) to
/// `cap`, leaving the perpendicular (movement) component untouched. The
/// gravity-direction-relative form of `vel.y = vel.y.min(cap)`.
fn cap_fall_speed(vel: &mut crate::Vec2, gravity_dir: crate::Vec2, cap: f32) {
    let along = vel.dot(gravity_dir);
    if along > cap {
        *vel -= (along - cap) * gravity_dir;
    }
}

/// Launch the body at `speed` OPPOSITE `gravity_dir` (a jump / pogo / wall-kick
/// vertical impulse), preserving the perpendicular (movement-axis) component.
/// The gravity-direction-relative form of `vel.y = -speed * gravity_sign`.
pub fn set_jump_velocity(vel: &mut crate::Vec2, gravity_dir: crate::Vec2, speed: f32) {
    let perp = *vel - vel.dot(gravity_dir) * gravity_dir;
    *vel = perp - speed * gravity_dir;
}

/// Screen-vertical input (`axis_y`, +Y = screen-down) → the gravity-relative
/// "descend" (toward-the-feet) intent that gates crouch / pogo / drop-through /
/// fast-fall and drives gravity-relative vertical movement. The vertical sibling
/// of the run axis ([`crate::AccelerationFrame::control_frame`]'s `side`): that
/// keeps the run axis player-relative, this keeps the gate axis sign-consistent.
///
/// CONVENTION — this game's; change it HERE and every gate moves together. The
/// gate stays on the up/down keys; its sign flips only when gravity rotates PAST
/// ±90° from screen-down (its screen-down component goes negative). So down AND
/// sideways gravity read screen-down as "descend"; only past horizontal (gravity
/// pointing up-ish) does screen-up become "descend". For default down gravity
/// this is the identity, so normal play is byte-identical.
pub fn gravity_descend(axis_y: f32, gravity_dir: crate::Vec2) -> f32 {
    let gate_sign = if gravity_dir.y < 0.0 { -1.0 } else { 1.0 };
    axis_y * gate_sign
}

/// The "drop through a one-way platform" gesture: press the descend gate (toward
/// the feet) + jump. The `descend` scalar is the resolved player-frame `y` from
/// [`AxisSweptParams::stick`], so it is gravity- AND input-mode-relative (under
/// inverted gravity, Hybrid reads screen-UP + jump). Computed at the consumer
/// rather than precomputed gravity-blind at the input boundary.
pub(super) fn wants_drop_through(descend: f32, jump_pressed: bool) -> bool {
    descend > DROP_THROUGH_DESCEND && jump_pressed
}

/// How far toward the feet the stick must be held for a drop request to count,
/// shared by both gestures so guard+down and jump+down cannot disagree about
/// what "down" is.
pub(super) const DROP_THROUGH_DESCEND: f32 = 0.35;

/// THE PLATFORM DROP: guard + down, on a surface that can be left downward.
///
/// ⛔ THE BUTTON, NOT `BodyShieldState::active`, exactly as the shield evade
/// reads it: the guard state outlives the press that raised it, so reading the
/// state would drop a body through a platform off a guard it already let go of.
pub(super) fn wants_platform_drop(descend: f32, shield_held: bool, on_one_way: bool) -> bool {
    shield_held && on_one_way && descend > DROP_THROUGH_DESCEND
}

/// THE PLATFORM DROP, asked of the world — the whole question, in one place.
///
/// ⛔⛔ ONE IMPLEMENTATION FOR TWO PHASES, and that is the point of the
/// function. The control phase must know so the evade STANDS DOWN, and the
/// simulation phase must know so the drop FIRES; they are separate top-level
/// passes, and two hand-written copies of this condition that drifted apart
/// would produce a press that stands the dodge down and then drops nobody — an
/// input that does nothing at all, which is the worst failure available here.
///
/// Both callers ask before the body has moved this tick, so they are asking
/// about the same pose and cannot disagree.
pub(super) fn platform_drop_requested(
    world: &crate::World,
    kinematics: &crate::body_clusters::BodyKinematics,
    ground: &crate::body_clusters::BodyGroundState,
    input: InputState,
    frame: crate::MotionFrame,
    tuning: AxisSweptParams,
) -> bool {
    tuning.abilities.shield.platform_drop
        && ground.on_ground
        && wants_platform_drop(
            input.local_axis().y,
            input.shield_held,
            super::collision::standing_on_one_way_aabb(
                world,
                kinematics.aabb_oriented(frame.down()),
                frame.down(),
            ),
        )
}

use super::dec;
use super::events::FrameEvents;
use super::input::InputState;
use super::model::{AxisManeuverState, PhasedJumpState};
use super::ops::MovementOp;
use super::tuning::{AxisHorizontalLaw, AxisJumpLaw, AxisSweptParams};
use crate::MotionFrame;

/// Apply one frame of velocity integration to the player: mode-select
/// between dash / climb / flight / normal physics, run the per-mode
/// integration, sweep the kinematics through X then Y collisions,
/// apply wall abilities + rebound + end-of-frame `pre_wall_vel`
/// bookkeeping. Reads and writes the shared clusters plus the axis
/// policy's model-private maneuver `state`.
#[allow(clippy::too_many_arguments)]
pub(super) fn integrate_velocity_clusters(
    world: &World,
    clusters: &mut crate::body_clusters::BodyClustersMut<'_>,
    state: &mut AxisManeuverState,
    input: InputState,
    dt: f32,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    contact: super::body_contact::BodyContactField<'_>,
    // See `MotionStepContext::recovery_commitment_outstanding`.
    recovery_commitment_outstanding: bool,
    events: &mut FrameEvents,
) {
    use crate::player_state::BodyMode;

    let climbing = clusters.body_mode.body_mode == BodyMode::Climbing
        && clusters.env_contact.climbable.is_some();
    if !climbing {
        clusters.jump.ladder_jump_boost = 0.0;
    }

    // ⭐⭐ AHEAD OF EVERY OTHER MODE, INCLUDING THE DASH TIMER. A body under the
    // stage is not doing anything else: a dash that survived the drop would
    // carry her out from under it, and flight would fight the mode for the same
    // velocity. Being absent outranks being busy.
    if clusters.body_mode.body_mode == BodyMode::Submerged {
        integrate_submerged_clusters(world, clusters.kinematics, state, input, dt, frame, tuning);
    } else if state.wire.is_some() {
        // ⭐⭐ AND A BODY ON A WIRE OUTRANKS EVERY MODE BELOW IT, for the reason
        // the submerged branch above outranks them: the wire decides where she
        // is. A dash timer, a climb or flight running underneath would each
        // write the same velocity the rope just wrote, and the last writer would
        // decide the move.
        //
        // ⛔ BENEATH THE TRAPDOOR, THOUGH, AND THE ORDER IS DELIBERATE. The two
        // are mutually exclusive by authoring — one move opens a door, the other
        // drops a rope — but a body that ended up in both is a body under the
        // stage, and being absent from the world outranks being on a wire.
        integrate_wire_clusters(clusters.kinematics, state, input, dt, frame);
    } else if state.dash_timer > 0.0 {
        state.dash_timer = dec(state.dash_timer, dt);
    } else if climbing {
        integrate_climb_clusters(
            clusters.kinematics,
            clusters.env_contact,
            state,
            clusters.jump,
            input,
            dt,
            frame,
            tuning,
        );
    } else if clusters.flight.fly_enabled && clusters.abilities.abilities.fly {
        integrate_flight_clusters(clusters.kinematics, state, input, dt, frame, tuning);
    } else {
        // Normal mode — the shared physics spine (gravity-direction-relative).
        integrate_normal_clusters(
            clusters.kinematics,
            clusters.flight,
            state,
            clusters.ground,
            clusters.env_contact,
            clusters.abilities,
            clusters.body_mode,
            input,
            dt,
            frame,
            tuning,
        );
    }

    // Pre-sweep state.
    clusters.wall.on_wall = false;
    let pre_wall_snapshot = clusters.kinematics.vel;
    clusters.wall.wall_normal_x = 0.0;
    state.wall_climbing = false;
    let was_clinging = state.wall_clinging;
    state.wall_clinging = false;

    // The sweeps are still X/Y because the world is axis-aligned, but both the ORDER and the
    // SEMANTICS are local-frame: sweep the controlled body's side axis first (arming wall
    // contact), apply wall abilities against last-frame ground state, clear ground, then sweep
    // the gravity/support axis, which owns landing.
    let gravity_on_x = frame.down().x != 0.0;
    let (side_axis, gravity_axis) = if gravity_on_x {
        (
            crate::collision_semantics::Axis::Y,
            crate::collision_semantics::Axis::X,
        )
    } else {
        (
            crate::collision_semantics::Axis::X,
            crate::collision_semantics::Axis::Y,
        )
    };

    let drop_through = wants_drop_through(input.local_axis().y, input.jump_pressed())
        || state.drop_through_timer > 0.0;

    let sweep =
        |clusters: &mut crate::body_clusters::BodyClustersMut<'_>,
         axis: crate::collision_semantics::Axis,
         contacts: &mut Vec<crate::collision_semantics::Contact>,
         conflicts: &mut Vec<crate::collision_semantics::AxisConstraintConflict>| {
            let prev_feet_coord = clusters
                .kinematics
                .aabb_oriented(frame.down())
                .feet_coord(frame.down());
            let delta_along = match axis {
                crate::collision_semantics::Axis::X => clusters.kinematics.vel.x,
                crate::collision_semantics::Axis::Y => clusters.kinematics.vel.y,
            } * dt;
            // Nothing is separated afterwards.
            //
            // SIDE AXIS AND GROUNDED ONLY, first slice. Standing ON another
            // body is `footstool`, which already exists and means something else;
            // and an airborne fighter passing another one is Smash-correct. The
            // ground flag read here is the body's ENTRY state — the side sweep
            // runs before support is re-established — which is the right answer
            // to *is this body standing* for this step.
            //
            // AND IT IS NOT A FORCE. A term summed into `vel` is erased:
            // `approach()` overwrites `vel` toward the input target every tick,
            // which is why the acceleration version of this had eight green tests
            // and moved nothing in a real match (`bbbc5e46c`).
            let delta_along = if axis == side_axis && clusters.ground.on_ground {
                super::body_contact::constrain_motion(
                    clusters.kinematics.aabb_oriented(frame.down()),
                    delta_along,
                    matches!(axis, crate::collision_semantics::Axis::X),
                    // ONE WALK'S WORTH, and no more. A body standing in
                    // the way pushes back with the force of standing there; a
                    // launched body ploughs through it. Without this the
                    // constraint ate knockback and two `smash_it` guards about
                    // matches ENDING went red.
                    tuning.locomotion.max_run_speed * dt,
                    dt,
                    contact,
                )
            } else {
                delta_along
            };
            // A crush on this axis rides out on the frame's events for the body's
            // OWNER to interpret; the kernel has already reported the contacts and
            // refused to invent a position no surface accepts.
            conflicts.extend(super::collision::sweep_player_axis_clusters(
                world,
                clusters.kinematics,
                clusters.ground,
                clusters.wall,
                clusters.body_mode,
                clusters.env_contact,
                axis,
                delta_along,
                prev_feet_coord,
                drop_through,
                frame.down(),
                contacts,
            ));
        };

    sweep(
        clusters,
        side_axis,
        &mut events.contacts,
        &mut events.constraint_conflicts,
    );
    apply_wall_abilities_clusters(
        clusters.kinematics,
        clusters.ground,
        clusters.wall,
        state,
        clusters.abilities,
        clusters.combo_trace,
        input,
        frame,
        tuning,
        was_clinging,
        events,
    );
    clusters.ground.on_ground = false;
    // Cleared with `on_ground` beside it: both ends of the gravity axis are
    // re-sampled by the sweep below, and a stale head contact would let a body
    // tech off a ceiling it left last tick.
    clusters.ground.head_contact = false;
    sweep(
        clusters,
        gravity_axis,
        &mut events.contacts,
        &mut events.constraint_conflicts,
    );

    // Emergent platform riding, and every body gets it because every body is
    // swept here: a grounded body resting on a MOVING solid is carried
    // by that solid's gravity-perpendicular velocity (the gravity-axis ride is
    // already handled by gravity + the landing). Static geometry carries `ZERO`, so
    // this is a no-op off moving platforms. This is why the player — and the
    // brain-driven clone, which runs this exact core — ride moving platforms: not a
    // player feature, a property of standing on a moving solid.
    if clusters.ground.on_ground {
        let g = frame.down();
        let oriented = clusters.kinematics.aabb_oriented(g);
        if let Some(support) = crate::collision_semantics::supporting_block(
            world,
            oriented,
            g,
            state.drop_through_timer > 0.0,
        ) {
            let v = support.velocity;
            clusters.kinematics.pos += v - v.dot(g) * g;
            // The grounded-frame REST contact: the one place per frame that
            // knows both the support and its frame motion, so the contact
            // carries `surface_velocity` (moving-platform carry made visible).
            // The support face's TRUE outward normal (a cardinal block face);
            // equals -g for cardinal gravity, and stays the surface fact under
            // an oblique frame.
            let axis = crate::collision_semantics::gravity_axis(g);
            let sign = match axis {
                crate::collision_semantics::Axis::X => -g.x.signum(),
                crate::collision_semantics::Axis::Y => -g.y.signum(),
            };
            let normal = match axis {
                crate::collision_semantics::Axis::X => crate::Vec2::new(sign, 0.0),
                crate::collision_semantics::Axis::Y => crate::Vec2::new(0.0, sign),
            };
            events
                .contacts
                .push(crate::collision_semantics::block_face_contact(
                    oriented,
                    support,
                    normal,
                    0.0,
                    crate::collision_semantics::ContactKind::Support,
                    crate::collision_semantics::closing_speed(clusters.kinematics.vel, normal),
                ));
        }
    }

    if clusters.ground.on_ground {
        crate::body_clusters::refresh_movement_resources_clusters(
            clusters.abilities,
            &mut *clusters.dash,
            &mut *clusters.jump,
            &mut *clusters.dodge,
            tuning.locomotion.air_jumps,
            super::recovery_refresh(recovery_commitment_outstanding),
        );
        state.blink_grace_timer = 0.0;
        state.fast_falling = false;
        state.gliding = false;
        state.wall_clinging = false;
        state.wall_climbing = false;
        state.drop_through_timer = 0.0;
        state.phased_jump.clear();
    } else if events
        .contacts
        .iter()
        .any(|contact| contact.kind == crate::collision_semantics::ContactKind::Head)
    {
        // A ceiling impact ends the controllable ascent. The next tick is a
        // normal fall in whatever gravity frame the environment resolves then.
        state.phased_jump.clear();
    }

    if clusters.abilities.abilities.rebound && state.rebound_cooldown <= 0.0 {
        if let Some(impulse) = super::collision::touching_rebound_aabb(
            world,
            clusters.kinematics.aabb_oriented(frame.down()),
        ) {
            clusters.kinematics.vel = impulse;
            crate::body_clusters::refresh_movement_resources_clusters(
                clusters.abilities,
                &mut *clusters.dash,
                &mut *clusters.jump,
                &mut *clusters.dodge,
                tuning.locomotion.air_jumps,
                // A rebound pad throws the body: it re-seats it the way a ledge
                // does, and it is an EVENT rather than a per-tick truth, so it
                // answers for the recovery outright.
                crate::body_clusters::RecoveryRefresh::Answered,
            );
            clusters.ground.on_ground = false;
            state.phased_jump.clear();
            state.rebound_cooldown = 0.18;
            events.op_clusters(clusters.combo_trace, MovementOp::Rebound);
        }
    }

    // End-of-integration: if the frame settled into airborne free
    // flight, commit the pre-wall snapshot as the most recent valid
    // `pre_wall_vel`.
    if !clusters.ground.on_ground && !state.wall_clinging {
        state.pre_wall_vel = pre_wall_snapshot;
        state.pre_wall_vel_age = 0.0;
    }

    // THE GAIT. Written HERE, at the end of the one step every axis-swept
    // body takes, so the fact reaches actors, players and bosses alike — the
    // three places that republish `BodyMotionFacts` are a carry list, and a
    // locomotion fact maintained at one of them would be a running attack the
    // CPU could not perform.
    //
    // post-sweep on purpose: a body pinned against a wall has been stopped,
    // and its Attack press is a standing one.
    let travel = clusters.kinematics.vel.dot(frame.side());
    let steer = input.local_axis().x;
    // ⛔⛔ A DASH IS NOT A RUN, and the distinction is the genre's rather than a
    // nicety. `running` is a SPEED test, and the initial dash is at full speed
    // on its first tick — so without this clause a body would be "running" the
    // instant it pressed a direction, and every reader of the fact would agree.
    //
    // ⭐ MEASURED, not reasoned: it cost the forward smash. The move selector
    // reads this fact, so a forward press + attack came out as
    // `player_robot_dash_attack` where it had always been `smash_forward` —
    // `a_quick_forward_smash_barely_travels_but_plain_forward_still_walks`
    // caught it. Committing to a run is what `run_commit_frac` names, and a
    // body still inside its dash window has committed to nothing.
    // ⭐ ON THE BRINK — derived here, beside `running`, because both are
    // POST-SWEEP facts about where the body actually ended up. Asking before
    // the sweep would describe the pose the body was leaving rather than the
    // one it is standing in.
    state.teetering = clusters.ground.on_ground
        && super::collision::teetering_at_edge(
            world,
            clusters.kinematics.aabb_oriented(frame.down()),
            frame,
            clusters.kinematics.facing,
            tuning.locomotion.teeter_margin,
        );
    // ⭐ WHAT IS LEFT OF THE RISE THE AIR JUMP PUT IN — the amount a
    // double-jump cancel may take back. Post-sweep beside the other two for the
    // same reason: it describes where the body ENDED UP, not the pose it was
    // leaving.
    //
    // ⛔⛔ IT ONLY SHRINKS HERE. The spend (`abilities::apply_jump_release`) is the one
    // place that ever raises it, to the jump's authored speed; this clamps it
    // down to whatever rise survived the step. Gravity eating the jump reduces
    // it; an opponent's launch ADDING rise cannot grow it back.
    //
    // ⭐ THAT ASYMMETRY IS THE OWNERSHIP. What stood here was a magnitude test
    // — "an air jump was spent at some point, and I am rising no faster than
    // one could push me" — and the resource half of it stays true for the rest
    // of the airtime, so a fighter launched upward at any speed below
    // `double_jump_speed` read as riding its own jump and an aerial deleted the
    // launch. `min` cannot make that mistake: it never hands the body rise it
    // did not buy.
    let rise = -clusters.kinematics.vel.dot(frame.down());
    state.air_jump_rise_owned = if clusters.ground.on_ground {
        // Landing settles the account, whatever the ground is doing.
        0.0
    } else {
        state.air_jump_rise_owned.min(rise).max(0.0)
    };
    state.running = clusters.ground.on_ground
        && state.initial_dash_timer <= 0.0
        && steer * travel > 0.0
        && travel.abs() >= tuning.locomotion.run_commit_frac * tuning.locomotion.max_run_speed;
}

/// Ladder integration: drive vel.y from `axis_y * climb_speed`, scale x by `strafe_factor`, and
/// clear transient flight flags. Suspends gravity by overwriting `vel.y` rather than
/// accumulating. Normal-mode integration — the shared physics SPINE (not a composable limb):
/// gravity-direction-relative gravity, fast-fall, glide-gate, run/friction, and the fall-speed
/// cap. The fourth mode-select branch alongside dash (skip), climb, and flight.
pub(super) fn integrate_normal_clusters(
    kinematics: &mut crate::body_clusters::BodyKinematics,
    flight: &mut crate::body_clusters::BodyFlightState,
    state: &mut AxisManeuverState,
    ground: &crate::body_clusters::BodyGroundState,
    env_contact: &crate::body_clusters::BodyEnvironmentContact,
    abilities: &crate::body_clusters::BodyAbilities,
    // This body's STANCE, for the one thing the spine asks of it: a crouch caps
    // how fast the stick may carry you. Threaded rather than read from clusters
    // because the spine is actor-generic and takes neither.
    body_mode: &crate::body_clusters::BodyModeState,
    input: InputState,
    dt: f32,
    frame: MotionFrame,
    tuning: AxisSweptParams,
) {
    // The player adapter: project its clusters + private maneuver state into
    // the actor-generic spine context (ability components → gating flags) and
    // run the one spine.
    let initial_dash_dir = resolve_initial_dash(state, ground.on_ground, input, dt, tuning);

    integrate_normal_spine(
        &mut kinematics.vel,
        &mut state.fast_falling,
        &mut state.gliding,
        &mut flight.carried_run,
        &mut flight.carried_hold,
        &mut state.phased_jump,
        NormalSpineCtx {
            on_ground: ground.on_ground,
            blink_grace: state.blink_grace_timer > 0.0,
            water: env_contact.water,
            can_fast_fall: abilities.abilities.fast_fall,
            can_glide: abilities.abilities.glide,
            crouching: body_mode.body_mode == crate::player_state::BodyMode::Crouching,
            initial_dash_dir,
            // ⛔ A GROUNDED BODY HOLDING SHIELD DOES NOT WALK. Jon, 2026-08-23:
            // *"If the player is holding shield... they should not be let the
            // control move them left or right."* That is the genre's rule and it
            // is what makes shield+direction mean ROLL rather than "shuffle
            // sideways with the guard up": the stick behind a raised guard is
            // choosing an evade, not steering.
            //
            // ⛔ GROUNDED ONLY — in the air the shield button is not a guard,
            // and air control is not this rule's business.
            //
            // ⚠ It does not fight the evade: a roll and a spot dodge SET
            // velocity directly, they do not steer through this term.
            // ⛔⛔ AND AN ACCEPTED EVADE OWNS ITS OWN TRAVEL. Gating on
            // `shield_held` alone meant a roll kept steering rights the moment
            // the hand let go of the guard, so the ordinary friction/steer law
            // resumed editing a velocity the roll had authored: the same roll
            // covered 107px held and 33px released, which is Jon's playtest
            // ("roll distance is input-dependent AFTER the roll has begun").
            // A maneuver the game has already accepted is not still taking input.
            //
            // ⛔ NOT GATED ON `on_ground`. A roll that leaves the edge keeps the
            // momentum it authored; handing air control back mid-roll would let
            // the stick edit the same velocity from the other law instead.
            can_move_horizontal: abilities.abilities.move_horizontal
                && !(input.shield_held && ground.on_ground)
                && state.dodge_roll_timer <= 0.0,
            // ⭐ PLANTED, NOT MERELY UNSTEERABLE. A guard raised mid-run sheds
            // the run; an EVADE does not, because a roll and a spot dodge set
            // their own velocity and this law must not brake them.
            settling: ground.on_ground
                && state.dodge_roll_timer <= 0.0
                && state.air_dodge_timer <= 0.0,
            can_variable_jump: abilities.abilities.variable_jump,
        },
        input,
        dt,
        frame,
        tuning,
    );
}

/// Read-only gating the normal-mode spine consults. Every field models a player
/// ability/contact; an actor that carries none of those components feeds
/// `on_ground` + `can_move_horizontal` and leaves the rest at their "absent"
/// values, getting pure gravity + run + fall-cap. This is the pay-for-use seam:
/// the spine is the SAME core the player runs with its abilities switched on.
#[derive(Clone, Copy)]
pub struct NormalSpineCtx {
    pub on_ground: bool,
    /// Blink hang-time is active this frame
    /// (`AxisManeuverState::blink_grace_timer > 0`).
    pub blink_grace: bool,
    pub water: Option<crate::world::WaterContact>,
    pub can_fast_fall: bool,
    pub can_glide: bool,
    pub can_move_horizontal: bool,
    /// Is this body CROUCHING? Carried on the context rather than read from the
    /// clusters here, because this function takes neither — the same reason
    /// `on_ground` is a field. See `crouch_speed_frac`.
    pub crouching: bool,
    /// THE INITIAL DASH's direction this tick, `0.0` when the phase is not
    /// running. Resolved by the caller for the same reason `crouching` is: this
    /// function takes neither the clusters nor the maneuver state.
    pub initial_dash_dir: f32,
    /// This body is PLANTED: grounded, not steering, and nothing else is
    /// driving its ground speed — so it should shed that speed to friction.
    ///
    /// ⛔ Resolved by the caller because the exclusion is an EVADE, which is
    /// maneuver state this function does not hold. A roll is shield-held too
    /// and sets its own velocity; braking it here would be reaching into a
    /// speed this law does not own.
    pub settling: bool,
    /// `AbilitySet::variable_jump` — whether an early button release may shorten
    /// this body's jump arc. The `VelocityCut` law reads the same capability in
    /// `apply_jump_release`; `PhasedGravity` resolves its arc HERE, so without
    /// this the integrator would grant variable height to a body whose ability
    /// set denies it.
    pub can_variable_jump: bool,
}

/// THE INITIAL DASH's phase for this tick — start it, keep it, or answer that
/// there is none.
///
/// ⭐⭐ ONE EDGE DOES ALL OF IT: a steer direction that DIFFERS from last
/// tick's starts the phase. That single rule is the initial dash, the free
/// reversal that makes dash-dancing possible, and the foxtrot's re-tap, without
/// any of them being a separate mechanic. A HELD direction never re-triggers,
/// which is exactly what lets the phase expire and an ordinary run begin.
///
/// ⛔ GROUNDED ONLY, and it ends the moment a body leaves the floor: the phase
/// describes a decision about footing, and an airborne body has already made a
/// different one.
///
/// ⛔ `initial_dash_time <= 0.0` disables it completely — no state is written
/// and the caller gets `0.0` — so a world that declares nothing keeps ground
/// speed as one continuum.
fn resolve_initial_dash(
    state: &mut AxisManeuverState,
    on_ground: bool,
    input: InputState,
    dt: f32,
    tuning: AxisSweptParams,
) -> f32 {
    if tuning.locomotion.initial_dash_time <= 0.0 {
        return 0.0;
    }
    // ⭐⭐ TWO STICKS, TWO QUESTIONS. `dir` is what this body may ACT on — damped
    // to nothing by a rooted move — and `held` is what the PLAYER is holding.
    // The dash arms on `dir` so a rooted body cannot dash out of its own
    // recovery; the MEMORY is of `held`, because forbidding an action must not
    // erase the state that recognises the next input.
    //
    // ⛔⛔ IT USED TO REMEMBER THE DAMPED ONE. A player who simply held right
    // through an attack was recorded as neutral for its whole duration, so the
    // tick it ended read as "pressed right from nothing" — the exact edge that
    // arms a full-speed dash. A free dash out of every rooted move.
    //
    // ⛔ AND THEY ARE THE SAME VALUE WHENEVER NOTHING IS DAMPING, which is every
    // body outside a motion-scaled window.
    let steer = input.local_axis().x;
    let deadzoned = |x: f32| {
        if x.abs() > STEER_DEADZONE {
            x.signum()
        } else {
            0.0
        }
    };
    let dir = deadzoned(steer);
    let held = deadzoned(input.steer_axis().x);
    if !on_ground {
        state.initial_dash_timer = 0.0;
        state.initial_dash_dir = 0.0;
        state.prev_steer_dir = held;
        return 0.0;
    }
    // ⛔ A BODY MID-TURNAROUND IS NOT STARTING A DASH. It committed to a run and
    // is paying to face the other way; handing it a fresh full-speed dash would
    // refund exactly what the turnaround is charging.
    if state.turnaround_timer > 0.0 {
        state.initial_dash_timer = 0.0;
        state.initial_dash_dir = 0.0;
        state.prev_steer_dir = held;
        return 0.0;
    }
    // The EDGE is against what the player was holding; the PERMISSION is `dir`.
    if dir != 0.0 && held != state.prev_steer_dir {
        state.initial_dash_timer = tuning.locomotion.initial_dash_time;
        state.initial_dash_dir = dir;
    } else {
        state.initial_dash_timer = (state.initial_dash_timer - dt).max(0.0);
        if state.initial_dash_timer <= 0.0 {
            state.initial_dash_dir = 0.0;
        }
    }
    state.prev_steer_dir = held;
    // Letting go mid-dash ends it: the phase is a committed DIRECTION, and a
    // body with no direction held is not dashing anywhere.
    if dir == 0.0 {
        state.initial_dash_timer = 0.0;
        state.initial_dash_dir = 0.0;
    }
    state.initial_dash_dir
}

/// How far the stick must leave centre before it names a direction for the
/// initial dash. Shared with nothing: the phase is the only reader.
pub(super) const STEER_DEADZONE: f32 = 0.5;

impl NormalSpineCtx {
    /// The gating a bare actor (enemy/NPC) with no player ability components
    /// presents: it moves horizontally (run) and falls, nothing else.
    pub fn bare(on_ground: bool) -> Self {
        Self {
            on_ground,
            blink_grace: false,
            water: None,
            can_fast_fall: false,
            can_glide: false,
            can_move_horizontal: true,
            can_variable_jump: false,
            // A bare actor has no crouch: nothing puts one in `BodyMode::Crouching`.
            crouching: false,
            // and no dash phase: the caller that resolves one is the player
            // road, and a bare actor walks the continuum.
            initial_dash_dir: 0.0,
            // A bare actor is never refused horizontal control, so it never
            // reaches the settling branch.
            settling: false,
        }
    }
}

/// Normal-mode integration — the shared physics SPINE, actor-generic. Applies
/// gravity-direction-relative gravity, fast-fall, glide-gate, run/friction, and
/// the fall-speed cap to ANY body's `vel`, gated only by the small
/// [`NormalSpineCtx`]. Everything projects through the supplied `MotionFrame` so sideways /
/// flipped gravity Just Works. The player feeds it via `integrate_normal_clusters`;
/// enemies/NPCs feed it via [`NormalSpineCtx::bare`] (+ per-actor `tuning`).
pub fn integrate_normal_spine(
    kin_vel: &mut Vec2,
    fast_falling: &mut bool,
    gliding: &mut bool,
    carried_run: &mut f32,
    // How long the carry is still owed. Counted down here rather than by a
    // watcher on the reaction timers, because this is the one function that
    // already runs every tick with `dt` for every body — a release condition
    // living anywhere else would need the flight cluster plumbed to a second
    // place, and the two would be free to disagree about when a launch ends.
    carried_hold: &mut f32,
    phased_jump: &mut PhasedJumpState,
    ctx: NormalSpineCtx,
    input: InputState,
    dt: f32,
    frame: MotionFrame,
    tuning: AxisSweptParams,
) {
    let g = frame.down();
    // Fall-direction speed BEFORE this frame's gravity (terminal velocity is an
    // equilibrium gravity accelerates UP TO, not a brake on an over-cap fling).
    let fall_along_before = kin_vel.dot(g).max(0.0);
    let blink_hang_active = ctx.blink_grace && kin_vel.dot(g) >= 0.0;
    let water_gravity_scale = ctx.water.map(|c| c.spec.gravity_scale).unwrap_or(1.0);
    if !blink_hang_active {
        let down_speed = kin_vel.dot(g);
        let jump_gravity_scale = match tuning.locomotion.jump_law {
            AxisJumpLaw::VelocityCut => 1.0,
            AxisJumpLaw::PhasedGravity(params) => {
                if phased_jump.active && down_speed < 0.0 {
                    let upward_speed = -down_speed;
                    // A body whose ability set denies `variable_jump` commits to
                    // the whole arc: only the apex threshold ends the weak-ascent
                    // phase, never the button. This is the same capability gate
                    // `apply_jump_release` applies to the `VelocityCut` law —
                    // without it, selecting `PhasedGravity` would silently hand
                    // out variable jump height the grant list never granted.
                    let sustaining = !ctx.can_variable_jump
                        || (!phased_jump.hold_cancelled && input.jump_held());
                    if sustaining && upward_speed > params.held_phase_min_upward_speed {
                        params.held_rise_gravity_scale
                    } else {
                        // The weak-ascent phase is one-way. Once the button is
                        // released OR the jump slows into its apex regime, a
                        // later gravity-frame rotation or button re-press must
                        // not resurrect it.
                        phased_jump.cancel_hold();
                        params.released_rise_gravity_scale
                    }
                } else {
                    phased_jump.cancel_hold();
                    params.fall_gravity_scale
                }
            }
        };
        *kin_vel += frame.gravity_acceleration() * (water_gravity_scale * jump_gravity_scale) * dt;
        *kin_vel += frame.external_acceleration() * dt;
    }
    if input.fast_fall_pressed() && ctx.can_fast_fall && !ctx.on_ground {
        *fast_falling = true;
    }
    if *fast_falling && !blink_hang_active && ctx.water.is_none() {
        *kin_vel += tuning.locomotion.fast_fall_accel * g * dt;
    }
    *gliding = ctx.can_glide
        && !ctx.on_ground
        && !*fast_falling
        && !blink_hang_active
        && ctx.water.is_none()
        && input.jump_held()
        && kin_vel.dot(g) > 0.0;

    if ctx.can_move_horizontal {
        // Run/friction act along the PHYSICAL run axis (`side`, perpendicular to
        // gravity). The input-frame mode chooses how the stick projects onto it.
        let m = frame.side();
        let run = input.local_axis().x;
        let along = kin_vel.dot(m);
        // THE CARRY IS OWED FOR A WINDOW, then it is ordinary momentum again.
        //
        // While the hold lasts the floor holds its value: this is the launch a
        // body was given while it could not act, and bleeding it under the
        // hands-off stop assist is what made a clean smash move a fighter 15px
        // instead of 110 (queue F0e). When the window ends the floor bleeds at
        // `carried_decay` exactly as it always has — which is also what a portal
        // fling gets, since a fling sets no hold.
        // this whole block is inside `can_move_horizontal`, so the hold counts
        // TICKS THE HORIZONTAL LAW RAN, not wall time. That is the right clock
        // for it — the carry only means anything while this law is deciding the
        // run axis — and it is inert in the gap: a body that cannot move
        // horizontally is not reading `carried_run` either.
        if *carried_hold > 0.0 {
            *carried_hold = (*carried_hold - dt).max(0.0);
            if *carried_hold == 0.0 {
                // SURRENDERED. The floor was owed because the body could not
                // answer for the momentum; control is back, so it stops being
                // owed and the tight stop-on-release feel returns immediately.
                //
                // this must ZERO the floor, not hand it to `carried_decay`. That rate is `0.0` on
                // the axis-swept profile — the floor never bleeds — so "expire into the ordinary
                // decay" would mean a body that was hit once coasts at its launch speed for the
                // rest of its life.
                //
                // Zeroing the FLOOR is not braking: `kin_vel` is untouched, so
                // the launched body flies on and decelerates under the ordinary
                // air stop assist, which is exactly what should happen once it
                // can act.
                *carried_run = 0.0;
            }
        } else {
            // No hold — a portal fling, or a launch that has already expired.
            // Unchanged behaviour, including a `carried_decay` of zero meaning
            // "a fling is conserved until input, a wall or the ground eats it".
            *carried_run = approach(*carried_run, 0.0, tuning.locomotion.carried_decay * dt);
        }

        let new_along = match tuning.locomotion.horizontal_law {
            AxisHorizontalLaw::Responsive => {
                let accel = if ctx.on_ground {
                    tuning.locomotion.run_accel
                } else if *gliding {
                    tuning.locomotion.glide_air_accel
                } else {
                    tuning.locomotion.air_accel
                };
                if ctx.on_ground {
                    // ⭐⭐ A CROUCH COSTS YOU YOUR MOBILITY, which is what pays
                    // for the smaller hurtbox and the shortened launch
                    // (`crouch_cancel_scale`). Measured 2026-08-24: this law read
                    // `BodyMode` only for `Climbing`, so a crouching fighter ran
                    // at full speed and kept BOTH defensive benefits for nothing.
                    //
                    // ⛔ THE CAP, NOT THE ACCELERATION — the same distinction the
                    // walk/run gait rests on. Scaling `accel` would make a crouch
                    // slow to start and then just as fast, which is a delay
                    // rather than a stance.
                    let stance = if ctx.crouching {
                        tuning.locomotion.crouch_speed_frac.clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    // ⭐⭐ THE INITIAL DASH IS A SET, NOT AN APPROACH, and that
                    // is the whole difference between a phase and a ramp: a
                    // dash is AT speed on its first tick, which is what makes
                    // reversing out of one instant enough to be a mechanic.
                    // `approach` would give a body that leans into the turn.
                    //
                    // ⛔ The crouch stance still applies: crouching out of a
                    // dash plants you, exactly as crouching out of a run does.
                    let mut v = if ctx.initial_dash_dir != 0.0 {
                        let speed = if tuning.locomotion.initial_dash_speed > 0.0 {
                            tuning.locomotion.initial_dash_speed
                        } else {
                            tuning.locomotion.max_run_speed
                        };
                        let want = ctx.initial_dash_dir * speed * stance;
                        // ⛔⛔ A DASH MAY ONLY SPEED YOU UP. A body already
                        // travelling faster than the dash along this axis is
                        // carrying somebody else's velocity — a LAUNCH — and
                        // this must not touch it.
                        //
                        // ⭐ MEASURED, and it deleted knockback: a grounded
                        // fighter launched at 1313px/s while holding a
                        // direction came out at 270 (toward) or 0 (away), so
                        // nobody was ever knocked off the stage and a one-stock
                        // match never ended. Same class as the ground roll's
                        // shed — a maneuver reaching into a shared velocity it
                        // does not own — and the same asymmetry answers it: the
                        // doubtful case does nothing.
                        if along.abs() > want.abs() {
                            along
                        } else {
                            want
                        }
                    } else {
                        approach(
                            along,
                            run * stance * tuning.locomotion.max_run_speed,
                            accel * dt,
                        )
                    };
                    if run.abs() <= 0.1 {
                        v = approach(v, 0.0, tuning.locomotion.ground_friction * dt);
                    }
                    v
                } else if run > 0.1 {
                    // the AIR cap, which is the ground cap unless the body
                    // authored its own. Air ACCELERATION was already a separate
                    // number; air TOP SPEED was the ground's, which made a
                    // slow-running heavy that drifts fast unspellable.
                    approach(
                        along,
                        (run * tuning.locomotion.air_speed_cap()).max(along),
                        accel * dt,
                    )
                } else if run < -0.1 {
                    approach(
                        along,
                        (run * tuning.locomotion.air_speed_cap()).min(along),
                        accel * dt,
                    )
                } else {
                    approach(along, *carried_run, tuning.locomotion.air_stop_assist * dt)
                }
            }
            AxisHorizontalLaw::Momentum(params) => {
                let has_input = run.abs() > 0.1;
                if !has_input {
                    // Hands-off coasting decays toward the CARRIED floor, not to
                    // zero — the same portal-fling / knockback doctrine the
                    // responsive law's stop assist honors. A momentum profile
                    // that coasts (`air_coast_decel > 0`) must still not bleed
                    // away speed the WORLD imparted; `carried_run` is 0 for
                    // ordinary locomotion, so this is identity for a body that
                    // was never flung. Mary-O coasts at 0 in air, which makes it
                    // identity for her either way.
                    let (decel, floor) = if ctx.on_ground {
                        (params.ground_coast_decel, 0.0)
                    } else {
                        (params.air_coast_decel, *carried_run)
                    };
                    approach(along, floor, decel * dt)
                } else {
                    // Same rule on the momentum law: whichever cap this body is
                    // under right now, read through the one accessor.
                    let target = run
                        * if ctx.on_ground {
                            tuning.locomotion.max_run_speed
                        } else {
                            tuning.locomotion.air_speed_cap()
                        };
                    let opposing = along.abs() > 1.0e-4 && along.signum() != run.signum();
                    let reducing_same_direction = !opposing && along.abs() > target.abs();
                    let accel = if ctx.on_ground {
                        if opposing {
                            params.ground_reverse_accel
                        } else if reducing_same_direction {
                            params.ground_coast_decel
                        } else {
                            tuning.locomotion.run_accel
                        }
                    } else if opposing {
                        params.air_reverse_accel
                    } else if *gliding {
                        tuning.locomotion.glide_air_accel
                    } else {
                        tuning.locomotion.air_accel
                    };
                    // On the ground, a lowered target (notably releasing the
                    // run modifier while still holding a direction) coasts back
                    // toward that gait instead of preserving overspeed forever.
                    // In air, same-direction input preserves externally acquired
                    // overspeed, matching the kernel's portal/impulse doctrine.
                    let equilibrium = if ctx.on_ground {
                        target
                    } else if run > 0.0 {
                        target.max(along)
                    } else {
                        target.min(along)
                    };
                    approach(along, equilibrium, accel * dt)
                }
            }
        };
        *kin_vel += (new_along - along) * m;
        *carried_run = carried_run.clamp(new_along.min(0.0), new_along.max(0.0));
    } else if ctx.settling {
        // ⛔⛔ MAY NOT STEER IS NOT MAY NOT STOP, and the whole ground-speed
        // block — friction included — sits inside `can_move_horizontal`. So a
        // body that raised its guard mid-run kept its run: measured at 270px/s,
        // still 270 sixty ticks later, guard up the whole time. A shielding
        // fighter GLIDED across the stage.
        //
        // ⭐ THIS IS THE "RUN CANCEL INTO SHIELD" ROW: planting yourself is what
        // makes a raised guard a decision rather than a free slide.
        //
        // ⛔ THE CALLER DECIDES `settling`, because the exclusion that matters
        // is an EVADE: a roll is also shield-held and it SETS its own velocity,
        // so braking here would be this function reaching into a speed it does
        // not own — the same mistake the initial dash made with knockback.
        let m = frame.side();
        let along = kin_vel.dot(m);
        // ⛔⛔ A BRAKE MAY ONLY TAKE BACK SPEED THE BODY COULD HAVE WALKED UP
        // TO. Anything faster than its own run is somebody else's velocity — a
        // LAUNCH — and planting yourself must not delete knockback.
        //
        // ⭐ MEASURED, and by the guard written for the ground roll earlier the
        // same day: `a_ground_roll_ends_stopped_but_never_eats_a_launch` went
        // red the moment this branch existed, because a launched body holding
        // its guard is grounded, not evading, and therefore looked "planted".
        // Third time this shape has appeared — the roll's shed, the initial
        // dash, and now this — and the answer is the same each time: bound the
        // effect by what the maneuver itself put in.
        let owned = tuning.locomotion.max_run_speed;
        if along.abs() <= owned {
            let braked = approach(along, 0.0, tuning.locomotion.ground_friction * dt);
            *kin_vel += (braked - along) * m;
            *carried_run = carried_run.clamp(braked.min(0.0), braked.max(0.0));
        }
    }

    if let Some(contact) = ctx.water {
        let drag = contact.spec.drag.clamp(0.0, 1.0);
        *kin_vel *= 1.0 - drag;
        cap_fall_speed(kin_vel, g, contact.spec.max_fall_speed);
    } else {
        // `relax` = treat the cap as an equilibrium (never decelerate an over-cap
        // fling like a portal exit). GLIDING is an intentional brake, so it keeps a
        // hard clamp; terminal velocity + fast-fall do not.
        let (fall_cap, relax) = if *fast_falling {
            (tuning.locomotion.fast_fall_speed, true)
        } else if *gliding {
            (tuning.locomotion.glide_fall_speed, false)
        } else {
            (tuning.locomotion.max_fall_speed, true)
        };
        let effective_cap = if relax {
            fall_cap.max(fall_along_before)
        } else {
            fall_cap
        };
        cap_fall_speed(kin_vel, g, effective_cap);
    }
}

/// Travel under the stage: no gravity, no geometry, and the stick still steers.
///
/// ⭐⭐ SHE STEERS, WHICH IS THE WHOLE REASON THIS IS A MODE. Jon, 2026-08-27:
/// *"I do want the player to be able to control where they move."* A trapdoor
/// that dropped her at a computed destination would be a teleport with a longer
/// animation; what makes it a different KIND of mobility move is that the time
/// under the stage is time she is playing.
///
/// ⛔ HORIZONTAL ONLY. Under the stage there is no up and no down to mean
/// anything — the floor she left is the ceiling of where she is — so vertical
/// stick does nothing and vertical velocity is pinned to zero. A body that
/// drifted vertically would surface through a different floor than the one it
/// entered, or through none.
///
/// ⛔ AND THE VELOCITY IS SET, NOT ACCELERATED. She has no traction against
/// anything down there; carrying her run's momentum in would make the move's
/// reach depend on how she entered it, which is a thing the player cannot see
/// and would have to learn.
pub(super) fn integrate_submerged_clusters(
    world: &crate::world::World,
    kinematics: &mut crate::body_clusters::BodyKinematics,
    state: &mut AxisManeuverState,
    input: InputState,
    dt: f32,
    frame: MotionFrame,
    tuning: AxisSweptParams,
) {
    // Under the stage a dash is over: nothing down here to dash against, and a
    // timer still running would resume on the way out.
    interrupt_maneuvers_for_mode_transition(state);
    let local_stick = input.local_axis();
    let body_frame = frame.basis();
    let world_stick = body_frame.to_world(Vec2::new(local_stick.x, 0.0));
    // Her own top speed, so a fast character travels under the stage the way she
    // travels over it. `SUBMERGED_SPEED_FRAC` is the one number this mode adds.
    let speed = tuning.locomotion.max_run_speed * SUBMERGED_SPEED_FRAC;
    let step = world_stick * speed * dt;
    // ⛔⛔ AND THE STEP IS REFUSED IF IT LEAVES THE SURFACE. Written here rather
    // than in the sweep below because a submerged body is passable against
    // every block in the world, so the sweep has nothing to stop it with.
    let step = if stays_over_its_surface(world, kinematics, step, frame.down()) {
        step
    } else {
        Vec2::ZERO
    };
    // ⛔⛔ VELOCITY ONLY — THIS MODE IS NOT A POSITION AUTHORITY. It wrote
    // `pos += step` as well, and the shared sweep below then advanced by the
    // very velocity written here, so every submerged tick moved TWICE: measured
    // 10.8px against an authored 5.4 at 60Hz, exactly 2.00x.
    //
    // ⛔ AND THE DOUBLE STEP DEFEATED THE LEDGE RULE ABOVE, which is the part
    // that is not merely a speed bug. `stays_over_its_surface` validates ONE
    // prospective step; the sweep then added a second, equal step that nothing
    // asked about, so a body could be approved for a supported step and land a
    // step further on, past the lip the rule exists to refuse.
    //
    // ⭐ THE VELOCITY IS ZEROED WITH THE STEP, not merely left unapplied: it is
    // what the swept sample publishes and what survives into the tick she
    // surfaces on, and a body that refused to move while still reporting
    // 245 px/s would come out of the boards sprinting.
    kinematics.vel = step / dt.max(f32::EPSILON);
}

/// End every ground maneuver an EXCLUSIVE mode takes the body away from.
///
/// ⛔⛔ CLEARING `dash_timer` ALONE IS NOT ENOUGH, and the half that was missed is
/// the one nothing ticks down. `initial_dash_timer` is decayed by NORMAL
/// movement — the mode branch that an exclusive mode replaces — so a body that
/// entered the wire or the trapdoor mid-dash carried the window FROZEN through
/// the whole mode and resumed it on the way out. Measured: armed at 0.2333s,
/// still exactly 0.2333s after 30 submerged ticks holding nothing, when 14
/// frames of normal movement would have spent it.
///
/// ⭐ ONE AUTHORITY, so the next exclusive mode inherits the list instead of
/// rediscovering which timers it owed.
fn interrupt_maneuvers_for_mode_transition(state: &mut AxisManeuverState) {
    state.dash_timer = 0.0;
    state.initial_dash_timer = 0.0;
    state.initial_dash_dir = 0.0;
}

/// Refuse a submerged step that would leave the surface the body is under.
///
/// ⭐⭐ THIS IS WHAT MAKES THE TRAPDOOR A DOOR. Jon, 2026-08-28: it *"can only
/// move along a ground surface (i.e. it can't go over a ledge)."* Travel under
/// the stage is travel along a specific piece of stage; walking off the end of
/// it and coming up in open air is a teleport with extra steps.
///
/// ⛔ REFUSED WHOLE, NOT CLAMPED TO THE EDGE. A tick of submerged travel is
/// about four world px, so stopping one tick short of the lip is invisible —
/// and a bisection to find the exact edge would be a solver in the middle of a
/// sweep, for a difference nobody can see.
///
/// ⛔ SOLID ONLY. A one-way platform is not a surface you can be UNDER: the
/// whole stage's drop-through geometry would otherwise read as ground and let
/// her travel the sky.
fn stays_over_its_surface(
    world: &crate::world::World,
    kinematics: &crate::body_clusters::BodyKinematics,
    step: Vec2,
    gravity_dir: Vec2,
) -> bool {
    if step == Vec2::ZERO {
        return true;
    }
    use crate::geometry::AabbExt as _;
    let Some(dir) = step.try_normalize() else {
        return true;
    };
    let body = kinematics.aabb_oriented(gravity_dir);
    let half = body.half_size();
    let gravity_half = half.x * gravity_dir.x.abs() + half.y * gravity_dir.y.abs();
    let travel_half = half.x * dir.x.abs() + half.y * dir.y.abs();
    // ⛔⛔ THE LEADING FOOT, NOT THE WHOLE FOOTPRINT. A probe the body's own
    // width asks *"is ANY of me still over ground"*, and the answer is yes until
    // the door has walked entirely off the lip — she stopped a body-width past
    // the edge, hanging over open air, which is the thing this rule exists to
    // refuse. The leading corner is what must still be supported.
    let feet = body.center() + gravity_dir * gravity_half;
    let lead = feet + step + dir * travel_half;
    let probe_half = dir.abs() * SURFACE_LEAD_PROBE
        + gravity_dir.abs() * (SUBMERGED_GROUND_PROBE * 0.5);
    let probe_center =
        lead - dir * SURFACE_LEAD_PROBE + gravity_dir * (SUBMERGED_GROUND_PROBE * 0.5);
    world.body_overlaps_any(crate::geometry::Aabb::new(probe_center, probe_half), |block| {
        matches!(block.kind, crate::world::BlockKind::Solid)
    })
}

/// Half-width of the leading-edge ground probe. Small, because it is asking
/// about a corner rather than about a footprint.
const SURFACE_LEAD_PROBE: f32 = 1.0;

/// How deep under a submerged body the kernel looks for the surface it is
/// travelling along. Two body-widths would let her round a corner she cannot
/// see; a single pixel would drop her on a seam between two blocks.
const SUBMERGED_GROUND_PROBE: f32 = 8.0;

/// How fast a submerged body travels, as a fraction of its own run speed.
///
/// ⛔ NOT A CONSTANT SPEED IN PIXELS. A shared number would make the trapdoor a
/// different move for a fast character than for a slow one, and this mode is
/// meant to be reusable — the mole, the burrower and the diver all want "like
/// running, but under things".
///
/// ⭐⭐ FASTER THAN RUNNING, WHICH IS A BALANCE POSITION AND NOT A PHYSICAL
/// CLAIM. Jon, 2026-08-27: *"1.2x run speed. I'm biasing towards making moves
/// too powerful to start."* Nothing about being under a stage makes a body
/// quicker; the number is here to make the trip worth taking while the move is
/// being judged, and it is the first knob to turn when it turns out to be too
/// good. The earlier 0.82 was the opposite instinct and was mine, not authored.
pub(super) const SUBMERGED_SPEED_FRAC: f32 = 1.2;

/// How hard the swing is damped, per second of angular velocity.
///
/// ⛔ A PENDULUM WITHOUT IT NEVER SETTLES, and the player's stick is an
/// acceleration: holding one direction for the whole lift would wind the swing
/// up past the cap and slam it against the stop every tick. Damping makes a
/// held direction converge on an ANGLE instead of oscillating at one.
const WIRE_SWING_DAMPING: f32 = 2.2;

/// The shortest the winch may pull the rope, in world px.
///
/// ⛔ NOT ZERO. At zero length the pendulum has no angle — every direction is
/// the same point — and the tangent it releases along is undefined. Stopping
/// the reel short of the pulley keeps the swing meaningful right up to the cut.
const WIRE_MIN_LENGTH: f32 = 40.0;

/// How much of the lift the winch reels at its full rate before easing off.
///
/// ⛔ THE EASE IS A TAIL, NOT THE WHOLE CLIMB. Ramping across the entire lift
/// forces the opening rate up to `2·rise/T − release_rise` to still cover the
/// authored distance, and `wire_probe` measured what that looks like: 1437 px/s
/// and a 23px first tick. Easing over the last third instead needs only 882 px/s
/// of cruise for the same 420px, and still arrives at `release_rise` exactly.
pub const WIRE_CRUISE_FRAC: f32 = 0.66;

/// THE WINCH RATE THAT TRAVELS `rise` IN `lift_s` AND ARRIVES AT `release_rise`.
///
/// ⛔⛔ IT LIVES BESIDE THE PROFILE IT INVERTS, and that is the whole point of
/// it being a function. The rate was solved in the AUTHORING executor while the
/// profile was integrated here, so the two were free to disagree — and the
/// moment `WIRE_CRUISE_FRAC` was introduced they did, silently, in the direction
/// that undershoots the authored rise. Two authorities for one fact means one of
/// them is deleted; this is the fact, and there is one of it.
///
/// The winch holds `v0` for `WIRE_CRUISE_FRAC` of the lift and eases QUADRATICALLY
/// to `release_rise` over the rest. The mean of `e²` over the tail is `1/3`, so
/// the distance covered is `v0·c·T + [v1 + (v0−v1)/3]·(1−c)·T`. Solved for `v0`.
///
/// ⛔ THE `1/3` IS THE QUADRATIC'S, NOT A LINEAR RAMP'S `1/2`, and getting it
/// wrong does not fail loudly — it just undershoots the authored rise by a few
/// per cent, which reads as "the up-B feels a bit short".
///
/// ⚠ CLAMPED AT `release_rise`, WHICH IS THE DEGENERATE AUTHORING: a move asking
/// to leave the wire faster than its own average climb would need the winch to
/// ACCELERATE into the cut. A flat rope is better than a rope that speeds up at
/// the top, and the content test names the condition.
pub fn winch_rate_for(rise: f32, lift_s: f32, release_rise: f32) -> f32 {
    if lift_s <= 0.0 {
        return 0.0;
    }
    let tail = 1.0 - WIRE_CRUISE_FRAC;
    let numerator = rise / lift_s - release_rise * tail * (2.0 / 3.0);
    (numerator / (WIRE_CRUISE_FRAC + tail / 3.0)).max(release_rise)
}

/// Ceiling on the speed the wire may drag the body at, in px/s.
///
/// ⛔⛔ THE WIRE STEERS BY CORRECTION, so a body the sweep has STOPPED — under a
/// platform, against a wall — falls further behind its rope target every tick
/// while the winch keeps reeling. Uncapped, that correction grows without bound
/// and fires her across the stage on the frame the obstruction clears. The cap
/// is what makes "the wire wins" a bounded claim.
const WIRE_MAX_TRACK_SPEED: f32 = 1600.0;

/// ONE TICK OF A BODY ON A WIRE: swing the pendulum, reel the winch in, and
/// steer her toward where the rope now says she is.
///
/// ⭐⭐ HER POSITION IS `(anchor, length, angle)` AND NOTHING ELSE. That is what
/// makes this a different integration rather than a shove with a nice comment:
/// gravity does not act on her, her velocity is not integrated, and the stick
/// buys ANGULAR acceleration instead of horizontal speed. Jon, 2026-08-29:
/// *"she doesn't teleport up, she gets lifted up by the wire… while she is being
/// lifted by the wire her motion controls should let her swing like a pendulum
/// so she has a bit of horizontal recovery with it too."*
///
/// ⛔⛔ AND IT PUBLISHES A VELOCITY RATHER THAN WRITING THE POSITION. The
/// trapdoor's submerged step sets `pos` directly because a submerged body is
/// passable and the sweep has nothing to stop it with. A body on a wire is a
/// NORMAL body — drawn, hittable, and solid against the stage — so the sweep
/// stays the one authority for where she ends up, and the rope only says where
/// she is being pulled. That is also what makes the underside of a platform
/// stop her: a recovery has to get around the lip, and swinging is how.
///
/// ⛔ THE RELEASE IS HERE TOO, and it is the ONE write of her exit velocity —
/// the swing's tangential speed plus the authored carry. `LEAP_OUT_SPEED` was
/// dead content for a month because the trapdoor had two writers for that fact;
/// the wire has one, and it is this branch.
pub(super) fn integrate_wire_clusters(
    kinematics: &mut crate::body_clusters::BodyKinematics,
    state: &mut AxisManeuverState,
    input: InputState,
    dt: f32,
    frame: MotionFrame,
) {
    let Some(mut wire) = state.wire else {
        return;
    };
    if dt <= 0.0 {
        return;
    }
    // The rope owns the body for the same reason the trapdoor does, so the same
    // maneuvers end.
    interrupt_maneuvers_for_mode_transition(state);
    // THE CLOCK FIRST, so the release tick is unambiguous: the wire either has
    // time left and lifts, or it is out of time and lets go. A tick that did
    // both would write two velocities.
    wire.lift_remaining_s -= dt;
    if wire.lift_remaining_s <= 0.0 {
        // ⭐⭐ THE RELEASE. The tangent at `angle` is the direction a body on a
        // rope is actually travelling — `d/dθ (sin θ, cos θ)` in the frame's own
        // basis — and `ang_vel * length` is how fast along it. That product IS
        // the horizontal recovery the swing bought; nothing else adds to it.
        let tangent = frame.to_world(Vec2::new(wire.angle.cos(), -wire.angle.sin()));
        let swing = tangent * (wire.ang_vel * wire.length);
        kinematics.vel = swing - frame.down() * wire.release_rise;
        state.wire = None;
        return;
    }

    // ── THE SWING ────────────────────────────────────────────────────────────
    // A real pendulum's restoring term, so the period comes from the stage's own
    // gravity and the rope's length rather than from a number somebody picked.
    // A shortening rope therefore swings FASTER as she rises, which is the thing
    // a winch actually does to a body on a wire.
    let gravity = frame.gravity_acceleration().length();
    let restoring = -(gravity / wire.length.max(WIRE_MIN_LENGTH)) * wire.angle.sin();
    // ⛔ THE DAMPED AXIS, the same one `integrate_submerged_clusters` reads. A
    // move that ROOTS her scales this to zero, which is why the flyline authors
    // a GAP in its windows over the lift — see `the_flyline`. Reading the
    // undamped stick here instead would make the wire the one maneuver a root
    // cannot hold, and hide the authoring bug rather than state it.
    let stick = input.local_axis().x;
    // ⛔⛔ THE STOP IS SOFT, AND A HARD ONE MADE THE HANDOVER A COIN FLIP. Clamping
    // the angle and zeroing `ang_vel` at the cap means a held stick leaves the
    // wire at either its full tangential speed or at nothing, depending on
    // whether she happened to clip the stop in the last tick or two: the kernel
    // measured +229 px/s and `wire_probe` measured 0 for the SAME authored
    // numbers. That is not a tunable feel, it is a cliff, and a test that
    // asserted the lucky side of it was asserting a coin flip.
    //
    // ⇒ the stick's authority FADES toward the stop instead. She asymptotes to
    // the angle where the drive balances gravity's restoring pull, arrives with
    // `ang_vel` already near zero, and a HELD stick therefore hands over almost
    // nothing — predictably. The expressive half is unharmed: a REVERSAL is
    // driving inward, is not faded at all, and still crosses the arc under real
    // angular speed.
    let reach = (wire.angle / wire.max_angle.max(f32::EPSILON)).clamp(-1.0, 1.0);
    let drive = stick * wire.swing_accel;
    let drive = if drive * reach > 0.0 {
        drive * (1.0 - reach * reach)
    } else {
        drive
    };
    let damping = -WIRE_SWING_DAMPING * wire.ang_vel;
    wire.ang_vel += (restoring + drive + damping) * dt;
    wire.angle += wire.ang_vel * dt;
    // The rope's hard stop — a BACKSTOP now rather than the mechanism. The fading
    // drive above is what actually holds the swing inside its cap; this catches a
    // body that arrived with enough angular speed to overshoot anyway (a
    // reversal at full tilt), and it should almost never fire.
    //
    // ⛔ The velocity is clamped toward the stop only — a swing arriving at the
    // cap stops, a swing leaving it is free — because zeroing it outright would
    // make the wire eat a reversal the player already asked for.
    if wire.angle > wire.max_angle {
        wire.angle = wire.max_angle;
        wire.ang_vel = wire.ang_vel.min(0.0);
    } else if wire.angle < -wire.max_angle {
        wire.angle = -wire.max_angle;
        wire.ang_vel = wire.ang_vel.max(0.0);
    }

    // ── THE WINCH ────────────────────────────────────────────────────────────
    // Shortening the rope IS the rise. Every pixel of lift in this move is a
    // pixel taken off `length`.
    //
    // ⭐⭐ AND IT SLOWS INTO THE RELEASE. The rate ramps linearly from
    // `winch_speed` at the catch to `release_rise` at the cut, so the speed the
    // rope is pulling her at on the last tick IS the speed she leaves with and
    // the handover has no step in it. A constant rate rose her at 764 px/s and
    // then handed her back doing 90 — a hard stop at the apex, which is the
    // teleport's own feel arriving through a different mechanic.
    //
    // ⛔ THE AUTHORED `rise` IS STILL EXACT: the area under a linear ramp from
    // `v0` to `v1` over `T` is `T·(v0+v1)/2`, and `catch_the_wire`'s caller
    // solves that for `v0`. Slowing down does not cost her any of the climb.
    //
    // ⛔⛔ AND IT EASES OUT AT THE TOP RATHER THAN RAMPING THE WHOLE WAY. A ramp
    // across the whole lift has to START at `2·rise/T − v1` to still travel the
    // authored distance — 1437 px/s here, which the probe caught as a 23px first
    // tick: a YANK, not a lift, and clause one is about how she travels. Holding
    // the rate flat for `WIRE_CRUISE_FRAC` and easing only over the tail buys the
    // smooth handover for a cruise of 882 px/s and a 15px tick.
    let through = if wire.lift_total_s > 0.0 {
        (wire.lift_remaining_s / wire.lift_total_s).clamp(0.0, 1.0)
    } else {
        0.0
    };
    // `through` counts DOWN from 1 at the catch to 0 at the cut, so the ease is
    // the last `1 - WIRE_CRUISE_FRAC` of it.
    let eased = (through / (1.0 - WIRE_CRUISE_FRAC)).min(1.0);
    // ⛔⛔ SQUARED, SO THE APPROACH FLATTENS. A LINEAR ease still lands with a
    // slope: the last integrated tick sits one `dt` short of the cut, so it was
    // reeling at 162 px/s when the rope let go at 90 — a 45% step, which is what
    // `the_lift_decelerates_into_the_release_instead_of_stopping_dead` measured.
    // A quadratic has zero derivative at the bottom, so the final ticks are
    // already travelling at `release_rise` and the handover is invisible.
    let reel = wire.release_rise + (wire.winch_speed - wire.release_rise) * eased * eased;
    wire.length = (wire.length - reel * dt).max(WIRE_MIN_LENGTH);

    // ── WHERE THE ROPE SAYS SHE IS ───────────────────────────────────────────
    // Local `+y` is toward the feet, so `(sin θ, cos θ)` hangs DOWN from the
    // anchor at rest and swings toward local `+x` for a positive angle.
    let hang = Vec2::new(wire.angle.sin(), wire.angle.cos()) * wire.length;
    let target = wire.anchor + frame.to_world(hang);
    let correction = (target - kinematics.pos) / dt;
    kinematics.vel = if correction.length() > WIRE_MAX_TRACK_SPEED {
        correction.normalize_or_zero() * WIRE_MAX_TRACK_SPEED
    } else {
        correction
    };
    state.wire = Some(wire);
}

pub(super) fn integrate_climb_clusters(
    kinematics: &mut crate::body_clusters::BodyKinematics,
    env_contact: &crate::body_clusters::BodyEnvironmentContact,
    state: &mut AxisManeuverState,
    jump: &mut crate::body_clusters::BodyJumpState,
    input: InputState,
    dt: f32,
    frame: MotionFrame,
    tuning: AxisSweptParams,
) {
    let Some(contact) = env_contact.climbable else {
        kinematics.vel = Vec2::ZERO;
        jump.ladder_jump_boost = 0.0;
        return;
    };
    let spec = contact.spec;
    // Resolve raw input into the controlled body's local frame, then project that
    // local intent through the body frame onto the climbable's authored world
    // axes. Today's climbable regions are vertical world-space spans with a
    // small horizontal strafe lane; when climbables grow an explicit authored
    // axis, this projection is the seam that should consume it.
    let local_stick = input.local_axis();
    let body_frame = frame.basis();
    let world_stick = body_frame.to_world(local_stick);
    let pressing_away_from_gravity = local_stick.y < -0.1;
    let mut target_vel = Vec2::new(
        world_stick.x * spec.climb_speed * spec.strafe_factor,
        world_stick.y * spec.climb_speed,
    );
    if jump.ladder_jump_boost > 0.0 && pressing_away_from_gravity {
        let away_from_feet = -tuning.locomotion.jump_speed;
        let along_down = target_vel.dot(body_frame.down);
        target_vel += body_frame.down * (away_from_feet - along_down);
    }
    kinematics.vel = target_vel;
    state.fast_falling = false;
    state.gliding = false;
    state.wall_clinging = false;
    state.wall_climbing = false;
    let _ = dt;
}

/// Free-flight integration: accelerate toward stick input, idle-hover
/// bob phase when sticks are centered, hard clamp to the flight
/// terminal speed. Clears fast-fall + wall-cling flags by mode.
pub(super) fn integrate_flight_clusters(
    kinematics: &mut crate::body_clusters::BodyKinematics,
    state: &mut AxisManeuverState,
    input: InputState,
    dt: f32,
    frame: MotionFrame,
    tuning: AxisSweptParams,
) {
    state.fast_falling = false;
    state.wall_clinging = false;
    state.wall_climbing = false;
    state.flight_phase += dt * tuning.flight.hover_hz * std::f32::consts::TAU;

    // Free flight consumes controlled-body-local input. Resolve raw input before
    // it reaches `InputState`; this layer only projects local side/down motion
    // into world space.
    let basis = frame.basis();
    let vel_run = kinematics.vel.dot(basis.side);
    let vel_descend = kinematics.vel.dot(basis.down);
    let local_stick = input.local_axis();

    // The ONE coordinate-speed bound this limb enforces, read by every branch below
    // and by both clamps. It is the authored terminal, held strictly below `c` when
    // an invariant speed is authored — so "an invariant speed is never reached" is a
    // postcondition of the limb, not a property of the proper-velocity branch that
    // happens to compute it on the way past.
    let terminal = tuning.flight.coordinate_speed_cap();

    // The TARGET keeps the raw authored terminal: it is the scale a caller
    // normalised its command by (`velocity_target / flight_terminal_speed` in the
    // actor integrator), so lowering it here would slow every *subluminal* command
    // too. Superluminal requests are answered by the clamps, which cost nothing a
    // representable command can feel.
    let target_run = local_stick.x * tuning.flight.terminal_speed;
    let mut target_descend = local_stick.y * tuning.flight.terminal_speed;
    if !tuning.flight.direct_velocity && local_stick.y.abs() <= 0.10 {
        target_descend = state.flight_phase.sin() * tuning.flight.hover_speed;
    }

    let (mut new_run, mut new_descend) = if tuning.flight.direct_velocity {
        // Direct-velocity free-mover: the controller commanded an exact velocity
        // (`stick × flight_terminal_speed` == its `velocity_target`), so take it
        // verbatim — no accel ramp, drag, hover-bob, or deadzone. Byte-identical to
        // a SNAP float (`step_floating_body`, `accel: None`) so a boss flies through
        // the ONE pipeline without a motion change. "Verbatim" still means "as
        // verbatim as `terminal` allows": this arm does NOT get to opt out of the
        // limb's speed bound, which is the whole reason the clamps below read the
        // hoisted `terminal` rather than the authored one.
        (target_run, target_descend)
    } else if let Some(invariant_speed) = tuning.flight.invariant_speed {
        // Relativistic free flight still runs through the ONE shared flight limb.
        // Acceleration and drag act on spatial proper velocity w = gamma*v; the
        // conversion back to coordinate velocity keeps |v| < c on its own. The
        // authored terminal remains a coordinate-speed cap and therefore a
        // game-feel knob — `terminal` above is that knob, already c-bounded.
        let c = invariant_speed.abs().max(f32::EPSILON);
        let current_v = crate::Vec2::new(vel_run, vel_descend);
        let current_speed_squared = current_v.length_squared().min(c * c * (1.0 - 1.0e-6));
        let current_gamma = 1.0 / (1.0 - current_speed_squared / (c * c)).sqrt();
        let mut proper_velocity = current_v * current_gamma;

        let target_v = if local_stick.length_squared() > 1.0 {
            local_stick.normalize() * terminal
        } else {
            local_stick * terminal
        };
        let target_speed_squared = target_v.length_squared().min(c * c * (1.0 - 1.0e-6));
        let target_gamma = 1.0 / (1.0 - target_speed_squared / (c * c)).sqrt();
        let target_w = target_v * target_gamma;
        let delta = target_w - proper_velocity;
        let max_step = tuning.flight.accel.max(0.0) * dt;
        if delta.length_squared() > max_step * max_step && max_step > 0.0 {
            proper_velocity += delta.normalize() * max_step;
        } else {
            proper_velocity = target_w;
        }
        if local_stick.length_squared() <= 0.01 {
            let speed = proper_velocity.length();
            let reduced = (speed - tuning.flight.drag.max(0.0) * dt).max(0.0);
            proper_velocity = proper_velocity.normalize_or_zero() * reduced;
        }
        let coordinate_velocity =
            proper_velocity / (1.0 + proper_velocity.length_squared() / (c * c)).sqrt();
        (coordinate_velocity.x, coordinate_velocity.y)
    } else {
        let mut new_run = approach(vel_run, target_run, tuning.flight.accel * dt);
        let mut new_descend = approach(vel_descend, target_descend, tuning.flight.accel * dt);

        if local_stick.x.abs() <= 0.10 {
            new_run = approach(new_run, 0.0, tuning.flight.drag * dt);
        }
        if local_stick.y.abs() <= 0.10 {
            new_descend = approach(new_descend, target_descend, tuning.flight.drag * dt);
        }
        (new_run, new_descend)
    };

    new_run = new_run.clamp(-terminal, terminal);
    new_descend = new_descend.clamp(-terminal, terminal);

    let mut local_velocity = crate::Vec2::new(new_run, new_descend);
    if tuning.flight.invariant_speed.is_some() {
        // Relativistic flight authors one radial coordinate-speed cap. Keeping
        // this opt-in preserves the established per-axis behavior of ordinary
        // flight bodies while preventing a diagonal command from exceeding the
        // experiment's terminal or invariant speed.
        let terminal_squared = terminal * terminal;
        if local_velocity.length_squared() > terminal_squared && terminal_squared > 0.0 {
            local_velocity = local_velocity.normalize() * terminal;
        }
    }
    kinematics.vel = frame.to_world(local_velocity);
}

/// Wall ability ride: while local side input presses into a wall, engage
/// wall-cling (clamp descent along the controlled body's down axis) or, with
/// `wall_climb` + local up/down input, drive motion along that down axis.
/// Records the first transition op so the trace recorder fires
/// `WallCling` / `WallClimb` exactly once per engagement.
pub(super) fn apply_wall_abilities_clusters(
    kinematics: &mut crate::body_clusters::BodyKinematics,
    ground: &crate::body_clusters::BodyGroundState,
    wall: &crate::body_clusters::BodyWallState,
    state: &mut AxisManeuverState,
    abilities: &crate::body_clusters::BodyAbilities,
    combo_trace: &mut crate::body_clusters::BodyComboTrace,
    input: InputState,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    was_clinging: bool,
    events: &mut FrameEvents,
) {
    if !wall.on_wall || ground.on_ground || !abilities.abilities.wall_cling {
        return;
    }
    let basis = frame.basis();
    let local_stick = input.local_axis();
    let pressing_into_wall =
        local_stick.x.abs() > 0.1 && local_stick.x.signum() == -wall.wall_normal_x;
    if !pressing_into_wall {
        return;
    }
    state.wall_clinging = true;
    if abilities.abilities.wall_climb && local_stick.y.abs() > 0.25 {
        state.wall_climbing = true;
        let along_down = kinematics.vel.dot(basis.down);
        kinematics.vel +=
            basis.down * (local_stick.y * tuning.locomotion.wall_climb_speed - along_down);
        if !was_clinging {
            events.op_clusters(combo_trace, MovementOp::WallClimb);
        }
    } else {
        let descend = kinematics.vel.dot(basis.down);
        if descend > tuning.locomotion.wall_slide_speed {
            kinematics.vel -= basis.down * (descend - tuning.locomotion.wall_slide_speed);
        }
        if !was_clinging {
            events.op_clusters(combo_trace, MovementOp::WallCling);
        }
    }
}
