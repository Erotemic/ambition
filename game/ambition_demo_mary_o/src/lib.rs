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

pub mod bricks;
pub mod crony;
pub mod flag;
pub mod movement;
pub mod powerups;
pub mod provider;

pub use provider::{
    mary_o_session_world, MaryOExperiencePlugin, MaryOSessionWorld, MARY_O_CHARACTER_ID,
    MARY_O_EXPERIENCE, MARY_O_GAMEPLAY_ROUTE, MARY_O_LAUNCHER_ROUTE,
};

use ambition::engine_core as ae;
use ambition::prelude::*;
use ambition::world::rooms::RoomSpec;

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
/// after the brick run, past the point where a crony is likely to have cost her
/// the cap, so the fire form is reachable on a normal messy playthrough rather
/// than only on a clean one.
const POWER_BLOCK_COLUMNS: [f32; 3] = [6.0, 30.0, 52.0];
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
const BRICK_COLUMNS: [f32; 3] = [40.0, 41.0, 42.0];
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
/// Flagpole placeholder colours. Flat quads until the pole has real art — the
/// silhouette (pale shaft, bright knob, dark banner) is what makes it read as a
/// goal rather than a bar, and it survives the sprite landing later.
const POLE_COLOR: [f32; 4] = [0.78, 0.82, 0.80, 1.0];
const POLE_KNOB_COLOR: [f32; 4] = [0.96, 0.88, 0.32, 1.0];
const POLE_BANNER_COLOR: [f32; 4] = [0.24, 0.62, 0.34, 1.0];

/// The level's world width and height. Named, rather than inlined into
/// [`level_1_1`], because [`goal_pole`] must derive the flag's geometry from the
/// same numbers the flag's BLOCK is built from — see `flag_geometry_oracle`.
const LEVEL_WIDTH: f32 = 96.0 * T;

/// The SURFACE half's height — every above-ground feature is placed against
/// this, so growing the world downward for the vault below leaves the authored
/// 1-1 layout byte-identical.
const SURFACE_HEIGHT: f32 = 15.0 * T;

/// How far below the ground slab the secret vault's floor sits.
const VAULT_DEPTH_TILES: f32 = 9.0;

const LEVEL_HEIGHT: f32 = SURFACE_HEIGHT + VAULT_DEPTH_TILES * T;

/// The warp pipe: which tile column it stands in, and how big it is.
///
/// Column 26 puts it on the safe run between pit A and pit B — far enough past
/// the open teach that a player has learned the jump, close enough that the
/// first pipe they ever see is not at the end of the level.
const PIPE_COLUMN: f32 = 26.0;
const PIPE_WIDTH_TILES: f32 = 2.0;
const PIPE_HEIGHT_TILES: f32 = 2.0;
const PIPE_NAME: &str = "secret_pipe";
const EXIT_PIPE_NAME: &str = "vault_return_pipe";
const PIPE_COLOR: [f32; 4] = [0.18, 0.62, 0.28, 1.0];
const VAULT_STONE_COLOR: [f32; 4] = [0.24, 0.20, 0.30, 1.0];

/// Coins waiting in the vault. The whole reward for finding the pipe.
const VAULT_COINS: usize = 8;

/// The vault's interior, in world coordinates: a sealed chamber dug under the
/// ground slab, directly below the pipe that leads into it.
///
/// One function so the level geometry, the warp destination, and the coin row
/// can never disagree about where the room actually is.
pub fn vault_bounds() -> ae::Aabb {
    let ceiling = SURFACE_HEIGHT;
    let floor = SURFACE_HEIGHT + (VAULT_DEPTH_TILES - 2.0) * T;
    let left = (PIPE_COLUMN - 1.0) * T;
    let size = ae::Vec2::new(14.0 * T, floor - ceiling);
    let min = ae::Vec2::new(left, ceiling);
    ae::Aabb::new(min + size * 0.5, size * 0.5)
}

