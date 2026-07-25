//! Super Mary-O demo content home — the M-track's world half.
//!
//! This crate names only `ambition` and `bevy`. It is the E9
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
pub mod bricks;
pub mod flag;
pub mod movement;
pub mod pipe;
pub mod powerups;
pub mod provider;
pub mod scenery;
pub mod snake;
pub mod stomp;

pub use provider::{
    mary_o_session_world, MaryOExperiencePlugin, MaryOSessionWorld, MARY_O_CHARACTER_ID,
    MARY_O_EXPERIENCE, MARY_O_GAMEPLAY_ROUTE, MARY_O_LAUNCHER_ROUTE,
};

use ambition::engine_core as ae;
use ambition::prelude::*;
use ambition::world::rooms::{PropSpec, RoomSpec};

/// Stable room id for level 1-1.
pub const LEVEL_1_1_ROOM_ID: &str = "mary_o_1_1";

/// The game-MODE tag this demo's rooms carry (decomposition D-C).
///
/// Ambition can host this demo alongside its own rooms; [`MaryORulesPlugin`] gates
/// its systems on `ambition::runtime::in_mode(MARY_O_MODE)` so the level clock never
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

/// Tile columns of the ?-blocks (bonk from below for the milk powerup), and how
/// many tiles above the ground they float. Shared by [`level_1_1`] (which builds
/// the solid blocks) and [`power_block_id`]/[`power_block_min`] (which derive
/// their durable [`GeoId`](ae::GeoId) and position) so the level and the powerup
/// runtime can never disagree on which block is a ?-block or where it is.
/// Three of them, and the third is the one that matters: the ladder is
/// state-driven (milk while small, blossom once grown), so with only two blocks a
/// player who took a hit between them could never reach the spark form at all —
/// the first block re-grows her and the second is already spent. The third sits
/// after the brick run, past the point where a snake is likely to have cost her
/// the cap, so the fire form is reachable on a normal messy playthrough rather
/// than only on a clean one.
const POWER_BLOCK_COLUMNS: [f32; 3] = [6.0, 30.0, 60.0];
const POWER_BLOCK_ROW: f32 = 4.0;
/// The IntGrid tile layer the ?-blocks are filed under, and the merge ordinal the
/// first ?-block's [`GeoId`](ae::GeoId) starts at. `solid_tiled` stamps
/// `GeoId::tile_layer(POWER_BLOCK_LAYER, POWER_BLOCK_BASE_INDEX + i)` — a STABLE
/// identity the powerup runtime matches a head-bonk contact against (no
/// point-matching): the engine's `ContactSource::Block` now carries the struck
/// block's `GeoId`.
const POWER_BLOCK_LAYER: &str = "mary_o_ground";
const POWER_BLOCK_BASE_INDEX: u16 = 10;

/// Tile columns of the breakable BRICKS — the ?-block's plain sibling and the
/// SECOND consumer of the reactive-block primitive (`ContactSource::Block` carrying
/// a durable [`GeoId`](ae::GeoId)). A head-bonk BREAKS a brick (removes it from the
/// world) — same durable-id match as the ?-block powerup, opposite effect: the
/// ?-block ADDS a milk pickup, the brick SUBTRACTS itself. A short run over the
/// ground after pit B, clear of the ?-blocks so the two motifs never blur. See
/// [`bricks`].
const BRICK_COLUMNS: [f32; 3] = [48.0, 49.0, 50.0];
/// Bricks sit at the same bonk height as the ?-blocks.
const BRICK_ROW: f32 = POWER_BLOCK_ROW;
/// The IntGrid tile layer + merge-ordinal base for the bricks' durable `GeoId`s. A
/// base index disjoint from the ?-blocks' so no brick ever shares an id with one.
const BRICK_LAYER: &str = "mary_o_ground";
const BRICK_BASE_INDEX: u16 = 20;

/// How thick the goal pole is drawn. Half a tile — a pole, not a pillar. Named
/// because [`goal_pole`] must derive the grab band from the SAME number
/// [`level_1_1`] draws the block with; a band narrower than the pole is a level
/// that cannot be finished.
const POLE_WIDTH: f32 = T * 0.5;

/// The level's world width and height. Named, rather than inlined into
/// [`level_1_1`], because [`goal_pole`] must derive the flag's geometry from the
/// same numbers the flag's BLOCK is built from — see `flag_geometry_oracle`.
const LEVEL_WIDTH: f32 = 104.0 * T;

/// The SURFACE half's height — every above-ground feature is placed against
/// this, so growing the world downward for the vault below leaves the authored
/// 1-1 layout byte-identical.
const SURFACE_HEIGHT: f32 = 15.0 * T;

/// How far below the ground slab the secret vault's floor sits.
const VAULT_DEPTH_TILES: f32 = 9.0;

const LEVEL_HEIGHT: f32 = SURFACE_HEIGHT + VAULT_DEPTH_TILES * T;

/// **A warp pipe is a TUBE THROUGH THE GROUND SLAB.** Each one is authored as two
/// halves at the SAME tile column: a SURFACE half standing on the ground with its
/// mouth up, and a VAULT half hanging from the vault's ceiling with its mouth down.
/// That is the whole geometric rule, and it buys three things the old layout was
/// missing: every pipe you go INTO has a pipe you come OUT of, a pipe that leads
/// upward hangs from the ceiling instead of standing on the floor pointing the
/// wrong way, and the two ends of one tube line up the way a real tube would.
///
/// There are two tubes:
/// * The **descent** tube at column 26 — press DOWN on its surface mouth and you
///   drop out of its vault half. Column 26 is the safe run between pit A and pit B:
///   far enough past the open teach that a player has learned the jump, close
///   enough that the first pipe they ever see is not at the end of the level.
/// * The **ascent** tube at column 35 — press UP at its vault half's mouth, which
///   hangs at head height at the far end of the chamber, and you rise out of its
///   surface half. Column 35 sits inside
///   the vault's span [23,41] and on the solid run [22,42) before pit B, so the tube
///   is straight: you come up exactly where you were standing, nine tiles past where
///   you went down, with five clear tiles to build the run-up for pit B. (It used to
///   surface ACROSS pit B, which bent the "tube" sideways through the ground for no
///   reason a player could read.)
const PIPE_COLUMN: f32 = 26.0;
const EXIT_PIPE_COLUMN: f32 = 35.0;
/// THREE tiles wide, not the two a Mario pipe nominally is, because a pipe here
/// has to be able to HIDE the body going through it. Mary-O's collision box is
/// about one tile across but her SPRITE is drawn far wider than that (character
/// art deliberately overflows its hitbox), so a two-tile pipe left her shoulders
/// sticking out either side while she slid down it.
const PIPE_WIDTH_TILES: f32 = 3.0;
const PIPE_HEIGHT_TILES: f32 = 2.0;
/// The descent tube: the surface half you press DOWN on, and the vault half you
/// drop out of directly below it.
const PIPE_NAME: &str = "secret_pipe";
const VAULT_ENTRY_PIPE_NAME: &str = "vault_entry_pipe";
/// The ascent tube: the vault half you press UP under, and the surface half you
/// rise out of directly above it.
const EXIT_PIPE_NAME: &str = "vault_return_pipe";
const SURFACE_EXIT_PIPE_NAME: &str = "surface_exit_pipe";
const VAULT_STONE_COLOR: [f32; 4] = [0.24, 0.20, 0.30, 1.0];

/// Coins waiting in the vault. The whole reward for finding the pipe.
const VAULT_COINS: usize = 8;

