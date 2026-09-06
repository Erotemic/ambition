//! Outlander: a tiny external game authored through `ambition_platformer2d`.
//!
//! It exercises one room, a playable character, an autonomous sentry, staged
//! construction, and an in-room transition using only the public facade. Any
//! engine-internal assumption required here is an SDK boundary defect.

use bevy::prelude::*;

use ambition_platformer2d::world::prelude::*;
use ambition_platformer2d::world::rooms::RoomSet;

/// This fixture's OWN asset tree — `fixtures/external_consumer/assets`.
///
/// Recorded SDK leak #3 said "consumer-owned art still has no home": a third
/// party could point the `AssetServer` at the ENGINE's tree or at nothing, so
/// its own sprites had nowhere to live. `layered_asset_source` is the answer,
/// and a fixture whose whole job is to be a third party has to exercise it.
///
/// Absolute, from this crate's manifest dir, because a consumer is built from
/// wherever it likes and a relative path would resolve against the process CWD.
pub fn outlander_asset_root() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/assets").to_string()
}

// `register_outlander_asset_source` stood here: a free function rather than a
// plugin, because it "must run BEFORE `AssetPlugin` builds — Bevy seals its
// sources there". That sentence is an engine rule, and a consumer holding it in
// a free function is a consumer being trusted to call it at the right moment.
//
// The source is DECLARED on `OutlanderModule::manifest` now, and the engine installs it at the only
// correct moment. See §host.

pub const OUTLANDER_EXPERIENCE: &str = "outlander";
pub const OUTLANDER_GAMEPLAY_ROUTE: &str = "outlander_gameplay";
pub const OUTLANDER_LAUNCHER_ROUTE: &str = "outlander_launcher";
pub const OUTLANDER_CHARACTER_ID: &str = "outlander_wanderer";
pub const OUTLANDER_ROOM_ID: &str = "outlander_ridge";
pub const OUTLANDER_ENEMY_BRAIN_KEY: &str = "outlander_sentry";
pub const OUTLANDER_SENTRY_CHARACTER_ID: &str = "outlander_sentry";
pub const OUTLANDER_SENTRY_ID: &str = "outlander_sentry_0";

// ── §character ────────────────────────────────────────────────────────────── LEAK #3 CLOSED, both
// halves. The ADDRESS half was the asset source plus a catalog pipeline that reduced every path to
// a basename under the engine's sprite folder; the DESCRIPTION half was sheet metadata living only
// in a table baked from the engine's own tree. Both are seams now: `game://` survives catalog
// assembly, and `register_character_sheet_ron` registers what the art looks like. The row below is
// what a third party writes.
/// The catalog this crate authors, exposed so a test can assemble it exactly as
/// the shell does rather than paraphrasing it — a fixture that proves a claim
/// about a paraphrase proves nothing about the game.
pub fn outlander_catalog_ron() -> &'static str {
    OUTLANDER_CATALOG_RON
}

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
            spritesheet: "game://sprites/outlander.png",
            manifest: "outlander_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "drifter",
            tags: ["player", "external_consumer"],
        ),
    },
)"#;

/// The sheet Outlander authors for its own character.
///
/// A catalog row says a character exists and names a sheet TARGET; this says what that sheet looks
/// like — frame size, rows, where the body sits. A consumer could address its own PNG and had no
/// way to describe it, so its character drew the placeholder rectangle whatever art it shipped.
///
/// One `idle` row, because the resolution rule is "a spec that maps Idle" and a
/// fixture should author the minimum that makes the claim true rather than a
/// convincing-looking sheet nobody reads.
pub const OUTLANDER_SHEET_RON: &str = r#"[
(
    target: "outlander",
    image: "game://sprites/outlander.png",
    label_width: 0,
    frame_width: 32,
    frame_height: 48,
    rows: [
    (
        animation: "idle",
        row_index: 0,
        frame_count: 1,
        duration_ms: 200,
        duration_secs: 0.2,
        rects: [ (x: 0, y: 0, w: 32, h: 48) ],
    ),
    ],
),
]"#;

// ── §enemy (character half) ─────────────────────────────────────────────────
//
// Outlander now demonstrates the public replacement from outside the workspace: body facts live
// on a CharacterDefinition, controller policy lives in BrainProfile, and the staged placement
// keeps its own respawn policy (EnemySpawnSpec's default is OnRoomReenter).

