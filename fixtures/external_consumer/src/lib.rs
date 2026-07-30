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

use ambition::world::prelude::*;
use ambition::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition::runtime::demo_fixture::{
    ActiveRoomMetadata, LdtkRuntimeIndex, RoomSet, StartingCharacter,
};
use ambition::runtime::PreparedPlatformerSource;

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
// DELETED 2026-07-30. The source is DECLARED on `OutlanderModule::manifest`
// now, and the engine installs it at the only correct moment. See §host.

pub const OUTLANDER_EXPERIENCE: &str = "outlander";
pub const OUTLANDER_GAMEPLAY_ROUTE: &str = "outlander_gameplay";
pub const OUTLANDER_LAUNCHER_ROUTE: &str = "outlander_launcher";
pub const OUTLANDER_CHARACTER_ID: &str = "outlander_wanderer";
pub const OUTLANDER_ROOM_ID: &str = "outlander_ridge";
pub const OUTLANDER_ENEMY_BRAIN_KEY: &str = "outlander_sentry";
pub const OUTLANDER_SENTRY_ID: &str = "outlander_sentry_0";

// ── §character ──────────────────────────────────────────────────────────────
// LEAK #3 CLOSED, both halves (2026-07-28). This used to read "reuses an
// engine-shipped spritesheet on purpose: consumer-owned art has no home" — and
// that was two separate gaps wearing one sentence. The ADDRESS half was the
// asset source plus a catalog pipeline that reduced every path to a basename
// under the engine's sprite folder; the DESCRIPTION half was sheet metadata
// living only in a table baked from the engine's own tree. Both are seams now:
// `game://` survives catalog assembly, and `register_character_sheet_ron`
// registers what the art looks like. The row below is what a third party writes.
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
            playable_kit: HostCode,
            tags: ["player", "external_consumer"],
        ),
    },
)"#;

/// **The sheet Outlander authors for its own character.** (queue U1)
///
/// A catalog row says a character exists and names a sheet TARGET; this says
/// what that sheet looks like — frame size, rows, where the body sits. Until
/// 2026-07-28 the second half was expressible only by putting a RON in the
/// ENGINE's asset tree and rebuilding the engine, which a third party cannot
/// do: `manifest_target()` strips `_spritesheet.ron` to a name and that name was
/// looked up in a table baked from `crates/ambition_actors/assets/sprites`. A
/// consumer could address its own PNG and had no way to describe it, so its
/// character drew the placeholder rectangle whatever art it shipped.
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

