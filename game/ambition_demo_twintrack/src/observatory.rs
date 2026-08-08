//! Visible TwinTrack presentation: the relativity plaza, full-screen optical
//! view, and an isometric 2+1D spacetime sculpture.
//!
//! The laboratory camera shows authoritative coordinate positions. The optical
//! camera consumes only observer-derived views and worldline telemetry. Switching
//! displays therefore never changes movement, collisions, clocks, or signals.

use ambition_platformer2d::platformer::camera_layers::MainCamera;
use ambition_platformer2d::relativity::{
    coordinate_photon_direction_from_observer, observe_photon_direction, InvariantSpeed,
};
use ambition_platformer2d::relativity2d::{
    ProperTimeElapsed, RelativisticOpticalView2d, RelativisticTargetingView2d,
    RelativitySignalView2d, WorldlineHistoryView2d, WorldlineTrackId,
};
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, OrthographicProjection, Projection, ScalingMode};
use bevy::prelude::*;

use crate::{
    decoded_clock_seconds, doppler_frequency_for_target, musical_note_name, payload_parts,
    LaboratoryTwin, TravelerTwin, TwinTrackCharacter, TwinTrackExperiment, TwinTrackPhase,
    TwinTrackViewMode, COURIER_ID, DJ_ID, DJ_POS, DOPPLER_PASSBAND_MAX, DOPPLER_PASSBAND_MIN,
    DOPPLER_TARGET_FREQUENCY, DRIFTER_ID, INVARIANT_SPEED, LAB_POS, LIGHT_TAG_ROUNDS,
    PAYLOAD_ASK_CLOCK, PAYLOAD_CLOCK_REPORT, PAYLOAD_DOPPLER_NOTE, PAYLOAD_LIGHT_TAG, ROOM_HEIGHT,
    ROOM_WIDTH, SPINNER_ID, TAGGER_ID, TRAVELER_TAG, TWINTRACK_EXPERIENCE, VIEW_CONSOLE_POS,
};

const OPTICAL_LAYER: usize = 28;
const VIEW_WIDTH: f32 = 1_200.0;
const VIEW_HEIGHT: f32 = 750.0;
const SKY_CENTER: Vec2 = Vec2::new(0.0, 10.0);
const SKY_RADIUS: f32 = 300.0;
const STAR_COUNT: usize = 256;
const TRACE_DOT_COUNT: usize = 360;
const SIGNAL_DOT_COUNT: usize = 32;
const CONE_LINE_COUNT: usize = 4;
const TRACE_WINDOW_SECONDS: f64 = 7.0;
const CLOCK_HAND_PERIOD_SECONDS: f64 = 4.0;

#[derive(Component)]
struct TwinTrackVisible;

#[derive(Component)]
struct LabCharacterVisual(u8);

#[derive(Component)]
struct LabCharacterLabel(u8);

#[derive(Component)]
struct LabSignalVisual(usize);

#[derive(Component)]
struct LabSignalLabel(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClockVisualTarget {
    Traveler,
    Laboratory,
    Character(u8),
}

#[derive(Component)]
struct ClockReadout(ClockVisualTarget);

#[derive(Component)]
struct ClockHand(ClockVisualTarget);

#[derive(Component)]
struct DjFrequencyBar;

#[derive(Component)]
struct DjFrequencyLabel;

#[derive(Component)]
struct DjBeatRing(usize);

#[derive(Component)]
struct LabAimLine;

#[derive(Component)]
struct LabDelayedImageMarker;

#[derive(Component)]
struct LabInterceptMarker;

#[derive(Component)]
struct LabTagGuideLabel;

#[derive(Component)]
struct ViewConsoleVisual;

#[derive(Component)]
struct LaboratoryVisual;

#[derive(Component)]
struct LabDialogueBubble;

#[derive(Component)]
pub struct ObservatoryCamera;

#[derive(Component)]
struct ObservatoryTitle;

#[derive(Component)]
struct ObservatoryStar {
    rest_direction: Vec2,
    radial_fraction: f32,
    size: f32,
    intensity: f32,
}

#[derive(Component)]
struct OpticalProxy(String);

#[derive(Component)]
struct OpticalProxyLabel(String);

#[derive(Component)]
struct OpticalAimLine;

#[derive(Component)]
struct OpticalInterceptMarker;

#[derive(Component)]
struct OpticalTagGuideLabel;

#[derive(Component)]
struct OpticalObserverMarker;

#[derive(Component)]
struct TraceDot(usize);

#[derive(Component)]
struct SignalDot(usize);

#[derive(Component)]
struct ConeLine(usize);

#[derive(Component)]
struct SpacetimeAxis(usize);

#[derive(Component)]
struct SpacetimeGuide;

pub(crate) fn install(app: &mut App) {
    app.add_systems(
        Update,
        (
            spawn_twintrack_visuals,
            sync_view_cameras,
            update_lab_character_visuals,
            update_visible_clocks,
            update_lab_dialogue_bubble,
            update_lab_signal_visuals,
            update_doppler_music_visuals,
            update_light_tag_guides,
            update_observatory_title,
            update_optical_observer_marker,
            update_observatory_stars,
            update_optical_proxies,
            update_spacetime_sculpture,
            update_spacetime_signals,
            update_light_cone,
            update_spacetime_guides,
            cleanup_visuals_when_inactive,
        ),
    );
}

fn twintrack_is_active(
    roots: &Query<&ambition_platformer2d::runtime::demo_fixture::ActiveRoomMetadata>,
) -> bool {
    roots
        .iter()
        .any(|metadata| metadata.0.mode.as_deref() == Some(TWINTRACK_EXPERIENCE))
}

fn spawn_clock_pair(commands: &mut Commands, target: ClockVisualTarget, name: &str) {
    commands.spawn((
        TwinTrackVisible,
        ClockReadout(target),
        Text2d::new("CLOCK 0.0 s"),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.96, 0.98, 1.0)),
        Transform::from_xyz(0.0, 0.0, 15.0),
        Name::new(format!("TwinTrack {name} clock readout")),
    ));
    commands.spawn((
        TwinTrackVisible,
        ClockHand(target),
        Sprite::from_color(Color::srgb(0.96, 0.98, 1.0), Vec2::ONE),
        Transform::from_xyz(0.0, 0.0, 14.0),
        Name::new(format!("TwinTrack {name} clock hand")),
    ));
}

