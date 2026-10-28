//! Movement integration and retained step facts for home bodies.

use bevy::prelude::*;
use ambition_platformer2d_core as ae;
use ambition_characters::actor::BodyCombat;
use ambition_combat::attack_support::engine_input_from_actor_control;
use ambition_combat::feel::Platformer2dFeelTuningMonolith;

/// Retained movement facts for downstream body consumers.
/// Integration replaces this output each step. Home bodies require it so
/// participant policy and presentation can read the same events.
#[derive(Component, Default)]
pub struct BodyFrameOutput {
    /// Events from the current movement step.
    pub events: ae::FrameEvents,
    /// A reset requested by the movement kernel. Policy applies its consequence.
    pub reset: Option<BodyReset>,
}

/// A movement reset cause and the body's position when it occurred.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyReset {
    /// The kernel's reset cause.
    pub cause: ae::ResetCause,
    /// Position before reset policy changes the body.
    pub origin: ae::Vec2,
}

/// Integrate a player-owned body through the shared body kernel.
///
/// Human control may use wall-clock `control_dt` while simulation uses `sim_dt`;
/// this is an input affordance, not a separate movement model. Reset causes from
/// the kernel are interpreted here by participant policy: out-of-play bodies are
/// left alone, and hazard resets respect the body's vulnerability state.
#[allow(clippy::too_many_arguments)]
pub(super) fn integrate_home_body(
    actor_control: ambition_characters::actor::control::ActorControlFrame,
    world: &ae::World,
    clusters: &mut ae::BodyClustersMut<'_>,
    combat: &mut BodyCombat,
    invulnerable: ambition_characters::actor::Invulnerability,
    evading: bool,
    // Is this body TUMBLING? The published projection
    // (`BodyMotionFacts::tumbling`), read by the caller for the same reason
    // `evading` beside it is. The post-hit gate needs it so a falling body's
    // tech press is not deleted before the kernel can read it.
    tumbling: bool,
    // Whether participant rules have removed this body from play.
    out_of_play: bool,
    // Something else has this body's pose (`ae::PoseOwnedExternally`): a
    // scripted transit, a saddle, a grab. The step still runs, and it gives
    // the body no landing of its own. The same fact the actor road reads.
    pose_owned_externally: bool,
    hurtbox: &mut ae::CenteredAabb,
    frame_out: &mut BodyFrameOutput,
    motion_model: &mut ambition_platformer2d_core::movement::MotionModel,
    motion_frame: ae::MotionFrame,
    axis_tuning: ae::MovementTuning,
    feel: Platformer2dFeelTuningMonolith,
    // The live move's authored motion lock (`MovePlayback::motion_scale_now`;
    // `1.0` with no move playing), applied to this body's steering INTENT — the
    // same rule, through the same helper, that the actor integrator applies.
    // ⛔⛔ this parameter did not exist, and its absence is what let a human
    // fighter walk while charging a smash long after the rule was written.
    move_motion_scale: f32,
    frame_dt: f32,
    scaled_dt: f32,
    // The move PLAYING on this body, if any — the last term of the helpless
    // derivation, and threaded for the reason `move_motion_scale` above is: the
    // playback lives on the entity and this function takes clusters. ⛔ the VALUE,
    // not a bool: only a RECOVERY postpones helplessness, and a bool cannot say
    // which move it was.
    playing_a_move: Option<&ambition_combat::moveset::MovePlayback>,
    // `BodyContactField::NONE` for a body whose composition never granted the capability, which
    // is every body in Ambition today.
    contact_field: ae::BodyContactField<'_>,
) -> Option<ae::Vec2> {
    let actor_control = actor_control.damped_by_move_motion(move_motion_scale);
    let input = engine_input_from_actor_control(
        actor_control,
        feel,
        combat,
        clusters.shield,
        frame_dt,
        tumbling,
        // HELPLESS, from the ONE rule `trigger_moveset_moves` also asks — so
        // this body cannot be helpless to the movement kernel and not to the
        // move-start authority, which is exactly what it was.
        ambition_combat::moveset::body_is_helpless(
            clusters.jump,
            clusters.ground.on_ground,
            playing_a_move,
        ),
    );
    // Ledge/platform carry is handled inside the shared simulation kernel.
    let result = ambition_characters::actor::step_body(
        motion_model,
        clusters,
        combat,
        axis_tuning,
        out_of_play,
        // A home body can be held: Mary-O in a pipe is (Jon, 2026-10-11:
        // a landing sound and dust on each tick of the ride). This was a
        // literal `false`, so the marker did nothing on the body a human
        // drives.
        pose_owned_externally,
        ae::MotionStepContext {
            world,
            input,
            frame: motion_frame,
            facing_intent: actor_control.facing,
            dt: scaled_dt,
            contact: contact_field,
            pose_owned_externally,
            // See the actor road's note: the same playback, asked whether this
            // body is mid-recovery so the grounded refresh does not refund it.
            recovery_commitment_outstanding:
                ambition_combat::moveset::recovery_commitment_outstanding(playing_a_move),
        },
    );

    // `LeftTheWorld` and `Drowned` are NOT exempted, deliberately.
    // `resolve_body_hit` already states the rule for damage — *"you cannot be
    // invulnerable to the edge of the world"* — and this seam has to agree or
    // the two disagree about the same body. Falling out is not something that
    // HIT you, and neither is running out of air. `Requested` is the reset verb
    // and is always honoured.
    let reset = result.events.reset.and_then(|cause| {
        let untouched = cause == ae::ResetCause::Hazard
            && !ambition_combat::util::body_vulnerable(
                invulnerable,
                evading,
                clusters.shield,
                combat,
            );
        // AND A BODY THAT IS ALREADY OUT OF PLAY IS NOT KILLED AGAIN (ADR 0033). The gate above
        // is a POSITION TEST and re-fires every tick a body is past the margin — the ACTOR path has
        // always known this and writes `em.health.alive() && …` for exactly this reason, and this
        // path never got the same guard. That is not correct."*
        //
        // she goes on FALLING, which is the classic behaviour and now costs nothing: the body is
        // simply no longer teleported out from under its own death.
        (!untouched && !out_of_play).then(|| BodyReset {
            cause,
            origin: clusters.kinematics.pos,
        })
    });
    // Until then she keeps falling, which is what a classic platformer death looks like and now
    // costs nothing to get.

    *frame_out = BodyFrameOutput {
        reset,
        events: result.events,
    };

    // A home body's collision box IS its footprint, so it passes no envelope.
    ambition_combat::body_geometry::publish_body_footprint(
        hurtbox,
        clusters.kinematics.pos,
        clusters.kinematics.size,
        clusters.kinematics.facing,
        -result.surface_normal,
    );

    // The ridden-surface presentation fact: a momentum rider plants its feet on
    // the ground under it, so the caller publishes this tick's outward support
    // normal as the body's visual up (`SurfaceUpright`). Axis bodies stay
    // gravity-upright — a wall slide is not a stance change — and crawler
    // enemies publish their own surface pose through the feature view.
    (matches!(
        motion_model,
        ambition_platformer2d_core::movement::MotionModel::SurfaceMomentum(_)
    ) && result.support.is_held())
    .then_some(result.surface_normal)
}

