//! Sanic-style demo content home.
//!
//! This crate intentionally depends only on the `ambition` facade crate. It is
//! the E9 engine-for-other-games ORACLE: a second platformer's content is
//! authored entirely through the umbrella surface, never by reaching into a
//! lower `ambition_*` crate or copying `game/ambition_app`'s dependency wall.
//! If authoring a room here needs a type the umbrella does not re-export, that
//! is a real engine leak — and it fails to compile HERE, which is the point.
//!
//! What lives here is the SHOWCASE GEOMETRY (a landmarked momentum speedway
//! with a rideable loop), plus the mode-local rules that make progress legible
//! through milestone SFX. The windowed shell loads the shared Ambition art tree
//! through the public facade, so generated Sanic art and existing parallax /
//! block assets light up without an `ambition_app` dependency.

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

/// Number of segments in the full 360-degree loop body.
pub const LOOP_SEGMENTS: usize = 128;

/// Samples in the raised entry ramp. The ramp, loop, and runout are one open
/// surface route; there is no chain-transfer seam at either side of the loop.
pub const LOOP_RAMP_SEGMENTS: usize = 32;

/// Samples in the post-loop route: a flat foreground overpass clears the
/// crossover before a separate descent returns to the tiled floor.
pub const LOOP_RUNOUT_SEGMENTS: usize = 32;

/// Flat samples after the full revolution. Keeping the rider attached until it
/// is horizontally clear of the inbound rail is the physical half of the 2.5D
/// crossover; depth ordering supplies the visual half.
pub const LOOP_OVERPASS_SEGMENTS: usize = 12;

/// Remaining samples in the descent from the overpass to the floor.
pub const LOOP_DESCENT_SEGMENTS: usize = LOOP_RUNOUT_SEGMENTS - LOOP_OVERPASS_SEGMENTS;

/// Index of the loop's first arc point inside the combined route.
pub const LOOP_ENTRY_POINT_INDEX: usize = LOOP_RAMP_SEGMENTS;

/// Index where the full loop returns to the crossover after 360 degrees.
pub const LOOP_CLOSURE_POINT_INDEX: usize = LOOP_ENTRY_POINT_INDEX + LOOP_SEGMENTS;

/// Lower-arc segments rendered in front of the player on each side of the
/// loop. The remaining loop body is on the ordinary/back lane.
pub const LOOP_FOREGROUND_SEGMENTS_PER_SIDE: usize = 22;

/// Index of the route's final floor-level runout point.
pub const LOOP_EXIT_POINT_INDEX: usize = LOOP_CLOSURE_POINT_INDEX + LOOP_RUNOUT_SEGMENTS;

const LOOP_RADIUS: f32 = 180.0;
const LOOP_START_ANGLE: f32 = std::f32::consts::FRAC_PI_2;
const LOOP_SWEEP_ANGLE: f32 = std::f32::consts::TAU;
const LOOP_CENTER_X: f32 = 2200.0;
const LOOP_TRACK_RISE: f32 = 84.0;
const LOOP_RAMP_START_X: f32 = 1740.0;
const LOOP_OVERPASS_END_X: f32 = 2480.0;
const LOOP_RUNOUT_END_X: f32 = 2920.0;

fn cubic_bezier(p0: ae::Vec2, p1: ae::Vec2, p2: ae::Vec2, p3: ae::Vec2, t: f32) -> ae::Vec2 {
    let u = 1.0 - t;
    p0 * (u * u * u) + p1 * (3.0 * u * u * t) + p2 * (3.0 * u * t * t) + p3 * (t * t * t)
}