fn spawn_twintrack_visuals(
    mut commands: Commands,
    roots: Query<&ambition_platformer2d::runtime::demo_fixture::ActiveRoomMetadata>,
    existing: Query<(), With<TwinTrackVisible>>,
) {
    if !twintrack_is_active(&roots) || !existing.is_empty() {
        return;
    }

    for (id, label, color) in [
        (1, "COURIER", Color::srgb(0.35, 0.90, 1.0)),
        (2, "DRIFTER", Color::srgb(0.85, 0.55, 1.0)),
        (3, "SPINNER", Color::srgb(1.0, 0.65, 0.25)),
        (DJ_ID, "DJ BLUE SHIFT", Color::srgb(0.25, 0.55, 1.0)),
        (TAGGER_ID, "PHOTON FOX", Color::srgb(1.0, 0.78, 0.20)),
    ] {
        commands.spawn((
            TwinTrackVisible,
            LabCharacterVisual(id),
            Sprite::from_color(color, Vec2::splat(if id == DJ_ID { 42.0 } else { 34.0 })),
            Transform::from_xyz(0.0, 0.0, 8.0),
            Name::new(format!("TwinTrack lab character {label}")),
        ));
        commands.spawn((
            TwinTrackVisible,
            LabCharacterLabel(id),
            Text2d::new(label),
            TextFont {
                font_size: 15.0,
                ..default()
            },
            TextColor(color),
            Transform::from_xyz(0.0, 0.0, 10.0),
            Name::new(format!("TwinTrack lab label {label}")),
        ));
        spawn_clock_pair(&mut commands, ClockVisualTarget::Character(id), label);
    }
    commands.spawn((
        TwinTrackVisible,
        LaboratoryVisual,
        Sprite::from_color(Color::srgb(0.92, 0.96, 1.0), Vec2::splat(46.0)),
        Transform::from_translation(LAB_POS.extend(6.0)),
        Name::new("TwinTrack laboratory twin marker"),
    ));
    spawn_clock_pair(&mut commands, ClockVisualTarget::Laboratory, "laboratory");
    spawn_clock_pair(&mut commands, ClockVisualTarget::Traveler, "traveler");
    commands.spawn((
        TwinTrackVisible,
        Text2d::new(
            "LAB TWIN
REUNION POINT",
        ),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.96, 1.0)),
        Transform::from_translation((LAB_POS + Vec2::new(0.0, -52.0)).extend(9.0)),
        Name::new("TwinTrack laboratory twin label"),
    ));
    commands.spawn((
        TwinTrackVisible,
        ViewConsoleVisual,
        Sprite::from_color(Color::srgb(0.10, 0.85, 0.65), Vec2::splat(58.0)),
        Transform::from_translation(VIEW_CONSOLE_POS.extend(5.0)),
        Name::new("TwinTrack view console"),
    ));
    commands.spawn((
        TwinTrackVisible,
        LabDialogueBubble,
        Text2d::new(""),
        TextFont {
            font_size: 17.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 0.82)),
        Visibility::Hidden,
        Transform::from_xyz(0.0, 0.0, 16.0),
        Name::new("TwinTrack character dialogue bubble"),
    ));
    commands.spawn((
        TwinTrackVisible,
        Text2d::new("VIEW CONSOLE\nINTERACT"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.55, 1.0, 0.85)),
        Transform::from_translation((VIEW_CONSOLE_POS + Vec2::new(0.0, -54.0)).extend(8.0)),
        Name::new("TwinTrack view console label"),
    ));
    commands.spawn((
        TwinTrackVisible,
        LabAimLine,
        Sprite::from_color(Color::srgb(0.20, 0.95, 1.0), Vec2::ONE),
        Visibility::Hidden,
        Transform::from_xyz(0.0, 0.0, 13.0),
        Name::new("TwinTrack laboratory aim ray"),
    ));
    commands.spawn((
        TwinTrackVisible,
        LabDelayedImageMarker,
        Sprite::from_color(Color::srgba(1.0, 0.18, 0.18, 0.62), Vec2::splat(46.0)),
        Visibility::Hidden,
        Transform::from_xyz(0.0, 0.0, 13.0),
        Name::new("TwinTrack light-delayed Photon Fox marker"),
    ));
    commands.spawn((
        TwinTrackVisible,
        LabInterceptMarker,
        Sprite::from_color(Color::srgba(0.20, 1.0, 0.35, 0.78), Vec2::splat(28.0)),
        Visibility::Hidden,
        Transform::from_xyz(0.0, 0.0, 14.0),
        Name::new("TwinTrack Photon Fox intercept marker"),
    ));
    commands.spawn((
        TwinTrackVisible,
        LabTagGuideLabel,
        Text2d::new(
            "RED = VISIBLE IMAGE   YELLOW FOX = WHERE FOX IS NOW   \
             GREEN = INTERCEPT   CYAN = YOUR AIM",
        ),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.78, 1.0, 0.92)),
        Visibility::Hidden,
        Transform::from_xyz(ROOM_WIDTH * 0.5, 66.0, 17.0),
        Name::new("TwinTrack laboratory light-tag guide"),
    ));
    for index in 0..SIGNAL_DOT_COUNT {
        commands.spawn((
            TwinTrackVisible,
            LabSignalVisual(index),
            Sprite::from_color(Color::srgb(1.0, 0.95, 0.35), Vec2::splat(12.0)),
            Visibility::Hidden,
            Transform::from_xyz(0.0, 0.0, 12.0),
            Name::new(format!("TwinTrack lab light signal {index}")),
        ));
        commands.spawn((
            TwinTrackVisible,
            LabSignalLabel(index),
            Text2d::new(""),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.95, 0.55)),
            Visibility::Hidden,
            Transform::from_xyz(0.0, 0.0, 13.0),
            Name::new(format!("TwinTrack lab light message {index}")),
        ));
    }

    commands.spawn((
        TwinTrackVisible,
        DjFrequencyBar,
        Sprite::from_color(Color::srgb(0.25, 0.55, 1.0), Vec2::new(10.0, 12.0)),
        Visibility::Hidden,
        Transform::from_xyz(DJ_POS.x, DJ_POS.y - 66.0, 13.0),
        Name::new("TwinTrack DJ continuous frequency meter"),
    ));
    commands.spawn((
        TwinTrackVisible,
        DjFrequencyLabel,
        Text2d::new(""),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::srgb(0.65, 0.82, 1.0)),
        Visibility::Hidden,
        Transform::from_xyz(DJ_POS.x, DJ_POS.y - 94.0, 14.0),
        Name::new("TwinTrack DJ continuous pitch label"),
    ));
    for index in 0..3 {
        commands.spawn((
            TwinTrackVisible,
            DjBeatRing(index),
            Sprite::from_color(Color::srgba(0.25, 0.65, 1.0, 0.35), Vec2::splat(48.0)),
            Visibility::Hidden,
            Transform::from_xyz(DJ_POS.x, DJ_POS.y, 7.0),
            Name::new(format!("TwinTrack DJ beat ring {index}")),
        ));
    }

    let layer = RenderLayers::layer(OPTICAL_LAYER);
    commands.spawn((
        TwinTrackVisible,
        ObservatoryCamera,
        Camera2d,
        Camera {
            order: 4,
            is_active: false,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.005, 0.008, 0.025)),
            ..default()
        },
        layer.clone(),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: VIEW_WIDTH,
                height: VIEW_HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new("TwinTrack relativity view camera"),
    ));
    commands.spawn((
        TwinTrackVisible,
        ObservatoryTitle,
        Text2d::new("WHAT REACHES YOU NOW"),
        TextFont {
            font_size: 28.0,
            ..default()
        },
        TextColor(Color::srgb(0.72, 0.92, 1.0)),
        Transform::from_xyz(0.0, 340.0, 20.0),
        layer.clone(),
        Name::new("TwinTrack relativity view title"),
    ));
    commands.spawn((
        TwinTrackVisible,
        Text2d::new(
            "OPTICAL: characters appear where their arriving light left them\n\
             SPACE + TIME: a 3D graph—X and Y are space; height is laboratory time",
        ),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::srgb(0.65, 0.78, 0.95)),
        Transform::from_xyz(0.0, 302.0, 20.0),
        layer.clone(),
        Name::new("TwinTrack relativity view explanation"),
    ));

    for index in 0..STAR_COUNT {
        let seed = splitmix64(index as u64 + 0x54_57_49_4e);
        let angle = unit_from_u64(seed) * std::f32::consts::TAU;
        let radial_fraction = 0.16 + 0.82 * unit_from_u64(splitmix64(seed));
        let size = 1.2 + 3.2 * unit_from_u64(splitmix64(seed ^ 0xabc));
        let intensity = 0.35 + 0.9 * unit_from_u64(splitmix64(seed ^ 0xdef));
        commands.spawn((
            TwinTrackVisible,
            ObservatoryStar {
                rest_direction: Vec2::from_angle(angle),
                radial_fraction,
                size,
                intensity,
            },
            Sprite::from_color(Color::WHITE, Vec2::splat(size)),
            Transform::from_xyz(0.0, 0.0, 2.0),
            layer.clone(),
            Name::new(format!("TwinTrack optical star {index}")),
        ));
    }

    for label in [
        "laboratory",
        "Courier",
        "Drifter",
        "Spinner",
        "DJ Blue Shift",
        "Photon Fox",
    ] {
        commands.spawn((
            TwinTrackVisible,
            OpticalProxy(label.to_owned()),
            Sprite::from_color(Color::WHITE, Vec2::splat(24.0)),
            Visibility::Hidden,
            Transform::from_xyz(0.0, 0.0, 8.0),
            layer.clone(),
            Name::new(format!("TwinTrack optical source {label}")),
        ));
        commands.spawn((
            TwinTrackVisible,
            OpticalProxyLabel(label.to_owned()),
            Text2d::new(label.to_uppercase()),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Visibility::Hidden,
            Transform::from_xyz(0.0, 0.0, 10.0),
            layer.clone(),
            Name::new(format!("TwinTrack optical label {label}")),
        ));
    }

    commands.spawn((
        TwinTrackVisible,
        OpticalObserverMarker,
        Sprite::from_color(Color::srgb(0.20, 0.95, 1.0), Vec2::splat(18.0)),
        Visibility::Hidden,
        Transform::from_translation(SKY_CENTER.extend(16.0)),
        layer.clone(),
        Name::new("TwinTrack optical observer marker"),
    ));
    commands.spawn((
        TwinTrackVisible,
        OpticalObserverMarker,
        Text2d::new(
            "YOU
DIRECTIONS AROUND YOU",
        ),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.45, 1.0, 1.0)),
        Visibility::Hidden,
        Transform::from_translation((SKY_CENTER + Vec2::new(0.0, -38.0)).extend(17.0)),
        layer.clone(),
        Name::new("TwinTrack optical observer label"),
    ));

    commands.spawn((
        TwinTrackVisible,
        OpticalAimLine,
        Sprite::from_color(Color::srgb(0.20, 0.95, 1.0), Vec2::ONE),
        Visibility::Hidden,
        Transform::from_xyz(0.0, 0.0, 14.0),
        layer.clone(),
        Name::new("TwinTrack optical aim ray"),
    ));
    commands.spawn((
        TwinTrackVisible,
        OpticalInterceptMarker,
        Sprite::from_color(Color::srgb(0.20, 1.0, 0.35), Vec2::splat(25.0)),
        Visibility::Hidden,
        Transform::from_xyz(0.0, 0.0, 15.0),
        layer.clone(),
        Name::new("TwinTrack optical intercept marker"),
    ));
    commands.spawn((
        TwinTrackVisible,
        OpticalTagGuideLabel,
        Text2d::new("RED FOX = VISIBLE IMAGE • GREEN = SEND LIGHT HERE • CYAN = YOUR AIM"),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::srgb(0.78, 1.0, 0.92)),
        Visibility::Hidden,
        Transform::from_xyz(0.0, -330.0, 19.0),
        layer.clone(),
        Name::new("TwinTrack optical light-tag guide"),
    ));

    for index in 0..TRACE_DOT_COUNT {
        commands.spawn((
            TwinTrackVisible,
            TraceDot(index),
            Sprite::from_color(Color::WHITE, Vec2::splat(5.0)),
            Visibility::Hidden,
            Transform::from_xyz(0.0, 0.0, 4.0),
            layer.clone(),
            Name::new(format!("TwinTrack spacetime trace {index}")),
        ));
    }
    for index in 0..SIGNAL_DOT_COUNT {
        commands.spawn((
            TwinTrackVisible,
            SignalDot(index),
            Sprite::from_color(Color::srgb(1.0, 0.95, 0.25), Vec2::splat(8.0)),
            Visibility::Hidden,
            Transform::from_xyz(0.0, 0.0, 7.0),
            layer.clone(),
            Name::new(format!("TwinTrack spacetime signal {index}")),
        ));
    }
    for index in 0..CONE_LINE_COUNT {
        commands.spawn((
            TwinTrackVisible,
            ConeLine(index),
            Sprite::from_color(Color::srgba(0.20, 0.72, 1.0, 0.72), Vec2::ONE),
            Visibility::Hidden,
            Transform::from_xyz(0.0, 0.0, 3.0),
            layer.clone(),
            Name::new(format!("TwinTrack past light cone edge {index}")),
        ));
    }
    for (index, color) in [
        Color::srgb(0.30, 0.95, 0.95),
        Color::srgb(0.90, 0.45, 1.0),
        Color::srgb(1.0, 0.80, 0.25),
    ]
    .into_iter()
    .enumerate()
    {
        commands.spawn((
            TwinTrackVisible,
            SpacetimeAxis(index),
            Sprite::from_color(color, Vec2::ONE),
            Visibility::Hidden,
            Transform::from_xyz(0.0, 0.0, 2.0),
            layer.clone(),
            Name::new(format!("TwinTrack spacetime axis {index}")),
        ));
    }
    for (text, position, color) in [
        (
            "X POSITION →",
            Vec2::new(470.0, -264.0),
            Color::srgb(0.30, 0.95, 0.95),
        ),
        (
            "Y POSITION ↗",
            Vec2::new(-390.0, -82.0),
            Color::srgb(0.90, 0.45, 1.0),
        ),
        (
            "LAB TIME ↑\nNOW IS HIGHER",
            Vec2::new(-520.0, 120.0),
            Color::srgb(1.0, 0.80, 0.25),
        ),
        (
            "COLORED TRAILS = CHARACTERS   YELLOW LINES = LIGHT SIGNALS
BLUE EDGES = PAST LIGHT CONE   LOWER DOTS ARE EARLIER EVENTS",
            Vec2::new(0.0, 310.0),
            Color::srgb(0.72, 0.85, 1.0),
        ),
    ] {
        commands.spawn((
            TwinTrackVisible,
            SpacetimeGuide,
            Text2d::new(text),
            TextFont {
                font_size: 15.0,
                ..default()
            },
            TextColor(color),
            Visibility::Hidden,
            Transform::from_translation(position.extend(18.0)),
            layer.clone(),
            Name::new(format!("TwinTrack spacetime guide {text}")),
        ));
    }
}

