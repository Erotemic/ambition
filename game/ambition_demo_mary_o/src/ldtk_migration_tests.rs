//! ⛔ **THE MIGRATION GATE.** The LDtk file was generated from `lib.rs`'s
//! constants, so before Jon edits anything the room it converts to must BE the
//! room those constants build. Until this passes, the file is a guess.
//!
//! ⚠ it compares what a player can TOUCH — every block's rect and kind — not the
//! whole `RoomSpec`. Props, art colours and the pieces still built Rust-side are
//! deliberately out of scope here; each moves under its own probe.

use ambition_platformer2d::engine_core as ae;

const WORLD_JSON: &str = include_str!("../assets/worlds/mary_o.ldtk");

fn ldtk_room() -> ambition_platformer2d::world::rooms::RoomSpec {
    let project = ambition_platformer2d::ldtk_map::LdtkProject::from_json_str(WORLD_JSON)
        .expect("mary_o.ldtk parses (regen: game/ambition_demo_mary_o/tools/author_mary_o_ldtk.py)");
    let room_set = project
        .to_room_set_with_entry("mary_o_1_1")
        .unwrap_or_else(|errors| panic!("mary_o.ldtk converts to rooms: {errors:?}"));
    room_set
        .rooms
        .into_iter()
        .find(|room| room.id == "mary_o_1_1")
        .expect("the world file authors the mary_o_1_1 area")
}

/// Which 16px cells this world's blocks occupy, per block kind.
///
/// ⛔ **cells, NOT rectangles, and the difference is the whole probe.** The first
/// draft compared block rects and reported 10 differences out of ~33 — every one
/// of them a staircase or a ground run. Nothing was misplaced: `area create`
/// lowers entities into an IntGrid and the IntGrid re-merges cells into maximal
/// rectangles, so a four-step staircase authored as four columns comes back as a
/// different PARTITION of the same area. Two partitions of one region are
/// gameplay-identical, and a test that calls them different is measuring the
/// packer, not the level.
///
/// ⚠ 16 not 32: half a tile, because the platforms and the pole are authored at
/// half-tile thickness and a tile-sized bucket would round them away.
fn occupied_cells(world: &ae::World) -> std::collections::BTreeSet<(String, i32, i32)> {
    const CELL: f32 = 16.0;
    let mut out = std::collections::BTreeSet::new();
    for block in &world.blocks {
        let kind = format!("{:?}", block.kind);
        let x0 = (block.aabb.min.x / CELL).round() as i32;
        let x1 = (block.aabb.max.x / CELL).round() as i32;
        let y0 = (block.aabb.min.y / CELL).round() as i32;
        let y1 = (block.aabb.max.y / CELL).round() as i32;
        for x in x0..x1 {
            for y in y0..y1 {
                out.insert((kind.clone(), x, y));
            }
        }
    }
    out
}

#[test]
fn the_ldtk_room_is_the_room_the_constants_built() {
    let ldtk = ldtk_room();
    let rust = crate::level_1_1();
    let (a, b) = (occupied_cells(&ldtk.world), occupied_cells(&rust.world));
    let only_ldtk: Vec<_> = a.difference(&b).take(24).collect();
    let only_rust: Vec<_> = b.difference(&a).take(24).collect();
    assert!(
        only_ldtk.is_empty() && only_rust.is_empty(),
        "the authored room and the built room cover different cells.\n  \
         only in LDtk ({} of {}): {:?}\n  only in Rust ({} of {}): {:?}",
        a.difference(&b).count(),
        a.len(),
        only_ldtk,
        b.difference(&a).count(),
        b.len(),
        only_rust
    );
}

/// The named blocks the runtime RECOGNISES have to survive conversion — this is
/// the half `area create`'s IntGrid lowering would silently eat.
#[test]
fn every_named_block_the_runtime_looks_for_survives_conversion() {
    let room = ldtk_room();
    let names: Vec<&str> = room.world.blocks.iter().map(|b| b.name.as_str()).collect();
    for expected in [
        "power_block_0", "power_block_1", "power_block_2",
        "quasar_block_0", "quasar_block_1", "quasar_block_2",
        "brick_0", "brick_1", "brick_2",
        "warp_pipe_descent_up", "warp_pipe_descent_down",
        "warp_pipe_ascent_down", "warp_pipe_ascent_up",
        "goal_pole", "goal_pole_knob", "goal_pole_banner",
        "vault_floor", "vault_wall_0", "vault_wall_1",
    ] {
        assert!(
            names.contains(&expected),
            "`{expected}` did not survive conversion; got {names:#?}"
        );
    }
}

/// ⚠ **ONE instrument, `#[ignore]`d so it never runs in the suite** — what the
/// authored file actually contains, when a claim about it needs settling.
///
/// ⭐ it earned its keep twice on the day it was written. The vault pipe hangs by
/// a clearance derived from Mary-O's TALL SPRITE, which the Python generator
/// cannot call into; I guessed 48, it is 67.2, and 24 cells of pipe hung too low
/// until this printed it. Then a capture *looked* like the ground slab had grown
/// to three rows — it prints `y 416..480`, exactly the two tiles authored, and
/// the zoom was the liar.
///
///     cargo test -p ambition_demo_mary_o what_the_file_authors -- --ignored --nocapture
#[test]
#[ignore]
fn print_what_the_file_authors() {
    let room = ldtk_room();
    println!("world size {:?}", room.world.size);
    println!(
        "tall_body_size = {:?}  (the generator mirrors this by hand)",
        crate::powerups::tall_body_size()
    );

    let mut named: Vec<String> = room
        .world
        .blocks
        .iter()
        .map(|b| format!("{} [{:?}]", b.name, b.id.source))
        .collect();
    named.sort();
    named.dedup();
    println!("{} blocks:", room.world.blocks.len());
    for n in &named {
        println!("  {n}");
    }

    let mut runs: Vec<(i32, i32, i32, i32)> = room
        .world
        .blocks
        .iter()
        .filter(|b| b.name == "ldtk solid")
        .map(|b| {
            (
                b.aabb.min.y as i32,
                b.aabb.max.y as i32,
                b.aabb.min.x as i32,
                b.aabb.max.x as i32,
            )
        })
        .collect();
    runs.sort();
    println!("{} merged terrain rects:", runs.len());
    for r in &runs {
        println!("  y {}..{}  x {}..{}", r.0, r.1, r.2, r.3);
    }

    use ambition_platformer2d::engine_core::AabbExt;
    println!("{} placements:", room.placements.len());
    for p in room.placements.iter().take(4) {
        println!("  {:?} center={:?}", p.id, p.aabb.center());
    }
}
