//! Hylbert's Hotel, in the Math dimension's town
//! (`tools/ambition_ldtk_tools/specs/math_town/make_specs.py`).
//!
//! Jon (2026-10-30): "hylberts hotel is something we should build tonight".
//! The player walks by keys (`common::walk`) from Euclid Street through the
//! hotel's door into the lobby, east into the corridor of rooms, on east
//! through its far end (which comes back in at its near end: there is always
//! one more room), then west back to the lobby and out to the street.
//!
//! In the ballroom, Monty Carlo's game show (Jon, 2026-10-30: "finish the
//! monty hall game"): the player plays rounds by talking to him, and walks
//! into the closets and to the prize case to see what the rounds did.

use ambition_platformer2d::engine_core as ae;
use bevy::prelude::KeyCode;

use crate::common::walk::{Keys, Press, Walker};
use crate::the_note_travels_from_alice_to_bob::{dialog_active, flag, npc, tap};
use crate::the_pirate_port::{a_session_in, go, go_to, live_room, talk};

const STREET: &str = "math_town_street";
const LOBBY: &str = "math_hotel_lobby";
const CORRIDOR: &str = "math_hotel_corridor";
const LIFT: &str = "math_hotel_lift";

#[test]
fn the_corridor_of_hylberts_hotel_always_has_one_more_room() {
    let mut app = a_session_in(LOBBY);
    go(&mut app, STREET, Some("math_hotel_lobby_door"));
    go(&mut app, LOBBY, Some("math_hotel_lobby_street"));
    go(&mut app, CORRIDOR, Some("math_hotel_lobby_east"));
    // On east through its far end, twice: each time it comes back in at its
    // near end, and it is still the corridor.
    for lap in 0..2 {
        go_to(&mut app, ae::Vec2::new(1540.0, 360.0), "the corridor's far end");
        let mut came_back_in = false;
        {
            let mut keys = Keys::new(&mut app);
            for _ in 0..240 {
                keys.step(Press { axis: 1.0, ..Press::default() });
                let world = keys.world_mut();
                let body = world
                    .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
                    .single(world)
                    .expect("one primary player");
                if world.get::<ae::BodyKinematics>(body).is_some_and(|kin| kin.pos.x < 400.0) {
                    came_back_in = true;
                    break;
                }
            }
        }
        assert!(came_back_in, "lap {lap}: walking east out of the corridor did not bring the player back in at its west end");
        assert_eq!(live_room(&mut app), CORRIDOR, "lap {lap}: the far end led out of the corridor");
    }
    go(&mut app, LOBBY, Some("math_hotel_corridor_west"));
}

/// The lift only goes up: its car carries the rider out of the top of the
/// shaft, the top comes back in at the bottom, and the car is waiting there.
#[test]
fn hylberts_lift_only_goes_up() {
    let mut app = a_session_in(LOBBY);
    go(&mut app, LIFT, Some("math_hotel_lobby_lift"));
    go(&mut app, LOBBY, Some("math_hotel_lift_door"));
    go(&mut app, LIFT, Some("math_hotel_lobby_lift"));
    // Onto the car (its top 68 over the bottom of the 1600-tall shaft).
    {
        use ambition_platformer2d::platformer::markers::PrimaryPlayer;
        use bevy::prelude::With;
        let half = {
            let mut query = app.world_mut().query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
            query.iter(app.world()).next().expect("a primary player").size.y * 0.5
        };
        let mut query = app.world_mut().query_filtered::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel), With<PrimaryPlayer>>();
        let world = app.world_mut();
        let (mut cluster_item, mut motion_model) = query.iter_mut(world).next().expect("a primary player");
        let mut clusters = cluster_item.as_clusters_mut();
        ae::movement::transit_body(&mut motion_model, &mut clusters, ae::Vec2::new(384.0, 1600.0 - 68.0 - half - 1.0), ae::movement::TransitVelocity::Zero);
    }
    // Rising, then low again (it came round), twice.
    let mut laps = 0;
    let mut was_high = false;
    for _ in 0..60 * 40 {
        app.update();
        assert_eq!(live_room(&mut app), LIFT, "the lift let the rider out");
        let world = app.world_mut();
        let y = world
            .query_filtered::<&ae::BodyKinematics, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .expect("one primary player")
            .pos
            .y;
        if y < 400.0 {
            was_high = true;
        } else if was_high && y > 1300.0 {
            laps += 1;
            was_high = false;
            if laps == 2 {
                break;
            }
        }
    }
    assert_eq!(laps, 2, "the rider did not come round the shaft twice");
}

