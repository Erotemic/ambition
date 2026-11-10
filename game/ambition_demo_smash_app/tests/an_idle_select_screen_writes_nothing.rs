//! The select screen does work on a frame only when what it shows changes.
//!
//! SD-06: its projection writes a UI component only for a new value. A write
//! that leaves the value alone still marks the component changed, and every
//! changed `Node` sends Bevy's UI layout back over the tree. SD-07: a card
//! resolves its portrait only when its look changes; asking every frame
//! restarted a failed load on each one.

use ambition_demo_smash::select_screen::cursor::SelectCursors;
use ambition_demo_smash::select_screen::{
    Anchored, CardPortrait, CursorNode, PortraitCell, PortraitMonogram, SlotCardFrame, SlotToken, StartButton,
};
use ambition_demo_smash_app::build_demo_app;
use bevy::prelude::*;

/// The select screen's projected nodes.
type ScreenPiece = Or<(
    With<Anchored>,
    With<SlotToken>,
    With<CursorNode>,
    With<PortraitCell>,
    With<SlotCardFrame>,
    With<CardPortrait>,
    With<PortraitMonogram>,
    With<StartButton>,
)>;

/// How many screen components were marked changed since the counter last ran:
/// (`Node`, `BorderColor`, `Visibility`).
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
struct Marked(usize, usize, usize);

fn count_marked(
    mut marked: ResMut<Marked>,
    nodes: Query<Ref<Node>, ScreenPiece>,
    borders: Query<Ref<BorderColor>, ScreenPiece>,
    visibility: Query<Ref<Visibility>, ScreenPiece>,
) {
    *marked = Marked(
        nodes.iter().filter(Ref::is_changed).count(),
        borders.iter().filter(Ref::is_changed).count(),
        visibility.iter().filter(Ref::is_changed).count(),
    );
}

/// The changes marked on each of `frames` updates.
fn marked_over(app: &mut App, frames: usize) -> Vec<Marked> {
    (0..frames)
        .map(|_| {
            app.update();
            *app.world().resource::<Marked>()
        })
        .collect()
}

#[test]
fn an_idle_select_screen_marks_nothing_changed() {
    let mut app = build_demo_app();
    app.init_resource::<Marked>();
    app.add_systems(Last, count_marked);
    app.world_mut().spawn(bevy::input::gamepad::Gamepad::default());
    marked_over(&mut app, 10);
    let pieces = app.world_mut().query_filtered::<Entity, ScreenPiece>().iter(app.world()).count();
    assert!(pieces > 20, "precondition: the select screen is up ({pieces} pieces)");

    let started = std::time::Instant::now();
    let idle = marked_over(&mut app, 120);
    println!(
        "SD-06 idle: (Node, BorderColor, Visibility) marked per frame {:?}; {:.1} us per frame over {} frames",
        idle[0],
        started.elapsed().as_secs_f64() * 1e6 / idle.len() as f64,
        idle.len()
    );
    assert!(
        idle.iter().all(|marked| *marked == Marked::default()),
        "an idle screen marked components changed: {idle:?}"
    );

    // Control: a moved cursor is a real change, and it is seen.
    app.world_mut()
        .resource_mut::<SelectCursors>()
        .seat_mut(0)
        .expect("seat 0")
        .move_to(Vec2::new(123.0, 77.0));
    let moved = marked_over(&mut app, 1)[0];
    assert!(moved.0 >= 1, "control: a moved cursor marks its node ({moved:?})");
}


/// SD-07: a card resolves its portrait when its look changes, not on every
/// frame. What that must not cost: another fighter, or the same fighter in a
/// skin, still changes the picture, and a card nobody holds hides it.
#[test]
fn a_card_shows_the_look_it_holds_after_the_look_changes() {
    use ambition_demo_smash::select::{SlotOccupant, SlotPick, SmashRoster, SmashSelect, SmashSkins};
    let picture = |app: &mut App| -> Option<String> {
        let world = app.world_mut();
        let mut cards = world.query::<(&CardPortrait, &ImageNode, &Visibility)>();
        let (_, image, visibility) = cards.iter(world).find(|(card, ..)| card.0 == 0).expect("card 0");
        (*visibility != Visibility::Hidden).then(|| image.image.path().map(|path| path.to_string()).unwrap_or_default())
    };
    let mut app = build_demo_app();
    for _ in 0..10 {
        app.update();
    }
    let roster = app.world().resource::<SmashRoster>().clone();
    assert!(roster.len() >= 2, "precondition: two fighters to choose between");
    let (first, second) = (roster.0[0].clone(), roster.0[1].clone());
    // The second fighter's sheet is a skin of the first: a look that is not
    // the fighter's own.
    app.world_mut()
        .insert_resource(SmashSkins([(first.clone(), vec![second.clone()])].into_iter().collect()));
    let mut select = |app: &mut App, change: &dyn Fn(&mut SmashSelect)| {
        change(&mut app.world_mut().resource_mut::<SmashSelect>());
        for _ in 0..3 {
            app.update();
        }
    };

    select(&mut app, &|select| {
        select.set_occupant(0, SlotOccupant::Cpu);
        select.set_pick(0, SlotPick::Fighter(0));
    });
    let own = picture(&mut app).expect("the first fighter's portrait is drawn");
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(picture(&mut app).as_ref(), Some(&own), "an idle card keeps its picture");

    select(&mut app, &|select| select.set_pick(0, SlotPick::Fighter(1)));
    let other = picture(&mut app).expect("the second fighter's portrait is drawn");
    assert_ne!(other, own, "another fighter changes the picture ({own})");

    select(&mut app, &|select| select.set_pick(0, SlotPick::Fighter(0)));
    assert_eq!(picture(&mut app).as_ref(), Some(&own), "back to the first fighter");
    select(&mut app, &|select| select.cycle_skin(0, 1));
    assert_eq!(picture(&mut app).as_ref(), Some(&other), "the first fighter in the second's look");

    select(&mut app, &|select| select.set_occupant(0, SlotOccupant::Absent));
    assert_eq!(picture(&mut app), None, "a card nobody holds draws no portrait");
}
