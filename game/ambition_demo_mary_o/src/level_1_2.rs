//! **World 1-2** — the underground level, and the demo's first SECOND ROOM.
//!
//! Until 2026-07-25 this could not exist. `RoomTransitionRequested` had exactly
//! one consumer and only `ambition_app` registered it, so a demo host could not
//! change rooms at all — which is why 1-1's coin vault had to be dug into the
//! same `RoomSpec` as the surface rather than being its own room. The
//! transaction is engine-side now (`ambition_platformer2d::runtime::room_transition`), and
//! this level is what proves it: two authored rooms in the demo's own binary,
//! linked both ways, reached by walking.
//!
//! The grammar, left to right:
//!
//! 1. **The drop** — you arrive under the descent shaft at the vault's far end,
//!    in a low stone corridor. The ceiling is solid the whole way, which is what
//!    makes it read as underground rather than as a pit.
//! 2. **The coin shelf** — a short raised run with coins on it, so the first
//!    thing the room teaches is that its ceiling is low enough to matter.
//! 3. **The chasm** — a five-tile gap with no stepping stone. The only way over
//!    is the moving platform, so the room's one new verb is load-bearing exactly
//!    once, the same rule 1-1's stepping stone follows.
//! 4. **The way out** — a walk-in alcove that returns you to 1-1's surface,
//!    past the pits you skipped. Going down is a shortcut, not a detour.
//! 5. **The goal** — a pole short of that alcove, so a body walking the last
//!    stretch meets the END before it meets the exit. Where finishing LEADS is
//!    [`crate::exit_for_room`]'s answer rather than this room's.
//!
//! ⛔ **THE LEVEL IS `assets/worlds/mary_o.ldtk` NOW, not this file (2026-08-05).**
//! It used to build every block from constants here, which meant Jon could lay
//! out one of his two levels and not the other. `mary_o_1_2` is a second AREA in
//! the same file 1-1 lives in — bootstrapped once by
//! `tools/author_mary_o_1_2_ldtk.py`, and edited in LDtk from here on.
//!
//! ⭐ **authoring it bought 1-1's last two coordinates as well as its own.** The
//! descent shaft and the surface return were built in Rust because a
//! `LoadingZone` has to name a `target_room`, and 1-2 was a room no world file
//! contained. Both zones and the LINKS between the rooms are authored now.
//!
//! The platform is authored as an ordinary `MovingPlatform` entity. Riding it is
//! engine behavior — the platform advance runs once per frame before the body
//! tick and the ride/ledge-carry logic reads its delta — so this level adds no
//! movement code of its own.

use ambition_platformer2d::world::rooms::RoomSpec;

use crate::MARY_O_MODE;

/// The authored area id, and the room id the runtime knows it by.
pub const LEVEL_1_2_ROOM_ID: &str = "mary_o_1_2";

/// The zone in 1-1's vault that drops you into 1-2, and its partner in 1-2.
///
/// Both are `Walk`: you step into the shaft and go. The vault's own two pipes
/// stay directional presses (Jon's rule — a pipe answers UP or DOWN, never a
/// generic Interact); this is a different affordance on purpose, an open shaft
/// in the vault floor rather than a third tube competing with them.
///
/// ⚠ **only two of these four name a target.** `DESCENT` and `EXIT` are exits;
/// `ARRIVAL` and `SURFACE_RETURN` are landing pads, and a landing pad that named
/// a target would fire on the body that just arrived on it.
pub const DESCENT_ZONE_ID: &str = "mary_o_1_1_descent";
pub const ARRIVAL_ZONE_ID: &str = "mary_o_1_2_arrival";
/// The way back up: 1-2's exit alcove, and where it puts you on the surface.
pub const EXIT_ZONE_ID: &str = "mary_o_1_2_exit";
pub const SURFACE_RETURN_ZONE_ID: &str = "mary_o_1_1_surface_return";

/// **The ferry's authored ID.**
///
/// ⛔ **this used to be its display NAME, because there was no id to use.**
/// `convert_moving_platform` went straight to the LDtk iid — a value the file
/// mints, which nothing outside the file can spell — so the only handle a reader
/// had was `name`, and a name is presentation: `FeatureName`'s own doc calls it
/// *"human-facing … for debug overlays / inspectors"*. Renaming the platform in
/// the editor would have silently broken every lookup, which is the same defect
/// the snake paid for twice.
///
/// The converter reads `field_string(entity, "id")` now and the engine's
/// `MovingPlatform` definition carries the field, so the ferry is addressed by
/// something an author chose on purpose.
pub const FERRY_ID: &str = "mary_o_1_2_ferry";

/// The stone the cavern is cut from. The one thing about 1-2 the LDtk file
/// cannot say, since a block carries no authored colour — the same reason 1-1
/// paints its vault masonry from Rust.
const UNDERGROUND_STONE: [f32; 4] = [0.20, 0.17, 0.28, 1.0];

