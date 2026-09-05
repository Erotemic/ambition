//! ⛔⛔ THE OWNERSHIP CLAIM IS THE ONE WORTH GUARDING. "A mine answers its
//! placer" is the whole difference between this and the bomb, and a suite that
//! only proved place-then-detonate would pass against a mine ANY press sets off
//! — which is not a mine, it is a shared button.

use super::*;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::item::{GroundItem, ItemCustody};
use ambition_platformer2d::vfx::{Effect, EffectRequest};

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<ambition_platformer2d::time::WorldTime>();
    app.add_message::<EffectRequest>();
    app.add_message::<ActorActionMessage>();
    let mut time = app
        .world_mut()
        .resource_mut::<ambition_platformer2d::time::WorldTime>();
    time.scaled_dt = 1.0 / 60.0;
    time.raw_dt = 1.0 / 60.0;
    // The shipped order: see the plugin. Arm before press.
    app.add_systems(
        Update,
        (arm_placed_mines, place_or_detonate_authored_mines).chain(),
    );
    app
}

/// A fighter who can place mines, at `x`.
fn a_fighter(app: &mut App, seat: usize, x: f32) -> Entity {
    app.world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: ae::Vec2::new(x, 0.0),
                ..Default::default()
            },
            MatchSeat(seat),
        ))
        .id()
}

fn params() -> PlaceMineParams {
    PlaceMineParams {
        item_id: "polygon_mine".to_string(),
        arm_s: 1.2,
        damage: 10,
        blast_radius: 52.0,
        half_extents: (8.0, 8.0),
        offset: (-18.0, 14.0),
    }
}

/// Press the mine button as `actor`.
fn press(app: &mut App, actor: Entity) {
    let request = ActionRequest::Special {
        spec: SpecialActionSpec::Special(PLACE_MINE.to_string()),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params())
            .expect("place-mine params serialize"),
    };
    app.world_mut()
        .write_message(ActorActionMessage { actor, request });
    app.update();
}

/// Every mine on the stage, with its owner and clock.
fn mines(app: &mut App) -> Vec<(usize, f32)> {
    app.world_mut()
        .query::<&PlacedMine>()
        .iter(app.world())
        .map(|mine| (mine.owner_seat, mine.arm_s))
        .collect()
}

/// Blasts written this frame, with their centres.
fn blasts(app: &mut App) -> Vec<ae::Vec2> {
    let messages = app.world().resource::<Messages<EffectRequest>>();
    let mut cursor = messages.get_cursor();
    cursor
        .read(messages)
        .filter_map(|request| match &request.effect {
            Effect::DamageBox(box_effect) if box_effect.name == Some("mine blast") => {
                Some(box_effect.center)
            }
            _ => None,
        })
        .collect()
}

/// Spend `secs` of arming without pressing anything.
fn wait(app: &mut App, secs: f32) {
    for _ in 0..((secs * 60.0).ceil() as usize) {
        app.update();
    }
}

#[test]
fn a_press_with_nothing_out_places_a_mine_owned_by_the_presser() {
    let mut app = app();
    let fighter = a_fighter(&mut app, 1, 100.0);
    press(&mut app, fighter);
    assert_eq!(mines(&mut app), vec![(1, 1.2)]);
    assert!(blasts(&mut app).is_empty(), "placing is not detonating");
}

#[test]
fn a_press_while_the_mine_is_still_arming_does_nothing_at_all() {
    let mut app = app();
    let fighter = a_fighter(&mut app, 1, 100.0);
    press(&mut app, fighter);
    wait(&mut app, 0.5);
    press(&mut app, fighter);
    // ⛔ ONE mine, not two, and no blast. Both halves matter: the failure this
    // guards against is a second placement (mine spam) AND a detonation that
    // ignores the arming delay, and they are opposite bugs in the same branch.
    let out = mines(&mut app);
    assert_eq!(out.len(), 1, "a second press must not plant a second mine");
    assert!(out[0].1 > 0.0, "still arming");
    assert!(blasts(&mut app).is_empty(), "an unarmed mine does not answer");
}

#[test]
fn a_press_once_armed_detonates_the_mine_and_takes_it_off_the_stage() {
    let mut app = app();
    let fighter = a_fighter(&mut app, 1, 100.0);
    press(&mut app, fighter);
    wait(&mut app, 1.3);
    press(&mut app, fighter);
    assert_eq!(blasts(&mut app).len(), 1, "the armed mine answered");
    assert!(mines(&mut app).is_empty(), "and it is gone");
}

#[test]
fn a_mine_answers_its_own_seat_and_nobody_elses() {
    let mut app = app();
    let owner = a_fighter(&mut app, 1, 100.0);
    let rival = a_fighter(&mut app, 0, -100.0);
    press(&mut app, owner);
    wait(&mut app, 1.3);
    // The rival presses their OWN mine button while the owner's mine is armed
    // and sitting there.
    press(&mut app, rival);
    assert!(
        blasts(&mut app).is_empty(),
        "seat 0 must not be able to set off seat 1's mine"
    );
    // ⭐ AND THE RIVAL GOT THEIR OWN MINE INSTEAD, which is the other half of
    // the same claim: the press was not swallowed, it was scoped.
    let mut out = mines(&mut app);
    out.sort_by_key(|(seat, _)| *seat);
    assert_eq!(out.len(), 2, "two seats, two mines");
    assert_eq!(out[0].0, 0);
    assert_eq!(out[1].0, 1);
}

#[test]
fn the_blast_lands_where_the_mine_is_now_not_where_it_was_planted() {
    let mut app = app();
    let owner = a_fighter(&mut app, 1, 100.0);
    press(&mut app, owner);
    wait(&mut app, 1.3);
    // Somebody picked it up and ran. `GroundItem::pos` is the world's stale
    // copy; `ItemWorldPos` is the live answer, and the mine must use it.
    let thief = a_fighter(&mut app, 0, -400.0);
    let mine = app
        .world_mut()
        .query_filtered::<Entity, With<PlacedMine>>()
        .iter(app.world())
        .next()
        .expect("a mine is out");
    app.world_mut()
        .entity_mut(mine)
        .insert(ItemCustody::Held { holder: thief });
    press(&mut app, owner);
    let centres = blasts(&mut app);
    assert_eq!(centres.len(), 1);
    assert!(
        centres[0].x < -300.0,
        "the blast followed the carrier to {:?}, not the planting spot",
        centres[0]
    );
}

#[test]
fn a_placer_with_no_seat_places_nothing() {
    let mut app = app();
    // ⛔ NO `MatchSeat`. A mine whose owner cannot be named could never be
    // detonated, so it must not reach the stage at all.
    let unseated = app
        .world_mut()
        .spawn(ae::BodyKinematics::default())
        .id();
    press(&mut app, unseated);
    assert!(mines(&mut app).is_empty());
}

#[test]
fn the_placed_mine_is_a_ground_item_somebody_could_pick_up() {
    let mut app = app();
    let fighter = a_fighter(&mut app, 1, 100.0);
    press(&mut app, fighter);
    let mut query = app.world_mut().query::<(&GroundItem, &ItemCustody)>();
    let (item, custody) = query.iter(app.world()).next().expect("the mine is an item");
    assert_eq!(item.spec.id.as_str(), "polygon_mine");
    assert_eq!(*custody, ItemCustody::InWorld);
    // Body-local offset, mirrored by facing: planted behind her, not inside her.
    assert!(item.pos.x < 100.0, "planted behind the placer");
}
