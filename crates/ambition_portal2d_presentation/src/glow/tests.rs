use super::*;
use ambition_portal2d::PortalChannelColor;
use bevy::camera::visibility::RenderLayers;
use std::time::Duration;

const WORLD: Vec2 = Vec2::new(1000.0, 600.0);
/// One step of these tests: a 60 Hz frame.
const FRAME_S: f32 = 1.0 / 60.0;

fn purple_at(x: f32) -> PlacedPortal {
    PlacedPortal::fixed(
        PortalChannel::Authored(PortalChannelColor::Purple),
        Vec2::new(x, 300.0),
        Vec2::new(-1.0, 0.0),
        46.0,
    )
}

fn yellow_at(x: f32) -> PlacedPortal {
    PlacedPortal::fixed(
        PortalChannel::Authored(PortalChannelColor::Yellow),
        Vec2::new(x, 300.0),
        Vec2::new(1.0, 0.0),
        46.0,
    )
}

fn test_app() -> App {
    let mut app = App::new();
    crate::one_live_room(&mut app, WORLD);
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<PortalGlowMaterial>::default());
    app.init_resource::<Time>();
    app.add_systems(Update, sync_portal_glows);
    app
}

/// Step `frames` frames of [`FRAME_S`].
fn step(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.world_mut().resource_mut::<Time>().advance_by(Duration::from_secs_f32(FRAME_S));
        app.update();
    }
}

fn frames_of(seconds: f32) -> usize {
    (seconds / FRAME_S).ceil() as usize + 1
}

fn glows(app: &mut App) -> Vec<PortalGlow> {
    app.world_mut().query::<&PortalGlow>().iter(app.world()).cloned().collect()
}

/// A portal that is in its room when the room comes is level content: it is
/// there, whole, and plays no opening.
#[test]
fn a_portal_that_was_there_with_its_room_does_not_open() {
    let mut app = test_app();
    app.world_mut().spawn(purple_at(500.0));
    step(&mut app, 2);
    let all = glows(&mut app);
    assert_eq!(all.len(), 1, "one line of light for one portal: {all:?}");
    assert_eq!(all[0].appear, 1.0, "a portal that was there opened");
    assert_eq!(all[0].dissolve, 0.0);
}

/// A portal that comes after its room has settled was added: it opens from
/// nothing, and is whole after [`APPEAR_S`]. The control is the portal of the
/// same room that was there, which stays whole all the time.
#[test]
fn a_portal_added_to_a_room_opens() {
    let mut app = test_app();
    app.world_mut().spawn(purple_at(500.0));
    step(&mut app, frames_of(ROOM_SETTLE_S));
    let added = app.world_mut().spawn(yellow_at(700.0)).id();
    step(&mut app, 2);
    let all = glows(&mut app);
    let (was_there, new): (Vec<_>, Vec<_>) = all.iter().partition(|glow| glow.portal != Some(added));
    assert_eq!((was_there.len(), new.len()), (1, 1), "{all:?}");
    assert_eq!(was_there[0].appear, 1.0);
    assert!(new[0].appear > 0.0 && new[0].appear < 0.2, "an added portal is opening: {}", new[0].appear);

    step(&mut app, frames_of(APPEAR_S));
    let all = glows(&mut app);
    assert!(all.iter().all(|glow| glow.appear == 1.0), "it did not finish opening: {all:?}");
}

/// The first portal of a room that has been live for a time was added to it.
/// A room with no portal is still a room that is seen.
#[test]
fn the_first_portal_of_an_old_room_opens() {
    let mut app = test_app();
    step(&mut app, frames_of(ROOM_SETTLE_S));
    assert!(glows(&mut app).is_empty());
    app.world_mut().spawn(purple_at(500.0));
    step(&mut app, 2);
    let all = glows(&mut app);
    assert_eq!(all.len(), 1);
    assert!(all[0].appear < 0.2, "the first portal of an old room did not open: {}", all[0].appear);
}

