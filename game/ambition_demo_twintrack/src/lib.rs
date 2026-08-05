//! TwinTrack — a compact special-relativity signal course.
//!
//! The controlled traveler must emit a forward light pulse while moving fast
//! enough for a stationary receiver to measure the pulse inside its Doppler
//! passband. The same null signal continues to a radar reflector and returns as
//! an echo. The transmitter recharges in traveler proper time, laboratory
//! machinery runs in coordinate time, and reunion still compares the twins'
//! elapsed proper times.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::lifecycle::{SessionRoot, SessionScopedEntity};
use ambition_platformer2d::platformer::schedule::{
    Platformer2dSimulationPhaseMonolith, PlayerInputSet, SimScheduleExt, WorldPrepSet,
};
use ambition_platformer2d::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition_platformer2d::relativity2d::{
    ActiveSpacetime2d, LightEmissionRequest2d, LightEmitter2d, LightReceiver2d, LightSignal2d,
    LightSignalPoolSlot2d, ProperTimeCooldown2d, ProperTimeElapsed, RelativisticClock2d,
    Relativity2dPlugin, Relativity2dSet, RelativityClockLabel, RelativityClockView2d,
    RelativitySignalView2d, SignalArrival2d, SignalArrivalHistory2d, SpacetimeCoordinateTime2d,
    WorldlineTracked2d,
};
// ⚠ **gated with their only consumer.** `draw_twintrack_sr_course` is
// `#[cfg(feature = "visible")]`, and these two names are used nowhere else — so
// importing them unconditionally warns on every headless build, which this repo
// treats as an error (`scripts/check_no_warnings.py`).
#[cfg(feature = "visible")]
use ambition_platformer2d::relativity2d::{LightReceiverMode2d, WorldlineHistoryView2d};
use ambition_platformer2d::runtime::demo_fixture::{
    ActiveRoomMetadata, LdtkRuntimeIndex, RoomSet, StartingCharacter,
};
use ambition_platformer2d::runtime::rollback::AmbitionRollbackApp;
use ambition_platformer2d::runtime::PreparedPlatformerSource;
use ambition_platformer2d::time::WorldTime;
use ambition_platformer2d::world::rooms::RoomSpec;
use bevy::prelude::*;

pub const TWINTRACK_EXPERIENCE: &str = "twintrack";
pub const TWINTRACK_GAMEPLAY_ROUTE: &str = "twintrack_gameplay";
pub const TWINTRACK_LAUNCHER_ROUTE: &str = "twintrack_launcher";
pub const TWINTRACK_CHARACTER_ID: &str = "twintrack_traveler";
pub const TWINTRACK_ROOM_ID: &str = "twintrack_lab";

pub const INVARIANT_SPEED: f32 = 600.0;
pub const TARGET_SPEED: f32 = 540.0;
pub const EMITTED_FREQUENCY: f64 = 100.0;
pub const DOPPLER_PASSBAND_MIN: f64 = 380.0;
pub const DOPPLER_PASSBAND_MAX: f64 = 480.0;
pub const TRANSMITTER_COOLDOWN_PROPER_SECONDS: f64 = 0.75;
pub const LAB_X: f32 = 112.0;
pub const DOPPLER_RECEIVER_X: f32 = 690.0;
pub const RADAR_REFLECTOR_X: f32 = 1_315.0;
pub const TURNAROUND_X: f32 = 1_470.0;
pub const TRAVELER_RECEIVER_CHANNEL: u8 = 0;
pub const DOPPLER_RECEIVER_CHANNEL: u8 = 1;
pub const RADAR_RECEIVER_CHANNEL: u8 = 2;

const FLOOR_TOP: f32 = 320.0;
const ROOM_HEIGHT: f32 = 420.0;
const DEPARTURE_X: f32 = LAB_X + 72.0;
const REUNION_X: f32 = LAB_X + 52.0;
const COMPLETE_HOLD_SECONDS: f32 = 7.0;
const SIGNAL_POOL_LABEL: &str = "twintrack_signal_pool";
const SIGNAL_POOL_SIZE: u16 = 8;
const TRANSMITTER_LABEL: &str = "traveler_transmitter";

pub const STATUS_HUD_SLOT: &str = "twintrack_status";
pub const CLOCKS_HUD_SLOT: &str = "twintrack_clocks";
pub const SIGNALS_HUD_SLOT: &str = "twintrack_signals";
pub const RESULT_HUD_SLOT: &str = "twintrack_result";