fn sync_view_cameras(
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut observatory: Query<&mut Camera, With<ObservatoryCamera>>,
    mut laboratory: Query<&mut Camera, (With<MainCamera>, Without<ObservatoryCamera>)>,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    let optical_active = experiment.view_mode != TwinTrackViewMode::Laboratory;
    if let Ok(mut camera) = observatory.single_mut() {
        camera.is_active = optical_active;
        camera.viewport = None;
    }
    for mut camera in &mut laboratory {
        camera.is_active = !optical_active;
        camera.viewport = None;
    }
}

fn update_lab_character_visuals(
    time: Res<Time>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    characters: Query<(
        &TwinTrackCharacter,
        &ambition_platformer2d::engine_core::BodyKinematics,
    )>,
    mut bodies: Query<
        (&LabCharacterVisual, &mut Transform, &mut Visibility),
        Without<LabCharacterLabel>,
    >,
    mut labels: Query<
        (&LabCharacterLabel, &mut Transform, &mut Visibility),
        Without<LabCharacterVisual>,
    >,
) {
    let experiment = experiment.single().ok();
    for (visual, mut transform, mut visibility) in &mut bodies {
        if let Some((_, body)) = characters
            .iter()
            .find(|(character, _)| character.id == visual.0)
        {
            let hidden_assistance = visual.0 == TAGGER_ID
                && experiment.is_some_and(|state| {
                    state.phase == TwinTrackPhase::LightTag && state.tag_hits >= 1
                });
            *visibility = if hidden_assistance {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
            transform.translation.x = body.pos.x;
            transform.translation.y = body.pos.y;
            transform.scale = Vec3::ONE;
            transform.rotation = Quat::IDENTITY;
            if visual.0 == DJ_ID
                && experiment.is_some_and(|state| {
                    state.doppler_frequency > 0.0 && state.phase != TwinTrackPhase::DopplerDance
                })
            {
                let pulse = 1.0 + 0.18 * (time.elapsed_secs() * 8.0).sin().abs();
                transform.scale = Vec3::splat(pulse);
                transform.rotation =
                    Quat::from_rotation_z((time.elapsed_secs() * 5.0).sin() * 0.14);
            } else if visual.0 == TAGGER_ID && experiment.is_some_and(|state| state.tag_hits > 0) {
                transform.scale =
                    Vec3::splat(1.0 + 0.22 * (time.elapsed_secs() * 10.0).sin().abs());
            }
        }
    }
    for (visual, mut transform, mut visibility) in &mut labels {
        if let Some((_, body)) = characters
            .iter()
            .find(|(character, _)| character.id == visual.0)
        {
            let hidden_assistance = visual.0 == TAGGER_ID
                && experiment.is_some_and(|state| {
                    state.phase == TwinTrackPhase::LightTag && state.tag_hits >= 1
                });
            *visibility = if hidden_assistance {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
            transform.translation.x = body.pos.x;
            transform.translation.y = body.pos.y - 34.0;
        }
    }
}

fn update_visible_clocks(
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    traveler: Query<
        (
            &ambition_platformer2d::engine_core::BodyKinematics,
            &ProperTimeElapsed,
        ),
        With<TravelerTwin>,
    >,
    laboratory: Query<
        (
            &ambition_platformer2d::engine_core::BodyKinematics,
            &ProperTimeElapsed,
        ),
        With<LaboratoryTwin>,
    >,
    characters: Query<(
        &TwinTrackCharacter,
        &ambition_platformer2d::engine_core::BodyKinematics,
        &ProperTimeElapsed,
    )>,
    mut readouts: Query<
        (&ClockReadout, &mut Text2d, &mut Transform, &mut Visibility),
        Without<ClockHand>,
    >,
    mut hands: Query<
        (&ClockHand, &mut Sprite, &mut Transform, &mut Visibility),
        Without<ClockReadout>,
    >,
) {
    let experiment = experiment.single().ok();
    let state = |target: ClockVisualTarget| -> Option<(Vec2, f64, bool)> {
        match target {
            ClockVisualTarget::Traveler => traveler
                .single()
                .ok()
                .map(|(body, clock)| (body.pos, clock.seconds, true)),
            ClockVisualTarget::Laboratory => laboratory
                .single()
                .ok()
                .map(|(body, clock)| (body.pos, clock.seconds, true)),
            ClockVisualTarget::Character(id) => characters
                .iter()
                .find(|(character, _, _)| character.id == id)
                .map(|(_, body, clock)| {
                    let visible = !(id == TAGGER_ID
                        && experiment.is_some_and(|state| {
                            state.phase == TwinTrackPhase::LightTag && state.tag_hits >= 1
                        }));
                    (body.pos, clock.seconds, visible)
                }),
        }
    };

    for (readout, mut text, mut transform, mut visibility) in &mut readouts {
        let Some((position, seconds, visible)) = state(readout.0) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        text.0 = format!("CLOCK {seconds:>5.1} s");
        transform.translation = (position + Vec2::new(0.0, 48.0)).extend(15.0);
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    for (hand, mut sprite, mut transform, mut visibility) in &mut hands {
        let Some((position, seconds, visible)) = state(hand.0) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let angle = (seconds / CLOCK_HAND_PERIOD_SECONDS * std::f64::consts::TAU) as f32;
        let center = position + Vec2::new(-48.0, 48.0);
        let end = center + Vec2::from_angle(angle) * 13.0;
        set_line(&mut sprite, &mut transform, center, end, 2.5, 14.0);
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn update_lab_dialogue_bubble(
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    characters: Query<(
        &TwinTrackCharacter,
        &ambition_platformer2d::engine_core::BodyKinematics,
    )>,
    mut bubbles: Query<(&mut Text2d, &mut Transform, &mut Visibility), With<LabDialogueBubble>>,
) {
    let (Ok(experiment), Ok((mut text, mut transform, mut visibility))) =
        (experiment.single(), bubbles.single_mut())
    else {
        return;
    };
    let (position, message) = match experiment.phase {
        TwinTrackPhase::Introduction => (
            LAB_POS,
            match experiment.intro_step {
                crate::TwinTrackIntroStep::Synchronize => "LAB TWIN: INTERACT TO BEGIN",
                crate::TwinTrackIntroStep::Drift => "LAB TWIN: DRIFT AWAY",
                crate::TwinTrackIntroStep::Accelerate => "LAB TWIN: PUSH PAST 50% OF LIGHT",
            }
            .to_owned(),
        ),
        TwinTrackPhase::ClockCensus if experiment.last_speaker_id != 0 => characters
            .iter()
            .find(|(character, _)| character.id == experiment.last_speaker_id)
            .map(|(character, body)| {
                (
                    body.pos,
                    format!(
                        "{}: MY CLOCK READ {:.2} s\nWHEN THIS MESSAGE LEFT",
                        character.label.to_uppercase(),
                        experiment.last_report_clock,
                    ),
                )
            })
            .unwrap_or((Vec2::ZERO, String::new())),
        TwinTrackPhase::LightTag if experiment.tag_hits == 0 => characters
            .iter()
            .find(|(character, _)| character.id == DJ_ID)
            .map(|(_, body)| {
                (
                    body.pos,
                    format!(
                        "G3! I HEARD {} AT {:.1} Hz",
                        musical_note_name(experiment.doppler_frequency),
                        experiment.doppler_frequency,
                    ),
                )
            })
            .unwrap_or((Vec2::ZERO, String::new())),
        TwinTrackPhase::LightTag => characters
            .iter()
            .find(|(character, _)| character.id == TAGGER_ID)
            .map(|(_, body)| {
                (
                    body.pos,
                    format!(
                        "PHOTON FOX: HIT {}/{}—LESS HELP NEXT!",
                        experiment.tag_hits, LIGHT_TAG_ROUNDS,
                    ),
                )
            })
            .unwrap_or((Vec2::ZERO, String::new())),
        TwinTrackPhase::Reunion => characters
            .iter()
            .find(|(character, _)| character.id == TAGGER_ID)
            .map(|(_, body)| (body.pos, "PHOTON FOX: THREE HITS!".to_owned()))
            .unwrap_or((Vec2::ZERO, String::new())),
        TwinTrackPhase::Complete => (
            LAB_POS,
            format!(
                "LAB TWIN: MY CLOCK {:.2} s\nYOUR CLOCK {:.2} s",
                experiment.result_lab_time, experiment.result_traveler_time,
            ),
        ),
        _ => (Vec2::ZERO, String::new()),
    };
    if message.is_empty() {
        *visibility = Visibility::Hidden;
        return;
    }
    text.0 = message;
    transform.translation = (position + Vec2::new(0.0, 76.0)).extend(16.0);
    *visibility = Visibility::Visible;
}

fn signal_message(payload: u64) -> (String, Color) {
    let (kind, actor_id, data) = payload_parts(payload);
    match kind {
        PAYLOAD_ASK_CLOCK => (
            format!(
                "CLOCK? → {}",
                crate::character_name(actor_id).to_uppercase()
            ),
            Color::srgb(0.35, 0.92, 1.0),
        ),
        PAYLOAD_CLOCK_REPORT => (
            format!("{:.1} s", decoded_clock_seconds(data)),
            Color::srgb(0.95, 0.98, 1.0),
        ),
        PAYLOAD_DOPPLER_NOTE => (
            format!("♪ {}", musical_note_name(crate::EMITTED_FREQUENCY)),
            Color::srgb(0.35, 0.60, 1.0),
        ),
        PAYLOAD_LIGHT_TAG => ("LIGHT TAG!".to_owned(), Color::srgb(1.0, 0.88, 0.20)),
        _ => ("LIGHT".to_owned(), Color::WHITE),
    }
}

fn update_lab_signal_visuals(
    signals: Res<RelativitySignalView2d>,
    mut visuals: ParamSet<(
        Query<(
            &LabSignalVisual,
            &mut Sprite,
            &mut Transform,
            &mut Visibility,
        )>,
        Query<(
            &LabSignalLabel,
            &mut Text2d,
            &mut TextColor,
            &mut Transform,
            &mut Visibility,
        )>,
    )>,
) {
    {
        let mut sprites = visuals.p0();
        for (slot, mut sprite, mut transform, mut visibility) in &mut sprites {
            if let Some(signal) = signals.active_signals.get(slot.0) {
                let (_, color) = signal_message(signal.payload);
                transform.translation.x = signal.position.x;
                transform.translation.y = signal.position.y;
                sprite.color = color;
                sprite.custom_size = Some(Vec2::splat(12.0 + signal.age as f32 * 1.5));
                *visibility = Visibility::Visible;
            } else {
                *visibility = Visibility::Hidden;
            }
        }
    }
    {
        let mut labels = visuals.p1();
        for (slot, mut text, mut color, mut transform, mut visibility) in &mut labels {
            if let Some(signal) = signals.active_signals.get(slot.0) {
                let (label, tint) = signal_message(signal.payload);
                text.0 = label;
                color.0 = tint.clone();
                transform.translation = (signal.position + Vec2::new(0.0, 22.0)).extend(13.0);
                *visibility = Visibility::Visible;
            } else {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

fn update_doppler_music_visuals(
    time: Res<Time>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    traveler: Query<&ambition_platformer2d::engine_core::BodyKinematics, With<TravelerTwin>>,
    characters: Query<(
        &TwinTrackCharacter,
        &ambition_platformer2d::engine_core::BodyKinematics,
    )>,
    // ⚠ **the three marker sets are disjoint, and Bevy needs to be TOLD.** All
    // three take `&mut Visibility`, and a `With<..>` marker does not prove
    // exclusivity — nothing stops an entity carrying two of them, so parameter
    // validation panics at first run rather than at compile time. The
    // `Without<..>` filters are the statement that the bars, the labels and the
    // beat rings are three different bodies.
    mut bars: Query<
        (&mut Sprite, &mut Transform, &mut Visibility),
        (
            With<DjFrequencyBar>,
            Without<DjFrequencyLabel>,
            Without<DjBeatRing>,
        ),
    >,
    mut labels: Query<
        (&mut Text2d, &mut TextColor, &mut Visibility),
        (
            With<DjFrequencyLabel>,
            Without<DjFrequencyBar>,
            Without<DjBeatRing>,
        ),
    >,
    mut rings: Query<
        (&DjBeatRing, &mut Sprite, &mut Transform, &mut Visibility),
        (Without<DjFrequencyBar>, Without<DjFrequencyLabel>),
    >,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    let active = matches!(
        experiment.phase,
        TwinTrackPhase::DopplerDance | TwinTrackPhase::LightTag
    );
    let frequency = traveler
        .single()
        .ok()
        .and_then(|traveler| {
            characters
                .iter()
                .find(|(character, _)| character.role == crate::TwinTrackRole::DopplerDj)
                .and_then(|(_, dj)| doppler_frequency_for_target(traveler, dj))
        })
        .unwrap_or(crate::EMITTED_FREQUENCY)
        .max(1.0);
    let accepted = (DOPPLER_PASSBAND_MIN..=DOPPLER_PASSBAND_MAX).contains(&frequency);
    let tint = if accepted {
        Color::srgb(0.30, 1.0, 0.45)
    } else if frequency < DOPPLER_PASSBAND_MIN {
        Color::srgb(0.25, 0.55, 1.0)
    } else {
        Color::srgb(1.0, 0.35, 0.75)
    };
    let octave_error = (frequency / DOPPLER_TARGET_FREQUENCY).log2().abs();
    let match_fraction = (1.0 - octave_error).clamp(0.06, 1.0) as f32;

    for (mut sprite, mut transform, mut visibility) in &mut bars {
        sprite.color = tint.clone();
        sprite.custom_size = Some(Vec2::new(190.0 * match_fraction, 12.0));
        transform.translation.x = DJ_POS.x - 95.0 + 95.0 * match_fraction;
        *visibility = if active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut text, mut color, mut visibility) in &mut labels {
        text.0 = format!(
            "DJ HEARS {}  {:.1} Hz   TARGET G3  {:.0} Hz",
            musical_note_name(frequency),
            frequency,
            DOPPLER_TARGET_FREQUENCY,
        );
        color.0 = tint.clone();
        *visibility = if active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (ring, mut sprite, mut transform, mut visibility) in &mut rings {
        let phase = ((time.elapsed_secs() * (frequency as f32 / 49.0) * 0.45
            + ring.0 as f32 / 3.0)
            .fract())
        .clamp(0.0, 1.0);
        let size = 44.0 + phase * 95.0;
        sprite.custom_size = Some(Vec2::splat(size));
        sprite.color = Color::srgba(
            if accepted { 0.30 } else { 0.25 },
            if accepted { 1.0 } else { 0.62 },
            if accepted { 0.45 } else { 1.0 },
            (1.0 - phase) * 0.42,
        );
        transform.translation = DJ_POS.extend(7.0);
        *visibility = if active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn update_light_tag_guides(
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    traveler: Query<&ambition_platformer2d::engine_core::BodyKinematics, With<crate::TravelerTwin>>,
    targeting: Res<RelativisticTargetingView2d>,
    mut guides: ParamSet<(
        Query<(&mut Sprite, &mut Transform, &mut Visibility), With<LabAimLine>>,
        Query<(&mut Transform, &mut Visibility), With<LabDelayedImageMarker>>,
        Query<(&mut Transform, &mut Visibility), With<LabInterceptMarker>>,
        Query<(&mut Text2d, &mut Visibility), With<LabTagGuideLabel>>,
        Query<(&mut Sprite, &mut Transform, &mut Visibility), With<OpticalAimLine>>,
        Query<(&mut Transform, &mut Visibility), With<OpticalInterceptMarker>>,
        Query<(&mut Text2d, &mut Visibility), With<OpticalTagGuideLabel>>,
    )>,
) {
    let (Ok(experiment), Ok(traveler)) = (experiment.single(), traveler.single()) else {
        return;
    };
    let active = experiment.phase == TwinTrackPhase::LightTag;
    let target = targeting
        .targets
        .iter()
        .find(|target| target.label == "Photon Fox");

    {
        let mut lab_aim = guides.p0();
        if let Ok((mut sprite, mut transform, mut visibility)) = lab_aim.single_mut() {
            if active {
                let start = traveler.pos;
                let local = experiment.aim_direction.normalize_or_zero();
                let coordinate = InvariantSpeed::new(f64::from(INVARIANT_SPEED))
                    .ok()
                    .and_then(|c| {
                        coordinate_photon_direction_from_observer(
                            [f64::from(local.x), f64::from(local.y), 0.0],
                            [f64::from(traveler.vel.x), f64::from(traveler.vel.y), 0.0],
                            c,
                        )
                    })
                    .map(|direction| Vec2::new(direction[0] as f32, direction[1] as f32))
                    .unwrap_or(local)
                    .normalize_or_zero();
                let end = start + coordinate * 230.0;
                set_line(&mut sprite, &mut transform, start, end, 4.0, 13.0);
                *visibility = Visibility::Visible;
            } else {
                *visibility = Visibility::Hidden;
            }
        }
    }
    {
        let mut delayed = guides.p1();
        if let Ok((mut transform, mut visibility)) = delayed.single_mut() {
            if let Some(position) = target
                .and_then(|target| target.retarded_position)
                .filter(|_| active)
            {
                transform.translation = position.extend(13.0);
                *visibility = Visibility::Visible;
            } else {
                *visibility = Visibility::Hidden;
            }
        }
    }
    {
        let mut intercept = guides.p2();
        if let Ok((mut transform, mut visibility)) = intercept.single_mut() {
            if let Some(target) = target.filter(|_| active && experiment.tag_hits < 2) {
                transform.translation = Vec2::new(
                    target.intercept_event.position[0] as f32,
                    target.intercept_event.position[1] as f32,
                )
                .extend(14.0);
                *visibility = Visibility::Visible;
            } else {
                *visibility = Visibility::Hidden;
            }
        }
    }
    {
        let mut lab_labels = guides.p3();
        for (mut text, mut visibility) in &mut lab_labels {
            text.0 = match experiment.tag_hits {
                0 => "RED = VISIBLE IMAGE   YELLOW = NOW   GREEN = INTERCEPT   CYAN = AIM",
                1 => "ROUND 2: YELLOW NOW-MARKER HIDDEN   RED = VISIBLE   GREEN = INTERCEPT",
                _ => "ROUND 3: ONLY THE RED VISIBLE IMAGE AND YOUR CYAN AIM REMAIN",
            }
            .to_owned();
            *visibility = if active && experiment.view_mode == TwinTrackViewMode::Laboratory {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
    {
        let mut optical_aim = guides.p4();
        if let Ok((mut sprite, mut transform, mut visibility)) = optical_aim.single_mut() {
            if active && experiment.view_mode == TwinTrackViewMode::Optical {
                let start = SKY_CENTER;
                let end = start + experiment.aim_direction.normalize_or_zero() * SKY_RADIUS;
                set_line(&mut sprite, &mut transform, start, end, 4.0, 14.0);
                *visibility = Visibility::Visible;
            } else {
                *visibility = Visibility::Hidden;
            }
        }
    }
    {
        let mut optical_intercept = guides.p5();
        if let Ok((mut transform, mut visibility)) = optical_intercept.single_mut() {
            if let Some(target) = target.filter(|_| {
                active
                    && experiment.tag_hits < 2
                    && experiment.view_mode == TwinTrackViewMode::Optical
            }) {
                transform.translation = (SKY_CENTER
                    + target.observer_local_emission_direction.normalize_or_zero()
                        * SKY_RADIUS
                        * 0.82)
                    .extend(15.0);
                *visibility = Visibility::Visible;
            } else {
                *visibility = Visibility::Hidden;
            }
        }
    }
    {
        let mut optical_labels = guides.p6();
        for (mut text, mut visibility) in &mut optical_labels {
            text.0 = match experiment.tag_hits {
                0 => "RED FOX = VISIBLE IMAGE • GREEN = SEND LIGHT HERE • CYAN = YOUR AIM",
                1 => "ROUND 2 • GREEN STILL SHOWS THE INTERCEPT • CYAN IS YOUR AIM",
                _ => "ROUND 3 • NO INTERCEPT MARKER • LEAD THE RED VISIBLE IMAGE YOURSELF",
            }
            .to_owned();
            *visibility = if active && experiment.view_mode == TwinTrackViewMode::Optical {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn update_observatory_title(
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut titles: Query<(&mut Text2d, &mut Visibility), With<ObservatoryTitle>>,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    for (mut title, mut visibility) in &mut titles {
        title.0 = match experiment.view_mode {
            TwinTrackViewMode::Laboratory => "LABORATORY MAP".to_owned(),
            TwinTrackViewMode::Optical => "WHAT REACHES YOU NOW".to_owned(),
            TwinTrackViewMode::Spacetime if experiment.phase == TwinTrackPhase::Complete => {
                format!(
                    "SPACE + TIME REPLAY  {:>3}% — LEFT/RIGHT SCRUBS",
                    (experiment.replay_cursor * 100.0).round() as u32,
                )
            }
            TwinTrackViewMode::Spacetime => "2D SPACE + TIME SCULPTURE".to_owned(),
        };
        *visibility = if experiment.view_mode == TwinTrackViewMode::Laboratory {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
}

fn update_optical_observer_marker(
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut markers: Query<&mut Visibility, With<OpticalObserverMarker>>,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    let visible = experiment.view_mode == TwinTrackViewMode::Optical;
    for mut visibility in &mut markers {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn update_observatory_stars(
    optical: Res<RelativisticOpticalView2d>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut stars: Query<(
        &ObservatoryStar,
        &mut Sprite,
        &mut Transform,
        &mut Visibility,
    )>,
) {
    let (Some(observer), Ok(experiment)) = (optical.observer.as_ref(), experiment.single()) else {
        return;
    };
    let visible = experiment.view_mode == TwinTrackViewMode::Optical;
    let Ok(c) = InvariantSpeed::new(observer.invariant_speed) else {
        return;
    };
    let observer_velocity = [
        f64::from(observer.coordinate_velocity.x),
        f64::from(observer.coordinate_velocity.y),
        0.0,
    ];
    for (star, mut sprite, mut transform, mut visibility) in &mut stars {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if !visible {
            continue;
        }
        let photon = -star.rest_direction;
        let Some(observation) = observe_photon_direction(
            [f64::from(photon.x), f64::from(photon.y), 0.0],
            observer_velocity,
            c,
        ) else {
            continue;
        };
        let direction = Vec2::new(
            observation.apparent_source_direction[0] as f32,
            observation.apparent_source_direction[1] as f32,
        )
        .normalize_or_zero();
        transform.translation =
            (SKY_CENTER + direction * SKY_RADIUS * star.radial_fraction).extend(2.0);
        let brightness = (star.intensity * observation.beaming_factor as f32)
            .sqrt()
            .clamp(0.2, 3.0);
        sprite.custom_size = Some(Vec2::splat(star.size * brightness.clamp(0.7, 2.8)));
        sprite.color = spectral_color(observation.doppler_factor, brightness);
    }
}

fn update_optical_proxies(
    optical: Res<RelativisticOpticalView2d>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut proxies: Query<
        (&OpticalProxy, &mut Sprite, &mut Transform, &mut Visibility),
        Without<OpticalProxyLabel>,
    >,
    mut labels: Query<
        (
            &OpticalProxyLabel,
            &mut TextColor,
            &mut Transform,
            &mut Visibility,
        ),
        Without<OpticalProxy>,
    >,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    let visible = experiment.view_mode == TwinTrackViewMode::Optical;
    for (proxy, mut sprite, mut transform, mut visibility) in &mut proxies {
        let source = optical
            .sources
            .iter()
            .find(|source| source.label == proxy.0);
        *visibility = if visible && source.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let Some(source) = source else {
            continue;
        };
        let position = SKY_CENTER + source.apparent_source_direction * (SKY_RADIUS * 0.82);
        transform.translation = position.extend(9.0);
        sprite.custom_size = Some(Vec2::splat(source.apparent_radius.clamp(14.0, 42.0)));
        sprite.color = if proxy.0 == "Photon Fox" {
            Color::srgb(1.0, 0.18, 0.18)
        } else {
            spectral_color(source.doppler_factor, source.beaming_factor as f32)
        };
    }
    for (label, mut color, mut transform, mut visibility) in &mut labels {
        let source = optical
            .sources
            .iter()
            .find(|source| source.label == label.0);
        *visibility = if visible && source.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let Some(source) = source else {
            continue;
        };
        let position = SKY_CENTER
            + source.apparent_source_direction * (SKY_RADIUS * 0.82)
            + Vec2::new(0.0, -28.0);
        transform.translation = position.extend(11.0);
        color.0 = if label.0 == "Photon Fox" {
            Color::srgb(1.0, 0.35, 0.35)
        } else {
            spectral_color(source.doppler_factor, 1.0)
        };
    }
}

fn replay_bounds(history: &WorldlineHistoryView2d) -> Option<(f64, f64)> {
    let start = history
        .tracks
        .values()
        .filter_map(|samples| samples.front().map(|sample| sample.coordinate_time))
        .min_by(f64::total_cmp)?;
    let end = history
        .tracks
        .values()
        .filter_map(|samples| samples.back().map(|sample| sample.coordinate_time))
        .max_by(f64::total_cmp)?;
    Some((start, end.max(start)))
}

fn selected_spacetime_time(
    experiment: &TwinTrackExperiment,
    history: &WorldlineHistoryView2d,
    live_now: f64,
) -> f64 {
    if experiment.phase != TwinTrackPhase::Complete {
        return live_now;
    }
    replay_bounds(history).map_or(live_now, |(start, end)| {
        start + (end - start) * f64::from(experiment.replay_cursor.clamp(0.0, 1.0))
    })
}

fn track_position_at(
    history: &WorldlineHistoryView2d,
    label: &str,
    coordinate_time: f64,
) -> Option<Vec2> {
    let samples = history.tracks.get(&WorldlineTrackId(label.to_owned()))?;
    samples
        .iter()
        .rev()
        .find(|sample| sample.coordinate_time <= coordinate_time)
        .or_else(|| samples.front())
        .map(|sample| sample.position)
}

fn emitter_track_label(emitter_tag: u64, payload: u64) -> Option<&'static str> {
    if emitter_tag == TRAVELER_TAG {
        return Some("traveler");
    }
    let (_, actor_id, _) = payload_parts(payload);
    match actor_id {
        COURIER_ID => Some("Courier"),
        DRIFTER_ID => Some("Drifter"),
        SPINNER_ID => Some("Spinner"),
        DJ_ID => Some("DJ Blue Shift"),
        TAGGER_ID => Some("Photon Fox"),
        _ => None,
    }
}

fn update_spacetime_sculpture(
    history: Res<WorldlineHistoryView2d>,
    optical: Res<RelativisticOpticalView2d>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut dots: Query<(&TraceDot, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    let visible = experiment.view_mode == TwinTrackViewMode::Spacetime;
    let live_now = optical
        .observer
        .as_ref()
        .map_or(0.0, |observer| observer.coordinate_time);
    let now = selected_spacetime_time(experiment, &history, live_now);
    let mut rows = Vec::new();
    if visible {
        for (label, samples) in &history.tracks {
            let color = track_color(label.as_str());
            for sample in samples.iter().filter(|sample| {
                sample.coordinate_time <= now
                    && (experiment.phase == TwinTrackPhase::Complete
                        || now - sample.coordinate_time <= TRACE_WINDOW_SECONDS)
            }) {
                rows.push((sample.coordinate_time, sample.position, color));
            }
        }
        rows.sort_by(|left, right| left.0.total_cmp(&right.0));
        if rows.len() > TRACE_DOT_COUNT {
            let stride = rows.len() as f32 / TRACE_DOT_COUNT as f32;
            rows = (0..TRACE_DOT_COUNT)
                .map(|index| rows[(index as f32 * stride) as usize])
                .collect();
        }
    }
    for (slot, mut sprite, mut transform, mut visibility) in &mut dots {
        if let Some((time, position, color)) = rows.get(slot.0).copied() {
            transform.translation = spacetime_projection(position, now - time).extend(5.0);
            sprite.color = color;
            sprite.custom_size = Some(Vec2::splat(5.0));
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn update_spacetime_signals(
    signals: Res<RelativitySignalView2d>,
    history: Res<WorldlineHistoryView2d>,
    optical: Res<RelativisticOpticalView2d>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut dots: Query<(&SignalDot, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    let visible = experiment.view_mode == TwinTrackViewMode::Spacetime;
    let live_now = optical
        .observer
        .as_ref()
        .map_or(signals.coordinate_time, |observer| observer.coordinate_time);
    let now = selected_spacetime_time(experiment, &history, live_now);
    let replay_rows = if visible && experiment.phase == TwinTrackPhase::Complete {
        signals
            .recent_arrivals
            .iter()
            .filter(|arrival| arrival.coordinate_time <= now)
            .filter_map(|arrival| {
                let label = emitter_track_label(arrival.emitter_tag, arrival.payload)?;
                let start = track_position_at(&history, label, arrival.signal_emission_time)?;
                Some((
                    start,
                    arrival.position,
                    arrival.signal_emission_time,
                    arrival.coordinate_time,
                ))
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    for (slot, mut sprite, mut transform, mut visibility) in &mut dots {
        if visible && experiment.phase == TwinTrackPhase::Complete {
            if let Some((start_position, end_position, start_time, end_time)) =
                replay_rows.get(slot.0).copied()
            {
                let start = spacetime_projection(start_position, now - start_time);
                let end = spacetime_projection(end_position, now - end_time);
                set_line(&mut sprite, &mut transform, start, end, 3.0, 8.0);
                *visibility = Visibility::Visible;
                continue;
            }
        } else if visible {
            if let Some(signal) = signals.active_signals.get(slot.0) {
                let start = spacetime_projection(signal.emission_position, signal.age);
                let end = spacetime_projection(signal.position, 0.0);
                set_line(&mut sprite, &mut transform, start, end, 3.0, 8.0);
                *visibility = Visibility::Visible;
                continue;
            }
        }
        *visibility = Visibility::Hidden;
    }
}

fn update_light_cone(
    optical: Res<RelativisticOpticalView2d>,
    history: Res<WorldlineHistoryView2d>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut lines: Query<(&ConeLine, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    let visible = experiment.view_mode == TwinTrackViewMode::Spacetime;
    let Some(live_observer) = optical.observer.as_ref() else {
        for (_, _, _, mut visibility) in &mut lines {
            *visibility = Visibility::Hidden;
        }
        return;
    };
    let now = selected_spacetime_time(experiment, &history, live_observer.coordinate_time);
    let observer_position = if experiment.phase == TwinTrackPhase::Complete {
        track_position_at(&history, "traveler", now).unwrap_or(live_observer.position)
    } else {
        live_observer.position
    };
    for (line, mut sprite, mut transform, mut visibility) in &mut lines {
        if !visible {
            *visibility = Visibility::Hidden;
            continue;
        }
        let origin = spacetime_projection(observer_position, 0.0);
        let direction = match line.0 {
            0 => Vec2::X,
            1 => -Vec2::X,
            2 => Vec2::Y,
            _ => -Vec2::Y,
        };
        let age = 0.72_f64;
        let past_position =
            observer_position + direction * live_observer.invariant_speed as f32 * age as f32;
        let end = spacetime_projection(past_position, age);
        set_line(&mut sprite, &mut transform, origin, end, 2.0, 3.0);
        *visibility = Visibility::Visible;
    }
}

fn update_spacetime_guides(
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut axes: Query<(&SpacetimeAxis, &mut Sprite, &mut Transform, &mut Visibility)>,
    mut guides: Query<&mut Visibility, (With<SpacetimeGuide>, Without<SpacetimeAxis>)>,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    let visible = experiment.view_mode == TwinTrackViewMode::Spacetime;
    let origin = Vec2::new(-405.0, -245.0);
    for (axis, mut sprite, mut transform, mut visibility) in &mut axes {
        let end = match axis.0 {
            0 => origin + Vec2::new(760.0, 126.0),
            1 => origin + Vec2::new(-250.0, 175.0),
            _ => origin + Vec2::new(0.0, 470.0),
        };
        set_line(&mut sprite, &mut transform, origin, end, 3.0, 2.0);
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut visibility in &mut guides {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn cleanup_visuals_when_inactive(
    mut commands: Commands,
    roots: Query<&ambition_platformer2d::runtime::demo_fixture::ActiveRoomMetadata>,
    visuals: Query<Entity, With<TwinTrackVisible>>,
) {
    if twintrack_is_active(&roots) {
        return;
    }
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
}

fn spacetime_projection(position: Vec2, age: f64) -> Vec2 {
    let normalized = Vec2::new(
        (position.x - LAB_POS.x) / ROOM_WIDTH,
        (position.y - LAB_POS.y) / ROOM_HEIGHT,
    );
    Vec2::new(
        -160.0 + normalized.x * 720.0 - normalized.y * 300.0,
        -245.0 + normalized.x * 120.0 + normalized.y * 200.0 - age as f32 * 47.0,
    )
}

fn set_line(
    sprite: &mut Sprite,
    transform: &mut Transform,
    start: Vec2,
    end: Vec2,
    width: f32,
    z: f32,
) {
    let delta = end - start;
    sprite.custom_size = Some(Vec2::new(delta.length().max(1.0), width));
    transform.translation = ((start + end) * 0.5).extend(z);
    transform.rotation = Quat::from_rotation_z(delta.y.atan2(delta.x));
}

fn track_color(label: &str) -> Color {
    match label {
        "traveler" => Color::srgb(0.30, 1.0, 0.95),
        "laboratory" => Color::srgb(1.0, 1.0, 1.0),
        "Courier" => Color::srgb(0.35, 0.90, 1.0),
        "Drifter" => Color::srgb(0.85, 0.55, 1.0),
        "Spinner" => Color::srgb(1.0, 0.65, 0.25),
        "DJ Blue Shift" => Color::srgb(0.25, 0.55, 1.0),
        "Photon Fox" => Color::srgb(1.0, 0.32, 0.24),
        _ => Color::srgb(0.65, 0.70, 0.80),
    }
}

fn spectral_color(doppler_factor: f64, luminance: f32) -> Color {
    let shift = doppler_factor.max(1.0e-6).ln().clamp(-1.5, 1.5) as f32 / 1.5;
    let brightness = luminance.clamp(0.15, 2.8);
    let (red, green, blue) = if shift >= 0.0 {
        (1.0 - 0.72 * shift, 0.82 + 0.18 * shift, 0.90 + 0.10 * shift)
    } else {
        let red_shift = -shift;
        (1.0, 0.88 - 0.55 * red_shift, 0.86 - 0.72 * red_shift)
    };
    Color::srgba(
        (red * brightness).min(1.0),
        (green * brightness).min(1.0),
        (blue * brightness).min(1.0),
        brightness.min(1.0),
    )
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn unit_from_u64(value: u64) -> f32 {
    ((value >> 40) as f32) / ((1_u32 << 24) as f32)
}