// ── §enemy (archetype half) ─────────────────────────────────────────────────
const OUTLANDER_ROSTER_RON: &str = r#"{
    "outlander_sentry": (
        max_health: 2,
        run_speed: 38.0,
        patrol_effort: 1.0,
        chase_effort: 1.0,
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
fn sentry_spawn_requests(spawn: Vec2) -> Vec<ambition::actor::SpawnActorRequest> {
    use ambition::actor::{ActorFaction, SpawnActorKind, SpawnActorRequest};
    vec![SpawnActorRequest {
        id: "outlander_sentry_0".to_string(),
        name: "Outlander Sentry".to_string(),
        pos: Vec2::new(420.0, spawn.y),
        half_size: Vec2::new(14.0, 16.0),
        faction: ActorFaction::Enemy,
        grudge_against: None,
        kind: SpawnActorKind::Enemy {
            brain: ambition::character::CharacterBrain::Custom(
                OUTLANDER_ENEMY_BRAIN_KEY.to_string(),
            ),
        },
    }]
}

pub fn install_outlander_content(app: &mut App) {
    use ambition::actor::{CharacterRosterFragment, RoomContentStagingRegistry};
    use ambition::character::CharacterRosterAppExt;
    use ambition::character::{
        CharacterCatalogAppExt, CharacterCatalogFragment,
    };

    // `from_ron_at`, not `from_ron`: these two constants are this crate's
    // authored content, and when one of them is wrong the message a stranger
    // reads should say WHERE. The seam took an anonymous `&str` until
    // 2026-07-28, so no diagnostic could name a file however hard it tried.
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
    // requires touching the engine's asset tree (queue U1).
    {
        use ambition::character::AuthoredSheetAppExt;
        app.register_character_sheet_ron("outlander", OUTLANDER_SHEET_RON);
    }
    // **The character-DEFINITION seam, exercised from outside the workspace.**
    //
    // The catalog fragment above says a character exists; this says what it can
    // DO, and since 2026-07-28 an authored value here outranks the row (queue
    // C3). Until now every caller of that seam was in-workspace — the two arena
    // duelists — which makes it a claim about this repo rather than about an
    // engine. The keystone rule applies: an engine claim nobody outside proves
    // is not an engine claim.
    //
    // Outlander authors an EMPTY action set, deliberately, and it is the harder
    // half of the claim rather than the lazy one. `Some(empty)` means "this
    // character reaches for nothing" and must outrank the catalog exactly as a
    // filled set would; a resolver that treated it as "unauthored" would fall
    // through to the row — whose `playable_kit: HostCode` rebuilds the HOST
    // protagonist's kit — and hand a third party's wanderer Ambition's sword.
    {
        use ambition::character::{CharacterDefinition, CharacterDefinitionAppExt};
        app.register_character(
            CharacterDefinition::new(
                OUTLANDER_CHARACTER_ID,
                "Outlander",
                OUTLANDER_EXPERIENCE,
            )
            .with_sheet("outlander")
            .with_action_set(ambition::character::ActionSet::default()),
        );
    }
    app.register_character_roster_fragment(
        CharacterRosterFragment::from_ron_at(
            "fixtures/external_consumer/src/lib.rs:OUTLANDER_ROSTER_RON",
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
            &ambition::actor::BodyKinematics,
            &mut BeaconCharge,
        ),
        With<ambition::actor::PrimaryPlayer>,
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
            With<ambition::actor::PrimaryPlayer>,
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
            ambition::actor::BodyClusterQueryData,
            &mut ambition::actor::MotionModel,
            Option<&BeaconCharge>,
        ),
        With<ambition::actor::PrimaryPlayer>,
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
            ambition::actor::transit_body(
                &mut model,
                &mut clusters,
                GATE_EXIT,
                ambition::actor::TransitVelocity::Zero,
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
            use ambition::sim::{SandboxSet, SimScheduleExt};
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
/// **Outlander, declared.** Everything the engine needs to stand this game up.
///
/// LEAK CLOSED 2026-07-30 — slice A of the API 1.0 campaign, and this fixture is
/// why it was found. What stood here was three hand-ordered compositions
/// totalling ~110 lines: `build_outlander_app`, `build_outlander_rollback_app`
/// and `build_windowed_app`, plus `compose_outlander_shell` and
/// `register_outlander_asset_source` for them to share. Between them they
/// encoded EIGHT engine ordering rules, of which four failed silently:
///
/// > the consumer's asset source registers before `DefaultPlugins` (Bevy seals
/// > its sources when `AssetPlugin` builds); `AssetPlugin.file_path` is the
/// > engine's own root; a GPU-less window needs five specific disables;
/// > `init_engine_states` before the engine groups; engine before host before
/// > shell; `PlatformerAssetsPlugin` after the content that registers the
/// > catalogs it reads and before the presentation that draws them; a host that
/// > names no initial route prepares nothing; manual stepping pins the frame dt
/// > to the tick dt read back out of the world.
///
/// A third party had to get all eight right by reading two in-repo demos. They
/// are the engine's rules and the engine states them once now, in
/// `ambition::app`. What is left here is what a consumer should still be
/// DECIDING: its id, its asset tree, its routes, its room, its content.
#[derive(Default)]
pub struct OutlanderModule;

impl ambition::app::GameModule for OutlanderModule {
    fn manifest(&self) -> ambition::app::ModuleManifest {
        ambition::app::ModuleManifest::new(OUTLANDER_EXPERIENCE).asset_source(
            // This fixture's OWN art first, the engine's tree for everything it
            // did not author. Recorded SDK leak #3 said "consumer-owned art has
            // no home"; a fixture whose whole job is to be a third party has to
            // exercise the answer.
            ambition::app::AssetSource::at("game", outlander_asset_root()),
        )
    }

    fn define(&self, module: &mut ambition::app::ModuleDraft) {
        module
            .experience(OUTLANDER_EXPERIENCE)
            .launcher_route(OUTLANDER_LAUNCHER_ROUTE)
            .gameplay_route(OUTLANDER_GAMEPLAY_ROUTE)
            .room(outlander_room().metadata)
            .capability(OutlanderExperiencePlugin);
    }
}

/// Outlander under a headless fixed-tick host. One `update()` is one sim tick.
pub fn build_outlander_app() -> App {
    ambition::app::PlatformerApp::headless()
        .mount(OutlanderModule)
        .build()
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
/// The host switch is one builder call versus [`build_outlander_app`]. That is
/// the claim: a consumer does not restructure its game to become
/// rollback-capable.
///
/// # LEAK CLOSED 2026-07-30 — the ninth, and this builder is where it hid
///
/// The A2 inventory found eight ordering rules in this fixture's hand-composed
/// hosts. Migrating to the builder found a NINTH, which had been sitting in
/// this function's own comment and nowhere else:
///
/// > "Under GGRS the sim advances only through session requests, so the frame
/// > dt must be the tick dt exactly (integer nanos, no drift)."
///
/// `Time::<Fixed>::from_hz(60.0)` rounds to `16_666_667`ns. GGRS wants the
/// truncated `16_666_666`. Composing this host with the obvious value — the
/// one the fixed-tick face correctly uses — cost **12 frames**: the parity walk
/// below took 192 `update()` calls to reach a world state the fixed-tick host
/// reached in 180, while every GGRS checksum still agreed.
///
/// That is the silent class. A consumer who wrote the obvious thing got a host
/// that runs, simulates correctly, agrees on every checksum, and quietly needs
/// 7% more frames — with nothing to grep for and no failure to read. It
/// surfaced only because `consumer_owned_authoritative_state_survives_real_resimulation`
/// compares the two hosts' timelines and went red on a change that looked
/// unrelated to either. The canary earned its keep.
///
/// `ambition::app` states the rule now, for both faces.
pub fn build_outlander_rollback_app() -> Result<App, String> {
    // `unstable_rollback_session` is deliberately not public API — rollback is
    // not a knob slice A promises. It exists so this composition goes through
    // the SAME builder as the other two instead of keeping a second
    // hand-ordered path alive, which is what would actually end the slice with
    // two paths.
    let mut app = ambition::app::PlatformerApp::headless()
        .unstable_rollback_session()
        .mount(OutlanderModule)
        .build();

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
        // A SPREAD rather than every field, deliberately: this is a THIRD PARTY
        // constructing an engine settings struct, and it should name only what it
        // cares about. Spelling every field out is how a consumer breaks on an
        // engine's internal addition — which is exactly what happened when
        // `players` landed (queue Y1) and this fixture was the only build in the
        // repo that noticed.
        //
        // ⚠ and the spread used to be `..Default::default()`, which took that
        // ergonomic argument one field too far: it also defaulted HOW MANY PEOPLE
        // ARE PLAYING, and a guess about topology is not a convenience. The base
        // is now a constructor that asks. Outlander is single-player, and now says
        // so (2026-07-29).
        ambition::runtime::rollback::SyncTestSettings {
            check_distance: 4,
            max_prediction_window: 10,
            ..ambition::runtime::rollback::SyncTestSettings::for_players(1)
        },
    )
    .map_err(|error| format!("failed to start the Outlander sync-test session: {error}"))?;
    app.update();
    Ok(app)
}

/// The window title the visible binary and the render test share.
pub const OUTLANDER_WINDOW_TITLE: &str = "Outlander — external consumer proof";

/// **Outlander, drawn.** The composition `src/bin/visible.rs` runs and the
/// render test observes — one function, so they cannot drift.
///
/// `gpu: false` builds the full render graph against no wgpu backend, which is
/// how a test asserts "the consumer's character is DRAWN" on CI that has none.
/// It is one builder call and nothing else — if it needed a second difference,
/// the test would be observing a composition nothing ships.
#[cfg(feature = "visible")]
pub fn build_windowed_app(gpu: bool) -> App {
    let composed = ambition::app::PlatformerApp::windowed(OUTLANDER_WINDOW_TITLE);
    let composed = if gpu { composed } else { composed.without_gpu() };
    composed.mount(OutlanderModule).build()
}

/// Drive one frame of input, through the engine's own driver seam.
///
/// LEAK CLOSED 2026-07-27. This used to carry its own branch — `PendingLocalInput`
/// under GGRS, the `ControlFrame` resource under fixed tick — because a consumer
/// running its game under both hosts had to know both, and writing the wrong one
/// is silently ignored: the walk runs, the body never moves, nothing says why.
/// That is a rule the engine can state once, and now does
/// (`ambition::runtime::rollback::drive_control_frame`).
///
/// Kept as a one-line wrapper rather than deleted, because the binaries and the
/// walkthrough all call it and the name says what it is FOR.
pub fn drive_control_frame(app: &mut App, frame: ambition::input::ControlFrame) {
    ambition::runtime::rollback::drive_control_frame(app.world_mut(), frame);
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
    use ambition::actor::PrimaryPlayer;
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
            .query_filtered::<&ambition::actor::BodyKinematics, With<PrimaryPlayer>>();
        let player_count = players.iter(world).count();
        if player_count != 1 {
            return Err(format!(
                "expected exactly one primary player after activation, found {player_count}"
            ));
        }
        let mut actors = world.query::<&ambition::actor::ActorConfig>();
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
    use ambition::actor::PrimaryPlayer;
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
            .query_filtered::<&ambition::actor::BodyKinematics, With<PrimaryPlayer>>();
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
        &ambition::actor::BodyKinematics,
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
    let geometry = RoomGeometry(room.world.clone());
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
