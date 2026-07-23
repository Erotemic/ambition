//! **Outlander** — the Phase-6 external-architecture proof.
//!
//! A complete (tiny) game authored from OUTSIDE the engine workspace, through
//! the `ambition` umbrella alone: one room, one character, one enemy, one
//! construction recipe, one transition. The point is not the game — it is the
//! evidence: every `ambition::` path this file imports is the de-facto SDK
//! surface, and every place it has to lean on an engine-internal assumption is
//! recorded in the campaign doc's Phase 6 account as an API leak.
//!
//! What each § authors and through which seam:
//! - §room     — `RoomSpec` in code (`ambition::world::rooms` + `engine_core`).
//! - §character— `CharacterCatalogFragment::from_ron` (the same catalog seam
//!               every in-repo provider uses).
//! - §enemy    — a `CharacterRosterFragment` archetype plus a
//!               `RoomContentStagingRegistry` stager. Because of Phase 4, the
//!               staged enemy is lowered as a CONSTRUCTION PLAN ROW through
//!               the `ambition.staged-actor` recipe — the "one construction
//!               recipe" this fixture consumes without defining (an external
//!               crate cannot add recipe BEHAVIOR; that closed enum is
//!               recorded leak #2).
//! - §transition — an in-room gate built on `engine_core::movement::transit_body`
//!               (ADR 0024). A cross-room `LoadingZone` swap is impossible from
//!               out here (wiring is app-local in `ambition_app::world_flow`) —
//!               recorded leak #1.

use bevy::prelude::*;

use ambition::engine_core as ae;
use ambition::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition::runtime::demo_fixture::{
    ActiveRoomMetadata, LdtkRuntimeIndex, RoomSet, StartingCharacter,
};
use ambition::runtime::PreparedPlatformerSource;
use ambition::world::rooms::RoomSpec;

pub const OUTLANDER_EXPERIENCE: &str = "outlander";
pub const OUTLANDER_GAMEPLAY_ROUTE: &str = "outlander_gameplay";
pub const OUTLANDER_LAUNCHER_ROUTE: &str = "outlander_launcher";
pub const OUTLANDER_CHARACTER_ID: &str = "outlander_wanderer";
pub const OUTLANDER_ROOM_ID: &str = "outlander_ridge";
pub const OUTLANDER_ENEMY_BRAIN_KEY: &str = "outlander_sentry";
pub const OUTLANDER_SENTRY_ID: &str = "outlander_sentry_0";

// ── §character ──────────────────────────────────────────────────────────────
// Reuses an engine-shipped spritesheet on purpose: consumer-owned art has no
// home under the current asset-root convention (leak #3). The catalog fragment
// itself — presets, body, kit — is authored here.
const OUTLANDER_CATALOG_RON: &str = r#"(
    brain_presets: { "stand_still": StandStill },
    action_set_presets: {
        "drifter": (
            move_style: Walk,
            melee: None,
            ranged: None,
            special: None,
        ),
    },
    characters: {
        "outlander_wanderer": (
            display_name: "Outlander",
            spritesheet: "sprites/mary_o_spritesheet.png",
            manifest: "sprites/mary_o_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "drifter",
            playable_kit: HostCode,
            tags: ["player", "external_consumer"],
        ),
    },
)"#;

// ── §enemy (archetype half) ─────────────────────────────────────────────────
const OUTLANDER_ROSTER_RON: &str = r#"{
    "outlander_sentry": (
        max_health: 2,
        patrol_speed: 38.0,
        chase_speed: 38.0,
        aggro_radius: 0.0,
        attack_range: 0.0,
        contact_strength: 0.5,
        damage_amount: 1,
        brain_template: Wanderer,
        move_style: Walk,
        respawn: OnRoomReenter,
    ),
}"#;

// ── §room ───────────────────────────────────────────────────────────────────
/// Two floors joined by the §transition gate: a lower ridge with the sentry,
/// and an upper ledge only the gate reaches (so the transition is load-bearing
/// for the fixture's acceptance walk, not decoration).
pub fn outlander_room() -> RoomSpec {
    let size = ae::Vec2::new(960.0, 540.0);
    let floor_top = 492.0;
    let ledge_top = 220.0;
    let world = ae::World::new(
        "Outlander Ridge",
        size,
        ae::Vec2::new(96.0, floor_top - 64.0),
        vec![
            ae::Block::solid(
                "ridge_floor",
                ae::Vec2::new(0.0, floor_top),
                ae::Vec2::new(size.x, 48.0),
            ),
            ae::Block::solid(
                "gate_ledge",
                ae::Vec2::new(600.0, ledge_top),
                ae::Vec2::new(280.0, 24.0),
            ),
        ],
    );
    let mut room = RoomSpec::new(OUTLANDER_ROOM_ID, world);
    room.metadata.mode = Some(OUTLANDER_EXPERIENCE.to_owned());
    room
}

