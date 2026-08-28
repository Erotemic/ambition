//! TwinTrack — a character-driven 2D special-relativity plaza.
//!
//! The controlled traveler uses the engine's shared free-flight limb with a
//! proper-velocity controller. Other clock-bearing characters follow authored
//! worldlines, exchange clock reports over finite-speed light signals, dance
//! when Doppler-shifted into a passband, and play light tag from their visible
//! light-delayed positions. A full-screen optical view and a default-on 2+1D
//! spacetime minimap explain the same authoritative simulation without
//! becoming another movement or game-state path.

mod chase_beacon;
mod dual_observer;
mod light_pulse;
#[cfg(feature = "visible")]
mod observatory;
mod participants;
#[cfg(feature = "visible")]
mod spacetime_3d;
#[cfg(feature = "visible")]
mod split_screen;
#[cfg(feature = "visible")]
pub use observatory::ObservatoryCamera;
#[cfg(feature = "visible")]
pub use split_screen::{SplitObserverCamera, SplitObserverPane};

pub use participants::LAB_TWIN_SLOT;

pub use dual_observer::{
    beacon_alpha_position, beacon_midpoint, beacon_omega_position, flash_coordinate_time,
    observe_beacon_pair, observer_frame_offset, BeaconReading, EventOrdering,
    ObserverOrderingReport, TwinTrackBeacon, TwinTrackDualObserverView, BEACON_FLASH_GLOW_SECONDS,
    BEACON_FLASH_PERIOD_SECONDS, BEACON_HALF_SEPARATION,
};
pub use light_pulse::{
    aberrated_direction, latest_pulse_index, measure_pulse_ray, observe_light_pulse,
    pulse_emission_position, pulse_emission_time, pulse_frame_sample, pulse_front_position,
    PulseFrameSample, PulseObserverReport, PulseRay, PulseRayMeasurement, TwinTrackLightPulseView,
    ABERRATION_EPSILON_DEGREES, PULSE_PERIOD_SECONDS, PULSE_REST_FREQUENCY_THZ,
    PULSE_VISIBLE_SECONDS, SPEED_INVARIANCE_TOLERANCE,
};

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::lifecycle::{
    SessionRoot, SessionScopeId, SessionScopedEntity,
};
use ambition_platformer2d::platformer::schedule::{
    Platformer2dSimulationPhaseMonolith, PlayerInputSet, SimScheduleExt, WorldPrepSet,
};
use ambition_platformer2d::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition_platformer2d::relativity::{
    coordinate_photon_direction_from_observer, minkowski_doppler_measurement,
    solve_null_intercept_constant_velocity, InvariantSpeed,
};
use ambition_platformer2d::relativity2d::{
    ActiveSpacetime2d, LightEmissionRequest2d, LightEmitter2d, LightReceiver2d, LightSignal2d,
    LightSignalPoolSlot2d, OpticalSource2d, ProperTimeCooldown2d, ProperTimeElapsed,
    RelativisticClock2d, RelativisticObserver2d, RelativisticOpticalView2d, RelativisticTarget2d,
    RelativisticTargetingView2d, Relativity2dPlugin, Relativity2dSet, RelativityClockLabel,
    RelativityClockView2d, RelativitySignalView2d, SignalArrival2d, SignalArrivalHistory2d,
    SpacetimeCoordinateTime2d, WorldlineHistoryView2d, WorldlineTracked2d,
};
use ambition_platformer2d::rollback::AmbitionRollbackApp;
use ambition_platformer2d::runtime::demo_fixture::{
    ActiveRoomMetadata, RoomSet, StartingCharacter,
};
use ambition_platformer2d::runtime::PreparedPlatformerSource;
use ambition_platformer2d::world::rooms::{
    CameraClampMode, CameraScrollPolicy, CameraZoneSpec, RoomSpec,
};
use bevy::prelude::*;

pub const TWINTRACK_EXPERIENCE: &str = "twintrack";
pub const TWINTRACK_GAMEPLAY_ROUTE: &str = "twintrack_gameplay";
pub const TWINTRACK_LAUNCHER_ROUTE: &str = "twintrack_launcher";
pub const TWINTRACK_CHARACTER_ID: &str = "twintrack_traveler";
/// The SECOND participant's character — Emmy, at rest in the laboratory
/// until somebody on a second controller moves her.
pub const TWINTRACK_LAB_TWIN_CHARACTER_ID: &str = "twintrack_lab_twin";
pub const TWINTRACK_ROOM_ID: &str = "twintrack_plaza";

pub const INVARIANT_SPEED: f32 = 600.0;
pub const TARGET_SPEED: f32 = 540.0;
pub const EMITTED_FREQUENCY: f64 = 98.0;
pub const DOPPLER_TARGET_FREQUENCY: f64 = 196.0;
pub const DOPPLER_PASSBAND_MIN: f64 = 190.0;
pub const DOPPLER_PASSBAND_MAX: f64 = 202.0;
pub const TRANSMITTER_COOLDOWN_PROPER_SECONDS: f64 = 0.45;

pub const ROOM_WIDTH: f32 = 1_440.0;
pub const ROOM_HEIGHT: f32 = 900.0;
pub const LAB_POS: Vec2 = Vec2::new(720.0, 450.0);
pub const VIEW_CONSOLE_POS: Vec2 = Vec2::new(470.0, 700.0);
pub const DJ_POS: Vec2 = Vec2::new(1_010.0, 680.0);
// AMBITION_REVIEW(spatial): the light-tagger's OFFSET FROM `LAB_POS` IS THE LESSON, not decoration.
// The tagger flies +x (orbit phase -PI/2 puts it at the bottom of a 4 km arc), so the angle between
// "where you SEE it" and "where you must AIM" is the TRANSVERSE fraction of that velocity as seen
// from the observer at `LAB_POS` — i.e. it is set by `TAG_START_POS.y - LAB_POS.y`. Put the tagger
// on the observer's eye line and the lead angle goes to zero: the red visible-image marker, the
// green intercept marker and the cyan aim ray all collapse onto one another and the overlay teaches
// nothing. The 150 px offset restored here is the one the SR-3 overlay was authored against. Keep
// the tagger well off `LAB_POS.y`; `x` is free.
pub const TAG_START_POS: Vec2 = Vec2::new(1_060.0, 300.0);
const TAG_ORBIT_CENTER: Vec2 = Vec2::new(1_060.0, 4_300.0);

/// TwinTrack is intentionally an open relativity laboratory. The room still
/// owns a finite authored coordinate frame for content placement, but camera
/// follow must not become pinned to that rectangle once the controlled body
/// leaves it. This camera zone is deliberately enormous rather than tied to
/// the room bounds; at the demo's 540-unit/s terminal speed its edge is many
/// hours of continuous flight away.
const OPEN_CAMERA_HALF_SPAN: f32 = 10_000_000.0;
const TAG_ORBIT_RADIUS: f32 = 4_000.0;
const TAG_SPEED: f32 = 60.0;

pub const TRAVELER_RECEIVER_CHANNEL: u8 = 0;
pub const COURIER_RECEIVER_CHANNEL: u8 = 10;
pub const DRIFTER_RECEIVER_CHANNEL: u8 = 11;
pub const SPINNER_RECEIVER_CHANNEL: u8 = 12;
pub const DJ_RECEIVER_CHANNEL: u8 = 13;
pub const TAG_RECEIVER_CHANNEL: u8 = 14;

pub const TRAVELER_TAG: u64 = 100;
pub const COURIER_ID: u8 = 1;
pub const DRIFTER_ID: u8 = 2;
pub const SPINNER_ID: u8 = 3;
pub const DJ_ID: u8 = 4;
pub const TAGGER_ID: u8 = 5;

const SIGNAL_POOL_LABEL: &str = "twintrack_plaza_signals";
const SIGNAL_POOL_SIZE: u16 = 32;
const CLOCK_REPORT_MASK: u8 = 0b111;
const INTERACTION_RADIUS: f32 = 180.0;
const VIEW_CONSOLE_RADIUS: f32 = 110.0;
const LAB_REUNION_RADIUS: f32 = 125.0;
const INTRO_DRIFT_DISTANCE: f32 = 230.0;
const INTRO_RELATIVISTIC_SPEED_FRACTION: f32 = 0.50;
pub const LIGHT_TAG_ROUNDS: u32 = 3;
pub const TWINTRACK_WORLDLINE_HISTORY_SAMPLES: usize = 12_000;

