//! TwinTrack — a complete, compact special-relativity game.
//!
//! Run from the laboratory to the turnaround beacon and return. The stationary
//! and traveling clocks start and reunite at the same events; the traveler
//! accumulates less proper time because the route is traversed near the
//! laboratory frame's invariant speed.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::lifecycle::{SessionRoot, SessionScopedEntity};
use ambition_platformer2d::platformer::schedule::{SimScheduleExt, WorldPrepSet};
use ambition_platformer2d::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition_platformer2d::relativity2d::{
    ActiveSpacetime2d, ProperTimeElapsed, Relativity2dPlugin, Relativity2dSet,
    RelativityClockLabel, RelativityClockView2d, RelativisticClock2d,
};
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
pub const LAB_X: f32 = 112.0;
pub const TURNAROUND_X: f32 = 1_470.0;
const FLOOR_TOP: f32 = 320.0;
const DEPARTURE_X: f32 = LAB_X + 72.0;
const REUNION_X: f32 = LAB_X + 52.0;
const COMPLETE_HOLD_SECONDS: f32 = 5.0;

pub const STATUS_HUD_SLOT: &str = "twintrack_status";
pub const CLOCKS_HUD_SLOT: &str = "twintrack_clocks";
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
                jump_speed: 420.0,
                max_run_speed: 540.0,
                run_accel: 1800.0,
                air_accel: 900.0,
                max_fall_speed: 540.0,
                coyote_time: 0.08,
                jump_buffer: 0.08,
            )),
            max_health: Some(1),
            playable_kit: Authored,
            tags: ["player", "relativity"],
            barks: (
                hall: ["Same reunion. Different elapsed time.", "The clock comes with me."],
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
    Outbound,
    Inbound,
    Complete,
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct TwinTrackExperiment {
    pub phase: TwinTrackPhase,
    pub peak_beta: f32,
    pub result_lab_time: f64,
    pub result_traveler_time: f64,
    pub complete_elapsed: f32,
}

impl Default for TwinTrackExperiment {
    fn default() -> Self {
        Self {
            phase: TwinTrackPhase::Ready,
            peak_beta: 0.0,
            result_lab_time: 0.0,
            result_traveler_time: 0.0,
            complete_elapsed: 0.0,
        }
    }
}

impl ambition_platformer2d::engine_core::snapshot::SnapshotState for TwinTrackExperiment {
    fn encode(&self, out: &mut Vec<u8>) {
        use ambition_platformer2d::engine_core::snapshot::{put_f32, put_u64, put_u8};
        put_u8(
            out,
            match self.phase {
                TwinTrackPhase::Ready => 0,
                TwinTrackPhase::Outbound => 1,
                TwinTrackPhase::Inbound => 2,
                TwinTrackPhase::Complete => 3,
            },
        );
        put_f32(out, self.peak_beta);
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
                1 => TwinTrackPhase::Outbound,
                2 => TwinTrackPhase::Inbound,
                3 => TwinTrackPhase::Complete,
                _ => return None,
            },
            peak_beta: reader.f32()?,
            result_lab_time: f64::from_bits(reader.u64()?),
            result_traveler_time: f64::from_bits(reader.u64()?),
            complete_elapsed: reader.f32()?,
        })
    }
}

pub fn twintrack_room() -> RoomSpec {
    let size = ae::Vec2::new(1_680.0, 420.0);
    let world = ae::World::new(
        "TwinTrack Minkowski Laboratory",
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
                "The lab keeps coordinate time.",
                "My animation and clock keep proper time.",
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
        .rollback_component_clone::<TravelerTwin>(
            "ambition_demo_twintrack",
            "twintrack.traveler",
        )
        .rollback_component_clone::<LaboratoryTwin>(
            "ambition_demo_twintrack",
            "twintrack.laboratory",
        );

        PlatformerExperienceAuthoring::new(
            TWINTRACK_EXPERIENCE,
            TWINTRACK_GAMEPLAY_ROUTE,
            "TwinTrack",
            "Choose a worldline, reunite, and compare proper time",
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
                        .with_font_size(20.0),
                )
                .slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(CLOCKS_HUD_SLOT)
                        .with_region(ambition_platformer2d::presentation::SurroundRegion::Bottom)
                        .with_font_size(18.0),
                )
                .slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(RESULT_HUD_SLOT)
                        .with_order(99)
                        .with_font_size(28.0)
                        .centered(),
                ),
        )
        .install(app, twintrack_prepared_session_world);

        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            install_twintrack_session
                .run_if(ambition_platformer2d::runtime::in_mode(TWINTRACK_EXPERIENCE))
                .before(Relativity2dSet::ResolveClocks)
                .in_set(WorldPrepSet::BeforeIntegrate),
        )
        .add_systems(
            sim,
            update_twintrack_experiment
                .run_if(ambition_platformer2d::runtime::in_mode(TWINTRACK_EXPERIENCE))
                .after(Relativity2dSet::ResolveClocks)
                .in_set(WorldPrepSet::BeforeIntegrate),
        )
        .add_systems(
            Update,
            publish_twintrack_hud
                .run_if(ambition_platformer2d::runtime::in_mode(TWINTRACK_EXPERIENCE)),
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
    player: Query<
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
    let Ok(player_entity) = player.single() else {
        return;
    };

    commands.entity(root_entity).insert(
        ActiveSpacetime2d::minkowski(INVARIANT_SPEED as f64)
            .expect("TwinTrack invariant speed is finite and positive"),
    );
    commands.entity(player_entity).insert((
        TravelerTwin,
        RelativisticClock2d,
        RelativityClockLabel("traveler".to_owned()),
    ));
    commands.spawn((
        LaboratoryTwin,
        RelativisticClock2d,
        RelativityClockLabel("laboratory".to_owned()),
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
        // ⛔ **no `Rollback` marker here.** Spawning one inline panics on any host
        // without a live ggrs session: `bevy_ggrs`'s `on_rollback_added` hook
        // fires immediately and reads a session resource that a fixed-tick host
        // has never inserted, which took out all three acceptance tests. It is
        // also the only such spawn in the repo — nothing else in `crates/` or
        // `game/` inserts the marker by hand, because entity tracking is the
        // session's business and what puts this clock in a snapshot is the
        // component registration below, not the marker.
    ));
}