/// The grounded braking read behind `BodyMotionFacts::skidding`: the rider is
/// steering against its own tangential travel while riding, fast enough that
/// the fight reads as a skid rather than an ordinary walk-speed turn-around.
/// `run` shares `v_t`'s sign convention (the kernel integrates
/// `v_t += run * accel * dt`), so "against travel" is exactly a negative
/// product. Published beside the ridden-surface fact after every movement step;
/// axis walkers don't ride a tangent and stay non-skidding.
pub(super) fn surface_skidding(
    motion_model: &ambition_platformer2d_core::movement::MotionModel,
    run: f32,
) -> bool {
    /// Below this tangential speed a direction change is a step, not a skid.
    /// Sits just above the picker's run threshold so the pose only interrupts
    /// a genuine run.
    const SKID_MIN_SPEED: f32 = 240.0;
    /// Deadzone so an analog flutter around neutral can't flicker the fact.
    const SKID_MIN_INPUT: f32 = 0.25;
    let ambition_platformer2d_core::movement::MotionModel::SurfaceMomentum(m) = motion_model else {
        return false;
    };
    let ae::SurfaceMotion::Riding { v_t, .. } = m.state else {
        return false;
    };
    run.abs() >= SKID_MIN_INPUT && v_t.abs() >= SKID_MIN_SPEED && run * v_t < 0.0
}

#[cfg(test)]
mod home_momentum_tests;

