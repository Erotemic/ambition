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
    let project = ambition_platformer2d::ldtk_map::LdtkProject::from_json_str(WORLD_JSON).expect(
        "mary_o.ldtk parses (regen: game/ambition_demo_mary_o/tools/author_mary_o_ldtk.py)",
    );
    let room_set = project
        .to_room_set_with_entry("mary_o_1_1", &crate::ldtk_vocabulary::vocabulary())
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

/// The vocabulary the runtime RECOGNISES has to survive conversion — this is the
/// half `area create`'s IntGrid lowering would silently eat.
///
/// ⛔ it used to pin a LIST OF NAMES (`power_block_0`, `brick_2`, …). Those were
/// the bootstrap generator's names, and the level authors `MaryOBlock` entities
/// with a `kind` field now — so pinning names would pin the scaffolding rather
/// than the contract. What has to hold is that each KIND still arrives, and that
/// the pieces addressed by name still are.
#[test]
fn every_named_block_the_runtime_looks_for_survives_conversion() {
    use crate::ldtk_vocabulary::{block_look_of, MaryOBlockLook};
    let room = ldtk_room();
    let names: Vec<&str> = room.world.blocks.iter().map(|b| b.name.as_str()).collect();

    for kind in [
        MaryOBlockLook::Question,
        MaryOBlockLook::Quasar,
        MaryOBlockLook::Brick,
    ] {
        assert!(
            names.iter().any(|n| block_look_of(n) == Some(kind)),
            "no block converts to {kind:?}; the level authors one of each"
        );
    }

    // ⚠ **and every reactive block has a DISTINCT id.** `Block::solid` leaves the
    // id anonymous, so a converter that forgets to stamp the placement produces
    // blocks that all answer to one identity — a bonk on any of them resolved to
    // whichever came first. That happened, and this is what would have caught it.
    let ids: std::collections::BTreeSet<String> = room
        .world
        .blocks
        .iter()
        .filter(|b| block_look_of(&b.name).is_some())
        .map(|b| format!("{:?}", b.id))
        .collect();
    let reactive = room
        .world
        .blocks
        .iter()
        .filter(|b| block_look_of(&b.name).is_some())
        .count();
    assert_eq!(
        ids.len(),
        reactive,
        "every reactive block needs its own identity"
    );

    // ⭐ **the four pipe halves are no longer on this list**, and that is the
    // 2026-08-04 change: they were `Solid`s called `warp_pipe_<link>_<up|down>`
    // and a test spelling all four names was the only thing that could notice a
    // typo. They are `MaryOPipe`s pairing on an authored `link` now, and
    // `a_pipe_with_no_partner_is_refused_by_name` checks the pairing itself
    // rather than a list of names that a fifth pipe would not be on.
    //
    // The pieces still addressed by NAME, because the flag and the vault's
    // masonry are not `MaryOBlock`s yet.
    for expected in [
        "goal_pole",
        "goal_pole_knob",
        "goal_pole_banner",
        "vault_floor",
        "vault_wall_0",
        "vault_wall_1",
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

/// **Two vocabularies, one process, and they disagree — correctly.**
///
/// ⛔ **this test could not have been written before.** The converter registry
/// was a process-global `OnceLock` whose contract was "first install wins,
/// later calls are ignored": whichever `App`, tool or test touched it first
/// defined LDtk conversion for everything else in the process, and the only
/// evidence was an error log. Two games in one binary, a game and a tool, or
/// two test Apps in one run could not carry different vocabularies at all.
///
/// ⭐ **the vocabulary is a PARAMETER now**, so both answers below are true at
/// the same time in the same process, which is the whole claim:
///
/// - to the ENGINE's vocabulary, `MaryOBlock` is an unknown identifier, and
///   conversion refuses the level rather than dropping the blocks — the
///   authored bonus quietly becoming a plain wall is exactly the failure that
///   refusal exists to prevent;
/// - to MARY-O's, it converts.
///
/// ⚠ **the engine-only conversion is asserted TWICE, and the second one is the
/// test.** The first draft only checked it before Mary-O's vocabulary had ever
/// been built — and a probe that re-created the old global left that draft
/// GREEN, because nothing had installed anything yet when it ran. The claim
/// that needs proving is that constructing hers does not CHANGE the engine's
/// answer, so the same refusal has to hold on the far side of a successful
/// conversion. That is the exact shape the global got wrong.
#[test]
fn the_same_level_reads_differently_to_two_vocabularies_in_one_process() {
    let project = ambition_platformer2d::ldtk_map::LdtkProject::from_json_str(WORLD_JSON)
        .expect("mary_o.ldtk parses");

    let engine_only = project.to_room_set_with_entry(
        "mary_o_1_1",
        &ambition_platformer2d::ldtk_map::LdtkVocabulary::engine(),
    );
    let errors = engine_only.expect_err(
        "the engine alone cannot convert a level that authors Mary-O's own noun — \
         accepting it would mean silently dropping every reactive block",
    );
    assert!(
        errors.iter().any(|e| e.contains("MaryOBlock")),
        "the refusal must NAME the identifier nothing could convert: {errors:?}"
    );

    let with_hers = project
        .to_room_set_with_entry("mary_o_1_1", &crate::ldtk_vocabulary::vocabulary())
        .expect("her own vocabulary converts her own level, in this same process");
    assert!(
        with_hers.rooms.iter().any(|room| room.id == "mary_o_1_1"),
        "and it produces the level, not an empty set"
    );

    // ⛔ **the anti-leak assertion.** Having converted the level once with her
    // vocabulary, the ENGINE's answer must be unchanged. Under the old
    // process-global this is the one that failed: her converter would have been
    // installed by the call above and would still be answering for everyone.
    let engine_again = project.to_room_set_with_entry(
        "mary_o_1_1",
        &ambition_platformer2d::ldtk_map::LdtkVocabulary::engine(),
    );
    // ⚠ matched rather than `expect_err`, which prints the whole converted
    // `RoomSet` — hundreds of lines of blocks — as its failure message.
    let Err(errors) = engine_again else {
        panic!(
            "converting WITH Mary-O's vocabulary taught it to everyone else: the \
             engine alone converted `MaryOBlock` on the second call. A vocabulary \
             that leaks is the process-global this replaced."
        );
    };
    assert!(
        errors.iter().any(|e| e.contains("MaryOBlock")),
        "and it refuses for the same reason as before: {errors:?}"
    );
}
