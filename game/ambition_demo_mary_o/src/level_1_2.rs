//! **World 1-2** — the underground level, and the demo's first SECOND ROOM.
//!
//! Until 2026-07-25 this could not exist. `RoomTransitionRequested` had exactly
//! one consumer and only `ambition_app` registered it, so a demo host could not
//! change rooms at all — which is why 1-1's coin vault had to be dug into the
//! same `RoomSpec` as the surface rather than being its own room. The
//! transaction is engine-side now (`ambition::runtime::room_transition`), and
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
//!
//! The platform is authored as an ordinary [`MovingPlatformState`] sweep. Riding
//! it is engine behavior — the platform advance runs once per frame before the
//! body tick and the ride/ledge-carry logic reads its delta — so this level adds
//! no movement code of its own.

use ambition::engine_core as ae;
use ambition::world::platforms::MovingPlatformState;
use ambition::world::rooms::{LoadingZone, LoadingZoneActivation, RoomSpec};

use crate::{MARY_O_MODE, T};

pub const LEVEL_1_2_ROOM_ID: &str = "mary_o_level_1_2";

/// The zone in 1-1's vault that drops you into 1-2, and its partner in 1-2.
///
/// Both are `Walk`: you step into the shaft and go. The vault's own two pipes
/// stay directional presses (Jon's rule — a pipe answers UP or DOWN, never a
/// generic Interact); this is a different affordance on purpose, an open shaft
/// in the vault floor rather than a third tube competing with them.
pub const DESCENT_ZONE_ID: &str = "mary_o_1_1_descent";
pub const ARRIVAL_ZONE_ID: &str = "mary_o_1_2_arrival";
/// The way back up: 1-2's exit alcove, and where it puts you on the surface.
pub const EXIT_ZONE_ID: &str = "mary_o_1_2_exit";
pub const SURFACE_RETURN_ZONE_ID: &str = "mary_o_1_1_surface_return";

const WIDTH_TILES: f32 = 56.0;
const HEIGHT_TILES: f32 = 14.0;
/// How thick the roof and floor slabs are. Two tiles each, so the playable
/// corridor is ten tiles tall — enough to jump in, low enough to feel enclosed.
const SLAB_TILES: f32 = 2.0;
const CHASM: (f32, f32) = (28.0, 33.0);

const UNDERGROUND_STONE: [f32; 4] = [0.20, 0.17, 0.28, 1.0];

fn floor_top() -> f32 {
    (HEIGHT_TILES - SLAB_TILES) * T
}

fn ceiling_bottom() -> f32 {
    SLAB_TILES * T
}

fn slab(blocks: &mut Vec<ae::Block>, name: &str, idx: u16, from: f32, to: f32, top: f32) {
    blocks.push(
        ae::Block::solid_tiled(
            name,
            ae::Vec2::new(from * T, top),
            ae::Vec2::new((to - from) * T, SLAB_TILES * T),
            "mary_o_ground",
            idx,
        )
        .with_art_color(UNDERGROUND_STONE),
    );
}

/// Where the moving platform starts, and how far it sweeps.
///
/// It starts on the near lip so the ride is always available rather than
/// something you wait for on the wrong side of a gap you cannot cross.
fn platform_sweep() -> (ae::Vec2, ae::Vec2, f32) {
    let size = ae::Vec2::new(3.0 * T, 0.5 * T);
    let start = ae::Vec2::new(CHASM.0 * T + size.x * 0.5, floor_top() - 3.0 * T);
    let sweep = (CHASM.1 - CHASM.0) * T - size.x;
    (start, size, sweep)
}

pub fn moving_platform() -> MovingPlatformState {
    let (start, size, sweep) = platform_sweep();
    MovingPlatformState::from_sweep(
        "mary_o_1_2_ferry",
        "Underground Ferry",
        start,
        size,
        sweep,
        90.0,
    )
}

/// The shaft you arrive out of, at the left end of the corridor.
pub fn arrival_zone() -> LoadingZone {
    let pos = ae::Vec2::new(3.0 * T, floor_top() - 1.5 * T);
    LoadingZone {
        id: ARRIVAL_ZONE_ID.to_string(),
        name: "From the vault".to_string(),
        activation: LoadingZoneActivation::Walk,
        aabb: ae::Aabb::new(pos, ae::Vec2::new(T, 1.5 * T)),
    }
}

/// The alcove at the far end that returns you to the surface.
pub fn exit_zone() -> LoadingZone {
    let pos = ae::Vec2::new((WIDTH_TILES - 3.0) * T, floor_top() - 1.5 * T);
    LoadingZone {
        id: EXIT_ZONE_ID.to_string(),
        name: "Up to the surface".to_string(),
        activation: LoadingZoneActivation::Walk,
        aabb: ae::Aabb::new(pos, ae::Vec2::new(T, 1.5 * T)),
    }
}

