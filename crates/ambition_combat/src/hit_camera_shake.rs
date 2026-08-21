//! ⭐ **carved out of `ambition_platformer2d_actor_monolith` on 2026-08-21
//! (D33).** The CAUSE is a landed hit and combat owns hits; the EFFECT is a
//! `CameraShakeRequest` that `shared_tangle::camera_ease` consumes, and combat
//! already depends on that crate. Putting it the other way round — camera easing
//! reaching for combat's hit state — would invert the dependency.
//!
//! ⚠ its last tie to the monolith was the feel tuning, which moved to
//! `crate::feel` earlier the same day. The `ambition_platformer2d_runtime`
//! mention below is PROSE, not a dependency: it names a failure this module
//! avoids, and runtime sits above this crate.

//! **A landed hit shakes the screen — for whichever BODY landed it.** (P4.37)
//!
//! ⛔⛔ **the first version of this shipped as home-avatar presentation, and that
//! was the defect wearing the fix's clothes.** It lived in the app's
//! `sync_player_presentation`, whose query is `With<PlayerEntity>` and whose
//! kick was gated again on `PrimaryPlayer`. Three consequences, each fatal on its
//! own:
//!
//! 1. `PrimaryPlayer` names the HOME AVATAR, not the controlled body — the
//!    marker's own docs say so, and `time_control` carries Jon's freeze from
//!    2026-08-07 as the standing proof: *"start a CPU-versus-CPU match. There is
//!    no `PrimaryPlayer` in it"*. A match under `InitialBodyPolicy::NoInitialBody`
//!    legitimately has ZERO. So the shake was silent in exactly the fight
//!    P4.37 was written for.
//! 2. `sync_player_presentation` is registered by `ambition_app` ALONE. The
//!    standalone smash binary (`ambition_demo_smash_app`) composes
//!    `PlatformerEnginePlugins` + `PlatformerHostPlugins` and never installs it,
//!    so the feature could not fire in the proving-ground binary at all.
//! 3. it read one body's `hitstop_timer` — the home avatar's — so a hit between
//!    two seated fighters moved nothing even where a `PrimaryPlayer` existed.
//!
//! ⭐ **the severity is already resolved, and it is already body-generic.** The
//! hit resolver arms `BodyCombat::hitstop_timer` on EVERY body it touches (see
//! the note in `features/ecs/actors/update.rs`), written from
//! `ae::hit_response::hitlag_duration` = `hitlag_time × reaction_scale(knockback)`.
//! So the camera needs no move ids, no per-character table and no notion of who
//! is playing: it asks every body how hard it was just frozen and takes the
//! loudest answer. A character that authors a heavier launch gets a heavier
//! camera for free, and a CPU hitting a CPU gets the same camera a human would.
//!
//! ⚠ **max, not sum, and order-independent** — `kick` is strongest-wins anyway,
//! but folding with `max` here keeps the result independent of query iteration
//! order, which is the determinism rule this repository has been bitten by more
//! than once.
//!
//! ⛔⛔ **AND IT REACHES PRESENTATION FROM INSIDE THE SIMULATION, WHICH IS THE
//! HAZARD BEING BODY-GENERIC PUT IT IN REACH OF.** This runs in the SIMULATION
//! schedule, which a rollback host executes more than once per frame;
//! `CameraShakeState` is PRESENTATION state and is not rollback-registered, so
//! it is not rewound between those passes.
//!
//! ⛔ **the first fix for that was a `replaying_history` guard, and it was half
//! the answer.** It removed the duplicate a local rollback produces and kept the
//! PHANTOM: under predicted remote input the FIRST execution of a frame is not a
//! replay, so it passed the guard and kicked the live camera — and when the real
//! input arrived and erased the hit, nothing could unkick it. A shake with no hit
//! under it, arriving exactly when the network hiccups, which is precisely the
//! failure `ambition_platformer2d_runtime::external_effects` exists to end.
//!
//! ⭐ **so this publishes a [`CameraShakeRequest`] and lets the existing
//! confirmed-frame quarantine decide when it is real.** The mechanism was already
//! built and already carries sound, VFX, explosions, fireworks and debris: the
//! frame's intents are journalled, a re-simulation REPLACES that frame's batch
//! (an empty batch erases the phantom — the half a boolean gate structurally
//! cannot do), and release happens once the host confirms the frame. There is no
//! replay guard here any more because there is nothing left to guard: the system
//! writes a message, and a message from an abandoned branch is discarded rather
//! than suppressed.
//!
//! ⚠ **and it stays free where nothing is predicted.** Every non-rollback host
//! installs no quarantine at all, so the request is read by
//! `apply_camera_shake_requests` in the same frame it was written, exactly as the
//! direct kick was.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::camera_ease::{hit_shake_amplitude, CameraShakeRequest};

