use super::*;
use ambition_platformer2d_shared_tangle::lifecycle::{
    session_world_component, InRoomInstance, LiveRoomInstance, RoomInstanceRoot, SessionRoot,
    SessionScopeId,
};
use ambition_platformer2d_world::rooms::{RoomSet, RoomSpec};

/// The hall's live room and the stage's.
pub(crate) type TwoRooms = (LiveRoomInstance, LiveRoomInstance);

/// A session in `app` whose hall (no mode) is live, and with `stage` (mode
/// `smash`) live beside it when `two`. The hall's live room is returned; the
/// stage's is the next. Shared by the per-room rule readers' witnesses.
pub(crate) fn two_game_session(app: &mut App, two: bool) -> LiveRoomInstance {
    fn room(id: &str, mode: Option<&str>) -> RoomSpec {
        let mut room = RoomSpec::new(
            id,
            ambition_platformer2d_core::World::new(
                id,
                ambition_platformer2d_core::Vec2::new(320.0, 240.0),
                ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        );
        room.metadata.mode = mode.map(str::to_owned);
        room
    }
    app.world_mut().spawn((
        SessionRoot(SessionScopeId(1)),
        RoomSet::from_parts_or_panic(
            "hall",
            vec![room("hall", None), room("stage", Some("smash"))],
            Vec::new(),
        ),
    ));
    ambition_platformer2d_world::rooms::seat_sole_live_room_by_id(app.world_mut(), "hall")
        .expect("the fixture set holds its room");
    let first = *ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<
        LiveRoomInstance,
    >(app.world())
    .expect("the hall is live");
    if two {
        let stage = session_world_component::<RoomSet>(app.world())
            .and_then(|rooms| rooms.definition_by_id("stage"))
            .expect("the fixture set holds the stage");
        app.world_mut().spawn((RoomInstanceRoot, first.next(), stage));
    }
    first
}

/// [`two_game_session`] with rule `u8`: 3 in untagged rooms, 12 in Smash's.
fn app(two: bool) -> (App, LiveRoomInstance) {
    let mut app = App::new();
    let mut rules = DeclaredRules::<u8>::default();
    rules.declare(RulesScope::UntaggedRooms, 3);
    rules.declare(RulesScope::Mode("smash"), 12);
    app.insert_resource(rules);
    let first = two_game_session(&mut app, two);
    (app, first)
}

#[derive(Resource, Default, Debug, PartialEq)]
struct Seen(Vec<Option<u8>>);

#[derive(Component)]
struct Subject(usize);

fn record(rules: RulesOf<u8>, subjects: Query<(Entity, &Subject)>, mut seen: ResMut<Seen>) {
    let mut found: Vec<(usize, Option<u8>)> = subjects
        .iter()
        .map(|(entity, subject)| (subject.0, rules.of(entity)))
        .collect();
    found.sort_by_key(|(index, _)| *index);
    seen.0 = found.into_iter().map(|(_, rule)| rule).collect();
}

/// OW1: each subject reads the rules of its own live room. A body in the hall
/// reads Ambition's 3, a body on the stage reads Smash's 12, and a body in no
/// live room reads the rules of no room. Through `GoverningRules::get`, THE
/// live room, all three read the rules of no room while two rooms are live.
#[test]
fn each_subject_reads_the_rules_of_its_own_live_room() {
    let (mut app, first) = app(true);
    app.init_resource::<Seen>().add_systems(Update, record);
    app.world_mut().spawn((Subject(0), InRoomInstance(first)));
    app.world_mut().spawn((Subject(1), InRoomInstance(first.next())));
    app.world_mut().spawn(Subject(2));
    app.update();
    assert_eq!(app.world().resource::<Seen>().0, vec![Some(3), Some(12), None]);
}

/// The control: with one live room an unstamped body is in it, as before.
#[test]
fn with_one_live_room_an_unstamped_subject_reads_that_rooms_rules() {
    let (mut app, _) = app(false);
    app.init_resource::<Seen>().add_systems(Update, record);
    app.world_mut().spawn(Subject(0));
    app.update();
    assert_eq!(app.world().resource::<Seen>().0, vec![Some(3)]);
}

#[derive(Resource, Default, Debug, PartialEq)]
struct Scopes(Vec<bool>);

fn ask(room: CurrentRoom, mut scopes: ResMut<Scopes>) {
    scopes.0 = vec![
        room.in_scope(RulesScope::UntaggedRooms),
        room.in_scope(RulesScope::Mode("smash")),
        room.in_scope(RulesScope::Mode("sanic")),
    ];
}

/// OW1: a gate with no subject runs while its scope governs ANY live room.
/// With Ambition's hall and Smash's stage live, both games' gates are open
/// and a third game's is shut. Through THE live room, every gate was shut.
#[test]
fn a_scope_governs_while_any_live_room_is_its_own() {
    let (mut app, _) = app(true);
    app.init_resource::<Scopes>().add_systems(Update, ask);
    app.update();
    assert_eq!(app.world().resource::<Scopes>().0, vec![true, true, false]);
}

/// The control: with the hall alone live, only Ambition's gate is open.
#[test]
fn with_one_live_room_only_its_own_scope_governs() {
    let (mut app, _) = app(false);
    app.init_resource::<Scopes>().add_systems(Update, ask);
    app.update();
    assert_eq!(app.world().resource::<Scopes>().0, vec![true, false, false]);
}
