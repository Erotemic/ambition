//! Super Mary-O demo content home — the M-track's world half.
//!
//! This crate names only `ambition_platformer2d` and `bevy`. It is the E9
//! engine-for-other-games ORACLE, run a SECOND time: if authoring a second
//! platformer's level, roster, and rules needs a type the umbrella does not
//! re-export, that is a real engine leak — and it fails to compile HERE, which is
//! the point. Sanic proved the oracle for a momentum game. This proves it for a
//! completely different grammar, which is the only way "engine, not game" stops
//! being a claim and starts being a measurement.
//!
//! **Parody names are policy** (Q28, Jon 2026-07-06): homage in grammar, never a
//! copy. The level below is the 1-1 GRAMMAR — open teach, pit rhythm, a stair
//! pyramid, a goal — authored as original geometry, not a traced layout.
//!
//! What lives here is the LEVEL and the RULES. Tile art, the powerup equipment
//! rows (M1), the camera scroll policy (M2), and the flagpole sequence (M3) are
//! the rest of the M-track; see `docs/planning/demos/super-mary-o.md`.

pub mod ai_slop;
#[cfg(test)]
mod binding_tests;
pub mod bricks;
pub mod death;
pub mod flag;
#[cfg(test)]
mod ldtk_migration_tests;
pub mod ldtk_vocabulary;
pub mod level_1_2;
pub mod movement;
pub mod pipe;
pub mod powerups;
pub mod provider;
pub mod quasar_shader;
pub mod scenery;
pub mod snake;
pub mod star;
pub mod stomp;
pub mod test_course;

pub use provider::{
    mary_o_session_world, MaryOExperiencePlugin, MaryOSessionWorld, MARY_O_CHARACTER_ID,
    MARY_O_EXPERIENCE, MARY_O_GAMEPLAY_ROUTE, MARY_O_LAUNCHER_ROUTE,
};

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::prelude::*;
use ambition_platformer2d::world::rooms::{PropSpec, RoomSpec};

/// Stable room id for level 1-1.
pub const LEVEL_1_1_ROOM_ID: &str = "mary_o_1_1";

/// The game-MODE tag this demo's rooms carry (decomposition D-C).
///
/// Ambition can host this demo alongside its own rooms; [`MaryORulesPlugin`] gates
/// its systems on `ambition_platformer2d::runtime::in_mode(MARY_O_MODE)` so the level clock never
/// ticks in a room that is not Mary-O's.
pub const MARY_O_MODE: &str = "mary_o";

/// The level clock starts here and counts DOWN. It is the demo's one rule.
pub const STARTING_TIME: f32 = 400.0;

/// Lives Mary-O starts a run with.
const STARTING_LIVES: u8 = 3;

/// How long the "WORLD 1-1 / MARY-O x3" card sits before play reads as normal.
const INTRO_CARD_SECONDS: f32 = 2.0;

/// How long the flag tally sits on screen before the level loops. "The next
/// level is the same level": completing the flagpole restarts 1-1, cyclically.
pub const LEVEL_CYCLE_DWELL: f32 = 2.0;

/// One tile. The whole level is authored on this grid, because the 1-1 grammar IS
/// a grid grammar: a jump clears a few tiles, a pit is two or three wide.
pub(crate) const T: f32 = 32.0;

/// Ground thickness, in tiles.
const GROUND_TILES: f32 = 2.0;

// ⭐ **THE COLUMN TABLES ARE GONE (2026-08-04)** — `POWER_BLOCK_COLUMNS`,
// `QUASAR_BLOCK_COLUMNS`, `BRICK_COLUMNS`, their `_ROW`s, and the
// `_LAYER`/`_BASE_INDEX` pairs that minted their `GeoId`s. Ten constants that
// said where the special blocks were, in a language the level editor could not
// read. Where they are is authored now; `authored_block` reads it back.
//
// ⚠ deleting them rather than leaving them "for reference" is the whole point.
// A constant that still names a block's column is a second authority waiting to
// disagree with the file the moment Jon drags one.

// ── THE AUTHORED VOCABULARY ────────────────────────────────────────────────
//
// ⛔ **A block's NAME is its meaning**, because the engine has no typed channel
// for one game's nouns: an authored block carries `{id, name, aabb, kind,
// velocity, art_color}` and nothing else. Sanic's monitor boxes work the same
// way (`monitors.rs`), and the trade-off is written up in
// `docs/planning/proposal-authored-vocabulary-2026-08-04.md` §4.
//
// ⚠ **these prefixes are the contract between the LDtk file and this crate.**
// Renaming one silently unhooks every block that wore it — the level still
// loads, the blocks are still solid, and they simply stop being special. Nothing
// in the type system can catch that, so
// `every_named_block_the_runtime_looks_for_survives_conversion` pins the whole
// list against the shipped file.

/// A ?-block: bonk it from below for the next rung of the powerup ladder.
pub const POWER_BLOCK_PREFIX: &str = "power_block_";
/// The pocket quasar — any form can take one and be briefly untouchable.
pub const QUASAR_BLOCK_PREFIX: &str = "quasar_block_";
/// Breakable masonry: a bonk from a grown body removes it.
pub const BRICK_PREFIX: &str = "brick_";
/// One half of a warp tube. Two halves sharing a `<link>` are a PAIR.
pub const WARP_PIPE_PREFIX: &str = "warp_pipe_";
/// The suffix that says a pipe's mouth points DOWN — you fall out of it, or rise
/// into it. Anything else is mouth-up: you press DOWN on it.
pub const PIPE_MOUTH_DOWN_SUFFIX: &str = "_down";
/// The flag: shaft, finial and banner, all the same width and column.
pub const GOAL_POLE_PREFIX: &str = "goal_pole";
/// The secret chamber's stone — `vault_floor` and `vault_wall_<n>`.
pub const VAULT_MASONRY_PREFIX: &str = "vault_";

/// How thick the goal pole is drawn. Half a tile — a pole, not a pillar. Named
/// because [`goal_pole`] must derive the grab band from the SAME number
/// [`level_1_1`] draws the block with; a band narrower than the pole is a level
/// that cannot be finished.
const POLE_WIDTH: f32 = T * 0.5;

// ⭐ `LEVEL_WIDTH` / `LEVEL_HEIGHT` are GONE (2026-08-04). They were the level's
// size as a Rust fact, and the level's size is now whatever Jon drew — read it
// off the loaded room (`room.world.size`). Deleting them rather than leaving
// them "for reference" is the point: a constant that still names the world is a
// second authority waiting to disagree with the file.

// ⭐⭐ **THE DOUBLE-STAIRS CONSTANTS ARE GONE (2026-08-04)** —
// `STAIR_FIRST_COLUMN`, `STAIR_STEPS`, `STAIR_GAP_TILES`,
// `stair_far_first_column()` and `stair_steps()`.
//
// They were the ONE authority for the pyramid's columns, and their own comment
// said why: the shape used to be inlined in `level_1_1` AND hand-copied into
// `ai_slop.rs`, so moving the stairs left eight enemies floating where the steps
// had been. Deriving both from one table fixed that — and the table is still a
// Rust fact the level editor cannot read.
//
// The pyramid is authored geometry now and the slop that stands on it is an
// authored placement, so the drift they guarded against is not possible: there
// is no second place for the shape to live.

/// The SURFACE half's height — every above-ground feature is placed against
/// this, so growing the world downward for the vault below leaves the authored
/// 1-1 layout byte-identical.
const SURFACE_HEIGHT: f32 = 15.0 * T;

/// How far below the ground slab the secret vault's floor sits.
const VAULT_DEPTH_TILES: f32 = 9.0;

/// The four halves of Mary-O's two warp tubes, by their AUTHORED names.
///
/// ⛔ **`pipe_halves()` — a four-element Rust table of positions, sizes and a
/// `mouth_down` flag — is GONE (2026-08-04), and with it `PIPE_COLUMN`,
/// `EXIT_PIPE_COLUMN`, `PIPE_WIDTH_TILES`, `PIPE_HEIGHT_TILES`,
/// `surface_pipe_min/size`, `vault_pipe_min/size` and `vault_pipe_clearance`.**
/// Nine constants and five functions computing where a pipe was, in a language
/// the level editor could not read — and the descent/ascent PAIRING was the
/// order of the tuples. A pipe is authored now; these names are how the runtime
/// finds the one it means.
///
/// ⚠ **the pairing is still the NAME**, `warp_pipe_<link>_<up|down>`, which is
/// the weakest part of the authored vocabulary: a typo pairs nothing and only a
/// load-time check can catch it. `a_pipe_you_enter_always_has_a_pipe_you_come_
/// out_of` is that check.
const PIPE_NAME: &str = "warp_pipe_descent_up";
const VAULT_ENTRY_PIPE_NAME: &str = "warp_pipe_descent_down";
const EXIT_PIPE_NAME: &str = "warp_pipe_ascent_down";
const SURFACE_EXIT_PIPE_NAME: &str = "warp_pipe_ascent_up";

/// The stone the secret chamber is cut from — the one thing about the vault the
/// LDtk file cannot say, since a block carries no authored colour.
const VAULT_STONE_COLOR: [f32; 4] = [0.24, 0.20, 0.30, 1.0];

/// The vault's interior, in world coordinates.
///
/// ▢ still derived, and it is the last of 1-1's geometry that is: the chamber's
/// walls are authored but its BOUNDS are computed here, and the two agree only
/// because the generator emitted the walls from this. Deriving it from the
/// authored `vault_wall_*` blocks is the obvious next move.
pub fn vault_bounds() -> ae::Aabb {
    let ceiling = SURFACE_HEIGHT;
    let floor = SURFACE_HEIGHT + (VAULT_DEPTH_TILES - 2.0) * T;
    let left = 23.0 * T;
    let size = ae::Vec2::new(18.0 * T, floor - ceiling);
    let min = ae::Vec2::new(left, ceiling);
    ae::Aabb::new(min + size * 0.5, size * 0.5)
}

/// How close to a pipe's open face counts as touching it: half a tile.
///
/// Not a trigger zone — a contact tolerance. It is small enough that a body has
/// to be AT the face (a grown body standing under a hanging lip is 8px from it;
/// a small one is 32px away and has to hop into the mouth) and large enough that
/// the press does not demand a pixel-exact overlap of two boxes that, resting on
/// each other, only ever touch at the edge.
const MOUTH_SLACK: f32 = 0.5 * T;

/// **A pipe's mouth is its OPEN FACE** — the lip end — with [`MOUTH_SLACK`] of
/// contact tolerance either side of it, spanning the pipe's own width.
///
/// One rule, derived from the half's own geometry, for both ends of every tube:
/// the mouth cannot drift away from the pipe it belongs to, and there is nothing
/// to hand-measure. Both mouths used to be authored as bands across the SURFACE
/// YOU STAND ON, which is right for a pipe you stand on top of and wrong for one
/// hanging overhead — it put the ascent trigger on the vault floor, several tiles
/// below the pipe, so pressing UP worked anywhere in the pipe's column of air.
fn mouth_band(min: ae::Vec2, size: ae::Vec2, mouth_down: bool) -> ae::Aabb {
    let face = if mouth_down { min.y + size.y } else { min.y };
    let band = ae::Vec2::new(size.x, 2.0 * MOUTH_SLACK);
    let corner = ae::Vec2::new(min.x, face - MOUTH_SLACK);
    ae::Aabb::new(corner + band * 0.5, band * 0.5)
}

/// The mouth of the AUTHORED pipe half called `name`.
///
/// ⛔ this used to read `pipe_halves()`, a four-element Rust table — so the warp
/// trigger sat where the constants said a pipe was, and moving the pipe in the
/// editor would have left the trigger behind. It reads the block Jon drew.
fn mouth_of(name: &str) -> ae::Aabb {
    let (_, aabb) = authored_named_blocks()
        .get(name)
        .unwrap_or_else(|| panic!("the level authors a `{name}` pipe half"));
    mouth_band(
        aabb.min,
        aabb.max - aabb.min,
        name.ends_with(PIPE_MOUTH_DOWN_SUFFIX),
    )
}

/// The mouth of the descent tube — the open top of the pipe you stand on.
pub fn pipe_mouth() -> ae::Aabb {
    mouth_of(PIPE_NAME)
}

/// Where the descent tube drops you: out of its VAULT half's mouth, so you fall
/// out of a pipe you can see rather than materializing in open stone.
pub fn vault_arrival() -> ae::Vec2 {
    let (_, pipe) = authored_named_blocks()
        .get(VAULT_ENTRY_PIPE_NAME)
        .expect("the level authors the descent tube's vault half");
    ae::Vec2::new((pipe.min.x + pipe.max.x) * 0.5, pipe.max.y + 0.5 * T)
}

/// Where the ascent tube puts you: on top of its SURFACE half, directly above the
/// vault pipe you entered — twelve tiles further into the level than you went down.
pub fn pipe_arrival() -> ae::Vec2 {
    let (_, pipe) = authored_named_blocks()
        .get(SURFACE_EXIT_PIPE_NAME)
        .expect("the level authors the ascent tube's surface half");
    ae::Vec2::new((pipe.min.x + pipe.max.x) * 0.5, pipe.min.y - T)
}

/// The ascent tube's mouth — the open BOTTOM of the pipe hanging from the vault
/// ceiling. The same [`mouth_band`] rule the descent uses, so both ends of a trip
/// are one verb: **touch the mouth, press into it.**
///
/// It used to be a band across the vault FLOOR, several tiles below the pipe. A
/// trigger that far from the thing it triggers reads as loose no matter how tight
/// the band is — "Mary-O can be anywhere under the pipe and press up" is what a
/// floor band feels like when the pipe is overhead. Now the pipe hangs at head
/// height ([`vault_pipe_clearance`]) and you have to be at its lip.
pub fn vault_exit() -> ae::Aabb {
    mouth_of(EXIT_PIPE_NAME)
}

/// Build Mary-O's level 1-1 through the `ambition_platformer2d` umbrella surface ONLY.
///
/// The grammar, left to right:
///
/// 1. **Open teach** — a long flat run with nothing on it. You learn to move.
/// 2. **The first platform** — a lone ledge at jump height over SAFE ground.
///    Missing it costs nothing. This is where a player learns the jump ARC.
/// 3. **Pit rhythm** — pits of 2, then 3, then 5 tiles. Each charges more for the
///    previous one's lesson. The widest has a stepping stone in it: the arc you
///    practised over safe ground at step 2 is now load-bearing, exactly once.
/// 4. **The stair pyramid** — four steps up, a gap, four down. Your run-up decides
///    the landing.
/// 5. **The goal** — a tall pole. Its geometry is here; the SEQUENCE that plays
///    when you grab it is [`flag`], and [`goal_pole`] is the one place both agree
///    on where it stands.
/// **World 1-1, as Jon authors it.**
///
/// ⛔ **The level is `assets/worlds/mary_o.ldtk` now, not this function.** It used
/// to build ~330 lines of blocks from constants, and every runtime that cared
/// about a specific block re-derived its position from those same constants — so
/// the geometry was the shadow of the code rather than the other way round, and
/// nothing Jon could do in an editor would have moved it. He asked for the
/// opposite (2026-08-04): *"I would like to make maryo an ldtk level so I can
/// manually play with it and lay it out."*
///
/// What is left here is what LDtk has no vocabulary for, and each piece is
/// DERIVED FROM THE LOADED ROOM rather than from a constant — dressing the
/// authored blocks in their art, and the two walk-in zones to World 1-2.
///
/// ⚠ regenerate the file with
/// `python3 game/ambition_demo_mary_o/tools/author_mary_o_ldtk.py` only to
/// re-derive it from scratch; ordinary layout edits belong in the LDtk editor,
/// and the generator's constants are history the moment Jon touches it.
pub fn level_1_1() -> RoomSpec {
    let mut room = authored_room(LEVEL_1_1_ROOM_ID);
    room.metadata.mode = Some(MARY_O_MODE.to_string());
    dress_authored_blocks(&mut room);
    room.props.extend(scenery_for_authored_room(&room));

    // The two ends of the trip to World 1-2. Walk-in zones, not a third pipe:
    // the vault's own pipes answer a directional press (Jon's rule), and a
    // shaft in the floor is a different affordance rather than a competing one.
    //
    // ▢ these still derive from `vault_bounds()`. They are the last constant in
    // 1-1 and they want to be authored `LoadingZone` entities like everything
    // else; leaving them is deliberate scope, not an oversight.
    room.loading_zones
        .extend([descent_to_1_2(), surface_return_from_1_2()]);
    room
}

/// The authored world file. Embedded, so a demo that ships its own binary needs
/// no asset root — the same choice Sanic's speedway makes.
pub const MARY_O_WORLD_JSON: &str = include_str!("../assets/worlds/mary_o.ldtk");