/// Build one continuous entry-ramp → full-loop → runout route.
///
/// The loop starts at its bottom with a horizontal tangent, makes a complete
/// 360-degree revolution, returns to the same screen-space point at a later arc
/// length, and continues into a distinct runout. The repeated point is a 2.5D
/// crossover: per-segment depth lanes distinguish the inbound/back rail from the
/// outbound/front rail while the riding solver follows one unambiguous arc-length
/// route. This is the classic Sonic topology rather than a literal planar circle.
fn raised_full_loop_points(floor_top: f32) -> (Vec<ae::Vec2>, ae::Vec2) {
    let center = ae::Vec2::new(LOOP_CENTER_X, floor_top - LOOP_TRACK_RISE - LOOP_RADIUS);
    let ramp_start = ae::Vec2::new(LOOP_RAMP_START_X, floor_top);
    let loop_start =
        center + ae::Vec2::new(LOOP_START_ANGLE.cos(), LOOP_START_ANGLE.sin()) * LOOP_RADIUS;
    debug_assert!((loop_start.y - (floor_top - LOOP_TRACK_RISE)).abs() < 1.0e-3);

    let mut points =
        Vec::with_capacity(1 + LOOP_RAMP_SEGMENTS + LOOP_SEGMENTS + LOOP_RUNOUT_SEGMENTS);
    points.push(ramp_start);

    // Rise from the tiled floor to the loop bottom with a horizontal tangent at
    // both ends. The first segment is a real ramp, not a teleport or rebound-only
    // gap, while the final tangent exactly matches the loop's bottom tangent.
    let ramp_control_1 = ramp_start + ae::Vec2::new(150.0, 0.0);
    let ramp_control_2 = loop_start - ae::Vec2::new(170.0, 0.0);
    for step in 1..=LOOP_RAMP_SEGMENTS {
        let t = step as f32 / LOOP_RAMP_SEGMENTS as f32;
        points.push(cubic_bezier(
            ramp_start,
            ramp_control_1,
            ramp_control_2,
            loop_start,
            t,
        ));
    }

    // Decreasing theta gives the rideable inward normals expected by the
    // surface kernel. The final sample intentionally equals `loop_start`: it is
    // non-adjacent to the entry sample, so no degenerate segment is introduced.
    for step in 1..=LOOP_SEGMENTS {
        let t = step as f32 / LOOP_SEGMENTS as f32;
        let theta = LOOP_START_ANGLE - LOOP_SWEEP_ANGLE * t;
        points.push(center + ae::Vec2::new(theta.cos(), theta.sin()) * LOOP_RADIUS);
    }

    // Cross the loop mouth on a flat foreground deck first. A high-speed rider
    // may legitimately launch when a track begins descending; doing that at the
    // coincident inbound/outbound point lets the airborne circle immediately
    // re-hit the back rail. The flat deck keeps the rider attached until the
    // simulated-depth crossover is physically clear.
    let overpass_end = ae::Vec2::new(LOOP_OVERPASS_END_X, loop_start.y);
    for step in 1..=LOOP_OVERPASS_SEGMENTS {
        let t = step as f32 / LOOP_OVERPASS_SEGMENTS as f32;
        points.push(loop_start.lerp(overpass_end, t));
    }

    // Descend only after clearing the loop's rightmost extent. Both cubic end
    // tangents are horizontal, so the overpass/descent seam and the return to
    // the tiled floor are smooth. Launching from this convex descent is valid
    // Sonic behavior because no back-lane rail remains underneath it.
    let runout_end = ae::Vec2::new(LOOP_RUNOUT_END_X, floor_top);
    let runout_control_1 = overpass_end + ae::Vec2::new(120.0, 0.0);
    let runout_control_2 = runout_end - ae::Vec2::new(160.0, 0.0);
    for step in 1..=LOOP_DESCENT_SEGMENTS {
        let t = step as f32 / LOOP_DESCENT_SEGMENTS as f32;
        points.push(cubic_bezier(
            overpass_end,
            runout_control_1,
            runout_control_2,
            runout_end,
            t,
        ));
    }

    (points, center)
}

/// Canonical transform pair for the demo's semantic Utility action (D in the
/// classic arrows+Z/X/C preset).
pub const SANIC_CHARACTER_ID: &str = "sanic";
pub const SUPER_SANIC_CHARACTER_ID: &str = "super_sanic";

