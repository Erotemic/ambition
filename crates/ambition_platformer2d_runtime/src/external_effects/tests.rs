//! The quarantine mechanism, driven through the REAL systems.

use super::*;
use bevy::ecs::message::MessageCursor;
use bevy::ecs::world::World;

/// Stands in for `OwnedSfxMessage` / `VfxMessage`: the payload identity is all
/// these tests need, and an integer makes "A vs corrected B" readable.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
struct TestFx(u32);

/// A host that speculates, plus one persistent cursor.
///
/// The cursor matters. A real presentation consumer holds one across frames and
/// therefore sees each message exactly once; a fresh cursor per read re-reports
/// everything still buffered, which quietly turns "delivered once" into
/// "delivered as many times as we looked".
struct Host {
    world: World,
    consumer: MessageCursor<TestFx>,
}

impl Host {
    fn new() -> Self {
        let mut world = World::new();
        world.init_resource::<Messages<TestFx>>();
        world.init_resource::<ExternalEffectJournal<TestFx>>();
        world.insert_resource(ConfirmedFrameBoundary {
            current: 0,
            confirmed: -1,
            session: 0,
        });
        Self {
            world,
            consumer: MessageCursor::default(),
        }
    }

    fn boundary(&mut self, current: i32, confirmed: i32) {
        let mut boundary = self.world.resource_mut::<ConfirmedFrameBoundary>();
        boundary.current = current;
        boundary.confirmed = confirmed;
    }

    /// Anything written outside the simulation: a menu sound, the render-side
    /// explosion fan-out. An advance must leave these completely alone.
    fn other_writer_queues(&mut self, id: u32) {
        self.world
            .resource_mut::<Messages<TestFx>>()
            .write(TestFx(id));
    }

    /// One complete simulation advance of `frame`: open a fresh outbox, let the
    /// sim write `emits`, journal the result and restore the real channel. This
    /// is the exact order `ExternalEffectQuarantinePlugin` schedules.
    fn advance(&mut self, frame: i32, confirmed: i32, emits: &[u32]) {
        self.boundary(frame, confirmed);
        self.world
            .run_system_cached(open_sim_effect_outbox::<TestFx>)
            .expect("outbox opens");
        {
            let mut messages = self.world.resource_mut::<Messages<TestFx>>();
            for &id in emits {
                messages.write(TestFx(id));
            }
        }
        self.world
            .run_system_cached(journal_sim_effects::<TestFx>)
            .expect("journal runs");
    }

    /// The host's post-advance release, returning what presentation observes.
    fn release(&mut self) -> Vec<u32> {
        self.world
            .run_system_cached(release_confirmed_effects::<TestFx>)
            .expect("release runs");
        let messages = self.world.resource::<Messages<TestFx>>();
        self.consumer.read(messages).map(|fx| fx.0).collect()
    }

    /// A rollback: the host restores `frame`, abandoning everything after it.
    fn load(&mut self, frame: i32) {
        let confirmed = self.world.resource::<ConfirmedFrameBoundary>().confirmed;
        self.boundary(frame, confirmed);
        self.world
            .run_system_cached(discard_abandoned_predictions::<TestFx>)
            .expect("discard runs");
    }

    fn journal(&self) -> &ExternalEffectJournal<TestFx> {
        self.world.resource::<ExternalEffectJournal<TestFx>>()
    }
}

#[test]
fn a_predicted_effect_waits_for_its_frame_to_confirm() {
    let mut host = Host::new();
    host.advance(0, -1, &[7]);
    assert!(
        host.release().is_empty(),
        "frame 0 is still predicted; its sound must not reach the speakers yet"
    );

    host.advance(1, 0, &[]);
    assert_eq!(host.release(), vec![7], "frame 0 confirmed — now it plays");
}

