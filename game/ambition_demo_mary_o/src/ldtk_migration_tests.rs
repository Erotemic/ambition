//! What has to survive LDtk conversion, whatever the level says.
//!
//! What remains asserts things that stay true across any legitimate edit: every block KIND survives
//! lowering, the named pieces the runtime addresses still exist, one file reads differently to two
//! vocabularies, and no lift teleports you somewhere visible.
//!
//! the level's real invariants live in `lib.rs`'s tests, not here — every
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
#[test]
fn every_named_block_the_runtime_looks_for_survives_conversion() {
    use crate::ldtk_vocabulary::{block_look_of, MaryOBlockLook};
    let room = ldtk_room();
    let names: Vec<&str> = room.world.blocks.iter().map(|b| b.name.as_str()).collect();

    // Asserting on the look would have gone green forever the moment the variant was deleted.
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

    // and every reactive block has a DISTINCT id. `Block::solid` leaves the
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

    // They are `MaryOPipe`s pairing on an authored `link` now, and
    // `a_pipe_with_no_partner_is_refused_by_name` checks the pairing itself rather than a list of
    // names that a fifth pipe would not be on.
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

/// A pole with no flag on it is still a pole to every other check.
///
/// right, and the dressing code was not wrong: `scenery_for_authored_room`
/// matches `goal_pole` → shaft, `goal_pole_knob` → finial, `goal_pole_banner` →
/// flag, and its `_ => {}` arm means an unauthored name produces no prop and
/// no complaint. 1-1 authored all three; 1-2 authored only the shaft.
///
/// Nothing between the file and the screen had an opinion about the flag.
///
/// so this pins the CLASS, not the level: any room that stands a shaft
/// dresses it completely. A third level authored with a bare pole fails here
/// rather than shipping, which is the whole point of not spelling `mary_o_1_2`
/// anywhere below.
///
/// asserted on the PROPS, not on the block names, because the prop is what
/// picture again, and a name list would still be green through it.
///
/// `test_course` is deliberately out of scope: it builds its shaft in Rust as
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
    assert!(
        shafts >= 2,
        "both authored levels finish on a flagpole; only {shafts} shaft(s) reached \
         this check, so it is not looking at the levels"
    );
}

/// ONE instrument, `#[ignore]`d so it never runs in the suite — what the
/// authored file actually contains, when a claim about it needs settling.
///
/// it earned its keep twice on the day it was written. The vault pipe hangs by a clearance
/// derived from Mary-O's TALL SPRITE, which the Python generator cannot call into; I guessed
/// 48, it is 67.2, and 24 cells of pipe hung too low until this printed it.
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

/// Two vocabulary values can coexist in one process and produce different
/// conversion answers without mutating one another. Constructing Mary-O's
/// vocabulary must not change the engine-only vocabulary's refusal.
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

    // the anti-leak assertion. Having converted the level once with her
    // vocabulary, the ENGINE's answer must be unchanged. Under the old
    // process-global this is the one that failed: her converter would have been
    // installed by the call above and would still be answering for everyone.
    let engine_again = project.to_room_set_with_entry(
        "mary_o_1_1",
        &ambition_platformer2d::ldtk_map::LdtkVocabulary::engine(),
    );
    // matched rather than `expect_err`, which prints the whole converted
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

/// A lift must wrap where nobody can see it.
///
/// *"When they go OOB (far enough so they are off screen of the player in normal
/// gameplay) they can teleport to the top / bottom of the screen to make an
/// infinite elevator effect."*
///
/// a teleport the player CAN see is not an elevator, it is a glitch — and
/// no other check catches it. The platform is in bounds, the shaft is well
/// formed, the ride works, and the effect is simply ruined. 1-2 is 1920x448
/// against a ~360-tall viewport, so a wrap inside the room lands mid-screen.
///
/// Why the room's own extent is the right line. Under
/// `CameraClampMode::RoomBounds` the camera target is clamped to
/// `[-H/2 + half_view, +H/2 - half_view]`, so the visible band is exactly
/// `[-H/2, +H/2]` — the room and nothing past it. A lift whose whole body is
/// outside the room at both wrap points therefore cannot be on screen at either,
/// and this never has to know the viewport size.
///
/// two escapes, stated rather than guarded, both visible in
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
    // Require a non-empty looping-platform census so this check cannot pass
    // against the wrong world or an empty fixture.
    assert!(
        looping >= 3,
        "1-2 authors a three-lift conveyor; only {looping} looping platform(s) \
         reached this check, so it is not looking at the shaft"
    );
}
