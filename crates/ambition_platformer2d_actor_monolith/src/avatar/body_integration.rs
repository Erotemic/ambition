//! Home/player body movement, decomposed so it joins the SAME scheduled body
//! integration phase as actors.
//!
//! The home body is NOT a separate gameplay species: [`integrate_home_body`] is
//! the per-body movement core the unified `integrate_sim_bodies` phase calls for
//! every `PlayerEntity`, right beside the actor bodies it integrates in the same
//! system. It runs the LITERAL same engine entry an actor uses
//! (`ae::step_motion`) over the body's `BodyClustersMut`
//! view. The ONLY home-specific work here is:
//!
//! - the two-clock precision-blink affordance carried by `InputState::control_dt`
//!   (an INPUT affordance, not a simulation structure);
//! - flagging a body reset ([`PlayerBodyFrameOutput::reset`]) for the separate
//!   home reset POLICY and PRESENTATION phases to consume.
//!
//! It performs NO sandbox reset, NO room reset, and NO presentation — those are
//! home-policy / home-view phases that read the [`PlayerBodyFrameOutput`] hand-off
//! this phase writes.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;

use crate::features::ecs::attack::engine_input_from_actor_control;
use ambition_characters::actor::BodyCombat;
use ambition_combat::feel::Platformer2dFeelTuningMonolith;

/// Movement→(reset/presentation) hand-off for a home/player body, written by the
/// unified body integration phase (`integrate_sim_bodies` → [`integrate_home_body`])
/// and read by the two home-policy phases: the home reset POLICY (sandbox reset on
/// `reset`) and the home PRESENTATION phase (screen shake / landing SFX / per-op
/// anim/SFX/VFX). Body-generic in SHAPE — it carries only integration facts (this
/// frame's `FrameEvents` + a reset flag), never any player
/// presentation state — so movement stays a pure integrate-and-report phase.
/// A required component of every player body.
#[derive(Component, Default)]
pub struct PlayerBodyFrameOutput {
    /// The movement tick's events (jump/dash/blink ops, blink endpoints, …).
    pub events: ae::FrameEvents,
    /// The world reset this body this frame, or `None` if it did not. The body
    /// was already teleported to spawn by this phase; the home reset POLICY
    /// consumes this to run the full sandbox reset for the primary, and the
    /// PRESENTATION phase skips the frame.
    ///
    /// One field, not the `reset: bool` + `reset_origin: Option<Vec2>` pair it
    /// replaces: those two had to agree, nothing made them, and a consumer
    /// reading only the bool could not say what had happened.
    pub reset: Option<BodyReset>,
}

/// A reset the world applied to a body this frame: WHY, and WHERE the body was
/// standing when it happened.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyReset {
    /// What the world did — the request, the spikes, the water, or the void.
    /// Carried through from the movement kernel's gate so downstream policy can
    /// answer "did this body fall out of the stage?" without guessing.
    pub cause: ae::ResetCause,
    /// Where the body was when the reset was triggered, before the home policy
    /// teleported it to spawn. This preserves the causal location for death VFX,
    /// replay tooling, and any other consumer that must not confuse respawn with
    /// impact.
    pub origin: ae::Vec2,
}