/// The case the old boolean gate could not express, and the reason this module
/// exists: the prediction was WRONG, so A must never be heard and B must be.
#[test]
fn a_corrected_frame_replaces_what_the_prediction_produced() {
    let mut host = Host::new();
    host.advance(0, -1, &[]);
    host.advance(1, -1, &[/* predicted */ 100]);

    // The real remote input arrives: rewind to 0 and re-run frame 1, which
    // this time produces a different effect.
    host.load(0);
    host.advance(1, 1, &[/* corrected */ 200]);

    assert_eq!(
        host.release(),
        vec![200],
        "the phantom must be gone and the correction must play, exactly once"
    );
}

/// The subtler half of the same rule. A re-simulation that produces NOTHING
/// still has to erase what the abandoned pass predicted — otherwise the
/// phantom survives because nothing overwrote it.
#[test]
fn a_correction_that_produces_nothing_still_erases_the_phantom() {
    let mut host = Host::new();
    host.advance(0, -1, &[]);
    host.advance(1, -1, &[100]);

    host.load(0);
    host.advance(1, 1, &[]);

    assert!(
        host.release().is_empty(),
        "frame 1 no longer produces an effect, so the predicted one must not play"
    );
}

#[test]
fn confirmed_frames_are_released_in_simulation_order() {
    let mut host = Host::new();
    host.advance(0, -1, &[10]);
    host.advance(1, -1, &[20]);
    host.advance(2, -1, &[30]);
    assert!(host.release().is_empty());

    host.advance(3, 2, &[40]);
    assert_eq!(
        host.release(),
        vec![10, 20, 30],
        "three frames confirmed at once must still arrive in the order they happened"
    );
}

#[test]
fn every_intent_is_released_exactly_once() {
    let mut host = Host::new();
    let mut delivered = Vec::new();
    for frame in 0..6 {
        host.advance(frame, frame - 2, &[frame as u32]);
        delivered.extend(host.release());
    }
    host.advance(6, 5, &[]);
    delivered.extend(host.release());

    assert_eq!(
        host.journal().released(),
        6,
        "six emitting frames, six released intents — no duplicates, no losses"
    );
    assert_eq!(
        delivered,
        vec![0, 1, 2, 3, 4, 5],
        "and the consumer saw each of them once, in order"
    );
}

/// Intents from a branch the host walked away from must never be released.
#[test]
fn an_abandoned_branch_is_discarded_on_load() {
    let mut host = Host::new();
    host.advance(0, -1, &[1]);
    host.advance(1, -1, &[2]);
    host.advance(2, -1, &[3]);

    host.load(0);
    assert_eq!(
        host.journal().depth(),
        1,
        "only frame 0 survives a restore to frame 0"
    );

    // The host re-advances the branch it actually took.
    host.advance(1, 1, &[9]);
    assert_eq!(host.release(), vec![1, 9]);
}

#[test]
fn a_new_session_invalidates_pending_intents() {
    let mut host = Host::new();
    host.advance(5, -1, &[42]);
    assert_eq!(host.journal().depth(), 1);

    host.world.resource_mut::<ConfirmedFrameBoundary>().session = 1;
    host.advance(0, 0, &[7]);

    assert_eq!(
        host.release(),
        vec![7],
        "the previous session's pending effect must not leak into this one"
    );
}

/// Advancing simulation effects must not disturb messages queued by non-simulation
/// writers such as menus or render-side fan-out.
#[test]
fn an_advance_does_not_disturb_what_another_writer_queued() {
    let mut host = Host::new();
    host.other_writer_queues(500);

    host.advance(0, 0, &[77]);
    let delivered = host.release();

    assert!(
        delivered.contains(&500),
        "an advance discarded a message the simulation did not write — this is \
         how a rollback host silently swallows menu audio"
    );
    assert!(
        delivered.contains(&77),
        "and the sim's own effect still arrives"
    );
}