/// A portal that is removed dissolves where it was, and its line is gone
/// after [`DISSOLVE_S`]. A portal that stays does not dissolve.
#[test]
fn a_portal_that_is_removed_dissolves_where_it_was() {
    let mut app = test_app();
    let leaves = app.world_mut().spawn(purple_at(500.0)).id();
    app.world_mut().spawn(yellow_at(700.0));
    step(&mut app, 3);
    app.world_mut().despawn(leaves);
    step(&mut app, 3);
    let all = glows(&mut app);
    let (gone, stays): (Vec<_>, Vec<_>) = all.iter().partition(|glow| glow.portal.is_none());
    assert_eq!((gone.len(), stays.len()), (1, 1), "{all:?}");
    assert!(gone[0].dissolve > 0.0 && gone[0].dissolve < 0.3, "{}", gone[0].dissolve);
    assert_eq!(gone[0].pos, Vec2::new(500.0, 300.0), "it dissolves where it was");
    assert_eq!(stays[0].dissolve, 0.0);

    step(&mut app, frames_of(DISSOLVE_S));
    let all = glows(&mut app);
    assert_eq!(all.len(), 1, "the dissolved line stayed: {all:?}");
    assert!(all[0].portal.is_some());
}

/// A rollback gives a restored portal a new entity. The same channel at the
/// same place in the same frame is the same aperture: no dissolve, no opening.
/// The control moves the new portal, which is then a removal and an addition.
#[test]
fn a_portal_that_comes_back_as_a_new_entity_at_its_place_is_the_same_aperture() {
    for (moved_to, effects) in [(500.0, false), (640.0, true)] {
        let mut app = test_app();
        let old = app.world_mut().spawn(purple_at(500.0)).id();
        step(&mut app, frames_of(ROOM_SETTLE_S));
        app.world_mut().despawn(old);
        let new = app.world_mut().spawn(purple_at(moved_to)).id();
        step(&mut app, 2);
        let all = glows(&mut app);
        let of_new: Vec<_> = all.iter().filter(|glow| glow.portal == Some(new)).collect();
        assert_eq!(of_new.len(), 1, "moved to {moved_to}: {all:?}");
        if effects {
            assert_eq!(all.len(), 2, "the old line dissolves and a new one opens: {all:?}");
            assert!(of_new[0].appear < 0.2, "a portal at a new place did not open");
        } else {
            assert_eq!(all.len(), 1, "one aperture, one line: {all:?}");
            assert_eq!(of_new[0].appear, 1.0, "the same aperture opened again");
        }
    }
}

/// The line follows a portal that moves (a portal on a moving face), and is on
/// the world layer and in its room, so a portal capture photographs it.
#[test]
fn the_line_follows_its_portal_and_is_drawn_in_its_room() {
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
    let mut app = test_app();
    let portal = app.world_mut().spawn(purple_at(500.0)).id();
    step(&mut app, 2);
    app.world_mut().get_mut::<PlacedPortal>(portal).unwrap().pos.y = 340.0;
    step(&mut app, 1);
    let frame = crate::PortalWorldFrame { size: WORLD };
    let (transform, stamp, layers) = app
        .world_mut()
        .query_filtered::<(&Transform, Option<&InRoomInstance>, Option<&RenderLayers>), With<PortalGlow>>()
        .single(app.world())
        .expect("one line");
    let at = frame.to_render(Vec2::new(500.0, 340.0), crate::PORTAL_FRAME_Z);
    assert_eq!(transform.translation, at, "the line did not follow its portal");
    assert_eq!(stamp.map(|stamp| stamp.0), Some(LiveRoomInstance::ACTIVATION));
    assert!(layers.is_none(), "the line is on the world layer, so captures photograph it");
    // A wall portal's opening is upright: the quad's long axis is the y axis.
    let long = transform.rotation * Vec3::X;
    assert!(long.x.abs() < 1e-4 && long.y.abs() > 0.999, "{long:?}");
    // Its local +y, the room side, is the portal's normal in render space.
    let room_side = transform.rotation * Vec3::Y;
    assert!((room_side.x - -1.0).abs() < 1e-4, "{room_side:?}");
}