/// Visually authored distance markers. The floating marker platforms and the
/// one-shot milestone SFX share this table so the eye and ear measure the same
/// positions instead of drifting as the speedway changes.
pub const SPEED_MARKER_XS: [f32; 5] = [600.0, 1200.0, 1800.0, 2600.0, 3400.0];

/// Build the Sanic momentum showcase room through the `ambition` umbrella
/// surface ONLY. The tiled solid floor remains the ordinary run surface. A
/// rebound feeds one continuously sampled raised ramp, complete 360-degree
/// depth-layered loop, and floor-level runout chain.
pub fn sanic_speedway() -> RoomSpec {
    let width = 4000.0;
    let height = 720.0;
    let floor_top = height - 48.0;

    // Ambition's tiled block-art path supplies a readable ground fill. This is
    // a real solid floor: the raised loop route is reached by the existing
    // rebound, completes a full revolution, then descends back to ground.
    let mut tiled_floor = ae::Block::solid(
        "speedway_floor",
        ae::Vec2::new(0.0, floor_top),
        ae::Vec2::new(width, 48.0),
    );
    tiled_floor.id = ae::GeoId::tile_layer("sanic_speedway_ground", 0);
    let mut blocks = vec![tiled_floor];
    blocks.push(ae::Block::one_way(
        "start_gantry",
        ae::Vec2::new(64.0, floor_top - 190.0),
        ae::Vec2::new(260.0, 18.0),
    ));
    for (index, x) in SPEED_MARKER_XS.into_iter().enumerate() {
        let lift = if index % 2 == 0 { 150.0 } else { 220.0 };
        blocks.push(ae::Block::one_way(
            format!("distance_marker_{}", index + 1),
            ae::Vec2::new(x - 52.0, floor_top - lift),
            ae::Vec2::new(104.0, 14.0),
        ));
    }
    blocks.push(ae::Block::rebound(
        "speed_booster",
        ae::Vec2::new(1640.0, floor_top - 22.0),
        ae::Vec2::new(72.0, 22.0),
        // Feed the raised ramp with enough horizontal speed to demonstrate the
        // complete revolution while leaving the player in control. The ramp,
        // loop, and runout remain one physical route; this is course pacing, not
        // a chain-transfer workaround.
        ae::Vec2::new(1120.0, -260.0),
    ));
    blocks.push(ae::Block::hazard(
        "finish_warning_spikes",
        ae::Vec2::new(width - 220.0, floor_top - 20.0),
        ae::Vec2::new(116.0, 20.0),
    ));
    blocks.push(ae::Block::solid(
        "finish_tower",
        ae::Vec2::new(width - 72.0, floor_top - 250.0),
        ae::Vec2::new(32.0, 250.0),
    ));
    let spawn = ae::Vec2::new(160.0, floor_top - 64.0);

    let (ramp_loop_points, loop_center) = raised_full_loop_points(floor_top);
    let mut loop_depths = vec![0_i8; ramp_loop_points.len() - 1];
    // The approach lives behind the player. The two lower loop shoulders and
    // the outbound runout live in front, creating the classic inside/outside
    // crossover without adding a second collision surface.
    loop_depths[..LOOP_RAMP_SEGMENTS].fill(-1);
    let front = LOOP_FOREGROUND_SEGMENTS_PER_SIDE.min(LOOP_SEGMENTS / 2);
    loop_depths[LOOP_ENTRY_POINT_INDEX..LOOP_ENTRY_POINT_INDEX + front].fill(1);
    loop_depths[LOOP_CLOSURE_POINT_INDEX - front..].fill(1);
    // Momentum bodies ride an authored floor guide over the same tiled solid
    // block. That makes the floor/ramp split a first-class route junction: the
    // player can keep running on the floor or hold toward the raised ramp,
    // without an airborne transfer hack. Chains are ordered loop first so the
    // long-lived test fixtures keep the loop at index 0.
    let floor_route = ae::SurfaceChain::open(
        "sanic_floor_route",
        vec![
            ae::Vec2::new(0.0, floor_top),
            ae::Vec2::new(LOOP_RAMP_START_X, floor_top),
            ae::Vec2::new(LOOP_RUNOUT_END_X, floor_top),
            ae::Vec2::new(width, floor_top),
        ],
    );
    let ramp_loop = ae::SurfaceChain::open("sanic_loop", ramp_loop_points)
        .with_segment_depths(loop_depths)
        .with_junctions(vec![
            ae::SurfaceJunction::new(vec![LOOP_ENTRY_POINT_INDEX, LOOP_CLOSURE_POINT_INDEX]),
            ae::SurfaceJunction::across(vec![
                ae::SurfacePort::local(0),
                ae::SurfacePort::chain(1, 1),
            ]),
            ae::SurfaceJunction::across(vec![
                ae::SurfacePort::local(LOOP_EXIT_POINT_INDEX),
                ae::SurfacePort::chain(1, 2),
            ]),
        ]);

    let world = ae::World::new(
        "Sanic Speedway",
        ae::Vec2::new(width, height),
        spawn,
        blocks,
    )
    .with_chains(vec![ramp_loop, floor_route]);

    let mut room = RoomSpec::new(SPEEDWAY_ROOM_ID, world);
    room.metadata.mode = Some(SANIC_MODE.to_string());
    // Borrow Ambition's generated skybridge stack. The visible shell loads the
    // shared `GameAssets`; if those optional images are absent the renderer keeps
    // the deterministic clear-color + landmark geometry fallback.
    room.metadata.biome = Some("skybridge".to_string());
    room.metadata.visual_theme = Some("skybridge".to_string());
    room.metadata.visual_profile.id = Some("sanic_speedway".to_string());
    room.metadata.visual_profile.parallax_theme = Some("skybridge".to_string());

    // World-space labels turn the speedway into a ruler. They are ordinary room
    // debug labels rendered by the generic presentation face, not app-local UI.
    let mut labels = vec![
        (
            "start".to_string(),
            "START   Z: JUMP   DOWN+X: REV   RELEASE DOWN: DASH   D: SUPER".to_string(),
            ae::Vec2::new(300.0, floor_top - 230.0),
        ),
        (
            "loop".to_string(),
            "LOOP".to_string(),
            ae::Vec2::new(loop_center.x, loop_center.y - LOOP_RADIUS - 36.0),
        ),
        (
            "finish".to_string(),
            "FINISH".to_string(),
            ae::Vec2::new(width - 130.0, floor_top - 300.0),
        ),
    ];
    labels.extend(SPEED_MARKER_XS.into_iter().enumerate().map(|(index, x)| {
        (
            format!("marker_{}", index + 1),
            format!("{x:.0}"),
            ae::Vec2::new(x, floor_top - 280.0),
        )
    }));
    room.debug_labels = labels
        .into_iter()
        .map(|(id, text, position)| {
            ambition::world::rooms::Authored::new(
                format!("sanic_{id}"),
                text.clone(),
                ae::Aabb::new(position, ae::Vec2::splat(1.0)),
                ambition::world::debug_label::DebugLabel::new(
                    text,
                    position,
                    ambition::world::debug_label::DebugLabelKind::Custom,
                ),
            )
        })
        .collect();
    room
}