/// Poison-check on the rule above: clearing instead of swapping reproduces the
/// loss. Without this, the guard could pass for the wrong reason.
#[test]
fn clearing_instead_of_swapping_would_lose_the_other_writer() {
    let mut host = Host::new();
    host.other_writer_queues(500);

    host.boundary(0, 0);
    host.world.resource_mut::<Messages<TestFx>>().clear(); // simulate clearing the live channel
    host.world
        .resource_mut::<Messages<TestFx>>()
        .write(TestFx(77));
    host.world
        .run_system_cached(journal_sim_effects::<TestFx>)
        .expect("journal runs");

    assert_eq!(
        host.release(),
        vec![77],
        "the menu sound is gone — if this ever stops reproducing, the swap has \
         become untestable and the guard above is hollow"
    );
}

/// A released effect must not be scooped back up by the next advance and played
/// again. `Messages::drain` takes BOTH of Bevy's double-buffers, so the outbox
/// the sim writes into has to be a genuinely separate channel.
#[test]
fn a_released_effect_is_not_journaled_a_second_time() {
    let mut host = Host::new();
    host.advance(0, 0, &[77]);
    assert_eq!(host.release(), vec![77]);

    for frame in 1..4 {
        host.advance(frame, frame, &[]);
        assert!(
            host.release().is_empty(),
            "frame {frame}: the already-released effect was re-journaled and replayed"
        );
    }
    assert_eq!(host.journal().released(), 1);
}

/// The journal is only bounded because the host bounds prediction. If a future
/// host stopped confirming, this is where it would show up as unbounded growth.
#[test]
fn the_journal_depth_tracks_the_unconfirmed_window() {
    let mut host = Host::new();
    for frame in 0..10 {
        host.advance(frame, frame - 4, &[frame as u32]);
        host.release();
    }
    assert_eq!(
        host.journal().depth(),
        4,
        "exactly the frames between confirmed and current stay pending"
    );
}

/// The camera shake, held to the confirmed boundary. (P0.1)
///
/// these three cases exist because the shake's first rollback guard was a
/// `replaying_history` check inside the producer, and that check is blind to the
/// case that matters. Under predicted remote input the FIRST execution of a frame
/// is not a replay, so a hit that never really happened passed the guard and
/// kicked the live camera; when the correction arrived there was nothing left to
/// undo, because `CameraShakeState` is presentation and is not rewound.
mod camera_shake {
    use super::*;
    use ambition_characters::actor::BodyCombat;
    use ambition_combat::hit_camera_shake::shake_camera_on_landed_hits;
    use ambition_combat::feel::Platformer2dFeelTuningMonolith;
    use ambition_platformer2d_shared_tangle::camera_ease::{
        apply_camera_shake_requests, CameraShakeRequest, CameraShakeState, CameraShakeTuning,
    };
    use bevy::prelude::Entity;

    /// A speculating host holding one body, the quarantine, and a camera.
    struct Fight {
        world: World,
        body: Entity,
    }

    impl Fight {
        fn new() -> Self {
            let mut world = World::new();
            world.init_resource::<Messages<CameraShakeRequest>>();
            world.init_resource::<ExternalEffectJournal<CameraShakeRequest>>();
            world.init_resource::<CameraShakeState>();
            world.init_resource::<CameraShakeTuning>();
            world.init_resource::<Platformer2dFeelTuningMonolith>();
            world.insert_resource(ConfirmedFrameBoundary {
                current: 0,
                confirmed: -1,
                session: 0,
            });
            let body = world.spawn(BodyCombat::default()).id();
            Self { world, body }
        }

        /// The hardest connect the hitlag band admits — a smash, not a poke.
        fn hard_hit(&self) -> f32 {
            self.world
                .resource::<Platformer2dFeelTuningMonolith>()
                .hitlag_time
                * 4.0
        }

        fn hitstop(&mut self, seconds: f32) {
            self.world
                .entity_mut(self.body)
                .get_mut::<BodyCombat>()
                .expect("the body keeps its combat state")
                .hitstop_timer = seconds;
        }

