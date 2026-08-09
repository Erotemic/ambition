//! **What has to survive LDtk conversion**, whatever the level says.
//!
//! ⛔ **the migration GATE that used to head this file is gone (2026-08-07), and
//! why matters more than that it went.** It compared the converted room against
//! `crate::level_1_1()` to prove the generated file matched the Rust constants
//! the level used to be. That job finished: `level_1_1()` is now
//! `authored_room(LEVEL_1_1_ROOM_ID)` — it READS `mary_o.ldtk` — so the test had
//! become a comparison of the file with itself. The dressing in between
//! (`dress_authored_blocks` sets `art_color`, `scenery_for_authored_room`
//! extends `props`) touches nothing the comparison looked at, which walked
//! `world.blocks`. **It could not fail, including on the edit it existed to
//! catch.** Its helper `occupied_cells` went with it, having no other caller.
//!
//! ⭐ **so nothing here pins the level's SHAPE, deliberately.** Jon authors this
//! file in LDtk; a test that says "the pyramid is where it was" would turn every
//! edit into a test edit and teach the habit of updating expectations to match
//! whatever came out. What remains asserts things that stay true across any
//! legitimate edit: every block KIND survives lowering, the named pieces the
//! runtime addresses still exist, one file reads differently to two
//! vocabularies, and no lift teleports you somewhere visible.
//!
//! ⚠ **the level's real invariants live in `lib.rs`'s tests, not here** — every
//! enemy has ground under it, the vault ceiling is unbroken, a pipe you enter
//! has a pipe you come out of, the trench is wide enough to patrol. Those SHOULD
//! fail on a bad edit; that is the safety net for editing, not friction against
//! it.

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

    // ⭐ **the quasar moved from the LOOK column to the CONTENTS column**, so
    // what the level must still author is a block that PAYS one — checked
    // below — rather than a look with that name. Asserting on the look would
    // have gone green forever the moment the variant was deleted.
    use crate::ldtk_vocabulary::{block_of, MaryOBlockContents, MaryOPickup};
    assert!(
        names.iter().any(|n| block_of(n).map(|b| b.contents)
            == Some(MaryOBlockContents::Always(MaryOPickup::Quasar))),
        "no block in the level pays a quasar any more"
    );
    for kind in [MaryOBlockLook::Question, MaryOBlockLook::Brick] {
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

/// **A pole with no flag on it is still a pole to every other check.**
///
/// Jon, 2026-08-09: *"In mary-o 1-2 the flagpole doesn't have a flag."* He was
/// right, and the dressing code was not wrong: `scenery_for_authored_room`
/// matches `goal_pole` → shaft, `goal_pole_knob` → finial, `goal_pole_banner` →
/// flag, and its `_ => {}` arm means **an unauthored name produces no prop and
/// no complaint**. 1-1 authored all three; 1-2 authored only the shaft.
///
/// ⛔ **the load-bearing name is exactly backwards from how the bug presents.**
/// [`crate::authored_pole`] panics if `goal_pole` is missing, so the piece the
/// player cannot see is mandatory and the two pieces they look at are optional.
/// Nothing between the file and the screen had an opinion about the flag.
///
/// ⭐ **so this pins the CLASS, not the level**: any room that stands a shaft
/// dresses it completely. A third level authored with a bare pole fails here
/// rather than shipping, which is the whole point of not spelling `mary_o_1_2`
/// anywhere below.
///
/// ⚠ **asserted on the PROPS, not on the block names**, because the prop is what
/// Jon looked at. A rename on either side of the `match` is a silently missing
/// picture again, and a name list would still be green through it.
///
/// ⚠ `test_course` is deliberately out of scope: it builds its shaft in Rust as
/// a scripted-run fixture that nothing renders, so dressing it would be a
/// costume for a test harness.
#[test]
fn every_authored_pole_wears_its_finial_and_its_flag() {
    let room_set = crate::authored_world();

    let mut shafts = 0usize;
    let mut bare: Vec<String> = Vec::new();

    for room in &room_set.rooms {
        let has_block = |name: &str| room.world.blocks.iter().any(|b| b.name == name);
        if !has_block(crate::GOAL_POLE_PREFIX) {
            continue;
        }
        shafts += 1;
        let props = crate::scenery_for_authored_room(room);
        let has_prop = |id: &str| props.iter().any(|p| p.id == id);
        for (prop, block) in [
            ("goal_pole_finial_art", "goal_pole_knob"),
            ("goal_pole_banner_art", "goal_pole_banner"),
        ] {
            if !has_prop(prop) {
                bare.push(format!(
                    "{}: stands a `{}` shaft but draws no `{prop}`, because the room \
                     authors no `{block}` block",
                    room.id,
                    crate::GOAL_POLE_PREFIX,
                ));
            }
        }
    }

    assert!(
        bare.is_empty(),
        "an authored flagpole is missing its dressing:\n{}",
        bare.join("\n"),
    );
    // ⚠ the floor, and it is the half of this that could rot silently: the
    // failure mode of a "for every room that …" check is that no room ever
    // matches, which reads exactly like a pass. Both authored levels finish on a
    // pole, so anything under two means the loop stopped seeing them.
    assert!(
        shafts >= 2,
        "both authored levels finish on a flagpole; only {shafts} shaft(s) reached \
         this check, so it is not looking at the levels"
    );
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

/// **A lift must wrap where nobody can see it.**
///
/// Jon's words are the requirement, and the parenthesis is the whole of it:
/// *"When they go OOB (far enough so they are off screen of the player in normal
/// gameplay) they can teleport to the top / bottom of the screen to make an
/// infinite elevator effect."*
///
/// ⭐ **a teleport the player CAN see is not an elevator, it is a glitch** — and
/// no other check catches it. The platform is in bounds, the shaft is well
/// formed, the ride works, and the effect is simply ruined. 1-2 is 1920x448
/// against a ~360-tall viewport, so a wrap inside the room lands mid-screen.
///
/// **Why the room's own extent is the right line.** Under
/// `CameraClampMode::RoomBounds` the camera target is clamped to
/// `[-H/2 + half_view, +H/2 - half_view]`, so the visible band is exactly
/// `[-H/2, +H/2]` — the room and nothing past it. A lift whose whole body is
/// outside the room at both wrap points therefore cannot be on screen at either,
/// and this never has to know the viewport size.
///
/// ⚠ **two escapes, stated rather than guarded**, both visible in
/// `clamp_camera_target` and neither reachable from what 1-2 authors: a room
/// SHORTER than the viewport falls back to centring, which shows past the
/// bounds, and portal padding can expand them. A room that gains either wants
/// this tightened to the real band rather than trusted.
#[test]
fn every_lift_wraps_outside_the_room() {
    let room_set = crate::authored_world();

    let mut looping = 0usize;
    let mut visible: Vec<String> = Vec::new();

    for room in &room_set.rooms {
        let height = room.world.size.y;
        for platform in &room.moving_platforms {
            let Some((min_y, max_y)) = platform.vertical_loop_span() else {
                continue;
            };
            looping += 1;
            let half = platform.size.y * 0.5;
            // Where it REAPPEARS: the bottom edge must still be above the room.
            if min_y + half > 0.0 {
                visible.push(format!(
                    "{}: `{}` reappears at y {min_y:.0} (bottom edge {:.0}) inside \
                     the room's 0..{height:.0} — the player watches it pop in",
                    room.id,
                    platform.name,
                    min_y + half,
                ));
            }
            // Where it VANISHES: the top edge must already be below the room.
            if max_y - half < height {
                visible.push(format!(
                    "{}: `{}` vanishes at y {max_y:.0} (top edge {:.0}) inside the \
                     room's 0..{height:.0} — the player watches it blink out",
                    room.id,
                    platform.name,
                    max_y - half,
                ));
            }
        }
    }

    assert!(
        visible.is_empty(),
        "a lift teleports where the player can see it:\n{}",
        visible.join("\n"),
    );
    // ⚠ the floor, and it has already earned itself: the first draft of this
    // check ran against the ANOTHER world's project and found no looping
    // platform at all, which is a green indistinguishable from a safe one.
    assert!(
        looping >= 3,
        "1-2 authors a three-lift conveyor; only {looping} looping platform(s) \
         reached this check, so it is not looking at the shaft"
    );
}
