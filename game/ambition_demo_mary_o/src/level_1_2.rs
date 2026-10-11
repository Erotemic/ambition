//! World 1-2: the level under the ground.
//!
//! The level is authored in the Mary-O LDtk world. Its generator is
//! `tools/author_mary_o_1_2_ldtk.py`, which builds it in tiles, one function for
//! each part. Left to right:
//!
//! 1. The arrival: a row of blocks, two walkers, and steps of rock with a
//!    walker in a hollow.
//! 2. The shell alley: a snake in front of a line of walkers under a band of
//!    bricks. The top of the band is a second road with coins on it.
//! 3. The brick maze: a mass of bricks with a tunnel of coins through it, and
//!    enemies under it.
//! 4. The teeth and the pits: three pits under rock that hangs low, with planes
//!    in the air. The low rock makes each jump a short one.
//! 5. The lifts: two lines of platforms over a pit, one down and one up. The
//!    lift that goes up goes on through a hole in the roof.
//! 6. The attic: a room of coins over the roof. Its pipe comes out before the
//!    ferry, so it is a way past the clock gallery.
//! 7. The clock gallery: a high road of ledges over a low road.
//! 8. The ferry: a pit wider than a jump at a run. Only the ferry crosses it.
//! 9. The goal: steps of rock and the pole. The level that comes next is the
//!    `next_room` field of this level.
//!
//! The roof has no hole in its top row, so the level reads as under the ground.
//! A `CameraZone` holds the camera on the corridor: the room is as high as the
//! corridor and the attic, and the view must not show the attic from below.
//!
//! A platform is an ordinary `MovingPlatform` entity. To ride it is engine
//! behavior, so this level adds no movement code.

use ambition_platformer2d::world::rooms::RoomSpec;

/// The authored area id, and the room id the runtime knows it by.
pub const LEVEL_1_2_ROOM_ID: &str = "mary_o_1_2";

/// The authored `id` of the ferry. The `name` of a platform is presentation:
/// a lookup by it stops when the platform gets a new name in the editor.
pub const FERRY_ID: &str = "mary_o_1_2_ferry";

/// 1-2's goal.
pub fn goal_pole() -> crate::flag::FlagPole {
    crate::authored_pole(&level_1_2())
}