        /// One simulation advance in the exact order the plugin schedules it,
        /// with the real combat-schedule producer in the middle.
        fn advance(&mut self, frame: i32, confirmed: i32) {
            {
                let mut boundary = self.world.resource_mut::<ConfirmedFrameBoundary>();
                boundary.current = frame;
                boundary.confirmed = confirmed;
            }
            self.world
                .run_system_cached(open_sim_effect_outbox::<CameraShakeRequest>)
                .expect("outbox opens");
            self.world
                .run_system_cached(shake_camera_on_landed_hits)
                .expect("the combat schedule's shake producer runs");
            self.world
                .run_system_cached(journal_sim_effects::<CameraShakeRequest>)
                .expect("journal runs");
        }

        /// The host's release, then presentation's applier: the amplitude this
        /// returns is what a player would actually see.
        fn present(&mut self) -> f32 {
            self.world
                .run_system_cached(release_confirmed_effects::<CameraShakeRequest>)
                .expect("release runs");
            self.world
                .run_system_cached(apply_camera_shake_requests)
                .expect("the applier runs");
            self.world.resource::<CameraShakeState>().amplitude_px
        }

        fn load(&mut self, frame: i32) {
            self.world.resource_mut::<ConfirmedFrameBoundary>().current = frame;
            self.world
                .run_system_cached(discard_abandoned_predictions::<CameraShakeRequest>)
                .expect("discard runs");
        }

        fn released(&self) -> u64 {
            self.world
                .resource::<ExternalEffectJournal<CameraShakeRequest>>()
                .released()
        }
    }

    /// A predicted hit does not move the screen.
    #[test]
    fn a_hit_on_an_unconfirmed_frame_does_not_shake_the_camera_yet() {
        let mut fight = Fight::new();
        let hard = fight.hard_hit();
        fight.hitstop(hard);
        fight.advance(0, -1);

        assert_eq!(
            fight.present(),
            0.0,
            "a hit on a frame the host has not confirmed already kicked the \
             camera. The remote input can still contradict it, and a screen that \
             has moved cannot be moved back"
        );
    }

    /// A hit the correction erases never reaches the screen at all.
    ///
    /// this is the clause a `replaying_history` guard structurally cannot
    /// satisfy: the pass that produced the phantom was not a replay.
    #[test]
    fn a_hit_the_correction_erases_never_shakes_the_camera() {
        let mut fight = Fight::new();
        let hard = fight.hard_hit();

        // The predicted timeline: a smash connects on frame 1.
        fight.advance(0, -1);
        fight.hitstop(hard);
        fight.advance(1, -1);
        assert_eq!(fight.present(), 0.0, "frame 1 is not confirmed yet");

        // The real input arrives and it was a whiff. Rewind, and re-simulate a
        // frame 1 on which nobody was ever hit.
        fight.load(0);
        fight.hitstop(0.0);
        fight.advance(1, 1);

        assert_eq!(
            fight.present(),
            0.0,
            "the screen shook for a hit that, on the timeline the session \
             actually settled on, never landed"
        );
        assert_eq!(
            fight.released(),
            0,
            "an intent from the abandoned branch was handed to presentation"
        );
    }

    /// A confirmed hit shakes the screen, exactly once.
    ///
    /// the clause that makes the two above mean "held" rather than "broken".
    /// The count is the exactly-once half: `kick` is a `max`, so a shake released
    /// twice is invisible in the amplitude and obvious in the journal.
    #[test]
    fn a_confirmed_hit_shakes_the_camera_exactly_once() {
        let mut fight = Fight::new();
        let hard = fight.hard_hit();
        fight.hitstop(hard);
        fight.advance(0, -1);
        fight.present();

        // Frame 0 settles. The body is still in hitlag — hitstop counts down
        // over several frames — but frame 1's own intent is its own.
        fight.hitstop(0.0);
        fight.advance(1, 0);

        assert!(
            fight.present() > 0.0,
            "the hit survived confirmation and the camera never answered it, so \
             the quarantine is not a delay — it is a hole"
        );
        assert_eq!(
            fight.released(),
            1,
            "the confirmed frame's single shake request reached presentation a \
             different number of times than once"
        );
    }
}