/// The demo's two-form catalog. Every demo installs its own roster; the engine
/// ships none (ADR 0017). The visible shell resolves both generated Sanic forms
/// through the shared Ambition asset catalog. Missing local artifacts remain a
/// loud, marked fallback rather than a second sprite path.
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
            sprite_tuning: Some((collision_scale: 1.6, frame_sample_inset: 1)),
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
        "super_sanic": (
            sprite_tuning: Some((collision_scale: 1.6, frame_sample_inset: 1)),
            display_name: "Super Sanic",
            spritesheet: "sprites/super_sanic_spritesheet.png",
            manifest: "sprites/super_sanic_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            tags: ["player", "super", "transformation"],
            // This slice is an identity/presentation transformation. It keeps
            // the same authored peaceful kit and momentum tuning so D cannot
            // accidentally become a second gameplay-authority path.
            momentum: Some((
                ground_accel: 900.0,
                top_speed: 1200.0,
                jump_speed: 700.0,
            )),
        ),
    },
)"#;

pub mod ball_dash;
pub mod provider;

pub use provider::{
    sanic_session_world, SanicExperiencePlugin, SanicSessionWorld, SANIC_EXPERIENCE,
    SANIC_GAMEPLAY_ROUTE, SANIC_LAUNCHER_ROUTE,
};