const TWINTRACK_CATALOG_RON: &str = r#"(
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
        "twintrack_traveler": (
            display_name: "TwinTrack Traveler",
            spritesheet: "sprites/mary_o_v2_spritesheet.png",
            manifest: "sprites/mary_o_v2_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            abilities: Some([RunJump]),
            axis_tuning: Some((
                horizontal_law: Responsive,
                jump_law: VelocityCut,
                gravity: 1800.0,
                air_jumps: 0,
                jump_speed: 0.0,
                max_run_speed: 540.0,
                run_accel: 1800.0,
                air_accel: 900.0,
                max_fall_speed: 540.0,
                coyote_time: 0.0,
                jump_buffer: 0.0,
            )),
            max_health: Some(1),
            playable_kit: Authored,
            tags: ["player", "relativity", "signal_course"],
            barks: (
                hall: ["Tune the pulse with velocity.", "The echo knows where I was."],
            ),
        ),
    },
)"#;

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct TravelerTwin;

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct LaboratoryTwin;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TwinTrackPhase {
    Ready,
    DopplerLock,
    AwaitEcho,
    Inbound,
    Complete,
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct TwinTrackExperiment {
    pub phase: TwinTrackPhase,
    pub peak_beta: f32,
    pub qualified_packet_id: u64,
    pub doppler_frequency: f64,
    pub doppler_arrival_time: f64,
    pub radar_arrival_time: f64,
    pub echo_arrival_time: f64,
    pub echo_receiver_proper_time: f64,
    pub accepted_packets: u32,
    pub result_lab_time: f64,
    pub result_traveler_time: f64,
    pub complete_elapsed: f32,
}

impl Default for TwinTrackExperiment {
    fn default() -> Self {
        Self {
            phase: TwinTrackPhase::Ready,
            peak_beta: 0.0,
            qualified_packet_id: 0,
            doppler_frequency: 0.0,
            doppler_arrival_time: 0.0,
            radar_arrival_time: 0.0,
            echo_arrival_time: 0.0,
            echo_receiver_proper_time: 0.0,
            accepted_packets: 0,
            result_lab_time: 0.0,
            result_traveler_time: 0.0,
            complete_elapsed: 0.0,
        }
    }
}

impl ambition_platformer2d::engine_core::snapshot::SnapshotState for TwinTrackExperiment {
    fn encode(&self, out: &mut Vec<u8>) {
        use ambition_platformer2d::engine_core::snapshot::{put_f32, put_u32, put_u64, put_u8};
        put_u8(
            out,
            match self.phase {
                TwinTrackPhase::Ready => 0,
                TwinTrackPhase::DopplerLock => 1,
                TwinTrackPhase::AwaitEcho => 2,
                TwinTrackPhase::Inbound => 3,
                TwinTrackPhase::Complete => 4,
            },
        );
        put_f32(out, self.peak_beta);
        put_u64(out, self.qualified_packet_id);
        put_u64(out, self.doppler_frequency.to_bits());
        put_u64(out, self.doppler_arrival_time.to_bits());
        put_u64(out, self.radar_arrival_time.to_bits());
        put_u64(out, self.echo_arrival_time.to_bits());
        put_u64(out, self.echo_receiver_proper_time.to_bits());
        put_u32(out, self.accepted_packets);
        put_u64(out, self.result_lab_time.to_bits());
        put_u64(out, self.result_traveler_time.to_bits());
        put_f32(out, self.complete_elapsed);
    }

    fn decode(
        reader: &mut ambition_platformer2d::engine_core::snapshot::Reader<'_>,
    ) -> Option<Self> {
        Some(Self {
            phase: match reader.u8()? {
                0 => TwinTrackPhase::Ready,
                1 => TwinTrackPhase::DopplerLock,
                2 => TwinTrackPhase::AwaitEcho,
                3 => TwinTrackPhase::Inbound,
                4 => TwinTrackPhase::Complete,
                _ => return None,
            },
            peak_beta: reader.f32()?,
            qualified_packet_id: reader.u64()?,
            doppler_frequency: f64::from_bits(reader.u64()?),
            doppler_arrival_time: f64::from_bits(reader.u64()?),
            radar_arrival_time: f64::from_bits(reader.u64()?),
            echo_arrival_time: f64::from_bits(reader.u64()?),
            echo_receiver_proper_time: f64::from_bits(reader.u64()?),
            accepted_packets: reader.u32()?,
            result_lab_time: f64::from_bits(reader.u64()?),
            result_traveler_time: f64::from_bits(reader.u64()?),
            complete_elapsed: reader.f32()?,
        })
    }
}

pub fn twintrack_room() -> RoomSpec {
    let size = ae::Vec2::new(1_680.0, ROOM_HEIGHT);
    let world = ae::World::new(
        "TwinTrack Minkowski Signal Laboratory",
        size,
        ae::Vec2::new(LAB_X, FLOOR_TOP - 64.0),
        vec![
            ae::Block::solid(
                "twintrack_floor",
                ae::Vec2::new(0.0, FLOOR_TOP),
                ae::Vec2::new(size.x, 64.0),
            ),
            ae::Block::solid(
                "laboratory_clock_pedestal",
                ae::Vec2::new(LAB_X - 38.0, FLOOR_TOP - 48.0),
                ae::Vec2::new(28.0, 48.0),
            ),
            ae::Block::solid(
                "turnaround_beacon",
                ae::Vec2::new(TURNAROUND_X + 64.0, FLOOR_TOP - 112.0),
                ae::Vec2::new(28.0, 112.0),
            ),
        ],
    );
    let mut room = RoomSpec::new(TWINTRACK_ROOM_ID, world);
    room.metadata.mode = Some(TWINTRACK_EXPERIENCE.to_owned());
    room
}

pub fn install_twintrack_content(app: &mut App) {
    use ambition_platformer2d::audio::catalog::{AudioCatalogAppExt, AudioCatalogFragment};
    use ambition_platformer2d::characters::actor::character_catalog::{
        CharacterCatalogAppExt, CharacterCatalogFragment,
    };

    app.register_character_catalog_fragment(
        CharacterCatalogFragment::from_ron(
            TWINTRACK_EXPERIENCE,
            Some(TWINTRACK_CHARACTER_ID),
            TWINTRACK_CATALOG_RON,
        )
        .expect("TwinTrack character catalog should be valid"),
    );
    {
        use ambition_platformer2d::actors::character_runtime::{
            CharacterDefinition, CharacterDefinitionAppExt,
        };
        app.register_character(
            CharacterDefinition::new(
                TWINTRACK_CHARACTER_ID,
                "TwinTrack Traveler",
                TWINTRACK_EXPERIENCE,
            )
            .with_sheet("mary_o_v2")
            .with_voice([
                "Coordinate time launches the pulse.",
                "My transmitter recharges in proper time.",
            ]),
        );
    }
    app.register_audio_catalog_fragment(
        AudioCatalogFragment::new(TWINTRACK_EXPERIENCE, None, None)
            .expect("TwinTrack explicitly declares silence"),
    );
}

pub struct TwinTrackExperiencePlugin;

impl Plugin for TwinTrackExperiencePlugin {
    fn build(&self, app: &mut App) {
        install_twintrack_content(app);
        if !app.is_plugin_added::<Relativity2dPlugin>() {
            app.add_plugins(Relativity2dPlugin);
        }
        app.rollback_component_canonical::<TwinTrackExperiment>(
            "ambition_demo_twintrack",
            "twintrack.experiment",
        )
        .rollback_component_clone::<TravelerTwin>("ambition_demo_twintrack", "twintrack.traveler")
        .rollback_component_clone::<LaboratoryTwin>(
            "ambition_demo_twintrack",
            "twintrack.laboratory",
        );

        PlatformerExperienceAuthoring::new(
            TWINTRACK_EXPERIENCE,
            TWINTRACK_GAMEPLAY_ROUTE,
            "TwinTrack",
            "Doppler-lock a null pulse, catch its radar echo, and compare proper time",
            "Prepare TwinTrack",
            AuthoredCatalogFragments::new(TWINTRACK_CHARACTER_ID, TWINTRACK_EXPERIENCE),
        )
        .with_presentation_profiles(
            ambition_platformer2d::presentation::profiles::high_speed_full_bleed(),
        )
        .with_hud(
            ambition_platformer2d::presentation::HudDeclaration::new()
                .slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(STATUS_HUD_SLOT)
                        .with_region(ambition_platformer2d::presentation::SurroundRegion::Top)
                        .with_font_size(19.0),
                )
                .slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(SIGNALS_HUD_SLOT)
                        .with_region(ambition_platformer2d::presentation::SurroundRegion::Top)
                        .with_order(2)
                        .with_font_size(17.0),
                )
                .slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(CLOCKS_HUD_SLOT)
                        .with_region(ambition_platformer2d::presentation::SurroundRegion::Bottom)
                        .with_font_size(18.0),
                )
                .slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(RESULT_HUD_SLOT)
                        .with_order(99)
                        .with_font_size(25.0)
                        .centered(),
                ),
        )
        .install(app, twintrack_prepared_session_world);

        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            install_twintrack_session
                .run_if(ambition_platformer2d::runtime::in_mode(
                    TWINTRACK_EXPERIENCE,
                ))
                .before(Relativity2dSet::AdvanceCoordinateTime)
                .in_set(WorldPrepSet::BeforeIntegrate),
        )
        .add_systems(
            sim,
            update_twintrack_experiment
                .run_if(ambition_platformer2d::runtime::in_mode(
                    TWINTRACK_EXPERIENCE,
                ))
                .after(Relativity2dSet::ResolveClocks)
                .in_set(WorldPrepSet::BeforeIntegrate),
        )
        .add_systems(
            sim,
            capture_twintrack_transmitter_input
                .run_if(ambition_platformer2d::runtime::in_mode(
                    TWINTRACK_EXPERIENCE,
                ))
                .in_set(PlayerInputSet::ControlGate)
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput),
        )
        .add_systems(
            sim,
            consume_twintrack_signal_arrivals
                .run_if(ambition_platformer2d::runtime::in_mode(
                    TWINTRACK_EXPERIENCE,
                ))
                .in_set(Platformer2dSimulationPhaseMonolith::GameplayEffects),
        )
        .add_systems(
            Update,
            publish_twintrack_hud.run_if(ambition_platformer2d::runtime::in_mode(
                TWINTRACK_EXPERIENCE,
            )),
        );

        // ⚠ **`Gizmos` needs `GizmoConfigStore`, and being in the right MODE is
        // not the same as having a renderer.** The workspace suite builds this
        // experience with the `visible` feature but without Bevy's `GizmoPlugin`,
        // and a `Gizmos` param whose store is missing fails validation and
        // PANICS the schedule rather than skipping the system — which is exactly
        // how it failed: `Parameter Res<GizmoConfigStore> failed validation:
        // Resource does not exist`. The mode condition cannot see that; the
        // resource condition can.
        #[cfg(feature = "visible")]
        app.add_systems(
            Update,
            draw_twintrack_sr_course
                .run_if(ambition_platformer2d::runtime::in_mode(
                    TWINTRACK_EXPERIENCE,
                ))
                .run_if(bevy::prelude::resource_exists::<bevy::gizmos::config::GizmoConfigStore>),
        );
    }
}

