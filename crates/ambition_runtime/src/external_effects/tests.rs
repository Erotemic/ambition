//! The quarantine mechanism, driven through the REAL systems.
//!
//! Every case here runs `clear_sim_effect_outbox` / `journal_sim_effects` /
//! `release_confirmed_effects` / `discard_abandoned_predictions` themselves
//! rather than reimplementing their rules, so a regression in the rule fails
//! the test rather than the test's copy of it.

use super::*;
use bevy::ecs::world::World;

/// Stands in for `OwnedSfxMessage` / `VfxMessage`: the payload identity is all
/// these tests need, and an integer makes "A vs corrected B" readable.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
struct TestFx(u32);

/// A host that speculates. Mirrors what the rollback bridge installs.
fn speculating_world() -> World {
    let mut world = World::new();
    world.init_resource::<Messages<TestFx>>();
    world.init_resource::<ExternalEffectJournal<TestFx>>();
    world.insert_resource(ConfirmedFrameBoundary {
        current: 0,
        confirmed: -1,
        session: 0,
    });
    world
}

fn boundary(world: &mut World, current: i32, confirmed: i32) {
    let mut b = world.resource_mut::<ConfirmedFrameBoundary>();
    b.current = current;
    b.confirmed = confirmed;
}

/// One complete simulation advance of `frame`: clear the outbox, let the sim
/// write `emits`, journal the result. This is the exact order
/// `ExternalEffectQuarantinePlugin` schedules.
fn advance(world: &mut World, frame: i32, confirmed: i32, emits: &[u32]) {
    boundary(world, frame, confirmed);
    world
        .run_system_cached(clear_sim_effect_outbox::<TestFx>)
        .expect("clear runs");
    {
        let mut messages = world.resource_mut::<Messages<TestFx>>();
        for &id in emits {
            messages.write(TestFx(id));
        }
    }
    world
        .run_system_cached(journal_sim_effects::<TestFx>)
        .expect("journal runs");
}

/// The host's post-advance release, returning what presentation would see.
fn release(world: &mut World) -> Vec<u32> {
    world
        .run_system_cached(release_confirmed_effects::<TestFx>)
        .expect("release runs");
    let messages = world.resource::<Messages<TestFx>>();
    let mut cursor = messages.get_cursor();
    cursor.read(messages).map(|fx| fx.0).collect()
}

/// A rollback: the host restores `frame`, which abandons everything after it.
fn load(world: &mut World, frame: i32) {
    boundary(
        world,
        frame,
        world.resource::<ConfirmedFrameBoundary>().confirmed,
    );
    world
        .run_system_cached(discard_abandoned_predictions::<TestFx>)
        .expect("discard runs");
}

#[test]
fn a_predicted_effect_waits_for_its_frame_to_confirm() {
    let mut world = speculating_world();
    advance(&mut world, 0, -1, &[7]);
    assert!(
        release(&mut world).is_empty(),
        "frame 0 is still predicted; its sound must not reach the speakers yet"
    );

    advance(&mut world, 1, 0, &[]);
    assert_eq!(
        release(&mut world),
        vec![7],
        "frame 0 confirmed — now it plays"
    );
}

/// The case the old boolean gate could not express, and the reason this module
/// exists: the prediction was WRONG, so A must never be heard and B must be.
#[test]
fn a_corrected_frame_replaces_what_the_prediction_produced() {
    let mut world = speculating_world();
    advance(&mut world, 0, -1, &[]);
    advance(&mut world, 1, -1, &[/* predicted */ 100]);

    // The real remote input arrives: rewind to 0 and re-run frame 1, which
    // this time produces a different effect.
    load(&mut world, 0);
    advance(&mut world, 1, 1, &[/* corrected */ 200]);

    assert_eq!(
        release(&mut world),
        vec![200],
        "the phantom must be gone and the correction must play, exactly once"
    );
}

/// The subtler half of the same rule. A re-simulation that produces NOTHING
/// still has to erase what the abandoned pass predicted — otherwise the
/// phantom survives because nothing overwrote it.
#[test]
fn a_correction_that_produces_nothing_still_erases_the_phantom() {
    let mut world = speculating_world();
    advance(&mut world, 0, -1, &[]);
    advance(&mut world, 1, -1, &[100]);

    load(&mut world, 0);
    advance(&mut world, 1, 1, &[]);

    assert!(
        release(&mut world).is_empty(),
        "frame 1 no longer produces an effect, so the predicted one must not play"
    );
}