/// Content plugin for the Sanic movement demo: registers Sanic's App-local
/// authored catalogs, installs the world, and adds the engine's sim-world setup. This is the shape
/// `crates/ambition_host/tests/demo_shell_smoke.rs` prescribes, built through the
/// `ambition` umbrella alone.
pub struct SanicDemoContentPlugin;

/// Register Sanic's immutable authored definitions in one Bevy `App`: the
/// character fragment and the provider-indexed music/SFX fragments. Shared by the historical
/// [`SanicDemoContentPlugin`] (Startup-driven construction) and the new
/// [`provider::SanicExperiencePlugin`] (shell-activation-driven construction), so
/// there is one definition of Sanic's content seam.
///
/// The App-local resources are the new composition authority. The final block
/// dual-writes the legacy pure-lookup seams until every remaining actor/sprite
/// consumer has been migrated to explicit catalog access.
pub fn install_sanic_content(app: &mut App) {
    use ambition::audio::catalog::{AudioCatalogAppExt, AudioCatalogFragment};
    use ambition::characters::actor::character_catalog::{
        CharacterCatalogAppExt, CharacterCatalogFragment,
    };

    app.register_character_catalog_fragment(
        CharacterCatalogFragment::from_ron(
            provider::SANIC_EXPERIENCE,
            Some(SANIC_CHARACTER_ID),
            SANIC_CATALOG_RON,
        )
        .expect("Sanic character catalog should be valid"),
    );
    app.register_audio_catalog_fragment(
        AudioCatalogFragment::new(
            provider::SANIC_EXPERIENCE,
            Some(ambition::audio::spec::MusicRegistry {
                default_track: "you_are_too_slow".to_string(),
                tracks: vec![ambition::audio::spec::MusicTrack {
                    id: "you_are_too_slow".to_string(),
                    display_name: "You Are Too Slow".to_string(),
                    asset_path: Some(SANIC_MUSIC_ASSET_PATH.to_string()),
                }],
            }),
            Some(ambition::audio::spec::SfxRegistry {
                sample_rate: 44_100,
                sfx: Vec::new(),
            }),
        )
        .expect("Sanic audio catalogs should be valid"),
    );

    // Audio is fully App-local: the fragment registered above is the sole
    // authority. The remaining legacy character seam stays fed while the
    // pure character-lookup consumers are migrated in the next slice.
    ambition::runtime::demo_fixture::install_character_catalog(SANIC_CATALOG_RON);
}