/// Load one authored area out of [`MARY_O_WORLD_JSON`].
///
/// ⚠ **`.expect` on a level file is normally forbidden** (the LDtk authoring
/// contract says startup must print every validator error and exit nonzero, so a
/// bad edit does not become a panic mid-play). It is acceptable HERE only because
/// this file is EMBEDDED at compile time: a broken edit cannot reach a running
/// player without passing the build and this crate's tests first. The moment the
/// world is loaded from disk instead, this has to become a reported refusal.
fn authored_room(area: &str) -> RoomSpec {
    // ⛔ **THE READER SUPPLIES THE VOCABULARY, because the file cannot be read
    // without it.** `MaryOBlock` is Mary-O's own LDtk noun; conversion refuses an
    // identifier it has no converter for, loudly and by design. Doing this only
    // in `MaryORulesPlugin::build` meant every test, tool and probe that loads
    // the level directly got nine refusals — and the level is not readable
    // without its vocabulary in ANY of those contexts, so the load is where the
    // requirement belongs.
    let project = ambition_platformer2d::ldtk_map::LdtkProject::from_json_str(MARY_O_WORLD_JSON)
        .expect(
            "mary_o.ldtk parses (regen: game/ambition_demo_mary_o/tools/author_mary_o_ldtk.py)",
        );
    let room_set = project
        .to_room_set_with_entry(area, &ldtk_vocabulary::vocabulary())
        .unwrap_or_else(|errors| panic!("mary_o.ldtk converts to rooms: {errors:?}"));
    room_set
        .rooms
        .into_iter()
        .find(|room| room.id == area)
        .unwrap_or_else(|| panic!("mary_o.ldtk authors the `{area}` area"))
}

/// Paint the authored blocks that wear something other than their kind's art.
///
/// ⛔ **LDtk cannot author a block's colour, so the game says it here — BY NAME.**
/// That is the whole authored vocabulary at work: a warp pipe and the flagpole
/// are collision only (their look comes from the props below, laid over them), and
/// the vault's masonry is its own stone. Doing this from the loaded names rather
/// than at construction is what lets Jon add a fourth pipe and have it dressed.
fn dress_authored_blocks(room: &mut RoomSpec) {
    for block in &mut room.world.blocks {
        if block.name.starts_with(WARP_PIPE_PREFIX) || block.name.starts_with(GOAL_POLE_PREFIX) {
            block.art_color = Some(scenery::TRANSPARENT);
        } else if block.name.starts_with(VAULT_MASONRY_PREFIX) {
            block.art_color = Some(VAULT_STONE_COLOR);
        }
    }
}

/// Every authored block whose name starts with `prefix`, as `(name, min, size)`.
fn authored_blocks_named(room: &RoomSpec, prefix: &str) -> Vec<(String, ae::Vec2, ae::Vec2)> {
    let mut out: Vec<_> = room
        .world
        .blocks
        .iter()
        .filter(|block| block.name.starts_with(prefix))
        .map(|block| {
            (
                block.name.clone(),
                block.aabb.min,
                block.aabb.max - block.aabb.min,
            )
        })
        .collect();
    // Sorted so the props a level produces do not depend on block order.
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// The flagpole and warp-pipe LOOK: decorative props laid over the transparent
/// collision blocks the file authors. Presentation only — none of it changes
/// geometry, the grab band, or the warp mouths.
///
/// ⭐ **derived from the loaded room**, so moving a pipe in the editor moves its
/// art with it and a new pipe is dressed without a line of Rust.
fn scenery_for_authored_room(room: &RoomSpec) -> Vec<PropSpec> {
    let mut props = Vec::new();

    for (name, min, size) in authored_blocks_named(room, GOAL_POLE_PREFIX) {
        match name.as_str() {
            // BUILT WORLD, not scenery: character sizing derives a sprite's width
            // from the box's LONGEST side, so a shaft whose box is nine tiles tall
            // was drawn eighteen tiles WIDE. It stays behind the cast, though —
            // she has to be visible climbing it.
            "goal_pole" => props.push(scenery::structure_prop(
                "goal_pole_shaft_art",
                scenery::POLE_BODY_SPRITE,
                min,
                size,
            )),
            "goal_pole_knob" => props.push(scenery::prop_over(
                "goal_pole_finial_art",
                scenery::POLE_TOP_SPRITE,
                min - ae::Vec2::new(size.x * 0.5, size.y * 0.5),
                size * 2.0,
            )),
            // The banner: wider than the pole and hung to the right of the shaft
            // top, so the flag reads as a flag without widening what she can touch.
            "goal_pole_banner" => props.push(PropSpec {
                id: "goal_pole_banner_art".to_string(),
                name: "goal_pole_banner_art".to_string(),
                kind: scenery::FLAG_SPRITE.to_string(),
                pos: min + ae::Vec2::new(T - size.x * 0.5, T - size.y * 0.5),
                size: ae::Vec2::splat(1.5 * T),
                flip_y: false,
                draw: Default::default(),
            }),
            _ => {}
        }
    }

    // Every pipe, tiled over its OWN block: one LIP tile at the open end and body
    // tiles filling the rest. The lip is what makes a pipe point somewhere — top
    // tile for a mouth-up half (you drop in), BOTTOM tile for a mouth-down one
    // (hanging from the ceiling, you fall out of it or rise into it), mirrored to
    // match.
    for (name, min, size) in authored_blocks_named(room, WARP_PIPE_PREFIX) {
        let mouth_down = name.ends_with(PIPE_MOUTH_DOWN_SUFFIX);
        // Laid FROM THE MOUTH inward, so the lip sits exactly on the open face
        // however long the half is — a pipe's length is set by where its mouth
        // has to be, not by a whole number of tiles, and only the far end (which
        // meets the ground slab and is never seen end-on) takes the remainder.
        let mut laid = 0.0f32;
        let mut row = 0usize;
        while laid < size.y - 0.5 {
            let height = (size.y - laid).min(T);
            let top = if mouth_down {
                min.y + size.y - laid - height
            } else {
                min.y + laid
            };
            let at = ae::Vec2::new(min.x, top);
            let tile = ae::Vec2::new(size.x, height);
            laid += height;
            row += 1;
            if row == 1 {
                // A mouth-down pipe is the SAME head sheet, mirrored — a pipe head
                // drawn right way up under a ceiling reads as standing on nothing.
                props.push(scenery::pipe_prop(
                    &format!("{name}_lip_art"),
                    scenery::PIPE_TOP_SPRITE,
                    at,
                    tile,
                    mouth_down,
                ));
            } else {
                props.push(scenery::pipe_prop(
                    &format!("{name}_body_art_{row}"),
                    scenery::PIPE_BODY_SPRITE,
                    at,
                    tile,
                    false,
                ));
            }
        }
    }
    props
}

/// The open shaft at the vault's far end that drops into World 1-2.
///
/// It sits ON the vault floor. The return pipe shipped floating 48px clear of
/// its own band (`cbc6902d2`) and its test passed anyway by probing a point
/// inside solid rock, so "does this thing meet the floor" is now something both
/// rooms assert.
pub fn descent_to_1_2() -> ambition_platformer2d::world::rooms::LoadingZone {
    let vault = vault_bounds();
    let size = ae::Vec2::new(T, 1.5 * T);
    let center = ae::Vec2::new(vault.max.x - 1.5 * T, vault.max.y - size.y * 0.5);
    ambition_platformer2d::world::rooms::LoadingZone {
        id: level_1_2::DESCENT_ZONE_ID.to_string(),
        name: "Down to 1-2".to_string(),
        activation: ambition_platformer2d::world::rooms::LoadingZoneActivation::Walk,
        aabb: ae::Aabb::new(center, size * 0.5),
    }
}

/// Where 1-2 puts you back on the surface: past pit B, on the long run before
/// the stair pyramid. Going underground is a SHORTCUT — you skip two pits — so
/// the route competes with the surface run instead of merely detouring from it.
pub fn surface_return_from_1_2() -> ambition_platformer2d::world::rooms::LoadingZone {
    let ground_top = SURFACE_HEIGHT - GROUND_TILES * T;
    let size = ae::Vec2::new(T, 1.5 * T);
    let center = ae::Vec2::new(57.0 * T, ground_top - size.y * 0.5);
    ambition_platformer2d::world::rooms::LoadingZone {
        id: level_1_2::SURFACE_RETURN_ZONE_ID.to_string(),
        name: "Back to the surface".to_string(),
        activation: ambition_platformer2d::world::rooms::LoadingZoneActivation::Walk,
        aabb: ae::Aabb::new(center, size * 0.5),
    }
}
// ── The authored blocks, resolved by NAME ──────────────────────────────────
//
// ⛔ **These used to CONSTRUCT ids from constants; they now LOOK THEM UP in the
// authored file.** `power_block_id(i)` was
// `GeoId::tile_layer("mary_o_ground", 10 + i)` — an id computed from an index
// into a Rust array, which is why nothing Jon did in an editor could move a
// ?-block: the level drew one place and the runtime matched another. The id is
// whatever the file says it is now (`GeoId::placement(<the LDtk iid>)`), and the
// position is the authored block's own corner.
//
// ⚠ **the index survives on purpose, for now.** Every consumer — the spent set,
// the broken-brick bitset, the dresser — is keyed by `i`, and `i` is the suffix
// of the authored name (`power_block_2` is index 2). That keeps this change to
// the LOOKUP and leaves the rollback-state shapes alone. It also means the
// suffixes must stay dense from 0; `authored_family_count` is what notices when
// they do not.

/// The authored block a CONTACT names, looked up in the room the player is in.
///
/// ⛔ **this is what replaced the index tables.** `ContactSource::Block` carries a
/// durable `GeoId` and nothing else, so a system that wants to know *what it
/// hit* has to ask the room. It used to answer by comparing the id against ids
/// RECONSTRUCTED from constant arrays (`power_block_id(i)` for i in 0..3), which
/// is why nothing an author did could move a ?-block: the level drew one place
/// and the runtime matched another.
pub fn authored_block_by_id<'a>(world: &'a ae::World, id: &ae::GeoId) -> Option<&'a ae::Block> {
    world.blocks.iter().find(|block| block.id == *id)
}

/// Every authored block in the embedded world file, by name.
///
/// ⚠ a process-global `LazyLock` over a `const &str`, which is safe for the
/// reason a `OnceLock` fed by a provider is not: the input is fixed at COMPILE
/// time, so there is no second value it could ever hold and no install order to
/// get wrong.
fn authored_named_blocks() -> &'static std::collections::BTreeMap<String, (ae::GeoId, ae::Aabb)> {
    static BLOCKS: std::sync::LazyLock<std::collections::BTreeMap<String, (ae::GeoId, ae::Aabb)>> =
        std::sync::LazyLock::new(|| {
            authored_room(LEVEL_1_1_ROOM_ID)
                .world
                .blocks
                .iter()
                .map(|block| (block.name.clone(), (block.id.clone(), block.aabb)))
                .collect()
        });
    &BLOCKS
}

// ⭐⭐ **THE INDEX HELPERS ARE GONE (2026-08-04)** — `power_block_id`,
// `power_block_min`, `power_block_index_for` and their quasar and brick twins,
// plus `brick_name`, `brick_count` and `BRICK_CAPACITY`.
//
// They answered "which position in a Rust array is this block", which is a
// question with no meaning once an author places the blocks: a converter sees one
// entity and cannot know an ordinal, and inserting a block would renumber every
// one after it — including the ones a rollback resource had already recorded as
// spent or broken.
//
// What replaced them is [`authored_block_by_id`] plus
// [`ldtk_vocabulary::block_look_of`]: ask the ROOM which block was struck, then
// ask the BLOCK what kind it is. Position comes from the block's own `aabb`, so a
// block dragged in the editor pops its reward where it now sits.

/// The pole's geometry, derived from the SAME constants [`level_1_1`] builds the
/// `goal_pole` block out of. A second source of truth for where the flag is would
/// be a bug that only surfaces after someone moves the level.
pub fn goal_pole() -> flag::FlagPole {
    let ground_top = SURFACE_HEIGHT - GROUND_TILES * T;
    flag::FlagPole {
        // `Block::one_way` takes a MIN corner; the pole is `POLE_WIDTH` wide.
        x: POLE_COLUMN * T + POLE_WIDTH * 0.5,
        top_y: ground_top - 9.0 * T,
        base_y: ground_top,
        half_width: POLE_WIDTH * 0.5,
    }
}

/// The goal pole's tile column — ONE source of truth shared by [`goal_pole`] and
/// the `goal_pole` block in [`level_1_1`], so moving the level can never leave the
/// flag's read-model behind (the drift the pole oracle guards against). Column 98
/// after the level was lengthened by 8 tiles for the vault.
const POLE_COLUMN: f32 = 98.0;

/// **Which pole a room finishes on.**
///
/// ⛔ the pole was inserted at plugin build as 1-1's, unconditionally — so the
/// entry-room seam could put a session in another room and that session had no
/// reachable goal at all. `run_flag_sequence` compares her position against this
/// resource and nothing else, so the failure is silent: the level simply never
/// ends. Which room she is in already decides which world she gets
/// ([`provider::mary_o_session_world_entering`]); it decides the goal by the same
/// answer here.
pub fn pole_for_room(room_id: &str) -> flag::FlagPole {
    if room_id == test_course::TEST_COURSE_ROOM_ID {
        test_course::course_pole()
    } else {
        goal_pole()
    }
}

/// Install the entry room's pole once the session's choice is readable.
///
/// Startup rather than plugin build, because [`provider::MaryOEntryRoom`] is a
/// resource a host inserts into the built app — the same lifetime the world
/// source reads it on. Absent means 1-1, for the reason the resource's own doc
/// gives: a shipped game must not depend on something only a test inserts.
fn install_goal_pole(
    mut commands: bevy::prelude::Commands,
    entry: Option<bevy::prelude::Res<provider::MaryOEntryRoom>>,
) {
    commands.insert_resource(pole_for_room(
        entry
            .as_ref()
            .map_or(LEVEL_1_1_ROOM_ID, |room| room.0.as_str()),
    ));
}

/// **Mary-O Classic's movement profile, authored ONCE.**
///
/// Every form she wears — small, tall, fire — must move identically; growing
/// changes her LOOK and size, never her physics. That was previously three
/// hand-copied blocks, where a one-line divergence would silently shrink her
/// jump on a power-up. It is now one string substituted into each row, so the
/// forms cannot disagree by construction rather than by test.
///
/// The numbers are the classic 16 px / 60 Hz tables converted to Mary-O's 32 px
/// tile scale. The target is that she plays effectively the same as the original
/// (Jon); the point of the demo is to show the ENGINE can express that as
/// parameters, so any deviation has to earn itself.
///
/// `ground_coast_decel` is the FAITHFUL conversion: classic friction equals
/// classic walk acceleration, so releasing the stick coasts to a stop in ~0.76 s
/// over ~3.6 tiles. It was previously stiffened to 1200 because that slide read
/// as skating — but "we deliberately flourished here" is not a licence to stop
/// converging (Jon), so the deviation is gone and the classic slide is back.
///
/// ⚠ `ground_reverse_accel` (1500) is the one number still NOT sourced from the
/// classic tables — the skid rate wants the real SMB1/SMB3 subpixel constant
/// rather than a picked value. It is deliberately left visible here rather than
/// blessed: converging it is a known outstanding item, not a settled choice.
///
/// Neutral AIR preserves momentum exactly, which IS faithful and is not in
/// question.
///
/// The jump law picks one of four launch bands from body-local side speed, then
/// runs weak gravity while the button is held and the body is rising, and full
/// gravity after release or near apex. `speed_thresholds` are the converted
/// classic cuts (1.0 / 1.5625 / 1.75 px-per-frame at 16 px scale), which puts
/// the TOP band inside her 300 px/s run — a running jump is the highest jump,
/// exactly as in the original. `launch_offsets` ride on `jump_speed`, so that
/// one knob still moves her whole jump family together.
///
/// `coyote_time` and `jump_buffer` are 0 ON PURPOSE (confirmed by Jon): the
/// original grants no ledge forgiveness and no pre-landing buffer, so neither
/// does she. Do not "fix" these.
///
/// All directions are interpreted through the resolved gravity frame.
const MARY_O_CLASSIC_AXIS_TUNING: &str = r#"(
                horizontal_law: Momentum((
                    ground_reverse_accel: 1500.0,
                    ground_coast_decel: 393.75,
                    air_reverse_accel: 900.0,
                    air_coast_decel: 0.0,
                )),
                jump_law: PhasedGravity((
                    speed_thresholds: (120.0, 187.5, 210.0),
                    launch_offsets: (-30.0, -15.0, 0.0, 30.0),
                    held_rise_gravity_scale: 0.2,
                    released_rise_gravity_scale: 1.0,
                    fall_gravity_scale: 1.0,
                    held_phase_min_upward_speed: 240.0,
                )),
                gravity: 2250.0,
                air_jumps: 0,
                jump_speed: 450.0,
                max_run_speed: 300.0,
                run_accel: 393.75,
                air_accel: 393.75,
                max_fall_speed: 480.0,
                coyote_time: 0.0,
                jump_buffer: 0.0,
            )"#;

/// Assemble the demo catalog, substituting the one authored movement profile
/// into every Mary-O form. `str::replace` rather than `format!` because the RON
/// is full of braces that would all need escaping.
fn mary_o_catalog_ron() -> String {
    MARY_O_CATALOG_RON_TEMPLATE.replace("$CLASSIC_AXIS_TUNING", MARY_O_CLASSIC_AXIS_TUNING)
}