#[test]
fn confirmed_frames_are_released_in_simulation_order() {
    let mut world = speculating_world();
    advance(&mut world, 0, -1, &[10]);
    advance(&mut world, 1, -1, &[20]);
    advance(&mut world, 2, -1, &[30]);
    assert!(release(&mut world).is_empty());

    advance(&mut world, 3, 2, &[40]);
    assert_eq!(
        release(&mut world),
        vec![10, 20, 30],
        "three frames confirmed at once must still arrive in the order they happened"
    );
}

#[test]
fn every_intent_is_released_exactly_once() {
    let mut world = speculating_world();
    for frame in 0..6 {
        advance(&mut world, frame, frame - 2, &[frame as u32]);
        release(&mut world);
    }
    advance(&mut world, 6, 5, &[]);
    release(&mut world);

    assert_eq!(
        world.resource::<ExternalEffectJournal<TestFx>>().released(),
        6,
        "six emitting frames, six released intents — no duplicates, no losses"
    );
}

/// Intents from a branch the host walked away from must never be released.
#[test]
fn an_abandoned_branch_is_discarded_on_load() {
    let mut world = speculating_world();
    advance(&mut world, 0, -1, &[1]);
    advance(&mut world, 1, -1, &[2]);
    advance(&mut world, 2, -1, &[3]);

    load(&mut world, 0);
    assert_eq!(
        world.resource::<ExternalEffectJournal<TestFx>>().depth(),
        1,
        "only frame 0 survives a restore to frame 0"
    );

    // The host re-advances the branch it actually took.
    advance(&mut world, 1, 1, &[9]);
    assert_eq!(release(&mut world), vec![1, 9]);
}

/// A new session is a new timeline. Anything still pending belongs to a world
/// that no longer exists and must not surface in the next one.
#[test]
fn a_new_session_invalidates_pending_intents() {
    let mut world = speculating_world();
    advance(&mut world, 5, -1, &[42]);
    assert_eq!(world.resource::<ExternalEffectJournal<TestFx>>().depth(), 1);

    world.resource_mut::<ConfirmedFrameBoundary>().session = 1;
    advance(&mut world, 0, 0, &[7]);

    assert_eq!(
        release(&mut world),
        vec![7],
        "the previous session's pending effect must not leak into this one"
    );
}

/// The double-buffer trap, pinned. `Messages::drain` takes BOTH of Bevy's
/// buffers, so a released effect still sitting in the older buffer would be
/// journaled a second time and played again on a later frame. The clear at the
/// start of each advance is what prevents it.
#[test]
fn the_outbox_clear_stops_a_released_effect_being_journaled_twice() {
    let mut world = speculating_world();
    advance(&mut world, 0, 0, &[77]);
    assert_eq!(release(&mut world), vec![77]);

    // Next advance. The released message is still physically present in the
    // channel's older buffer; the clear is what removes it.
    advance(&mut world, 1, 1, &[]);
    assert!(
        release(&mut world).is_empty(),
        "the already-released effect must not be re-journaled and replayed"
    );
    assert_eq!(
        world.resource::<ExternalEffectJournal<TestFx>>().released(),
        1
    );
}

/// Poison-check on the rule above: skipping the clear reproduces the duplicate.
/// Without this, `the_outbox_clear_stops_...` could pass for the wrong reason.
#[test]
fn without_the_clear_the_effect_would_be_replayed() {
    let mut world = speculating_world();
    boundary(&mut world, 0, 0);
    world.resource_mut::<Messages<TestFx>>().write(TestFx(77));
    world
        .run_system_cached(journal_sim_effects::<TestFx>)
        .expect("journal runs");
    assert_eq!(release(&mut world), vec![77]);

    // Same advance shape as the passing test, minus the clear.
    boundary(&mut world, 1, 1);
    world
        .run_system_cached(journal_sim_effects::<TestFx>)
        .expect("journal runs");
    assert_eq!(
        release(&mut world),
        vec![77],
        "this is the bug the clear exists to prevent — if this ever stops \
         reproducing, the clear has become untestable and the guard above is hollow"
    );
}

/// The journal is only bounded because the host bounds prediction. If a future
/// host stopped confirming, this is where it would show up as unbounded growth.
#[test]
fn the_journal_depth_tracks_the_unconfirmed_window() {
    let mut world = speculating_world();
    for frame in 0..10 {
        advance(&mut world, frame, frame - 4, &[frame as u32]);
        release(&mut world);
    }
    assert_eq!(
        world.resource::<ExternalEffectJournal<TestFx>>().depth(),
        4,
        "exactly the frames between confirmed and current stay pending"
    );
}