/// A new session has the same room ordinals. Its room is new, so a portal
/// that is there with it does not open; and the glow of the old session is
/// taken away and is not the new portal's. The control is the same steps in
/// one session (`the_first_portal_of_an_old_room_opens`): there the portal
/// was added to an old room, and it opens.
#[test]
fn a_new_session_has_new_rooms_and_none_of_the_old_sessions_glows() {
    use ambition_platformer2d_shared_tangle::lifecycle::{ActiveSessionScope, SessionScopedEntity};
    let mut app = test_app();
    app.init_resource::<ActiveSessionScope>();
    let first = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    let old_portal = app.world_mut().spawn(purple_at(500.0)).id();
    step(&mut app, frames_of(ROOM_SETTLE_S));
    let old_glow = app
        .world_mut()
        .query::<(Entity, &PortalGlow, &SessionScopedEntity)>()
        .single(app.world())
        .map(|(entity, _, owner)| (entity, owner.0))
        .expect("the first session's glow, owned by it");
    assert_eq!(old_glow.1, first);

    // The next session: the old portal is gone, and a portal of the new
    // session's level is at another place in the room of the same ordinal.
    app.world_mut().despawn(old_portal);
    let second = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    assert_ne!(first, second);
    app.world_mut().spawn(yellow_at(700.0));
    step(&mut app, 2);

    assert!(app.world().get_entity(old_glow.0).is_err(), "the old session's glow went into the new session");
    let glows: Vec<(PortalGlow, SessionScopedEntity)> = app
        .world_mut()
        .query::<(&PortalGlow, &SessionScopedEntity)>()
        .iter(app.world())
        .map(|(glow, owner)| (glow.clone(), *owner))
        .collect();
    assert_eq!(glows.len(), 1, "one portal in the new session, one glow: {glows:?}");
    assert_eq!(glows[0].1 .0, second, "the glow is the new session's");
    assert_eq!(
        glows[0].0.appear, 1.0,
        "a portal that was there with the new session's room opened: the room's age came from the old session"
    );
}

/// With the session lifecycle installed and no session current, there is no
/// gameplay to draw: no glow stays.
#[test]
fn no_glow_stays_with_no_session() {
    use ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope;
    let mut app = test_app();
    app.init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    app.world_mut().spawn(purple_at(500.0));
    step(&mut app, 2);
    assert_eq!(glows(&mut app).len(), 1);
    app.world_mut().resource_mut::<ActiveSessionScope>().clear();
    step(&mut app, 2);
    assert!(glows(&mut app).is_empty(), "a glow stayed after its session");
}

/// A line has one colour, its own, unless the style asks for its partner's
/// on its far side. The partner's colour is seen through the portal.
#[test]
fn a_line_has_its_own_colour_and_its_partners_only_when_the_style_asks() {
    let mut app = test_app();
    app.world_mut().spawn(purple_at(100.0));
    app.world_mut().spawn(yellow_at(400.0));
    step(&mut app, 3);
    let colours = |app: &mut App| -> Vec<(Vec4, Vec4)> {
        let handles: Vec<_> = app
            .world_mut()
            .query::<&MeshMaterial2d<PortalGlowMaterial>>()
            .iter(app.world())
            .map(|material| material.0.clone())
            .collect();
        let materials = app.world().resource::<Assets<PortalGlowMaterial>>();
        handles.iter().map(|handle| materials.get(handle).map(|m| (m.front, m.back)).unwrap()).collect()
    };
    let plain = colours(&mut app);
    assert_eq!(plain.len(), 2, "premise: two lines");
    assert!(plain.iter().all(|(front, back)| front == back), "a line has its partner's colour by default: {plain:?}");
    assert_ne!(plain[0].0, plain[1].0, "premise: the two portals have two colours");

    app.insert_resource(PortalGlowStyle { partner_colour: true });
    step(&mut app, 2);
    let paired = colours(&mut app);
    assert!(paired.iter().all(|(front, back)| front != back), "the style did not reach the lines: {paired:?}");
    // The far side of each line is the near side of the other.
    assert!(paired.iter().all(|(_, back)| paired.iter().any(|(front, _)| front == back)));
}