const SFX_CLOCK_REPORT: &str = "twintrack.clock_report";
const SFX_DOPPLER_G2: &str = "twintrack.doppler.g2";
const SFX_DOPPLER_B2: &str = "twintrack.doppler.b2";
const SFX_DOPPLER_D3: &str = "twintrack.doppler.d3";
const SFX_DOPPLER_F3: &str = "twintrack.doppler.f3";
const SFX_DOPPLER_G3: &str = "twintrack.doppler.g3";
const SFX_TAG_HIT: &str = "twintrack.light_tag_hit";

const PAYLOAD_KIND_SHIFT: u32 = 56;
const PAYLOAD_ACTOR_SHIFT: u32 = 48;
const PAYLOAD_DATA_MASK: u64 = (1_u64 << 48) - 1;
const PAYLOAD_ASK_CLOCK: u8 = 1;
const PAYLOAD_CLOCK_REPORT: u8 = 2;
const PAYLOAD_DOPPLER_NOTE: u8 = 3;
const PAYLOAD_LIGHT_TAG: u8 = 4;

pub const OBJECTIVE_HUD_SLOT: &str = "twintrack_objective";
pub const CLOCKS_HUD_SLOT: &str = "twintrack_clocks";
pub const DIALOGUE_HUD_SLOT: &str = "twintrack_dialogue";
pub const TEACHER_HUD_SLOT: &str = "twintrack_teacher";
pub const RESULT_HUD_SLOT: &str = "twintrack_result";