/// Integrate a player-owned body through the shared body kernel.
///
/// Human control may use wall-clock `control_dt` while simulation uses `sim_dt`;
/// this is an input affordance, not a separate movement model. Reset causes from
/// the kernel are interpreted here by participant policy: out-of-play bodies are
/// left alone, and hazard resets respect the body's vulnerability state.
#[allow(clippy::too_many_arguments)]
pub fn integrate_home_body(
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
    hurtbox: &mut ae::CenteredAabb,
    frame_out: &mut PlayerBodyFrameOutput,
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
        ae::MotionStepContext {
            world,
            input,
            frame: motion_frame,
            facing_intent: actor_control.facing,
            dt: scaled_dt,
            contact: contact_field,
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

    *frame_out = PlayerBodyFrameOutput {
        reset,
        events: result.events,
    };

    // A home body's collision box IS its footprint, so it passes no envelope.
    ambition_boss_encounter::attack_geometry::publish_body_footprint(
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
pub fn surface_skidding(motion_model: &ambition_platformer2d_core::movement::MotionModel, run: f32) -> bool {
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

/// Advance the world's moving platforms ONCE per frame, ahead of every body
/// integration (home + actors), so every body rides this frame's platform
/// positions. Peeled out of the per-entity body loop so it can't multiply.
///
/// `InitialBodyPolicy::NoInitialBody` makes zero primary players an ordinary steady state, so in
/// every match — and in any session that lowers no home avatar — not one moving platform advanced.
///
/// the hitstop read was a DUPLICATE AUTHORITY, not a lost feature. The
/// primary body's hitstop already drives the global clock to zero — hitstop is
/// the top rung of `emit_player_time_intent_system`'s ladder — so
/// `WorldTime::sim_dt` carries the freeze that this system was re-deriving from
/// the same component. Reading the world's own clock is what every body
/// integrating against these platforms already does.
///
/// The platform now freezes on the same frame everything else does.
pub fn advance_moving_platforms(
    world_time: Res<ambition_time::WorldTime>,
    mut platforms: ResMut<ambition_platformer2d_world::collision::MovingPlatformSet>,
) {
    let sim_dt = world_time.sim_dt();
    for platform in platforms.0.iter_mut() {
        platform.update(sim_dt);
    }
}

#[cfg(test)]
mod home_momentum_tests;
#[cfg(test)]
mod platform_advance_tests;

// ⛔ THIS SYSTEM MOVED HERE FROM `ambition_damage`, 2026-08-26, and
// it was the LAST monolith-owned thing that module named. Its own doc says
// *"'died' here means the local player's attempt ended"* — that is avatar
// language, not damage language, and the fact it reads is
// `PlayerBodyFrameOutput`, declared thirty lines above. A system that reads one
// module's output and reports one module's concern belongs in that module.
//
// ⇒ `damage_apply` now names NO monolith type at all.
/// Publish the authoritative death fact for a body the MOVEMENT KERNEL reset.
///
/// A pit fall, a drown, or a tile-grid `HazardBlock` never reaches
/// [`resolve_body_hit`]: the kernel flags `FrameEvents::reset`,
/// `integrate_home_body` teleports the body to spawn, and no health is ever
/// touched — `hazard_runtime` says so outright ("tile-grid hazards run through
/// the engine's reset-to-spawn path and never reach `HazardRuntime`"). So the
/// most common death in a platformer emitted no death signal at all, and the one
/// consumer that wanted it — Mary-O's lives — had to infer death from
/// `BodyLifetime.resets` instead.
///
/// Six unrelated callers bump `resets`: two real deaths, a room load, an avatar rebuild, a sandbox
/// reset, and a room replay's own reset. Mary-O read the replay's bump as a fresh death, spent
/// another life, and requested another replay — an unbounded loop that drained the whole lives
/// counter many times a second in the hosted app. A counter cannot carry a reason; a message can.
///
/// Scoped to the primary player because "died" here means "the local player's
/// attempt ended". An actor's own hazard reaction is its business — it never
/// teleports to the player spawn, so it never sets this flag.
pub fn publish_kernel_reset_death(
    mut died: MessageWriter<ambition_combat::death_rules::ActorDiedMessage>,
    bodies: Query<(Entity, &PlayerBodyFrameOutput), ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly>,
) {
    for (victim, frame_out) in &bodies {
        let Some(reset) = frame_out.reset else {
            continue;
        };
        died.write(ambition_combat::death_rules::ActorDiedMessage {
            victim,
            pos: reset.origin,
            // The kernel gate now says WHICH world killed her, so this reports
            // it instead of apologizing for not knowing. No entity claims any
            // of these kills — `attacker` stays `None` for all of them.
            cause: ambition_combat::death_rules::DeathCause {
                source: death_source_of(reset.cause),
                attacker: None,
            },
        });
    }
}

/// The killing category a kernel reset belongs to.
///
/// A voluntary reset is still a death: the run ended, the lives counter should
/// spend one, and a player who presses the restart verb in a platformer expects
/// the attempt to be over. It is charged to `Hazard` — the same anonymous
/// world-killed-you category the spikes use — because no vocabulary exists for
/// "you asked", and inventing one would only be honest if something read it.
fn death_source_of(cause: ae::ResetCause) -> ambition_combat::HitSource {
    match cause {
        ae::ResetCause::LeftTheWorld => ambition_combat::HitSource::LeftTheWorld,
        ae::ResetCause::Hazard | ae::ResetCause::Drowned | ae::ResetCause::Requested => {
            ambition_combat::HitSource::Hazard
        }
    }
}

#[cfg(test)]
mod kernel_reset_death_tests {
    use super::*;
    use bevy::prelude::App;

    #[test]
    fn kernel_reset_death_reports_the_pre_respawn_impact_position() {
        let mut app = App::new();
        app.add_message::<ambition_combat::death_rules::ActorDiedMessage>();
        app.add_systems(Update, publish_kernel_reset_death);

        let impact = ambition_platformer2d_core::Vec2::new(321.0, -45.0);
        app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::markers::PlayerEntity,
            ambition_platformer2d_shared_tangle::markers::PrimaryPlayer,
            crate::avatar::PlayerBodyFrameOutput {
                reset: Some(crate::avatar::BodyReset {
                    cause: ambition_platformer2d_core::ResetCause::Hazard,
                    origin: impact,
                }),
                ..default()
            },
        ));

        app.update();

        let deaths: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<ambition_combat::death_rules::ActorDiedMessage>>()
            .drain()
            .collect();
        assert_eq!(deaths.len(), 1);
        assert_eq!(
            deaths[0].pos, impact,
            "the death fact must preserve where the hazard struck, not the spawn destination"
        );
    }
}