fn update_twintrack_experiment(
    time: Res<WorldTime>,
    mut traveler: Query<
        (&mut ae::BodyKinematics, &mut ProperTimeElapsed),
        (With<TravelerTwin>, Without<LaboratoryTwin>),
    >,
    mut lab: Query<
        (&mut ProperTimeElapsed, &mut TwinTrackExperiment),
        (With<LaboratoryTwin>, Without<TravelerTwin>),
    >,
    relativity: Query<
        &ambition_platformer2d::relativity2d::RelativityState2d,
        With<TravelerTwin>,
    >,
) {
    let (Ok((mut body, mut traveler_clock)), Ok((mut lab_clock, mut experiment))) =
        (traveler.single_mut(), lab.single_mut())
    else {
        return;
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
                traveler_clock.reset();
                lab_clock.reset();
                experiment.peak_beta = beta;
                experiment.phase = TwinTrackPhase::Outbound;
            }
        }
        TwinTrackPhase::Outbound => {
            if body.pos.x >= TURNAROUND_X {
                experiment.phase = TwinTrackPhase::Inbound;
            }
        }
        TwinTrackPhase::Inbound => {
            if body.pos.x <= REUNION_X && body.vel.x < 0.0 {
                experiment.result_lab_time = lab_clock.seconds;
                experiment.result_traveler_time = traveler_clock.seconds;
                experiment.complete_elapsed = 0.0;
                experiment.phase = TwinTrackPhase::Complete;
                body.vel = Vec2::ZERO;
            }
        }
        TwinTrackPhase::Complete => {
            experiment.complete_elapsed += time.sim_dt();
            if experiment.complete_elapsed >= COMPLETE_HOLD_SECONDS {
                body.pos = Vec2::new(LAB_X, FLOOR_TOP - 64.0);
                body.vel = Vec2::ZERO;
                traveler_clock.reset();
                lab_clock.reset();
                *experiment = TwinTrackExperiment::default();
            }
        }
    }
}

fn publish_twintrack_hud(
    view: Res<RelativityClockView2d>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut readouts: ResMut<ambition_platformer2d::presentation::HudReadouts>,
) {
    let (Some(traveler), Some(lab), Ok(experiment)) = (
        view.clocks.get("traveler"),
        view.clocks.get("laboratory"),
        experiment.single(),
    ) else {
        readouts.clear_slot(STATUS_HUD_SLOT);
        readouts.clear_slot(CLOCKS_HUD_SLOT);
        readouts.clear_slot(RESULT_HUD_SLOT);
        return;
    };

    let instruction = match experiment.phase {
        TwinTrackPhase::Ready => "HOLD RIGHT — DEPART THE LAB",
        TwinTrackPhase::Outbound => "REACH THE FAR BEACON",
        TwinTrackPhase::Inbound => "TURN AROUND AND REUNITE AT THE LAB",
        TwinTrackPhase::Complete => "REUNION EVENT — WORLDLINES COMPARED",
    };
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
    readouts.set(
        CLOCKS_HUD_SLOT,
        ambition_platformer2d::presentation::HudReadout::bare(format!(
            "LAB CLOCK {:>7.3}s    TRAVELER CLOCK {:>7.3}s    DIFFERENCE {:>7.3}s",
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
                    "TWINTRACK COMPLETE\n",
                    "LAB {:.3}s    TRAVELER {:.3}s\n",
                    "THE TRAVELER AGED {:.3}s LESS    PEAK beta {:.3}",
                ),
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
        let c = ambition_platformer2d::relativity::InvariantSpeed::new(INVARIANT_SPEED as f64)
            .unwrap();
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
}