/// **1-2's goal.**
///
/// ⭐ **the level had an exit but no END.** The alcove at the far wall returns
/// you to the surface, which is the shortcut's other mouth — walking into it is
/// leaving, not finishing. Jon: *"The end of 1-2 should transition back to
/// 1-1."* Finishing is grabbing a pole, the same verb 1-1 ends with, and the
/// pole is read off the authored shaft by the same rule 1-1's is
/// ([`crate::authored_pole`]).
pub fn goal_pole() -> crate::flag::FlagPole {
    crate::authored_pole(&level_1_2())
}

pub fn level_1_2() -> RoomSpec {
    let mut room = crate::authored_room(LEVEL_1_2_ROOM_ID);
    room.metadata.mode = Some(MARY_O_MODE.to_string());
    // The cavern is cut from ONE stone, so the colour goes on before the
    // by-name dressing rather than instead of it: `dress_authored_blocks` then
    // takes the pole back out again (its look is the prop laid over it).
    for block in &mut room.world.blocks {
        block.art_color = Some(UNDERGROUND_STONE);
    }
    crate::dress_authored_blocks(&mut room);
    room.props.extend(crate::scenery_for_authored_room(&room));
    room
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::engine_core as ae;
    use ambition_platformer2d::engine_core::AabbExt;

    /// One tile, which is the only measurement these tests still carry: it is
    /// the grid Jon draws on, not a fact about where anything is.
    const T: f32 = crate::T;

    /// The IntGrid cell — half a tile, and the smallest hole an author can make.
    ///
    /// ⛔ **the sweeps below step by CELL, not by tile, and that is not fussiness.**
    /// The first draft walked in whole tiles and probed with a quarter-tile box,
    /// which leaves 16px of every tile unexamined: a probe that erased one roof
    /// cell left the test GREEN, because the box straddled the cell next to the
    /// hole. A check whose resolution is coarser than the thing it checks is a
    /// check that passes for the wrong reason.
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

    /// **The floor's gap, measured off the room rather than named.**
    ///
    /// ⛔ this was a `CHASM: (f32, f32)` constant, which made the test a
    /// restatement of the number the level was built from. Probing the floor
    /// slab for the run of columns with nothing under them asks the LEVEL where
    /// its hole is, so dragging the far floor run in the editor moves the
    /// assertion with it.
    fn floor_gap(room: &RoomSpec) -> (f32, f32) {
        let depth = room.world.size.y - CELL;
        let mut runs: Vec<(f32, f32)> = Vec::new();
        let mut column = 0.0;
        while column < room.world.size.x {
            if !solid_in_cell(room, ae::Vec2::new(column, depth)) {
                match runs.last_mut() {
                    Some(run) if (run.1 - column).abs() < 0.5 => run.1 = column + CELL,
                    _ => runs.push((column, column + CELL)),
                }
            }
            column += CELL;
        }
        assert_eq!(
            runs.len(),
            1,
            "1-2's floor has {} gaps in it; the room's one new verb is the ferry \
             over ONE chasm, and a second hole is either a second crossing or a \
             pit that kills",
            runs.len()
        );
        runs[0]
    }

    #[test]
    fn the_chasm_is_only_crossable_by_the_platform() {
        let room = level_1_2();
        let (start, end) = floor_gap(&room);
        assert!(
            end - start >= 4.0 * T,
            "a {}px gap is a stride, not a chasm",
            end - start
        );

        // Nothing stands in the gap: no stepping stone, no floor.
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
            "the chasm has something standing in it, so the ferry is decoration",
        );

        // And the ferry spans it, lip to lip. RIDDEN rather than read: the sweep
        // range is the platform's own business, so the reach is measured by
        // advancing it until it has been everywhere it goes.
        let mut ferry = room
            .moving_platforms
            .iter()
            .find(|platform| platform.id == FERRY_ID)
            .cloned()
            .expect("1-2 authors its ferry");
        let (mut left, mut right) = (f32::MAX, f32::MIN);
        for _ in 0..200 {
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

    /// **The coin shelf has coins on it, and they rest ON it.**
    ///
    /// ⛔ **it was bare, and only the module doc said otherwise.** This level's
    /// own grammar calls the shelf *"a short raised run with coins on it, so the
    /// first thing the room teaches is that its ceiling is low enough to
    /// matter"* — and the Rust version never authored a single one, so the beat
    /// it describes did not exist. A doc comment claiming content is the easiest
    /// kind of claim to leave untrue, because nothing reads it.
    ///
    /// ⚠ **the rest position is the assertion that matters.** A coin floating
    /// above the shelf or sunk into it still "exists" and still collects; what
    /// it stops being is the thing the shelf teaches, which is that this run is
    /// worth the jump. So this checks they sit on its top edge, not merely that
    /// some placements are present.
    /// **Nothing authored into 1-2 is buried in the rock.**
    ///
    /// 1-2 was a corridor with six coins and a ferry — no enemies and no reactive
    /// blocks at all — until they were placed on 2026-08-05 by reading the
    /// IntGrid out of the file and picking coordinates, without being able to
    /// look at the result (the demo boots into 1-1, so a capture cannot see this
    /// room). Blind placement needs an oracle that is not a screenshot.
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
    fn the_coin_shelf_carries_coins_that_rest_on_it() {
        let room = level_1_2();
        // ⚠ identified STRUCTURALLY, not by a constant. `floor_top()` went away
        // when the level became authored, and re-deriving it here would put a
        // second copy of the room's geometry in a test of the room. The shelf is
        // the only SOLID that touches no edge of the world: the roof, the floor
        // runs and both walls each meet one.
        //
        // ⛔ **and "the only solid that touches no edge" EXPIRED the moment 1-2
        // got reactive blocks.** A `MaryOBlock` is a `BlockKind::Solid` floating
        // in mid-air too, so `find` started returning whichever came first and
        // this test failed claiming a coin hung off a shelf 300px away. The
        // structural idea is still right; it just has to say TERRAIN, and
        // `block_of` is the question that separates an authored reactive block
        // from the room's own stone.
        //
        // It also `collect`s and demands exactly one, because a `find` over an
        // ambiguous predicate answers confidently and wrongly — which is
        // precisely what happened.
        //
        // ⛔ **and "the only terrain shelf clear of every wall" EXPIRED TOO**,
        // the moment 1-2 stopped being one shelf in an empty box (2026-08-06,
        // Jon: *"World 1-2 also needs to be built out a lot more, its very
        // plain"*). Ceiling teeth, a second shelf and a staircase are all terrain
        // clear of every wall, and the count went to eight. Its own failure
        // message asked the right question — *"this test has to say which carries
        // the coins"* — so it does: the coin shelf is the one the COINS name, by
        // spanning them. That is a stronger statement than the count ever was,
        // because it survives any amount of further building.
        let size = room.world.size;
        let coins: Vec<_> = room
            .placements
            .iter()
            .filter(|placement| placement.name.starts_with("cavern_coin"))
            .collect();
        assert!(
            coins.len() >= 4,
            "the shelf is a RUN of coins — one or two reads as decoration, not as \
             a reason to jump. found {}",
            coins.len()
        );
        let coin_span = coins
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), coin| {
                (lo.min(coin.aabb.min.x), hi.max(coin.aabb.max.x))
            });
        let shelves: Vec<_> = room
            .world
            .blocks
            .iter()
            .filter(|block| {
                matches!(block.kind, ae::BlockKind::Solid)
                    && crate::ldtk_vocabulary::block_of(&block.name).is_none()
                    && block.aabb.min.x > 0.0
                    && block.aabb.min.y > 0.0
                    && block.aabb.max.x < size.x
                    && block.aabb.max.y < size.y
                    && block.aabb.min.x <= coin_span.0
                    && block.aabb.max.x >= coin_span.1
            })
            .collect();
        assert_eq!(
            shelves.len(),
            1,
            "exactly ONE raised terrain shelf spans the cavern coins ({}..{}); \
             found {}. Two would mean the coins rest on a stack and this test \
             cannot say which one they are for.",
            coin_span.0,
            coin_span.1,
            shelves.len()
        );
        let shelf = shelves[0];

        for coin in &coins {
            assert!(
                (coin.aabb.max.y - shelf.aabb.min.y).abs() < 1.0,
                "`{}` does not rest on the shelf: its bottom is {} and the shelf \
                 top is {}",
                coin.name,
                coin.aabb.max.y,
                shelf.aabb.min.y
            );
            assert!(
                coin.aabb.min.x >= shelf.aabb.min.x && coin.aabb.max.x <= shelf.aabb.max.x,
                "`{}` hangs off the shelf horizontally ({}..{} against {}..{})",
                coin.name,
                coin.aabb.min.x,
                coin.aabb.max.x,
                shelf.aabb.min.x,
                shelf.aabb.max.x
            );
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

    #[test]
    fn both_ends_of_the_room_are_ways_out() {
        let room = level_1_2();
        for id in [ARRIVAL_ZONE_ID, EXIT_ZONE_ID] {
            let zone = crate::authored_zone(&room, id);
            // Both stand ON the floor, not floating in it — the bug the 1-1
            // vault's return pipe shipped with (`cbc6902d2`). Asked of the
            // BLOCK under the zone rather than of a floor constant.
            let feet = zone.aabb.max.y;
            assert!(
                room.world.blocks.iter().any(|block| {
                    (block.aabb.min.y - feet).abs() <= 1.0
                        && block.aabb.min.x <= zone.aabb.min.x
                        && block.aabb.max.x >= zone.aabb.max.x
                }),
                "zone '{id}' does not stand on anything",
            );
        }
    }
}
