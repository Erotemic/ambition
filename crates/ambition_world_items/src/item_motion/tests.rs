//! What the two authored plans actually do to a pickup, stepped against real
//! geometry rather than asserted about the numbers that were written down.

use super::*;
use ambition_characters::equipment::{EquipmentRow, OnHit};

fn row() -> EquipmentRow {
    EquipmentRow {
        id: "token".into(),
        modifiers: Vec::new(),
        grants: Vec::new(),
        on_hit: Some(OnHit::ConsumeAsArmor { downgrade_to: None }),
        exclusive_slot: None,
    }
}

/// A floor across the bottom with a wall at each end — a real corridor, so a
/// bouncer has something to ricochet BETWEEN. (With only the right wall it
/// turned, walked off the left end of the floor, and fell forever, which is
/// correct behaviour and a useless fixture.)
fn corridor() -> ae::World {
    ae::World::new(
        String::from("corridor"),
        ae::Vec2::new(1000.0, 1000.0),
        ae::Vec2::new(50.0, 50.0),
        vec![
            ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 500.0),
                ae::Vec2::new(1000.0, 100.0),
            ),
            ae::Block::solid(
                "wall_right",
                ae::Vec2::new(400.0, 300.0),
                ae::Vec2::new(40.0, 200.0),
            ),
            ae::Block::solid(
                "wall_left",
                ae::Vec2::new(100.0, 300.0),
                ae::Vec2::new(40.0, 200.0),
            ),
        ],
    )
}

/// Drive the SAME stepping the system runs, so these exercise the resolve
/// rather than a re-implementation of it.
fn run(plan: ItemMotionPlan, ticks: usize) -> (WorldItem, ItemMotion) {
    let world = corridor();
    let mut item = WorldItem::equipping(
        row(),
        ae::Vec2::new(200.0, 480.0),
        ae::Vec2::new(12.0, 12.0),
    );
    let mut motion = ItemMotion::new(plan);
    for _ in 0..ticks {
        step_one_item(&world, &mut item, &mut motion, 1.0 / 60.0);
    }
    (item, motion)
}

#[test]
fn a_pickup_with_no_plan_is_exactly_where_it_was_put() {
    let (item, _) = run(ItemMotionPlan::still(), 120);
    assert_eq!(
        item.pos,
        ae::Vec2::new(200.0, 480.0),
        "a still plan does not move it"
    );
}

#[test]
fn the_rise_lifts_it_clear_and_then_hands_over_to_travel() {
    let plan = ItemMotionPlan::walker(60.0).emerging(32.0, 0.4);
    let (item, motion) = run(plan, 12);
    assert!(
        motion.emerging(),
        "still climbing out at 0.2s of a 0.4s rise"
    );
    assert!(item.pos.y < 480.0, "and it has risen (screen up is -y)");
    assert_eq!(item.pos.x, 200.0, "a rising pickup does not travel yet");

    let (item, motion) = run(plan, 60);
    assert!(!motion.emerging(), "the rise is over");
    assert!(item.pos.x > 200.0, "and it set off: {:?}", item.pos);
}

/// A rise that is NOT a whole number of ticks still rises exactly as far as it
/// was authored to.
///
/// 0.13s against a 1/60s step is 7.8 ticks, and the eighth has only 0.0133s of rise left in it.
/// Spending a full tick's fraction there — which is what the code did — carried the pickup 32.8px
/// out of a block that authored 32, and it then started travelling from a height nobody wrote down.
#[test]
fn a_rise_the_timestep_does_not_divide_still_stops_at_the_authored_height() {
    let plan = ItemMotionPlan::walker(60.0).emerging(32.0, 0.13);
    let (item, motion) = run(plan, 8);
    assert!(!motion.emerging(), "0.13s is spent after eight 1/60s ticks");
    assert!(
        (item.pos.y - (480.0 - 32.0)).abs() < 1e-3,
        "the rise must end exactly 32px up, got {:?} (was 480 - 32.82 before the clamp)",
        item.pos,
    );
}

#[test]
fn a_walker_turns_around_at_a_wall_instead_of_pressing_into_it() {
    let (item, motion) = run(ItemMotionPlan::walker(60.0), 60 * 5);
    assert_eq!(motion.facing(), -1.0, "it turned at the wall");
    assert!(item.pos.x < 380.0, "and is heading back: {:?}", item.pos);
}

#[test]
fn a_walker_settles_onto_the_floor_and_a_bouncer_does_not() {
    let (_, walker) = run(ItemMotionPlan::walker(0.0), 60 * 2);
    assert_eq!(
        walker.velocity().y,
        0.0,
        "a walker gives nothing back and lies on the ground"
    );

    let (_, bouncer) = run(ItemMotionPlan::bouncer(0.0, 0.8), 60 * 2);
    assert!(
        bouncer.velocity().y.abs() > 1.0,
        "a bouncer is still moving vertically two seconds in: {:?}",
        bouncer.velocity()
    );
}

#[test]
fn a_bouncer_stays_inside_the_level_it_is_ricocheting_around() {
    let (item, _) = run(ItemMotionPlan::bouncer(90.0, 0.8), 60 * 8);
    assert!(
        item.pos.y <= 488.001,
        "it never sinks through the floor it bounces on: {:?}",
        item.pos
    );
    assert!(
        (152.0..=388.001).contains(&item.pos.x),
        "and stays between the two walls it ricochets off: {:?}",
        item.pos
    );
}