fn twintrack_prepared_session_world() -> PreparedPlatformerSource {
    let room = twintrack_room();
    PreparedPlatformerSource::new(
        TWINTRACK_EXPERIENCE,
        RoomSet::from_parts(TWINTRACK_ROOM_ID, vec![room.clone()], Vec::new()),
        ae::RoomGeometry(room.world.clone()),
        ActiveRoomMetadata(room.metadata),
        StartingCharacter::new(TWINTRACK_CHARACTER_ID),
        LdtkRuntimeIndex::default(),
    )
}

fn install_twintrack_session(
    mut commands: Commands,
    roots: Query<(Entity, &SessionRoot, &ActiveRoomMetadata), Without<ActiveSpacetime2d>>,
    traveler: Query<
        Entity,
        (
            With<ambition_platformer2d::actor::PrimaryPlayer>,
            Without<TravelerTwin>,
        ),
    >,
) {
    let Ok((root_entity, root, metadata)) = roots.single() else {
        return;
    };
    if metadata.0.mode.as_deref() != Some(TWINTRACK_EXPERIENCE) {
        return;
    }
    let Ok(traveler_entity) = traveler.single() else {
        return;
    };

    commands.entity(root_entity).insert((
        ActiveSpacetime2d::minkowski(INVARIANT_SPEED as f64)
            .expect("TwinTrack invariant speed is finite and positive"),
        SpacetimeCoordinateTime2d::default(),
        SignalArrivalHistory2d::default(),
    ));
    commands.entity(traveler_entity).insert((
        TravelerTwin,
        RelativisticClock2d,
        RelativityClockLabel("traveler".to_owned()),
        WorldlineTracked2d("traveler".to_owned()),
        LightEmitter2d::new(
            TRANSMITTER_LABEL,
            SIGNAL_POOL_LABEL,
            EMITTED_FREQUENCY,
            TRANSMITTER_COOLDOWN_PROPER_SECONDS,
        )
        .with_source_receiver_channel(TRAVELER_RECEIVER_CHANNEL),
        ProperTimeCooldown2d::default(),
        LightReceiver2d::observer(
            "traveler_echo_receiver",
            TRAVELER_RECEIVER_CHANNEL,
            Vec2::new(22.0, 36.0),
        )
        .consuming(),
    ));
    commands.spawn((
        LaboratoryTwin,
        RelativisticClock2d,
        RelativityClockLabel("laboratory".to_owned()),
        WorldlineTracked2d("laboratory".to_owned()),
        ProperTimeElapsed::ZERO,
        TwinTrackExperiment::default(),
        ae::BodyKinematics {
            pos: Vec2::new(LAB_X - 26.0, FLOOR_TOP - 76.0),
            vel: Vec2::ZERO,
            size: Vec2::splat(24.0),
            facing: 1.0,
        },
        SessionScopedEntity(root.0),
        ambition_platformer2d::platformer::sim_id::SimId::placement("twintrack_lab_clock"),
    ));
    commands.spawn((
        RelativisticClock2d,
        LightReceiver2d::observer(
            "doppler_passband",
            DOPPLER_RECEIVER_CHANNEL,
            Vec2::new(12.0, 80.0),
        )
        .with_passband(DOPPLER_PASSBAND_MIN, DOPPLER_PASSBAND_MAX),
        ae::BodyKinematics {
            pos: Vec2::new(DOPPLER_RECEIVER_X, FLOOR_TOP - 86.0),
            vel: Vec2::ZERO,
            size: Vec2::new(24.0, 160.0),
            facing: 1.0,
        },
        SessionScopedEntity(root.0),
        ambition_platformer2d::platformer::sim_id::SimId::placement("twintrack_doppler_receiver"),
    ));
    commands.spawn((
        RelativisticClock2d,
        LightReceiver2d::reflector(
            "radar_reflector",
            RADAR_RECEIVER_CHANNEL,
            Vec2::new(14.0, 92.0),
        ),
        ae::BodyKinematics {
            pos: Vec2::new(RADAR_REFLECTOR_X, FLOOR_TOP - 96.0),
            vel: Vec2::ZERO,
            size: Vec2::new(28.0, 184.0),
            facing: -1.0,
        },
        SessionScopedEntity(root.0),
        ambition_platformer2d::platformer::sim_id::SimId::placement("twintrack_radar_reflector"),
    ));
    for slot_index in 0..SIGNAL_POOL_SIZE {
        let signal_id = format!("twintrack_signal_slot_{slot_index}");
        commands.spawn((
            LightSignalPoolSlot2d::new(SIGNAL_POOL_LABEL, slot_index),
            LightSignal2d::inactive(),
            SessionScopedEntity(root.0),
            ambition_platformer2d::platformer::sim_id::SimId::placement(&signal_id),
        ));
    }
}

