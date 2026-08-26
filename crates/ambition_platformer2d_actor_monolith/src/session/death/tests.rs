//! A death holds the body by CLAIMING, so nobody else's release can free
//! it.
//!
//! The two tests below are the invariant and its poison in the same file:
//! [`a_death_claims_the_sequence_hold`] says the claim happened at all, and
//! [`a_captor_letting_go_cannot_free_a_body_that_died_in_its_grip`] says what
//! the claim BUYS — which is the whole reason the direct
//! `try_insert(ScriptedControl)` was wrong.

use super::*;
use ambition_combat::death_rules::DeathCause;
use ambition_characters::control::{release_control_hold, ControlHold, ControlHolds, ScriptedControl};
use ambition_combat::events::HitSource;
use bevy::prelude::{App, Commands, Entity, Query, Update};

/// A minimal world with the death beat wired and nothing else.
///
/// `GoverningDeathRules` is deliberately unfurnished. Both of its halves
/// are optional and the absent case is the engine default, so this harness
/// exercises the same code path a composition with no death declarations does.
fn app_with_the_death_beat() -> App {
    let mut app = App::new();
    app.add_message::<ActorDiedMessage>();
    app.init_resource::<crate::session::lifecycle_commit::PendingLifecycleCommit>();
    app.add_systems(Update, open_death_interlude);
    app
}

/// Kill `victim` through the real channel, one tick.
fn kill(app: &mut App, victim: Entity) {
    app.world_mut().write_message(ActorDiedMessage {
        victim,
        pos: ambition_platformer2d_core::Vec2::ZERO,
        cause: DeathCause {
            source: HitSource::Hazard,
            attacker: None,
        },
    });
    app.update();
}

/// The claim itself. `ScriptedControl` is DERIVED — its presence means
/// `ControlHolds` is non-empty — so a death that produced the marker without a
/// bit would leave the two disagreeing.
#[test]
fn a_death_claims_the_sequence_hold() {
    let mut app = app_with_the_death_beat();
    let victim = app.world_mut().spawn(PlayerEntity).id();

    kill(&mut app, victim);

    assert!(
        app.world().get::<OutOfPlay>(victim).is_some(),
        "the death beat did not run at all — every assertion below would pass \
         vacuously on a body that was never killed"
    );
    assert!(
        app.world().get::<ScriptedControl>(victim).is_some(),
        "a dead body still answers input"
    );
    assert_eq!(
        app.world().get::<ControlHolds>(victim).copied(),
        Some(ControlHolds::only(ControlHold::Sequence)),
        "the death interlude produced `ScriptedControl` without claiming a bit, so \
         the marker and the claim set disagree about who is holding this body"
    );
}

/// THE POINT: a body that died inside a capture stays held when the captor
/// lets go.
#[test]
fn a_captor_letting_go_cannot_free_a_body_that_died_in_its_grip() {
    let mut app = app_with_the_death_beat();
    // The captor's hold, spelled the way `claim_control_hold` leaves it.
    let victim = app
        .world_mut()
        .spawn((
            PlayerEntity,
            ScriptedControl,
            ControlHolds::only(ControlHold::Relationship),
        ))
        .id();

    kill(&mut app, victim);

    // both terms OBSERVED before the release. A version of this test that
    // went straight to the release would also pass on a world where the capture
    // hold had silently vanished, or where the death never ran — neither of
    // which is the state whose behaviour is being pinned.
    let held_by_both = app.world().get::<ControlHolds>(victim).copied();
    assert!(
        held_by_both
            .is_some_and(|holds| holds.holds(ControlHold::Relationship)
                && holds.holds(ControlHold::Sequence)),
        "the setup did not produce a body held by TWO authorities: {held_by_both:?}"
    );

    // The captor lets go — of ITS hold, which is all it owns.
    fn captor_lets_go(mut commands: Commands, mut held: Query<(Entity, &mut ControlHolds)>) {
        for (body, mut holds) in &mut held {
            release_control_hold(
                &mut commands,
                body,
                Some(&mut holds),
                ControlHold::Relationship,
            );
        }
    }
    app.add_systems(Update, captor_lets_go);
    app.update();

    assert!(
        app.world().get::<ScriptedControl>(victim).is_some(),
        "a captor's release freed a body that is still mid-death-interlude: the death \
         claimed no bit, so the release read an empty claim set as `nobody is holding \
         this` and took the marker off a corpse"
    );
    assert_eq!(
        app.world().get::<ControlHolds>(victim).copied(),
        Some(ControlHolds::only(ControlHold::Sequence)),
        "the release cleared more than the one hold it owns"
    );
}