/// Each light line this crate keeps for a module: its owner, its slot, its
/// middle and its colour.
fn held_lines(app: &mut App) -> Vec<(Entity, u32, Vec2, PortalChannel)> {
    let mut rows: Vec<_> = app
        .world_mut()
        .query::<(&HeldLightLineOf, &PortalLightLine)>()
        .iter(app.world())
        .map(|(of, line)| (of.owner, of.slot, line.pos, line.channel))
        .collect();
    rows.sort_by_key(|row| (row.0.index(), row.1));
    rows
}

fn view(lines: &[(u32, f32)]) -> ambition_portal2d::extension::HeldLightLines {
    ambition_portal2d::extension::HeldLightLines(
        lines
            .iter()
            .map(|(slot, x)| ambition_portal2d::extension::HeldLightLine {
                entry: "test::lines/conduct".into(),
                slot: *slot,
                at: Vec2::new(*x, 300.0),
                normal: Vec2::new(1.0, 0.0),
                length: 120.0,
                look: 25,
                on_limb: None,
            })
            .collect(),
    )
}

/// ⭐ THE PRESENTATION DRAWS EACH LINE OF A BODY'S VIEW, AND ONLY THOSE. A
/// line of `ambition.portal.light_line` is a view on its body; this crate
/// keeps one `PortalLightLine` for each, follows it, and takes it away when
/// the view loses it or the body goes.
#[test]
fn a_light_line_is_drawn_while_its_body_has_it_in_its_view() {
    let mut app = App::new();
    crate::one_live_room(&mut app, WORLD);
    app.add_systems(Update, dress_held_light_lines);
    let a = app.world_mut().spawn(view(&[(0, 100.0), (1, 200.0)])).id();
    let b = app.world_mut().spawn(view(&[(0, 700.0)])).id();
    app.update();
    let orange = PortalChannel::Authored(PortalChannelColor::Indexed(25));
    assert_eq!(
        held_lines(&mut app),
        vec![
            (a, 0, Vec2::new(100.0, 300.0), orange),
            (a, 1, Vec2::new(200.0, 300.0), orange),
            (b, 0, Vec2::new(700.0, 300.0), orange),
        ],
        "each line of each view is drawn once"
    );

    app.world_mut().entity_mut(a).insert(view(&[(1, 250.0)]));
    app.update();
    app.update();
    assert_eq!(
        held_lines(&mut app),
        vec![(a, 1, Vec2::new(250.0, 300.0), orange), (b, 0, Vec2::new(700.0, 300.0), orange)],
        "a line the view lost is taken away, and a line that moved follows"
    );

    app.world_mut().despawn(b);
    app.world_mut().entity_mut(a).remove::<ambition_portal2d::extension::HeldLightLines>();
    app.update();
    app.update();
    assert_eq!(held_lines(&mut app), vec![], "no view, no line");
}