// ── §room ───────────────────────────────────────────────────────────────────
/// Two floors joined by the §transition gate: a lower ridge with the sentry,
/// and an upper ledge only the gate reaches (so the transition is load-bearing
/// for the fixture's acceptance walk, not decoration).
pub fn outlander_room() -> RoomSpec {
    let size = Vec2::new(960.0, 540.0);
    let floor_top = 492.0;
    let ledge_top = 220.0;
    let world = AuthoredWorld::new(
        "Outlander Ridge",
        size,
        Vec2::new(96.0, floor_top - 64.0),
        vec![
            Block::solid(
                "ridge_floor",
                Vec2::new(0.0, floor_top),
                Vec2::new(size.x, 48.0),
            ),
            Block::solid(
                "gate_ledge",
                Vec2::new(600.0, ledge_top),
                Vec2::new(280.0, 24.0),
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
pub const GATE_EXIT: Vec2 = Vec2::new(700.0, 180.0);

// ── §enemy (staging half) ───────────────────────────────────────────────────
fn sentry_spawn_requests(spawn: Vec2) -> Vec<ambition_platformer2d::actor::SpawnActorRequest> {
    use ambition_platformer2d::actor::{ActorFaction, SpawnActorKind, SpawnActorRequest};
    vec![SpawnActorRequest {
        id: "outlander_sentry_0".to_string(),
        name: "Outlander Sentry".to_string(),
        pos: Vec2::new(420.0, spawn.y),
        half_size: Vec2::new(14.0, 16.0),
        faction: ActorFaction::Enemy,
        grudge_against: None,
        kind: SpawnActorKind::Enemy {
            brain: ambition_platformer2d::character::CharacterBrain::Custom(
                OUTLANDER_ENEMY_BRAIN_KEY.to_string(),
            ),
            // The controller key above is placement vocabulary; it is not a second source of
            // body facts.
            character: ambition_platformer2d::character::CharacterId::new(
                OUTLANDER_SENTRY_CHARACTER_ID,
            ),
        },
    }]
}

pub fn install_outlander_content(app: &mut App) {
    use ambition_platformer2d::actor::RoomContentStagingRegistry;
    use ambition_platformer2d::character::{CharacterCatalogAppExt, CharacterCatalogFragment};

    app.register_character_catalog_fragment(
        CharacterCatalogFragment::from_ron_at(
            "fixtures/external_consumer/src/lib.rs:OUTLANDER_CATALOG_RON",
            OUTLANDER_EXPERIENCE,
            Some(OUTLANDER_CHARACTER_ID),
            OUTLANDER_CATALOG_RON,
        )
        .expect("Outlander character catalog should be valid"),
    );
    // The sheet half of authoring a character. Same shape as the catalog
    // fragment above and deliberately so: a provider says WHO its characters are
    // and WHAT THEY LOOK LIKE through two registrations, neither of which
    // requires touching the engine's asset tree.
    {
        use ambition_platformer2d::character::AuthoredSheetAppExt;
        app.register_character_sheet_ron("outlander", OUTLANDER_SHEET_RON);
    }
    // The character-DEFINITION seam, exercised from outside the workspace.
    //
    // Outlander authors an EMPTY action set, deliberately, and it is the harder
    // half of the claim rather than the lazy one. `Some(empty)` means "this
    // character reaches for nothing" and must outrank the catalog exactly as a
    // filled set would; a resolver that treated it as "unauthored" would fall
    // through to the row — whose `default_action_set: "drifter"` is a kit this
    // character did not ask for — and hand a third party's wanderer a weapon.
    //
    // This RON kept the field for six days and NOTHING SAID SO: a catalog fragment is parsed at
    // runtime, so `cargo check` on this fixture stayed green while every test that boots it
    // panicked on an unknown field. The fall-through this block guards is unchanged and still real
    // — the row's own `drifter` set — and it never depended on the deleted field.
    {
        use ambition_platformer2d::character::{
            ActionSet, BrainProfile, CharacterBrainTemplate, CharacterDefinition,
            CharacterDefinitionAppExt, CharacterLocomotion, ContactDamage, MoveStyleSpec,
        };

        app.register_character(
            CharacterDefinition::new(OUTLANDER_CHARACTER_ID, "Outlander", OUTLANDER_EXPERIENCE)
                .with_sheet("outlander")
                .with_action_set(ActionSet::default()),
        );

        // Body:      max_health, run_speed, move_style, contact damage.
        // Controller: Wanderer + effort/radius policy.
        // Placement: OnRoomReenter is EnemySpawnSpec's named default.
        //
        // This is deliberately a SECOND character rather than pretending the
        // sentry's brain key is a body identity. A third-party author can now
        // state the same creature entirely through the supported umbrella API.
        let mut sentry = CharacterDefinition::new(
            OUTLANDER_SENTRY_CHARACTER_ID,
            "Outlander Sentry",
            OUTLANDER_EXPERIENCE,
        )
        .with_sheet("outlander")
        .with_action_set(ActionSet::default())
        .with_locomotion(CharacterLocomotion {
            run_speed: 38.0,
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_contact_damage(ContactDamage {
            strength: 0.5,
            amount: 1,
        })
        .with_autonomous_profile(BrainProfile {
            template: CharacterBrainTemplate::Wanderer,
            aggro_radius: 0.0,
            attack_range: 0.0,
            patrol_effort: 1.0,
            chase_effort: 1.0,
            ..Default::default()
        });
        sentry.vitals.max_health = Some(2);
        app.register_character(sentry);
    }
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
    // Both halves are closed now. The refusal is legible (slice C gave it
    // `HostStatus::Refused`), and the declaration is a word on the draft:
    // `no_audio()`. See §host.
}

// ── §authority ──────────────────────────────────────────────────────────────
// Task 1's exit criterion, from the only place that can actually test it: *"a
// feature-owned authoritative component and system are mechanically accounted,
// run under the simulation gate, and survive real rewind/resimulation without
// edits to a giant runtime list."* Everything below is authored in the CONSUMER
// crate and reaches the engine only through `ambition_platformer2d::runtime::rollback`.
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
    /// Ticks the body has spent inside the field.
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
impl ambition_platformer2d::rollback::SnapshotState for BeaconCharge {
    fn encode(&self, out: &mut Vec<u8>) {
        ambition_platformer2d::rollback::put_f32(out, self.seconds);
        ambition_platformer2d::rollback::put_u32(out, self.ticks);
    }

    fn decode(reader: &mut ambition_platformer2d::rollback::Reader<'_>) -> Option<Self> {
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
    time: Res<ambition_platformer2d::sim::WorldTime>,
    mut bodies: Query<
        (
            &ambition_platformer2d::actor::BodyKinematics,
            &mut BeaconCharge,
        ),
        With<ambition_platformer2d::actor::PrimaryPlayer>,
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
            With<ambition_platformer2d::actor::PrimaryPlayer>,
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
            ambition_platformer2d::actor::BodyClusterQueryData,
            &mut ambition_platformer2d::actor::MotionModel,
            Option<&BeaconCharge>,
        ),
        With<ambition_platformer2d::actor::PrimaryPlayer>,
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
            ambition_platformer2d::actor::transit_body(
                &mut model,
                &mut clusters,
                GATE_EXIT,
                ambition_platformer2d::actor::TransitVelocity::Zero,
            );
        }
    }
}

#[derive(Clone)]
pub struct OutlanderExperiencePlugin;

impl Plugin for OutlanderExperiencePlugin {
    fn build(&self, app: &mut App) {
        install_outlander_content(app);
        // The §transition gate joins the SIM schedule through the same
        // schedule-extension seam engine plugins use — external code never
        // names a literal schedule, so the same system runs under the fixed
        // tick and a GGRS host alike.
        {
            use ambition_platformer2d::sim::{Platformer2dSimulationPhaseMonolith, SimScheduleExt};
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
                    .in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation),
            );
        }
        // Consumer-owned authoritative state joins rollback through the public
        // registrar; fixed-tick hosts intentionally treat this as a no-op.
        {
            use ambition_platformer2d::rollback::AmbitionRollbackApp;
            app.rollback_component_canonical::<BeaconCharge>(
                "outlander::beacon",
                "outlander.beacon_charge",
            );
        }
    }
}

// ── §host ───────────────────────────────────────────────────────────────────
/// Outlander declares only consumer-owned policy: identity, assets, routes,
/// room, and content. Engine composition and ordering stay in
/// `ambition_platformer2d::app`.
#[derive(Default)]
pub struct OutlanderModule;

impl ambition_platformer2d::app::GameModule for OutlanderModule {
    fn manifest(&self) -> ambition_platformer2d::app::ModuleManifest {
        ambition_platformer2d::app::ModuleManifest::new(OUTLANDER_EXPERIENCE).asset_source(
            // Prefer consumer-owned art, then fall back to the engine asset tree.
            ambition_platformer2d::app::AssetSource::at("game", outlander_asset_root()),
        )
    }

    fn define(&self, module: &mut ambition_platformer2d::app::ModuleDraft) {
        module
            .experience(OUTLANDER_EXPERIENCE)
            .launcher_route(OUTLANDER_LAUNCHER_ROUTE)
            .gameplay_route(OUTLANDER_GAMEPLAY_ROUTE)
            .room(outlander_room().metadata)
            // Declared silence, on the draft — Outlander authors no sound and
            // now has a word for saying so.
            .no_audio()
            .playable(
                "Outlander",
                "External-consumer architecture proof",
                OUTLANDER_CHARACTER_ID,
                OUTLANDER_ROOM_ID,
                vec![outlander_room()],
            )
            .capability(OutlanderExperiencePlugin);
    }
}

/// Outlander under a headless fixed-tick host. One `update()` is one sim tick.
pub fn build_outlander_app() -> App {
    ambition_platformer2d::app::PlatformerApp::headless()
        .mount(OutlanderModule)
        .build()
}

/// Build the same Outlander composition under a GGRS sync-test host.
///
/// Switching hosts is a builder choice; the consumer game does not restructure
/// itself for rollback. GGRS owns rollback startup ordering and checksum-based
/// resimulation. The host also owns the exact simulation tick duration so fixed
/// and rollback compositions advance the same timeline.
pub fn build_outlander_rollback_app() -> Result<App, String> {
    // Rollback is a supported session mode as of slice F, and Outlander says
    // how many people are playing — one. The count is declared HERE, at
    // composition, so a restart reuses it instead of re-sampling live devices.
    let mut app = ambition_platformer2d::app::PlatformerApp::headless()
        .rollback(1)
        .mount(OutlanderModule)
        .build();

    // `ambition_platformer2d::rollback::start` owns rollback startup ordering so
    // consumers cannot construct invalid startup sequences.
    ambition_platformer2d::rollback::start(
        &mut app,
        ambition_platformer2d::rollback::RollbackPlan::new(),
    )
    .map_err(|refused| format!("Outlander could not start rollback: {refused}"))?;
    Ok(app)
}

/// The window title the visible binary and the render test share.
pub const OUTLANDER_WINDOW_TITLE: &str = "Outlander — external consumer proof";

/// Outlander, drawn. The composition `src/bin/visible.rs` runs and the
/// render test observes — one function, so they cannot drift.
///
/// `gpu: false` builds the full render graph against no wgpu backend, which is
/// how a test asserts "the consumer's character is DRAWN" on CI that has none.
/// It is one builder call and nothing else — if it needed a second difference,
/// the test would be observing a composition nothing ships.
#[cfg(feature = "visible")]
pub fn build_windowed_app(gpu: bool) -> App {
    let composed = ambition_platformer2d::app::PlatformerApp::windowed(OUTLANDER_WINDOW_TITLE);
    let composed = if gpu {
        composed
    } else {
        composed.without_gpu()
    };
    composed.mount(OutlanderModule).build()
}

/// Drive one frame of input, through the engine's own driver seam.
///
/// That is a rule the engine can state once, and now does
/// (`ambition_platformer2d::sim::drive_control_frame`).
///
/// Kept as a one-line wrapper rather than deleted, because the binaries and the
/// walkthrough all call it and the name says what it is FOR.
pub fn drive_control_frame(app: &mut App, frame: ambition_platformer2d::sim::ControlFrame) {
    ambition_platformer2d::sim::drive_control_frame(app.world_mut(), frame);
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
    pub player_pos: Vec2,
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
    use ambition_platformer2d::actor::PrimaryPlayer;
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
            .query_filtered::<&ambition_platformer2d::actor::BodyKinematics, With<PrimaryPlayer>>();
        let player_count = players.iter(world).count();
        if player_count != 1 {
            return Err(format!(
                "expected exactly one primary player after activation, found {player_count}"
            ));
        }
        let mut actors = world.query::<&ambition_platformer2d::actor::ActorConfig>();
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
        let status = ambition_platformer2d::app::host_status(app);
        let router = format!("{status:?}");
        let session = match &status {
            ambition_platformer2d::app::HostStatus::Running { prepared, .. } => {
                prepared.to_string()
            }
            _ => "no active session".to_string(),
        };
        let world = app.world_mut();
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
    use ambition_platformer2d::actor::PrimaryPlayer;
    use bevy::prelude::With;

    // 3. The ridge gate is load-bearing: hold right on the engine's input seam
    //    until `transit_body` delivers the body onto the upper ledge.
    let mut ticks_to_gate = None;
    for tick in 0..1200 {
        drive_control_frame(
            app,
            ambition_platformer2d::sim::ControlFrame {
                axis_x: 1.0,
                ..Default::default()
            },
        );
        app.update();
        let world = app.world_mut();
        let mut players = world
            .query_filtered::<&ambition_platformer2d::actor::BodyKinematics, With<PrimaryPlayer>>();
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
        &ambition_platformer2d::actor::BodyKinematics,
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

// The provider's authored source for the shared preparation lifecycle.
// `outlander_prepared_session_world` stood here: RoomSet, RoomGeometry,
// ActiveRoomMetadata, StartingCharacter and LdtkRuntimeIndex assembled by hand
// out of `ambition_platformer2d::runtime::demo_fixture`. A module named `demo_fixture` in a
// shipped game's imports was the namespace mirror confessing.
