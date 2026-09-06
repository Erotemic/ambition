use super::*;

fn app() -> App {
    let mut app = App::new();
    app.add_systems(Update, sweep_objects_from_ended_matches);
    app
}

/// ⚠ SESSION IS `None` HERE ON PURPOSE. This crate cannot name `SessionScopeId`
/// — it depends only on the umbrella, which does not re-export it — so the arm
/// that proves identity is BOTH facts lives with the type, in
/// `ambition_match::seating`. What these tests own is the SWEEP.
fn seated(seats: usize, tick: u64) -> ActiveMatch {
    ActiveMatch::activated(seats, None, None, Some(tick))
}

/// ⛔⛔ AN OBJECT OUTLIVES ITS MOVE AND NOT ITS MATCH.
///
/// Jon, playing: *"a mine laid in a match still persists into the next match…
/// Ending a match should be cleaning everything up."* Three arms, because the
/// first two alone are satisfied by a sweep that despawns everything.
#[test]
fn only_objects_from_a_different_match_are_swept() {
    let mut app = app();
    let now = seated(2, 100);
    let instance = now.instance();
    app.insert_resource(now);

    let mine_of_this_match = app.world_mut().spawn(MatchScoped(instance)).id();
    let mine_of_the_last_one = app
        .world_mut()
        .spawn(MatchScoped(seated(2, 40).instance()))
        .id();
    // Something with no match identity at all: the stage, a prop, a fighter's
    // own body. The sweep must not touch it.
    let not_a_match_object = app.world_mut().spawn_empty().id();

    app.update();

    assert!(
        app.world().get_entity(mine_of_this_match).is_ok(),
        "the sweep despawned an object belonging to the match now running, which \
         would delete a live mine the instant it was placed"
    );
    assert!(
        app.world().get_entity(mine_of_the_last_one).is_err(),
        "an object created by a PREVIOUS match survived into this one — the \
         defect this sweep exists for"
    );
    assert!(
        app.world().get_entity(not_a_match_object).is_ok(),
        "the sweep despawned an entity carrying no match identity; it must only \
         claim what a match stamped"
    );
}

/// ⛔ AND BETWEEN MATCHES, NOTHING BELONGS.
///
/// The select screen has no `ActiveMatch`, and a mine sitting there is the same
/// defect wearing a different hat. ⚠ Asserted separately because the arm above
/// always has an active match, so it cannot see this answer at all.
#[test]
fn with_no_active_match_every_scoped_object_is_stale() {
    let mut app = app();
    let orphan = app
        .world_mut()
        .spawn(MatchScoped(seated(2, 100).instance()))
        .id();
    let bystander = app.world_mut().spawn_empty().id();

    app.update();

    assert!(
        app.world().get_entity(orphan).is_err(),
        "a match-scoped object survived into a world with no match at all"
    );
    assert!(
        app.world().get_entity(bystander).is_ok(),
        "the sweep despawned an unmarked entity when no match was running"
    );
}