fn reset_course_state(
    traveler_clock: &mut ProperTimeElapsed,
    lab_clock: &mut ProperTimeElapsed,
    coordinate_time: &mut SpacetimeCoordinateTime2d,
    arrival_history: &mut SignalArrivalHistory2d,
    cooldown: &mut ProperTimeCooldown2d,
    emitter: &mut LightEmitter2d,
    signals: &mut Query<&mut LightSignal2d>,
) {
    traveler_clock.reset();
    lab_clock.reset();
    coordinate_time.reset();
    arrival_history.clear();
    cooldown.reset();
    emitter.next_packet_id = 1;
    for mut signal in signals.iter_mut() {
        *signal = LightSignal2d::inactive();
    }
}

#[allow(clippy::too_many_arguments)]
fn update_twintrack_experiment(
    time: Res<WorldTime>,
    // ⭐ **the whole cluster, not just `BodyKinematics`.** The experiment
    // RELOCATES the traveler at the end of the hold and STOPS it dead at the
    // reunion; both are discrete relocations, and `transit_body` is the
    // authority for those. It needs the motion model and the sibling clusters to
    // invalidate the contacts, attachment and collapsed sweep that were true
    // only where the body used to be. See the two calls below.
    mut traveler: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d::actors::features::MotionModel,
            &mut ProperTimeElapsed,
            &mut ProperTimeCooldown2d,
            &mut LightEmitter2d,
        ),
        (With<TravelerTwin>, Without<LaboratoryTwin>),
    >,
    mut lab: Query<
        (&mut ProperTimeElapsed, &mut TwinTrackExperiment),
        (With<LaboratoryTwin>, Without<TravelerTwin>),
    >,
    mut spacetime: Query<
        (&mut SpacetimeCoordinateTime2d, &mut SignalArrivalHistory2d),
        (With<SessionRoot>, With<ActiveSpacetime2d>),
    >,
    mut signals: Query<&mut LightSignal2d>,
    relativity: Query<&ambition_platformer2d::relativity2d::RelativityState2d, With<TravelerTwin>>,
) {
    let (
        Ok((mut cluster_item, mut motion_model, mut traveler_clock, mut cooldown, mut emitter)),
        Ok((mut lab_clock, mut experiment)),
        Ok((mut coordinate_time, mut arrival_history)),
    ) = (
        traveler.single_mut(),
        lab.single_mut(),
        spacetime.single_mut(),
    )
    else {
        return;
    };
    let mut clusters = cluster_item.as_clusters_mut();
    // Read the pose ONCE: the phase machine only asks where the traveler is and
    // which way it is going, and every WRITE goes through `transit_body`, which
    // needs `&mut clusters` and cannot coexist with a long-lived member borrow.
    let body = ae::BodyKinematics {
        pos: clusters.kinematics.pos,
        vel: clusters.kinematics.vel,
        ..*clusters.kinematics
    };

    let beta = relativity
        .single()
        .ok()
        .map(|state| state.beta_squared.max(0.0).sqrt())
        .unwrap_or(0.0);
    experiment.peak_beta = experiment.peak_beta.max(beta);

    match experiment.phase {
        TwinTrackPhase::Ready => {
            if body.pos.x >= DEPARTURE_X && body.vel.x > 0.0 {
                reset_course_state(
                    &mut traveler_clock,
                    &mut lab_clock,
                    &mut coordinate_time,
                    &mut arrival_history,
                    &mut cooldown,
                    &mut emitter,
                    &mut signals,
                );
                experiment.peak_beta = beta;
                experiment.phase = TwinTrackPhase::DopplerLock;
            }
        }
        TwinTrackPhase::DopplerLock | TwinTrackPhase::AwaitEcho => {}
        TwinTrackPhase::Inbound => {
            if body.pos.x <= REUNION_X && body.vel.x < 0.0 {
                experiment.result_lab_time = lab_clock.seconds;
                experiment.result_traveler_time = traveler_clock.seconds;
                experiment.complete_elapsed = 0.0;
                experiment.phase = TwinTrackPhase::Complete;
                // Stops dead at the reunion. Through the authority even though
                // the position does not change: a bare `vel = ZERO` leaves the
                // collapsed §3.1 sweep still describing the inbound run.
                ae::movement::transit_body(
                    &mut motion_model,
                    &mut clusters,
                    body.pos,
                    ae::movement::TransitVelocity::Zero,
                );
            }
        }
        TwinTrackPhase::Complete => {
            experiment.complete_elapsed += time.sim_dt();
            if experiment.complete_elapsed >= COMPLETE_HOLD_SECONDS {
                // ⛔ **this wrote `pos` and `vel` directly** — a discrete
                // relocation performed outside the movement authority, leaving
                // contacts, ledge grab, model-private attachment and the
                // collapsed sweep describing the reunion point (GPT 5.6, review
                // through `f0f97f5`; fixed in `edad6a5e9`, and this overlay
                // predates that fix and reintroduced it).
                ae::movement::transit_body(
                    &mut motion_model,
                    &mut clusters,
                    Vec2::new(LAB_X, FLOOR_TOP - 64.0),
                    ae::movement::TransitVelocity::Zero,
                );
                reset_course_state(
                    &mut traveler_clock,
                    &mut lab_clock,
                    &mut coordinate_time,
                    &mut arrival_history,
                    &mut cooldown,
                    &mut emitter,
                    &mut signals,
                );
                *experiment = TwinTrackExperiment::default();
            }
        }
    }
}

