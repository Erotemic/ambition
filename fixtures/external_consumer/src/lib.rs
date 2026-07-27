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

// ── §authority ──────────────────────────────────────────────────────────────
// Task 1's exit criterion, from the only place that can actually test it: *"a
// feature-owned authoritative component and system are mechanically accounted,
// run under the simulation gate, and survive real rewind/resimulation without
// edits to a giant runtime list."* Everything below is authored in the CONSUMER
// crate and reaches the engine only through `ambition::runtime::rollback`.
//
// The engine's own 246 registrations do live in one function. That is not what
// the criterion is about: the question is whether a game the engine has never
// heard of can put authoritative state into the simulation, and the honest place
// to answer it is outside the workspace, where a forgotten `pub` is a compile
// error rather than a crate-private convenience.

/// Charge accumulated by standing in the beacon's field. AUTHORITATIVE: the
/// ridge gate refuses to fire until it is full, so a value that fails to rewind
/// is a gate that opens on a frame the real timeline never had.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct BeaconCharge {
    /// Seconds of contact accumulated, saturating at [`BEACON_FULL_SECONDS`].
    pub seconds: f32,
    /// Ticks the body has spent inside the field. A second field on purpose: an
    /// encoder that round-trips one member and drops the other is a real bug
    /// class, and a single-field state cannot catch it.
    pub ticks: u32,
}

/// Contact seconds required before the ridge gate will transit a body.
pub const BEACON_FULL_SECONDS: f32 = 0.25;
/// The field's left edge. A body walking right crosses it well before the gate,
/// so the ordinary acceptance walk charges the beacon on its way past.
pub const BEACON_FIELD_X: f32 = 700.0;

impl BeaconCharge {
    pub fn is_full(&self) -> bool {
        self.seconds >= BEACON_FULL_SECONDS
    }
}

// The snapshot codec, written by the consumer for the consumer's own type. The
// engine cannot supply this: it has never seen `BeaconCharge`. `put_f32`
// canonicalizes NaN so two peers that computed the same value byte-agree.
impl ambition::runtime::rollback::SnapshotState for BeaconCharge {
    fn encode(&self, out: &mut Vec<u8>) {
        ambition::runtime::rollback::put_f32(out, self.seconds);
        ambition::runtime::rollback::put_u32(out, self.ticks);
    }

    fn decode(reader: &mut ambition::runtime::rollback::Reader<'_>) -> Option<Self> {
        Some(Self {
            seconds: reader.f32()?,
            ticks: reader.u32()?,
        })
    }
}

/// Charge the beacon while the primary body stands in its field.
///
/// On the SIM clock (`WorldTime::scaled_dt`), never `Res<Time>`: a resimulated
/// tick must add exactly what the original tick added, and wall-clock dt does
/// not repeat.
pub fn beacon_charge_system(
    time: Res<ambition::time::WorldTime>,
    mut bodies: Query<
        (
            &ambition::platformer::body::BodyKinematics,
            &mut BeaconCharge,
        ),
        With<ambition::platformer::markers::PrimaryPlayer>,
    >,
) {
    for (kin, mut charge) in &mut bodies {
        if kin.pos.x < BEACON_FIELD_X {
            continue;
        }
        charge.seconds = (charge.seconds + time.scaled_dt).min(BEACON_FULL_SECONDS);
        charge.ticks += 1;
    }
}

/// Give the primary body its beacon charge the tick it appears.
///
/// A separate system rather than a spawn-time bundle because the body is built
/// by the ENGINE's construction path from the character catalog — a consumer
/// adding authoritative state to an engine-constructed entity is the case worth
/// proving, and it is strictly harder than authoring its own entity.
pub fn attach_beacon_charge(
    mut commands: Commands,
    bodies: Query<
        Entity,
        (
            With<ambition::platformer::markers::PrimaryPlayer>,
            Without<BeaconCharge>,
        ),
    >,
) {
    for body in &bodies {
        commands.entity(body).insert(BeaconCharge::default());
    }
}