const TWINTRACK_CATALOG_RON: &str = r#"(
    brain_presets: { "stand_still": StandStill },
    action_set_presets: {
        "peaceful": (
            move_style: Float,
            melee: None,
            ranged: None,
            special: None,
        ),
    },
    characters: {
        "twintrack_lab_twin": (
            display_name: "Emmy No-Ether",
            spritesheet: "sprites/noether_spritesheet.png",
            manifest: "sprites/noether_spritesheet.ron",
            tier: MainHall,
            body_kind: Floating,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            abilities: Some([FreeFlight]),
            axis_tuning: Some((
                horizontal_law: Responsive,
                jump_law: VelocityCut,
                gravity: 0.0,
                air_jumps: 0,
                jump_speed: 0.0,
                max_run_speed: 540.0,
                run_accel: 960.0,
                air_accel: 960.0,
                max_fall_speed: 540.0,
                coyote_time: 0.0,
                jump_buffer: 0.0,
                flight_accel: 700.0,
                flight_drag: 180.0,
                flight_terminal_speed: 540.0,
                flight_direct_velocity: false,
                flight_invariant_speed: Some(600.0),
            )),
            max_health: Some(1),
            tags: ["participant", "relativity", "free_flight", "plaza"],
            barks: (
                hall: [
                    "Stay put and my clock is the plaza's clock.",
                    "Every symmetry here hides a conservation law.",
                ],
            ),
        ),
        "twintrack_traveler": (
            display_name: "TwinTrack Traveler",
            spritesheet: "sprites/patent_clerk_spritesheet.png",
            manifest: "sprites/patent_clerk_spritesheet.ron",
            tier: MainHall,
            body_kind: Floating,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            abilities: Some([FreeFlight]),
            axis_tuning: Some((
                horizontal_law: Responsive,
                jump_law: VelocityCut,
                gravity: 0.0,
                air_jumps: 0,
                jump_speed: 0.0,
                max_run_speed: 540.0,
                run_accel: 960.0,
                air_accel: 960.0,
                max_fall_speed: 540.0,
                coyote_time: 0.0,
                jump_buffer: 0.0,
                flight_accel: 700.0,
                flight_drag: 180.0,
                flight_terminal_speed: 540.0,
                flight_direct_velocity: false,
                flight_invariant_speed: Some(600.0),
            )),
            max_health: Some(1),
            tags: ["participant", "relativity", "free_flight", "plaza"],
            barks: (
                hall: [
                    "Ask everyone what their own clock reads.",
                    "What reaches you now left them earlier.",
                    "Lead the visible image in light tag.",
                ],
            ),
        ),
    },
)"#;

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct TravelerTwin;

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct LaboratoryTwin;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TwinTrackRole {
    ClockCitizen,
    DopplerDj,
    LightTagger,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TwinTrackTrajectory {
    Stationary,
    Orbit {
        center: Vec2,
        radius: f32,
        angular_speed: f32,
        phase: f32,
    },
}

#[derive(Component, Clone, Debug, PartialEq)]
pub struct TwinTrackCharacter {
    pub id: u8,
    pub label: String,
    pub role: TwinTrackRole,
    pub receiver_channel: u8,
    pub trajectory: TwinTrackTrajectory,
}

impl TwinTrackCharacter {
    fn clock_citizen(
        id: u8,
        label: &str,
        receiver_channel: u8,
        center: Vec2,
        radius: f32,
        speed: f32,
        phase: f32,
    ) -> Self {
        Self {
            id,
            label: label.to_owned(),
            role: TwinTrackRole::ClockCitizen,
            receiver_channel,
            trajectory: TwinTrackTrajectory::Orbit {
                center,
                radius,
                angular_speed: speed / radius,
                phase,
            },
        }
    }

    fn stationary(id: u8, label: &str, role: TwinTrackRole, receiver_channel: u8) -> Self {
        Self {
            id,
            label: label.to_owned(),
            role,
            receiver_channel,
            trajectory: TwinTrackTrajectory::Stationary,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TwinTrackViewMode {
    #[default]
    Laboratory,
    Optical,
    Spacetime,
    /// Two observers side by side, each pane resolved in its own frame.
    ///
    /// appended on purpose. The discriminant is the snapshot byte, so a
    /// new variant may only be added at the end or every stored view mode
    /// shifts under an old save.
    SplitObservers,
}

impl TwinTrackViewMode {
    fn next(self) -> Self {
        match self {
            // The split observers pane is the relativity exhibit, so it is the
            // first thing the view console offers.
            Self::Laboratory => Self::SplitObservers,
            Self::SplitObservers => Self::Optical,
            Self::Optical | Self::Spacetime => Self::Laboratory,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TwinTrackIntroStep {
    #[default]
    Synchronize,
    Drift,
    Accelerate,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TwinTrackPhase {
    #[default]
    Introduction,
    ClockCensus,
    DopplerDance,
    LightTag,
    Reunion,
    Complete,
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct TwinTrackExperiment {
    pub phase: TwinTrackPhase,
    pub intro_step: TwinTrackIntroStep,
    pub view_mode: TwinTrackViewMode,
    pub aim_direction: Vec2,
    pub clock_report_mask: u8,
    pub last_speaker_id: u8,
    pub last_report_clock: f64,
    pub last_receive_clock: f64,
    pub last_receive_lab_time: f64,
    pub last_message_departure_time: f64,
    pub doppler_frequency: f64,
    pub tag_attempts: u32,
    pub tag_hits: u32,
    pub replay_cursor: f32,
    pub result_lab_time: f64,
    pub result_traveler_time: f64,
}

impl Default for TwinTrackExperiment {
    fn default() -> Self {
        Self {
            phase: TwinTrackPhase::Introduction,
            intro_step: TwinTrackIntroStep::Synchronize,
            view_mode: TwinTrackViewMode::Laboratory,
            aim_direction: Vec2::X,
            clock_report_mask: 0,
            last_speaker_id: 0,
            last_report_clock: 0.0,
            last_receive_clock: 0.0,
            last_receive_lab_time: 0.0,
            last_message_departure_time: 0.0,
            doppler_frequency: 0.0,
            tag_attempts: 0,
            tag_hits: 0,
            replay_cursor: 1.0,
            result_lab_time: 0.0,
            result_traveler_time: 0.0,
        }
    }
}

impl ambition_platformer2d::engine_core::snapshot::SnapshotState for TwinTrackExperiment {
    fn encode(&self, out: &mut Vec<u8>) {
        use ambition_platformer2d::engine_core::snapshot::{
            put_f32, put_u32, put_u64, put_u8, put_vec2,
        };
        put_u8(out, self.phase as u8);
        put_u8(out, self.intro_step as u8);
        put_u8(out, self.view_mode as u8);
        put_vec2(out, self.aim_direction);
        put_u8(out, self.clock_report_mask);
        put_u8(out, self.last_speaker_id);
        put_u64(out, self.last_report_clock.to_bits());
        put_u64(out, self.last_receive_clock.to_bits());
        put_u64(out, self.last_receive_lab_time.to_bits());
        put_u64(out, self.last_message_departure_time.to_bits());
        put_u64(out, self.doppler_frequency.to_bits());
        put_u32(out, self.tag_attempts);
        put_u32(out, self.tag_hits);
        put_f32(out, self.replay_cursor);
        put_u64(out, self.result_lab_time.to_bits());
        put_u64(out, self.result_traveler_time.to_bits());
    }

    fn decode(
        reader: &mut ambition_platformer2d::engine_core::snapshot::Reader<'_>,
    ) -> Option<Self> {
        let phase = match reader.u8()? {
            0 => TwinTrackPhase::Introduction,
            1 => TwinTrackPhase::ClockCensus,
            2 => TwinTrackPhase::DopplerDance,
            3 => TwinTrackPhase::LightTag,
            4 => TwinTrackPhase::Reunion,
            5 => TwinTrackPhase::Complete,
            _ => return None,
        };
        let intro_step = match reader.u8()? {
            0 => TwinTrackIntroStep::Synchronize,
            1 => TwinTrackIntroStep::Drift,
            2 => TwinTrackIntroStep::Accelerate,
            _ => return None,
        };
        let view_mode = match reader.u8()? {
            0 => TwinTrackViewMode::Laboratory,
            1 => TwinTrackViewMode::Optical,
            2 => TwinTrackViewMode::Spacetime,
            3 => TwinTrackViewMode::SplitObservers,
            _ => return None,
        };
        let value = Self {
            phase,
            intro_step,
            view_mode,
            aim_direction: reader.vec2()?,
            clock_report_mask: reader.u8()?,
            last_speaker_id: reader.u8()?,
            last_report_clock: f64::from_bits(reader.u64()?),
            last_receive_clock: f64::from_bits(reader.u64()?),
            last_receive_lab_time: f64::from_bits(reader.u64()?),
            last_message_departure_time: f64::from_bits(reader.u64()?),
            doppler_frequency: f64::from_bits(reader.u64()?),
            tag_attempts: reader.u32()?,
            tag_hits: reader.u32()?,
            replay_cursor: reader.f32()?.clamp(0.0, 1.0),
            result_lab_time: f64::from_bits(reader.u64()?),
            result_traveler_time: f64::from_bits(reader.u64()?),
        };
        Some(value)
    }
}

pub fn twintrack_room() -> RoomSpec {
    let size = ae::Vec2::new(ROOM_WIDTH, ROOM_HEIGHT);
    // TwinTrack is a relativity laboratory, not a platforming corridor. The
    // authored room remains a convenient coordinate frame for the clustered
    // exhibits, but collision and camera travel are deliberately open.
    let mut world = ae::World::new("TwinTrack Relativity Plaza", size, LAB_POS, Vec::new());
    // Zero-gravity free flight should not silently inherit a platformer's bottom
    // blast margin once the participant leaves the authored exhibit rectangle.
    world.edges.fall = OPEN_CAMERA_HALF_SPAN;
    let mut room = RoomSpec::new(TWINTRACK_ROOM_ID, world);
    room.metadata.mode = Some(TWINTRACK_EXPERIENCE.to_owned());
    room.camera_zones.push(CameraZoneSpec {
        id: "twintrack_open_follow".to_owned(),
        name: "TwinTrack open follow".to_owned(),
        aabb: ae::Aabb::new(LAB_POS, Vec2::splat(OPEN_CAMERA_HALF_SPAN)),
        priority: i32::MAX,
        zoom: Some(1.0),
        target_offset: Vec2::ZERO,
        easing_hz: None,
        cinematic_lock: false,
        clamp_mode: CameraClampMode::None,
        scroll_policy: CameraScrollPolicy::Free,
    });
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
        use ambition_platformer2d::actors::character_runtime::CharacterDefinitionAppExt;
        use ambition_platformer2d::character::CharacterDefinition;
        app.register_character(
            CharacterDefinition::new(
                TWINTRACK_CHARACTER_ID,
                "TwinTrack Traveler",
                TWINTRACK_EXPERIENCE,
            )
            .with_sheet("patent_clerk")
            .with_voice([
                "Ask what their own clocks read.",
                "The visible image is old light.",
            ]),
        );
        app.register_character(
            CharacterDefinition::new(
                TWINTRACK_LAB_TWIN_CHARACTER_ID,
                "Emmy No-Ether",
                TWINTRACK_EXPERIENCE,
            )
            .with_sheet("noether")
            // a character has to author its own locomotion to be BUILT.
            // The traveler never needed this: the starting character is worn by
            // the session's home avatar, which brings its own body. A second
            // participant goes through ordinary actor construction, and that
            // road refuses a character that cannot say how it moves rather than
            // inheriting somebody's idea of a walk.
            .with_locomotion(
                ambition_platformer2d::characters::actor::CharacterLocomotion {
                    run_speed: 540.0,
                    move_style: ambition_platformer2d::characters::brain::MoveStyleSpec::Float,
                    // The plaza has no gravity. Silence would resolve to GROUNDED,
                    // which is a body falling forever through an open room.
                    baseline_free_flight: Some(true),
                    ..Default::default()
                },
            )
            .with_voice([
                "I am the frame everybody else is moving in.",
                "My clock is the plaza's, by construction.",
            ]),
        );
    }
    app.register_audio_catalog_fragment(
        AudioCatalogFragment::new(
            TWINTRACK_EXPERIENCE,
            None,
            Some(ambition_platformer2d::audio::spec::SfxRegistry {
                sample_rate: 44_100,
                sfx: twintrack_sfx_specs(),
            }),
        )
        .expect("TwinTrack procedural teaching cues should be valid"),
    );
}

fn twintrack_sfx_specs() -> Vec<ambition_platformer2d::audio::spec::SfxSpec> {
    use ambition_platformer2d::audio::spec::WaveformSpec as Wave;
    vec![
        twintrack_sfx(SFX_CLOCK_REPORT, Wave::Triangle, 720.0, 920.0, 0.12, 0.24),
        twintrack_sfx(SFX_DOPPLER_G2, Wave::Sine, 98.0, 98.0, 0.24, 0.30),
        twintrack_sfx(SFX_DOPPLER_B2, Wave::Sine, 123.47, 123.47, 0.24, 0.30),
        twintrack_sfx(SFX_DOPPLER_D3, Wave::Sine, 146.83, 146.83, 0.24, 0.30),
        twintrack_sfx(SFX_DOPPLER_F3, Wave::Sine, 174.61, 174.61, 0.24, 0.30),
        twintrack_sfx(SFX_DOPPLER_G3, Wave::Triangle, 196.0, 392.0, 0.34, 0.34),
        twintrack_sfx(SFX_TAG_HIT, Wave::Square, 560.0, 1_120.0, 0.16, 0.28),
    ]
}

fn twintrack_sfx(
    id: &str,
    waveform: ambition_platformer2d::audio::spec::WaveformSpec,
    frequency: f32,
    frequency_end: f32,
    duration: f32,
    volume: f32,
) -> ambition_platformer2d::audio::spec::SfxSpec {
    ambition_platformer2d::audio::spec::SfxSpec {
        cue: None,
        id: Some(id.to_owned()),
        waveform,
        frequency,
        frequency_end,
        duration,
        volume,
        attack: 0.008,
        release: (duration * 0.35).min(0.08),
        noise: 0.0,
    }
}

fn doppler_feedback_sfx(frequency: f64, accepted: bool) -> &'static str {
    if accepted {
        return SFX_DOPPLER_G3;
    }
    if frequency < 111.0 {
        SFX_DOPPLER_G2
    } else if frequency < 135.0 {
        SFX_DOPPLER_B2
    } else if frequency < 161.0 {
        SFX_DOPPLER_D3
    } else {
        SFX_DOPPLER_F3
    }
}

pub struct TwinTrackExperiencePlugin;

impl Plugin for TwinTrackExperiencePlugin {
    fn build(&self, app: &mut App) {
        install_twintrack_content(app);
        if !app.is_plugin_added::<Relativity2dPlugin>() {
            app.add_plugins(Relativity2dPlugin);
        }
        if app
            .world()
            .get_resource::<ambition_platformer2d::runtime::SimulationHost>()
            .copied()
            .is_some_and(|host| host.is_rollback())
        {
            let mut registrar = ambition_platformer2d::rollback::GgrsRollbackRegistrar::new(app);
            ambition_platformer2d::relativity2d::register_rollback_state(&mut registrar);
        }
        app.rollback_component_canonical::<TwinTrackExperiment>(
            "ambition_demo_twintrack",
            "twintrack.experiment",
        )
        .rollback_component_clone::<TravelerTwin>("ambition_demo_twintrack", "twintrack.traveler")
        .rollback_component_clone::<LaboratoryTwin>(
            "ambition_demo_twintrack",
            "twintrack.laboratory",
        )
        .rollback_component_clone_probed::<TwinTrackCharacter>(
            "ambition_demo_twintrack",
            "twintrack.character",
            |character| u64::from(character.id),
        );

        PlatformerExperienceAuthoring::new(
            TWINTRACK_EXPERIENCE,
            TWINTRACK_GAMEPLAY_ROUTE,
            "TwinTrack",
            "Meet moving characters, exchange light-delayed clock reports, \
             Doppler-shift a song, and play light tag",
            "Prepare TwinTrack",
            AuthoredCatalogFragments::new(TWINTRACK_CHARACTER_ID, TWINTRACK_EXPERIENCE),
        )
        .with_presentation_profiles(
            ambition_platformer2d::presentation::profiles::high_speed_full_bleed(),
        )
        .with_hud(
            ambition_platformer2d::presentation::HudDeclaration::new()
                .slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(OBJECTIVE_HUD_SLOT)
                        .with_region(ambition_platformer2d::presentation::SurroundRegion::Top)
                        .with_font_size(16.0),
                )
                .slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(DIALOGUE_HUD_SLOT)
                        .with_region(ambition_platformer2d::presentation::SurroundRegion::Top)
                        .with_order(2)
                        .with_font_size(15.0),
                )
                .slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(CLOCKS_HUD_SLOT)
                        .with_region(ambition_platformer2d::presentation::SurroundRegion::Bottom)
                        .with_font_size(18.0),
                )
                .slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(TEACHER_HUD_SLOT)
                        .with_region(ambition_platformer2d::presentation::SurroundRegion::Bottom)
                        .with_order(2)
                        .with_font_size(14.0),
                )
                .slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(RESULT_HUD_SLOT)
                        .with_region(ambition_platformer2d::presentation::SurroundRegion::Bottom)
                        .with_order(3)
                        .with_font_size(17.0),
                ),
        )
        .with_defense_presentation(
            ambition_platformer2d::presentation::DefensePresentationPolicy::shared_iframe_blink(),
        )
        .install(app, twintrack_prepared_session_world);

        participants::install(app);

        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            (
                participants::adopt_the_laboratory_twin,
                participants::restore_the_laboratory_twins_mark,
            )
                .run_if(ambition_platformer2d::runtime::in_mode(
                    TWINTRACK_EXPERIENCE,
                ))
                .before(Relativity2dSet::AdvanceCoordinateTime)
                .in_set(WorldPrepSet::BeforeIntegrate),
        )
        .add_systems(
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
            chase_beacon::update_twintrack_character_worldlines
                .run_if(ambition_platformer2d::runtime::in_mode(
                    TWINTRACK_EXPERIENCE,
                ))
                .after(Relativity2dSet::AdvanceCoordinateTime)
                .before(Relativity2dSet::ResolveClocks)
                .in_set(WorldPrepSet::BeforeIntegrate),
        )
        .add_systems(
            sim,
            capture_twintrack_interaction
                .run_if(ambition_platformer2d::runtime::in_mode(
                    TWINTRACK_EXPERIENCE,
                ))
                // ⛔ THE LEAF SET ALONE. Naming the parent phase BESIDE it was
                // redundant while `ControlGate` lived in `PlayerInput` and became
                // a schedule-build error the moment it moved to `WorldPrep`
                // (D202) — the system was then in two ordered phases at once.
                // A membership declares itself; restating its parent pins a fact
                // that is not this call site's to know.
                .in_set(PlayerInputSet::ControlGate),
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

        // an `Update` read model, not a simulation system. It writes no
        // canonical state and holds no memo, so it is deliberately absent from
        // the rollback registration above; see `TwinTrackDualObserverView`.
        app.init_resource::<TwinTrackDualObserverView>()
            .init_resource::<TwinTrackLightPulseView>()
            .add_systems(
                Update,
                (
                    dual_observer::publish_dual_observer_view,
                    light_pulse::publish_light_pulse_view,
                )
                    .run_if(ambition_platformer2d::runtime::in_mode(
                        TWINTRACK_EXPERIENCE,
                    ))
                    .before(publish_twintrack_hud),
            );

        #[cfg(feature = "visible")]
        {
            observatory::install(app);
            split_screen::install(app);
            spacetime_3d::install(app);
        }
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
    )
}

fn install_twintrack_session(
    mut commands: Commands,
    mut spawns: MessageWriter<ambition_platformer2d::actor::SpawnActorRequest>,
    mut worldlines: ResMut<WorldlineHistoryView2d>,
    roots: Query<(Entity, &SessionRoot, &ActiveRoomMetadata), Without<ActiveSpacetime2d>>,
    traveler: Query<
        Entity,
        (
            With<ambition_platformer2d::actor::PrimaryPlayer>,
            Without<TravelerTwin>,
        ),
    >,
) {
    let (Ok((root_entity, root, metadata)), Ok(traveler_entity)) =
        (roots.single(), traveler.single())
    else {
        return;
    };
    if metadata.0.mode.as_deref() != Some(TWINTRACK_EXPERIENCE) {
        return;
    }

    worldlines.capacity_per_track = TWINTRACK_WORLDLINE_HISTORY_SAMPLES;

    commands.entity(root_entity).insert((
        ActiveSpacetime2d::minkowski(INVARIANT_SPEED as f64)
            .expect("TwinTrack invariant speed is finite and positive"),
        SpacetimeCoordinateTime2d::default(),
        SignalArrivalHistory2d {
            capacity: 256,
            ..default()
        },
    ));
    commands.entity(traveler_entity).insert((
        TravelerTwin,
        ae::BodyFlightState {
            fly_enabled: true,
            ..default()
        },
        RelativisticClock2d,
        RelativityClockLabel("traveler".to_owned()),
        WorldlineTracked2d::new("traveler"),
        RelativisticObserver2d("traveler".to_owned()),
        LightEmitter2d::new(
            "traveler_transmitter",
            SIGNAL_POOL_LABEL,
            EMITTED_FREQUENCY,
            TRANSMITTER_COOLDOWN_PROPER_SECONDS,
        )
        .with_tag(TRAVELER_TAG)
        .with_source_receiver_channel(TRAVELER_RECEIVER_CHANNEL),
        ProperTimeCooldown2d::default(),
        LightReceiver2d::observer(
            "traveler_messages",
            TRAVELER_RECEIVER_CHANNEL,
            Vec2::splat(72.0),
        )
        .consuming(),
    ));

    spawns.write(participants::laboratory_twin_request());

    spawn_character(
        &mut commands,
        root.0,
        TwinTrackCharacter::clock_citizen(
            COURIER_ID,
            "Courier",
            COURIER_RECEIVER_CHANNEL,
            Vec2::new(455.0, 310.0),
            110.0,
            210.0,
            0.0,
        ),
        150.0,
        1.0,
        19.0,
    );
    spawn_character(
        &mut commands,
        root.0,
        TwinTrackCharacter::clock_citizen(
            DRIFTER_ID,
            "Drifter",
            DRIFTER_RECEIVER_CHANNEL,
            Vec2::new(720.0, 300.0),
            125.0,
            330.0,
            1.7,
        ),
        205.0,
        1.15,
        21.0,
    );
    spawn_character(
        &mut commands,
        root.0,
        TwinTrackCharacter::clock_citizen(
            SPINNER_ID,
            "Spinner",
            SPINNER_RECEIVER_CHANNEL,
            Vec2::new(980.0, 320.0),
            115.0,
            450.0,
            3.1,
        ),
        260.0,
        1.3,
        22.0,
    );

    let dj = TwinTrackCharacter::stationary(
        DJ_ID,
        "DJ Blue Shift",
        TwinTrackRole::DopplerDj,
        DJ_RECEIVER_CHANNEL,
    );
    spawn_character_at(
        &mut commands,
        root.0,
        dj,
        DJ_POS,
        320.0,
        1.4,
        24.0,
        Some((DOPPLER_PASSBAND_MIN, DOPPLER_PASSBAND_MAX)),
        true,
    );

    let tagger = TwinTrackCharacter {
        id: TAGGER_ID,
        label: "Photon Fox".to_owned(),
        role: TwinTrackRole::LightTagger,
        receiver_channel: TAG_RECEIVER_CHANNEL,
        trajectory: TwinTrackTrajectory::Orbit {
            // A huge-radius arc is effectively inertial over the light-tag
            // encounter, so the engine's constant-velocity null-intercept
            // solution remains an honest predictor while the character stays
            // inside the finite plaza for a useful play window.
            center: TAG_ORBIT_CENTER,
            radius: TAG_ORBIT_RADIUS,
            angular_speed: TAG_SPEED / TAG_ORBIT_RADIUS,
            phase: -std::f32::consts::FRAC_PI_2,
        },
    };
    let tag_entity = spawn_character_at(
        &mut commands,
        root.0,
        tagger,
        TAG_START_POS,
        220.0,
        1.6,
        25.0,
        None,
        true,
    );
    commands
        .entity(tag_entity)
        .insert(RelativisticTarget2d("Photon Fox".to_owned()));

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

fn spawn_character(
    commands: &mut Commands,
    // a `SessionScopeId`, not an `Entity` — `SessionRoot` stopped carrying a
    // raw handle, and `SessionScopedEntity` is what consumes this.
    root: SessionScopeId,
    character: TwinTrackCharacter,
    rest_frequency: f64,
    intensity: f32,
    radius: f32,
) -> Entity {
    let position = match character.trajectory {
        TwinTrackTrajectory::Stationary => Vec2::ZERO,
        TwinTrackTrajectory::Orbit {
            center,
            radius,
            phase,
            ..
        } => center + Vec2::from_angle(phase) * radius,
    };
    spawn_character_at(
        commands,
        root,
        character,
        position,
        rest_frequency,
        intensity,
        radius,
        None,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn spawn_character_at(
    commands: &mut Commands,
    // A `SessionScopeId`, not an `Entity` — see `spawn_character`.
    root: SessionScopeId,
    character: TwinTrackCharacter,
    position: Vec2,
    rest_frequency: f64,
    intensity: f32,
    radius: f32,
    passband: Option<(f64, f64)>,
    consuming: bool,
) -> Entity {
    let receiver_radius = match character.role {
        TwinTrackRole::ClockCitizen => radius.max(72.0),
        TwinTrackRole::DopplerDj => radius.max(50.0),
        TwinTrackRole::LightTagger => radius.max(42.0),
    };
    let receiver = LightReceiver2d::observer(
        format!("{}_receiver", character.label),
        character.receiver_channel,
        Vec2::splat(receiver_radius),
    );
    let receiver = if let Some((minimum, maximum)) = passband {
        receiver.with_passband(minimum, maximum)
    } else {
        receiver
    };
    let receiver = if consuming {
        receiver.consuming()
    } else {
        receiver
    };
    let stable = format!("twintrack_character_{}", character.id);
    commands
        .spawn((
            character.clone(),
            RelativisticClock2d,
            RelativityClockLabel(character.label.clone()),
            WorldlineTracked2d::new(character.label.clone()),
            OpticalSource2d::new(character.label.clone(), rest_frequency, intensity, radius),
            LightEmitter2d::new(
                format!("{}_transmitter", character.label),
                SIGNAL_POOL_LABEL,
                EMITTED_FREQUENCY,
                0.0,
            )
            .with_tag(u64::from(character.id))
            .with_source_receiver_channel(character.receiver_channel),
            ProperTimeCooldown2d::default(),
            receiver,
            ae::BodyKinematics {
                pos: position,
                vel: Vec2::ZERO,
                size: Vec2::splat(radius * 1.3),
                facing: 1.0,
            },
            SessionScopedEntity(root),
            ambition_platformer2d::platformer::sim_id::SimId::placement(&stable),
        ))
        .id()
}

fn payload(kind: u8, actor_id: u8, data: u64) -> u64 {
    (u64::from(kind) << PAYLOAD_KIND_SHIFT)
        | (u64::from(actor_id) << PAYLOAD_ACTOR_SHIFT)
        | (data & PAYLOAD_DATA_MASK)
}

fn payload_parts(value: u64) -> (u8, u8, u64) {
    (
        (value >> PAYLOAD_KIND_SHIFT) as u8,
        (value >> PAYLOAD_ACTOR_SHIFT) as u8,
        value & PAYLOAD_DATA_MASK,
    )
}

fn encoded_clock_seconds(seconds: f64) -> u64 {
    (seconds.max(0.0) * 1_000.0).round() as u64
}

fn decoded_clock_seconds(milliseconds: u64) -> f64 {
    milliseconds as f64 / 1_000.0
}

fn character_name(id: u8) -> &'static str {
    match id {
        COURIER_ID => "Courier",
        DRIFTER_ID => "Drifter",
        SPINNER_ID => "Spinner",
        DJ_ID => "DJ Blue Shift",
        TAGGER_ID => "Photon Fox",
        _ => "Unknown traveler",
    }
}

fn report_bit(id: u8) -> u8 {
    match id {
        COURIER_ID => 0b001,
        DRIFTER_ID => 0b010,
        SPINNER_ID => 0b100,
        _ => 0,
    }
}

fn coordinate_direction_to_moving_target(source: Vec2, target: &ae::BodyKinematics) -> Vec2 {
    let relative = target.pos - source;
    let Ok(c) = InvariantSpeed::new(f64::from(INVARIANT_SPEED)) else {
        return relative.normalize_or_zero();
    };
    solve_null_intercept_constant_velocity(
        [f64::from(relative.x), f64::from(relative.y), 0.0],
        [f64::from(target.vel.x), f64::from(target.vel.y), 0.0],
        c,
    )
    .map(|solution| {
        Vec2::new(
            solution.emission_direction[0] as f32,
            solution.emission_direction[1] as f32,
        )
    })
    .unwrap_or_else(|| relative.normalize_or_zero())
}

pub(crate) fn musical_note_name(frequency: f64) -> String {
    if !frequency.is_finite() || frequency <= 0.0 {
        return "—".to_owned();
    }
    const NAMES: [&str; 12] = [
        "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B",
    ];
    let midi = (69.0 + 12.0 * (frequency / 440.0).log2()).round() as i32;
    let name = NAMES[midi.rem_euclid(12) as usize];
    let octave = midi.div_euclid(12) - 1;
    format!("{name}{octave}")
}

pub(crate) fn doppler_frequency_for_target(
    source: &ae::BodyKinematics,
    target: &ae::BodyKinematics,
) -> Option<f64> {
    let c = InvariantSpeed::new(f64::from(INVARIANT_SPEED)).ok()?;
    let direction = coordinate_direction_to_moving_target(source.pos, target);
    minkowski_doppler_measurement(
        EMITTED_FREQUENCY,
        [f64::from(direction.x), f64::from(direction.y), 0.0],
        [f64::from(source.vel.x), f64::from(source.vel.y), 0.0],
        [f64::from(target.vel.x), f64::from(target.vel.y), 0.0],
        c,
    )
    .map(|measurement| measurement.observed_frequency)
}

#[allow(clippy::too_many_arguments)]
fn capture_twintrack_interaction(
    traveler: Query<
        (
            Entity,
            &ambition_platformer2d::characters::control::ActorControl,
            &ae::BodyKinematics,
            &ProperTimeElapsed,
        ),
        With<TravelerTwin>,
    >,
    characters: Query<(Entity, &TwinTrackCharacter, &ae::BodyKinematics)>,
    lab: Query<(&ae::BodyKinematics, &ProperTimeElapsed), With<LaboratoryTwin>>,
    coordinate_time: Query<&SpacetimeCoordinateTime2d>,
    mut experiment: Query<&mut TwinTrackExperiment, With<LaboratoryTwin>>,
    mut requests: MessageWriter<LightEmissionRequest2d>,
) {
    let (
        Ok((traveler_entity, control, traveler_body, traveler_clock)),
        Ok(mut experiment),
        Ok((lab_body, lab_clock)),
    ) = (traveler.single(), experiment.single_mut(), lab.single())
    else {
        return;
    };

    let direct_aim = Vec2::new(control.0.aim.x, -control.0.aim.y);
    let movement_aim = Vec2::new(control.0.locomotion.x, control.0.locomotion.y);
    if direct_aim.length_squared() > 0.04 {
        experiment.aim_direction = direct_aim.normalize();
    } else if movement_aim.length_squared() > 0.04 {
        experiment.aim_direction = movement_aim.normalize();
    } else if traveler_body.vel.length_squared() > 1.0 {
        experiment.aim_direction = traveler_body.vel.normalize();
    }

    if experiment.phase == TwinTrackPhase::Complete && control.0.locomotion.x.abs() > 0.08 {
        experiment.replay_cursor =
            (experiment.replay_cursor + control.0.locomotion.x * 0.008).clamp(0.0, 1.0);
    }

    if experiment.phase == TwinTrackPhase::Introduction {
        match experiment.intro_step {
            TwinTrackIntroStep::Synchronize => {
                if control.0.interact_pressed
                    && traveler_body.pos.distance(lab_body.pos) <= LAB_REUNION_RADIUS
                {
                    experiment.intro_step = TwinTrackIntroStep::Drift;
                    experiment.last_speaker_id = 0;
                }
            }
            TwinTrackIntroStep::Drift => {
                if traveler_body.pos.distance(lab_body.pos) >= INTRO_DRIFT_DISTANCE {
                    experiment.intro_step = TwinTrackIntroStep::Accelerate;
                }
            }
            TwinTrackIntroStep::Accelerate => {
                if traveler_body.vel.length() >= INVARIANT_SPEED * INTRO_RELATIVISTIC_SPEED_FRACTION
                {
                    experiment.phase = TwinTrackPhase::ClockCensus;
                    experiment.last_speaker_id = 0;
                }
            }
        }
        return;
    }

    if !control.0.interact_pressed {
        return;
    }

    // The 3D spacetime teaching surface is now a default-on minimap and never
    // replaces gameplay. Keep the historical Spacetime enum decode-compatible,
    // but normalize old/restored state back to the laboratory view.
    if experiment.view_mode == TwinTrackViewMode::Spacetime {
        experiment.view_mode = TwinTrackViewMode::Laboratory;
    }
    if experiment.view_mode == TwinTrackViewMode::Optical
        && experiment.phase != TwinTrackPhase::LightTag
    {
        experiment.view_mode = TwinTrackViewMode::Laboratory;
        return;
    }

    if traveler_body.pos.distance(VIEW_CONSOLE_POS) <= VIEW_CONSOLE_RADIUS {
        experiment.view_mode = experiment.view_mode.next();
        return;
    }

    match experiment.phase {
        TwinTrackPhase::Introduction => {}
        TwinTrackPhase::ClockCensus => {
            let nearest = characters
                .iter()
                .filter(|(_, character, _)| character.role == TwinTrackRole::ClockCitizen)
                .map(|row @ (_, _, body)| (body.pos.distance(traveler_body.pos), row))
                .filter(|(distance, _)| *distance <= INTERACTION_RADIUS)
                .min_by(|(left, _), (right, _)| left.total_cmp(right));
            let Some((_, (_, character, body))) = nearest else {
                return;
            };
            let direction = coordinate_direction_to_moving_target(traveler_body.pos, body);
            requests.write(
                LightEmissionRequest2d::new(traveler_entity, direction)
                    .to_receiver(character.receiver_channel)
                    .with_payload(payload(PAYLOAD_ASK_CLOCK, character.id, 0)),
            );
        }
        TwinTrackPhase::DopplerDance => {
            let Some((_, dj, body)) = characters
                .iter()
                .find(|(_, character, _)| character.role == TwinTrackRole::DopplerDj)
            else {
                return;
            };
            let direction = coordinate_direction_to_moving_target(traveler_body.pos, body);
            requests.write(
                LightEmissionRequest2d::new(traveler_entity, direction)
                    .to_receiver(dj.receiver_channel)
                    .with_payload(payload(PAYLOAD_DOPPLER_NOTE, dj.id, 0)),
            );
        }
        TwinTrackPhase::LightTag => {
            let Ok(c) = InvariantSpeed::new(f64::from(INVARIANT_SPEED)) else {
                return;
            };
            let local = experiment.aim_direction.normalize_or_zero();
            if local == Vec2::ZERO {
                return;
            }
            let coordinate = coordinate_photon_direction_from_observer(
                [f64::from(local.x), f64::from(local.y), 0.0],
                [
                    f64::from(traveler_body.vel.x),
                    f64::from(traveler_body.vel.y),
                    0.0,
                ],
                c,
            )
            .map(|direction| Vec2::new(direction[0] as f32, direction[1] as f32))
            .unwrap_or(local);
            requests.write(
                LightEmissionRequest2d::new(traveler_entity, coordinate)
                    .to_receiver(TAG_RECEIVER_CHANNEL)
                    .with_payload(payload(PAYLOAD_LIGHT_TAG, TAGGER_ID, 0)),
            );
            experiment.tag_attempts = experiment.tag_attempts.saturating_add(1);
            experiment.last_message_departure_time = coordinate_time
                .single()
                .map_or(experiment.last_message_departure_time, |time| time.seconds);
            if experiment.view_mode == TwinTrackViewMode::Optical {
                experiment.view_mode = TwinTrackViewMode::Laboratory;
            }
        }
        TwinTrackPhase::Reunion => {
            if traveler_body.pos.distance(lab_body.pos) <= LAB_REUNION_RADIUS {
                experiment.phase = TwinTrackPhase::Complete;
                experiment.replay_cursor = 1.0;
                experiment.result_lab_time = lab_clock.seconds;
                experiment.result_traveler_time = traveler_clock.seconds;
            }
        }
        TwinTrackPhase::Complete => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn consume_twintrack_signal_arrivals(
    mut arrivals: MessageReader<SignalArrival2d>,
    traveler: Query<(Entity, &ae::BodyKinematics, &ProperTimeElapsed), With<TravelerTwin>>,
    characters: Query<(
        Entity,
        &TwinTrackCharacter,
        &ae::BodyKinematics,
        &ProperTimeElapsed,
    )>,
    lab_clock: Query<&ProperTimeElapsed, With<LaboratoryTwin>>,
    mut experiment: Query<&mut TwinTrackExperiment, With<LaboratoryTwin>>,
    mut requests: MessageWriter<LightEmissionRequest2d>,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
) {
    let (Ok((traveler_entity, traveler_body, traveler_clock)), Ok(lab_clock), Ok(mut experiment)) = (
        traveler.single(),
        lab_clock.single(),
        experiment.single_mut(),
    ) else {
        return;
    };

    for SignalArrival2d(arrival) in arrivals.read() {
        let (kind, actor_id, data) = payload_parts(arrival.payload);
        match kind {
            PAYLOAD_ASK_CLOCK => {
                let Some((entity, character, body, clock)) =
                    characters.iter().find(|(_, character, _, _)| {
                        character.id == actor_id
                            && character.receiver_channel == arrival.receiver_channel
                    })
                else {
                    continue;
                };
                let reported = arrival.receiver_proper_time.unwrap_or(clock.seconds);
                let direction = coordinate_direction_to_moving_target(body.pos, traveler_body);
                requests.write(
                    LightEmissionRequest2d::new(entity, direction)
                        .to_receiver(TRAVELER_RECEIVER_CHANNEL)
                        .with_payload(payload(
                            PAYLOAD_CLOCK_REPORT,
                            character.id,
                            encoded_clock_seconds(reported),
                        )),
                );
            }
            PAYLOAD_CLOCK_REPORT if arrival.receiver_channel == TRAVELER_RECEIVER_CHANNEL => {
                experiment.clock_report_mask |= report_bit(actor_id);
                experiment.last_speaker_id = actor_id;
                experiment.last_report_clock = decoded_clock_seconds(data);
                experiment.last_receive_clock = arrival
                    .receiver_proper_time
                    .unwrap_or(traveler_clock.seconds);
                experiment.last_receive_lab_time = lab_clock.seconds;
                experiment.last_message_departure_time = arrival.signal_emission_time;
                sfx.write_from(
                    TWINTRACK_EXPERIENCE,
                    ambition_platformer2d::sfx::SfxMessage::Play {
                        id: ambition_platformer2d::sfx::SfxId::from_static(SFX_CLOCK_REPORT),
                        pos: arrival.position,
                    },
                );
                if experiment.clock_report_mask == CLOCK_REPORT_MASK {
                    experiment.phase = TwinTrackPhase::DopplerDance;
                }
            }
            PAYLOAD_DOPPLER_NOTE if arrival.receiver_channel == DJ_RECEIVER_CHANNEL => {
                experiment.doppler_frequency = arrival.observed_frequency;
                experiment.last_speaker_id = DJ_ID;
                experiment.last_report_clock = arrival.receiver_proper_time.unwrap_or_default();
                experiment.last_receive_lab_time = lab_clock.seconds;
                experiment.last_message_departure_time = arrival.signal_emission_time;
                let cue = doppler_feedback_sfx(arrival.observed_frequency, arrival.accepted);
                sfx.write_from(
                    TWINTRACK_EXPERIENCE,
                    ambition_platformer2d::sfx::SfxMessage::Play {
                        id: ambition_platformer2d::sfx::SfxId::from_static(cue),
                        pos: arrival.position,
                    },
                );
                if arrival.accepted {
                    experiment.phase = TwinTrackPhase::LightTag;
                }
            }
            PAYLOAD_LIGHT_TAG if arrival.receiver_channel == TAG_RECEIVER_CHANNEL => {
                experiment.tag_hits = experiment.tag_hits.saturating_add(1);
                experiment.last_speaker_id = TAGGER_ID;
                experiment.last_receive_clock = traveler_clock.seconds;
                experiment.last_receive_lab_time = lab_clock.seconds;
                experiment.last_message_departure_time = arrival.signal_emission_time;
                sfx.write_from(
                    TWINTRACK_EXPERIENCE,
                    ambition_platformer2d::sfx::SfxMessage::Play {
                        id: ambition_platformer2d::sfx::SfxId::from_static(SFX_TAG_HIT),
                        pos: arrival.position,
                    },
                );
                if experiment.tag_hits >= LIGHT_TAG_ROUNDS {
                    experiment.phase = TwinTrackPhase::Reunion;
                }
            }
            _ => {}
        }
    }

    let _ = traveler_entity;
}

#[allow(clippy::too_many_arguments)]
fn publish_twintrack_hud(
    clocks: Res<RelativityClockView2d>,
    signals: Res<RelativitySignalView2d>,
    optics: Res<RelativisticOpticalView2d>,
    targeting: Res<RelativisticTargetingView2d>,
    // WHOSE eyes this line is written from. Both resources hold one row
    // per observer, and reading them through `Deref` takes the FIRST row in
    // label order — which became the laboratory twin the moment she became an
    // observer, because "laboratory" sorts before "traveler". This teacher line
    // says "YOUR NOW" and "FOX IMAGE AGE"; it is the traveler's.
    traveler_observer: Query<Entity, With<TravelerTwin>>,
    dual: Res<TwinTrackDualObserverView>,
    pulse: Res<TwinTrackLightPulseView>,
    traveler_body: Query<&ae::BodyKinematics, With<TravelerTwin>>,
    characters: Query<(&TwinTrackCharacter, &ae::BodyKinematics)>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut readouts: ResMut<ambition_platformer2d::presentation::HudReadouts>,
) {
    let (Some(traveler), Some(lab), Ok(experiment)) = (
        clocks.clocks.get("traveler"),
        clocks.clocks.get("laboratory"),
        experiment.single(),
    ) else {
        for slot in [
            OBJECTIVE_HUD_SLOT,
            CLOCKS_HUD_SLOT,
            DIALOGUE_HUD_SLOT,
            TEACHER_HUD_SLOT,
            RESULT_HUD_SLOT,
        ] {
            readouts.clear_slot(slot);
        }
        return;
    };

    let doppler_preview = traveler_body
        .single()
        .ok()
        .and_then(|traveler_body| {
            characters
                .iter()
                .find(|(character, _)| character.role == TwinTrackRole::DopplerDj)
                .and_then(|(_, dj_body)| doppler_frequency_for_target(traveler_body, dj_body))
        })
        .unwrap_or(EMITTED_FREQUENCY);

    let objective = match experiment.phase {
        TwinTrackPhase::Introduction => match experiment.intro_step {
            TwinTrackIntroStep::Synchronize => {
                "1/4 START — PRESS INTERACT BESIDE THE LAB TWIN".to_owned()
            }
            TwinTrackIntroStep::Drift => {
                "1/4 START — DRIFT AWAY; BOTH CLOCKS STILL FEEL NORMAL".to_owned()
            }
            TwinTrackIntroStep::Accelerate => {
                "1/4 START — ACCELERATE PAST 50% OF LIGHT; WATCH THE CLOCK FACES".to_owned()
            }
        },
        TwinTrackPhase::ClockCensus => {
            let remaining = 3 - experiment.clock_report_mask.count_ones();
            format!("2/4 CLOCK RACE — ASK THE 35%, 55%, AND 75% c ORBITERS ({remaining} LEFT)")
        }
        TwinTrackPhase::DopplerDance => format!(
            "3/4 DOPPLER MUSIC — TUNE G2 INTO G3: {}  {:.1} Hz",
            musical_note_name(doppler_preview),
            doppler_preview,
        ),
        TwinTrackPhase::LightTag => format!(
            "4/4 LIGHT TAG — ROUND {}/{}: LEAD PHOTON FOX'S VISIBLE IMAGE",
            (experiment.tag_hits + 1).min(LIGHT_TAG_ROUNDS),
            LIGHT_TAG_ROUNDS,
        ),
        TwinTrackPhase::Reunion => "REUNION — RETURN TO THE LAB TWIN AND PRESS INTERACT".to_owned(),
        TwinTrackPhase::Complete => {
            "COMPLETE — WATCH THE 3D MINIMAP; LEFT/RIGHT SCRUBS THE HELICAL WORLDLINES".to_owned()
        }
    };
    let view = match experiment.view_mode {
        TwinTrackViewMode::Laboratory => "LAB MAP",
        TwinTrackViewMode::Optical => "WHAT REACHES YOU NOW",
        TwinTrackViewMode::Spacetime => "3D SPACE + TIME",
        TwinTrackViewMode::SplitObservers => "TWO OBSERVERS, TWO ORDERINGS",
    };
    let view_instruction = match experiment.view_mode {
        TwinTrackViewMode::Laboratory => {
            "3D MINIMAP ON BY DEFAULT • M/SPECIAL TO HIDE   VIEW CONSOLE: SPLIT OBSERVERS"
        }
        TwinTrackViewMode::SplitObservers => {
            "LEFT PANE = LAB TWIN AT REST • RIGHT PANE = YOU • FLY ALONG THE BEACON AXIS"
        }
        TwinTrackViewMode::Optical if experiment.phase == TwinTrackPhase::LightTag => {
            "ALIGN CYAN AIM; INTERACT FIRES   M/SPECIAL: MINIMAP"
        }
        TwinTrackViewMode::Optical => "INTERACT: RETURN TO LAB   M/SPECIAL: MINIMAP",
        TwinTrackViewMode::Spacetime => "LEGACY FULL-SCREEN STATE → LAB MAP",
    };
    readouts.set(
        OBJECTIVE_HUD_SLOT,
        ambition_platformer2d::presentation::HudReadout::bare(format!(
            "{objective}\n{view} — {view_instruction}"
        )),
    );

    readouts.set(
        CLOCKS_HUD_SLOT,
        ambition_platformer2d::presentation::HudReadout::bare(format!(
            "YOUR CLOCK {:>6.2} s    LAB {:>6.2} s    SPEED {:>5.1}% OF LIGHT",
            traveler.proper_time_seconds,
            lab.proper_time_seconds,
            100.0 * traveler.relative_velocity.length() / traveler.invariant_speed.max(1.0),
        )),
    );

    let active_tag_signal = signals
        .active_signals
        .iter()
        .any(|signal| payload_parts(signal.payload).0 == PAYLOAD_LIGHT_TAG);
    let dialogue = match experiment.phase {
        TwinTrackPhase::Introduction => match experiment.intro_step {
            TwinTrackIntroStep::Synchronize => {
                "LAB TWIN: “WE START TOGETHER. YOUR CLOCK WILL ALWAYS FEEL NORMAL TO YOU.”"
                    .to_owned()
            }
            TwinTrackIntroStep::Drift => {
                "LAB TWIN: “COAST AWAY. SPEED CHANGES HOW MUCH TIME OUR PATHS ACCUMULATE.”"
                    .to_owned()
            }
            TwinTrackIntroStep::Accelerate => {
                "LAB TWIN: “NOW PUSH HARD. WATCH THE BIG CLOCK FACES—FAST PATHS ACCUMULATE LESS TIME.”".to_owned()
            }
        },
        TwinTrackPhase::ClockCensus
            if matches!(experiment.last_speaker_id, COURIER_ID | DRIFTER_ID | SPINNER_ID) =>
        {
            let travel =
                (experiment.last_receive_lab_time - experiment.last_message_departure_time)
                    .max(0.0);
            format!(
                "{}: “WHEN THIS MESSAGE LEFT, MY CLOCK READ {:.2} s.”\nARRIVED AT YOUR {:.2} s · LAB {:.2} s · LIGHT TRAVEL {:.2} s",
                character_name(experiment.last_speaker_id),
                experiment.last_report_clock,
                experiment.last_receive_clock,
                experiment.last_receive_lab_time,
                travel,
            )
        }
        TwinTrackPhase::ClockCensus => {
            "THE THREE ORBITERS MOVE AT 35%, 55%, AND 75% OF LIGHT. ASK EACH WHAT THEIR OWN CLOCK READS."
                .to_owned()
        }
        TwinTrackPhase::DopplerDance if experiment.doppler_frequency <= 0.0 => format!(
            "DJ BLUE SHIFT: “YOUR TRANSMITTER PLAYS G2 ({EMITTED_FREQUENCY:.0} Hz). APPROACH UNTIL I HEAR G3 ({DOPPLER_TARGET_FREQUENCY:.0} Hz).”"
        ),
        TwinTrackPhase::DopplerDance if experiment.doppler_frequency < DOPPLER_PASSBAND_MIN => {
            format!(
                "DJ BLUE SHIFT: “I HEARD {} AT {:.1} Hz—STILL FLAT. APPROACH FASTER.”",
                musical_note_name(experiment.doppler_frequency),
                experiment.doppler_frequency,
            )
        }
        TwinTrackPhase::DopplerDance => format!(
            "DJ BLUE SHIFT: “I HEARD {} AT {:.1} Hz—TOO SHARP. EASE OFF.”",
            musical_note_name(experiment.doppler_frequency),
            experiment.doppler_frequency,
        ),
        TwinTrackPhase::LightTag if active_tag_signal => {
            "PHOTON FOX: “YOUR LIGHT IS ON THE WAY. MY VISIBLE IMAGE CANNOT DODGE IT.”"
                .to_owned()
        }
        TwinTrackPhase::LightTag if experiment.tag_attempts > experiment.tag_hits => format!(
            "PHOTON FOX: “MISSED. ROUND {}/{}—AIM FARTHER AHEAD OF THE OLD LIGHT.”",
            (experiment.tag_hits + 1).min(LIGHT_TAG_ROUNDS),
            LIGHT_TAG_ROUNDS,
        ),
        TwinTrackPhase::LightTag if experiment.tag_hits > 0 => format!(
            "PHOTON FOX: “HIT {}! THE NEXT ROUND HIDES MORE HELP.”",
            experiment.tag_hits,
        ),
        TwinTrackPhase::LightTag => format!(
            "DJ BLUE SHIFT: “PERFECT G3. NOW TAG THE FOX THREE TIMES; EACH ROUND HIDES A GUIDE.”"
        ),
        TwinTrackPhase::Reunion => {
            "PHOTON FOX: “THREE HITS! NOW MEET THE LAB TWIN AND COMPARE CLOCKS.”"
                .to_owned()
        }
        TwinTrackPhase::Complete => format!(
            "LAB TWIN: “WE MET AGAIN. MY CLOCK READ {:.2} s; YOURS READ {:.2} s.”",
            experiment.result_lab_time, experiment.result_traveler_time,
        ),
    };
    readouts.set(
        DIALOGUE_HUD_SLOT,
        ambition_platformer2d::presentation::HudReadout::bare(dialogue),
    );

    if experiment.view_mode == TwinTrackViewMode::SplitObservers {
        readouts.set(
            TEACHER_HUD_SLOT,
            ambition_platformer2d::presentation::HudReadout::bare(format!(
                "{}\n{}",
                dual_observer_teacher_line(&dual),
                light_pulse_teacher_line(&pulse),
            )),
        );
    } else if experiment.view_mode == TwinTrackViewMode::Spacetime {
        let observer = traveler_observer.single().ok();
        let target = observer
            .and_then(|observer| targeting.for_observer(observer))
            .and_then(|aim| {
                aim.targets
                    .iter()
                    .find(|target| target.label == "Photon Fox")
            });
        let image_age = observer
            .and_then(|observer| optics.for_observer(observer))
            .and_then(|view| {
                view.sources
                    .iter()
                    .find(|source| source.label == "Photon Fox")
            })
            .map_or(0.0, |source| source.light_age);
        let lead = target
            .and_then(|target| target.apparent_to_intercept_angle)
            .map_or(0.0, f32::to_degrees);
        readouts.set(
            TEACHER_HUD_SLOT,
            ambition_platformer2d::presentation::HudReadout::bare(format!(
                "3D GUIDE  GOLD BEADS = 1 s ON EACH OWN CLOCK  •  BLUE = PAST LIGHT CONE  •  CYAN PLANE = YOUR NOW\nADVANCED  β {:.3}  γ {:.3}  dτ/dt {:.3}  LIGHT {}  FOX IMAGE AGE {:.2} s  LEAD {:.1}°  REPLAY {:>3}%",
                traveler.beta_squared.max(0.0).sqrt(),
                traveler.lorentz_factor,
                traveler.proper_time_rate,
                signals.active_signals.len(),
                image_age,
                lead,
                (experiment.replay_cursor * 100.0).round() as u32,
            )),
        );
    } else {
        readouts.clear_slot(TEACHER_HUD_SLOT);
    }

    if experiment.phase == TwinTrackPhase::Complete {
        readouts.set(
            RESULT_HUD_SLOT,
            ambition_platformer2d::presentation::HudReadout::bare(format!(
                "REUNION — LAB {:.2} s · YOU {:.2} s · DIFFERENCE {:.2} s",
                experiment.result_lab_time,
                experiment.result_traveler_time,
                (experiment.result_lab_time - experiment.result_traveler_time).abs(),
            )),
        );
    } else {
        readouts.clear_slot(RESULT_HUD_SLOT);
    }
}

/// One line naming, for each observer, which flash it saw first and which flash
/// it says HAPPENED first. Two orderings that differ is the whole exhibit, so
/// the line says so out loud rather than leaving a viewer to compare numbers.
fn dual_observer_teacher_line(view: &TwinTrackDualObserverView) -> String {
    let Some((lab, traveler)) = view.both() else {
        return "BOTH BEACONS FLASH TOGETHER IN THE LAB — WAIT FOR THEIR LIGHT TO REACH BOTH \
                OBSERVERS"
            .to_owned();
    };
    let verdict = if view.frame_orders_disagree() {
        "THE TWO OBSERVERS DISAGREE ABOUT WHICH FLASH HAPPENED FIRST"
    } else {
        "FLY ALONG THE BEACON AXIS UNTIL THE TWO PANES DISAGREE"
    };
    format!(
        "FLASH #{} — BOTH BEACONS FLASH AT ONE LAB TIME\n\
         LAB TWIN (REST): SAW {} · SAYS {}    YOU ({:.0}% c): SAW {} · SAYS {} BY {:.2} s\n{verdict}",
        lab.compared_flash_index,
        lab.seen_order.caption(),
        lab.frame_order.caption(),
        100.0 * traveler.beta,
        traveler.seen_order.caption(),
        traveler.frame_order.caption(),
        traveler.frame_split_seconds().abs(),
    )
}

/// One line comparing what the two observers measure about the SAME light
/// pulse: the same speed, a different direction, a different colour.
///
/// The speeds are printed rather than asserted because printing them is the
/// exhibit — a viewer who expects `c - v` in the right-hand pane has to read
/// `1.000 c` there instead.
fn light_pulse_teacher_line(view: &TwinTrackLightPulseView) -> String {
    let Some((lab, traveler)) = view.both() else {
        return format!(
            "LIGHT PULSE — THE LAB FIRES A THREE-RAY FLARE EVERY {PULSE_PERIOD_SECONDS:.1} s"
        );
    };
    let lab_cross = lab.ray(PulseRay::Crosswise);
    let traveler_cross = traveler.ray(PulseRay::Crosswise);
    format!(
        "PULSE #{} — SPEED WE BOTH MEASURE: LAB {:.3} c · YOU {:.3} c   \
         CROSSWISE RAY TRAVELS AT {:.0}° / {:.0}°   \
         CHASED x{:.2} · HEAD-ON x{:.2}",
        traveler.pulse_index,
        lab.ray(PulseRay::TowardOmega).measured_speed_fraction,
        traveler.ray(PulseRay::TowardOmega).measured_speed_fraction,
        lab_cross.apparent_angle_degrees,
        traveler_cross.apparent_angle_degrees,
        traveler.ray(PulseRay::TowardOmega).doppler_factor,
        traveler.ray(PulseRay::TowardAlpha).doppler_factor,
    )
}