/// The corridor's guests talk: the barber, Furrier and Python Goras, each at
/// a door (`math.yarn`'s `hotel_*`).
#[test]
fn the_guests_of_hylberts_hotel_talk() {
    let mut app = a_session_in(CORRIDOR);
    talk(&mut app, "npc_paradox_barber", "Do you shave the guests?");
    talk(&mut app, "npc_joseph_furrier", "Which room are you in?");
    talk(&mut app, "npc_python_goras", "What's wrong with beans?");
}

// ---- The ballroom's game show (`math.yarn` `hotel_monty_carlo`) ----------

const BALLROOM: &str = "math_hotel_ballroom";
const MONTY: &str = "npc_math_monty";
const PRIZE_CASE: &str = "math_hotel_prize_case";

/// The gate walls that stand in the live room, by id.
fn standing_walls(app: &mut bevy::prelude::App) -> std::collections::BTreeSet<String> {
    ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<ambition_platformer2d::world::FeatureEcsWorldOverlay>(app.world())
        .expect("the live room has a collision overlay")
        .gate_solids
        .iter()
        .filter_map(|solid| solid.name.strip_prefix("gated_lock:").map(str::to_owned))
        .collect()
}

/// The one door of 1, 2 and 3 whose flag `pattern` (`{}` the door) is set.
fn the_door_flagged(app: &bevy::prelude::App, pattern: &str) -> u8 {
    let doors: Vec<u8> = (1..=3).filter(|n| flag(app, &pattern.replace("{}", &n.to_string()))).collect();
    assert_eq!(doors.len(), 1, "exactly one door has `{pattern}`; these do: {doors:?}");
    doors[0]
}

/// Press through the lines until the talk offers choices; answers them.
fn the_choices(app: &mut bevy::prelude::App) -> Vec<String> {
    for _ in 0..40 {
        let offered: Vec<String> = app
            .world()
            .resource::<ambition_platformer2d::dialog::DialogState>()
            .options()
            .iter()
            .map(|choice| choice.label.clone())
            .collect();
        if !offered.is_empty() {
            return offered;
        }
        tap(app, KeyCode::Enter);
    }
    panic!("the talk offered no choices");
}

/// Take the choice `label` with the arrow keys and Enter.
fn choose(app: &mut bevy::prelude::App, label: &str) {
    let offered = the_choices(app);
    let wanted = offered
        .iter()
        .position(|offered| offered == label)
        .unwrap_or_else(|| panic!("the talk does not offer `{label}`; it offers {offered:?}"));
    let selected = |app: &bevy::prelude::App| app.world().resource::<ambition_platformer2d::dialog::DialogState>().selected_option();
    for _ in 0..offered.len() {
        if selected(app) == wanted {
            break;
        }
        tap(app, KeyCode::ArrowDown);
    }
    assert_eq!(selected(app), wanted, "the arrow keys did not reach `{label}`");
    tap(app, KeyCode::Enter);
    for _ in 0..4 {
        app.update();
    }
}

/// One round of the show, as it came out.
#[derive(Debug)]
struct Round {
    pick: u8,
    /// The door Monty opened before the player chose to stay or switch.
    goat: u8,
    car: u8,
    switched: bool,
    won: bool,
}

/// Walk to Monty, play a round: pick door `pick`, then switch or stay.
fn play(app: &mut bevy::prelude::App, pick: u8, switch: bool) -> Round {
    let at = {
        let body = npc(app, MONTY);
        app.world().get::<ae::BodyKinematics>(body).expect("Monty stands on the stage").pos
    };
    go_to(app, at, MONTY);
    tap(app, KeyCode::KeyF);
    for _ in 0..30 {
        if dialog_active(app) {
            break;
        }
        app.update();
    }
    assert!(dialog_active(app), "Interact beside Monty did not open his talk");
    choose(app, "Let's play!");
    choose(app, &format!("Door number {pick}."));
    // He has opened a goat door, and asks.
    let offered = the_choices(app);
    let goat = the_door_flagged(app, "monty_open_{}");
    assert!(flag(app, &format!("monty_goat_{goat}")), "door {goat} was opened on a goat");
    let other = 6 - pick - goat;
    assert_eq!(
        offered[..2],
        [format!("Switch to door number {other}."), format!("Stay with door number {pick}.")],
        "Monty offers the other shut door, or the pick"
    );
    choose(app, &offered[if switch { 0 } else { 1 }]);
    for _ in 0..40 {
        if !dialog_active(app) {
            break;
        }
        tap(app, KeyCode::Enter);
    }
    assert!(!dialog_active(app), "the round did not end");
    for _ in 0..30 {
        app.update();
    }
    assert!((1..=3).all(|n| flag(app, &format!("monty_open_{n}"))), "the show ends with every door open");
    let car = the_door_flagged(app, "monty_car_{}");
    assert_eq!(
        (1..=3).filter(|n| flag(app, &format!("monty_goat_{n}"))).count(),
        2,
        "a goat behind each other door"
    );
    Round { pick, goat, car, switched: switch, won: flag(app, "monty_won") }
}