impl Plugin for SanicDemoContentPlugin {
    fn build(&self, app: &mut App) {
        use ambition::runtime::demo_fixture::{ActiveRoomMetadata, RoomSet};
        use bevy::prelude::IntoScheduleConfigs;

        install_sanic_content(app);
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
    character_catalog: bevy::prelude::Res<
        ambition::characters::actor::character_catalog::CharacterCatalog,
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
            editable_tuning: &editable_tuning,
            starting_character: &starting_character,
            character_catalog: &character_catalog,
            default_character_id: SANIC_CHARACTER_ID,
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
    /// Next index in [`SPEED_MARKER_XS`] that should emit its one-shot progress
    /// cue. Mode-scoped with the act, so leaving and re-entering the demo resets
    /// the audible ruler without a global resource leak.
    pub next_milestone: usize,
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
        // The plugin OWNS its mandatory message channels: three of its systems
        // write SFX cues, so a thin host without the audio stack still builds.
        app.add_message::<ambition::sfx::SfxMessage>();
        use bevy::prelude::IntoScheduleConfigs;
        let sim = ambition::platformer::schedule::SimScheduleExt::sim_schedule(app);
        app.init_resource::<ball_dash::BallDashTuning>();
        // Attach the mode-local state and consume Sanic's semantic input verbs
        // before the generic peaceful-kit gate erases combat intent. Chaining
        // provides an apply-deferred seam, so the first eligible X edge cannot be
        // lost on a newly controlled body. Utility is D in the classic preset;
        // the transform system consumes that edge so it cannot also toggle a
        // host-code flight ability inherited by the control box.
        let sanic_input_rules = (
            ball_dash::attach_ball_dash,
            ball_dash::capture_ball_dash_input,
            toggle_sanic_form,
        )
            .chain()
            .in_set(ambition::platformer::schedule::SandboxSet::PlayerInput)
            .after(ambition::actors::avatar::tick_player_brains)
            .before(ambition::actors::avatar::gate_worn_player_control);
        if self.hosted {
            app.add_systems(
                sim,
                sanic_input_rules.run_if(ambition::runtime::in_mode(SANIC_MODE)),
            );
        } else {
            app.add_systems(sim, sanic_input_rules);
        }

        // The ball dash is a RULE, not world content: it exists while the Sanic
        // mode is live and nowhere else, exactly like the act clock. Effects run
        // after PlayerInput captured the technique and before later presentation.
        // `tick_ball_dash` precedes `tick_rolling`, so a launch cannot un-ball in
        // the same frame even if tuning changes.
        let rules = (
            spawn_sanic_mode_owner,
            tick_sanic_act,
            ball_dash::tick_ball_dash,
            ball_dash::tick_rolling,
        )
            .chain()
            .in_set(ambition::platformer::schedule::SandboxSet::GameplayEffects);
        let milestone_sfx = emit_sanic_milestone_sfx
            .in_set(ambition::platformer::schedule::SandboxSet::GameplayEffects);
        if self.hosted {
            app.add_systems(sim, rules.run_if(ambition::runtime::in_mode(SANIC_MODE)));
            app.add_systems(
                sim,
                milestone_sfx.run_if(ambition::runtime::in_mode(SANIC_MODE)),
            );
        } else {
            app.add_systems(sim, rules);
            app.add_systems(sim, milestone_sfx);
        }
    }
}

/// Toggle the controlled body between the two catalog-authored Sanic forms.
///
/// This consumes the already-semantic Utility edge (`D` in the demo's classic
/// keyboard preset), never a raw key. Both rows carry the same movement and
/// peaceful action profile, so `WornCharacter` remains the single gameplay +
/// presentation authority and the transformation cannot fork a second kit path.
fn toggle_sanic_form(
    subject: Option<bevy::prelude::Res<ambition::platformer::markers::ControlledSubject>>,
    mut bodies: bevy::prelude::Query<(
        &mut ambition::characters::brain::ActorControl,
        &mut ambition::characters::actor::WornCharacter,
        &ae::BodyKinematics,
    )>,
    mut sfx: bevy::prelude::MessageWriter<ambition::sfx::SfxMessage>,
) {
    let Some(entity) = subject.and_then(|subject| subject.0) else {
        return;
    };
    let Ok((mut control, mut worn, kinematics)) = bodies.get_mut(entity) else {
        return;
    };
    if !control.0.fly_toggle_pressed {
        return;
    }

    // Utility belongs to this mode-local transformation. Consume the edge before
    // lower movement layers can interpret it as the generic fly toggle.
    control.0.fly_toggle_pressed = false;
    let next = match worn.id() {
        SANIC_CHARACTER_ID => SUPER_SANIC_CHARACTER_ID,
        SUPER_SANIC_CHARACTER_ID => SANIC_CHARACTER_ID,
        _ => return,
    };
    *worn = ambition::characters::actor::WornCharacter::new(next);
    sfx.write(ambition::sfx::SfxMessage::Dash {
        pos: kinematics.pos,
    });
}