/// The demo's one-character catalog. Every demo installs its own roster; the
/// engine ships none (ADR 0017).
const MARY_O_CATALOG_RON_TEMPLATE: &str = r#"(
    brain_presets: { "stand_still": StandStill },
    action_set_presets: {
        "peaceful": (
            move_style: Walk,
            melee: None,
            ranged: None,
            special: None,
        ),
    },
    characters: {
        "mary_o": (
            display_name: "Mary-O",
            spritesheet: "sprites/mary_o_v2_spritesheet.png",
            manifest: "sprites/mary_o_v2_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            // Mary-O Classic is deliberately only the run/jump floor. Wall jump
            // and ground pound are later, independent abilities; the existing
            // Hollow-Knight-style wall bundle and generic fast fall must not leak
            // into the core movement oracle.
            abilities: Some([RunJump]),
            // Reusable AxisSwept laws. Launch, gravity, and acceleration start
            // from the classic 16 px / 60 Hz tables converted to Mary-O's 32 px
            // tile scale. Ground coast and reversal are deliberately polished:
            // the direct friction conversion took about 0.76 seconds and 3.6
            // tiles to stop from full speed, which read as excessive sliding in
            // Ambition. Neutral air still preserves momentum. The jump law selects
            // one of four launch speeds from body-local side speed, then uses weak
            // gravity while held and rising and full gravity after release / near
            // apex.
            // All directions are interpreted through the resolved gravity frame.
            axis_tuning: Some($CLASSIC_AXIS_TUNING),
            // The classic contract: whatever you are wearing absorbs the hit
            // (beacon -> wand -> nothing), and once there is no armor left the
            // next one is fatal. One pool, authored on every form, so growing
            // changes what a hit COSTS and never how much punishment the body
            // underneath can take.
            max_health: Some(1),
            playable_kit: Authored,
            tags: ["player"],
            barks: (
                hall: ["I solve masonry disputes from below.", "One jump. No second opinions, no insurance.", "Every pipe is hiding something."],
            ),
            hall_dialogue_id: Some("hall_mary_o"),
        ),
        // TALL Mary-O: the grown form. A wand-powerup swaps the worn identity to
        // this row (a distinct SHEET — `mary_o_v2_tall` — not a scaled copy of
        // the small sheet, per Jon), and the powerup runtime bumps her body size so
        // the taller art draws bigger. Kit is byte-identical to `mary_o` — same
        // grant list, same Mary-O Classic `axis_tuning` (re-wearing re-reads
        // `axis_tuning`, so a mismatch here would silently shrink her jump on grow)
        // and the same peaceful Authored kit — so growing changes only her LOOK and
        // size, never her moveset.
        "mary_o_tall": (
            display_name: "Mary-O (Tall)",
            spritesheet: "sprites/mary_o_v2_tall_spritesheet.png",
            manifest: "sprites/mary_o_v2_tall_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            abilities: Some([RunJump]),
            axis_tuning: Some($CLASSIC_AXIS_TUNING),
            // The classic contract: whatever you are wearing absorbs the hit
            // (beacon -> wand -> nothing), and once there is no armor left the
            // next one is fatal. One pool, authored on every form, so growing
            // changes what a hit COSTS and never how much punishment the body
            // underneath can take.
            max_health: Some(1),
            playable_kit: Authored,
            tags: ["player"],
            barks: (
                hall: ["One power-up; every ceiling gets an opinion.", "Ask the doorframes whether taller is better.", "I shrink after one professional-grade mistake."],
            ),
            hall_dialogue_id: Some("hall_mary_o_tall"),
        ),
        // FIRE Mary-O: the cinder beacon (fire-flower) form. A second power-up
        // ABOVE the wand swaps the worn identity to this row — a DISTINCT fire sheet
        // (`mary_o_v2_fire`, the white-and-red fire palette with its own
        // fireball pose), the SAME height as the grown form so `sync_grown_form`
        // changes only her LOOK + spark loadout, never her size, on the
        // grown↔fire transition. Kit mirrors `mary_o_tall` byte-for-byte: the
        // fireball is granted by WEARING the cinder beacon (see `MaryOSpark`), not
        // by this row, so becoming fire never alters her base moveset. Before this
        // she wore the plain tall sheet while spark-powered, so there was no
        // visible fire form at all (Jon bug #10).
        "mary_o_fire": (
            display_name: "Mary-O (Fire)",
            spritesheet: "sprites/mary_o_v2_fire_spritesheet.png",
            manifest: "sprites/mary_o_v2_fire_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            abilities: Some([RunJump]),
            axis_tuning: Some($CLASSIC_AXIS_TUNING),
            // The classic contract: whatever you are wearing absorbs the hit
            // (beacon -> wand -> nothing), and once there is no armor left the
            // next one is fatal. One pool, authored on every form, so growing
            // changes what a hit COSTS and never how much punishment the body
            // underneath can take.
            max_health: Some(1),
            playable_kit: Authored,
            tags: ["player"],
            barks: (
                hall: ["One beacon, and every ceiling gets a warm answer.", "I throw solutions now — mind the sparks.", "Fireproof opinions, freshly lit."],
            ),
        ),
        // Solid Snake's IDENTITY row (the Koopa-equivalent): its sprite resolves
        // from this display name. The `solid_snake` sheet carries the shell-withdraw
        // rows (retreat / boxed_idle / peek / emerge) the in-place shell drives.
        // Behavior/HP/contact come from the `mary_o_snake` ROSTER archetype (see
        // `snake.rs`), not this catalog row — this is only the sprite + name.
        "solid_snake": (
            display_name: "Solid Snake",
            spritesheet: "sprites/solid_snake_spritesheet.png",
            manifest: "sprites/solid_snake_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            tags: ["enemy"],
            fallback_dialogue: [
                "The shell is load-bearing. Please stop.",
                "I was told there would be a corridor.",
                "Kick me and I become somebody else's problem.",
            ],
        ),
        // AI Slop's IDENTITY row (the plain stompable walker): its sprite is the
        // published `ai_slop` sheet, resolved from this display name. Behavior/HP/
        // contact come from the `mary_o_ai_slop` ROSTER archetype (see `ai_slop.rs`),
        // not this catalog row — this is only the sprite + name.
        //
        // AI Slop also appears in Ambition's Hall of Characters (a frozen display
        // NPC, `npc_ai_slop`). They are the SAME character in different modes; ideally
        // one shared catalog entry both experiences draw from, with each supplying its
        // own behavior (the Hall freezes it; Mary-O makes it a stompable walker). Until
        // that unification lands they are two rows — distinct catalog ids, and distinct
        // display strings so the assembled catalog's display-name uniqueness holds when
        // hosted. Nothing stops Mary-O from spawning the Hall's `npc_ai_slop` directly
        // when Ambition is loaded; ids are the cross-provider identity.
        "ai_slop": (
            display_name: "AI Slop",
            spritesheet: "sprites/ai_slop_spritesheet.png",
            manifest: "sprites/ai_slop_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            tags: ["enemy"],
            fallback_dialogue: [
                "I walk left. It is a complete philosophy.",
                "Statistically, one of us is about to be stomped.",
                "I was trained on a thousand walkers and became the average one.",
            ],
        ),
    },
)"#;

/// Content plugin: registers Mary-O's App-local character fragment, installs
/// the level, and adds the engine's sim-world setup. The shape `crates/ambition_platformer2d_host/tests/demo_shell_smoke.rs` prescribes.
pub struct MaryODemoContentPlugin;

/// Register Mary-O's immutable authored character fragment in one Bevy `App`.
/// Shared by the historical [`MaryODemoContentPlugin`] (Startup construction) and
/// the new [`provider::MaryOExperiencePlugin`] (shell-activation construction).
pub fn install_mary_o_content(app: &mut App) {
    use ambition_platformer2d::characters::actor::character_catalog::{
        CharacterCatalogAppExt, CharacterCatalogFragment,
    };

    app.register_character_catalog_fragment(
        CharacterCatalogFragment::from_ron(
            provider::MARY_O_EXPERIENCE,
            Some(provider::MARY_O_CHARACTER_ID),
            &mary_o_catalog_ron(),
        )
        .expect("Mary-O character catalog should be valid"),
    );
    // §7.6: the ONE character seam. Mary-O and her grown form each register as a
    // single definition, which publishes the prepared authority AND demands their
    // art -- so this provider no longer names sheets, checks whether they bound, or
    // knows that art and gameplay numbers are consumed by different subsystems.
    //
    // Deliberately BOTH forms: a runtime growth into `mary_o_tall` is a different
    // character definition, not a mode of this one (§4.3), and it needs its own art
    // demanded or the grown Mary-O draws a placeholder.
    {
        use ambition_platformer2d::actors::character_runtime::{
            CharacterDefinition, CharacterDefinitionAppExt,
        };
        // The sheet TARGET, not the sheet file: `mary_o_v2_spritesheet.ron`
        // declares `target: "mary_o_v2"`, and the registry is keyed by the target.
        // A VOICE apiece — see the same note in the Sanic provider. Without one a
        // registered-only character has no bark pool anywhere, and the Hall's
        // ambient ticker skips whoever has nothing to say.
        //
        // **All THREE forms, each handing its body to its own sheet.** Growing
        // is a change of ART, and until now her boxes were hand-guessed against
        // it: small was the engine's default 30x48 and tall was that times an
        // authored 1.5, while the sheets say 11x16 and 12x22 pixels — a real
        // ratio of 1.375. Two authorities for one silhouette, so the render had
        // to reconcile them with a scale factor, and that factor is what drew
        // her tall form far bigger than the body it belonged to.
        //
        // One `world_per_pixel` for all three is the point, not a shortcut: the
        // forms differ in SIZE because their art differs, at a shared scale.
        // Authoring a per-form number would put the ratio back.
        for (id, display, sheet, voice) in [
            (
                provider::MARY_O_CHARACTER_ID,
                "Mary-O",
                powerups::SMALL_SHEET_TARGET,
                [
                    "Jump, land, repeat. It's honest work.",
                    "The bricks owe me nothing and I break them anyway.",
                    "Every pipe goes somewhere. That's the deal.",
                ],
            ),
            (
                "mary_o_tall",
                "Mary-O (Tall)",
                powerups::TALL_SHEET_TARGET,
                [
                    "One mushroom. That's the whole story.",
                    "Taller, yes. Braver, unclear.",
                    "I can see the top of the flagpole from here.",
                ],
            ),
            // The fire form was never registered at all — it had a catalog row
            // and a sheet but no definition, so it was the one form whose art
            // nothing demanded and whose body nothing could author.
            (
                "mary_o_fire",
                "Mary-O (Fire)",
                powerups::FIRE_SHEET_TARGET,
                [
                    "The beacon does the talking now.",
                    "Warm opinions, thrown at speed.",
                    "Everything flammable, please step back.",
                ],
            ),
        ] {
            app.register_character(
                CharacterDefinition::new(id, display, provider::MARY_O_EXPERIENCE)
                    .with_sheet(sheet)
                    .with_sprite_authored_body(powerups::mary_o_world_per_pixel())
                    .with_voice(voice),
            );
        }
    }
    // Mary-O's two enemies — Solid Snake (the shell) and AI Slop (the plain
    // stomp-and-die walker) — are authored content, so install their archetypes and
    // room stagers before direct or shell preparation fingerprints the App. Both
    // archetypes share ONE roster fragment: assembly rejects a second fragment from
    // the same provider, so the per-enemy `register_*_roster` helpers (used by
    // single-enemy tests) are folded here.
    {
        use ambition_platformer2d::actors::features::{
            CharacterRosterAppExt, CharacterRosterFragment,
        };
        app.register_character_roster_fragment(
            CharacterRosterFragment::from_ron(
                provider::MARY_O_EXPERIENCE,
                None::<String>,
                &format!(
                    "{{{}{}}}",
                    snake::SNAKE_ROSTER_ROWS,
                    ai_slop::AI_SLOP_ROSTER_ROWS
                ),
            )
            .expect("Mary-O enemy roster should be valid"),
        );
    }
    // ⛔ **Mary-O stages NO enemies of her own, and must not.** She used to
    // register two `RoomContentStagingRegistry` closures that walked
    // `room.enemy_spawns` and minted a `SpawnActorRequest` per matching brain.
    // That was correct while the level authored no enemies — but once 1-1 moved
    // into LDtk and DID author them, the engine's `authored_actor_requests`
    // built one actor per placement and these closures built a second under a
    // prefixed id. Seventeen placements, thirty-four actors, and only the
    // prefixed half carried `SnakeShell` / `AiSlop`, so half of 1-1's enemies
    // were un-stompable lookalikes. The ids differed, so the construction plan's
    // duplicate-id check could not see it.
    //
    // ⭐ **one authored placement, one root.** The engine builds every authored
    // enemy; this crate only decides what its own archetypes MEAN, in the tag
    // passes, keyed off `ActorConfig.brain`. The staging registry itself is
    // untouched and still right for content a room does not author (the duel
    // arena uses it exactly that way) — Mary-O's enemies are simply not that
    // anymore.
    app.init_resource::<ambition_platformer2d::actors::features::RoomContentStagingRegistry>();
    // The flagpole + warp-pipe LOOK: load the construction sheets into
    // `GameAssets.props` so the decorative props authored on the level resolve to
    // real art instead of the placeholder quad. Presentation-only, so it rides the
    // plain `Update` schedule (self-heals after a `GameAssets` rebuild).
    app.add_systems(
        bevy::prelude::Update,
        (
            scenery::register_mary_o_construction_props,
            // Each enemy owns its own sheet so its bodies never fall back to the
            // generic goblin — the deferred room-staging barrier that would load
            // them lives in the app host and isn't reliably driven for a demo-staged
            // enemy.
            snake::register_solid_snake_sheet,
            ai_slop::register_ai_slop_sheet,
            // ⛔ **the bonus blocks' LOOK, and it was in the SIM chain first.**
            // Registered beside `bonk_power_blocks` because that is where the
            // powerup rules live — and it mutates RENDER entities, from inside
            // the rollback schedule, which is a category error dressed as
            // tidiness. It drew nothing, the crate's tests passed, and only a
            // capture of the running demo showed the `?`-blocks as plain grey
            // tiles (queue D11/D20, 2026-08-04).
            //
            // Presentation reads sim state and writes render components, in
            // `Update`, exactly like the two sheet registrations above it.
            powerups::dress_power_blocks,
        ),
    );

    // Mary-O's mutable sim state joins the rollback contract through the same
    // seam engine crates use — here, before either construction path
    // fingerprints the App, so the schema fingerprint (part of the content
    // identity) includes these rows; a non-GGRS shell records metadata only.
    {
        use ambition_platformer2d::runtime::rollback::AmbitionRollbackApp;
        // The ANCHOR comes first: the mode owner is a bare state-holder entity
        // — no body, no projectile, no feature marker — so none of the
        // engine's rollback anchors reach it, and a registered-but-unanchored
        // component silently never snapshots (found by the behavioral restore
        // test: a dirty score survived a GGRS rollback).
        app.require_rollback::<MaryOLevelState>("ambition_demo_mary_o", "entity:mary_o_mode_owner")
            .rollback_component_clone::<MaryOLevelState>(
                "ambition_demo_mary_o",
                "content.mary_o_level_state",
            )
            .rollback_component_clone::<flag::FlagSequence>(
                "ambition_demo_mary_o",
                "content.mary_o_flag_sequence",
            )
            // The death beat rides the same owner entity as those two and was
            // simply missed: it decides how long the level is held, where the
            // body is pinned, and whether the replay has been asked for, none of
            // which a rewind could reproduce without it.
            .rollback_component_clone::<death::MaryODeathSequence>(
                "ambition_demo_mary_o",
                "content.mary_o_death_sequence",
            )
            // A snake's shell phase (and its stage timers) is authoritative sim
            // state — two sims that disagree on where a shell is in its withdraw
            // are in different states. It rides on the snake BODY, which the
            // engine already anchors, so a plain component clone snapshots it.
            .rollback_component_clone::<snake::SnakeShell>(
                "ambition_demo_mary_o",
                "content.mary_o_snake_shell",
            )
            // The AI Slop marker rides on the enemy BODY (already anchored). It is a
            // bare tag, but snapshotting it keeps the stomp-eligible set identical
            // across a rollback rather than relying on the re-tag pass to converge.
            .rollback_component_clone::<ai_slop::AiSlop>(
                "ambition_demo_mary_o",
                "content.mary_o_ai_slop",
            )
            // A burning star is authoritative sim state by the strictest reading
            // of it: while it runs, hits do not land, and anything that can make
            // a hit be IGNORED has to survive a rewind or the two sims disagree
            // about whether the player took damage. It rides the player BODY,
            // which the engine already anchors.
            //
            // Its own doc comment said this and it was not done — which matters
            // more than the tidiness: `BodyOffense` IS rollback state, so a star
            // that vanishes across a restore leaves the fact it was writing
            // un-refreshed, and the invincibility silently stops without the
            // pickup ever having been spent.
            // A transit in flight is authoritative sim state — it OWNS the body's
            // position for half a second, so a rewind that dropped it would put a
            // half-swallowed player back on the surface. It rides on the player
            // BODY, which the engine already anchors.
            .rollback_component_clone::<pipe::PipeTransit>(
                "ambition_demo_mary_o",
                "content.mary_o_pipe_transit",
            )
            // WHICH blocks are spent is authoritative: a rewind across the frame
            // a block was struck must leave that block ARMED again, or the same
            // bonk on the re-simulated timeline finds a block that already gave
            // up its pickup and the two sims disagree about what is in the room.
            // (GPT review of 5cc4337..47d7de3, finding 1.)
            .rollback_resource_clone::<powerups::SpentPowerBlocks>(
                "ambition_demo_mary_o",
                "content.mary_o_spent_power_blocks",
            )
            // ⭐ Its BRICK twin, and the same argument exactly: which bricks are
            // broken decides what the room is MADE OF — the feature overlay
            // subtracts them from collision — so a rewind that left a brick
            // broken puts a hole in a wall the other timeline still has. Found
            // by the shipped-composition resource sweep rather than by review;
            // the sandbox sweep could never have seen it, because this resource
            // only exists in Mary-O's composition.
            .rollback_resource_clone::<bricks::BrokenBricks>(
                "ambition_demo_mary_o",
                "content.mary_o_broken_bricks",
            )
            .rollback_component_clone::<pipe::PipeEntryLatch>(
                "ambition_demo_mary_o",
                "content.mary_o_pipe_entry_latch",
            )
            // The spark cadence GATES whether a press fires, so it is
            // authoritative: a rewind that restored input and live sparks but
            // left this at its future value would swallow the replayed press and
            // diverge. It rides on the player BODY, which the engine anchors.
            // Its sibling `MaryOGait` is deliberately NOT here — every field on
            // it is rebuilt from the current tick's control frame.
            .rollback_component_clone::<movement::MaryOSparkCooldown>(
                "ambition_demo_mary_o",
                "content.mary_o_spark_cooldown",
            );
    }
}