/// ⭐ THE SHOW IS HONEST. Each round Monty opens a door that is neither the
/// pick nor the car, and so switching wins exactly when the first pick was
/// wrong (2 times in 3) and staying exactly when it was right. The car is
/// not always behind one door, and the prize case is open only after a win.
#[test]
fn monty_opens_a_goat_and_switching_wins_when_the_first_pick_was_wrong() {
    let mut app = a_session_in(LOBBY);
    go(&mut app, BALLROOM, Some("math_hotel_lobby_ballroom"));
    assert!(standing_walls(&mut app).contains(PRIZE_CASE), "the prize case is shut before a show");
    let mut cars = std::collections::BTreeSet::new();
    for (pick, switch) in [(1, true), (2, false), (3, true), (1, false), (2, true), (3, false)] {
        let round = play(&mut app, pick, switch);
        assert_ne!(round.goat, round.pick, "{round:?}: Monty opened the door that was picked");
        assert_ne!(round.goat, round.car, "{round:?}: Monty opened the car's door");
        assert_eq!(round.won, round.switched != (round.pick == round.car), "{round:?}: the round broke the law of the show");
        assert_eq!(
            !standing_walls(&mut app).contains(PRIZE_CASE),
            round.won,
            "{round:?}: the prize case is open exactly after a win"
        );
        cars.insert(round.car);
    }
    assert_eq!(cars.len(), 3, "in six shows the car was behind only {cars:?}");
}

/// The closets show nothing of a show before it ends: a shut curtain, no
/// goat and no car. After a win, the chosen closet has the car and the
/// prize case gives its money; a closet of a goat has a goat.
#[test]
fn a_winner_finds_the_car_and_takes_the_prize() {
    let mut app = a_session_in(LOBBY);
    go(&mut app, BALLROOM, Some("math_hotel_lobby_ballroom"));
    go(&mut app, "math_hotel_door_1", Some("math_hotel_ballroom_door_1"));
    let walls = standing_walls(&mut app);
    assert!(
        walls.contains("math_hotel_door_1_curtain") && !walls.contains("math_hotel_door_1_goat") && !walls.contains("math_hotel_door_1_car"),
        "before a show the closet is a shut curtain, with nothing behind it: {walls:?}"
    );
    go(&mut app, BALLROOM, Some("math_hotel_door_1_door"));

    // Switch until a round is won (2 times in 3; the shows go round in thirds).
    let round = (0..4)
        .map(|_| play(&mut app, 1, true))
        .find(|round| round.won)
        .expect("switching won no show in four");
    let car = round.car;
    let goat = (1..=3).find(|n| *n != car).unwrap();

    // The prize: the chest in the open case.
    let balance = |app: &mut bevy::prelude::App| {
        let world = app.world_mut();
        world
            .query_filtered::<&ambition_platformer2d::characters::actor::BodyWallet, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .map_or(0, |wallet| wallet.balance)
    };
    let before = balance(&mut app);
    go_to(&mut app, ae::Vec2::new(1048.0, 490.0), "the prize case");
    tap(&mut app, KeyCode::KeyF);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(balance(&mut app) - before, 200, "the chest in the prize case gave its money");

    go(&mut app, &format!("math_hotel_door_{car}"), Some(&format!("math_hotel_ballroom_door_{car}")));
    let walls = standing_walls(&mut app);
    assert!(
        !walls.contains(&format!("math_hotel_door_{car}_curtain"))
            && walls.contains(&format!("math_hotel_door_{car}_car"))
            && !walls.contains(&format!("math_hotel_door_{car}_goat")),
        "behind door {car} is the car: {walls:?}"
    );
    go(&mut app, BALLROOM, Some(&format!("math_hotel_door_{car}_door")));
    go(&mut app, &format!("math_hotel_door_{goat}"), Some(&format!("math_hotel_ballroom_door_{goat}")));
    let walls = standing_walls(&mut app);
    assert!(
        walls.contains(&format!("math_hotel_door_{goat}_goat")) && !walls.contains(&format!("math_hotel_door_{goat}_car")),
        "behind door {goat} is a goat: {walls:?}"
    );
}
