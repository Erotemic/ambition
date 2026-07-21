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
const POWER_BLOCK_COLUMNS: [f32; 2] = [6.0, 30.0];
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

/// The level's world width and height. Named, rather than inlined into
/// [`level_1_1`], because [`goal_pole`] must derive the flag's geometry from the
/// same numbers the flag's BLOCK is built from — see `flag_geometry_oracle`.
const LEVEL_WIDTH: f32 = 96.0 * T;
const LEVEL_HEIGHT: f32 = 15.0 * T;

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
    let ground_top = height - GROUND_TILES * T;

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

    // 5. The goal pole.
    blocks.push(ae::Block::solid(
        "goal_pole",
        ae::Vec2::new(90.0 * T, ground_top - 9.0 * T),
        ae::Vec2::new(T * 0.5, 9.0 * T),
    ));

    let spawn = ae::Vec2::new(2.0 * T, ground_top - 2.0 * T);
    let world = ae::World::new("Mary-O 1-1", ae::Vec2::new(width, height), spawn, blocks);

    let mut room = RoomSpec::new(LEVEL_1_1_ROOM_ID, world);
    room.metadata.mode = Some(MARY_O_MODE.to_string());
    room
}

/// The min corner of ?-block `i`, from the SAME constants [`level_1_1`] builds the
/// `power_block_*` blocks out of — so the powerup runtime pops the milk out at the
/// exact block it was authored at.
pub fn power_block_min(i: usize) -> ae::Vec2 {
    let ground_top = LEVEL_HEIGHT - GROUND_TILES * T;
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
    let ground_top = LEVEL_HEIGHT - GROUND_TILES * T;
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
    let ground_top = LEVEL_HEIGHT - GROUND_TILES * T;
    flag::FlagPole {
        // `Block::solid` takes a MIN corner; the pole is `T * 0.5` wide.
        x: 90.0 * T + T * 0.25,
        top_y: ground_top - 9.0 * T,
        base_y: ground_top,
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
            axis_tuning: Some((jump_speed: 913.0)),
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
            axis_tuning: Some((jump_speed: 913.0)),
            playable_kit: Authored,
            tags: ["player"],
            barks: (
                hall: ["One power-up; every ceiling gets an opinion.", "Ask the doorframes whether taller is better.", "I shrink after one professional-grade mistake."],
            ),
            hall_dialogue_id: Some("hall_mary_o_tall"),
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
#[derive(bevy::prelude::Component, Debug)]
pub struct MaryOLevelState {
    /// Counts DOWN from [`STARTING_TIME`]; clamps at zero.
    pub time_remaining: f32,
}

impl Default for MaryOLevelState {
    fn default() -> Self {
        Self {
            time_remaining: STARTING_TIME,
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
        // The crony stager reads room-load facts and writes spawn requests; the
        // engine registers both in a full app, but a thin rules-only test harness
        // may not, and `add_message` is idempotent.
        app.add_message::<ambition::actors::rooms::RoomLoaded>();
        app.add_message::<ambition::actors::features::SpawnActorRequest>();
        // The crony squash pops a dust burst through the engine's vfx seam; a full
        // app registers this via the presentation plugins, but a thin rules-only
        // harness may not, and `add_message` is idempotent.
        app.add_message::<ambition::vfx::VfxMessage>();
        // The flag runs BEFORE the clock: a level whose flag has been grabbed is
        // over, and `tick_level_clock` reads the sequence to know it. The cycle
        // emitter runs LAST so it sees the settled tally and its clock reset is not
        // immediately decremented on the same frame.
        let rules = (
            spawn_mary_o_mode_owner,
            flag::run_flag_sequence,
            tick_level_clock,
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
        let cronies = crony::bounce_squash_cronies
            .before(ambition::actors::features::apply_actor_contact_damage);
        // The powerup rules on the two engine primitives: re-arm the ?-blocks on
        // (re)load, pop milk on a head-bonk, and keep the tall form in sync with
        // wearing the cap. The engine's `collect_world_items` (touch → equip) sits
        // between the bonk and the grow — no demo wiring for it.
        let powerups = (
            powerups::refill_power_blocks_on_room_loaded,
            powerups::bonk_power_blocks,
            powerups::sync_grown_form,
        );
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
            app.add_systems(
                sim,
                brick_overlay.run_if(ambition::runtime::in_mode(MARY_O_MODE)),
            );
        } else {
            app.add_systems(sim, rules);
            app.add_systems(sim, cronies);
            app.add_systems(sim, powerups);
            app.add_systems(sim, bricks);
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
        // A level whose flag has been grabbed is over. The clock stopping is what
        // turns the remaining time from a threat into a score.
        if sequence.active() {
            continue;
        }
        state.time_remaining = (state.time_remaining - time.scaled_dt).max(0.0);
    }
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
    *sequence = flag::FlagSequence::default();
    level.time_remaining = STARTING_TIME;
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
        add_demo_content(&mut app);
        let catalog = app
            .world()
            .resource::<ambition::characters::actor::character_catalog::CharacterCatalog>();
        assert!(catalog.get(provider::MARY_O_CHARACTER_ID).is_some());
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