impl Plugin for MaryODemoContentPlugin {
    fn build(&self, app: &mut App) {
        use ambition_platformer2d::runtime::demo_fixture::{ActiveRoomMetadata, RoomSet};
        use bevy::prelude::IntoScheduleConfigs;

        install_mary_o_content(app);
        quasar_shader::install(app);
        let room = level_1_1();
        let source = ambition_platformer2d::runtime::PreparedPlatformerSource::new(
            provider::MARY_O_EXPERIENCE,
            RoomSet::from_parts(LEVEL_1_1_ROOM_ID, vec![room.clone()], Vec::new()),
            ae::RoomGeometry(room.world.clone()),
            ActiveRoomMetadata(room.metadata.clone()),
            ambition_platformer2d::runtime::demo_fixture::StartingCharacter::new(
                provider::MARY_O_CHARACTER_ID,
            ),
            ambition_platformer2d::runtime::demo_fixture::LdtkRuntimeIndex::default(),
        );
        let content = ambition_platformer2d::provider::prepare_platformer_content_for_app(
            app,
            source,
            &provider::mary_o_authored_catalogs(),
        )
        .expect("Mary-O direct prepared-content assembly must succeed");
        app.world_mut().spawn((
            ambition_platformer2d::platformer::lifecycle::SessionRoot(
                ambition_platformer2d::platformer::lifecycle::SessionScopeId(0),
            ),
            content.source().instantiate_live(),
            content.identity(),
            content,
        ));
        app.add_systems(
            bevy::app::Startup,
            mary_o_setup.in_set(ambition_platformer2d::runtime::demo_fixture::SimulationSetupSet),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn mary_o_setup(
    mut commands: bevy::prelude::Commands,
    world: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<ae::RoomGeometry>,
    room_set: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<
        ambition_platformer2d::runtime::demo_fixture::RoomSet,
    >,
    ldtk_index: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<
        ambition_platformer2d::runtime::demo_fixture::LdtkRuntimeIndex,
    >,
    editable_abilities: bevy::prelude::Res<
        ambition_platformer2d::runtime::demo_fixture::EditableAbilitySet,
    >,
    tuning: bevy::prelude::Res<ambition_platformer2d::runtime::demo_fixture::ActiveMovementTuning>,
    starting_character: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<
        ambition_platformer2d::runtime::demo_fixture::StartingCharacter,
    >,
    asset_server: bevy::prelude::Res<bevy::asset::AssetServer>,
    character_catalog: bevy::prelude::Res<
        ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog,
    >,
    prepared_characters: Option<
        bevy::prelude::Res<
            ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry,
        >,
    >,
    authored_sheets: bevy::prelude::Res<
        ambition_platformer2d::actors::character_sprites::AuthoredSheets,
    >,
    character_roster: bevy::prelude::Res<ambition_platformer2d::actors::features::CharacterRoster>,
    boss_catalog: bevy::prelude::Res<ambition_platformer2d::actors::boss_encounter::BossCatalog>,
    placement_lowering: bevy::prelude::Res<
        ambition_platformer2d::runtime::demo_fixture::PlacementLoweringRegistry,
    >,
    content_staging: bevy::prelude::Res<
        ambition_platformer2d::runtime::demo_fixture::RoomContentStagingRegistry,
    >,
    construction_recipes: bevy::prelude::Res<
        ambition_platformer2d::runtime::demo_fixture::ActorConstructionRegistry,
    >,
) {
    ambition_platformer2d::runtime::demo_fixture::simulation_world(
        &mut commands,
        ambition_platformer2d::platformer::lifecycle::SessionSpawnScope::UNSCOPED,
        ambition_platformer2d::runtime::demo_fixture::SimulationSetup {
            world: &world,
            room_set: &room_set,
            ldtk_index: &ldtk_index,
            editable_abilities: &editable_abilities,
            tuning: &tuning,
            starting_character: &starting_character,
            character_catalog: &character_catalog,
            prepared_characters: prepared_characters.as_deref(),
            authored_sheets: &authored_sheets,
            character_roster: &character_roster,
            placement_lowering: &placement_lowering,
            content_staging: &content_staging,
            // A demo enters directly rather than through provider activation,
            // so it has no prepared-content generation to state.
            construction:
                ambition_platformer2d::runtime::demo_fixture::ActorConstructionContext::new(
                    &construction_recipes,
                    Default::default(),
                ),
            boss_catalog: &boss_catalog,
            default_character_id: provider::MARY_O_CHARACTER_ID,
            sandbox_data_asset: None,
            sandbox_asset_collection: None,
            asset_server: &asset_server,
        },
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// The RULES plugin — the same D-C seam Sanic uses, for a different game.
// ─────────────────────────────────────────────────────────────────────────────

/// The level clock, owned by the mode. It rides a `ModeScopedEntity`, so leaving
/// Mary-O's rooms tears it down through the engine's lifetime-scope vocabulary
/// rather than any teardown code in this crate.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq)]
pub struct MaryOLevelState {
    /// Counts DOWN from [`STARTING_TIME`]; clamps at zero.
    pub time_remaining: f32,
    /// Running total across flag grabs. `flag_score` scores ONE grab by
    /// contact height; this accumulates them, so the HUD can show a career
    /// total rather than the last banner.
    pub score: u32,
    /// Lives left. A death spends one; the run restarts at zero.
    pub lives: u8,
    /// Seconds left on the level-intro card. Counts down on the sim clock, and
    /// the card is published only while it is positive — an unpublished HUD
    /// slot draws nothing, so the card retires itself.
    pub intro_card: f32,
}

impl Default for MaryOLevelState {
    fn default() -> Self {
        Self {
            time_remaining: STARTING_TIME,
            score: 0,
            lives: STARTING_LIVES,
            intro_card: INTRO_CARD_SECONDS,
        }
    }
}

/// Mary-O's level rules. ONE system list; a constructor flag decides its gating —
/// [`MaryORulesPlugin::hosted`] when Ambition hosts the demo alongside its own
/// rooms, [`MaryORulesPlugin::global`] when the demo IS the game.
///
/// That two demos with nothing else in common share this exact shape is the D-C
/// pattern's whole point: a mode is a ROOM property, not a latch some plugin owns.
pub struct MaryORulesPlugin {
    hosted: bool,
}

impl MaryORulesPlugin {
    /// Ambition hosts this demo: every rule sleeps outside Mary-O's rooms.
    pub fn hosted() -> Self {
        Self { hosted: true }
    }

    /// The demo IS the game: the rules run unconditionally.
    pub fn global() -> Self {
        Self { hosted: false }
    }
}

impl Plugin for MaryORulesPlugin {
    fn build(&self, app: &mut App) {
        use bevy::prelude::IntoScheduleConfigs;
        // ⭐ **NOTHING IS INSTALLED HERE ANY MORE, and that is the improvement.**
        // This used to have to run before any world load, because the converter
        // registry was process-wide: LDtk conversion runs from pure non-system
        // code with no `World` in hand, so a plugin-build install was the only
        // moment that reached it. The vocabulary is a value handed to the
        // conversion now ([`ldtk_vocabulary::vocabulary`]), so a reader that
        // forgets it cannot get a half-populated global — it does not compile.
        let sim = ambition_platformer2d::platformer::schedule::SimScheduleExt::sim_schedule(app);
        // 1-1's pole up front so nothing that reads the resource before the first
        // frame finds it missing; `install_goal_pole` re-answers it from the entry
        // room, which is only readable once the host has finished building.
        app.insert_resource(goal_pole());
        app.add_systems(bevy::app::Startup, install_goal_pole);
        app.init_resource::<powerups::SpentPowerBlocks>();
        app.init_resource::<bricks::BrokenBricks>();
        // The brick overlay contributor writes the collision overlay; a full app
        // inserts it (features/render plugins), but a thin rules-only harness may
        // not, and `init_resource` is idempotent — a no-op when already present.
        app.init_resource::<ambition_platformer2d::actors::features::FeatureEcsWorldOverlay>();
        // The cycle emitter writes this; the host's replay consumer drains it. The
        // engine registers it too (`NewGameResetPlugin`), but a thin host
        // may not, and `add_message` is idempotent — a no-op when already present.
        app.add_message::<ambition_platformer2d::actors::session::reset::RoomReplayRequested>();
        // ⚠ declared HERE as well as engine-side, because a channel's EMITTER
        // owes its existence: a composition that installs this demo without the
        // full sim-core resources (every one of this crate's own test apps) still
        // runs `bonk_power_blocks`, and an unregistered message fails parameter
        // validation rather than being ignored.
        app.add_message::<ambition_platformer2d::platformer::block_nudge::BlockStruck>();
        // The authoritative attempt-lost fact `spend_lives_on_death` reads. The
        // engine registers it in `SimCoreResourcesPlugin`; a rules-only harness
        // does not, and a missing message is a hard system-param panic rather
        // than a skip. Idempotent, same as the rest of this block.
        app.add_message::<ambition_platformer2d::actors::ActorDiedMessage>();
        // The snake stager reads room-load facts and writes spawn requests; the
        // engine registers both in a full app, but a thin rules-only test harness
        // may not, and `add_message` is idempotent.
        app.add_message::<ambition_platformer2d::actors::rooms::RoomLoaded>();
        app.add_message::<ambition_platformer2d::actors::features::SpawnActorRequest>();
        // The snake reset listens to the engine's ONE "put this room back"
        // signal, which a full host emits and a rules-only harness does not.
        app.add_message::<ambition_platformer2d::actors::features::ResetRoomFeaturesEvent>();
        // The snake squash pops a dust burst through the engine's vfx seam; a full
        // app registers this via the presentation plugins, but a thin rules-only
        // harness may not, and `add_message` is idempotent.
        app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
        // Same story for the cue queue: the brick-break voices through the shared
        // sfx seam, a full app registers this via the audio plugins, and a thin
        // rules-only harness may not. `add_message` is idempotent.
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        // A sliding snake shell deals damage through the shared `HitEvent` pipeline
        // (`run_snake_shells`); the full app registers this via the engine's damage
        // plugin, but a thin rules-only harness may not. `add_message` is idempotent.
        app.add_message::<ambition_platformer2d::actors::features::HitEvent>();
        // Level progression lives in the canonical gameplay-effects phase. The
        // flag runs before the clock; the cycle emitter runs last so it sees the
        // settled tally and its clock reset is not immediately decremented.
        let rules = (
            spawn_mary_o_mode_owner,
            flag::run_flag_sequence,
            flag::play_victory_music,
            tick_level_clock,
            // The engine's death fact arms the beat before the life counter runs,
            // so a hit death and a timeout reach `spend_lives_on_death` in the
            // same state and it can treat them identically.
            death::begin_death_sequence,
            // Reads the clock the tick above just settled, so a timeout is spent
            // on the frame it happens rather than one late.
            spend_lives_on_death,
            // Then the beat itself: hold her in the death pose where she fell,
            // and restart the level only once it has played out.
            death::run_death_sequence,
            death::play_death_music,
            death::restart_level_after_death,
            cycle_level_on_flag_tally,
        )
            .chain()
            .in_set(ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhaseMonolith::GameplayEffects);
        // Pipe input is authoritative rollback state on the player body. Entry
        // and transit run after ordinary WorldPrep movement, so the scripted
        // position wins this frame instead of racing the shared integrator.
        //
        // (Her transformation numbers used to be installed here, once per body.
        // They ride the REQUEST now — `sync_grown_form` authors the policy for
        // the tier change it is making, because a beat's clip and length are
        // facts about that change, not standing facts about her body.)
        //
        // Mary-O's half of a body reset, answered wherever a body is restarted.
        // Outside the mode gate for the same reason Sanic's is: the observer is
        // inert without her components, and gating it would make the seam a
        // no-op in any stage that seats her outside her own level.
        app.add_observer(movement::clear_spark_cooldown_on_restart);
        let pipe_input = pipe::ensure_pipe_entry_latch
            .in_set(ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhaseMonolith::PlayerInput)
            .after(ambition_platformer2d::actors::avatar::PlayerBrainTick)
            .before(warp_through_secret_pipe);
        let pipe_rules = (warp_through_secret_pipe, pipe::run_pipe_transits)
            .chain()
            .in_set(ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhaseMonolith::PlayerSimulation)
            .after(ambition_platformer2d::actors::features::ecs::damage_apply::PlayerHitResolutionSet);
        // The walkers are registered by `install_mary_o_content`, the single
        // authored-content composition seam shared by direct and shell hosts.
        // Rules consume the staged actors; they do not mutate construction
        // registries after prepared-content fingerprinting.
        // Tag freshly staged enemies, then run each one's stomp mechanic.
        //
        // **Both sit BETWEEN the movement phase and the shared body-contact
        // pass**, and both edges matter:
        //
        // * `.after(integrate_sim_bodies)` — a stomp is classified from where the
        //   bodies ARE, and the contact pass that follows reads exactly the same
        //   post-movement positions. Read pre-movement instead and the two
        //   disagree by one frame of falling: on the landing frame the contact
        //   pass sees the overlap while the stomp rule saw the player still in
        //   the air, so the enemy stays armed and landing on it HURTS. (That is
        //   not hypothetical — it is the regression that made this edge explicit
        //   rather than an accident of plugin insertion order.)
        // * `.before(apply_actor_contact_damage)` — a stomp resolves the enemy
        //   (snake → inert shell; AI Slop → dead) in time for that pass to skip
        //   it, so the stomper is never also hurt.
        let cronies = (
            // A reset hands back walkers, never the shell state the last attempt
            // left behind. First in the chain so a snake reset this frame is a
            // walker for every rule that follows it.
            snake::reset_snakes_on_room_reset,
            snake::tag_mary_o_snakes,
            ai_slop::tag_mary_o_ai_slop,
            snake::run_snake_shells,
            ai_slop::bounce_squash_ai_slop,
        )
            .chain()
            // Both edges are PHASES now, which is the same statement without a
            // game crate reaching for two engine function names.
            .in_set(ambition_platformer2d::platformer::schedule::WorldPrepSet::AfterIntegrate)
            .before(ambition_platformer2d::platformer::schedule::WorldPrepSet::ContactDamage);
        // The powerup rules on the two engine primitives: re-arm the ?-blocks on
        // (re)load, pop wand on a head-bonk, and keep the tall form in sync with
        // wearing the wand. The engine's `collect_world_items` (touch → equip) sits
        // between the bonk and the grow — no demo wiring for it.
        let powerups = (
            powerups::refill_power_blocks_on_room_loaded,
            powerups::bonk_power_blocks,
            powerups::sync_grown_form,
            // The star, after the form sync: collecting the quasar converts a
            // worn token into a timed body state, and `run_star_power` asserts
            // the untouchable fact AFTER the transformation beat has had its say
            // on the same flag this tick (see `star`'s module docs).
            star::begin_star_power,
            ambition_platformer2d::actors::features::empowerment::run_empowerments,
            // Contact harm AFTER the tick that may have ended the empowerment,
            // so the frame it expires on is not also a frame it flattens
            // something.
            ambition_platformer2d::actors::features::empowerment::apply_contact_harm,
            star::play_star_music,
            powerups::tag_mary_o_sparks,
        )
            .chain()
            .in_set(ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhaseMonolith::FeatureInteraction);
        // Mary-O's locomotion POLICY and her spark's press edge. Both read the
        // sustained control slot off the body's freshly-produced `ActorControl`,
        // so they sit after the brain tick and before the shared movement phase
        // consumes the frame — the throttle they set then flows through the
        // ordinary body path, replay and rollback included.
        let gait = (
            movement::ensure_gait,
            movement::walk_by_default_run_while_held,
            movement::tick_spark_cooldown,
            movement::fire_spark_on_run_press,
            movement::sync_run_action_scheme,
        )
            .chain()
            .in_set(ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhaseMonolith::PlayerInput)
            .after(ambition_platformer2d::actors::avatar::PlayerBrainTick);
        // The bricks — the reactive-block primitive's SECOND consumer: re-arm on
        // (re)load, break the bonked one, and contribute broken bricks to the
        // collision overlay's `removed_block_names` so they stop colliding (and, via
        // the render reconcile, drawing). The contribution runs AFTER the engine's
        // overlay rebuild clears that list — the same slot `contribute_encounter_lock_walls`
        // takes — so the removals survive the per-frame clean slate.
        let bricks = (bricks::refill_bricks_on_room_loaded, bricks::break_bricks)
            .chain()
            .in_set(ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhaseMonolith::FeatureInteraction);
        let brick_overlay = bricks::contribute_broken_bricks_to_overlay
            .in_set(ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep)
            .after(ambition_platformer2d::actors::features::FeatureWorldOverlaySet);
        if self.hosted {
            app.add_systems(
                sim,
                rules.run_if(ambition_platformer2d::runtime::in_mode(MARY_O_MODE)),
            );
            app.add_systems(
                sim,
                pipe_input.run_if(ambition_platformer2d::runtime::in_mode(MARY_O_MODE)),
            );
            app.add_systems(
                sim,
                pipe_rules.run_if(ambition_platformer2d::runtime::in_mode(MARY_O_MODE)),
            );
            app.add_systems(
                sim,
                cronies.run_if(ambition_platformer2d::runtime::in_mode(MARY_O_MODE)),
            );
            app.add_systems(
                sim,
                powerups.run_if(ambition_platformer2d::runtime::in_mode(MARY_O_MODE)),
            );
            app.add_systems(
                sim,
                bricks.run_if(ambition_platformer2d::runtime::in_mode(MARY_O_MODE)),
            );
            app.add_systems(
                sim,
                gait.run_if(ambition_platformer2d::runtime::in_mode(MARY_O_MODE)),
            );
            app.add_systems(
                sim,
                brick_overlay.run_if(ambition_platformer2d::runtime::in_mode(MARY_O_MODE)),
            );
        } else {
            app.add_systems(sim, rules);
            app.add_systems(sim, pipe_input);
            app.add_systems(sim, pipe_rules);
            app.add_systems(sim, cronies);
            app.add_systems(sim, powerups);
            app.add_systems(sim, bricks);
            app.add_systems(sim, gait);
            app.add_systems(sim, brick_overlay);
        }
    }
}

fn spawn_mary_o_mode_owner(
    mut commands: bevy::prelude::Commands,
    existing: bevy::prelude::Query<(), bevy::prelude::With<MaryOLevelState>>,
    session: Option<
        bevy::prelude::Res<ambition_platformer2d::platformer::lifecycle::ActiveSessionScope>,
    >,
) {
    use ambition_platformer2d::platformer::lifecycle::{SessionSpawnScope, SpawnSessionScopedExt};
    // Sleep once a session-scoped host has retired the live session (at the
    // launcher), so the level state is not resurrected from stale "mary_o" room
    // metadata. Inert when no `ActiveSessionScope` exists (Startup path / D-C
    // tests). Mirrors Sanic's `spawn_sanic_mode_owner`.
    let session_live = session
        .as_ref()
        .map_or(true, |scope| scope.current().is_some());
    let spawn_scope = session
        .as_ref()
        .map_or(SessionSpawnScope::UNSCOPED, |scope| scope.spawn_scope());
    if session_live && existing.iter().next().is_none() {
        // The sequence rides the same entity as the clock. Owned by BOTH the mode
        // (survives in-session room changes) and the active session (torn down on
        // a shell relaunch, which a same-mode reload is NOT).
        commands
            .spawn_session_scoped(
                spawn_scope,
                (
                    MaryOLevelState::default(),
                    flag::FlagSequence::default(),
                    death::MaryODeathSequence::default(),
                ),
            )
            .insert(
                ambition_platformer2d::platformer::lifecycle::ModeScopedEntity(
                    MARY_O_MODE.to_string(),
                ),
            );
    }
}

/// The level clock runs on the SIM clock, so pause and bullet-time slow it exactly
/// as they slow everything else. It clamps at zero rather than going negative.
fn tick_level_clock(
    time: bevy::prelude::Res<ambition_platformer2d::time::WorldTime>,
    mut level: bevy::prelude::Query<(
        &mut MaryOLevelState,
        &flag::FlagSequence,
        &death::MaryODeathSequence,
    )>,
) {
    for (mut state, flag, dying) in &mut level {
        state.intro_card = (state.intro_card - time.scaled_dt).max(0.0);
        // A level whose flag has been grabbed is over. The clock stopping is what
        // turns the remaining time from a threat into a score.
        //
        // A level she just died on is over too, and for the same reason: the
        // attempt has already been decided, and a clock that kept draining
        // through the death beat would eat the fresh attempt's time before she
        // ever got it.
        if flag.active() || dying.active() {
            continue;
        }
        state.time_remaining = (state.time_remaining - time.scaled_dt).max(0.0);
    }
}

/// **Death costs a life, and running out of time is a death.**
///
/// Two ways to die, one accounting, and exactly one life per attempt lost.
///
/// # Why this reads a message and not the respawn counter
///
/// This used to watch `BodyLifetime.resets` for an increase. That counter is
/// bumped by SIX unrelated callers — a combat death, a kernel hazard/pit reset,
/// a room load, an avatar rebuild, a sandbox reset, and **a room replay's own
/// body reset**. The last one closed a loop: a death spent a life and requested
/// a replay, the replay reset the body, the reset bumped the counter, this
/// system read that as a second death, spent another life, and requested another
/// replay. Unbounded, at frame rate. Grabbing the FLAG entered the same loop,
/// because the level-cycle also requests a replay. The counter cannot say why it
/// moved, so no amount of edge-detection here could have fixed it.
///
/// [`ActorDiedMessage`] is the engine's authoritative "the local player's
/// attempt ended" fact, published from both real death paths — the hit resolver
/// for combat deaths, and `publish_kernel_reset_death` for the pit/drown/hazard
/// reset that never reaches the resolver. A replay's reset publishes nothing, so
/// the loop cannot form by construction rather than by guard.
///
/// # What is deliberately NOT a death
///
/// A `SafeRespawn` hazard bump-back does not publish it, so it costs no life —
/// that is the engine saying "returned to safety", not "died", and Mary-O now
/// agrees with it. A room replay and a room load cost no life either.
///
/// At zero lives the RUN is over: lives, score, and clock return to their
/// starting values and the room replays. That is the arcade loop — a game over
/// is a fresh run, not a stuck screen.
fn spend_lives_on_death(
    mut level: bevy::prelude::Query<(&mut MaryOLevelState, &mut death::MaryODeathSequence)>,
    // The KINEMATICS are optional on purpose. Whether a life is spent must not
    // depend on being able to read a position — a body that exists is what says
    // an attempt was in progress, and requiring more silently skipped the whole
    // system for any body without the extra component.
    bodies: bevy::prelude::Query<
        (
            bevy::prelude::Entity,
            Option<&ambition_platformer2d::engine_core::BodyKinematics>,
        ),
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
    mut deaths: bevy::prelude::MessageReader<ambition_platformer2d::actors::ActorDiedMessage>,
) {
    // Drain unconditionally: the cursor must advance even on a frame with no
    // level, or a death that landed during a load would be re-read later and
    // charged to the next attempt.
    //
    // ⚠ **drain FIRST, filter after.** The victim filter below needs the body
    // query, and the early returns between here and there must not be allowed to
    // skip the drain — that is the invariant this comment has always been about.
    let victims: Vec<bevy::prelude::Entity> = deaths.read().map(|death| death.victim).collect();

    let Ok((mut level, mut death_beat)) = level.single_mut() else {
        return;
    };
    // No body, no attempt in progress — so nothing to lose. This matters for
    // the TIMEOUT branch specifically: the level owner can exist for frames
    // before a body does, and a clock that reaches zero in that window is a
    // level that never started, not a life the player spent. (The old counter
    // version got this for free by querying the body's `BodyLifetime`; the
    // authoritative signal does not need the body, so the guard is now
    // explicit.)
    let Some((body, kin)) = bodies.iter().next() else {
        return;
    };
    // ⭐ **HER death, not any death.** This used to count every `ActorDiedMessage`
    // in the frame, which is right only while one body can die: an enemy dying
    // would have spent one of her lives the moment anything else emitted the
    // fact (GPT 5.6 review, 2026-08-04).
    let died = victims.contains(&body);
    let died_at = kin.map(|kin| kin.pos);

    // The clock reaching zero is its own death, and it must not fire twice
    // while the replay is in flight — restoring the clock below is what
    // disarms it.
    let timed_out = level.time_remaining <= 0.0;
    if !died && !timed_out {
        return;
    }
    // The RESTART is the death beat's, not this system's. Every lost attempt —
    // a hit that got past her armor, a pit, the clock running out — plays the
    // same death and leaves by the same door, so a timeout cannot silently skip
    // the beat a death gets. A hit death has already armed it from the engine's
    // own death fact one system earlier; a timeout has no such fact, so arming
    // here from where she stands is what makes the two identical by the time the
    // life is counted.
    death_beat.begin(died_at);

    // ONE attempt lost costs ONE life, however many ways it was reported. A
    // frame can carry both a lethal hit and a hazard reset for the same fall —
    // and a death during the DWELL is the same attempt too. She is pinned
    // exactly where she fell, so a pit keeps reporting her, and without this
    // she spent a life on every frame of her own death animation until the run
    // was over. The beat carries the debt because the beat is what knows which
    // attempt is being lost.
    if death_beat.life_spent {
        return;
    }
    death_beat.life_spent = true;

    level.lives = level.lives.saturating_sub(1);
    level.time_remaining = STARTING_TIME;
    // A fresh attempt gets a fresh card — it is how the player reads how many
    // lives that death cost them.
    level.intro_card = INTRO_CARD_SECONDS;

    if level.lives == 0 {
        // Game over: the whole run resets, score included.
        level.lives = STARTING_LIVES;
        level.score = 0;
    }
}

/// **The secret pipe.** Press DOWN on the surface mouth and you fall out of the
/// pipe hanging from the vault's ceiling; press UP at the mouth of the pipe at the
/// vault's far end and you rise out of its surface half, nine tiles further along.
///
/// One verb at both ends: **touch a pipe's mouth and press into it.** The mouth is
/// the pipe's own open face ([`mouth_band`]) and [`at_mouth`] is the whole test —
/// centred on the pipe, box against the face. Nothing here measures a region of
/// the room, which is what made the overhead end feel like a button that worked
/// anywhere below the pipe.
///
/// The warp is a real TRANSIT, not a position poke: `transit_body` is the engine
/// authority for discretely relocating a body (ADR 0024), and it reconciles the
/// motion model's private attachment and maneuver state on the way. Without that
/// a player who entered the pipe while wall-clinging would arrive in the vault
/// still clinging to a wall that is no longer there.
///
/// The pipe is entered DIRECTIONALLY (Jon bug list #8): press DOWN standing on
/// the entry mouth to drop in, press UP with your head in the return pipe's mouth
/// to surface — the classic warp-pipe verb. That does NOT break the "a single Up/Down must not
/// trigger a door" rule: a pipe is not a door, and the press has to point INTO
/// the pipe while you stand on its mouth, which reads as deliberate, not
/// incidental. It also removes the ping-pong for free — the two ends need
/// OPPOSITE directions, so a held press that warped you down can never fire the
/// up-return at the far end.
/// The press does not relocate the body — it STARTS a [`pipe::PipeTransit`], the
/// scripted half-second slide in and out that `pipe::run_pipe_transits` drives.
/// A body already in a tube is excluded from this query, so the trip cannot be
/// re-triggered from inside itself.
fn warp_through_secret_pipe(
    mut commands: bevy::prelude::Commands,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
    mut bodies: bevy::prelude::Query<
        (
            bevy::prelude::Entity,
            &ae::BodyKinematics,
            &ambition_platformer2d::characters::brain::ActorControl,
            &mut pipe::PipeEntryLatch,
        ),
        (
            ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
            bevy::prelude::Without<pipe::PipeTransit>,
        ),
    >,
) {
    // Body-local locomotion, `+y` toward the feet (screen-down under Mary-O's
    // normal gravity): press toward the ground to go DOWN a pipe, away to go UP.
    const DIR_DEADZONE: f32 = 0.5;
    for (entity, kin, control, mut latch) in &mut bodies {
        let down = control.0.locomotion.y > DIR_DEADZONE;
        let up = control.0.locomotion.y < -DIR_DEADZONE;
        let body = ae::Aabb::new(kin.pos, kin.size * 0.5);

        // Each mouth answers only its own direction: DOWN at the entry pipe, UP
        // at the return pipe.
        let at_entry = at_mouth(body, pipe_mouth());
        let at_return = at_mouth(body, vault_exit());
        let destination = warp_destination(down, up, at_entry, at_return);
        let pressed = destination.is_some();
        let rising_edge = pressed && !latch.pressed;
        latch.pressed = pressed;
        let Some(destination) = destination else {
            continue;
        };
        if !rising_edge {
            continue;
        }

        // The way INTO the near pipe: down the entry mouth, up the return pipe.
        // Both tubes are vertical, so the axis is the press direction itself.
        let axis = if at_entry {
            ae::DEFAULT_GRAVITY_DIR
        } else {
            -ae::DEFAULT_GRAVITY_DIR
        };
        commands
            .entity(entity)
            .try_insert(pipe::PipeTransit::begin(kin.pos, destination, axis, T));
        // H2: the warp is the entering BODY's — she is the one sliding down it.
        sfx.write_for(
            entity,
            ambition_platformer2d::sfx::SfxMessage::Play {
                id: ambition_platformer2d::sfx::SfxId::new(pipe::PIPE_WARP_SFX),
                pos: kin.pos,
            },
        );
    }
}

/// Where a directional pipe press sends the body, if anywhere (Jon bug #8).
///
/// A mouth answers ONLY its own direction: DOWN drops you in at the entry pipe,
/// UP surfaces you at the return pipe. Pressing the wrong way — or Interact,
/// which is neither — does nothing, which is the whole point: you no longer warp
/// by bumping a generic button, and the opposite-direction ends can never
/// ping-pong a held press.
fn warp_destination(down: bool, up: bool, at_entry: bool, at_return: bool) -> Option<ae::Vec2> {
    if down && at_entry {
        Some(vault_arrival())
    } else if up && at_return {
        Some(pipe_arrival())
    } else {
        None
    }
}

/// Is `body` **at** `mouth` — lined up with the pipe and touching its open face?
///
/// Two conditions, one for each half of "she is under the pipe and her box is
/// close enough to it":
/// * her CENTRE is inside the pipe's column, not merely a shoulder's worth of
///   box overlapping its edge. A pipe is something you stand in front of, and
///   grazing one is not entering it;
/// * her box reaches the open face, within [`MOUTH_SLACK`].
///
/// The second condition is the one that makes an overhead pipe honest: standing
/// on the vault floor is not enough, you have to reach the lip — grown you touch
/// it standing, small you hop into it.
fn at_mouth(body: ae::Aabb, mouth: ae::Aabb) -> bool {
    let centre = (body.min.x + body.max.x) * 0.5;
    centre >= mouth.min.x
        && centre <= mouth.max.x
        && body.min.y < mouth.max.y
        && body.max.y > mouth.min.y
}

/// **Cyclic level completion.** Once the flag tally has settled, restart the
/// level — "the next level is the same level," the classic arcade loop.
///
/// Emitting the engine's generic [`RoomReplayRequested`] restarts the ACTIVE room
/// in place (player warped back to spawn, room-scoped state rebuilt); it is the
/// exact "replay the current room" seam a "try again" beat uses, and "next level
/// = same level" maps straight onto it with no new message type. Resetting the
/// sequence to `Idle` and the clock to [`STARTING_TIME`] here is what arms the
/// next lap so the tally does not re-fire every frame. The walk-off has already
/// carried the body clear of the pole's grab band, so the freshly-`Idle` sequence
/// cannot immediately re-grab in the one frame before the host warps the body home.
///
/// [`RoomReplayRequested`]: ambition_platformer2d::actors::session::reset::RoomReplayRequested
fn cycle_level_on_flag_tally(
    time: bevy::prelude::Res<ambition_platformer2d::time::WorldTime>,
    mut dwell: bevy::prelude::Local<f32>,
    mut owners: bevy::prelude::Query<(&mut flag::FlagSequence, &mut MaryOLevelState)>,
    mut replay: bevy::prelude::MessageWriter<
        ambition_platformer2d::actors::session::reset::RoomReplayRequested,
    >,
) {
    let Ok((mut sequence, mut level)) = owners.single_mut() else {
        *dwell = 0.0;
        return;
    };
    if !matches!(sequence.phase, flag::FlagPhase::Tallied { .. }) {
        *dwell = 0.0;
        return;
    }
    // Let the tally sit a beat before the level loops.
    *dwell += time.scaled_dt;
    if *dwell < LEVEL_CYCLE_DWELL {
        return;
    }
    *dwell = 0.0;
    // Bank this grab before the sequence resets — `score()` reads the phase
    // that is about to be cleared.
    if let Some(grabbed) = sequence.score() {
        level.score = level.score.saturating_add(grabbed);
    }
    *sequence = flag::FlagSequence::default();
    level.time_remaining = STARTING_TIME;
    level.intro_card = INTRO_CARD_SECONDS;
    replay.write(ambition_platformer2d::actors::session::reset::RoomReplayRequested);
}

/// Install the Mary-O demo content layer into an engine app.
pub fn add_demo_content(app: &mut App) {
    app.add_plugins(MaryODemoContentPlugin);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Reading the AUTHORED level, instead of the names it used to be built with ──
    //
    // ⛔ **Seven tests here looked terrain up by the name `level_1_1` gave it —
    // `ground_open_teach`, `stair_up_3`, `secret_pipe`.** Terrain is painted into
    // an IntGrid now, and `area create`'s lowering EATS the name (the merged
    // rectangles all come back as `ldtk solid`), so those lookups could not
    // survive the migration and should not: a test that needs the level's fourth
    // ground run to still be CALLED something is pinned to how the level was
    // built, not to what it is.
    //
    // ⭐ these ask the collision instead, which is the question they always meant
    // and the version that survives Jon repainting the floor.

    /// Is any solid or one-way surface covering this world point?
    fn solid_at(room: &RoomSpec, at: ae::Vec2) -> bool {
        room.world.blocks.iter().any(|b| {
            !matches!(b.kind, ae::BlockKind::Hazard)
                && b.aabb.min.x <= at.x
                && b.aabb.max.x >= at.x
                && b.aabb.min.y <= at.y
                && b.aabb.max.y >= at.y
        })
    }

    /// The level's GROUND RUNS as `[from_x, to_x)`, derived from the authored
    /// collision: contiguous spans of solid slab at the ground row. The gaps
    /// between them are the pits, which is what every pit assertion here wants.
    fn authored_ground_runs(room: &RoomSpec) -> Vec<(f32, f32)> {
        let probe_y = SURFACE_HEIGHT - GROUND_TILES * T + T * 0.5;
        let mut runs: Vec<(f32, f32)> = Vec::new();
        let mut x = 0.0f32;
        while x < room.world.size.x {
            if solid_at(room, ae::Vec2::new(x, probe_y)) {
                match runs.last_mut() {
                    Some(run) if (run.1 - x).abs() < T * 0.5 => run.1 = x + T,
                    _ => runs.push((x, x + T)),
                }
            }
            x += T;
        }
        runs
    }

    /// The TOP of the highest solid standing at column `x` above the ground row —
    /// how tall the step, stair or slab there is. `None` where the column is open.
    fn surface_top_at(room: &RoomSpec, x: f32) -> Option<f32> {
        room.world
            .blocks
            .iter()
            .filter(|b| {
                matches!(b.kind, ae::BlockKind::Solid)
                    && b.aabb.min.x <= x
                    && b.aabb.max.x >= x
                    && b.aabb.min.y < SURFACE_HEIGHT
            })
            .map(|b| b.aabb.min.y)
            .min_by(|a, b| a.partial_cmp(b).expect("finite"))
    }

    /// The authored block with this exact name.
    fn authored_named(room: &RoomSpec, name: &str) -> ae::Aabb {
        room.world
            .blocks
            .iter()
            .find(|b| b.name == name)
            .unwrap_or_else(|| panic!("the level authors a `{name}` block"))
            .aabb
    }

    #[test]
    fn mary_o_demo_content_plugin_installs() {
        let mut app = App::new();
        // See the note in `movement::tests`: authored placements require the
        // engine foundation's lowering registry.
        ambition_platformer2d::engine::add_headless_foundation(&mut app);
        app.add_plugins(ambition_platformer2d::actors::features::WorldPrepSchedulePlugin);
        add_demo_content(&mut app);
        let catalog = app
            .world()
            .resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>();
        assert!(catalog.get(provider::MARY_O_CHARACTER_ID).is_some());
        // Her three power forms are all catalog characters — the small starting
        // sheet, the grown (star wand) sheet, and the fire (cinder beacon) sheet.
        // Before the fire row existed she wore the grown sheet while spark-powered,
        // so there was no distinct fire look (Jon bug #10).
        assert!(
            catalog.get("mary_o_tall").is_some(),
            "the grown power form is a catalog character"
        );
        assert!(
            catalog.get("mary_o_fire").is_some(),
            "the fire power form is a catalog character (Jon bug #10)"
        );
        // Solid Snake (the Koopa) is a catalog character, its display name is what
        // the enemy render resolves its sheet by, and — critically — a sheet SPEC
        // must resolve for it. If it does NOT, the loader SKIPS the row entirely
        // (not even deferring it), and the enemy falls back to the generic goblin
        // sheet. This is the guard for "the snakes render as goblins".
        let snake = catalog
            .get("solid_snake")
            .expect("Solid Snake is a catalog row");
        assert_eq!(snake.display_name, snake::SNAKE_DISPLAY_NAME);
        assert!(
            ambition_platformer2d::actors::character_sprites::sheet_for_character_id_in(
                &Default::default(),
                catalog,
                "solid_snake"
            )
            .is_some(),
            "a sheet spec resolves for solid_snake — else it is skipped and renders as a goblin"
        );
        // AI Slop is the SECOND enemy: the plain stomp-and-die walker. Same guard —
        // its `ai_slop` sheet spec must resolve, or it renders as a goblin too.
        let ai_slop_row = catalog.get("ai_slop").expect("AI Slop is a catalog row");
        assert_eq!(ai_slop_row.display_name, ai_slop::AI_SLOP_DISPLAY_NAME);
        assert!(
            ambition_platformer2d::actors::character_sprites::sheet_for_character_id_in(
                &Default::default(),
                catalog,
                "ai_slop"
            )
            .is_some(),
            "a sheet spec resolves for ai_slop — else it is skipped and renders as a goblin"
        );
        // Mary-O Classic is deliberately the run/jump floor only. Wall jump
        // and ground pound are later abilities; the current Hollow-Knight wall
        // bundle and generic fast-fall must not contaminate the core oracle.
        let mary_o_kit = catalog
            .ability_set(provider::MARY_O_CHARACTER_ID)
            .expect("Mary-O authors a grant list");
        assert_eq!(
            mary_o_kit,
            ambition_platformer2d::engine_core::AbilitySet::compose(&[
                ambition_platformer2d::engine_core::AbilityGrant::RunJump,
            ]),
            "Mary-O Classic composes to ordinary horizontal movement + one variable jump"
        );
        assert!(mary_o_kit.jump && mary_o_kit.move_horizontal && mary_o_kit.variable_jump);
        assert!(
            !mary_o_kit.double_jump
                && !mary_o_kit.wall_jump
                && !mary_o_kit.wall_cling
                && !mary_o_kit.fast_fall
                && !mary_o_kit.blink
                && !mary_o_kit.dash
                && !mary_o_kit.fly
                && !mary_o_kit.attack,
            "advanced movement and the full Ambition kit stay out of the classic core"
        );
        // ⭐ **and no TALK verb.** Jon, from a phone: *"maryo has more than 2 on
        // screen buttons … that shouldn't be the case for her."* The third was
        // Interact, which `derive_action_scheme` used to upsert for every
        // controllable body. It is an ability now, absent from `NONE` and
        // therefore from any composed grant list — so the classic run-and-jump
        // floor draws exactly two buttons, and her pipes are unaffected because
        // they answer UP or DOWN and never a button (her own rule, in
        // `level_1_2.rs`).
        assert!(
            !mary_o_kit.interact,
            "Mary-O Classic has no talk verb: the game she converges on has none, \
             and a button that does nothing is what put a third control on her \
             phone HUD"
        );

        let mary_o_tuning = catalog
            .axis_tuning(provider::MARY_O_CHARACTER_ID)
            .expect("Mary-O authors an axis tuning");
        let horizontal = match mary_o_tuning.horizontal_law {
            ambition_platformer2d::engine_core::AxisHorizontalLaw::Momentum(params) => params,
            other => panic!("Mary-O must use the momentum law, got {other:?}"),
        };
        assert_eq!(mary_o_tuning.max_run_speed, 300.0);
        assert_eq!(mary_o_tuning.run_accel, 393.75);
        assert_eq!(mary_o_tuning.air_accel, 393.75);
        assert_eq!(horizontal.ground_reverse_accel, 1500.0);
        assert_eq!(
            horizontal.ground_coast_decel, 393.75,
            "the FAITHFUL conversion: classic friction equals classic walk \
             acceleration. An earlier pass stiffened this to 1200 to kill the \
             slide; converging on the source material won."
        );
        assert_eq!(horizontal.air_coast_decel, 0.0);

        let jump = match mary_o_tuning.jump_law {
            ambition_platformer2d::engine_core::AxisJumpLaw::PhasedGravity(params) => params,
            other => panic!("Mary-O must use phased gravity, got {other:?}"),
        };
        assert_eq!(jump.speed_thresholds, [120.0, 187.5, 210.0]);
        assert_eq!(jump.launch_offsets, [-30.0, -15.0, 0.0, 30.0]);
        assert_eq!(jump.held_rise_gravity_scale, 0.2);
        assert_eq!(jump.released_rise_gravity_scale, 1.0);
        assert_eq!(jump.fall_gravity_scale, 1.0);
        assert_eq!(jump.held_phase_min_upward_speed, 240.0);
        assert_eq!(mary_o_tuning.max_fall_speed, 480.0);
        // THE band decision, pinned: a RUNNING jump is her highest jump, exactly
        // as in the original. The top band's cut sits inside her run cap, so all
        // four bands are reachable through ordinary locomotion — none of them is
        // reserved for externally supplied overspeed. Flip this assertion (and
        // the threshold) if the top band is ever meant to be a fling-only reward.
        assert!(
            jump.top_band_speed() < mary_o_tuning.max_run_speed,
            "the top launch band must be reachable by running ({} vs cap {})",
            jump.top_band_speed(),
            mary_o_tuning.max_run_speed
        );
        assert_eq!(
            jump.band_for_side_speed(mary_o_tuning.max_run_speed),
            3,
            "full run selects the top band"
        );
        assert_eq!(
            jump.band_for_side_speed(movement::WALK_THROTTLE * mary_o_tuning.max_run_speed),
            1,
            "a walk selects a lower band, so the two gaits jump differently"
        );
        // The base knob still owns the whole family: every band is an offset
        // from it, so there is exactly ONE ground-jump height authority.
        assert_eq!(
            jump.launch_speed_for_band(mary_o_tuning.jump_speed, 3),
            mary_o_tuning.jump_speed + 30.0
        );
        assert_eq!(mary_o_tuning.coyote_time, 0.0);
        assert_eq!(
            mary_o_tuning.jump_buffer, 0.0,
            "Classic honors only the current press; it adds no pre-landing forgiveness"
        );
        assert_eq!(mary_o_tuning.gravity, 2250.0);
        assert_eq!(
            catalog.axis_tuning("mary_o_tall"),
            Some(mary_o_tuning),
            "growing changes Mary-O's body/art, not her physics profile"
        );
        assert_eq!(
            catalog.axis_tuning("mary_o_fire"),
            Some(mary_o_tuning),
            "the spark form keeps the exact same classic physics"
        );
        let defaults = app
            .world()
            .resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalogDefaults>(
        );
        assert_eq!(
            defaults.for_provider(provider::MARY_O_EXPERIENCE),
            Some(provider::MARY_O_CHARACTER_ID)
        );
    }

    /// Jon reported "pit B is not a pit — it opens directly into the secret
    /// vault", and the triage filed it as unfixed. It is NOT live at HEAD: the
    /// level was lengthened (pit B and everything past it pushed 8 tiles right)
    /// so the vault fits under UNBROKEN ground, which closed it as a side
    /// effect. This pins that, because the bug is invisible until someone falls
    /// in — a pit whose floor is the secret's ceiling reads as a normal pit
    /// right up until you drop through it.
    #[test]
    fn no_pit_drops_into_the_secret_vault() {
        let room = level_1_1();
        let vault = vault_bounds();
        // Each pit is the gap between consecutive ground runs, read off the
        // authored collision rather than off the names the runs used to carry.
        let runs = authored_ground_runs(&room);
        assert!(runs.len() >= 2, "the level authors ground with pits in it");
        for pair in runs.windows(2) {
            let (left, right) = (pair[0], pair[1]);
            let gap_min = left.1;
            let gap_max = right.0;
            assert!(gap_max > gap_min, "these runs do not bound a pit");
            assert!(
                gap_max <= vault.min.x || gap_min >= vault.max.x,
                "a pit spans [{gap_min}, {gap_max}] and the vault spans \
                 [{}, {}] — falling in the pit lands you in the secret",
                vault.min.x,
                vault.max.x,
            );
        }
    }

    /// **The 1-1 grammar, asserted as geometry rather than as a screenshot.** An
    /// open teach run, three WIDENING pits, a stepping stone inside the widest,
    /// a stair pyramid, a goal past it. If a future edit flattens the rhythm this
    /// fails — which is what makes it a level design and not a pile of boxes.
    #[test]
    fn level_1_1_carries_the_grammar_it_claims() {
        let room = level_1_1();
        let world = &room.world;

        // The spawn sits inside the room, on the open-teach run.
        let s = world.spawn;
        assert!(s.x >= 0.0 && s.x <= world.size.x && s.y >= 0.0 && s.y <= world.size.y);

        // Three pits, WIDENING. A pit is the gap between consecutive ground runs.
        let runs = authored_ground_runs(&room);
        assert!(
            runs.len() >= 4,
            "1-1's grammar is an open run and three pits: got {runs:?}"
        );
        let pits: Vec<f32> = runs.windows(2).map(|p| p[1].0 - p[0].1).collect();
        assert!(
            pits.windows(2).all(|p| p[0] < p[1]),
            "the pit rhythm must WIDEN — each pit charges more for the last one's \
             lesson: {pits:?}"
        );

        // The first platform hangs over SAFE ground: missing it costs nothing.
        let (open_from, open_to) = runs[0];
        let teach_top = SURFACE_HEIGHT - GROUND_TILES * T - 4.0 * T;
        let teach_x = (open_from + open_to) * 0.5;
        assert!(
            room.world.blocks.iter().any(|b| {
                matches!(b.kind, ae::BlockKind::OneWay)
                    && (b.aabb.min.y - teach_top).abs() < T
                    && b.aabb.min.x >= open_from
                    && b.aabb.max.x <= open_to
            }),
            "a jump-height platform must hang over the OPEN run (around x={teach_x}), \
             never over a pit"
        );

        // ...and the same jump is load-bearing exactly once, inside the WIDEST pit:
        // a one-way standing in the gap you cannot clear without it.
        let widest = runs
            .windows(2)
            .max_by(|a, b| {
                (a[1].0 - a[0].1)
                    .partial_cmp(&(b[1].0 - b[0].1))
                    .expect("finite")
            })
            .map(|p| (p[0].1, p[1].0))
            .expect("the level has pits");
        assert!(
            room.world.blocks.iter().any(|b| {
                matches!(b.kind, ae::BlockKind::OneWay)
                    && b.aabb.min.x >= widest.0
                    && b.aabb.max.x <= widest.1
            }),
            "a stepping stone must stand INSIDE the widest pit [{}, {}]",
            widest.0,
            widest.1
        );

        // ⚠ every platform in the level is one-way: you rise through them and never
        // get stuck under one. Stated over the whole population rather than over two
        // names, so a platform Jon adds is held to the same rule.
        let platform_top = SURFACE_HEIGHT - GROUND_TILES * T - 2.0 * T;
        for block in room.world.blocks.iter().filter(|b| {
            b.aabb.min.y < platform_top
                && b.aabb.max.y - b.aabb.min.y <= T
                && !b.name.starts_with(GOAL_POLE_PREFIX)
                && !authored_named_blocks().contains_key(&b.name)
        }) {
            assert!(
                matches!(block.kind, ae::BlockKind::OneWay),
                "`{}` floats at jump height and is not a one-way — this grammar's \
                 platforms admit from below",
                block.name
            );
        }

        // The pyramid ascends, then descends, and the goal is past both halves.
        let pole = authored_named(&room, "goal_pole");
        let tallest = (0..(room.world.size.x / T) as i32)
            .filter_map(|c| surface_top_at(&room, c as f32 * T + T * 0.5).map(|top| (c, top)))
            .filter(|(_, top)| *top < SURFACE_HEIGHT - GROUND_TILES * T - T)
            .map(|(c, _)| c as f32 * T)
            .collect::<Vec<_>>();
        assert!(
            !tallest.is_empty(),
            "the level raises something above ground height — the pyramid"
        );
        assert!(
            pole.min.x > tallest.iter().cloned().fold(f32::MIN, f32::max),
            "the goal is past the pyramid"
        );
    }

    /// **The trench between the double stairs is somewhere an enemy can PACE.**
    ///
    /// It was two tiles — barely wider than a slop, so the ones that tumbled in
    /// were wedged rather than patrolling (Jon: *"more space between them for the
    /// aislop to move around in"*). "Wide enough" is expressed against the thing
    /// that has to move in it, not as a tile count, so a future slop that grows
    /// takes this with it.
    #[test]
    fn the_trench_between_the_double_stairs_is_wide_enough_to_patrol() {
        let room = level_1_1();
        // ⚠ the trench is the GAP between the two raised halves, found by walking
        // the authored surface — not by naming `stair_up_4` and `stair_down_4`,
        // which were how the pyramid used to be BUILT rather than what it is.
        let ground_top = SURFACE_HEIGHT - GROUND_TILES * T;
        let raised: Vec<f32> = (0..(room.world.size.x / T) as i32)
            .map(|c| c as f32 * T + T * 0.5)
            .filter(|&x| surface_top_at(&room, x).is_some_and(|top| top < ground_top - T * 0.5))
            .collect();
        assert!(!raised.is_empty(), "the level raises a pyramid");
        // The widest gap between consecutive raised columns IS the trench.
        let trench = raised
            .windows(2)
            .map(|p| p[1] - p[0])
            .fold(0.0f32, f32::max)
            - T;
        let slop_width = ai_slop::AI_SLOP_HALF * 2.0;
        assert!(
            trench >= slop_width * 4.0,
            "the trench is {trench} wide and a slop is {slop_width} — it needs \
             room to walk, turn, and be run through, not just to fit"
        );
        let pole = room
            .world
            .blocks
            .iter()
            .find(|b| b.name == "goal_pole")
            .expect("the flagpole")
            .aabb;
        assert!(
            pole.min.x > raised.iter().cloned().fold(f32::MIN, f32::max),
            "widening the trench must not push the pyramid into the flagpole"
        );
    }

    /// **Every authored enemy has ground under it.**
    ///
    /// ⛔ the stair columns used to live in TWO files — the block builder here and
    /// a hand-copied table in `ai_slop.rs` — so widening the trench left four slop
    /// floating over where the far half used to be. Both then read one
    /// `stair_steps()`, which fixed the drift and left the shape a Rust fact.
    ///
    /// ⭐ **there is no second place now**: the pyramid is authored geometry and
    /// the slop on it are authored placements, so they cannot disagree. What is
    /// still worth checking is the thing an author can get wrong by hand —
    /// dropping an enemy over a pit, where it falls out of the level before it is
    /// ever seen.
    #[test]
    fn every_authored_enemy_has_ground_under_it() {
        let room = level_1_1();
        assert!(
            !room.enemy_spawns.is_empty(),
            "the level authors its enemies"
        );
        for spawn in &room.enemy_spawns {
            use ae::AabbExt;
            let at = spawn.aabb.center();
            let ground = (0..40)
                .map(|step| at.y + step as f32 * T)
                .find(|y| solid_at(&room, ae::Vec2::new(at.x, *y)));
            assert!(
                ground.is_some(),
                "`{}` is authored at {at:?} with nothing under it — it falls out of \
                 the level before a player ever sees it",
                spawn.id
            );
        }
    }

    #[test]
    fn level_1_1_authors_the_bricks_the_break_runtime_expects() {
        use crate::ldtk_vocabulary::{block_look_of, MaryOBlockLook};
        let room = level_1_1();
        let bricks: Vec<&ae::Block> = room
            .world
            .blocks
            .iter()
            .filter(|b| block_look_of(&b.name) == Some(MaryOBlockLook::Brick))
            .collect();
        assert!(!bricks.is_empty(), "the level authors a brick wall");
        for brick in bricks {
            assert!(
                matches!(brick.kind, ae::BlockKind::Solid),
                "`{}` is a brick and must be solid geometry until a bonk breaks it",
                brick.name
            );
            assert_ne!(
                block_look_of(&brick.name),
                Some(MaryOBlockLook::Question),
                "`{}` must not read as a ?-block as well",
                brick.name
            );
        }
    }

    /// The room claims its mode, which is what a hosted `MaryORulesPlugin` wakes on.
    #[test]
    fn level_1_1_claims_the_mary_o_mode() {
        assert_eq!(level_1_1().metadata.mode.as_deref(), Some(MARY_O_MODE));
        assert_ne!(MARY_O_MODE, "sanic", "two demos, two modes, one binary");
    }

    /// The level clock counts DOWN on the sim clock and clamps at zero. `hosted()`
    /// gates it on the mode; `global()` does not. The same seam as Sanic's act
    /// timer, for a completely different game — which is the D-C pattern's claim.
    #[test]
    fn hosted_rules_tick_the_level_clock_only_in_mary_o_rooms() {
        use ambition_platformer2d::world::rooms::{ActiveRoomMetadata, RoomMetadata};

        fn remaining(app: &mut App) -> Option<f32> {
            let mut q = app.world_mut().query::<&MaryOLevelState>();
            q.iter(app.world()).next().map(|s| s.time_remaining)
        }
        fn shell(rules: MaryORulesPlugin, mode: Option<&str>, dt: f32) -> App {
            let mut app = App::new();
            ambition_platformer2d::engine::add_headless_foundation(&mut app);
            ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
                app.world_mut(),
                ActiveRoomMetadata(RoomMetadata {
                    mode: mode.map(str::to_string),
                    ..Default::default()
                }),
            );
            app.insert_resource(ambition_platformer2d::time::WorldTime {
                scaled_dt: dt,
                ..Default::default()
            });
            app.add_plugins(rules);
            app
        }

        // Hosted, in a Mary-O room: the clock counts DOWN. (`.chain()` puts a sync
        // point between spawn and tick, so the owner ticks on its own first frame.)
        let mut app = shell(MaryORulesPlugin::hosted(), Some(MARY_O_MODE), 1.0);
        app.update();
        app.update();
        assert_eq!(remaining(&mut app), Some(STARTING_TIME - 2.0));

        // Hosted, in one of Ambition's own rooms: no owner, no clock.
        let mut app = shell(MaryORulesPlugin::hosted(), None, 1.0);
        app.update();
        assert_eq!(remaining(&mut app), None, "the rules sleep out of mode");

        // Standalone: the demo IS the game, so no mode is needed.
        let mut app = shell(MaryORulesPlugin::global(), None, 1.0);
        app.update();
        assert_eq!(remaining(&mut app), Some(STARTING_TIME - 1.0));

        // The clock clamps at zero rather than running negative.
        let mut app = shell(MaryORulesPlugin::global(), None, STARTING_TIME * 2.0);
        app.update();
        assert_eq!(remaining(&mut app), Some(0.0));
    }

    /// **The vault is a SECRET: reachable only through the pipe, and sealed.**
    ///
    /// This is geometry, and geometry is exactly the thing that is invisible in a
    /// headless build and expensive to eyeball in a running one. A vault whose
    /// wall is one tile short, or whose arrival lands inside the stone, is a
    /// silently broken secret — the pipe still "works", you just fall through the
    /// world or get stuck. So: assert the arrival is inside the chamber, that the
    /// chamber is under the ground slab, and that both warp ends actually
    /// overlap a body standing where the player would be.
    /// Jon bug #8: the pipe is entered DIRECTIONALLY — DOWN drops in at the entry
    /// mouth, UP surfaces at the return mouth — and a generic press (Interact,
    /// which is neither direction, or the wrong direction) no longer warps you.
    #[test]
    fn the_pipe_only_answers_the_correct_directional_press() {
        // The intended verbs work.
        assert_eq!(
            warp_destination(true, false, true, false),
            Some(vault_arrival()),
            "DOWN on the entry pipe drops into the vault"
        );
        assert_eq!(
            warp_destination(false, true, false, true),
            Some(pipe_arrival()),
            "UP on the return pipe surfaces"
        );
        // The bug: a generic press (Interact = no direction) used to warp. It
        // must not anymore.
        assert_eq!(
            warp_destination(false, false, true, false),
            None,
            "Interact / no direction must NOT warp at the entry"
        );
        assert_eq!(warp_destination(false, false, false, true), None);
        // The WRONG direction at a mouth does nothing.
        assert_eq!(
            warp_destination(false, true, true, false),
            None,
            "pressing UP at the DOWN pipe does nothing"
        );
        assert_eq!(
            warp_destination(true, false, false, true),
            None,
            "pressing DOWN at the UP pipe does nothing"
        );
        // Standing on no mouth: nothing warps whatever you press.
        assert_eq!(warp_destination(true, true, false, false), None);
    }

    #[test]
    fn the_pipe_leads_into_a_sealed_vault_and_back_out() {
        use ambition_platformer2d::engine_core::AabbExt;

        let vault = vault_bounds();
        let arrival = vault_arrival();
        let ground_top = SURFACE_HEIGHT - GROUND_TILES * T;
        let authored = level_1_1();

        // The vault hangs BELOW the ground slab — that is what makes it secret
        // rather than a visible annex of the level.
        assert!(
            vault.min.y >= ground_top + GROUND_TILES * T,
            "the vault ceiling must be at or under the ground slab; vault top \
             {} vs slab bottom {}",
            vault.min.y,
            ground_top + GROUND_TILES * T
        );

        // Arrival is strictly inside, with room for a body.
        assert!(
            arrival.x > vault.min.x
                && arrival.x < vault.max.x
                && arrival.y > vault.min.y
                && arrival.y < vault.max.y,
            "the pipe drops the player inside the vault, not into its stone: \
             arrival {arrival:?} vs {vault:?}"
        );

        // The world is tall enough to contain the vault it was grown for.
        // ⚠ read off the AUTHORED room, not a constant: Jon can resize the level
        // in the editor, and the question is whether the vault fits in the world
        // he drew.
        assert!(
            vault.max.y < authored.world.size.y,
            "the vault floor must be inside the world bounds"
        );

        // Leaving the vault surfaces you standing ON the SURFACE EXIT pipe past pit
        // B — a visible pipe, not mid-air. Read the block's top off the AUTHORED
        // level, never the formula it was built from.
        let surface_exit = level_1_1()
            .world
            .blocks
            .iter()
            .find(|b| b.name == SURFACE_EXIT_PIPE_NAME)
            .expect("the level has a visible surface exit pipe past pit B")
            .aabb;
        assert!(
            pipe_arrival().x > surface_exit.min.x
                && pipe_arrival().x < surface_exit.max.x
                && pipe_arrival().y <= surface_exit.min.y,
            "the vault surfaces you standing on the exit pipe, not mid-air: \
             arrival {:?} vs pipe {surface_exit:?}",
            pipe_arrival()
        );
        // The return pipe hangs from the vault ceiling, so a player cannot stand
        // ON it — they stand UNDER it and press UP. Assert from where a body
        // actually ends up: on the vault FLOOR, beneath the pipe. Read the floor
        // off the AUTHORED level, never recompute it from the formula it was
        // supposed to use — recomputing tests the intent and passes no matter
        // where the geometry actually ended up. (An earlier version of this test
        // stood the body on the return pipe's top face; that face is now the
        // ceiling, and standing there is not a thing a player can do.)
        let vault_floor = level_1_1()
            .world
            .blocks
            .iter()
            .find(|b| b.name == "vault_floor")
            .expect("the vault has a floor")
            .aabb
            .min
            .y;
        //
        // You enter a pipe by TOUCHING its mouth, so the pipe is hung at exactly
        // the height where a GROWN body standing on that floor reaches its lip.
        let standing = |size: ae::Vec2, x: f32| {
            ae::Aabb::new(ae::Vec2::new(x, vault_floor - size.y * 0.5), size * 0.5)
        };
        let under = vault_exit().center().x;
        assert!(
            at_mouth(standing(powerups::tall_body_size(), under), vault_exit()),
            "a GROWN body standing on the vault floor under the return pipe must \
             reach its mouth, or the press can never fire and the vault is a \
             one-way trip: mouth {:?}, floor {vault_floor}",
            vault_exit()
        );
        // ...and standing OFF to the side is not, however close: a mouth is a
        // place on the pipe, not a region of the room it hangs in.
        assert!(
            !at_mouth(
                standing(
                    powerups::tall_body_size(),
                    // One pipe-width clear, measured off the AUTHORED pipe
                    // rather than a constant tile count.
                    under + (vault_exit().max.x - vault_exit().min.x)
                ),
                vault_exit()
            ),
            "standing beside the return pipe must NOT enter it: mouth {:?}",
            vault_exit()
        );
        // The small form does not reach a lip hung for the grown one — she hops
        // into it. That is the shape of the rule, not a gap in it: the trigger is
        // contact with the pipe, so a body that is not touching does not warp.
        // (While the mouth was a band on the FLOOR, every body standing anywhere
        // in the pipe's column warped, which is what read as loose.)
        let small = ae::movement::default_player_body_size();
        assert!(
            !at_mouth(standing(small, under), vault_exit()),
            "the small form standing flat-footed is not touching a lip hung at \
             grown head height: mouth {:?}",
            vault_exit()
        );
        let hopped = {
            let flat = standing(small, under);
            ae::Aabb::new(flat.center() - ae::Vec2::new(0.0, T), flat.half_size())
        };
        assert!(
            at_mouth(hopped, vault_exit()),
            "...but a hop of one tile puts her head in the mouth: mouth {:?}",
            vault_exit()
        );

        // The level really does carry BOTH pipes and the coins that reward
        // them. The return pipe is asserted because its absence is exactly the
        // bug that shipped: the exit was a logical zone with no geometry, so
        // the vault looked like a dead end and the only way out was knowing
        // where to press Interact. A warp whose mouth you cannot see is not a
        // warp, and no assertion about the ZONE would have caught it.
        let room = level_1_1();
        assert!(
            room.world.blocks.iter().any(|b| b.name == PIPE_NAME),
            "the entrance pipe is authored into the level"
        );
        let return_pipe = room
            .world
            .blocks
            .iter()
            .find(|b| b.name == EXIT_PIPE_NAME)
            .expect("the vault has a VISIBLE return pipe, not just an exit zone")
            .aabb;
        // The mouth IS the pipe's open face: exactly its width, straddling its lip.
        // Not a nearby region that happens to be under it — that is the difference
        // between "press up while touching the pipe" and "press up somewhere below
        // the pipe", and only the first one reads as entering anything.
        assert!(
            (vault_exit().min.x - return_pipe.min.x).abs() < 1.0
                && (vault_exit().max.x - return_pipe.max.x).abs() < 1.0,
            "the exit mouth spans the return pipe's own width: mouth {:?} vs pipe \
             {return_pipe:?}",
            vault_exit()
        );
        assert!(
            vault_exit().min.y < return_pipe.max.y && vault_exit().max.y > return_pipe.max.y,
            "and it straddles that pipe's LIP — its open bottom face, the part she \
             touches — not the floor several tiles below it: mouth {:?} vs pipe \
             {return_pipe:?}",
            vault_exit()
        );
        // ⚠ **an INVARIANT, not the authored count.** This used to pin exactly
        // `VAULT_COINS`, which was right while a Rust constant stocked the
        // chamber and is a tripwire now that Jon stocks it: adding a ninth coin
        // is authoring, not a regression. What has to stay true is that the
        // reward for finding the pipe EXISTS and is reachable — inside the
        // chamber rather than buried in its stone.
        // ⛔ **selected by WHERE THEY ARE, not by what they are called.** An
        // authored pickup's id is its LDtk iid (`PickupSpawn-106857`), not a name
        // the generator chose — so a filter on `vault_coin_` found nothing while
        // all eight coins sat in the chamber. The invariant is the reward itself:
        // the secret holds currency, inside the room you reach by pipe.
        let coins: Vec<_> = room
            .placements
            .iter()
            .filter(|p| {
                let at = p.aabb.center();
                matches!(
                    p.schema,
                    ambition_platformer2d::entity_catalog::placements::PlacementSchema::Pickup(_)
                ) && at.x > vault.min.x
                    && at.x < vault.max.x
                    && at.y > vault.min.y
                    && at.y < vault.max.y
            })
            .collect();
        assert!(
            !coins.is_empty(),
            "the vault is stocked — the whole reward for finding the pipe"
        );
    }

    /// **A pipe you go INTO always has a pipe you come OUT of.**
    ///
    /// The three things Jon caught by looking at the level, which every test here
    /// missed because they only ever checked a mouth against its own zone:
    ///
    /// 1. The descent pipe had NO output pipe — you pressed down on a pipe and
    ///    materialized in open stone, with nothing at the far end to come out of.
    /// 2. The vault's return pipe STOOD ON THE FLOOR pointing up out of solid rock,
    ///    when the way it leads is up through the ceiling.
    /// 3. Its surface pipe was across the pit instead of above it, so the "tube"
    ///    bent sideways through the ground for no reason a player could read.
    ///
    /// **(1) is the universal rule** and the only one that is really about pipes:
    /// wherever a warp puts you, there is a pipe there to come out of. A pipe whose
    /// far end is in another room may well be nowhere near its entrance — what has
    /// to hold is that it READS as connected, which means arriving at a visible
    /// mouth.
    ///
    /// **(2) and (3) are the SAME-ROOM rule.** Both of level 1-1's tubes are one
    /// physical object inside one room — a tube through the ground slab — so their
    /// halves are genuinely connected and must line up: matching columns, one
    /// hanging from the vault ceiling and one standing on the slab. That is a
    /// property of these tubes, not of every pipe the engine will ever host.
    #[test]
    fn a_pipe_you_enter_always_has_a_pipe_you_come_out_of() {
        let room = level_1_1();
        let vault = vault_bounds();
        let ground_top = SURFACE_HEIGHT - GROUND_TILES * T;
        let block = |name: &str| {
            room.world
                .blocks
                .iter()
                .find(|b| b.name == name)
                .unwrap_or_else(|| panic!("the level authors a `{name}` pipe"))
                .aabb
        };

        // SAME-ROOM rule: these two tubes each pierce the slab inside one room, so
        // each is one physical object and its halves must line up. A tube whose far
        // end lived in a DIFFERENT room would be exempt — it only has to read as
        // connected, which the arrival-at-a-mouth checks below cover.
        for (surface_name, vault_name) in [
            (PIPE_NAME, VAULT_ENTRY_PIPE_NAME),
            (SURFACE_EXIT_PIPE_NAME, EXIT_PIPE_NAME),
        ] {
            let (surface, under) = (block(surface_name), block(vault_name));
            assert!(
                (surface.min.x - under.min.x).abs() < 1.0
                    && (surface.max.x - under.max.x).abs() < 1.0,
                "{surface_name} and {vault_name} are two halves of ONE tube and must \
                 share a column: {surface:?} vs {under:?}"
            );
            // The surface half stands ON the ground slab...
            assert!(
                (surface.max.y - ground_top).abs() < 1.0,
                "{surface_name} must stand on the ground slab, not float: {surface:?}"
            );
            // ...and the vault half HANGS FROM THE CEILING, which is that same slab.
            // (The bug: the return pipe sat on the vault FLOOR.)
            assert!(
                (under.min.y - vault.min.y).abs() < 1.0,
                "{vault_name} must hang from the vault ceiling — a pipe that leads UP \
                 cannot stand on the floor: {under:?} vs ceiling {}",
                vault.min.y
            );
        }

        // A vault half REACHES DOWN to her, because you enter a pipe by TOUCHING its
        // mouth. Both bounds are forced: clear her tallest form or she cannot walk
        // under it (and so can never reach the lip at all), but stay within touching
        // distance of that same form or the mouth floats above every reachable head
        // and pressing UP becomes a button that works in a column of air.
        let vault_floor = block("vault_floor").min.y;
        let tall = powerups::tall_body_size().y;
        for name in [VAULT_ENTRY_PIPE_NAME, EXIT_PIPE_NAME] {
            let lip = block(name).max.y;
            let clearance = vault_floor - lip;
            assert!(
                clearance > tall,
                "{name}'s lip must clear Mary-O's TALL form ({tall}px) or she cannot \
                 walk under it: {clearance}px"
            );
            assert!(
                clearance - tall < MOUTH_SLACK,
                "...and must hang within touching distance of that form's head, or \
                 she can stand under it and still not be at its mouth: {clearance}px \
                 leaves a {}px gap, slack is {MOUTH_SLACK}px",
                clearance - tall
            );
        }

        // THE UNIVERSAL RULE: each warp delivers you at a visible pipe's MOUTH, not
        // into bare stone. This is the half that would still have to hold for a pipe
        // whose far end is in another room entirely.
        let dropped_out_of = block(VAULT_ENTRY_PIPE_NAME);
        assert!(
            vault_arrival().x > dropped_out_of.min.x
                && vault_arrival().x < dropped_out_of.max.x
                && vault_arrival().y >= dropped_out_of.max.y,
            "going DOWN the descent tube must drop you out of its vault pipe's mouth: \
             {:?} vs pipe {dropped_out_of:?}",
            vault_arrival()
        );
        let rose_out_of = block(SURFACE_EXIT_PIPE_NAME);
        assert!(
            pipe_arrival().x > rose_out_of.min.x
                && pipe_arrival().x < rose_out_of.max.x
                && pipe_arrival().y <= rose_out_of.min.y,
            "going UP the ascent tube must put you on top of its surface pipe: {:?} vs \
             pipe {rose_out_of:?}",
            pipe_arrival()
        );
    }

    /// **The vault ceiling is unbroken — no surface pit punches a hole into it.**
    ///
    /// The vault's ceiling IS the level's ground slab. If any surface PIT sits over
    /// the vault's x-range, the pit's gap in the slab is a hole straight into the
    /// secret: a body that falls into that pit drops into the vault instead of
    /// dying, and from inside you see daylight through the ceiling. That is exactly
    /// what shipped — the 14-tile vault reached under pit B — and no existing test
    /// caught it, because they all checked the pipes and the arrival, never the
    /// roof. Sample across the whole vault width; every column must sit under the
    /// slab, never over a pit.
    #[test]
    fn the_vault_ceiling_is_unbroken_no_pit_opens_a_hole() {
        let vault = vault_bounds();
        let ground_top = SURFACE_HEIGHT - GROUND_TILES * T;
        let room = level_1_1();
        // ⚠ ASK THE COLLISION, not the name. Terrain is painted into an IntGrid
        // and the merged blocks carry no `ground_*` name to match on — and the
        // question was never about names: is there slab over the secret at this
        // column, whatever drew it.
        let slab_covers = |x: f32| solid_at(&room, ae::Vec2::new(x, ground_top + T * 0.5));
        let mut x = vault.min.x;
        while x <= vault.max.x {
            assert!(
                slab_covers(x),
                "the vault ceiling has a HOLE at x={x}: a surface pit sits above the \
                 secret, so a faller drops into it (vault {vault:?})"
            );
            x += 4.0;
        }
    }

    /// **A death spends a life, and running out of time is a death.**
    ///
    /// Drives [`ActorDiedMessage`] — the engine's authoritative attempt-lost
    /// fact, published by the hit resolver for combat deaths and by
    /// `publish_kernel_reset_death` for the pit/drown/hazard reset that never
    /// reaches the resolver.
    ///
    /// This deliberately no longer bumps `BodyLifetime.resets`. That counter is
    /// bumped by six unrelated callers including a room replay's own body reset,
    /// and driving it here made the old test structurally incapable of catching
    /// the replay feedback loop — see
    /// [`a_replay_reset_is_not_a_death_so_lives_cannot_drain`], which is the
    /// regression the old oracle could not express.
    #[test]
    fn a_death_or_a_timeout_spends_a_life_and_zero_lives_restarts_the_run() {
        use ambition_platformer2d::world::rooms::{ActiveRoomMetadata, RoomMetadata};

        fn shell(dt: f32) -> App {
            let mut app = App::new();
            ambition_platformer2d::engine::add_headless_foundation(&mut app);
            ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
                app.world_mut(),
                ActiveRoomMetadata(RoomMetadata::default()),
            );
            app.insert_resource(ambition_platformer2d::time::WorldTime {
                scaled_dt: dt,
                ..Default::default()
            });
            app.add_message::<ambition_platformer2d::actors::ActorDiedMessage>();
            app.add_plugins(MaryORulesPlugin::global());
            app.world_mut().spawn((
                ambition_platformer2d::engine_core::BodyLifetime::default(),
                ambition_platformer2d::platformer::markers::PlayerEntity,
                ambition_platformer2d::platformer::markers::PrimaryPlayer,
            ));
            app
        }
        fn kill(app: &mut App) {
            // The life counter charges HER death, so the fixture has to say the
            // body died — an unattributed death now spends nothing, correctly.
            let victim = app
                .world_mut()
                .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
                .iter(app.world())
                .next()
                .expect("the fixture spawns the primary player before killing it");
            app.world_mut()
                .write_message(ambition_platformer2d::actors::ActorDiedMessage {
                    victim,
                    pos: ambition_platformer2d::engine_core::Vec2::ZERO,
                    cause: ambition_platformer2d::actors::DeathCause {
                        source: ambition_platformer2d::combat::HitSource::Hazard,
                        attacker: None,
                    },
                });
        }
        fn level(app: &mut App) -> (u8, u32, f32) {
            let mut q = app.world_mut().query::<&MaryOLevelState>();
            let s = q.iter(app.world()).next().expect("the mode owner exists");
            (s.lives, s.score, s.time_remaining)
        }

        // ── A death spends exactly one life ──────────────────────────────────
        let mut app = shell(0.0);
        app.update();
        assert_eq!(level(&mut app).0, STARTING_LIVES, "no death, no cost");

        kill(&mut app);
        app.update();
        assert_eq!(
            level(&mut app).0,
            STARTING_LIVES - 1,
            "she died, so a life is spent"
        );

        // No further deaths reported: the cost does not repeat per frame.
        app.update();
        app.update();
        assert_eq!(
            level(&mut app).0,
            STARTING_LIVES - 1,
            "a life is spent per death, not per frame after one"
        );

        // ── Running out of time is a death, and refills the clock ────────────
        let mut app = shell(STARTING_TIME * 2.0);
        app.update();
        let (lives, _, remaining) = level(&mut app);
        assert_eq!(lives, STARTING_LIVES - 1, "the clock hitting zero kills");
        assert_eq!(
            remaining, STARTING_TIME,
            "and the clock refills, which is also what disarms the timeout so it \
             cannot spend every remaining life on consecutive frames"
        );

        // ── Zero lives restarts the RUN, score included ──────────────────────
        let mut app = shell(0.0);
        app.update();
        {
            let mut q = app.world_mut().query::<&mut MaryOLevelState>();
            let mut state = q.iter_mut(app.world_mut()).next().unwrap();
            state.lives = 1;
            state.score = 4200;
        }
        kill(&mut app);
        app.update();
        let (lives, score, remaining) = level(&mut app);
        assert_eq!(lives, STARTING_LIVES, "a game over starts a fresh run");
        assert_eq!(score, 0, "and a fresh run scores from zero");
        assert_eq!(remaining, STARTING_TIME, "on a full clock");
    }

    /// **A replay's own body reset must not read as a death.**
    ///
    /// The regression for the feedback loop. Before the fix, lives were inferred
    /// from `BodyLifetime.resets`, so this sequence was recursive: a death spent
    /// a life and requested a replay; the replay consumer called
    /// `reset_body_clusters`; that bumped `resets`; the bump was read as a second
    /// death; another life, another replay — unbounded, at frame rate, wrapping
    /// the counter forever. Grabbing the FLAG entered the same loop, because the
    /// level cycle also requests a replay.
    ///
    /// This stands in for the replay consumer by doing the one thing it does to
    /// the body — bumping the respawn counter — and then asserting that nothing
    /// happens. That is the whole claim: the counter is no longer an input to
    /// life accounting, so no caller of `reset_body_clusters` can spend a life.
    ///
    /// NOTE ON SCOPE: this composes Mary-O's rules, not the real host consumer,
    /// which lives in `ambition_app` and is unreachable from this crate. The
    /// hosted end-to-end proof is still open — see the demo's planning doc.
    #[test]
    fn a_replay_reset_is_not_a_death_so_lives_cannot_drain() {
        use ambition_platformer2d::world::rooms::{ActiveRoomMetadata, RoomMetadata};

        let mut app = App::new();
        ambition_platformer2d::engine::add_headless_foundation(&mut app);
        ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
            app.world_mut(),
            ActiveRoomMetadata(RoomMetadata::default()),
        );
        app.insert_resource(ambition_platformer2d::time::WorldTime {
            scaled_dt: 0.0,
            ..Default::default()
        });
        app.add_message::<ambition_platformer2d::actors::ActorDiedMessage>();
        app.add_plugins(MaryORulesPlugin::global());
        let body = app
            .world_mut()
            .spawn((
                ambition_platformer2d::engine_core::BodyLifetime::default(),
                ambition_platformer2d::platformer::markers::PlayerEntity,
                ambition_platformer2d::platformer::markers::PrimaryPlayer,
            ))
            .id();
        app.update();
        assert_eq!(
            level_lives(&mut app),
            STARTING_LIVES,
            "a fresh level starts on three lives"
        );

        // Every caller of `reset_body_clusters` looks like this from the
        // outside: a room replay, a room load, a sandbox reset, an avatar
        // rebuild. None of them is a death.
        for _ in 0..8 {
            app.world_mut()
                .get_mut::<ambition_platformer2d::engine_core::BodyLifetime>(body)
                .unwrap()
                .resets += 1;
            app.update();
        }

        assert_eq!(
            level_lives(&mut app),
            STARTING_LIVES,
            "eight body resets with no death reported must cost NOTHING — under \
             the old counter inference this had already drained and wrapped the \
             lives counter"
        );
    }

    fn level_lives(app: &mut App) -> u8 {
        let mut q = app.world_mut().query::<&MaryOLevelState>();
        q.iter(app.world())
            .next()
            .expect("the mode owner exists")
            .lives
    }

    /// **The level loops: a settled tally rearms the level after a dwell.** The
    /// tally holds for [`LEVEL_CYCLE_DWELL`] before the sequence returns to `Idle`
    /// and the clock refills — that reset is what the cycle emitter does on the
    /// same line it writes `RoomReplayRequested` (so observing the reset proves the
    /// emit ran), and it must NOT fire early or the tally would never be seen.
    #[test]
    fn a_settled_tally_rearms_the_level_after_a_dwell() {
        use ambition_platformer2d::world::rooms::{ActiveRoomMetadata, RoomMetadata};

        let mut app = App::new();
        ambition_platformer2d::engine::add_headless_foundation(&mut app);
        ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
            app.world_mut(),
            ActiveRoomMetadata(RoomMetadata::default()),
        );
        // Half the dwell per frame: frame 1 arms nothing, frame 2 crosses it.
        app.insert_resource(ambition_platformer2d::time::WorldTime {
            scaled_dt: LEVEL_CYCLE_DWELL * 0.5,
            ..Default::default()
        });
        app.add_plugins(MaryORulesPlugin::global());

        // First update spawns the mode owner; drive the clock below full so the
        // rearm's refill is observable, then drop a settled tally onto the owner.
        app.update();
        {
            let mut q = app
                .world_mut()
                .query::<(&mut flag::FlagSequence, &mut MaryOLevelState)>();
            let world = app.world_mut();
            let (mut seq, mut level) = q.iter_mut(world).next().expect("owner spawned");
            seq.phase = flag::FlagPhase::Tallied { score: 800 };
            level.time_remaining = 123.0;
        }

        fn state(app: &mut App) -> (flag::FlagPhase, f32) {
            let mut q = app
                .world_mut()
                .query::<(&flag::FlagSequence, &MaryOLevelState)>();
            let (seq, level) = q.iter(app.world()).next().unwrap();
            (seq.phase, level.time_remaining)
        }

        // One dwell-half in: still tallied, clock untouched — the tally is on screen.
        app.update();
        let (phase, remaining) = state(&mut app);
        assert!(
            matches!(phase, flag::FlagPhase::Tallied { .. }),
            "the tally must hold for the full dwell, not rearm early"
        );
        assert_eq!(remaining, 123.0, "the clock does not refill mid-dwell");

        // Crossing the dwell rearms: sequence back to Idle, clock refilled.
        app.update();
        let (phase, remaining) = state(&mut app);
        assert_eq!(
            phase,
            flag::FlagPhase::Idle,
            "crossing the dwell returns the sequence to Idle for the next lap"
        );
        assert_eq!(
            remaining, STARTING_TIME,
            "the new lap starts with a full clock"
        );
    }
}

#[cfg(test)]
mod flag_geometry_oracle {
    use super::*;

