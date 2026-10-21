//! The link is the move: two apertures on unrelated channels go nowhere, and
//! the recovery looks like it failed. The recovery comes from its module on
//! the module road.

use bevy::prelude::*;

use crate::module_road::{app_on, runner_of, Road};
use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::smash_portal::{PortalPairParams, PORTAL_PAIR};
use ambition_platformer2d::extension::sdk::phases;
use ambition_platformer2d::extension::ExtensionSet;
use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
use ambition_platformer2d::platformer::sim_id::SimId;
use ambition_platformer2d::portal::{PlacedPortal, PortalChannel};

const DT: f32 = 1.0 / 60.0;

fn params(lifetime_s: f32) -> PortalPairParams {
    PortalPairParams {
        rise: 320.0,
        half_extent: (26.0, 6.0),
        lifetime_s,
        close_on_transit: false,
        tilt_degrees: 0.0,
        aim_tilt_degrees: 0.0,
        channel_index: 8,
    }
}

fn app_with(road: Road) -> App {
    let mut app = app_on(road, ambition_demo_smash_modules::portal::module);
    app.add_systems(
        Update,
        ambition_platformer2d::portal::resolve_portal_links.after(ExtensionSet::Lower(phases::TECHNIQUE_EXECUTION)),
    );
    app.world_mut().spawn((RoomInstanceRoot, LiveRoomInstance::ACTIVATION));
    app
}

fn app() -> App {
    app_with(Road::Linked)
}

fn fighter(app: &mut App, slot: u8, at: ae::Vec2) -> Entity {
    app.world_mut()
        .spawn((
            ae::BodyKinematics { pos: at, ..Default::default() },
            SimId::player_slot(slot),
            InRoomInstance(LiveRoomInstance::ACTIVATION),
        ))
        .id()
}

/// Press the recovery as `actor`; the press's tick has run.
fn open(app: &mut App, actor: Entity, params: &PortalPairParams) {
    let request = ActionRequest::Special {
        spec: SpecialActionSpec::Special(PORTAL_PAIR.to_string()),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(params).expect("portal params serialize"),
    };
    app.world_mut().write_message(ActorActionMessage { actor, request, move_instance: None });
    app.update();
}

/// Each open aperture: its channel, centre, normal and depth, the higher
/// (the exit) first.
fn placed(app: &mut App) -> Vec<(PortalChannel, ae::Vec2, ae::Vec2, f32)> {
    let mut out: Vec<_> = app
        .world_mut()
        .query::<&PlacedPortal>()
        .iter(app.world())
        .map(|portal| (portal.channel, portal.pos, portal.normal, portal.half_thickness))
        .collect();
    out.sort_by(|a, b| a.1.y.total_cmp(&b.1.y).then(a.1.x.total_cmp(&b.1.x)));
    out
}

/// The move opens a LINKED pair: the entrance at her facing up, the exit
/// `rise` above facing down, as thin as authored.
#[test]
fn the_move_opens_a_linked_pair_with_the_exit_above() {
    let mut app = app();
    let her = fighter(&mut app, 0, ae::Vec2::new(100.0, 200.0));
    open(&mut app, her, &params(5.0));
    app.update();
    let open = placed(&mut app);
    assert_eq!(open.len(), 2, "not one pair: {open:?}");
    let (exit, entrance) = (open[0], open[1]);
    assert_eq!((entrance.1, exit.1), (ae::Vec2::new(100.0, 200.0), ae::Vec2::new(100.0, -120.0)));
    assert!(entrance.2.y < 0.0 && exit.2.y > 0.0, "the normals do not face each other: {open:?}");
    assert_eq!(entrance.0.partner(), exit.0, "the two ends are not a pair: {open:?}");
    assert!(open.iter().all(|end| end.3 == 6.0), "the authored thickness was not kept: {open:?}");
}

/// Two fighters recovering at once open four apertures in two pairs that
/// cannot see each other, and one pair closing leaves the other open.
#[test]
fn two_fighters_recovering_at_once_get_pairs_that_cannot_see_each_other() {
    let mut app = app();
    let first = fighter(&mut app, 0, ae::Vec2::new(100.0, 200.0));
    let second = fighter(&mut app, 1, ae::Vec2::new(600.0, 200.0));
    open(&mut app, first, &params(0.25));
    open(&mut app, second, &params(5.0));
    app.update();
    let open = placed(&mut app);
    assert_eq!(open.len(), 4, "{open:?}");
    let pair_of = |x: f32| -> Vec<PortalChannel> { open.iter().filter(|end| end.1.x == x).map(|end| end.0).collect() };
    let (a, b) = (pair_of(100.0), pair_of(600.0));
    assert_eq!((a[0].partner(), b[0].partner()), (a[1], b[1]), "a fighter's ends are not a pair: {open:?}");
    assert!(b.iter().all(|end| !a.contains(&end.partner())), "the two pairs see each other: {open:?}");
    for _ in 0..(0.25 / DT) as usize + 2 {
        app.update();
    }
    let left = placed(&mut app);
    assert!(left.len() == 2 && left.iter().all(|end| end.1.x == 600.0), "the wrong pair closed: {left:?}");
}

/// One fighter, one pair: a second opening takes the first's place on its
/// own tick, so she never has two pairs at once.
#[test]
fn a_second_cast_by_one_fighter_retires_its_first_pair() {
    let mut app = app();
    let her = fighter(&mut app, 0, ae::Vec2::new(100.0, 200.0));
    open(&mut app, her, &params(5.0));
    app.update();
    app.world_mut().get_mut::<ae::BodyKinematics>(her).unwrap().pos = ae::Vec2::new(400.0, 200.0);
    open(&mut app, her, &params(5.0));
    let open = placed(&mut app);
    assert!(open.len() == 2 && open.iter().all(|end| end.1.x == 400.0), "the first pair survived: {open:?}");
}

