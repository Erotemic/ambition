//! Sanic-style demo content home.
//!
//! This crate intentionally depends only on the `ambition` facade crate. It is
//! the E9 engine-for-other-games ORACLE: a second platformer's content is
//! authored entirely through the umbrella surface, never by reaching into a
//! lower `ambition_*` crate or copying `game/ambition_app`'s dependency wall.
//! If authoring a room here needs a type the umbrella does not re-export, that
//! is a real engine leak — and it fails to compile HERE, which is the point.
//!
//! What lives here is the SHOWCASE GEOMETRY (a long momentum speedway with a
//! rideable Sonic loop). The movement-identity FEEL (momentum tuning, the
//! playable binary, character art) is a separate interactive build — it cannot
//! be dialed in headlessly — but the room a Sanic demo runs on is authored and
//! verified here through the engine's public surface.

use ambition::engine_core as ae;
use ambition::prelude::*;
use ambition::world::rooms::RoomSpec;

/// Stable room id for the momentum speedway.
pub const SPEEDWAY_ROOM_ID: &str = "sanic_speedway";

/// The game-MODE tag this demo's rooms carry (decomposition D-C).
///
/// Ambition hosts this demo by loading its rooms alongside its own; a Sanic
/// rules plugin gates its systems on `ambition::runtime::in_mode(SANIC_MODE)`
/// so they sleep everywhere else. [`SanicRulesPlugin`] is that ruleset, and its
/// `hosted()` / `global()` constructor flag is the D-C pattern made real.
pub const SANIC_MODE: &str = "sanic";

/// Authored soundtrack for the standalone Sanic demo. The rendered asset lives
/// in the shared engine asset tree beside the other generated music tracks.
pub const SANIC_MUSIC_ASSET_PATH: &str = "audio/music/generated/you_are_too_slow/full.ogg";

/// Number of segments in the generated Sonic loop polygon.
const LOOP_SEGMENTS: usize = 24;

/// Build the Sanic momentum showcase room through the `ambition` umbrella
/// surface ONLY: a wide room with a long solid floor and a rideable full LOOP
/// authored as a `SurfaceChain` (the momentum-locomotion geometry — a fast body
/// rides up the inside of the loop, across the top, and back down).
///
/// The loop winds with DECREASING angle so each segment's `(t.y, -t.x)` normal
/// points toward the loop center (interior-rideable), matching the engine's
/// `SurfaceLoop` marker convention.
pub fn sanic_speedway() -> RoomSpec {
    let width = 4000.0;
    let height = 720.0;
    let floor_top = height - 48.0;

    // A single long solid floor spanning the room, plus a spawn just above it.
    let floor = ae::Block::solid(
        "speedway_floor",
        ae::Vec2::new(0.0, floor_top),
        ae::Vec2::new(width, 48.0),
    );
    let spawn = ae::Vec2::new(160.0, floor_top - 64.0);

    // The Sonic loop: a closed polygon centered over the floor, interior-rideable.
    let loop_center = ae::Vec2::new(width * 0.5, floor_top - 200.0);
    let loop_radius = 180.0;
    let loop_points: Vec<ae::Vec2> = (0..LOOP_SEGMENTS)
        .map(|k| {
            let theta = -std::f32::consts::TAU * (k as f32) / (LOOP_SEGMENTS as f32);
            loop_center + ae::Vec2::new(theta.cos(), theta.sin()) * loop_radius
        })
        .collect();
    let sonic_loop = ae::SurfaceChain::closed_loop("sanic_loop", loop_points);

    let world = ae::World::new(
        "Sanic Speedway",
        ae::Vec2::new(width, height),
        spawn,
        vec![floor],
    )
    .with_chains(vec![sonic_loop]);

    let mut room = RoomSpec::new(SPEEDWAY_ROOM_ID, world);
    room.metadata.mode = Some(SANIC_MODE.to_string());
    room
}