    /// [`goal_pole`] and the authored `goal_pole` block are the SAME object. This is
    /// the test that catches someone moving the level and leaving the flag behind.
    #[test]
    fn the_pole_resource_is_the_authored_block() {
        let room = level_1_1();
        let block = room
            .world
            .blocks
            .iter()
            .find(|b| b.name == "goal_pole")
            .expect("the level authors a goal pole");
        let aabb = block.aabb;
        let pole = goal_pole();

        let center_x = (aabb.min.x + aabb.max.x) * 0.5;
        assert!((pole.x - center_x).abs() < 1.0e-3, "pole is centered");
        assert_eq!(pole.top_y, aabb.min.y, "top of the pole");
        assert_eq!(pole.base_y, aabb.max.y, "base of the pole");
    }

    /// The grab band is narrower than the pole is tall, and the pole spans a real
    /// slide. A pole with `top_y == base_y` would score every grab 100 and read as
    /// a bug in the score table rather than in the level.
    #[test]
    fn the_pole_is_tall_enough_to_have_score_bands() {
        let pole = goal_pole();
        let span = pole.base_y - pole.top_y;
        assert!(span > 100.0, "a {span}px pole has no bands worth sliding");
        assert_eq!(flag::flag_score(pole.grab_height(pole.top_y)), 5000);
        assert_eq!(flag::flag_score(pole.grab_height(pole.base_y)), 100);
    }
}
