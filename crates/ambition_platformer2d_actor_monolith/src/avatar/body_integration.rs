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
use ambition_combat::feel::Platformer2dFeelTuningMonolith;
use ambition_characters::actor::BodyCombat;

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

/// The per-body home movement core — control phase **and** simulation phase in ONE
/// combined kernel call, `ae::step_motion`: the literal same
/// engine entry a brain-driven actor uses (`ActorMut::integrate_body`). Called by
/// the unified `integrate_sim_bodies` phase for every `PlayerEntity`, so the home
/// body and every actor integrate through one function inside one scheduled system.
///
/// THE TWO-CLOCK SPLIT IS AN INPUT AFFORDANCE, NOT A SIMULATION STRUCTURE.
/// Precision-blink bullet-time keeps the player's aim responsive while the world
/// slows. It is carried entirely by `InputState::control_dt`: the human sets
/// `control_dt = real frame_dt` (so the engine runs the control phase at real time
/// and the simulation phase at `sim_dt`), while a brain leaves `control_dt = 0`.
///
/// `ActorControl` is the single source of truth for input — the brain translates
/// every verb the simulation consumes. The hitstun gate applies inside
/// `engine_input_from_actor_control`.
///
/// On a flagged reset (drown / hazard / out-of-bounds) the body teleports to
/// spawn (engine-level body reset, the same on every body) and `frame_out.reset`
/// is set.
///
/// ⛔ **but only while the participant is still IN PLAY** (ADR 0033). A body
/// whose attempt has already ended is not teleported, not reset, and not
/// re-flagged — the world stops acting on it and the ruleset owns what happens
/// next. The room-feature reset that used to ride this flag every frame
/// (`apply_home_reset_policy`, deleted) is now a consequence the game authors
/// through `DeathRules`.
///
/// ⛔ **except a hazard a body that cannot be hurt walked into**, which is where
/// this function decides. [`ae::ResetCause`]'s own contract is *"the kernel
/// reports WHAT the world did to the body; the owner decides what it MEANS"*, and
/// this is the owner. The reasoning is at the filter below.
///
/// `invulnerable` and `evading` are the two halves of
/// [`crate::combat::util::body_vulnerable`] the clusters do not already carry.
/// ⚠ they are passed as INPUTS rather than as a resolved `bool` on purpose: the
/// predicate is applied in ONE place — here — so a second caller cannot invent a
/// slightly different rule for "can this body be hurt".
#[allow(clippy::too_many_arguments)]
pub fn integrate_home_body(
    actor_control: ambition_characters::actor::control::ActorControlFrame,
    world: &ae::World,
    clusters: &mut ae::BodyClustersMut<'_>,
    combat: &BodyCombat,
    invulnerable: ambition_characters::actor::Invulnerability,
    evading: bool,
    // Has this participant's attempt already ended (`OutOfPlay`, ADR 0033)? An
    // input for the same reason `invulnerable` is one: the predicate for "may
    // the world act on this body" is applied HERE, in one place, so a second
    // caller cannot invent a slightly different rule.
    out_of_play: bool,
    hurtbox: &mut ae::CenteredAabb,
    frame_out: &mut PlayerBodyFrameOutput,
    motion_model: &mut crate::features::MotionModel,
    motion_frame: ae::MotionFrame,
    axis_tuning: ae::MovementTuning,
    feel: Platformer2dFeelTuningMonolith,
    frame_dt: f32,
    scaled_dt: f32,
    // **The other solid bodies this one may not walk through**, sampled before
    // any body moved. `BodyContactField::NONE` for a body whose composition
    // never granted the capability, which is every body in Ambition today.
    contact_field: ae::BodyContactField<'_>,
) -> Option<ae::Vec2> {
    // ⭐ the BODY, so both roads read the same authority. The actor road used to
    // spell half of it (ledger D108); the signature no longer has a half to
    // spell.
    let input =
        engine_input_from_actor_control(actor_control, feel, combat, clusters.shield, frame_dt);
    // ⭐⭐ **the hitlag freeze and the tuning refresh are no longer SPELLED here.**
    // Both roads used to write the same two steps beside their own
    // `ae::step_motion` call, which is exactly how D114 happened: the freeze was
    // a line one road had and the other did not. They are one call now —
    // `ambition_characters::actor::step_body` — so a rule about how a body
    // integrates cannot reach this road and miss the actor one.

    // ⭐ **the ledge-platform carry is GONE from here, and that is the point.** It
    // was the last thing in this function that only a home body could get: a
    // `&[MovingPlatformState]` scan that matched the hang against each platform
    // by position and carried it by `last_delta`. `integrate_actor_body` was
    // never handed the platform set, so an enemy or NPC hanging on a moving
    // platform — kernel state, no player marker — was left behind by it.
    //
    // It now runs inside `update_body_simulation_inner` for EVERY body, reading
    // the carrying solid's own `Block::velocity` straight off the collision world
    // this function already composites. See `ledge_grab::ledge_carry_for_frame`.
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

    // Capture the causal position before home-body policy teleports to spawn.
    // Reading kinematics after `reset_body_clusters` would report the respawn
    // point as the death impact location.
    // ⛔ **A HAZARD TILE IS DAMAGE, so it has to ask what damage asks.** Jon,
    // from play: *"in super sanic mode, sanic should be invincible, even to
    // spikes."* He was right, and the cause is structural rather than a Sanic
    // bug: an authored `HazardBlock` reaches the runtime by TWO roads. Drawn as
    // an entity it becomes an ECS damage volume, and `ambition_combat::hazards`
    // gates it on `body_vulnerable` like every other emitter. Drawn as an
    // IntGrid tile it becomes `BlockKind::Hazard`, the kernel flags
    // `ResetCause::Hazard`, and this line teleported the body to spawn with
    // nothing consulted at all. **The same authored spikes therefore behaved
    // differently depending on how they had been drawn**, and no invulnerability
    // — super form, transformation beat, scripted grant, i-frames — could see
    // the tile road.
    //
    // ⚠ **`LeftTheWorld` and `Drowned` are NOT exempted, deliberately.**
    // `resolve_body_hit` already states the rule for damage — *"you cannot be
    // invulnerable to the edge of the world"* — and this seam has to agree or
    // the two disagree about the same body. Falling out is not something that
    // HIT you, and neither is running out of air. `Requested` is the reset verb
    // and is always honoured.
    let reset = result.events.reset.and_then(|cause| {
        let untouched = cause == ae::ResetCause::Hazard
            && !crate::combat::util::body_vulnerable(
                invulnerable,
                evading,
                clusters.shield,
                combat,
            );
        // ⛔ **AND A BODY THAT IS ALREADY OUT OF PLAY IS NOT KILLED AGAIN**
        // (ADR 0033). The gate above is a POSITION TEST and re-fires every tick
        // a body is past the margin — the ACTOR path has always known this and
        // writes `em.health.alive() && …` for exactly this reason, and this
        // path never got the same guard. Measured 2026-08-09: one fall into a
        // Mary-O pit re-flagged the reset on 192 of the 192 frames of her death
        // beat, and in the hosted app every one of those frames was a full
        // room-feature reset — Jon, from play: *"enemies respawning immediately
        // when she dies even though the animation and music is still playing.
        // That is not correct."*
        //
        // ⭐ she goes on FALLING, which is the classic behaviour and now costs
        // nothing: the body is simply no longer teleported out from under its
        // own death. The pose pin that used to fake this is deleted.
        (!untouched && !out_of_play).then(|| BodyReset {
            cause,
            origin: clusters.kinematics.pos,
        })
    });
    // ⛔ **AND THE BODY IS NOT TELEPORTED HOME** (ADR 0033). This used to call
    // `reset_body_clusters(.., world.spawn, ..)` right here, which is why every
    // ruleset that wanted a death to MEAN something had to claw the body back:
    // Mary-O's beat pinned her at the place she died precisely because the
    // engine had already moved her to spawn, and the pin — outside the world —
    // is what re-fired the gate 192 times per death.
    //
    // ⭐ **the respawn is a CONSEQUENCE now, not a reflex.** A reset is
    // reported, `publish_kernel_reset_death` turns it into the death fact, and
    // the game's authored `DeathRules` decides what happens: a level reset puts
    // her back through the one shared road (`reset_sandbox`, via
    // `RoomReplayRequested`), and a ruleset that owns its own respawn — a versus
    // stock — does it there instead. Until then she keeps falling, which is what
    // a classic platformer death looks like and now costs nothing to get.
    //
    // (The `axis_tuning.air_jumps` argument this used to thread lives on at the
    // remaining call sites; the note it carried — that this site was the one of
    // five that forgot the follow-up refresh — is answered by there being one
    // fewer site.)

    *frame_out = PlayerBodyFrameOutput {
        reset,
        events: result.events,
    };

    // ⭐ the ONE footprint publish, shared with the actor road (D117). A home
    // body's collision box IS its footprint, so it passes no envelope.
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
        crate::features::MotionModel::SurfaceMomentum(_)
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
pub fn surface_skidding(motion_model: &crate::features::MotionModel, run: f32) -> bool {
    /// Below this tangential speed a direction change is a step, not a skid.
    /// Sits just above the picker's run threshold so the pose only interrupts
    /// a genuine run.
    const SKID_MIN_SPEED: f32 = 240.0;
    /// Deadzone so an analog flutter around neutral can't flicker the fact.
    const SKID_MIN_INPUT: f32 = 0.25;
    let crate::features::MotionModel::SurfaceMomentum(m) = motion_model else {
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
/// ⛔⛔ **THIS USED TO ASK THE HOME AVATAR WHETHER THE WORLD WAS ALLOWED TO
/// MOVE**, through a `Query<&BodyCombat, PrimaryPlayerOnly>` whose `single()`
/// returned early when there was none. `InitialBodyPolicy::NoInitialBody` makes
/// zero primary players an ordinary steady state, so in every match — and in any
/// session that lowers no home avatar — not one moving platform advanced. This is
/// the same freeze Jon reported on 2026-08-07 (*"the characters are just stuck in
/// air"*): that fix took the clock, in `emit_player_time_intent_system`, and left
/// this system one step downstream still holding the old shape. `markers.rs`
/// names it exactly — *"the shape to watch for is not `With<PrimaryPlayer>`
/// itself but `single()` + `else { return }` around it"*.
///
/// ⭐ **the hitstop read was a DUPLICATE AUTHORITY, not a lost feature.** The
/// primary body's hitstop already drives the global clock to zero — hitstop is
/// the top rung of `emit_player_time_intent_system`'s ladder — so
/// `WorldTime::sim_dt` carries the freeze that this system was re-deriving from
/// the same component. Reading the world's own clock is what every body
/// integrating against these platforms already does.
///
/// ⚠ **and that agreement is the point, not a side effect.** The clock request
/// lands a frame after the hitstop timer is armed, so the old direct read froze
/// the platforms one frame BEFORE the bodies riding them stopped — a rider
/// integrating on a nonzero `dt` while its surface reported `last_delta` of zero.
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