// ── §transition ─────────────────────────────────────────────────────────────
/// The ridge gate: a body standing past `GATE_ENTRY_X` on the lower floor is
/// discretely relocated to the upper ledge through the engine's ONE transit
/// authority (`transit_body`, ADR 0024) — arrival at rest, contacts and
/// attachment reconciled, no pushout, no teleport hack.
/// The gate is GATED on [`BeaconCharge`], which is what makes that component
/// authoritative rather than decorative: a charge that failed to rewind would
/// open the gate on a frame the real timeline never had, and the position it
/// produces is the thing the acceptance walk asserts on.
pub fn ridge_gate_system(
    mut bodies: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition::actors::features::MotionModel,
            Option<&BeaconCharge>,
        ),
        With<ambition::platformer::markers::PrimaryPlayer>,
    >,
) {
    for (clusters, mut model, charge) in &mut bodies {
        if !charge.is_some_and(BeaconCharge::is_full) {
            continue;
        }
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
            app.add_systems(
                sim,
                // §authority, then §transition — the gate reads what the beacon
                // wrote this tick, so the order is a real dependency and not a
                // preference. Both in `PlayerSimulation`, the engine's semantic
                // phase for post-input body authority; no leaf system of the
                // engine's is named.
                (
                    attach_beacon_charge,
                    beacon_charge_system,
                    ridge_gate_system,
                )
                    .chain()
                    .in_set(SandboxSet::PlayerSimulation),
            );
        }
        // The consumer's own authoritative state joins the rollback contract
        // through the public vocabulary. No engine file lists `BeaconCharge`;
        // nothing in `ambition` could, because nothing in `ambition` has heard
        // of it. `rollback_component_canonical` is a no-op on a fixed-tick host
        // by design (the registration vocabulary gates installation on host
        // kind), so this one line is correct for BOTH Outlander hosts.
        {
            use ambition::runtime::rollback::AmbitionRollbackApp;
            app.rollback_component_canonical::<BeaconCharge>(
                "outlander::beacon",
                "outlander.beacon_charge",
            );
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
    let mut app = App::new();
    ambition::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition::windowed_host::PlatformerHostPlugins);
    compose_outlander_shell(&mut app);

    // Pin the frame dt to the tick dt so one `update()` is exactly one sim tick.
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

/// The SAME game under a GGRS rollback host, with a sync-test session.
///
/// Sync test is the strongest available proof and the cheapest to run: GGRS
/// resimulates every frame `check_distance` times from a restored snapshot and
/// compares checksums itself, so a divergence is a panic inside the engine
/// rather than an assertion this fixture had to invent. If `BeaconCharge` did
/// not round-trip, or its encoder dropped a field, or the charge system read
/// wall-clock time, that comparison is what notices.
///
/// The host switch is one line versus [`build_outlander_app`]. That is the
/// claim: a consumer does not restructure its game to become rollback-capable.
pub fn build_outlander_rollback_app() -> Result<App, String> {
    let mut app = App::new();
    ambition::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition::engine::PlatformerEnginePlugins::rollback());
    app.add_plugins(ambition::windowed_host::PlatformerHostPlugins);
    compose_outlander_shell(&mut app);

    // Under GGRS the sim advances only through session requests, so the frame dt
    // must be the tick dt exactly (integer nanos, no drift).
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_nanos(1_000_000_000u64 / ambition::runtime::SIM_TICK_HZ as u64),
    ));
    // Boot and ACTIVATE before the session exists. This ordering is the whole
    // lesson of the first draft, which started the session on update #1 and then
    // watched GGRS report a checksum mismatch on frames 2, 3 and 4 forever: the
    // shell's preparation and the session-world commit build the room and the
    // body through `Commands`, and a rollback cannot undo construction. Rewinding
    // across it is a guaranteed divergence, and the sync test says so immediately
    // — correctly. `start_sync_test_session` rebases onto the CURRENT live world
    // as frame zero, so the fix is to let construction finish first.
    activate_outlander(&mut app)?;
    // A few settled frames past activation, for the same reason: activation
    // completing is not the same fact as the tick after it being quiet.
    for _ in 0..8 {
        app.update();
    }
    ambition::runtime::rollback::start_sync_test_session(
        app.world_mut(),
        ambition::runtime::rollback::SyncTestSettings {
            check_distance: 4,
            max_prediction_window: 10,
        },
    )
    .map_err(|error| format!("failed to start the Outlander sync-test session: {error}"))?;
    app.update();
    Ok(app)
}

/// The shell wiring the headless and visible hosts SHARE — one provider, one
/// route table, one session lifecycle, exactly the "visibly and headlessly
/// from the same content" claim.
pub fn compose_outlander_shell(app: &mut App) {
    use ambition::game_shell::{
        ShellHostConfiguration, ShellHostSpec, ShellLaunchCatalog, ShellRouteCatalog,
        ShellRouteSpec,
    };

    app.add_plugins(ambition::game_shell::MinimalShellPlugins);
    // The frontend audio context for launcher/loading frames. Outlander
    // authors no sounds, so the empty profile keeps those frames silent
    // rather than inheriting another provider's cached audio.
    app.insert_resource(ambition::audio::selection::FrontendAudioProfile::new(
        OUTLANDER_EXPERIENCE,
    ));
    // `AmbitionLoadPlugin` is NOT added here: `PlatformerEnginePlugins` supplies
    // it (the room-transition transaction IS a load plan), and a second copy is a
    // hard Bevy panic. The in-repo shells carry the same note.
    //
    // LEAK (recorded): the engine group's guarded `add_plugins` makes "who owes
    // the load coordinator" an undocumented composition rule. Both in-repo demos
    // were edited when it moved; this fixture — outside the workspace, invisible
    // to a repo grep — was not, and sat red until someone read the panic.
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
}