/// Where the §transition gate stands on the lower floor, and where it delivers
/// the body on the upper ledge.
pub const GATE_ENTRY_X: f32 = 840.0;
pub const GATE_EXIT: ae::Vec2 = ae::Vec2::new(700.0, 180.0);

// ── §enemy (staging half) ───────────────────────────────────────────────────
fn sentry_spawn_requests(spawn: ae::Vec2) -> Vec<ambition::actors::features::SpawnActorRequest> {
    use ambition::actors::features::{ActorFaction, SpawnActorKind, SpawnActorRequest};
    vec![SpawnActorRequest {
        id: "outlander_sentry_0".to_string(),
        name: "Outlander Sentry".to_string(),
        pos: ae::Vec2::new(420.0, spawn.y),
        half_size: ae::Vec2::new(14.0, 16.0),
        faction: ActorFaction::Enemy,
        grudge_against: None,
        kind: SpawnActorKind::Enemy {
            brain: ambition::entity_catalog::placements::CharacterBrain::Custom(
                OUTLANDER_ENEMY_BRAIN_KEY.to_string(),
            ),
        },
    }]
}

pub fn install_outlander_content(app: &mut App) {
    use ambition::actors::features::{
        CharacterRosterAppExt, CharacterRosterFragment, RoomContentStagingRegistry,
    };
    use ambition::characters::actor::character_catalog::{
        CharacterCatalogAppExt, CharacterCatalogFragment,
    };

    app.register_character_catalog_fragment(
        CharacterCatalogFragment::from_ron(
            OUTLANDER_EXPERIENCE,
            Some(OUTLANDER_CHARACTER_ID),
            OUTLANDER_CATALOG_RON,
        )
        .expect("Outlander character catalog should be valid"),
    );
    app.register_character_roster_fragment(
        CharacterRosterFragment::from_ron(
            OUTLANDER_EXPERIENCE,
            None::<String>,
            OUTLANDER_ROSTER_RON,
        )
        .expect("Outlander roster fragment should be valid"),
    );
    app.init_resource::<RoomContentStagingRegistry>();
    app.world_mut()
        .resource_mut::<RoomContentStagingRegistry>()
        .register(
            OUTLANDER_ROOM_ID,
            "outlander",
            "sentry",
            "sentry-staging.v1",
            |spec| sentry_spawn_requests(spec.world.spawn),
        )
        .expect("sentry staging registration is unique");
    // DELIBERATE SILENCE, declared. Preparation validation refuses an
    // experience whose provider registered no explicit audio fragment
    // ("provider registered no explicit audio fragment" — a good message that
    // a headless host surfaced NOWHERE; recorded in the Phase-6 error-quality
    // account). The empty fragment is the declaration.
    {
        use ambition::audio::catalog::{AudioCatalogAppExt, AudioCatalogFragment};
        app.register_audio_catalog_fragment(
            AudioCatalogFragment::new(OUTLANDER_EXPERIENCE, None, None)
                .expect("the silent Outlander audio fragment is valid"),
        );
    }
}

// ── §transition ─────────────────────────────────────────────────────────────
/// The ridge gate: a body standing past `GATE_ENTRY_X` on the lower floor is
/// discretely relocated to the upper ledge through the engine's ONE transit
/// authority (`transit_body`, ADR 0024) — arrival at rest, contacts and
/// attachment reconciled, no pushout, no teleport hack.
pub fn ridge_gate_system(
    mut bodies: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition::actors::features::MotionModel,
        ),
        With<ambition::platformer::markers::PrimaryPlayer>,
    >,
) {
    for (clusters, mut model) in &mut bodies {
        let mut item = clusters;
        let mut clusters = item.as_clusters_mut();
        let pos = clusters.kinematics.pos;
        if pos.x >= GATE_ENTRY_X && pos.y > 300.0 {
            ae::movement::transit_body(
                &mut model,
                &mut clusters,
                GATE_EXIT,
                ae::movement::TransitVelocity::Zero,
            );
        }
    }
}

pub struct OutlanderExperiencePlugin;