/// Kick the camera by the hardest hitlag any body in the world is currently
/// serving, measured against the route's reference connect.
///
/// Runs in `CombatSet::Settle` — the phase that reads the frame's resolved
/// damage — so the hitstop it reads is this frame's.
///
/// ⚠ **kicked every frame the freeze is live, deliberately.** `kick` is
/// strongest-wins, so re-asserting needs no edge detection and no `Local`
/// remembering last frame's timer (which would be cross-frame state in a
/// rollback schedule, for an effect that is already idempotent). It does NOT
/// hold the shake flat: `hitstop_timer` counts down, so the amplitude this asks
/// for falls through the freeze while the decay pulls the live value down too —
/// the hit peaks on the frame it lands and eases out, which is the beat.
///
/// The feel tuning is optional for the same reason the rest of this layer's
/// resources are: a headless fixture that installed no route still runs the
/// combat schedule, and a missing one means no shake rather than a panic.
///
/// ⛔ **it publishes an INTENT and touches no presentation state.** The cap, the
/// clamp and the live amplitude belong to `apply_camera_shake_requests` on the
/// far side of the confirmed-frame boundary; see the module docs for why the
/// earlier in-place kick could not be made correct by any guard living here.
pub fn shake_camera_on_landed_hits(
    feel: Option<Res<crate::feel::Platformer2dFeelTuningMonolith>>,
    mut requests: MessageWriter<CameraShakeRequest>,
    bodies: Query<&ambition_characters::actor::BodyCombat>,
) {
    let Some(feel) = feel else {
        return;
    };
    let mut hardest = 0.0f32;
    for combat in &bodies {
        hardest = hardest.max(hit_shake_amplitude(combat.hitstop_timer, feel.hitlag_time));
    }
    if hardest > 0.0 {
        requests.write(CameraShakeRequest {
            amplitude_px: hardest,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::actor::BodyCombat;
    use ambition_platformer2d_shared_tangle::camera_ease::{
        apply_camera_shake_requests, CameraShakeState, CameraShakeTuning,
    };

    /// The fixture a home avatar CANNOT satisfy: bodies with no `PlayerEntity`
    /// and no `PrimaryPlayer` at all, which is what a CPU-versus-CPU match is.
    ///
    /// ⚠ **the applier is installed too, deliberately.** These tests measure the
    /// amplitude a hit produces, which is a claim about the whole seam; a fixture
    /// that only counted the request would go green on a request nobody applies.
    /// This models the ordinary NON-ROLLBACK host, where no quarantine exists and
    /// the request is read in the frame it was written — the confirmed-boundary
    /// behaviour is the app-level `effect_quarantine` suite's to prove, because
    /// only there does a journal exist to prove it against.
    fn app_with_bodies(hitstops: &[f32]) -> App {
        let mut app = App::new();
        app.init_resource::<CameraShakeState>();
        app.init_resource::<CameraShakeTuning>();
        app.init_resource::<crate::feel::Platformer2dFeelTuningMonolith>();
        app.add_message::<CameraShakeRequest>();
        for &hitstop in hitstops {
            app.world_mut().spawn(BodyCombat {
                hitstop_timer: hitstop,
                ..Default::default()
            });
        }
        app.add_systems(
            Update,
            (shake_camera_on_landed_hits, apply_camera_shake_requests).chain(),
        );
        app
    }

    fn reference(app: &App) -> f32 {
        app.world()
            .resource::<crate::feel::Platformer2dFeelTuningMonolith>()
            .hitlag_time
    }

    /// **A hit between two bodies nobody is playing shakes the screen.**
    ///
    /// ⭐ this is the assertion the app-side version could not make: there is no
    /// `PlayerEntity` and no `PrimaryPlayer` in this world, and the camera still
    /// answers.
    #[test]
    fn a_hard_hit_on_a_body_no_one_is_playing_still_shakes_the_camera() {
        let mut app = app_with_bodies(&[]);
        let hard = reference(&app) * 4.0;
        app.world_mut().spawn(BodyCombat {
            hitstop_timer: hard,
            ..Default::default()
        });
        app.update();
        assert!(
            app.world().resource::<CameraShakeState>().amplitude_px > 0.0,
            "the hardest connect the hitlag band allows left the camera still, \
             on a body with no player marker — the shake is home-avatar-gated \
             again"
        );
    }

    /// The poison: the WEAKEST connect the hitlag law admits is the dead zone,
    /// so the softest possible poke must move nothing. A camera that shakes on
    /// everything passes the test above and fails here.
    #[test]
    fn the_weakest_possible_connect_moves_nothing() {
        let mut app = app_with_bodies(&[]);
        let poke = reference(&app) * ambition_platformer2d_core::hit_response::MIN_HITLAG_SCALE;
        app.world_mut().spawn(BodyCombat {
            hitstop_timer: poke,
            ..Default::default()
        });
        app.update();
        assert_eq!(
            app.world().resource::<CameraShakeState>().amplitude_px,
            0.0,
            "the softest connect in the game rattled the screen, so the dead \
             zone is not being applied and the camera now moves on every hit"
        );
    }

    /// A body that is not in hitlag at all contributes nothing — the guard that
    /// keeps a world full of idle bodies from holding the shake up.
    #[test]
    fn bodies_that_were_not_hit_shake_nothing() {
        let mut app = app_with_bodies(&[0.0, 0.0, 0.0]);
        app.update();
        assert_eq!(
            app.world().resource::<CameraShakeState>().amplitude_px,
            0.0,
            "a world where nobody has been hit is shaking"
        );
    }

    /// **The LOUDEST hit in the world wins, whichever body is serving it.** A
    /// system that read only the first body it found would pass the two tests
    /// above and still show the wrong hit here.
    #[test]
    fn the_hardest_hit_in_the_world_sets_the_shake_not_the_first_body_found() {
        let mut app = app_with_bodies(&[]);
        let reference = reference(&app);
        // Jab first, smash second — and then the reverse, because a
        // first-body-wins bug is only visible in one of the two orders and
        // query iteration order is not ours to choose.
        app.world_mut().spawn(BodyCombat {
            hitstop_timer: reference,
            ..Default::default()
        });
        app.world_mut().spawn(BodyCombat {
            hitstop_timer: reference * 4.0,
            ..Default::default()
        });
        app.update();
        let both = app.world().resource::<CameraShakeState>().amplitude_px;

        let mut alone = app_with_bodies(&[reference * 4.0]);
        alone.update();
        let solo = alone.world().resource::<CameraShakeState>().amplitude_px;

        assert_eq!(
            both, solo,
            "a world holding one jab and one smash shook by {both} where the \
             smash alone shakes by {solo} — the jab is being allowed to speak \
             for the frame"
        );
        assert!(solo > 0.0, "the smash itself shook nothing");
    }

    /// **The simulation half writes an INTENT and touches nothing the player can
    /// see.** (P0.1)
    ///
    /// ⛔ this is the property that makes the quarantine capable of holding the
    /// shake at all. The system used to `kick` the live resource, and no
    /// mechanism downstream could take that back once a predicted hit turned out
    /// not to have happened. Here the applier is deliberately NOT installed: an
    /// armed hit runs the producer alone, and the camera must still be perfectly
    /// still while exactly one request stands waiting.
    #[test]
    fn the_simulation_half_only_requests_the_shake_and_never_moves_the_camera() {
        let mut app = App::new();
        app.init_resource::<CameraShakeState>();
        app.init_resource::<CameraShakeTuning>();
        app.init_resource::<crate::feel::Platformer2dFeelTuningMonolith>();
        app.add_message::<CameraShakeRequest>();
        let hard = app
            .world()
            .resource::<crate::feel::Platformer2dFeelTuningMonolith>()
            .hitlag_time
            * 4.0;
        app.world_mut().spawn(BodyCombat {
            hitstop_timer: hard,
            ..Default::default()
        });
        app.add_systems(Update, shake_camera_on_landed_hits);

        app.update();

        assert_eq!(
            app.world().resource::<CameraShakeState>().amplitude_px,
            0.0,
            "the combat schedule moved the live camera itself. That write is not \
             rewindable: under a rollback host the FIRST pass over a frame is \
             already speculative, so a hit the next correction erases would have \
             kicked the screen before anyone could know it did not happen"
        );
        assert_eq!(
            app.world().resource::<Messages<CameraShakeRequest>>().len(),
            1,
            "the hardest connect the hitlag band allows produced no shake \
             request, so the seam is not merely deferred — it is severed"
        );
    }
}