/// The vault's interior, in world coordinates: a sealed chamber dug under the
/// ground slab, directly below the pipe that leads into it.
///
/// One function so the level geometry, the warp destination, and the coin row
/// can never disagree about where the room actually is.
///
/// It is a WIDE 18-tile chamber at columns [23,41], tucked entirely under the SOLID
/// ground between pit A [20,22) and the (shifted-right) pit B [42,45). The original
/// 14-tile chamber reached under pit B when pit B sat at [34,37), so its ceiling
/// (the ground slab) had the pit's gap punched through it — a body that fell into
/// pit B dropped straight into the secret ("the hole in the ceiling"). The level was
/// LENGTHENED (pit B and everything past it pushed 8 tiles right) so a roomy vault
/// fits under unbroken ground here, still directly under the entry pipe (col 26) so
/// warping down is a short camera move, not a pan across the level.
pub fn vault_bounds() -> ae::Aabb {
    let ceiling = SURFACE_HEIGHT;
    let floor = SURFACE_HEIGHT + (VAULT_DEPTH_TILES - 2.0) * T;
    let left = 23.0 * T;
    let size = ae::Vec2::new(18.0 * T, floor - ceiling);
    let min = ae::Vec2::new(left, ceiling);
    ae::Aabb::new(min + size * 0.5, size * 0.5)
}

/// How big a SURFACE half of a tube is: two tiles, the classic stub you stand on.
fn surface_pipe_size() -> ae::Vec2 {
    ae::Vec2::new(PIPE_WIDTH_TILES * T, PIPE_HEIGHT_TILES * T)
}

/// How big a VAULT half is. It reaches most of the way DOWN the chamber rather
/// than being a matching two-tile stub, so its lip hangs at STANDING HEAD HEIGHT
/// instead of near the ceiling five tiles up — see [`vault_pipe_clearance`] for
/// why that height exactly. The chamber is [`VAULT_DEPTH_TILES`] - 2 tiles tall.
fn vault_pipe_size() -> ae::Vec2 {
    let interior = vault_bounds().max.y - vault_bounds().min.y;
    ae::Vec2::new(PIPE_WIDTH_TILES * T, interior - vault_pipe_clearance())
}

/// The min corner of the SURFACE half of the tube at `column` — standing on the
/// ground slab, mouth up.
fn surface_pipe_min(column: f32) -> ae::Vec2 {
    let ground_top = SURFACE_HEIGHT - GROUND_TILES * T;
    ae::Vec2::new(column * T, ground_top - PIPE_HEIGHT_TILES * T)
}

/// The min corner of the VAULT half of the tube at `column` — hanging from the
/// vault's ceiling (which is the level's own ground slab), mouth down.
fn vault_pipe_min(column: f32) -> ae::Vec2 {
    ae::Vec2::new(column * T, vault_bounds().min.y)
}

/// Walking room left under a vault half's lip: **exactly one grown body, plus a
/// hair.** That number is not a taste call — it is what makes the mouth reachable.
///
/// You enter a pipe by TOUCHING its mouth (see [`mouth_band`]), so a hanging pipe
/// has to hang where a body standing under it can touch. Any less and her grown
/// form cannot walk under it at all — she would be stopped by its side, outside
/// the column, and could never reach the mouth. Any more and the mouth floats
/// above every reachable head, which is the state that made pressing UP feel like
/// a button that worked in a column of air.
///
/// The quarter-tile is clearance for the walk itself, so she is not scraping the
/// lip on every step.
fn vault_pipe_clearance() -> f32 {
    powerups::tall_body_size().y + 0.25 * T
}