impl Plugin for OutlanderExperiencePlugin {
    fn build(&self, app: &mut App) {
        install_outlander_content(app);
        // The §transition gate joins the SIM schedule through the same
        // schedule-extension seam engine plugins use — external code never
        // names a literal schedule, so the same system runs under the fixed
        // tick and a GGRS host alike.
        {
            use ambition::platformer::schedule::{SandboxSet, SimScheduleExt};
            let sim = app.sim_schedule();
            app.add_systems(sim, ridge_gate_system.in_set(SandboxSet::PlayerSimulation));
        }
        PlatformerExperienceAuthoring::new(
            OUTLANDER_EXPERIENCE,
            OUTLANDER_GAMEPLAY_ROUTE,
            "Outlander",
            "External-consumer architecture proof",
            "Prepare Outlander",
            AuthoredCatalogFragments::new(OUTLANDER_CHARACTER_ID, OUTLANDER_EXPERIENCE),
        )
        .install(app, outlander_prepared_session_world);
    }
}

// ── §host ───────────────────────────────────────────────────────────────────
/// Assemble Outlander under a standalone headless shell host, launched
/// DIRECTLY into the gameplay route — the same composition the in-repo
/// standalone demo shells use (`build_demo_app` in `ambition_demo_mary_o_app`):
/// foundation + engine + host + minimal shell + THIS crate's provider, an
/// initial route naming [`OUTLANDER_GAMEPLAY_ROUTE`], and a launcher home so
/// `QuitToHome` has somewhere to land. Zero engine edits.
///
/// The route wiring is load-bearing: `ShellHostConfiguration::default()`
/// carries `spec: None`, and a host that never names an initial route never
/// prepares or activates ANY experience — an earlier draft of the headless
/// binary "ran" 120 ticks of exactly that empty host (GPT 5.6 review finding).
pub fn build_outlander_app() -> App {
    use ambition::game_shell::{
        ShellHostConfiguration, ShellHostSpec, ShellLaunchCatalog, ShellRouteCatalog,
        ShellRouteSpec,
    };

    let mut app = App::new();
    ambition::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition::windowed_host::PlatformerHostPlugins);
    app.add_plugins(ambition::game_shell::MinimalShellPlugins);
    // The frontend audio context for launcher/loading frames. Outlander
    // authors no sounds, so the empty profile keeps those frames silent
    // rather than inheriting another provider's cached audio.
    app.insert_resource(ambition::audio::selection::FrontendAudioProfile::new(
        OUTLANDER_EXPERIENCE,
    ));
    app.add_plugins(ambition::load::AmbitionLoadPlugin);
    app.add_plugins(ambition::load_presentation::MinimalShellLoadPresentationPlugins);
    app.add_plugins(OutlanderExperiencePlugin);

    app.world_mut()
        .resource_mut::<ShellRouteCatalog>()
        .register(ShellRouteSpec::new(
            OUTLANDER_LAUNCHER_ROUTE,
            ShellLaunchCatalog::basic_experience_id(),
        ));
    app.world_mut()
        .resource_mut::<ShellHostConfiguration>()
        .spec = Some(ShellHostSpec::new(
        OUTLANDER_GAMEPLAY_ROUTE,
        OUTLANDER_LAUNCHER_ROUTE,
    ));

    // Pin the frame dt to the tick dt so one `update()` is exactly one sim tick.
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

/// What the acceptance walk proved, for the binary to print and tests to pin.
#[derive(Debug)]
pub struct OutlanderRunReport {
    /// Ticks from boot until the Outlander session was ACTIVE (room
    /// constructed, player + sentry present).
    pub ticks_to_activate: usize,
    /// Ticks of rightward walking until the ridge gate transited the player
    /// onto the upper ledge.
    pub ticks_to_gate: usize,
    /// Player position after the gate delivered it (upper-ledge coordinates).
    pub player_pos: ae::Vec2,
}