/// When the clock runs out, both ends close.
#[test]
fn an_expired_pair_closes_at_both_ends() {
    let mut app = app();
    let her = fighter(&mut app, 0, ae::Vec2::new(100.0, 200.0));
    open(&mut app, her, &params(0.1));
    assert_eq!(placed(&mut app).len(), 2);
    for _ in 0..(0.1 / DT) as usize + 2 {
        app.update();
    }
    assert!(placed(&mut app).is_empty(), "an expired pair stayed open: {:?}", placed(&mut app));
}

fn came_out_at(app: &mut App, body: Entity, at: ae::Vec2) {
    app.world_mut().write_message(ambition_platformer2d::portal::PortalBodyTransited {
        body,
        enter_normal: ae::Vec2::new(0.0, -1.0),
        exit_normal: ae::Vec2::new(0.0, 1.0),
        facing_flip: false,
        input_warp: false,
        exit_pos: at,
    });
    app.update();
}

/// The one-shot pair closes behind whoever went through it; an ordinary
/// pair survives being used; a use of another portal leaves it alone.
#[test]
fn a_one_shot_pair_closes_behind_whoever_used_it_and_an_ordinary_one_does_not() {
    let exit = ae::Vec2::new(100.0, -120.0);
    for (close_on_transit, at, open_after) in
        [(true, exit, 0), (false, exit, 2), (true, ae::Vec2::new(900.0, 900.0), 2)]
    {
        let mut app = app();
        let her = fighter(&mut app, 0, ae::Vec2::new(100.0, 200.0));
        open(&mut app, her, &PortalPairParams { close_on_transit, ..params(5.0) });
        came_out_at(&mut app, her, at);
        app.update();
        assert_eq!(placed(&mut app).len(), open_after, "close_on_transit {close_on_transit}, out at {at:?}");
    }
}

/// The exit of a pair opened with `stick` latched by the move, and an aim
/// range of `aim_tilt_degrees`.
fn aimed_exit(aim_tilt_degrees: f32, stick: Option<ae::Vec2>) -> ae::Vec2 {
    let mut app = app();
    let her = fighter(&mut app, 0, ae::Vec2::ZERO);
    // The latch, not a live stick: an aimed special is rooted, so a live
    // read is neutral.
    if let Some(stick) = stick {
        let spec = ambition_platformer2d::entity_catalog::MoveSpec {
            id: "aim_fixture".to_string(),
            display_name: None,
            clip: ambition_platformer2d::entity_catalog::ClipBinding {
                clip: "special_up".to_string(),
                fallbacks: Vec::new(),
            },
            duration_s: 1.0,
            windows: Vec::new(),
            events: Vec::new(),
            gates: Default::default(),
            start_impulse: None,
            charge_mult: 1.0,
            move_charge: None,
            charge_gesture: Default::default(),
            repeat: None,
            landing_lag_s: None,
            autocancel_after_s: None,
            sprite_spin_hz: None,
            equips: None,
            flow: None,
        };
        app.world_mut().entity_mut(her).insert(
            ambition_platformer2d::combat::moveset::MovePlayback::new(spec, 1.0).with_aimed_stick(Some(stick)),
        );
    }
    open(&mut app, her, &PortalPairParams { aim_tilt_degrees, ..params(5.0) });
    placed(&mut app)[0].1
}

/// The player's direction angles the shaft: right leans the exit right by
/// the aim range, left leans it left, and a neutral stick goes straight up.
/// A pair with no aim range ignores the stick.
#[test]
fn the_players_direction_angles_the_shaft_and_neutral_goes_straight_up() {
    assert!(aimed_exit(32.0, None).x.abs() < 1e-3, "a neutral stick leaned the recovery");
    let right = aimed_exit(32.0, Some(ae::Vec2::new(1.0, 0.0)));
    assert!((right.x - 320.0 * 32f32.to_radians().sin()).abs() < 1e-2, "right leaned it to {right:?}");
    assert!(aimed_exit(32.0, Some(ae::Vec2::new(-1.0, 0.0))).x < 0.0, "left did not lean it left");
    assert!(aimed_exit(0.0, Some(ae::Vec2::new(1.0, 0.0))).x.abs() < 1e-3, "a pair with no aim range moved");
}

/// The recovery built from its module file opens and closes as the linked
/// one does.
#[test]
fn the_recovery_from_its_module_file_opens_as_the_linked_one_does() {
    let run = |road: Road| {
        let mut app = app_with(road);
        let her = fighter(&mut app, 0, ae::Vec2::new(100.0, 200.0));
        open(&mut app, her, &params(0.1));
        let mut seen = vec![placed(&mut app).iter().map(|end| end.1).collect::<Vec<_>>()];
        for _ in 0..10 {
            app.update();
            seen.push(placed(&mut app).iter().map(|end| end.1).collect());
        }
        (seen, runner_of(&app, PORTAL_PAIR))
    };
    let (linked_seen, linked) = run(Road::Linked);
    let (loaded_seen, loaded) = run(Road::Loaded);
    assert!(loaded.starts_with("Loaded") && !linked.starts_with("Loaded"), "{linked} / {loaded}");
    assert_eq!(loaded_seen, linked_seen);
    assert!(linked_seen[0].len() == 2 && linked_seen.last().unwrap().is_empty(), "{linked_seen:?}");
}
