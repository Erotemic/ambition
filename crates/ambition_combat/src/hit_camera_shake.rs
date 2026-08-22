//! Camera-shake intents derived from landed hits.
//!
//! The system is body-generic: it uses each body's hitstop and publishes the
//! strongest request for the frame. Taking the maximum keeps the result independent
//! of query iteration order.
//!
//! This code runs in simulation and must not mutate presentation state directly.
//! Requests cross the confirmed-frame effect boundary so discarded predicted hits
//! cannot leave presentation artifacts. Non-rollback hosts consume the request in
//! the same frame.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::camera_ease::{hit_shake_amplitude, CameraShakeRequest};

/// Runs in `CombatSet::Settle` — the phase that reads the frame's resolved
/// damage — so the hitstop it reads is this frame's.
///
///  kicked every frame the freeze is live, deliberately. `kick` is
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
///  it publishes an INTENT and touches no presentation state. The cap, the
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
    ///  the applier is installed too, deliberately. These tests measure the
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

    /// A hit between two bodies nobody is playing shakes the screen.
    ///
    ///  this is the assertion the app-side version could not make: there is no
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

    /// The LOUDEST hit in the world wins, whichever body is serving it. A
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

    /// The simulation half writes an INTENT and touches nothing the player can
    /// see. (P0.1)
    ///
    /// this is the property that makes the quarantine capable of holding the shake at all. Here the
    /// applier is deliberately NOT installed: an armed hit runs the producer alone, and the camera
    /// must still be perfectly still while exactly one request stands waiting.
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
