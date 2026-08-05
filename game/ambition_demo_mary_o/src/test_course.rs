//! **A level for the TESTS to play, which nobody authors.**
//!
//! Jon, 2026-08-04: *"the plays level1 test might get stale awfully quick if I
//! modify level 1. We should probably ensure there is a fixture for the test
//! that won't change as we modify the level itself."*
//!
//! ⛔ **He was right, and it had already happened.** Two scripted playthroughs
//! walked a ROUTE through 1-1 — stand here, jump now, *"the wand needs ~2.9 s to
//! reach the pit"* — and every one of those numbers was measured against one
//! arrangement of the level. Moving the enemies into the LDtk file was enough to
//! desynchronise both. They are `#[ignore]`d, and this is what replaces them.
//!
//! ## Why it is DELIBERATELY boring
//!
//! Flat ground the whole way, one ?-block, one snake, a flag. No pits, no
//! stairs, no stepping stones, no vault.
//!
//! ⭐ **that is the design, not a shortcut.** What made the old route brittle was
//! not the level being complicated — it was the route needing to be *timed*: a
//! jump that must clear a pit is a jump whose distance, speed and launch frame
//! all have to stay true. A course with no timing has no numbers to go stale.
//! What it still exercises is every SEAM the playthrough existed for: she walks,
//! she bonks a reactive block and takes what pops out, she stomps an enemy, she
//! reaches the goal.
//!
//! ⚠ **it is Rust-built on purpose**, which is the opposite of the rule for a
//! real level. 1-1 is authored because Jon has to be able to change it; this is
//! Rust because he must NOT change it by accident. A fixture that lives in the
//! editor is a fixture one drag away from being a different test.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::world::rooms::RoomSpec;

use crate::ldtk_vocabulary::{reactive_block, MaryOBlockKind};

/// Room id of the fixture course.
pub const TEST_COURSE_ROOM_ID: &str = "mary_o_test_course";

/// One tile, same as the real level so her tuning reads the same.
const T: f32 = 32.0;
/// How wide the course is, in tiles. Long enough for a walk to mean something.
const WIDTH_TILES: f32 = 40.0;
/// Ground thickness.
const GROUND_TILES: f32 = 2.0;
/// The surface height, matching 1-1 so gravity and jump arcs behave identically.
const SURFACE_HEIGHT: f32 = 15.0 * T;

/// Tile column of the one ?-block.
pub const COURSE_BLOCK_COLUMN: f32 = 8.0;
/// Tile column the one snake patrols from.
pub const COURSE_SNAKE_COLUMN: f32 = 20.0;
/// Tile column of the goal.
pub const COURSE_POLE_COLUMN: f32 = 34.0;

fn ground_top() -> f32 {
    SURFACE_HEIGHT - GROUND_TILES * T
}

/// The one ?-block's box — exposed so a test can aim at it without restating it.
pub fn course_block_aabb() -> ae::Aabb {
    let min = ae::Vec2::new(COURSE_BLOCK_COLUMN * T, ground_top() - 4.0 * T);
    ae::Aabb::new(min + ae::Vec2::splat(T * 0.5), ae::Vec2::splat(T * 0.5))
}

/// Where the goal stands.
pub fn course_pole_x() -> f32 {
    COURSE_POLE_COLUMN * T
}

/// The fixture course.
pub fn test_course() -> RoomSpec {
    let ground_top = ground_top();
    let mut blocks = vec![
        // Unbroken ground: nothing to fall into, so no jump has to be timed.
        ae::Block::solid_tiled(
            "course_ground",
            ae::Vec2::new(0.0, ground_top),
            ae::Vec2::new(WIDTH_TILES * T, GROUND_TILES * T),
            "mary_o_course",
            0,
        ),
        // A wall at each end, so a runaway script cannot walk out of the world
        // and turn a routing bug into a fall death.
        ae::Block::solid_tiled(
            "course_wall_left",
            ae::Vec2::new(-T, ground_top - 8.0 * T),
            ae::Vec2::new(T, 10.0 * T),
            "mary_o_course",
            1,
        ),
        ae::Block::solid_tiled(
            "course_wall_right",
            ae::Vec2::new(WIDTH_TILES * T, ground_top - 8.0 * T),
            ae::Vec2::new(T, 10.0 * T),
            "mary_o_course",
            2,
        ),
    ];
    // ONE ?-block, at the same bonk height 1-1 uses, built exactly the way the
    // LDtk converter builds one — encoded name and durable placement id — so the
    // runtime recognises it through the same path a real level uses.
    let block = course_block_aabb();
    blocks.push(reactive_block(
        MaryOBlockKind::Power,
        "course_power_block",
        block.min,
        block.max - block.min,
    ));

    let spawn = ae::Vec2::new(2.0 * T, ground_top - 2.0 * T);
    let world = ae::World::new(
        "Mary-O test course",
        ae::Vec2::new(WIDTH_TILES * T, SURFACE_HEIGHT),
        spawn,
        blocks,
    );
    let mut room = RoomSpec::new(TEST_COURSE_ROOM_ID, world);
    room.metadata.mode = Some(crate::MARY_O_MODE.to_string());

    // ONE snake, authored the way the level authors its enemies so the staging
    // reads it through the same filter.
    room.enemy_spawns
        .push(ambition_platformer2d::world::rooms::Authored::new(
            "course_snake",
            crate::snake::SNAKE_DISPLAY_NAME,
            ae::Aabb::new(
                ae::Vec2::new(COURSE_SNAKE_COLUMN * T, ground_top - T),
                ae::Vec2::new(14.0, 16.0),
            ),
            ambition_platformer2d::entity_catalog::placements::CharacterBrain::Custom(
                crate::snake::SNAKE_BRAIN_KEY.to_string(),
            ),
        ));
    room
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The course has the three things a playthrough needs, and no timing.
    ///
    /// ⚠ this is not ceremony: the whole value of the fixture is that it stays
    /// simple. A pit added here — by anyone, for any reason — puts the timing
    /// back and re-arms exactly the staleness it exists to prevent.
    #[test]
    fn the_course_stays_simple_enough_to_have_no_timing() {
        let room = test_course();
        let ground = room
            .world
            .blocks
            .iter()
            .find(|b| b.name == "course_ground")
            .expect("the course has ground");
        assert_eq!(
            ground.aabb.min.x, 0.0,
            "the ground starts at the left edge — a gap is a jump that has to be timed"
        );
        assert_eq!(
            ground.aabb.max.x,
            WIDTH_TILES * T,
            "…and runs unbroken to the right edge"
        );
        assert!(
            crate::ldtk_vocabulary::block_kind_of(
                &crate::ldtk_vocabulary::encoded_name(MaryOBlockKind::Power, "course_power_block")
            ) == Some(MaryOBlockKind::Power),
            "the ?-block is recognised through the same vocabulary a real level uses"
        );
        assert_eq!(room.enemy_spawns.len(), 1, "one enemy, so a stomp is unambiguous");
        assert!(
            course_pole_x() > COURSE_SNAKE_COLUMN * T,
            "the goal is past the enemy, so reaching it means getting through"
        );
    }
}