/// Boot-to-gate acceptance walk through the PUBLIC surface only: update until
/// the session activates, verify the constructed world (room identity, exactly
/// one player, the staged sentry), then hold right on the input seam until the
/// ridge gate transits the body onto the upper ledge. Errors name the first
/// broken claim so the binary and the integration test fail identically.
pub fn run_outlander_walkthrough(app: &mut App) -> Result<OutlanderRunReport, String> {
    use ambition::platformer::markers::PrimaryPlayer;
    use bevy::prelude::With;

    // 1. The session activates: the shell prepares the route, the provider's
    //    prepared world commits, and the room set publishes the ridge.
    let mut ticks_to_activate = None;
    for tick in 0..600 {
        app.update();
        let world = app.world_mut();
        let mut rooms = world.query::<&RoomSet>();
        let active = rooms
            .iter(world)
            .next()
            .map(|set| set.active_spec().id.clone());
        if active.as_deref() == Some(OUTLANDER_ROOM_ID) {
            ticks_to_activate = Some(tick + 1);
            break;
        }
    }
    let ticks_to_activate = ticks_to_activate.ok_or_else(|| {
        // Name where the shell actually got stuck — the difference between
        // "misconfigured route", "preparation never finished", and "activated
        // into the wrong room" is the whole diagnosis.
        let world = app.world_mut();
        let router = world
            .get_resource::<ambition::game_shell::ShellRouter>()
            .map(|router| {
                format!(
                    "initialized: {}, active route: {:?}, pending: {}, prepared session: {:?}",
                    router.is_initialized(),
                    router.active.as_ref().map(|active| active.route_id.clone()),
                    router.pending.is_some(),
                    router
                        .active
                        .as_ref()
                        .map(|active| active.prepared_session.is_some()),
                )
            })
            .unwrap_or_else(|| "<no ShellRouter resource>".to_string());
        let session = world
            .get_resource::<ambition::game_shell::ActiveGameplaySession>()
            .map(|session| format!("{:?}", session.0.is_some()))
            .unwrap_or_else(|| "<no ActiveGameplaySession resource>".to_string());
        let mut rooms = world.query::<&RoomSet>();
        let active_rooms: Vec<String> = rooms
            .iter(world)
            .map(|set| set.active_spec().id.clone())
            .collect();
        format!(
            "the Outlander session never activated in 600 ticks; \
             router: {router}; session active: {session}; room sets: {active_rooms:?}"
        )
    })?;

    // 2. The constructed world holds the authored population.
    {
        let world = app.world_mut();
        let mut players = world.query_filtered::<
            &ambition::platformer::body::BodyKinematics,
            With<PrimaryPlayer>,
        >();
        let player_count = players.iter(world).count();
        if player_count != 1 {
            return Err(format!(
                "expected exactly one primary player after activation, found {player_count}"
            ));
        }
        let mut actors = world.query::<&ambition::actors::features::ActorConfig>();
        if !actors.iter(world).any(|config| config.id == OUTLANDER_SENTRY_ID) {
            let present: Vec<String> =
                actors.iter(world).map(|config| config.id.clone()).collect();
            return Err(format!(
                "the staged sentry {OUTLANDER_SENTRY_ID:?} is missing; actors present: {present:?}"
            ));
        }
    }

    // 3. The ridge gate is load-bearing: hold right on the engine's input seam
    //    until `transit_body` delivers the body onto the upper ledge.
    let mut ticks_to_gate = None;
    for tick in 0..1200 {
        {
            let mut control = app
                .world_mut()
                .resource_mut::<ambition::input::ControlFrame>();
            *control = ambition::input::ControlFrame {
                axis_x: 1.0,
                ..Default::default()
            };
        }
        app.update();
        let world = app.world_mut();
        let mut players = world.query_filtered::<
            &ambition::platformer::body::BodyKinematics,
            With<PrimaryPlayer>,
        >();
        let pos = players
            .single(world)
            .map(|kin| kin.pos)
            .map_err(|error| format!("primary player lost mid-walk: {error}"))?;
        // The gate delivers to GATE_EXIT (700, 180); the body then settles on
        // the ledge (top y = 220). Anywhere in the upper half past the gate
        // column is proof of transit — the lower floor sits near y = 470.
        if pos.y < 300.0 {
            ticks_to_gate = Some(tick + 1);
            break;
        }
    }
    let ticks_to_gate = ticks_to_gate.ok_or_else(|| {
        "the player never reached the upper ledge — the ridge gate did not fire in 1200 ticks"
            .to_string()
    })?;

    let world = app.world_mut();
    let mut players = world.query_filtered::<
        &ambition::platformer::body::BodyKinematics,
        With<PrimaryPlayer>,
    >();
    let player_pos = players
        .single(world)
        .map(|kin| kin.pos)
        .map_err(|error| format!("primary player lost after the gate: {error}"))?;

    Ok(OutlanderRunReport {
        ticks_to_activate,
        ticks_to_gate,
        player_pos,
    })
}

/// The provider's authored source for the shared preparation lifecycle.
fn outlander_prepared_session_world() -> PreparedPlatformerSource {
    let room = outlander_room();
    let geometry = ae::RoomGeometry(room.world.clone());
    let metadata = ActiveRoomMetadata(room.metadata.clone());
    PreparedPlatformerSource::new(
        OUTLANDER_EXPERIENCE,
        RoomSet::from_parts(OUTLANDER_ROOM_ID, vec![room], Vec::new()),
        geometry,
        metadata,
        StartingCharacter::new(OUTLANDER_CHARACTER_ID),
        LdtkRuntimeIndex::default(),
    )
}