/// **Every pipe half in level 1-1**, as `(name, min corner, size, mouth points
/// down, art index)`. ONE list, so the collision blocks and the art can never
/// disagree about where a pipe is or which way it faces.
fn pipe_halves() -> [(&'static str, ae::Vec2, ae::Vec2, bool, u16); 4] {
    [
        // The descent tube: you press DOWN on this one...
        (
            PIPE_NAME,
            surface_pipe_min(PIPE_COLUMN),
            surface_pipe_size(),
            false,
            0,
        ),
        // ...and fall out of this one, which hangs from the ceiling right below.
        (
            VAULT_ENTRY_PIPE_NAME,
            vault_pipe_min(PIPE_COLUMN),
            vault_pipe_size(),
            true,
            1,
        ),
        // The ascent tube: you press UP under this one near the vault's far end...
        (
            EXIT_PIPE_NAME,
            vault_pipe_min(EXIT_PIPE_COLUMN),
            vault_pipe_size(),
            true,
            2,
        ),
        // ...and rise out of this one, straight up through the slab.
        (
            SURFACE_EXIT_PIPE_NAME,
            surface_pipe_min(EXIT_PIPE_COLUMN),
            surface_pipe_size(),
            false,
            3,
        ),
    ]
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

/// The mouth of the pipe half called `name`, straight off [`pipe_halves`].
fn mouth_of(name: &str) -> ae::Aabb {
    let (_, min, size, mouth_down, _) = pipe_halves()
        .into_iter()
        .find(|half| half.0 == name)
        .expect("every named mouth belongs to a pipe half in this level");
    mouth_band(min, size, mouth_down)
}

/// The mouth of the descent tube — the open top of the pipe you stand on.
pub fn pipe_mouth() -> ae::Aabb {
    mouth_of(PIPE_NAME)
}

/// Where the descent tube drops you: out of its VAULT half's mouth, so you fall
/// out of a pipe you can see rather than materializing in open stone.
pub fn vault_arrival() -> ae::Vec2 {
    let min = vault_pipe_min(PIPE_COLUMN);
    ae::Vec2::new(
        min.x + PIPE_WIDTH_TILES * 0.5 * T,
        min.y + vault_pipe_size().y + 0.5 * T,
    )
}

/// Where the ascent tube puts you: on top of its SURFACE half, directly above the
/// vault pipe you entered — twelve tiles further into the level than you went down.
pub fn pipe_arrival() -> ae::Vec2 {
    let min = surface_pipe_min(EXIT_PIPE_COLUMN);
    ae::Vec2::new(min.x + PIPE_WIDTH_TILES * 0.5 * T, min.y - T)
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

/// Build Mary-O's level 1-1 through the `ambition` umbrella surface ONLY.
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
pub fn level_1_1() -> RoomSpec {
    let width = LEVEL_WIDTH; // 104 tiles — a real 1-1 is ~210; this is its grammar.
    let height = LEVEL_HEIGHT;
    let ground_top = SURFACE_HEIGHT - GROUND_TILES * T;

    let mut blocks = Vec::new();

    // A ground segment spanning tiles `[from, to)`, full depth. Surfaces are
    // TILED, not stretched entity art — the engine default a game should reach for
    // (`Block::solid_tiled`). `idx` keeps each segment's tile-layer geo id unique.
    let ground = |blocks: &mut Vec<ae::Block>, name: &str, idx: u16, from: f32, to: f32| {
        blocks.push(ae::Block::solid_tiled(
            name,
            ae::Vec2::new(from * T, ground_top),
            ae::Vec2::new((to - from) * T, GROUND_TILES * T),
            "mary_o_ground",
            idx,
        ));
    };

    // 1 + 3. Open teach, then the widening pit rhythm. The first run after pit A is
    // extended (34 → 42) to roof the WIDE secret vault below it with unbroken ground;
    // pit B and everything past it were pushed 8 tiles right to make that room.
    ground(&mut blocks, "ground_open_teach", 0, 0.0, 20.0);
    ground(&mut blocks, "ground_after_pit_a", 1, 22.0, 42.0); // 2-tile pit at [20,22)
    ground(&mut blocks, "ground_after_pit_b", 2, 45.0, 60.0); // 3-tile pit at [42,45)
    ground(&mut blocks, "ground_after_pit_c", 3, 65.0, 104.0); // 5-tile pit at [60,65)

    // 2. The first platform: over SAFE ground, at jump height. Tiled, like the
    // ground it teaches you to leave.
    blocks.push(ae::Block::one_way_tiled(
        "teach_platform",
        ae::Vec2::new(12.0 * T, ground_top - 4.0 * T),
        ae::Vec2::new(3.0 * T, 0.5 * T),
        "mary_o_platform",
        0,
    ));

    // 3. The widest pit's stepping stone: the same jump, now load-bearing. (Pit C
    // moved to [60,65) with the +8 shift, so its stone follows to column 62.)
    blocks.push(ae::Block::one_way_tiled(
        "pit_c_stepping_stone",
        ae::Vec2::new(62.0 * T, ground_top - 3.0 * T),
        ae::Vec2::new(1.0 * T, 0.5 * T),
        "mary_o_platform",
        1,
    ));

    // The ?-blocks: SOLID one-tile blocks floating at bonk height. Jump into one
    // from below and the milk powerup pops out (see `powerups`). They are plain
    // level geometry here; the powerup runtime recognizes a bonked ?-block by the
    // durable `GeoId` `solid_tiled` stamps — `power_block_id(i)` re-derives the
    // SAME id, so the level and the runtime never drift.
    for i in 0..POWER_BLOCK_COLUMNS.len() {
        let min = power_block_min(i);
        blocks.push(ae::Block::solid_tiled(
            format!("power_block_{i}"),
            min,
            ae::Vec2::new(T, T),
            POWER_BLOCK_LAYER,
            POWER_BLOCK_BASE_INDEX + i as u16,
        ));
    }

    // The breakable bricks: SOLID one-tile blocks, same as any wall until a
    // head-bonk breaks one. Plain level geometry here; `bricks::break_bricks`
    // recognizes a bonked brick by the durable `GeoId` `solid_tiled` stamps —
    // `brick_id(i)` re-derives the SAME id, so the level and the runtime never
    // drift, exactly like the ?-blocks above.
    for i in 0..BRICK_COLUMNS.len() {
        let min = brick_min(i);
        blocks.push(ae::Block::solid_tiled(
            format!("brick_{i}"),
            min,
            ae::Vec2::new(T, T),
            BRICK_LAYER,
            BRICK_BASE_INDEX + i as u16,
        ));
    }

    // 4. The stair pyramid: four up at x=74.., a gap, four down ending at x=83.
    // (Shifted +8 with the rest of the post-vault level.)
    for step in 1..=4u16 {
        let h = step as f32;
        blocks.push(ae::Block::solid_tiled(
            format!("stair_up_{step}"),
            ae::Vec2::new((73.0 + h) * T, ground_top - h * T),
            ae::Vec2::new(T, h * T),
            "mary_o_stairs",
            step,
        ));
        blocks.push(ae::Block::solid_tiled(
            format!("stair_down_{step}"),
            ae::Vec2::new((84.0 - h) * T, ground_top - (5.0 - h) * T),
            ae::Vec2::new(T, (5.0 - h) * T),
            "mary_o_stairs",
            step + 4,
        ));
    }

    // 5. The goal. ONE-WAY, not solid: touching it must END the level, and a
    // solid pole stops the body a half-body-width short of its own center, so the
    // grab could only ever fire from above the top. One-way lets her run straight
    // into it at any height while still holding her up if she drops onto the top
    // from the stairs.
    //
    // ART: the collision blocks are made VISUALLY TRANSPARENT and a decorative
    // `super_mary_o_flag_pole_*` construction sprite is layered over each (see
    // `scenery.rs`). The shared block art can only key on `BlockKind`, so a
    // one-way pole would take the platform texture stretched down a 16x288 column
    // into a smear; a zero-alpha `art_color` shortcuts that, and the prop supplies
    // the real look. Collision is unchanged, so the flag grab band is identical.
    //
    // Three pieces so the goal READS as a flagpole instead of a bar: a shaft, a
    // finial capping it, and a banner hanging off the top. All three are the SAME
    // width and column as the pole, so none of them changes what is reachable or
    // where the grab band is — the silhouette is new, the level is not.
    let pole_x = POLE_COLUMN * T;
    let pole_top = ground_top - 9.0 * T;
    blocks.push(
        ae::Block::one_way(
            "goal_pole",
            ae::Vec2::new(pole_x, pole_top),
            ae::Vec2::new(POLE_WIDTH, 9.0 * T),
        )
        .with_art_color(scenery::TRANSPARENT),
    );
    blocks.push(
        ae::Block::one_way(
            "goal_pole_knob",
            ae::Vec2::new(pole_x, pole_top - POLE_WIDTH),
            ae::Vec2::splat(POLE_WIDTH),
        )
        .with_art_color(scenery::TRANSPARENT),
    );
    blocks.push(
        ae::Block::one_way(
            "goal_pole_banner",
            ae::Vec2::new(pole_x, pole_top + POLE_WIDTH),
            ae::Vec2::new(POLE_WIDTH, POLE_WIDTH * 2.0),
        )
        .with_art_color(scenery::TRANSPARENT),
    );

    // ── 6. The secret pipe, and the vault under the level ───────────────────
    //
    // A warp pipe standing on safe ground between pit A and pit B. Press DOWN on
    // its mouth and you drop into a sealed coin vault built into the SAME room,
    // below the ground slab — which is why the world grew downward rather than a
    // second room being authored: cross-room transition lives in `ambition_app`'s
    // `world_flow`, so a demo that ships its own app could not use it and would
    // have worked only when Ambition hosted it.
    //
    // The vault is reachable ONLY through the pipe: it is walled on all four
    // sides, and the ground slab above is its ceiling.
    //
    // Because BOTH ends of both warps live in this one room, each warp is one
    // physical TUBE through that slab, authored as two halves at matching columns:
    // surface + vault for the descent at col 26, vault + surface for the ascent at
    // col 35. Transparent collision blocks; the props below supply the look.
    // A surface half is the classic two-tile stub; a vault half reaches down the
    // chamber so its mouth hangs at head height (`mouth_down` is also what
    // mirrors its lip art).
    for (name, min, size, _, art_index) in pipe_halves() {
        blocks.push(
            ae::Block::solid_tiled(name, min, size, "mary_o_pipe", art_index)
                .with_art_color(scenery::TRANSPARENT),
        );
    }

    let vault = vault_bounds();
    let wall = T;
    // Floor, then the two side walls. The ceiling is the level's own ground
    // slab, so a vault dug directly under solid ground needs no lid.
    blocks.push(
        ae::Block::solid_tiled(
            "vault_floor",
            ae::Vec2::new(vault.min.x - wall, vault.max.y),
            ae::Vec2::new(vault.max.x - vault.min.x + wall * 2.0, wall),
            "mary_o_ground",
            10,
        )
        .with_art_color(VAULT_STONE_COLOR),
    );
    for (idx, x) in [(11u16, vault.min.x - wall), (12, vault.max.x)] {
        blocks.push(
            ae::Block::solid_tiled(
                "vault_wall",
                ae::Vec2::new(x, vault.min.y),
                ae::Vec2::new(wall, vault.max.y - vault.min.y),
                "mary_o_ground",
                idx,
            )
            .with_art_color(VAULT_STONE_COLOR),
        );
    }

    let spawn = ae::Vec2::new(2.0 * T, ground_top - 2.0 * T);
    let world = ae::World::new("Mary-O 1-1", ae::Vec2::new(width, height), spawn, blocks);

    let mut room = RoomSpec::new(LEVEL_1_1_ROOM_ID, world);
    room.metadata.mode = Some(MARY_O_MODE.to_string());

    // ── Scenery: the flagpole + warp-pipe LOOK (decorative props over the
    // transparent collision blocks; see `scenery.rs`). Presentation only — none
    // of this changes geometry, the grab band, or the warp mouths.
    //
    // Flagpole: the finial caps the shaft, the shaft runs its full height, and the
    // banner hangs off the top. The banner PROP is wider than its (pole-width)
    // collision block and offset to the side, so the flag reads as a flag without
    // widening what the body can touch.
    let mut scenery_props = vec![
        // BUILT WORLD, not scenery: character sizing derives a sprite's width
        // from the box's LONGEST side, so a shaft whose box is nine tiles tall
        // was drawn eighteen tiles WIDE. It stays behind the cast, though —
        // she has to be visible climbing it.
        scenery::structure_prop(
            "goal_pole_shaft_art",
            scenery::POLE_BODY_SPRITE,
            ae::Vec2::new(pole_x, pole_top),
            ae::Vec2::new(POLE_WIDTH, 9.0 * T),
        ),
        scenery::prop_over(
            "goal_pole_finial_art",
            scenery::POLE_TOP_SPRITE,
            ae::Vec2::new(pole_x - POLE_WIDTH * 0.5, pole_top - POLE_WIDTH * 1.5),
            ae::Vec2::splat(POLE_WIDTH * 2.0),
        ),
        // The banner: wider than the pole and hung to the right of the shaft top.
        PropSpec {
            id: "goal_pole_banner_art".to_string(),
            name: "goal_pole_banner_art".to_string(),
            kind: scenery::FLAG_SPRITE.to_string(),
            pos: ae::Vec2::new(pole_x + T, pole_top + T),
            size: ae::Vec2::new(1.5 * T, 1.5 * T),
            flip_y: false,
            draw: Default::default(),
        },
    ];
    // Every pipe half, tiled over its OWN block from the same list the blocks came
    // from: one LIP tile at the open end and body tiles filling the rest. The lip
    // is what makes a pipe point somewhere — top tile for a surface half (mouth up,
    // you drop in), BOTTOM tile for a vault half (mouth down, hanging from the
    // ceiling — you fall out of it, or rise into it), mirrored to match.
    for (name, min, size, mouth_down, _) in pipe_halves() {
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
                scenery_props.push(scenery::pipe_prop(
                    &format!("{name}_lip_art"),
                    scenery::PIPE_TOP_SPRITE,
                    at,
                    tile,
                    mouth_down,
                ));
            } else {
                scenery_props.push(scenery::pipe_prop(
                    &format!("{name}_body_art_{row}"),
                    scenery::PIPE_BODY_SPRITE,
                    at,
                    tile,
                    false,
                ));
            }
        }
    }
    room.props.extend(scenery_props);

    // The vault's reward, on the ordinary placements channel: `currency`
    // pickups the SHARED economy collects and credits to the body wallet. No
    // demo collection code, and they land in the HUD's COINS readout for free —
    // the same path Sanic's rings already take.
    let vault = vault_bounds();
    let coin_y = vault.max.y - 1.5 * T;
    for i in 0..VAULT_COINS {
        // Spread across the wide chamber: 8 coins at 2-tile spacing span [+1,+15]
        // inside the 18-tile vault.
        let x = vault.min.x + (1.0 + i as f32 * 2.0) * T;
        room.placements
            .push(ambition::world::placements::PlacementRecord::new(
                format!("vault_coin_{i}"),
                ambition::entity_catalog::placements::PlacementSchema::Pickup(
                    ambition::entity_catalog::placements::PickupSpec::new(
                        ambition::entity_catalog::placements::PickupKindSpec::Currency {
                            amount: 1,
                        },
                    ),
                ),
                {
                    let size = ae::Vec2::splat(0.75 * T);
                    ae::Aabb::new(ae::Vec2::new(x, coin_y) + size * 0.5, size * 0.5)
                },
            ));
    }
    room
}

/// The min corner of ?-block `i`, from the SAME constants [`level_1_1`] builds the
/// `power_block_*` blocks out of — so the powerup runtime pops the milk out at the
/// exact block it was authored at.
pub fn power_block_min(i: usize) -> ae::Vec2 {
    let ground_top = SURFACE_HEIGHT - GROUND_TILES * T;
    ae::Vec2::new(POWER_BLOCK_COLUMNS[i] * T, ground_top - POWER_BLOCK_ROW * T)
}

/// The durable [`GeoId`](ae::GeoId) of ?-block `i` — the SAME id `solid_tiled`
/// stamps in [`level_1_1`], which the engine reports on a head-bonk contact
/// (`ContactSource::Block`). Matching against this is how the powerup runtime
/// knows a specific ?-block was struck, with no point-matching.
pub fn power_block_id(i: usize) -> ae::GeoId {
    ae::GeoId::tile_layer(POWER_BLOCK_LAYER, POWER_BLOCK_BASE_INDEX + i as u16)
}

/// If `id` is one of the ?-blocks, its column index — the inverse of
/// [`power_block_id`]. `None` for any other block the player bonks.
pub fn power_block_index_for(id: &ae::GeoId) -> Option<usize> {
    (0..POWER_BLOCK_COLUMNS.len()).find(|&i| power_block_id(i) == *id)
}

/// The min corner of brick `i`, from the SAME constants [`level_1_1`] builds the
/// `brick_*` blocks out of — so the break runtime removes the exact authored brick.
pub fn brick_min(i: usize) -> ae::Vec2 {
    let ground_top = SURFACE_HEIGHT - GROUND_TILES * T;
    ae::Vec2::new(BRICK_COLUMNS[i] * T, ground_top - BRICK_ROW * T)
}

/// The durable [`GeoId`](ae::GeoId) of brick `i` — the SAME id `solid_tiled` stamps
/// in [`level_1_1`], which the engine reports on a head-bonk contact
/// (`ContactSource::Block`). Matching against this is how [`bricks::break_bricks`]
/// knows a specific brick was struck, with no point-matching.
pub fn brick_id(i: usize) -> ae::GeoId {
    ae::GeoId::tile_layer(BRICK_LAYER, BRICK_BASE_INDEX + i as u16)
}

/// If `id` is one of the bricks, its index — the inverse of [`brick_id`]. `None`
/// for any other block. Disjoint from [`power_block_index_for`] by construction
/// (the two use different `GeoId` base indices), so a bonk is a ?-block OR a brick,
/// never both.
pub fn brick_index_for(id: &ae::GeoId) -> Option<usize> {
    (0..BRICK_COLUMNS.len()).find(|&i| brick_id(i) == *id)
}

/// The authored NAME of brick `i` (`brick_<i>`) — the key the collision overlay's
/// `removed_block_names` subtraction and the render reconcile both match on. Kept
/// beside [`brick_id`] so the name and the id are derived from the same `i`.
pub fn brick_name(i: usize) -> String {
    format!("brick_{i}")
}

/// How many bricks the level authors.
pub const BRICK_COUNT: usize = BRICK_COLUMNS.len();

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

/// The demo's one-character catalog. Every demo installs its own roster; the
/// engine ships none (ADR 0017).
const MARY_O_CATALOG_RON: &str = r#"(
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
            spritesheet: "sprites/super_mary_o_spritesheet.png",
            manifest: "sprites/super_mary_o_spritesheet.ron",
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
            axis_tuning: Some((
                horizontal_law: Momentum((
                    ground_reverse_accel: 1500.0,
                    ground_coast_decel: 1200.0,
                    air_reverse_accel: 900.0,
                    air_coast_decel: 0.0,
                )),
                jump_law: PhasedGravity((
                    speed_thresholds: (120.0, 240.0, 360.0),
                    launch_speeds: (420.0, 435.0, 450.0, 480.0),
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
            )),
            playable_kit: Authored,
            tags: ["player"],
            barks: (
                hall: ["I solve masonry disputes from below.", "One jump. No second opinions, no insurance.", "Every pipe is hiding something."],
            ),
            hall_dialogue_id: Some("hall_mary_o"),
        ),
        // TALL Mary-O: the grown form. A milk-powerup swaps the worn identity to
        // this row (a distinct SHEET — `super_mary_o_tall` — not a scaled copy of
        // the small sheet, per Jon), and the powerup runtime bumps her body size so
        // the taller art draws bigger. Kit is byte-identical to `mary_o` — same
        // grant list, same Mary-O Classic `axis_tuning` (re-wearing re-reads
        // `axis_tuning`, so a mismatch here would silently shrink her jump on grow)
        // and the same peaceful Authored kit — so growing changes only her LOOK and
        // size, never her moveset.
        "mary_o_tall": (
            display_name: "Mary-O (Tall)",
            spritesheet: "sprites/super_mary_o_tall_spritesheet.png",
            manifest: "sprites/super_mary_o_tall_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            abilities: Some([RunJump]),
            axis_tuning: Some((
                horizontal_law: Momentum((
                    ground_reverse_accel: 1500.0,
                    ground_coast_decel: 1200.0,
                    air_reverse_accel: 900.0,
                    air_coast_decel: 0.0,
                )),
                jump_law: PhasedGravity((
                    speed_thresholds: (120.0, 240.0, 360.0),
                    launch_speeds: (420.0, 435.0, 450.0, 480.0),
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
            )),
            playable_kit: Authored,
            tags: ["player"],
            barks: (
                hall: ["One power-up; every ceiling gets an opinion.", "Ask the doorframes whether taller is better.", "I shrink after one professional-grade mistake."],
            ),
            hall_dialogue_id: Some("hall_mary_o_tall"),
        ),
        // FIRE Mary-O: the spark-blossom (fire-flower) form. A second power-up
        // ABOVE the cap swaps the worn identity to this row — a DISTINCT fire sheet
        // (`super_mary_o_fire`, the white-and-red fire palette with its own
        // fireball pose), the SAME height as the grown form so `sync_grown_form`
        // changes only her LOOK + spark loadout, never her size, on the
        // grown↔fire transition. Kit mirrors `mary_o_tall` byte-for-byte: the
        // fireball is granted by WEARING the spark blossom (see `MaryOSpark`), not
        // by this row, so becoming fire never alters her base moveset. Before this
        // she wore the plain tall sheet while spark-powered, so there was no
        // visible fire form at all (Jon bug #10).
        "mary_o_fire": (
            display_name: "Mary-O (Fire)",
            spritesheet: "sprites/super_mary_o_fire_spritesheet.png",
            manifest: "sprites/super_mary_o_fire_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            abilities: Some([RunJump]),
            axis_tuning: Some((
                horizontal_law: Momentum((
                    ground_reverse_accel: 1500.0,
                    ground_coast_decel: 1200.0,
                    air_reverse_accel: 900.0,
                    air_coast_decel: 0.0,
                )),
                jump_law: PhasedGravity((
                    speed_thresholds: (120.0, 240.0, 360.0),
                    launch_speeds: (420.0, 435.0, 450.0, 480.0),
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
            )),
            playable_kit: Authored,
            tags: ["player"],
            barks: (
                hall: ["One blossom, and every ceiling gets a warm answer.", "I throw solutions now — mind the sparks.", "Fireproof opinions, freshly lit."],
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
        ),
    },
)"#;

/// Content plugin: registers Mary-O's App-local character fragment, installs
/// the level, and adds the engine's sim-world setup. The shape `crates/ambition_host/tests/demo_shell_smoke.rs` prescribes.
pub struct MaryODemoContentPlugin;

/// Register Mary-O's immutable authored character fragment in one Bevy `App`.
/// Shared by the historical [`MaryODemoContentPlugin`] (Startup construction) and
/// the new [`provider::MaryOExperiencePlugin`] (shell-activation construction).
pub fn install_mary_o_content(app: &mut App) {
    use ambition::characters::actor::character_catalog::{
        CharacterCatalogAppExt, CharacterCatalogFragment,
    };

    app.register_character_catalog_fragment(
        CharacterCatalogFragment::from_ron(
            provider::MARY_O_EXPERIENCE,
            Some(provider::MARY_O_CHARACTER_ID),
            MARY_O_CATALOG_RON,
        )
        .expect("Mary-O character catalog should be valid"),
    );
    // Mary-O's two enemies — Solid Snake (the shell) and AI Slop (the plain
    // stomp-and-die walker) — are authored content, so install their archetypes and
    // room stagers before direct or shell preparation fingerprints the App. Both
    // archetypes share ONE roster fragment: assembly rejects a second fragment from
    // the same provider, so the per-enemy `register_*_roster` helpers (used by
    // single-enemy tests) are folded here.
    {
        use ambition::actors::features::{CharacterRosterAppExt, CharacterRosterFragment};
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
    app.init_resource::<ambition::actors::features::RoomContentStagingRegistry>();
    {
        let mut registry = app
            .world_mut()
            .resource_mut::<ambition::actors::features::RoomContentStagingRegistry>();
        snake::register_snake_content_staging(&mut registry);
        ai_slop::register_ai_slop_content_staging(&mut registry);
    }
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
        ),
    );

    // Mary-O's mutable sim state joins the rollback contract through the same
    // seam engine crates use — here, before either construction path
    // fingerprints the App, so the schema fingerprint (part of the content
    // identity) includes these rows; a non-GGRS shell records metadata only.
    {
        use ambition::runtime::rollback::AmbitionRollbackApp;
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
            // A transit in flight is authoritative sim state — it OWNS the body's
            // position for half a second, so a rewind that dropped it would put a
            // half-swallowed player back on the surface. It rides on the player
            // BODY, which the engine already anchors.
            .rollback_component_clone::<pipe::PipeTransit>(
                "ambition_demo_mary_o",
                "content.mary_o_pipe_transit",
            )
            .rollback_component_clone::<pipe::PipeEntryLatch>(
                "ambition_demo_mary_o",
                "content.mary_o_pipe_entry_latch",
            );
    }
}

impl Plugin for MaryODemoContentPlugin {
    fn build(&self, app: &mut App) {
        use ambition::runtime::demo_fixture::{ActiveRoomMetadata, RoomSet};
        use bevy::prelude::IntoScheduleConfigs;

        install_mary_o_content(app);
        let room = level_1_1();
        let source = ambition::runtime::PreparedPlatformerSource::new(
            provider::MARY_O_EXPERIENCE,
            RoomSet::from_parts(LEVEL_1_1_ROOM_ID, vec![room.clone()], Vec::new()),
            ae::RoomGeometry(room.world.clone()),
            ActiveRoomMetadata(room.metadata.clone()),
            ambition::runtime::demo_fixture::StartingCharacter::new(provider::MARY_O_CHARACTER_ID),
            ambition::runtime::demo_fixture::LdtkRuntimeIndex::default(),
        );
        let content = ambition::provider::prepare_platformer_content_for_app(
            app,
            source,
            &provider::mary_o_authored_catalogs(),
        )
        .expect("Mary-O direct prepared-content assembly must succeed");
        app.world_mut().spawn((
            ambition::platformer::lifecycle::SessionRoot(
                ambition::platformer::lifecycle::SessionScopeId(0),
            ),
            content.source().instantiate_live(),
            content.identity(),
            content,
        ));
        app.add_systems(
            bevy::app::Startup,
            mary_o_setup.in_set(ambition::runtime::demo_fixture::SimulationSetupSet),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn mary_o_setup(
    mut commands: bevy::prelude::Commands,
    world: ambition::platformer::lifecycle::SessionWorldRef<ae::RoomGeometry>,
    room_set: ambition::platformer::lifecycle::SessionWorldRef<
        ambition::runtime::demo_fixture::RoomSet,
    >,
    ldtk_index: ambition::platformer::lifecycle::SessionWorldRef<
        ambition::runtime::demo_fixture::LdtkRuntimeIndex,
    >,
    editable_abilities: bevy::prelude::Res<ambition::runtime::demo_fixture::EditableAbilitySet>,
    tuning: bevy::prelude::Res<ambition::runtime::demo_fixture::ActiveMovementTuning>,
    starting_character: ambition::platformer::lifecycle::SessionWorldRef<
        ambition::runtime::demo_fixture::StartingCharacter,
    >,
    asset_server: bevy::prelude::Res<bevy::asset::AssetServer>,
    character_catalog: bevy::prelude::Res<
        ambition::characters::actor::character_catalog::CharacterCatalog,
    >,
    character_roster: bevy::prelude::Res<ambition::actors::features::CharacterRoster>,
    boss_catalog: bevy::prelude::Res<ambition::actors::boss_encounter::BossCatalog>,
    placement_lowering: bevy::prelude::Res<
        ambition::runtime::demo_fixture::PlacementLoweringRegistry,
    >,
    content_staging: bevy::prelude::Res<
        ambition::runtime::demo_fixture::RoomContentStagingRegistry,
    >,
    construction_recipes: bevy::prelude::Res<
        ambition::runtime::demo_fixture::ActorConstructionRegistry,
    >,
) {
    ambition::runtime::demo_fixture::simulation_world(
        &mut commands,
        ambition::platformer::lifecycle::SessionSpawnScope::UNSCOPED,
        ambition::runtime::demo_fixture::SimulationSetup {
            world: &world,
            room_set: &room_set,
            ldtk_index: &ldtk_index,
            editable_abilities: &editable_abilities,
            tuning: &tuning,
            starting_character: &starting_character,
            character_catalog: &character_catalog,
            character_roster: &character_roster,
            placement_lowering: &placement_lowering,
            content_staging: &content_staging,
            // A demo enters directly rather than through provider activation,
            // so it has no prepared-content generation to state.
            construction: ambition::runtime::demo_fixture::ActorConstructionContext::new(
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
        let sim = ambition::platformer::schedule::SimScheduleExt::sim_schedule(app);
        app.insert_resource(goal_pole());
        app.init_resource::<powerups::SpentPowerBlocks>();
        app.init_resource::<bricks::BrokenBricks>();
        // The brick overlay contributor writes the collision overlay; a full app
        // inserts it (features/render plugins), but a thin rules-only harness may
        // not, and `init_resource` is idempotent — a no-op when already present.
        app.init_resource::<ambition::actors::features::FeatureEcsWorldOverlay>();
        // The cycle emitter writes this; the host's replay consumer drains it. The
        // engine registers it too (`SandboxResetSchedulePlugin`), but a thin host
        // may not, and `add_message` is idempotent — a no-op when already present.
        app.add_message::<ambition::actors::session::reset::RoomReplayRequested>();
        // The authoritative attempt-lost fact `spend_lives_on_death` reads. The
        // engine registers it in `SimCoreResourcesPlugin`; a rules-only harness
        // does not, and a missing message is a hard system-param panic rather
        // than a skip. Idempotent, same as the rest of this block.
        app.add_message::<ambition::actors::ActorDiedMessage>();
        // The snake stager reads room-load facts and writes spawn requests; the
        // engine registers both in a full app, but a thin rules-only test harness
        // may not, and `add_message` is idempotent.
        app.add_message::<ambition::actors::rooms::RoomLoaded>();
        app.add_message::<ambition::actors::features::SpawnActorRequest>();
        // The snake squash pops a dust burst through the engine's vfx seam; a full
        // app registers this via the presentation plugins, but a thin rules-only
        // harness may not, and `add_message` is idempotent.
        app.add_message::<ambition::vfx::VfxMessage>();
        // Same story for the cue queue: the brick-break voices through the shared
        // sfx seam, a full app registers this via the audio plugins, and a thin
        // rules-only harness may not. `add_message` is idempotent.
        app.add_message::<ambition::sfx::OwnedSfxMessage>();
        // A sliding snake shell deals damage through the shared `HitEvent` pipeline
        // (`run_snake_shells`); the full app registers this via the engine's damage
        // plugin, but a thin rules-only harness may not. `add_message` is idempotent.
        app.add_message::<ambition::actors::features::HitEvent>();
        // Level progression lives in the canonical gameplay-effects phase. The
        // flag runs before the clock; the cycle emitter runs last so it sees the
        // settled tally and its clock reset is not immediately decremented.
        let rules = (
            spawn_mary_o_mode_owner,
            flag::run_flag_sequence,
            tick_level_clock,
            // Reads the clock the tick above just settled, so a timeout is spent
            // on the frame it happens rather than one late.
            spend_lives_on_death,
            cycle_level_on_flag_tally,
        )
            .chain()
            .in_set(ambition::platformer::schedule::SandboxSet::GameplayEffects);
        // Pipe input is authoritative rollback state on the player body. Entry
        // and transit run after ordinary WorldPrep movement, so the scripted
        // position wins this frame instead of racing the shared integrator.
        let pipe_input = pipe::ensure_pipe_entry_latch
            .in_set(ambition::platformer::schedule::SandboxSet::PlayerInput)
            .after(ambition::actors::avatar::tick_player_brains)
            .before(warp_through_secret_pipe);
        let pipe_rules = (warp_through_secret_pipe, pipe::run_pipe_transits)
            .chain()
            .in_set(ambition::platformer::schedule::SandboxSet::PlayerSimulation)
            .after(ambition::actors::features::ecs::damage_apply::apply_player_hit_events);
        // The walkers are registered by `install_mary_o_content`, the single
        // authored-content composition seam shared by direct and shell hosts.
        // Rules consume the staged actors; they do not mutate construction
        // registries after prepared-content fingerprinting.
        // Tag freshly staged enemies, then run each one's stomp mechanic. Both run
        // BEFORE the engine's shared body-contact-damage pass so a stomp resolves
        // the enemy (snake → inert shell; AI Slop → dead) that frame, which the
        // contact pass then skips — the stomper is never also hurt.
        let cronies = (
            snake::tag_mary_o_snakes,
            ai_slop::tag_mary_o_ai_slop,
            snake::run_snake_shells.before(ambition::actors::features::apply_actor_contact_damage),
            ai_slop::bounce_squash_ai_slop
                .before(ambition::actors::features::apply_actor_contact_damage),
        )
            .chain()
            .in_set(ambition::platformer::schedule::SandboxSet::WorldPrep);
        // The powerup rules on the two engine primitives: re-arm the ?-blocks on
        // (re)load, pop milk on a head-bonk, and keep the tall form in sync with
        // wearing the cap. The engine's `collect_world_items` (touch → equip) sits
        // between the bonk and the grow — no demo wiring for it.
        let powerups = (
            powerups::refill_power_blocks_on_room_loaded,
            powerups::bonk_power_blocks,
            powerups::sync_grown_form,
            powerups::tag_mary_o_sparks,
        )
            .chain()
            .in_set(ambition::platformer::schedule::SandboxSet::FeatureInteraction);
        // Mary-O's locomotion POLICY and her spark's press edge. Both read the
        // sustained control slot off the body's freshly-produced `ActorControl`,
        // so they sit after the brain tick and before the shared movement phase
        // consumes the frame — the throttle they set then flows through the
        // ordinary body path, replay and rollback included.
        let gait = (
            movement::ensure_gait,
            movement::walk_by_default_run_while_held,
            movement::fire_spark_on_run_press,
            movement::sync_run_action_scheme,
        )
            .chain()
            .in_set(ambition::platformer::schedule::SandboxSet::PlayerInput)
            .after(ambition::actors::avatar::tick_player_brains);
        // The bricks — the reactive-block primitive's SECOND consumer: re-arm on
        // (re)load, break the bonked one, and contribute broken bricks to the
        // collision overlay's `removed_block_names` so they stop colliding (and, via
        // the render reconcile, drawing). The contribution runs AFTER the engine's
        // overlay rebuild clears that list — the same slot `contribute_encounter_lock_walls`
        // takes — so the removals survive the per-frame clean slate.
        let bricks = (bricks::refill_bricks_on_room_loaded, bricks::break_bricks)
            .chain()
            .in_set(ambition::platformer::schedule::SandboxSet::FeatureInteraction);
        let brick_overlay = bricks::contribute_broken_bricks_to_overlay
            .in_set(ambition::platformer::schedule::SandboxSet::WorldPrep)
            .after(ambition::actors::features::rebuild_feature_ecs_world_overlay);
        if self.hosted {
            app.add_systems(sim, rules.run_if(ambition::runtime::in_mode(MARY_O_MODE)));
            app.add_systems(
                sim,
                pipe_input.run_if(ambition::runtime::in_mode(MARY_O_MODE)),
            );
            app.add_systems(
                sim,
                pipe_rules.run_if(ambition::runtime::in_mode(MARY_O_MODE)),
            );
            app.add_systems(sim, cronies.run_if(ambition::runtime::in_mode(MARY_O_MODE)));
            app.add_systems(
                sim,
                powerups.run_if(ambition::runtime::in_mode(MARY_O_MODE)),
            );
            app.add_systems(sim, bricks.run_if(ambition::runtime::in_mode(MARY_O_MODE)));
            app.add_systems(sim, gait.run_if(ambition::runtime::in_mode(MARY_O_MODE)));
            app.add_systems(
                sim,
                brick_overlay.run_if(ambition::runtime::in_mode(MARY_O_MODE)),
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
    session: Option<bevy::prelude::Res<ambition::platformer::lifecycle::ActiveSessionScope>>,
) {
    use ambition::platformer::lifecycle::{SessionSpawnScope, SpawnSessionScopedExt};
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
                (MaryOLevelState::default(), flag::FlagSequence::default()),
            )
            .insert(ambition::platformer::lifecycle::ModeScopedEntity(
                MARY_O_MODE.to_string(),
            ));
    }
}

/// The level clock runs on the SIM clock, so pause and bullet-time slow it exactly
/// as they slow everything else. It clamps at zero rather than going negative.
fn tick_level_clock(
    time: bevy::prelude::Res<ambition::time::WorldTime>,
    mut level: bevy::prelude::Query<(&mut MaryOLevelState, &flag::FlagSequence)>,
) {
    for (mut state, sequence) in &mut level {
        state.intro_card = (state.intro_card - time.scaled_dt).max(0.0);
        // A level whose flag has been grabbed is over. The clock stopping is what
        // turns the remaining time from a threat into a score.
        if sequence.active() {
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
    mut level: bevy::prelude::Query<&mut MaryOLevelState>,
    bodies: bevy::prelude::Query<
        bevy::prelude::Entity,
        ambition::platformer::markers::PrimaryPlayerOnly,
    >,
    mut deaths: bevy::prelude::MessageReader<ambition::actors::ActorDiedMessage>,
    mut replay: bevy::prelude::MessageWriter<ambition::actors::session::reset::RoomReplayRequested>,
) {
    // Drain unconditionally: the cursor must advance even on a frame with no
    // level, or a death that landed during a load would be re-read later and
    // charged to the next attempt.
    let died = deaths.read().count() > 0;

    let Ok(mut level) = level.single_mut() else {
        return;
    };
    // No body, no attempt in progress — so nothing to lose. This matters for
    // the TIMEOUT branch specifically: the level owner can exist for frames
    // before a body does, and a clock that reaches zero in that window is a
    // level that never started, not a life the player spent. (The old counter
    // version got this for free by querying the body's `BodyLifetime`; the
    // authoritative signal does not need the body, so the guard is now
    // explicit.)
    if bodies.iter().next().is_none() {
        return;
    }

    // The clock reaching zero is its own death, and it must not fire twice
    // while the replay is in flight — restoring the clock below is what
    // disarms it.
    let timed_out = level.time_remaining <= 0.0;
    if !died && !timed_out {
        return;
    }
    // ONE attempt lost costs ONE life, however many ways it was reported. A
    // frame can carry both a lethal hit and a hazard reset for the same fall.

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
    // A timeout has no engine respawn behind it, so ask for one. A pit death
    // already replayed the body; replaying the room too is what puts the
    // rebuilt level under her.
    replay.write(ambition::actors::session::reset::RoomReplayRequested);
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
    mut sfx: ambition::sfx::SfxWriter,
    mut bodies: bevy::prelude::Query<
        (
            bevy::prelude::Entity,
            &ae::BodyKinematics,
            &ambition::characters::brain::ActorControl,
            &mut pipe::PipeEntryLatch,
        ),
        (
            ambition::platformer::markers::PrimaryPlayerOnly,
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
        sfx.write(ambition::sfx::SfxMessage::Play {
            id: ambition::sfx::SfxId::new(pipe::PIPE_WARP_SFX),
            pos: kin.pos,
        });
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
/// [`RoomReplayRequested`]: ambition::actors::session::reset::RoomReplayRequested
fn cycle_level_on_flag_tally(
    time: bevy::prelude::Res<ambition::time::WorldTime>,
    mut dwell: bevy::prelude::Local<f32>,
    mut owners: bevy::prelude::Query<(&mut flag::FlagSequence, &mut MaryOLevelState)>,
    mut replay: bevy::prelude::MessageWriter<ambition::actors::session::reset::RoomReplayRequested>,
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
    replay.write(ambition::actors::session::reset::RoomReplayRequested);
}

/// Install the Mary-O demo content layer into an engine app.
pub fn add_demo_content(app: &mut App) {
    app.add_plugins(MaryODemoContentPlugin);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mary_o_demo_content_plugin_installs() {
        let mut app = App::new();
        // See the note in `movement::tests`: authored placements require the
        // engine foundation's lowering registry.
        ambition::engine::add_headless_foundation(&mut app);
        app.add_plugins(ambition::actors::features::WorldPrepSchedulePlugin);
        add_demo_content(&mut app);
        let catalog = app
            .world()
            .resource::<ambition::characters::actor::character_catalog::CharacterCatalog>();
        assert!(catalog.get(provider::MARY_O_CHARACTER_ID).is_some());
        // Her three power forms are all catalog characters — the small starting
        // sheet, the grown (milk/cap) sheet, and the fire (spark-blossom) sheet.
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
            ambition::actors::character_sprites::sheet_for_character_id_in(catalog, "solid_snake")
                .is_some(),
            "a sheet spec resolves for solid_snake — else it is skipped and renders as a goblin"
        );
        // AI Slop is the SECOND enemy: the plain stomp-and-die walker. Same guard —
        // its `ai_slop` sheet spec must resolve, or it renders as a goblin too.
        let ai_slop_row = catalog.get("ai_slop").expect("AI Slop is a catalog row");
        assert_eq!(ai_slop_row.display_name, ai_slop::AI_SLOP_DISPLAY_NAME);
        assert!(
            ambition::actors::character_sprites::sheet_for_character_id_in(catalog, "ai_slop")
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
            ambition::engine_core::AbilitySet::compose(&[
                ambition::engine_core::AbilityGrant::RunJump,
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

        let mary_o_tuning = catalog
            .axis_tuning(provider::MARY_O_CHARACTER_ID)
            .expect("Mary-O authors an axis tuning");
        let horizontal = match mary_o_tuning.horizontal_law {
            ambition::engine_core::AxisHorizontalLaw::Momentum(params) => params,
            other => panic!("Mary-O must use the momentum law, got {other:?}"),
        };
        assert_eq!(mary_o_tuning.max_run_speed, 300.0);
        assert_eq!(mary_o_tuning.run_accel, 393.75);
        assert_eq!(mary_o_tuning.air_accel, 393.75);
        assert_eq!(horizontal.ground_reverse_accel, 1500.0);
        assert_eq!(horizontal.ground_coast_decel, 1200.0);
        assert_eq!(horizontal.air_coast_decel, 0.0);

        let jump = match mary_o_tuning.jump_law {
            ambition::engine_core::AxisJumpLaw::PhasedGravity(params) => params,
            other => panic!("Mary-O must use phased gravity, got {other:?}"),
        };
        assert_eq!(jump.speed_thresholds, [120.0, 240.0, 360.0]);
        assert_eq!(jump.launch_speeds, [420.0, 435.0, 450.0, 480.0]);
        assert_eq!(jump.held_rise_gravity_scale, 0.2);
        assert_eq!(jump.released_rise_gravity_scale, 1.0);
        assert_eq!(jump.fall_gravity_scale, 1.0);
        assert_eq!(jump.held_phase_min_upward_speed, 240.0);
        assert_eq!(mary_o_tuning.max_fall_speed, 480.0);
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
            .resource::<ambition::characters::actor::character_catalog::CharacterCatalogDefaults>(
        );
        assert_eq!(
            defaults.for_provider(provider::MARY_O_EXPERIENCE),
            Some(provider::MARY_O_CHARACTER_ID)
        );
    }

    /// **The 1-1 grammar, asserted as geometry rather than as a screenshot.** An
    /// open teach run, three WIDENING pits, a stepping stone inside the widest,
    /// a stair pyramid, a goal past it. If a future edit flattens the rhythm this
    /// fails — which is what makes it a level design and not a pile of boxes.
    #[test]
    fn level_1_1_carries_the_grammar_it_claims() {
        let room = level_1_1();
        let world = &room.world;
        let named = |n: &str| world.blocks.iter().find(|b| b.name == n);
        let aabb = |n: &str| named(n).unwrap_or_else(|| panic!("block {n}")).aabb;

        // The spawn sits inside the room, on the open-teach run.
        let s = world.spawn;
        assert!(s.x >= 0.0 && s.x <= world.size.x && s.y >= 0.0 && s.y <= world.size.y);

        // Three pits, WIDENING. A pit is the gap between consecutive ground runs.
        let a = aabb("ground_open_teach");
        let b = aabb("ground_after_pit_a");
        let c = aabb("ground_after_pit_b");
        let d = aabb("ground_after_pit_c");
        let (pit_a, pit_b, pit_c) = (b.min.x - a.max.x, c.min.x - b.max.x, d.min.x - c.max.x);
        assert!(
            pit_a < pit_b && pit_b < pit_c,
            "the pit rhythm must WIDEN — each pit charges more for the last one's \
             lesson: {pit_a} then {pit_b} then {pit_c}"
        );

        // The first platform hangs over SAFE ground: missing it costs nothing.
        let teach = aabb("teach_platform");
        assert!(
            teach.min.x > a.min.x && teach.max.x < a.max.x,
            "the teach platform must hang over the open run, never over a pit"
        );

        // ...and the same jump is load-bearing exactly once, inside the widest pit.
        let stone = aabb("pit_c_stepping_stone");
        assert!(
            stone.min.x > c.max.x && stone.max.x < d.min.x,
            "the stepping stone must be INSIDE the widest pit"
        );

        // Both are one-way: you rise through them and never get stuck under one.
        for name in ["teach_platform", "pit_c_stepping_stone"] {
            assert!(
                matches!(named(name).unwrap().kind, ae::BlockKind::OneWay),
                "{name} must be a one-way — this grammar's platforms admit from below"
            );
        }

        // The pyramid ascends, then descends, and the goal is past it.
        assert!(
            aabb("stair_up_4").max.x < aabb("stair_down_4").min.x,
            "up before down"
        );
        assert!(
            aabb("goal_pole").min.x > aabb("stair_down_1").max.x,
            "the goal is past the pyramid"
        );
    }

    /// The bricks are authored exactly where the break runtime expects them: a
    /// `brick_<i>` block per column whose durable `GeoId` is `brick_id(i)`, solid
    /// until bonked, and never sharing an id with a ?-block. The brick twin of the
    /// ?-block/powerup agreement — the level and the reactive-block runtime can never
    /// drift on which block is a brick or where it is.
    #[test]
    fn level_1_1_authors_the_bricks_the_break_runtime_expects() {
        let room = level_1_1();
        assert_eq!(BRICK_COUNT, BRICK_COLUMNS.len());
        for i in 0..BRICK_COUNT {
            let block = room
                .world
                .blocks
                .iter()
                .find(|b| b.name == brick_name(i))
                .unwrap_or_else(|| panic!("brick {i} is authored into the level"));
            assert_eq!(
                block.id,
                brick_id(i),
                "brick {i}'s GeoId matches the runtime's"
            );
            assert_eq!(
                brick_index_for(&block.id),
                Some(i),
                "the runtime resolves the authored brick back to its index"
            );
            assert!(
                matches!(block.kind, ae::BlockKind::Solid),
                "a brick is solid geometry until a bonk breaks it"
            );
            assert!(
                power_block_index_for(&block.id).is_none(),
                "a brick's id never collides with a ?-block's"
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
        use ambition::world::rooms::{ActiveRoomMetadata, RoomMetadata};

        fn remaining(app: &mut App) -> Option<f32> {
            let mut q = app.world_mut().query::<&MaryOLevelState>();
            q.iter(app.world()).next().map(|s| s.time_remaining)
        }
        fn shell(rules: MaryORulesPlugin, mode: Option<&str>, dt: f32) -> App {
            let mut app = App::new();
            ambition::engine::add_headless_foundation(&mut app);
            ambition::platformer::lifecycle::insert_session_world_component(
                app.world_mut(),
                ActiveRoomMetadata(RoomMetadata {
                    mode: mode.map(str::to_string),
                    ..Default::default()
                }),
            );
            app.insert_resource(ambition::time::WorldTime {
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
        use ambition::engine_core::AabbExt;

        let vault = vault_bounds();
        let arrival = vault_arrival();
        let ground_top = SURFACE_HEIGHT - GROUND_TILES * T;

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
        assert!(
            vault.max.y < LEVEL_HEIGHT,
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
                    under + PIPE_WIDTH_TILES * T // one pipe-width clear
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
        assert_eq!(
            room.placements
                .iter()
                .filter(|p| p.id.as_str().starts_with("vault_coin_"))
                .count(),
            VAULT_COINS,
            "the vault is stocked"
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
        // A ground-slab block (min.y at the slab top) covering world-x `x`.
        let slab_covers = |x: f32| {
            room.world.blocks.iter().any(|b| {
                b.name.starts_with("ground_")
                    && (b.aabb.min.y - ground_top).abs() < 1.0
                    && b.aabb.min.x <= x
                    && b.aabb.max.x >= x
            })
        };
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
        use ambition::world::rooms::{ActiveRoomMetadata, RoomMetadata};

        fn shell(dt: f32) -> App {
            let mut app = App::new();
            ambition::engine::add_headless_foundation(&mut app);
            ambition::platformer::lifecycle::insert_session_world_component(
                app.world_mut(),
                ActiveRoomMetadata(RoomMetadata::default()),
            );
            app.insert_resource(ambition::time::WorldTime {
                scaled_dt: dt,
                ..Default::default()
            });
            app.add_message::<ambition::actors::ActorDiedMessage>();
            app.add_plugins(MaryORulesPlugin::global());
            app.world_mut().spawn((
                ambition::engine_core::BodyLifetime::default(),
                ambition::platformer::markers::PlayerEntity,
                ambition::platformer::markers::PrimaryPlayer,
            ));
            app
        }
        fn kill(app: &mut App) {
            app.world_mut()
                .write_message(ambition::actors::ActorDiedMessage {
                    pos: ambition::engine_core::Vec2::ZERO,
                    cause: ambition::actors::DeathCause {
                        source: ambition::combat::HitSource::Hazard,
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
        use ambition::world::rooms::{ActiveRoomMetadata, RoomMetadata};

        let mut app = App::new();
        ambition::engine::add_headless_foundation(&mut app);
        ambition::platformer::lifecycle::insert_session_world_component(
            app.world_mut(),
            ActiveRoomMetadata(RoomMetadata::default()),
        );
        app.insert_resource(ambition::time::WorldTime {
            scaled_dt: 0.0,
            ..Default::default()
        });
        app.add_message::<ambition::actors::ActorDiedMessage>();
        app.add_plugins(MaryORulesPlugin::global());
        let body = app
            .world_mut()
            .spawn((
                ambition::engine_core::BodyLifetime::default(),
                ambition::platformer::markers::PlayerEntity,
                ambition::platformer::markers::PrimaryPlayer,
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
                .get_mut::<ambition::engine_core::BodyLifetime>(body)
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
        use ambition::world::rooms::{ActiveRoomMetadata, RoomMetadata};

        let mut app = App::new();
        ambition::engine::add_headless_foundation(&mut app);
        ambition::platformer::lifecycle::insert_session_world_component(
            app.world_mut(),
            ActiveRoomMetadata(RoomMetadata::default()),
        );
        // Half the dwell per frame: frame 1 arms nothing, frame 2 crosses it.
        app.insert_resource(ambition::time::WorldTime {
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
