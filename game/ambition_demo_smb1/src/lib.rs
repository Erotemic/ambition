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

pub mod flag;
pub mod goomba;
pub mod powerups;
pub mod provider;

pub use provider::{
    smb1_session_world, Smb1ExperiencePlugin, Smb1SessionWorld, MARY_O_CHARACTER_ID,
    MARY_O_EXPERIENCE, MARY_O_GAMEPLAY_ROUTE, MARY_O_LAUNCHER_ROUTE,
};

use ambition::engine_core as ae;
use ambition::prelude::*;
use ambition::world::rooms::RoomSpec;

/// Stable room id for level 1-1.
pub const LEVEL_1_1_ROOM_ID: &str = "mary_o_1_1";

/// The game-MODE tag this demo's rooms carry (decomposition D-C).
///
/// Ambition can host this demo alongside its own rooms; [`Smb1RulesPlugin`] gates
/// its systems on `ambition::runtime::in_mode(SMB1_MODE)` so the level clock never
/// ticks in a room that is not Mary-O's.
pub const SMB1_MODE: &str = "mary_o";

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
    room.metadata.mode = Some(SMB1_MODE.to_string());
    room
}

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
const SMB1_CATALOG_RON: &str = r#"(
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
            // floor, an air jump (double jump — a triple jump once her authored
            // `air_jumps` count below raises it), wall mobility (cling + kick), and
            // a fast fall (the ground-pound dive). Each is a single-verb grant
            // appended to the list — NOT a preset the roster forks — and the union
            // of them is her AbilityBase. It still keeps her OFF the full Ambition
            // kit (blink, dash, fly, fireball) without touching the shared session
            // ability set, so the multi-game host's own protagonist is unaffected;
            // the session mask can gate these verbs off but can never clobber the
            // base back up to sandbox_all. Paired with the `peaceful` authored kit
            // (Authored, not HostCode) so she carries no combat verbs either.
            abilities: Some([RunJump, AirJump, WallMobility, FastFall]),
            // Her AirJump grant is a TRIPLE jump: the grant lights the air-jump
            // capability, this authored count (2 air jumps) is the feel. It rides
            // an AuthoredMovementTuning marker so it comes from HER row, never the
            // shared F3 dev tuning (which defaults air_jumps to 1) — the axis-path
            // analogue of Sanic's authored `momentum`.
            axis_tuning: Some((air_jumps: 2)),
            playable_kit: Authored,
            tags: ["player"],
        ),
        // The goomba's IDENTITY row: its sprite resolves from this display name.
        // It points its OWN name at the published `ai_slop` sheet (Ambition owns
        // the "Ai Slop" display name; a duplicate would fail catalog assembly when
        // hosted). Behavior/HP/contact come from the `mary_o_goomba` ROSTER
        // archetype (see `goomba.rs`), not this catalog row — this is only the
        // sprite + name.
        "mary_o_goomba": (
            display_name: "Mary-O Goomba",
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
pub struct Smb1DemoContentPlugin;

/// Register Mary-O's immutable authored character fragment in one Bevy `App`.
/// Shared by the historical [`Smb1DemoContentPlugin`] (Startup construction) and
/// the new [`provider::Smb1ExperiencePlugin`] (shell-activation construction).
pub fn install_smb1_content(app: &mut App) {
    use ambition::characters::actor::character_catalog::{
        CharacterCatalogAppExt, CharacterCatalogFragment,
    };

    app.register_character_catalog_fragment(
        CharacterCatalogFragment::from_ron(
            provider::MARY_O_EXPERIENCE,
            Some(provider::MARY_O_CHARACTER_ID),
            SMB1_CATALOG_RON,
        )
        .expect("Mary-O character catalog should be valid"),
    );
    // The goomba's hostile archetype (body/walk/contact) lives in a roster
    // fragment beside the catalog identity above.
    goomba::register_goomba_roster(app);
}

impl Plugin for Smb1DemoContentPlugin {
    fn build(&self, app: &mut App) {
        use ambition::runtime::demo_fixture::{ActiveRoomMetadata, RoomSet};
        use bevy::prelude::IntoScheduleConfigs;

        install_smb1_content(app);
        let room = level_1_1();
        app.world_mut().spawn((
            ambition::platformer::lifecycle::SessionRoot(
                ambition::platformer::lifecycle::SessionScopeId(0),
            ),
            ambition::runtime::PlatformerSessionWorld::new(
                provider::MARY_O_EXPERIENCE,
                RoomSet::from_parts(LEVEL_1_1_ROOM_ID, vec![room.clone()], Vec::new()),
                ae::RoomGeometry(room.world.clone()),
                ActiveRoomMetadata(room.metadata.clone()),
                ambition::runtime::demo_fixture::StartingCharacter::new(
                    provider::MARY_O_CHARACTER_ID,
                ),
                ambition::runtime::demo_fixture::LdtkRuntimeIndex::default(),
            ),
        ));
        app.add_systems(
            bevy::app::Startup,
            smb1_setup.in_set(ambition::runtime::demo_fixture::SimulationSetupSet),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn smb1_setup(
    mut commands: bevy::prelude::Commands,
    world: ambition::platformer::lifecycle::SessionWorldRef<ae::RoomGeometry>,
    room_set: ambition::platformer::lifecycle::SessionWorldRef<
        ambition::runtime::demo_fixture::RoomSet,
    >,
    ldtk_index: ambition::platformer::lifecycle::SessionWorldRef<
        ambition::runtime::demo_fixture::LdtkRuntimeIndex,
    >,
    editable_abilities: bevy::prelude::Res<ambition::runtime::demo_fixture::EditableAbilitySet>,
    editable_tuning: bevy::prelude::Res<ambition::runtime::demo_fixture::EditableMovementTuning>,
    starting_character: ambition::platformer::lifecycle::SessionWorldRef<
        ambition::runtime::demo_fixture::StartingCharacter,
    >,
    asset_server: bevy::prelude::Res<bevy::asset::AssetServer>,
    character_catalog: bevy::prelude::Res<
        ambition::characters::actor::character_catalog::CharacterCatalog,
    >,
    character_roster: bevy::prelude::Res<ambition::actors::features::CharacterRoster>,
    boss_catalog: bevy::prelude::Res<ambition::actors::boss_encounter::BossCatalog>,
) {
    ambition::runtime::demo_fixture::simulation_world(
        &mut commands,
        ambition::platformer::lifecycle::SessionSpawnScope::UNSCOPED,
        ambition::runtime::demo_fixture::SimulationSetup {
            world: &world,
            room_set: &room_set,
            ldtk_index: &ldtk_index,
            editable_abilities: &editable_abilities,
            editable_tuning: &editable_tuning,
            starting_character: &starting_character,
            character_catalog: &character_catalog,
            character_roster: &character_roster,
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
pub struct Smb1LevelState {
    /// Counts DOWN from [`STARTING_TIME`]; clamps at zero.
    pub time_remaining: f32,
}

impl Default for Smb1LevelState {
    fn default() -> Self {
        Self {
            time_remaining: STARTING_TIME,
        }
    }
}

/// Mary-O's level rules. ONE system list; a constructor flag decides its gating —
/// [`Smb1RulesPlugin::hosted`] when Ambition hosts the demo alongside its own
/// rooms, [`Smb1RulesPlugin::global`] when the demo IS the game.
///
/// That two demos with nothing else in common share this exact shape is the D-C
/// pattern's whole point: a mode is a ROOM property, not a latch some plugin owns.
pub struct Smb1RulesPlugin {
    hosted: bool,
}

impl Smb1RulesPlugin {
    /// Ambition hosts this demo: every rule sleeps outside Mary-O's rooms.
    pub fn hosted() -> Self {
        Self { hosted: true }
    }

    /// The demo IS the game: the rules run unconditionally.
    pub fn global() -> Self {
        Self { hosted: false }
    }
}

impl Plugin for Smb1RulesPlugin {
    fn build(&self, app: &mut App) {
        use bevy::prelude::IntoScheduleConfigs;
        let sim = ambition::platformer::schedule::SimScheduleExt::sim_schedule(app);
        app.insert_resource(goal_pole());
        // The cycle emitter writes this; the host's replay consumer drains it. The
        // engine registers it too (`SandboxResetSchedulePlugin`), but a thin host
        // may not, and `add_message` is idempotent — a no-op when already present.
        app.add_message::<ambition::actors::session::reset::RoomReplayRequested>();
        // The goomba stager reads room-load facts and writes spawn requests; the
        // engine registers both in a full app, but a thin rules-only test harness
        // may not, and `add_message` is idempotent.
        app.add_message::<ambition::actors::rooms::RoomLoaded>();
        app.add_message::<ambition::actors::features::SpawnActorRequest>();
        // The flag runs BEFORE the clock: a level whose flag has been grabbed is
        // over, and `tick_level_clock` reads the sequence to know it. The cycle
        // emitter runs LAST so it sees the settled tally and its clock reset is not
        // immediately decremented on the same frame.
        let rules = (
            spawn_smb1_mode_owner,
            flag::run_flag_sequence,
            tick_level_clock,
            cycle_level_on_flag_tally,
        )
            .chain();
        // The goomba systems: stage the walkers when the room loads, and run the
        // head-stomp BEFORE the engine's shared body-contact-damage pass so a
        // squash never also hurts the stomper (the rule zeroes the goomba's health
        // that frame, which the contact pass then skips).
        let goombas = (
            goomba::stage_goombas_on_room_loaded,
            goomba::bounce_squash_goombas
                .before(ambition::actors::features::apply_actor_contact_damage),
        );
        if self.hosted {
            app.add_systems(sim, rules.run_if(ambition::runtime::in_mode(SMB1_MODE)));
            app.add_systems(sim, goombas.run_if(ambition::runtime::in_mode(SMB1_MODE)));
        } else {
            app.add_systems(sim, rules);
            app.add_systems(sim, goombas);
        }
    }
}

fn spawn_smb1_mode_owner(
    mut commands: bevy::prelude::Commands,
    existing: bevy::prelude::Query<(), bevy::prelude::With<Smb1LevelState>>,
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
                (Smb1LevelState::default(), flag::FlagSequence::default()),
            )
            .insert(ambition::platformer::lifecycle::ModeScopedEntity(
                SMB1_MODE.to_string(),
            ));
    }
}

/// The level clock runs on the SIM clock, so pause and bullet-time slow it exactly
/// as they slow everything else. It clamps at zero rather than going negative.
fn tick_level_clock(
    time: bevy::prelude::Res<ambition::time::WorldTime>,
    mut level: bevy::prelude::Query<(&mut Smb1LevelState, &flag::FlagSequence)>,
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
    mut owners: bevy::prelude::Query<(&mut flag::FlagSequence, &mut Smb1LevelState)>,
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

/// Install the SMB1 demo content layer into an engine app.
pub fn add_demo_content(app: &mut App) {
    app.add_plugins(Smb1DemoContentPlugin);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smb1_demo_content_plugin_installs() {
        let mut app = App::new();
        add_demo_content(&mut app);
        let catalog = app
            .world()
            .resource::<ambition::characters::actor::character_catalog::CharacterCatalog>();
        assert!(catalog.get(provider::MARY_O_CHARACTER_ID).is_some());
        // Mary-O's authored grant list composes to her platformer moveset —
        // run+jump, air jump, wall mobility, fast fall — and NOTHING from the
        // full Ambition kit (blink/dash/fly/attack). This is her AbilityBase; the
        // session mask can only narrow it, never restore the sandbox kit.
        let mary_o_kit = catalog
            .ability_set(provider::MARY_O_CHARACTER_ID)
            .expect("Mary-O authors a grant list");
        assert_eq!(
            mary_o_kit,
            ambition::engine_core::AbilitySet::compose(&[
                ambition::engine_core::AbilityGrant::RunJump,
                ambition::engine_core::AbilityGrant::AirJump,
                ambition::engine_core::AbilityGrant::WallMobility,
                ambition::engine_core::AbilityGrant::FastFall,
            ]),
            "Mary-O composes to the classic platformer moveset"
        );
        assert!(
            mary_o_kit.jump
                && mary_o_kit.move_horizontal
                && mary_o_kit.double_jump
                && mary_o_kit.wall_jump
                && mary_o_kit.wall_cling
                && mary_o_kit.fast_fall,
            "the platformer verbs are all lit"
        );
        assert!(
            !mary_o_kit.blink
                && !mary_o_kit.dash
                && !mary_o_kit.fly
                && !mary_o_kit.attack
                && !mary_o_kit.wall_climb,
            "but none of the full Ambition kit"
        );
        // Her AirJump grant is a TRIPLE jump: she authors a per-character axis
        // tuning (air_jumps = 2) that rides an AuthoredMovementTuning marker, so
        // the count comes from HER row rather than the shared F3 dev tuning
        // (which defaults to 1). This is the axis-path analogue of `momentum`.
        let mary_o_tuning = catalog
            .axis_tuning(provider::MARY_O_CHARACTER_ID)
            .expect("Mary-O authors an axis tuning");
        assert_eq!(
            mary_o_tuning.air_jumps, 2,
            "two air jumps: a ground jump plus two = a triple jump"
        );
        // Everything else in her feel stays at the shared default — she overrides
        // only what she authors (the gravity/jump arc Jon blessed is untouched).
        assert_eq!(
            mary_o_tuning.gravity,
            ambition::engine_core::DEFAULT_TUNING.gravity,
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

    /// The room claims its mode, which is what a hosted `Smb1RulesPlugin` wakes on.
    #[test]
    fn level_1_1_claims_the_mary_o_mode() {
        assert_eq!(level_1_1().metadata.mode.as_deref(), Some(SMB1_MODE));
        assert_ne!(SMB1_MODE, "sanic", "two demos, two modes, one binary");
    }

    /// The level clock counts DOWN on the sim clock and clamps at zero. `hosted()`
    /// gates it on the mode; `global()` does not. The same seam as Sanic's act
    /// timer, for a completely different game — which is the D-C pattern's claim.
    #[test]
    fn hosted_rules_tick_the_level_clock_only_in_mary_o_rooms() {
        use ambition::world::rooms::{ActiveRoomMetadata, RoomMetadata};

        fn remaining(app: &mut App) -> Option<f32> {
            let mut q = app.world_mut().query::<&Smb1LevelState>();
            q.iter(app.world()).next().map(|s| s.time_remaining)
        }
        fn shell(rules: Smb1RulesPlugin, mode: Option<&str>, dt: f32) -> App {
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
        let mut app = shell(Smb1RulesPlugin::hosted(), Some(SMB1_MODE), 1.0);
        app.update();
        app.update();
        assert_eq!(remaining(&mut app), Some(STARTING_TIME - 2.0));

        // Hosted, in one of Ambition's own rooms: no owner, no clock.
        let mut app = shell(Smb1RulesPlugin::hosted(), None, 1.0);
        app.update();
        assert_eq!(remaining(&mut app), None, "the rules sleep out of mode");

        // Standalone: the demo IS the game, so no mode is needed.
        let mut app = shell(Smb1RulesPlugin::global(), None, 1.0);
        app.update();
        assert_eq!(remaining(&mut app), Some(STARTING_TIME - 1.0));

        // The clock clamps at zero rather than running negative.
        let mut app = shell(Smb1RulesPlugin::global(), None, STARTING_TIME * 2.0);
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
        app.add_plugins(Smb1RulesPlugin::global());

        // First update spawns the mode owner; drive the clock below full so the
        // rearm's refill is observable, then drop a settled tally onto the owner.
        app.update();
        {
            let mut q = app
                .world_mut()
                .query::<(&mut flag::FlagSequence, &mut Smb1LevelState)>();
            let world = app.world_mut();
            let (mut seq, mut level) = q.iter_mut(world).next().expect("owner spawned");
            seq.phase = flag::FlagPhase::Tallied { score: 800 };
            level.time_remaining = 123.0;
        }

        fn state(app: &mut App) -> (flag::FlagPhase, f32) {
            let mut q = app
                .world_mut()
                .query::<(&flag::FlagSequence, &Smb1LevelState)>();
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