fn capture_twintrack_transmitter_input(
    traveler: Query<
        (
            Entity,
            &ambition_platformer2d::actors::control::PlayerInputFrame,
            &ae::BodyKinematics,
        ),
        With<TravelerTwin>,
    >,
    mut requests: MessageWriter<LightEmissionRequest2d>,
) {
    let Ok((entity, input, body)) = traveler.single() else {
        return;
    };
    if input.frame.interact_pressed {
        let facing = if body.facing < 0.0 { -1.0 } else { 1.0 };
        requests.write(LightEmissionRequest2d::new(entity, Vec2::new(facing, 0.0)));
    }
}

fn consume_twintrack_signal_arrivals(
    mut arrivals: MessageReader<SignalArrival2d>,
    mut experiment: Query<&mut TwinTrackExperiment, With<LaboratoryTwin>>,
) {
    let Ok(mut experiment) = experiment.single_mut() else {
        return;
    };
    for message in arrivals.read() {
        let arrival = &message.0;
        match arrival.receiver_channel {
            DOPPLER_RECEIVER_CHANNEL if arrival.accepted && !arrival.signal_was_reflected => {
                experiment.qualified_packet_id = arrival.packet_id;
                experiment.doppler_frequency = arrival.observed_frequency;
                experiment.doppler_arrival_time = arrival.coordinate_time;
                experiment.accepted_packets = experiment.accepted_packets.saturating_add(1);
                if matches!(
                    experiment.phase,
                    TwinTrackPhase::DopplerLock | TwinTrackPhase::AwaitEcho
                ) {
                    experiment.phase = TwinTrackPhase::AwaitEcho;
                }
            }
            RADAR_RECEIVER_CHANNEL
                if arrival.packet_id == experiment.qualified_packet_id
                    && experiment.qualified_packet_id != 0 =>
            {
                experiment.radar_arrival_time = arrival.coordinate_time;
            }
            TRAVELER_RECEIVER_CHANNEL
                if arrival.packet_id == experiment.qualified_packet_id
                    && experiment.qualified_packet_id != 0
                    && arrival.signal_was_reflected =>
            {
                experiment.echo_arrival_time = arrival.coordinate_time;
                experiment.echo_receiver_proper_time =
                    arrival.receiver_proper_time.unwrap_or_default();
                experiment.phase = TwinTrackPhase::Inbound;
            }
            _ => {}
        }
    }
}