/// The demo's one-character catalog. Every demo installs its own roster; the
/// engine ships none (ADR 0017). The speedster wears the engine's fallback box
/// until the sheet lands — the FEEL half is the separate interactive build, but
/// the shell must not wait on it.
const SANIC_CATALOG_RON: &str = r#"(
    brain_presets: { "stand_still": StandStill },
    action_set_presets: {
        // A peaceful speedster: the momentum ride + ball dash ARE the kit; no
        // combat moveset. Referenced by the row below so the catalog is valid.
        "peaceful": (
            move_style: Walk,
            melee: None,
            ranged: None,
            special: None,
        ),
    },
    characters: {
        "sanic": (
            display_name: "Sanic",
            spritesheet: "sprites/sanic_spritesheet.png",
            manifest: "sprites/sanic_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            tags: ["player"],
            // The MOVEMENT identity that makes this a Sanic demo: the worn home
            // box opts into `MotionModel::SurfaceMomentum` (rides the speedway +
            // loop), which is also what `ball_dash` requires to charge/launch.
            // Without this the body is axis-swept and ball dash is inert — the
            // demo would be an Ambition player wearing the name "Sanic".
            momentum: Some((
                ground_accel: 900.0,
                top_speed: 1200.0,
                jump_speed: 700.0,
            )),
        ),
    },
)"#;

pub mod ball_dash;

/// Content plugin for the Sanic movement demo: installs the roster, the world,
/// and the engine's own sim-world setup. This is the shape
/// `crates/ambition_host/tests/demo_shell_smoke.rs` prescribes, built through the
/// `ambition` umbrella alone.
pub struct SanicDemoContentPlugin;

impl Plugin for SanicDemoContentPlugin {
    fn build(&self, app: &mut App) {
        use ambition::runtime::demo_fixture::{ActiveRoomMetadata, RoomSet};
        use bevy::prelude::IntoScheduleConfigs;

        ambition::runtime::demo_fixture::install_character_catalog(SANIC_CATALOG_RON);
        // The demo's player is explicitly the speedster rather than relying on
        // whichever row happens to be the installed catalog default.
        app.insert_resource(ambition::runtime::demo_fixture::StartingCharacter::new(
            "sanic",
        ));
        let room = sanic_speedway();
        app.insert_resource(ae::RoomGeometry(room.world.clone()));
        app.insert_resource(ActiveRoomMetadata(room.metadata.clone()));
        app.insert_resource(RoomSet::from_parts(
            SPEEDWAY_ROOM_ID,
            vec![room],
            Vec::new(),
        ));
        app.add_systems(
            bevy::app::Startup,
            sanic_setup.in_set(ambition::runtime::demo_fixture::SimulationSetupSet),
        );
    }
}