pub fn level_1_2() -> RoomSpec {
    crate::authored_level(LEVEL_1_2_ROOM_ID)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::engine_core as ae;
    use ambition_platformer2d::engine_core::AabbExt;

    /// One authored tile; this is a grid unit, not a placement coordinate.
    const T: f32 = crate::T;

    /// IntGrid cell size: the smallest authored hole. Sweeps step at this
    /// resolution so no cell can be skipped by a coarser tile-sized probe.
    const CELL: f32 = T * 0.5;

    /// Is anything solid in this cell?
    fn solid_in_cell(room: &RoomSpec, min: ae::Vec2) -> bool {
        let probe = ae::Aabb::new(
            min + ae::Vec2::splat(CELL * 0.5),
            ae::Vec2::splat(CELL * 0.25),
        );
        room.world
            .blocks
            .iter()
            .any(|block| block.aabb.strict_intersects(probe))
    }

    /// The run of floor columns with nothing in them that `x` is over. The
    /// level says where its pit is: a pit that moves in the editor moves this
    /// answer with it.
    fn pit_under(room: &RoomSpec, x: f32) -> (f32, f32) {
        let depth = room.world.size.y - CELL;
        let open = |column: f32| !solid_in_cell(room, ae::Vec2::new(column, depth));
        let mut start = (x / CELL).floor() * CELL;
        assert!(open(start), "no pit is under x={x}");
        while start > 0.0 && open(start - CELL) {
            start -= CELL;
        }
        let mut end = start;
        while end < room.world.size.x && open(end) {
            end += CELL;
        }
        (start, end)
    }

    /// A ferry that stops short of a lip of its pit makes the level impossible
    /// to finish: the pit is wider than a jump.
    #[test]
    fn the_ferry_goes_from_one_lip_of_its_pit_to_the_other() {
        let room = level_1_2();
        let mut ferry = room
            .moving_platforms
            .iter()
            .find(|platform| platform.id == FERRY_ID)
            .cloned()
            .expect("1-2 authors its ferry");
        let (start, end) = pit_under(&room, ferry.pos.x);

        // Nothing stands in the pit at the floor: no stepping stone, no floor.
        let gap = ae::Aabb::new(
            ae::Vec2::new((start + end) * 0.5, room.world.size.y - 2.0 * T),
            ae::Vec2::new((end - start) * 0.5 - 1.0, T),
        );
        assert!(
            !room
                .world
                .blocks
                .iter()
                .any(|b| b.aabb.strict_intersects(gap)),
            "the pit of the ferry has something standing in it",
        );

        let (mut left, mut right) = (f32::MAX, f32::MIN);
        for _ in 0..1200 {
            left = left.min(ferry.pos.x - ferry.size.x * 0.5);
            right = right.max(ferry.pos.x + ferry.size.x * 0.5);
            ferry.update(1.0 / 60.0);
        }
        assert!(
            left <= start + 1.0,
            "the ferry never reaches the near lip: {left} vs {start}"
        );
        assert!(
            right >= end - 1.0,
            "the ferry never reaches the far lip: {right} vs {end}"
        );
    }

    /// Nothing authored into 1-2 is buried in the rock.
    ///
    /// Blind placement needs an oracle that is not a screenshot.
    ///
    /// A block or an enemy overlapping terrain is the failure that matters: a
    /// buried ?-block cannot be hit, and an enemy inside the floor is either
    /// stuck or ejected somewhere surprising.
    #[test]
    fn nothing_authored_in_the_cavern_is_buried_in_the_rock() {
        let room = level_1_2();
        let terrain: Vec<_> = room
            .world
            .blocks
            .iter()
            .filter(|block| crate::ldtk_vocabulary::block_of(&block.name).is_none())
            .collect();

        let reactive: Vec<_> = room
            .world
            .blocks
            .iter()
            .filter(|block| crate::ldtk_vocabulary::block_of(&block.name).is_some())
            .collect();
        assert!(
            !reactive.is_empty(),
            "1-2 authors reactive blocks; if it stops, this test checks nothing"
        );
        for block in &reactive {
            for rock in &terrain {
                assert!(
                    !block.aabb.strict_intersects(rock.aabb),
                    "reactive block `{}` at {:?} is inside terrain at {:?}",
                    block.name,
                    block.aabb.min,
                    rock.aabb.min
                );
            }
        }

        assert!(
            !room.enemy_spawns.is_empty(),
            "1-2 authors enemies; if it stops, this test checks nothing"
        );
        for spawn in &room.enemy_spawns {
            for rock in &terrain {
                assert!(
                    !spawn.aabb.strict_intersects(rock.aabb),
                    "enemy `{}` at {:?} is inside terrain at {:?}",
                    spawn.name,
                    spawn.aabb.min,
                    rock.aabb.min
                );
            }
        }
    }

    #[test]
    fn the_room_is_closed_and_underground() {
        let room = level_1_2();
        // A roof over EVERY cell — the difference between an underground level
        // and a pit. The end columns count: a wall is a roof as far as "can she
        // leave through the top" is concerned.
        let mut column = 0.0;
        while column < room.world.size.x {
            assert!(
                solid_in_cell(&room, ae::Vec2::new(column, 0.0)),
                "the cell at x={column} has no roof over it",
            );
            column += CELL;
        }
        // And closed at both ends, or the corridor trails off into space.
        for (label, x) in [("left", 0.0), ("right", room.world.size.x - CELL)] {
            assert!(
                solid_in_cell(&room, ae::Vec2::new(x, room.world.size.y * 0.5)),
                "the {label} end of the corridor is open",
            );
        }
    }

    /// She arrives standing on something.
    ///
    /// It probed the two `LoadingZone`s; there are none, so the old body could only panic in
    /// `authored_zone`.
    ///
    /// 1-2 still has exactly such a place: the flag route arrives at `world.spawn`, so that is what
    /// gets asked now, and the question is the same one.
    #[test]
    fn the_arrival_spawn_stands_on_the_floor() {
        let room = level_1_2();
        let spawn = room.world.spawn;
        // A body materializes with its FEET on the ground it spawns over, so the
        // block under the spawn is the one whose top is at or below it and whose
        // span contains it. Nearest below, then require it to be close.
        let ground = room
            .world
            .blocks
            .iter()
            .filter(|block| {
                block.aabb.min.x <= spawn.x
                    && block.aabb.max.x >= spawn.x
                    && block.aabb.min.y >= spawn.y
            })
            .map(|block| block.aabb.min.y)
            .fold(f32::INFINITY, f32::min);
        assert!(
            ground.is_finite(),
            "1-2's spawn at {spawn:?} has no floor under it at all — she arrives \
             over the pit or over nothing",
        );
        assert!(
            ground - spawn.y <= T * 3.0,
            "1-2's spawn at {spawn:?} is {} above its floor at {ground}; the room \
             drops her in rather than standing her up",
            ground - spawn.y,
        );
    }
}
