//! A death holds the body by CLAIMING, so nobody else's release can free
//! it.
//!
//! [`a_death_claims_the_sequence_hold`] says the claim happened at all, and
//! [`a_captor_letting_go_cannot_free_a_body_that_died_in_its_grip`] says what
//! the claim buys.

use super::*;
use ambition_combat::death_rules::DeathCause;
use ambition_characters::control::{release_control_hold, ControlHold, ControlHolds};
use ambition_combat::events::HitSource;
use bevy::prelude::{App, Commands, Entity, Query, Update};

/// A minimal world with the death beat wired and nothing else.
///
/// `GoverningRules<DeathRules>` is deliberately unfurnished. Both of its halves
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

/// The claim itself: a dead body is held by exactly the sequence bit.
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
    assert_eq!(
        app.world().get::<ControlHolds>(victim).copied(),
        Some(ControlHolds::only(ControlHold::Sequence)),
        "a dead body still answers input, or is held by an authority other than the death beat"
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
        app.world().get::<ControlHolds>(victim).is_some(),
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

/// Two live rooms of two games: Ambition's hall (#0) replays its level after
/// a 1.5 s beat, and Smash's stage (#1) holds a 4 s beat and never replays.
fn app_with_two_games_death_rules() -> (App, crate::session::governing_rules::tests::TwoRooms) {
    use ambition_combat::scoped_rules::{DeclareRulesExt, RulesScope};
    let mut app = App::new();
    app.add_message::<ActorDiedMessage>()
        .add_message::<RoomReplayRequested>();
    app.init_resource::<crate::session::lifecycle_commit::PendingLifecycleCommit>();
    app.declare_rules(RulesScope::UntaggedRooms, DeathRules::replay_level_after(1.5));
    app.declare_rules(
        RulesScope::Mode("smash"),
        DeathRules {
            interlude: 4.0,
            level_reset: LevelReset::Never,
        },
    );
    let hall = crate::session::governing_rules::tests::two_game_session(&mut app, true);
    (app, (hall, hall.next()))
}

/// OW1: a death opens the beat of its own room's rules. With THE live room's
/// rules, both rooms had the rules of no room (no beat at all).
#[test]
fn a_death_holds_the_beat_of_its_own_live_room() {
    use ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance;
    let (mut app, (hall, stage)) = app_with_two_games_death_rules();
    app.add_systems(Update, open_death_interlude);
    let victims = [hall, stage].map(|room| app.world_mut().spawn((PlayerEntity, InRoomInstance(room))).id());
    for victim in victims {
        app.world_mut().write_message(ActorDiedMessage {
            victim,
            pos: ambition_platformer2d_core::Vec2::ZERO,
            cause: DeathCause {
                source: HitSource::Hazard,
                attacker: None,
            },
        });
    }
    app.update();
    assert_eq!(
        victims.map(|victim| app.world().get::<DeathInterlude>(victim).map(|window| window.remaining)),
        [Some(1.5), Some(4.0)],
        "[the hall's beat, the stage's beat]"
    );
}

/// OW1: a closing beat asks its own room's rules whether the level goes back.
/// The last participant's beat closes in the hall, whose rules replay the
/// level. With THE live room's rules (the rules of no room: never), nothing
/// was replayed.
#[test]
fn a_closing_beat_replays_by_its_own_live_rooms_rules() {
    use ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance;
    let (mut app, (hall, _stage)) = app_with_two_games_death_rules();
    app.add_systems(Update, close_death_interlude);
    app.world_mut().spawn((
        PlayerEntity,
        OutOfPlay,
        InRoomInstance(hall),
        DeathInterlude {
            remaining: 0.0,
            consequence_pending: true,
        },
    ));
    app.update();
    let replays = app
        .world_mut()
        .resource_mut::<bevy::prelude::Messages<RoomReplayRequested>>()
        .drain()
        .count();
    assert_eq!(replays, 1, "the hall's last beat did not send the level back");
}