/// The demo's world construction: the engine's `simulation_world` on the
/// speedway. Labeled `SimulationSetupSet` so the host's input attach orders after
/// the player body exists.
#[allow(clippy::too_many_arguments)]
fn sanic_setup(
    mut commands: bevy::prelude::Commands,
    world: bevy::prelude::Res<ae::RoomGeometry>,
    room_set: bevy::prelude::Res<ambition::runtime::demo_fixture::RoomSet>,
    ldtk_index: bevy::prelude::Res<ambition::runtime::demo_fixture::LdtkRuntimeIndex>,
    editable_abilities: bevy::prelude::Res<ambition::runtime::demo_fixture::EditableAbilitySet>,
    editable_tuning: bevy::prelude::Res<ambition::runtime::demo_fixture::EditableMovementTuning>,
    starting_character: bevy::prelude::Res<ambition::runtime::demo_fixture::StartingCharacter>,
    asset_server: bevy::prelude::Res<bevy::asset::AssetServer>,
) {
    ambition::runtime::demo_fixture::simulation_world(
        &mut commands,
        ambition::runtime::demo_fixture::SimulationSetup {
            world: &world,
            room_set: &room_set,
            ldtk_index: &ldtk_index,
            editable_abilities: &editable_abilities,
            editable_tuning: &editable_tuning,
            starting_character: &starting_character,
            sandbox_data_asset: None,
            sandbox_asset_collection: None,
            asset_server: &asset_server,
        },
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// The RULES plugin — the D-C mode-scope seam, used for real.
// ─────────────────────────────────────────────────────────────────────────────

/// The act's live state, owned by the mode. It rides a `ModeScopedEntity`, so
/// leaving the Sanic rooms tears it down through the engine's lifetime-scope
/// vocabulary rather than a bespoke reset.
#[derive(bevy::prelude::Component, Default, Debug)]
pub struct SanicActState {
    /// Seconds the act has been running (sim clock, so bullet-time slows it).
    pub elapsed: f32,
}

/// Sanic's level rules. **ONE system list; a constructor flag decides its gating**
/// — [`SanicRulesPlugin::hosted`] when Ambition hosts the demo alongside its own
/// rooms, [`SanicRulesPlugin::global`] when the demo IS the game. That is the D-C
/// pattern (`docs/planning/engine/decomposition.md` §Phase D-C), and this is its
/// first real consumer: before this, `in_mode` had no ruleset to gate.
pub struct SanicRulesPlugin {
    hosted: bool,
}

impl SanicRulesPlugin {
    /// Ambition hosts this demo: every rule sleeps outside the Sanic rooms.
    pub fn hosted() -> Self {
        Self { hosted: true }
    }

    /// The demo IS the game: the rules run unconditionally.
    pub fn global() -> Self {
        Self { hosted: false }
    }
}

impl Plugin for SanicRulesPlugin {
    fn build(&self, app: &mut App) {
        use bevy::prelude::IntoScheduleConfigs;
        let sim = ambition::platformer::schedule::SimScheduleExt::sim_schedule(app);
        app.init_resource::<ball_dash::BallDashTuning>();
        // The ball dash is a RULE, not world content: it exists while the Sanic
        // mode is live and nowhere else, exactly like the act clock. Ordered
        // before `tick_rolling` so a launch and its un-balling can never share a
        // frame — a body that just launched is above `exit_speed` by definition,
        // but the ordering says so rather than relying on the tuning.
        let rules = (
            spawn_sanic_mode_owner,
            tick_sanic_act,
            ball_dash::attach_ball_dash,
            ball_dash::tick_ball_dash,
            ball_dash::tick_rolling,
        )
            .chain();
        if self.hosted {
            app.add_systems(sim, rules.run_if(ambition::runtime::in_mode(SANIC_MODE)));
        } else {
            app.add_systems(sim, rules);
        }
    }
}

/// Bring the act state into being the first frame the mode is live. Spawned
/// `spawn_mode_scoped`, so the engine despawns it when the active room's mode
/// changes — no teardown code here.
fn spawn_sanic_mode_owner(
    mut commands: bevy::prelude::Commands,
    existing: bevy::prelude::Query<(), bevy::prelude::With<SanicActState>>,
) {
    use ambition::platformer::lifecycle::SpawnScopedExt;
    if existing.iter().next().is_none() {
        commands.spawn_mode_scoped(SANIC_MODE, SanicActState::default());
    }
}

/// The act timer runs on the SIM clock (`scaled_dt`), so bullet-time and pause
/// slow it exactly as they slow everything else — `WorldTime`, never `Res<Time>`.
fn tick_sanic_act(
    time: bevy::prelude::Res<ambition::time::WorldTime>,
    mut act: bevy::prelude::Query<&mut SanicActState>,
) {
    for mut state in &mut act {
        state.elapsed += time.scaled_dt;
    }
}

/// Install the Sanic demo content layer into an engine app.
pub fn add_demo_content(app: &mut App) {
    app.add_plugins(SanicDemoContentPlugin);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanic_demo_content_plugin_installs() {
        let mut app = App::new();
        add_demo_content(&mut app);
    }

    /// The oracle: the momentum showcase room composes through the umbrella
    /// surface alone — floor geometry present, the Sonic loop validates, and the
    /// spawn sits inside the room bounds.
    #[test]
    fn sanic_speedway_composes_through_the_umbrella() {
        let room = sanic_speedway();
        assert_eq!(room.id, SPEEDWAY_ROOM_ID);

        // Solid floor geometry made it into the world.
        assert!(
            room.world.blocks.iter().any(|b| b.name == "speedway_floor"),
            "the speedway floor block is present"
        );

        // The Sonic loop is a valid rideable closed chain (the engine's own
        // validator runs here, so a degenerate/self-intersecting loop fails).
        let loop_chain = room
            .world
            .chains
            .iter()
            .find(|c| c.name == "sanic_loop")
            .expect("the sanic loop chain is present");
        assert_eq!(loop_chain.points.len(), LOOP_SEGMENTS);
        assert!(
            loop_chain.validate().is_empty(),
            "the generated Sonic loop is a valid rideable chain: {:?}",
            loop_chain.validate()
        );

        // Spawn is inside the room bounds (not floating/falling on load).
        let s = room.world.spawn;
        assert!(
            s.x >= 0.0 && s.x <= room.world.size.x && s.y >= 0.0 && s.y <= room.world.size.y,
            "spawn {s:?} is inside room bounds {:?}",
            room.world.size
        );
    }

    /// **The D-C pattern, end to end.** `SanicRulesPlugin::hosted()` ticks the act
    /// timer only inside the Sanic rooms; `::global()` ticks it everywhere. The
    /// mode-owner entity is `spawn_mode_scoped`, so the engine tears it down when
    /// the active room leaves the mode — this demo writes no teardown code.
    #[test]
    fn hosted_rules_run_only_in_sanic_rooms_and_global_rules_run_everywhere() {
        use ambition::bevy::ecs::system::RunSystemOnce as _;
        use ambition::world::rooms::{ActiveRoomMetadata, RoomMetadata};

        fn elapsed(app: &mut App) -> Option<f32> {
            let mut q = app.world_mut().query::<&SanicActState>();
            q.iter(app.world()).next().map(|s| s.elapsed)
        }
        fn shell(rules: SanicRulesPlugin, mode: Option<&str>) -> App {
            let mut app = App::new();
            ambition::engine::add_headless_foundation(&mut app);
            app.insert_resource(ActiveRoomMetadata(RoomMetadata {
                mode: mode.map(str::to_string),
                ..Default::default()
            }));
            app.insert_resource(ambition::time::WorldTime {
                scaled_dt: 0.5,
                ..Default::default()
            });
            app.add_plugins(rules);
            app
        }

        // HOSTED, inside a `sanic` room: the mode owner spawns and the act ticks.
        // `.chain()` puts a sync point between spawn and tick, so the owner exists
        // in time to tick on its own first frame: two frames = two ticks.
        let mut app = shell(SanicRulesPlugin::hosted(), Some(SANIC_MODE));
        app.update();
        app.update();
        assert_eq!(elapsed(&mut app), Some(1.0), "hosted rules tick in-mode");

        // HOSTED, in one of Ambition's own rooms: nothing spawns, nothing ticks.
        let mut app = shell(SanicRulesPlugin::hosted(), None);
        app.update();
        app.update();
        assert_eq!(elapsed(&mut app), None, "hosted rules sleep out of mode");

        // GLOBAL (the demo IS the game): the rules run with no mode at all.
        let mut app = shell(SanicRulesPlugin::global(), None);
        app.update();
        app.update();
        assert_eq!(
            elapsed(&mut app),
            Some(1.0),
            "standalone rules need no mode"
        );

        // The mode owner really is mode-scoped: the engine's own sweep retires it.
        let mut app = shell(SanicRulesPlugin::hosted(), Some(SANIC_MODE));
        app.update();
        app.update();
        assert!(elapsed(&mut app).is_some());
        app.insert_resource(ActiveRoomMetadata::default()); // left the Sanic rooms
        app.world_mut()
            .run_system_once(ambition::runtime::despawn_departed_mode_entities)
            .expect("the engine's mode sweep runs");
        assert_eq!(
            elapsed(&mut app),
            None,
            "leaving the mode tears the act state down — no demo teardown code"
        );
    }

    /// The D-C hosting oracle: a demo's room claims its mode, and the run
    /// condition that wakes a hosted ruleset inside it reaches this crate
    /// through the `ambition` umbrella alone. If gating a hosted demo ever
    /// needs a lower `ambition_*` crate, it fails to compile HERE.
    ///
    /// The condition is evaluated directly rather than through `.run_if` on a
    /// bespoke marker resource: a crate whose manifest names only `ambition`
    /// cannot `#[derive(Resource)]`, because bevy's derive macros resolve
    /// `bevy_ecs` through the CONSUMER's manifest and a re-export does not
    /// satisfy them. The `.run_if` wiring itself is pinned in
    /// `ambition_runtime/tests/mode_scope.rs`.
    #[test]
    fn the_speedway_claims_the_sanic_mode_and_wakes_a_hosted_ruleset() {
        use ambition::bevy::ecs::system::RunSystemOnce as _;
        use ambition::runtime::in_mode;
        use ambition::world::rooms::ActiveRoomMetadata;

        let room = sanic_speedway();
        assert_eq!(room.metadata.mode.as_deref(), Some(SANIC_MODE));

        let mut app = App::new();
        app.insert_resource(ActiveRoomMetadata(room.metadata.clone()));
        let awake = app
            .world_mut()
            .run_system_once(in_mode(SANIC_MODE))
            .expect("the mode condition runs");
        assert!(awake, "a hosted Sanic ruleset wakes inside the speedway");

        // Ambition's own rooms carry no mode, so the demo's rules sleep there.
        app.insert_resource(ActiveRoomMetadata::default());
        let awake = app
            .world_mut()
            .run_system_once(in_mode(SANIC_MODE))
            .expect("the mode condition runs");
        assert!(!awake, "and it sleeps in a room that claims no mode");
    }
}