fn publish_twintrack_hud(
    clocks: Res<RelativityClockView2d>,
    signals: Res<RelativitySignalView2d>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut readouts: ResMut<ambition_platformer2d::presentation::HudReadouts>,
) {
    let (Some(traveler), Some(lab), Ok(experiment)) = (
        clocks.clocks.get("traveler"),
        clocks.clocks.get("laboratory"),
        experiment.single(),
    ) else {
        readouts.clear_slot(STATUS_HUD_SLOT);
        readouts.clear_slot(CLOCKS_HUD_SLOT);
        readouts.clear_slot(SIGNALS_HUD_SLOT);
        readouts.clear_slot(RESULT_HUD_SLOT);
        return;
    };

    let instruction = match experiment.phase {
        TwinTrackPhase::Ready => "HOLD RIGHT — DEPART THE LAB",
        TwinTrackPhase::DopplerLock => {
            "AT HIGH SPEED PRESS INTERACT/F/RB — BLUE-SHIFT INTO THE PASSBAND"
        }
        TwinTrackPhase::AwaitEcho => "QUALIFIED PULSE SENT — CHASE THE RADAR ECHO",
        TwinTrackPhase::Inbound => "ECHO RECEIVED — TURN AROUND AND REUNITE",
        TwinTrackPhase::Complete => "REUNION EVENT — WORLDLINES AND SIGNAL EVENTS COMPARED",
    };
    let emitter = signals
        .emitters
        .iter()
        .find(|emitter| emitter.label == TRANSMITTER_LABEL);
    let cooldown = emitter.map_or(0.0, |emitter| emitter.cooldown_remaining);
    readouts.set(
        STATUS_HUD_SLOT,
        ambition_platformer2d::presentation::HudReadout::bare(format!(
            "{instruction}    SPEED {:>5.1}/{:.0}    beta {:.3}    gamma {:.3}    dτ/dt {:.3}",
            traveler.relative_velocity.length(),
            traveler.invariant_speed,
            traveler.beta_squared.max(0.0).sqrt(),
            traveler.lorentz_factor,
            traveler.proper_time_rate,
        )),
    );

    let last_arrival = signals.recent_arrivals.last().map_or_else(
        || "NO SIGNAL ARRIVALS YET".to_owned(),
        |arrival| {
            let traveler_coordinates =
                ambition_platformer2d::relativity::InvariantSpeed::new(signals.invariant_speed)
                    .ok()
                    .and_then(|invariant_speed| {
                        ambition_platformer2d::relativity::lorentz_boost_event(
                            ambition_platformer2d::relativity::MinkowskiEvent {
                                coordinate_time: arrival.coordinate_time - signals.coordinate_time,
                                position: [
                                    f64::from(arrival.position.x - traveler.position.x),
                                    f64::from(arrival.position.y - traveler.position.y),
                                    0.0,
                                ],
                            },
                            [
                                f64::from(traveler.coordinate_velocity.x),
                                f64::from(traveler.coordinate_velocity.y),
                                0.0,
                            ],
                            invariant_speed,
                        )
                    })
                    .map_or_else(
                        || "traveler frame unavailable".to_owned(),
                        |event| {
                            format!(
                                "traveler-frame offset Δt'={:+.3}s Δx'={:+.1}",
                                event.coordinate_time, event.position[0]
                            )
                        },
                    );
            format!(
                "LAST: packet {} -> {} at lab t={:.3}s  measured {:.1}Hz  {}  |  {}",
                arrival.packet_id,
                arrival.receiver_label,
                arrival.coordinate_time,
                arrival.observed_frequency,
                if arrival.accepted {
                    "ACCEPTED"
                } else {
                    "OUT OF BAND"
                },
                traveler_coordinates,
            )
        },
    );
    readouts.set(
        SIGNALS_HUD_SLOT,
        ambition_platformer2d::presentation::HudReadout::bare(format!(
            "TRANSMITTER {:.0}Hz  COOLDOWN {:.3}s proper  TARGET {:.0}-{:.0}Hz  ACTIVE PULSES {}  {last_arrival}",
            EMITTED_FREQUENCY,
            cooldown,
            DOPPLER_PASSBAND_MIN,
            DOPPLER_PASSBAND_MAX,
            signals.active_signals.len(),
        )),
    );
    readouts.set(
        CLOCKS_HUD_SLOT,
        ambition_platformer2d::presentation::HudReadout::bare(format!(
            "COORDINATE TIME {:>7.3}s    LAB CLOCK {:>7.3}s    TRAVELER CLOCK {:>7.3}s    DIFFERENCE {:>7.3}s",
            signals.coordinate_time,
            lab.proper_time_seconds,
            traveler.proper_time_seconds,
            lab.proper_time_seconds - traveler.proper_time_seconds,
        )),
    );

    if experiment.phase == TwinTrackPhase::Complete {
        readouts.set(
            RESULT_HUD_SLOT,
            ambition_platformer2d::presentation::HudReadout::bare(format!(
                concat!(
                    "TWINTRACK SIGNAL COURSE COMPLETE\n",
                    "DOPPLER {:.1}Hz at t={:.3}s    RADAR t={:.3}s    ECHO t={:.3}s / traveler τ={:.3}s\n",
                    "LAB {:.3}s    TRAVELER {:.3}s    AGED {:.3}s LESS    PEAK beta {:.3}",
                ),
                experiment.doppler_frequency,
                experiment.doppler_arrival_time,
                experiment.radar_arrival_time,
                experiment.echo_arrival_time,
                experiment.echo_receiver_proper_time,
                experiment.result_lab_time,
                experiment.result_traveler_time,
                experiment.result_lab_time - experiment.result_traveler_time,
                experiment.peak_beta,
            )),
        );
    } else {
        readouts.clear_slot(RESULT_HUD_SLOT);
    }
}