/// The mouth of the pipe — the rectangle you must be standing in to warp down.
pub fn pipe_mouth() -> ae::Aabb {
    let ground_top = SURFACE_HEIGHT - GROUND_TILES * T;
    let top = ground_top - PIPE_HEIGHT_TILES * T;
    let size = ae::Vec2::new(PIPE_WIDTH_TILES * T, T);
    let min = ae::Vec2::new(PIPE_COLUMN * T, top - 0.5 * T);
    ae::Aabb::new(min + size * 0.5, size * 0.5)
}

/// Where the pipe drops you: just inside the vault, under its entrance.
pub fn vault_arrival() -> ae::Vec2 {
    let vault = vault_bounds();
    ae::Vec2::new(vault.min.x + 1.5 * T, vault.min.y + 1.5 * T)
}

/// Where leaving the vault puts you: back on top of the pipe you came down.
pub fn pipe_arrival() -> ae::Vec2 {
    let ground_top = SURFACE_HEIGHT - GROUND_TILES * T;
    ae::Vec2::new(
        (PIPE_COLUMN + PIPE_WIDTH_TILES * 0.5) * T,
        ground_top - PIPE_HEIGHT_TILES * T - T,
    )
}

/// The return pipe's mouth: stand on it at the vault's far end and press
/// Interact to surface.
///
/// Sized and positioned like [`pipe_mouth`] — a band across the top of the pipe
/// you are standing on — rather than a loose 2x2 block of air, so entering and
/// leaving read the same way to the player.
pub fn vault_exit() -> ae::Aabb {
    let vault = vault_bounds();
    let pipe_top = vault.max.y - PIPE_HEIGHT_TILES * T;
    let left = vault.max.x - PIPE_WIDTH_TILES * T;
    let size = ae::Vec2::new(PIPE_WIDTH_TILES * T, T);
    let min = ae::Vec2::new(left, pipe_top - 0.5 * T);
    ae::Aabb::new(min + size * 0.5, size * 0.5)
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
    let width = LEVEL_WIDTH; // 96 tiles — a real 1-1 is ~210; this is its grammar.
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

    // 1 + 3. Open teach, then the widening pit rhythm.
    ground(&mut blocks, "ground_open_teach", 0, 0.0, 20.0);
    ground(&mut blocks, "ground_after_pit_a", 1, 22.0, 34.0); // 2-tile pit at [20,22)
    ground(&mut blocks, "ground_after_pit_b", 2, 37.0, 52.0); // 3-tile pit at [34,37)
    ground(&mut blocks, "ground_after_pit_c", 3, 57.0, 96.0); // 5-tile pit at [52,57)

    // 2. The first platform: over SAFE ground, at jump height. Tiled, like the
    // ground it teaches you to leave.
    blocks.push(ae::Block::one_way_tiled(
        "teach_platform",
        ae::Vec2::new(12.0 * T, ground_top - 4.0 * T),
        ae::Vec2::new(3.0 * T, 0.5 * T),
        "mary_o_platform",
        0,
    ));

    // 3. The widest pit's stepping stone: the same jump, now load-bearing.
    blocks.push(ae::Block::one_way_tiled(
        "pit_c_stepping_stone",
        ae::Vec2::new(54.0 * T, ground_top - 3.0 * T),
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

    // 4. The stair pyramid: four up at x=66.., a gap, four down ending at x=75.
    for step in 1..=4u16 {
        let h = step as f32;
        blocks.push(ae::Block::solid_tiled(
            format!("stair_up_{step}"),
            ae::Vec2::new((65.0 + h) * T, ground_top - h * T),
            ae::Vec2::new(T, h * T),
            "mary_o_stairs",
            step,
        ));
        blocks.push(ae::Block::solid_tiled(
            format!("stair_down_{step}"),
            ae::Vec2::new((76.0 - h) * T, ground_top - (5.0 - h) * T),
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
    // ART: flat colours, authored on the blocks. It used to claim to be "a flat
    // colored quad" while actually taking the shared one-way PLATFORM texture,
    // stretched down a 16x288 column into a smear — the shared block art assumes a
    // footprint roughly matching its texture's aspect, which a pole violates
    // badly. Declaring a placeholder colour is content saying "this shape has no
    // sprite yet", and it stays a flat quad until one exists.
    //
    // Three pieces so the goal READS as a flagpole instead of a bar: a pale shaft,
    // a bright knob capping it, and a banner hanging off the top. All three are the
    // SAME width and column as the pole, so none of them changes what is reachable
    // or where the grab band is — the silhouette is new, the level is not.
    let pole_x = 90.0 * T;
    let pole_top = ground_top - 9.0 * T;
    blocks.push(
        ae::Block::one_way(
            "goal_pole",
            ae::Vec2::new(pole_x, pole_top),
            ae::Vec2::new(POLE_WIDTH, 9.0 * T),
        )
        .with_art_color(POLE_COLOR),
    );
    blocks.push(
        ae::Block::one_way(
            "goal_pole_knob",
            ae::Vec2::new(pole_x, pole_top - POLE_WIDTH),
            ae::Vec2::splat(POLE_WIDTH),
        )
        .with_art_color(POLE_KNOB_COLOR),
    );
    blocks.push(
        ae::Block::one_way(
            "goal_pole_banner",
            ae::Vec2::new(pole_x, pole_top + POLE_WIDTH),
            ae::Vec2::new(POLE_WIDTH, POLE_WIDTH * 2.0),
        )
        .with_art_color(POLE_BANNER_COLOR),
    );

    // ── 6. The secret pipe, and the vault under the level ───────────────────
    //
    // A warp pipe standing on safe ground between pit A and pit B. Stand on its
    // mouth and press Interact and you drop into a sealed coin vault built into
    // the SAME room, below the ground slab — which is why the world grew
    // downward rather than a second room being authored: cross-room transition
    // lives in `ambition_app`'s `world_flow`, so a demo that ships its own app
    // could not use it and would have worked only when Ambition hosted it.
    //
    // The vault is reachable ONLY through the pipe: it is walled on all four
    // sides, and the ground slab above is its ceiling.
    blocks.push(
        ae::Block::solid_tiled(
            PIPE_NAME,
            ae::Vec2::new(PIPE_COLUMN * T, ground_top - PIPE_HEIGHT_TILES * T),
            ae::Vec2::new(PIPE_WIDTH_TILES * T, PIPE_HEIGHT_TILES * T),
            "mary_o_pipe",
            0,
        )
        .with_art_color(PIPE_COLOR),
    );

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

    // The RETURN pipe. The vault's exit was a logical zone with no geometry —
    // nothing to see and nothing to aim at, so the only way out was knowing it
    // was there. A pipe you can see is the whole affordance.
    //
    // It STANDS ON THE VAULT FLOOR, and `vault_exit` is the band straddling its
    // top face — exactly the relationship the entry pipe has with `pipe_mouth`.
    // This used to derive the block's top from the BAND (`exit.max.y - height`),
    // which floated it 48px clear of the floor and left its top face ABOVE its
    // own band: a body standing on the pipe spanned 544..592 against a band at
    // 624..656, so Interact could never fire and the vault had no working exit.
    // The only way out was the hole pit B punches in its ceiling.
    let exit = vault_exit();
    blocks.push(
        ae::Block::solid_tiled(
            EXIT_PIPE_NAME,
            ae::Vec2::new(exit.min.x, vault.max.y - PIPE_HEIGHT_TILES * T),
            ae::Vec2::new(PIPE_WIDTH_TILES * T, PIPE_HEIGHT_TILES * T),
            "mary_o_pipe",
            1,
        )
        .with_art_color(PIPE_COLOR),
    );

    let spawn = ae::Vec2::new(2.0 * T, ground_top - 2.0 * T);
    let world = ae::World::new("Mary-O 1-1", ae::Vec2::new(width, height), spawn, blocks);

    let mut room = RoomSpec::new(LEVEL_1_1_ROOM_ID, world);
    room.metadata.mode = Some(MARY_O_MODE.to_string());
    // The vault's reward, on the ordinary placements channel: `currency`
    // pickups the SHARED economy collects and credits to the body wallet. No
    // demo collection code, and they land in the HUD's COINS readout for free —
    // the same path Sanic's rings already take.
    let vault = vault_bounds();
    let coin_y = vault.max.y - 1.5 * T;
    for i in 0..VAULT_COINS {
        let x = vault.min.x + (1.0 + i as f32 * 1.5) * T;
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
        x: 90.0 * T + POLE_WIDTH * 0.5,
        top_y: ground_top - 9.0 * T,
        base_y: ground_top,
        half_width: POLE_WIDTH * 0.5,
    }
}

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
            // A PROPER Mary-O moveset, composed grant-by-grant: the run+jump
            // floor, wall mobility (cling + kick), and a fast fall (the
            // ground-pound dive). Each is a single-verb grant appended to the list
            // — NOT a preset the roster forks — and the union of them is her
            // AbilityBase. Deliberately WITHOUT `AirJump`: hers is the classic
            // one-press arc, so the only way to clear a gap is to commit to the
            // jump from the ground. It also keeps her OFF the full Ambition kit
            // (blink, dash, fly, fireball) without touching the shared session
            // ability set, so the multi-game host's own protagonist is unaffected;
            // the session mask can gate these verbs off but can never clobber the
            // base back up to sandbox_all. Paired with the `peaceful` authored kit
            // (Authored, not HostCode) so she carries no combat verbs either.
            abilities: Some([RunJump, WallMobility, FastFall]),
            // That one jump is a BIG one: 2.1x the default apex, which is what a
            // ?-block at bonk height and a three-tile pit are drawn around. Apex is
            // v²/(2·gravity), so 2.1x the height is √2.1 ≈ 1.449x the launch speed
            // — 630 * √2.1 = 913. Gravity is untouched, so she falls on the shared
            // curve and simply hangs √2.1x longer on the way up. It rides an
            // AuthoredMovementTuning marker so it comes from HER row, never the
            // shared F3 dev tuning (which every other body still follows) — the
            // axis-path analogue of Sanic's authored `momentum`.
            // Her GAIT, alongside her arc. `max_run_speed` 320 is her hold-to-run
            // top speed; the walk is half of it (the demo's WALK_THROTTLE), which
            // is the classic two-gear feel. `run_accel` 900 is the important one:
            // the shared default (5200) reaches top speed in ~0.06s, which reads
            // as a velocity snap. At 900 she takes ~0.35s to wind up to a run, and
            // a reversal at speed spends ~0.7s sliding through zero — the skid
            // that says she has weight. Both ride the same AuthoredMovementTuning
            // marker her jump does, so they come from HER row and leave every
            // other body on the shared F3 dev tuning.
            axis_tuning: Some((jump_speed: 913.0, max_run_speed: 320.0, run_accel: 900.0)),
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
        // grant list, same tall-jump `axis_tuning` (re-wearing re-reads
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
            abilities: Some([RunJump, WallMobility, FastFall]),
            // Her GAIT, alongside her arc. `max_run_speed` 320 is her hold-to-run
            // top speed; the walk is half of it (the demo's WALK_THROTTLE), which
            // is the classic two-gear feel. `run_accel` 900 is the important one:
            // the shared default (5200) reaches top speed in ~0.06s, which reads
            // as a velocity snap. At 900 she takes ~0.35s to wind up to a run, and
            // a reversal at speed spends ~0.7s sliding through zero — the skid
            // that says she has weight. Both ride the same AuthoredMovementTuning
            // marker her jump does, so they come from HER row and leave every
            // other body on the shared F3 dev tuning.
            axis_tuning: Some((jump_speed: 913.0, max_run_speed: 320.0, run_accel: 900.0)),
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
            abilities: Some([RunJump, WallMobility, FastFall]),
            axis_tuning: Some((jump_speed: 913.0, max_run_speed: 320.0, run_accel: 900.0)),
            playable_kit: Authored,
            tags: ["player"],
            barks: (
                hall: ["One blossom, and every ceiling gets a warm answer.", "I throw solutions now — mind the sparks.", "Fireproof opinions, freshly lit."],
            ),
        ),
        // The crony's IDENTITY row: its sprite resolves from this display name.
        // It points its OWN name at the published `ai_slop` sheet (Ambition owns
        // the "Ai Slop" display name; a duplicate would fail catalog assembly when
        // hosted). Behavior/HP/contact come from the `mary_o_crony` ROSTER
        // archetype (see `crony.rs`), not this catalog row — this is only the
        // sprite + name.
        "mary_o_crony": (
            display_name: "Mary-O Crony",
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
    // The crony's hostile archetype and room stager are authored content, so
    // install both before direct or shell preparation fingerprints the App.
    crony::register_crony_roster(app);
    app.init_resource::<ambition::actors::features::RoomContentStagingRegistry>();
    crony::register_crony_content_staging(
        &mut app
            .world_mut()
            .resource_mut::<ambition::actors::features::RoomContentStagingRegistry>(),
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
        // The crony stager reads room-load facts and writes spawn requests; the
        // engine registers both in a full app, but a thin rules-only test harness
        // may not, and `add_message` is idempotent.
        app.add_message::<ambition::actors::rooms::RoomLoaded>();
        app.add_message::<ambition::actors::features::SpawnActorRequest>();
        // The crony squash pops a dust burst through the engine's vfx seam; a full
        // app registers this via the presentation plugins, but a thin rules-only
        // harness may not, and `add_message` is idempotent.
        app.add_message::<ambition::vfx::VfxMessage>();
        // Same story for the cue queue: the brick-break voices through the shared
        // sfx seam, a full app registers this via the audio plugins, and a thin
        // rules-only harness may not. `add_message` is idempotent.
        app.add_message::<ambition::sfx::OwnedSfxMessage>();
        // The flag runs BEFORE the clock: a level whose flag has been grabbed is
        // over, and `tick_level_clock` reads the sequence to know it. The cycle
        // emitter runs LAST so it sees the settled tally and its clock reset is not
        // immediately decremented on the same frame.
        let rules = (
            spawn_mary_o_mode_owner,
            flag::run_flag_sequence,
            tick_level_clock,
            // Reads the clock the tick above just settled, so a timeout is spent
            // on the frame it happens rather than one late.
            spend_lives_on_death,
            warp_through_secret_pipe,
            cycle_level_on_flag_tally,
        )
            .chain();
        // The walkers are registered by `install_mary_o_content`, the single
        // authored-content composition seam shared by direct and shell hosts.
        // Rules consume the staged actors; they do not mutate construction
        // registries after prepared-content fingerprinting.
        // The head-stomp runs BEFORE the engine's shared body-contact-damage
        // pass so a squash never also hurts the stomper (the rule zeroes the
        // crony's health that frame, which the contact pass then skips).
        let cronies = (
            crony::bounce_squash_cronies
                .before(ambition::actors::features::apply_actor_contact_damage),
            // The shell trio, after the stomp that creates one: tag what the
            // engine spawned, let the player kick it, then drive it.
            crony::tag_mary_o_shells,
            crony::kick_mary_o_shells,
            crony::drive_mary_o_shells,
        )
            .chain();
        // The powerup rules on the two engine primitives: re-arm the ?-blocks on
        // (re)load, pop milk on a head-bonk, and keep the tall form in sync with
        // wearing the cap. The engine's `collect_world_items` (touch → equip) sits
        // between the bonk and the grow — no demo wiring for it.
        let powerups = (
            powerups::refill_power_blocks_on_room_loaded,
            powerups::bonk_power_blocks,
            powerups::sync_grown_form,
            powerups::tag_mary_o_sparks,
        );
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
            .after(ambition::actors::avatar::tick_player_brains);
        // The bricks — the reactive-block primitive's SECOND consumer: re-arm on
        // (re)load, break the bonked one, and contribute broken bricks to the
        // collision overlay's `removed_block_names` so they stop colliding (and, via
        // the render reconcile, drawing). The contribution runs AFTER the engine's
        // overlay rebuild clears that list — the same slot `contribute_encounter_lock_walls`
        // takes — so the removals survive the per-frame clean slate.
        let bricks = (bricks::refill_bricks_on_room_loaded, bricks::break_bricks);
        let brick_overlay = bricks::contribute_broken_bricks_to_overlay
            .after(ambition::actors::features::rebuild_feature_ecs_world_overlay);
        if self.hosted {
            app.add_systems(sim, rules.run_if(ambition::runtime::in_mode(MARY_O_MODE)));
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

/// **The secret pipe.** Stand on its mouth, press Interact, drop into the vault;
/// stand at the vault's far end, press Interact, surface again.
///
/// The warp is a real TRANSIT, not a position poke: `transit_body` is the engine
/// authority for discretely relocating a body (ADR 0024), and it reconciles the
/// motion model's private attachment and maneuver state on the way. Without that
/// a player who entered the pipe while wall-clinging would arrive in the vault
/// still clinging to a wall that is no longer there.
///
/// The pipe is entered DIRECTIONALLY (Jon bug list #8): press DOWN standing on
/// the entry mouth to drop in, press UP standing on the return mouth to surface —
/// the classic warp-pipe verb. That does NOT break the "a single Up/Down must not
/// trigger a door" rule: a pipe is not a door, and the press has to point INTO
/// the pipe while you stand on its mouth, which reads as deliberate, not
/// incidental. It also removes the ping-pong for free — the two ends need
/// OPPOSITE directions, so a held press that warped you down can never fire the
/// up-return at the far end.
fn warp_through_secret_pipe(
    // Rising-edge latch on the directional trigger, so a HELD press warps exactly
    // once rather than re-firing every frame it stays down.
    mut was_pressed: bevy::prelude::Local<bool>,
    mut bodies: bevy::prelude::Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition::actors::features::MotionModel,
            &ambition::characters::brain::ActorControl,
        ),
        ambition::platformer::markers::PrimaryPlayerOnly,
    >,
) {
    // Body-local locomotion, `+y` toward the feet (screen-down under Mary-O's
    // normal gravity): press toward the ground to go DOWN a pipe, away to go UP.
    const DIR_DEADZONE: f32 = 0.5;
    let mut any_trigger = false;
    for (clusters, mut model, control) in &mut bodies {
        let down = control.0.locomotion.y > DIR_DEADZONE;
        let up = control.0.locomotion.y < -DIR_DEADZONE;
        let mut item = clusters;
        let mut clusters = item.as_clusters_mut();
        let body = ae::Aabb::new(clusters.kinematics.pos, clusters.kinematics.size * 0.5);

        // Each mouth answers only its own direction: DOWN at the entry pipe, UP
        // at the return pipe.
        let destination = warp_destination(
            down,
            up,
            overlaps(body, pipe_mouth()),
            overlaps(body, vault_exit()),
        );
        any_trigger |= destination.is_some();
        let Some(destination) = destination else {
            continue;
        };
        if *was_pressed {
            continue;
        }

        ambition::engine_core::movement::transit_body(
            &mut model,
            &mut clusters,
            destination,
            ambition::engine_core::movement::TransitVelocity::Zero,
        );
    }
    *was_pressed = any_trigger;
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

/// Plain AABB overlap. `IntersectsVolume` would do, but this keeps the warp rule
/// readable at the call site and free of a trait import for one comparison.
fn overlaps(a: ae::Aabb, b: ae::Aabb) -> bool {
    a.min.x < b.max.x && a.max.x > b.min.x && a.min.y < b.max.y && a.max.y > b.min.y
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
        // Mary-O's authored grant list composes to her platformer moveset —
        // run+jump, wall mobility, fast fall — and NOTHING from the full Ambition
        // kit (blink/dash/fly/attack). This is her AbilityBase; the session mask
        // can only narrow it, never restore the sandbox kit.
        let mary_o_kit = catalog
            .ability_set(provider::MARY_O_CHARACTER_ID)
            .expect("Mary-O authors a grant list");
        assert_eq!(
            mary_o_kit,
            ambition::engine_core::AbilitySet::compose(&[
                ambition::engine_core::AbilityGrant::RunJump,
                ambition::engine_core::AbilityGrant::WallMobility,
                ambition::engine_core::AbilityGrant::FastFall,
            ]),
            "Mary-O composes to the classic platformer moveset"
        );
        assert!(
            mary_o_kit.jump
                && mary_o_kit.move_horizontal
                && mary_o_kit.wall_jump
                && mary_o_kit.wall_cling
                && mary_o_kit.fast_fall,
            "the platformer verbs are all lit"
        );
        assert!(
            !mary_o_kit.double_jump,
            "but NOT the air jump: hers is a single committed arc, so no AirJump \
             grant means air_jump_count is 0 no matter what any tuning says"
        );
        assert!(
            !mary_o_kit.blink
                && !mary_o_kit.dash
                && !mary_o_kit.fly
                && !mary_o_kit.attack
                && !mary_o_kit.wall_climb,
            "but none of the full Ambition kit"
        );
        // That single jump is 2.1x the default apex. She authors it as a
        // per-character axis tuning riding an AuthoredMovementTuning marker, so the
        // launch speed comes from HER row rather than the shared F3 dev tuning.
        // This is the axis-path analogue of `momentum`.
        let mary_o_tuning = catalog
            .axis_tuning(provider::MARY_O_CHARACTER_ID)
            .expect("Mary-O authors an axis tuning");
        let default_tuning = ambition::engine_core::DEFAULT_TUNING;
        // Apex = v²/(2·gravity), and she shares the default gravity, so the height
        // ratio is exactly the SPEED ratio squared. Assert the ratio, not the
        // magic number: retuning the shared jump keeps her 2.1x relationship.
        let height_ratio = (mary_o_tuning.jump_speed / default_tuning.jump_speed).powi(2);
        assert!(
            (height_ratio - 2.1).abs() < 0.005,
            "Mary-O jumps ~2.1x the default height, got {height_ratio}x"
        );
        // Everything else in her feel stays at the shared default — she overrides
        // only what she authors (the gravity Jon blessed is untouched, so she falls
        // on the same curve every other body does).
        assert_eq!(
            mary_o_tuning.gravity, default_tuning.gravity,
            "an un-authored knob stays at the default feel"
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

        // Both warp ends catch a player-sized body standing at them. A mouth
        // that does not overlap is a pipe that cannot be entered.
        let body_at = |p: ae::Vec2| ae::Aabb::new(p, ae::Vec2::new(0.5 * T, 0.9 * T));
        assert!(
            overlaps(body_at(pipe_arrival()), pipe_mouth()),
            "standing on the pipe overlaps its mouth, or the directional press can never fire"
        );
        // Deliberately NOT `body_at(vault_exit().center())`. That point is
        // inside the return pipe's own SOLID geometry, so it asserted a
        // position no player can occupy — which is why it stayed green while
        // the vault had no working exit at all. Stand her on the pipe's top
        // face, where a player actually ends up, and check from there.
        // Read the top face off the AUTHORED block, never recompute it from the
        // formula it was supposed to use — recomputing tests the intent and
        // passes no matter where the block actually ended up.
        let return_pipe_top = level_1_1()
            .world
            .blocks
            .iter()
            .find(|b| b.name == EXIT_PIPE_NAME)
            .expect("the vault has a visible return pipe")
            .aabb
            .min
            .y;
        let standing_on_return_pipe =
            ae::Vec2::new(vault_exit().center().x, return_pipe_top - 0.9 * T);
        assert!(
            overlaps(body_at(standing_on_return_pipe), vault_exit()),
            "a body STANDING ON the return pipe must overlap the exit band, or \
             Interact can never fire and the vault is a one-way trip: body at \
             {standing_on_return_pipe:?} vs band {:?}",
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
            .expect("the vault has a VISIBLE return pipe, not just an exit zone");
        assert!(
            overlaps(return_pipe.aabb, vault_exit()),
            "and the exit zone sits on that pipe's mouth, so what you press \
             Interact on is the thing you can see"
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