/// Drive one frame of input, whichever host is running.
///
/// LEAK (recorded, Phase-6 evidence): a consumer that wants to run its game
/// under both hosts must know TWO input seams. A fixed-tick host consumes the
/// `ControlFrame` resource directly; a GGRS host consumes
/// `PendingLocalInput`, because the frame it simulates is the one the session
/// confirmed and not the one the device produced. Writing `ControlFrame` under
/// GGRS is silently ignored — the walk still runs, the body never moves, and
/// nothing says why. Detected by resource presence rather than by a host flag
/// the consumer would have to thread everywhere, but the underlying asymmetry
/// is the engine's and should be a single seam.
pub fn drive_control_frame(app: &mut App, frame: ambition::input::ControlFrame) {
    let world = app.world_mut();
    if let Some(mut pending) =
        world.get_resource_mut::<ambition::runtime::rollback::PendingLocalInput>()
    {
        pending.0 = frame;
    } else if let Some(mut control) = world.get_resource_mut::<ambition::input::ControlFrame>() {
        *control = frame;
    }
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
    /// The consumer-owned authoritative state at the end of the walk. Reported
    /// rather than merely asserted so a rollback run can compare it against the
    /// fixed-tick run: the two hosts must produce the same number, and a report
    /// that only said "the gate fired" could not tell them apart.
    pub beacon: BeaconCharge,
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
    //    prepared world commits, and the room set publishes the ridge. Already
    //    done under the rollback host, which has to activate BEFORE it starts a
    //    session; this returns 1 there and re-proves nothing, which is correct.
    let ticks_to_activate = activate_outlander(app)?;

    // 2. The constructed world holds the authored population.
    {
        let world = app.world_mut();
        let mut players = world
            .query_filtered::<&ambition::platformer::body::BodyKinematics, With<PrimaryPlayer>>();
        let player_count = players.iter(world).count();
        if player_count != 1 {
            return Err(format!(
                "expected exactly one primary player after activation, found {player_count}"
            ));
        }
        let mut actors = world.query::<&ambition::actors::features::ActorConfig>();
        if !actors
            .iter(world)
            .any(|config| config.id == OUTLANDER_SENTRY_ID)
        {
            let present: Vec<String> = actors.iter(world).map(|config| config.id.clone()).collect();
            return Err(format!(
                "the staged sentry {OUTLANDER_SENTRY_ID:?} is missing; actors present: {present:?}"
            ));
        }
    }

    walk_outlander_to_the_ledge(app, ticks_to_activate)
}

/// Update until the Outlander session is active, returning the tick it landed
/// on. Split out of the walkthrough because the rollback host must complete
/// construction BEFORE it starts a session, and running the same loop twice is
/// cheaper than two versions of it drifting apart.
pub fn activate_outlander(app: &mut App) -> Result<usize, String> {
    for tick in 0..600 {
        app.update();
        let world = app.world_mut();
        let mut rooms = world.query::<&RoomSet>();
        let active = rooms
            .iter(world)
            .next()
            .map(|set| set.active_spec().id.clone());
        if active.as_deref() == Some(OUTLANDER_ROOM_ID) {
            return Ok(tick + 1);
        }
    }
    Err({
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
    })
}

/// Hold right on the engine's input seam until the ridge gate delivers the body
/// to the upper ledge, then report what the walk produced.
fn walk_outlander_to_the_ledge(
    app: &mut App,
    ticks_to_activate: usize,
) -> Result<OutlanderRunReport, String> {
    use ambition::platformer::markers::PrimaryPlayer;
    use bevy::prelude::With;

    // 3. The ridge gate is load-bearing: hold right on the engine's input seam
    //    until `transit_body` delivers the body onto the upper ledge.
    let mut ticks_to_gate = None;
    for tick in 0..1200 {
        drive_control_frame(
            app,
            ambition::input::ControlFrame {
                axis_x: 1.0,
                ..Default::default()
            },
        );
        app.update();
        let world = app.world_mut();
        let mut players = world
            .query_filtered::<&ambition::platformer::body::BodyKinematics, With<PrimaryPlayer>>();
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
    let mut players = world.query_filtered::<(
        &ambition::platformer::body::BodyKinematics,
        Option<&BeaconCharge>,
    ), With<PrimaryPlayer>>();
    let (player_pos, beacon) = players
        .single(world)
        .map(|(kin, charge)| (kin.pos, charge.copied()))
        .map_err(|error| format!("primary player lost after the gate: {error}"))?;
    // The gate cannot have fired without it, so this is a premise check rather
    // than a discovery — but a missing component here would mean the gate opened
    // through some other path and the walk proved nothing about §authority.
    let beacon = beacon.ok_or_else(|| {
        "the player reached the ledge with no BeaconCharge, so the gate did not \
         open through the consumer's authoritative state"
            .to_string()
    })?;

    Ok(OutlanderRunReport {
        ticks_to_activate,
        ticks_to_gate,
        player_pos,
        beacon,
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
