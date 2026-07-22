//! The quarantine mechanism, driven through the REAL systems.
//!
//! Every case here runs `open_sim_effect_outbox` / `journal_sim_effects` /
//! `release_confirmed_effects` / `discard_abandoned_predictions` themselves
//! rather than reimplementing their rules, so a regression in the rule fails
//! the test rather than the test's copy of it.

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

/// A new session is a new timeline. Anything still pending belongs to a world
/// that no longer exists and must not surface in the next one.
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

/// The trap that made the first version of this wrong. The sim is NOT the only
/// writer: menus and the render-side fan-out write these same channels from
/// `Update`. An advance must leave their traffic completely alone.
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
    host.world.resource_mut::<Messages<TestFx>>().clear(); // what the first version did
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