#[cfg(feature = "visible")]
fn draw_twintrack_sr_course(
    mut gizmos: Gizmos,
    signals: Res<RelativitySignalView2d>,
    worldlines: Res<WorldlineHistoryView2d>,
) {
    fn to_bevy(position: Vec2) -> Vec2 {
        Vec2::new(position.x, ROOM_HEIGHT - position.y)
    }

    fn draw_box(gizmos: &mut Gizmos, center: Vec2, half: Vec2, color: Color) {
        let min = center - half;
        let max = center + half;
        let top_left = Vec2::new(min.x, max.y);
        let top_right = max;
        let bottom_right = Vec2::new(max.x, min.y);
        let bottom_left = min;
        gizmos.line_2d(top_left, top_right, color);
        gizmos.line_2d(top_right, bottom_right, color);
        gizmos.line_2d(bottom_right, bottom_left, color);
        gizmos.line_2d(bottom_left, top_left, color);
    }

    for receiver in &signals.receivers {
        let center = to_bevy(receiver.position);
        let half = Vec2::new(receiver.half_extents.x, receiver.half_extents.y);
        let color = match receiver.mode {
            LightReceiverMode2d::Reflect => Color::srgb(1.0, 0.72, 0.12),
            LightReceiverMode2d::Observe if receiver.accepted_frequency.is_some() => {
                Color::srgb(0.20, 0.95, 0.72)
            }
            LightReceiverMode2d::Observe => Color::srgb(0.72, 0.82, 1.0),
        };
        draw_box(&mut gizmos, center, half, color);
    }

    for signal in &signals.active_signals {
        let head = to_bevy(signal.position);
        let direction = Vec2::new(signal.direction.x, -signal.direction.y);
        let tail = head - direction * 72.0;
        let ratio = signal.coordinate_frequency / EMITTED_FREQUENCY;
        let color = if ratio > 1.5 {
            Color::srgb(0.15, 0.72, 1.0)
        } else if ratio < 0.75 {
            Color::srgb(1.0, 0.22, 0.12)
        } else {
            Color::srgb(0.95, 0.95, 0.78)
        };
        gizmos.line_2d(tail, head, color);
        gizmos.line_2d(
            head + Vec2::new(-8.0, -8.0),
            head + Vec2::new(8.0, 8.0),
            color,
        );
        gizmos.line_2d(
            head + Vec2::new(-8.0, 8.0),
            head + Vec2::new(8.0, -8.0),
            color,
        );
    }

    let window = 8.0;
    for (label, samples) in &worldlines.tracks {
        let color = if label == "traveler" {
            Color::srgb(0.25, 1.0, 0.38)
        } else {
            Color::srgb(1.0, 0.82, 0.22)
        };
        let points: Vec<_> = samples
            .iter()
            .filter_map(|sample| {
                let age = signals.coordinate_time - sample.coordinate_time;
                (age >= 0.0 && age <= window).then(|| {
                    to_bevy(Vec2::new(
                        sample.position.x,
                        28.0 + (age / window) as f32 * 120.0,
                    ))
                })
            })
            .collect();
        for pair in points.windows(2) {
            gizmos.line_2d(pair[0], pair[1], color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_declares_the_twintrack_mode() {
        assert_eq!(
            twintrack_room().metadata.mode.as_deref(),
            Some(TWINTRACK_EXPERIENCE)
        );
    }

    #[test]
    fn target_speed_is_timelike_and_visibly_dilated() {
        let c =
            ambition_platformer2d::relativity::InvariantSpeed::new(INVARIANT_SPEED as f64).unwrap();
        let result = ambition_platformer2d::relativity::minkowski_clock_rate(
            (TARGET_SPEED * TARGET_SPEED) as f64,
            c,
        );
        assert_eq!(
            result.interval_kind,
            ambition_platformer2d::relativity::IntervalKind::Timelike
        );
        assert!(result.proper_time_rate.get() < 0.5);
    }

    #[test]
    fn target_speed_tunes_the_transmitter_into_the_station_passband() {
        let c =
            ambition_platformer2d::relativity::InvariantSpeed::new(INVARIANT_SPEED as f64).unwrap();
        let measurement = ambition_platformer2d::relativity::minkowski_doppler_measurement(
            EMITTED_FREQUENCY,
            [1.0, 0.0, 0.0],
            [f64::from(TARGET_SPEED), 0.0, 0.0],
            [0.0, 0.0, 0.0],
            c,
        )
        .unwrap();
        assert!(measurement.observed_frequency >= DOPPLER_PASSBAND_MIN);
        assert!(measurement.observed_frequency <= DOPPLER_PASSBAND_MAX);
    }
}