pub fn level_1_2() -> RoomSpec {
    let mut blocks = Vec::new();

    // Roof, unbroken: this is what makes it underground.
    slab(&mut blocks, "cavern_roof", 0, 0.0, WIDTH_TILES, 0.0);
    // Floor, in two runs with the chasm between them.
    slab(
        &mut blocks,
        "cavern_floor_near",
        1,
        0.0,
        CHASM.0,
        floor_top(),
    );
    slab(
        &mut blocks,
        "cavern_floor_far",
        2,
        CHASM.1,
        WIDTH_TILES,
        floor_top(),
    );
    // End walls, so the room is closed rather than trailing off into space.
    for (idx, x) in [(3u16, -SLAB_TILES), (4, WIDTH_TILES)] {
        blocks.push(
            ae::Block::solid_tiled(
                "cavern_wall",
                ae::Vec2::new(x * T, 0.0),
                ae::Vec2::new(SLAB_TILES * T, HEIGHT_TILES * T),
                "mary_o_ground",
                idx,
            )
            .with_art_color(UNDERGROUND_STONE),
        );
    }
    // The coin shelf: one raised run, reachable with an ordinary jump.
    blocks.push(
        ae::Block::solid_tiled(
            "coin_shelf",
            ae::Vec2::new(12.0 * T, floor_top() - 4.0 * T),
            ae::Vec2::new(6.0 * T, T),
            "mary_o_ground",
            5,
        )
        .with_art_color(UNDERGROUND_STONE),
    );

    // Spawn sits under the arrival shaft, on the near floor. A body that somehow
    // reaches this room without a transition still starts somewhere sane.
    let spawn = ae::Vec2::new(3.0 * T, floor_top() - 2.0 * T);
    let world = ae::World::new(
        "Mary-O 1-2",
        ae::Vec2::new(WIDTH_TILES * T, HEIGHT_TILES * T),
        spawn,
        blocks,
    );

    let mut room = RoomSpec::new(LEVEL_1_2_ROOM_ID, world);
    room.metadata.mode = Some(MARY_O_MODE.to_string());
    room.loading_zones = vec![arrival_zone(), exit_zone()];
    room.moving_platforms = vec![moving_platform()];
    room
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition::engine_core::AabbExt;

    #[test]
    fn the_chasm_is_only_crossable_by_the_platform() {
        let room = level_1_2();
        let (start, size, sweep) = platform_sweep();

        // Nothing stands in the gap: no stepping stone, no floor.
        let gap = ae::Aabb::new(
            ae::Vec2::new((CHASM.0 + CHASM.1) * 0.5 * T, floor_top() + T),
            ae::Vec2::new((CHASM.1 - CHASM.0) * 0.5 * T - 1.0, T),
        );
        assert!(
            !room
                .world
                .blocks
                .iter()
                .any(|b| b.aabb.strict_intersects(gap)),
            "the chasm has something standing in it, so the ferry is decoration",
        );

        // And the ferry spans it: near lip to far lip, both ends reachable.
        assert!(start.x - size.x * 0.5 <= CHASM.0 * T + 1.0);
        assert!(start.x + sweep + size.x * 0.5 >= CHASM.1 * T - 1.0);
    }

    #[test]
    fn the_room_is_closed_and_underground() {
        let room = level_1_2();
        // A roof over every column of the corridor — the difference between an
        // underground level and a pit.
        for column in 1..(WIDTH_TILES as i32 - 1) {
            let probe = ae::Aabb::new(
                ae::Vec2::new(column as f32 * T + T * 0.5, ceiling_bottom() - T * 0.5),
                ae::Vec2::splat(T * 0.25),
            );
            assert!(
                room.world
                    .blocks
                    .iter()
                    .any(|b| b.aabb.strict_intersects(probe)),
                "column {column} has no roof",
            );
        }
    }

    #[test]
    fn both_ends_of_the_room_are_ways_out() {
        let room = level_1_2();
        let ids: Vec<&str> = room
            .loading_zones
            .iter()
            .map(|zone| zone.id.as_str())
            .collect();
        assert!(ids.contains(&ARRIVAL_ZONE_ID));
        assert!(ids.contains(&EXIT_ZONE_ID));
        // Both stand ON the floor, not floating in it — the bug the 1-1 vault's
        // return pipe shipped with (`cbc6902d2`).
        for zone in &room.loading_zones {
            let feet = zone.aabb.max.y;
            assert!(
                (feet - floor_top()).abs() <= T * 0.5,
                "zone '{}' does not meet the floor",
                zone.id,
            );
        }
    }
}