/// Bring the act state into being the first frame the mode is live. Spawned
/// `spawn_mode_scoped`, so the engine despawns it when the active room's mode
/// changes — no teardown code here.
fn spawn_sanic_mode_owner(
    mut commands: bevy::prelude::Commands,
    existing: bevy::prelude::Query<(), bevy::prelude::With<SanicActState>>,
    session: Option<bevy::prelude::Res<ambition::platformer::lifecycle::ActiveSessionScope>>,
    mut sfx: bevy::prelude::MessageWriter<ambition::sfx::SfxMessage>,
) {
    use ambition::platformer::lifecycle::{SessionSpawnScope, SpawnSessionScopedExt};
    // Sleep when a session-scoped host has retired the live session (i.e. at the
    // launcher): the room metadata may still read "sanic" until the next
    // activation overwrites it, but there is no session to own new state, so the
    // act owner must not be resurrected. When no `ActiveSessionScope` exists at
    // all (the historical Startup path and the D-C tests) the guard is inert.
    let session_live = session
        .as_ref()
        .map_or(true, |scope| scope.current().is_some());
    let spawn_scope = session
        .as_ref()
        .map_or(SessionSpawnScope::UNSCOPED, |scope| scope.spawn_scope());
    if session_live && existing.iter().next().is_none() {
        // Owned by BOTH the mode (survives in-session room changes) and the
        // active session (torn down on a shell relaunch, which a same-mode
        // reload is NOT — so mode-scope alone would leak the act across a
        // launch → quit → relaunch cycle).
        commands
            .spawn_session_scoped(spawn_scope, SanicActState::default())
            .insert(ambition::platformer::lifecycle::ModeScopedEntity(
                SANIC_MODE.to_string(),
            ));
        // Audible confirmation that the standalone shell is draining the
        // standard SfxMessage seam. Distance markers emit alternating cues as
        // the player advances, so this one also proves the bank at room entry.
        sfx.write(ambition::sfx::SfxMessage::Dash {
            pos: ae::Vec2::ZERO,
        });
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

/// Emit a small, existing Ambition cue when the primary body crosses each
/// visible distance marker. These are deliberately simple diagnostic sounds:
/// the demo is proving that its shell drains the standard [`ambition::sfx::SfxMessage`] seam,
/// not inventing a parallel Sanic audio stack.
fn emit_sanic_milestone_sfx(
    player: bevy::prelude::Query<
        &ae::BodyKinematics,
        bevy::prelude::With<ambition::actors::actor::PrimaryPlayer>,
    >,
    mut act: bevy::prelude::Query<&mut SanicActState>,
    mut sfx: bevy::prelude::MessageWriter<ambition::sfx::SfxMessage>,
) {
    let Ok(kin) = player.single() else {
        return;
    };
    for mut state in &mut act {
        while let Some(&marker_x) = SPEED_MARKER_XS.get(state.next_milestone) {
            if kin.pos.x < marker_x {
                break;
            }
            let message = if state.next_milestone % 2 == 0 {
                ambition::sfx::SfxMessage::Dash { pos: kin.pos }
            } else {
                ambition::sfx::SfxMessage::Jump { pos: kin.pos }
            };
            sfx.write(message);
            state.next_milestone += 1;
        }
    }
}

/// Install the Sanic demo content layer into an engine app.
pub fn add_demo_content(app: &mut App) {
    app.add_plugins(SanicDemoContentPlugin);
}

#[cfg(test)]
mod tests;