/// ⭐ A LINE ON A LIMB IS ON THE LIMB'S CUT (Jon, 2026-10-24: "We need the
/// capability to know exactly where the end points of the arms "moving
/// through" the portal are, so we can draw the portal lines correctly"). A
/// module puts a shoulder's line and a wrist's line near the arm; the drawn
/// pictures publish where the arm is cut (`LimbSeams`). Each line is laid on
/// its cut: its middle within a few px of the cut's middle, as wide as the
/// arm there plus its margin, rotated to the cut, and drawn over the arm. A
/// cut of another limb, or a limb's own cut out of reach, is not taken.
#[test]
fn a_line_on_a_limb_is_laid_on_the_limbs_cut() {
    use crate::limb_seams::{LimbSeam, LimbSeams, LAY_REACH, LAY_S, LIMB_LINE_MARGIN, OVER_LIMB_Z};
    let mut app = App::new();
    crate::one_live_room(&mut app, WORLD);
    app.init_resource::<Time>();
    app.add_systems(Update, (dress_held_light_lines, crate::limb_seams::lay_light_lines_on_limb_seams).chain());
    let on_limb = |slot: u32, x: f32, limb: &str, on_its_body: bool| ambition_portal2d::extension::HeldLightLine {
        entry: "test::lines/conduct".into(),
        slot,
        at: Vec2::new(x, 300.0),
        normal: Vec2::new(1.0, 0.0),
        // Tight: shorter than the arm is wide.
        length: 30.0,
        look: 24,
        on_limb: Some(ambition_portal2d::extension::LineOnLimb { limb: limb.into(), on_its_body }),
    };
    app.world_mut().spawn(ambition_portal2d::extension::HeldLightLines(vec![
        on_limb(0, 100.0, "hand_left", false),
        on_limb(2, 600.0, "hand_left", true),
        on_limb(3, 900.0, "hand_right", true),
    ]));
    let seam = |limb: Option<&str>, middle: Vec2, along: Vec2, half: f32| LimbSeam {
        limb: limb.map(str::to_owned),
        middle,
        along,
        half,
        depth: 12.0,
    };
    let slant = Vec2::new(0.2, 1.0).normalize();
    // The giant: its left stump, and its right stump nearer the left line.
    app.world_mut().spawn(LimbSeams(vec![
        seam(Some("hand_left"), Vec2::new(130.0, 320.0), slant, 28.0),
        seam(Some("hand_right"), Vec2::new(101.0, 300.0), Vec2::Y, 28.0),
    ]));
    // A lone fist by the wrist line at 600; none in reach of the one at 900.
    app.world_mut().spawn(LimbSeams(vec![seam(None, Vec2::new(590.0, 302.0), Vec2::Y, 29.0)]));
    app.world_mut().spawn(LimbSeams(vec![seam(None, Vec2::new(900.0 + LAY_REACH + 40.0, 300.0), Vec2::Y, 29.0)]));
    step(&mut app, frames_of(LAY_S) + 2);

    let lines: Vec<(u32, PortalLightLine)> = app
        .world_mut()
        .query::<(&HeldLightLineOf, &PortalLightLine)>()
        .iter(app.world())
        .map(|(of, line)| (of.slot, *line))
        .collect();
    let line = |slot: u32| lines.iter().find(|(s, _)| *s == slot).map(|(_, line)| *line).expect("the line is drawn");

    let stump = line(0);
    assert!(stump.pos.distance(Vec2::new(130.0, 320.0)) <= 2.0, "the shoulder's line is off its cut: {:?}", stump.pos);
    assert!((stump.length - (56.0 + LIMB_LINE_MARGIN)).abs() < 0.5, "not as wide as the arm: {}", stump.length);
    assert!(stump.normal.dot(slant).abs() < 1e-3, "not rotated to the cut: {:?}", stump.normal);
    assert!(stump.normal.x > 0.0, "the room's side is the side the module named");
    assert_eq!(stump.depth, Some(12.0 + OVER_LIMB_Z), "drawn over the arm it cuts");

    let wrist = line(2);
    assert!(wrist.pos.distance(Vec2::new(590.0, 302.0)) <= 2.0, "the wrist's line is off its cut: {:?}", wrist.pos);
    assert!((wrist.length - (58.0 + LIMB_LINE_MARGIN)).abs() < 0.5);

    let lonely = line(3);
    assert_eq!(lonely.pos, Vec2::new(900.0, 300.0), "a cut out of reach is another limb's");
    assert_eq!((lonely.length, lonely.depth), (30.0, None));
}
