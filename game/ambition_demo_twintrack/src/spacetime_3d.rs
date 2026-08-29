//! Real perspective 2+1D spacetime presentation for TwinTrack.
//!
//! X and Z are the plaza's two spatial coordinates. Y is `ct`, so light rays
//! form 45-degree cones in graph units. The scene is presentation-only: it
//! consumes bounded worldline/signal views and never writes simulation state.

use ambition_platformer2d::relativity2d::{
    RelativitySignalView2d, WorldlineHistoryView2d, WorldlineSample2d,
};
use bevy::camera::visibility::RenderLayers;
use bevy::camera::Viewport;
use bevy::camera::{
    ClearColorConfig, OrthographicProjection, PerspectiveProjection, Projection, ScalingMode,
};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::{
    LaboratoryTwin, TravelerTwin, TwinTrackExperiment, TwinTrackPhase, INVARIANT_SPEED,
    ROOM_HEIGHT, ROOM_WIDTH, TWINTRACK_EXPERIENCE,
};

const SPACETIME_LAYER: usize = 29;
const SPACETIME_OVERLAY_LAYER: usize = 30;
const MINIMAP_MARGIN_PX: u32 = 18;
const MINIMAP_TOP_PX: u32 = 72;
const MINIMAP_MIN_PX: u32 = 220;
const MINIMAP_LOGICAL_WIDTH: f32 = 480.0;
const MINIMAP_LOGICAL_HEIGHT: f32 = 320.0;
const SPACE_SCALE: f32 = 0.18;
const LIVE_WINDOW_SECONDS: f64 = 4.5;
const REPLAY_WINDOW_SECONDS: f64 = 6.0;
const TRACK_SEGMENTS: usize = 84;
const CLOCK_BEADS: usize = 18;
const SIGNAL_SEGMENTS: usize = 32;
const CONE_SPOKES: usize = 16;
const CONE_RINGS: usize = 4;
const CONE_RING_SEGMENTS: usize = 16;
const LINE_THICKNESS: f32 = 5.0;

const TRACK_LABELS: [&str; 7] = [
    "traveler",
    "laboratory",
    "Courier",
    "Drifter",
    "Spinner",
    "DJ Blue Shift",
    "Photon Fox",
];

#[derive(Resource, Clone, Copy, Debug)]
pub(crate) struct SpacetimeMinimapState {
    pub visible: bool,
}

impl Default for SpacetimeMinimapState {
    fn default() -> Self {
        Self { visible: true }
    }
}

#[derive(Component)]
struct TwinTrackSpacetime3d;

#[derive(Component)]
struct SpacetimeCamera3d;

#[derive(Component)]
struct SpacetimeOverlayCamera;

#[derive(Component)]
struct WorldlineSegment3d {
    track: &'static str,
    slot: usize,
}

#[derive(Component)]
struct ClockBead3d {
    track: &'static str,
    slot: usize,
}

#[derive(Component)]
struct CurrentEvent3d(&'static str);

#[derive(Component)]
struct SignalSegment3d(usize);

#[derive(Component)]
struct ConeSpoke3d(usize);

#[derive(Component)]
struct ConeRing3d {
    ring: usize,
    segment: usize,
}

#[derive(Component, Clone, Copy)]
enum PlaneKind3d {
    LaboratoryNow,
    ObserverNow,
}

#[derive(Component)]
struct SpacetimePlane3d(PlaneKind3d);

#[derive(Component)]
struct SpacetimeAxis3d(usize);

#[derive(Component)]
struct SpacetimeLegend3d(usize);

pub(crate) fn install(app: &mut App) {
    app.init_resource::<SpacetimeMinimapState>();

    // ⭐⭐ THE SPACETIME DISPLAY IS DORMANT OUTSIDE TWINTRACK, and it now says so
    // ONCE instead of thirteen times. Every one of these already no-opped
    // elsewhere — most by an early return on a `TwinTrackExperiment` query that
    // matches nothing — which is the "install many systems, then spend every
    // frame discovering they have nothing to do" shape. The predicate was
    // already in this file; only two of the thirteen consulted it.
    //
    // ⭐ A tuple-level `run_if` in Bevy 0.18 is COLLECTIVE — it builds one
    // anonymous set and is evaluated at most once per schedule run — so this is
    // twelve system invocations replaced by one condition.
    //
    // ⚠ NOT A MEASURED SPEED WIN, and it should not be sold as one: twelve
    // early-returning systems are microseconds, far under this machine's noise
    // floor. It is here because a dormant capability should be dormant, which
    // is the invariant the whole runtime-composition direction rests on.
    app.add_systems(
        Update,
        (
            spawn_spacetime_3d,
            toggle_spacetime_minimap,
            sync_spacetime_cameras,
            update_spacetime_camera,
            update_worldlines,
            update_clock_beads,
            update_current_events,
            update_signal_paths,
            update_light_cone,
            update_simultaneity_planes,
            update_axes,
            update_legend,
        )
            .run_if(twintrack_display_is_live),
    );

    // ⛔⛔ THE CLEANUP IS DELIBERATELY UNGATED, and gating it with the rest would
    // be a bug rather than an optimization. It despawns the 3D visuals and
    // restores the minimap flag precisely WHEN TWINTRACK IS NOT ACTIVE — putting
    // it inside a live-only gate means leaving the display standing forever
    // after the experience ends. Its own first line is `if twintrack_is_active
    // { return; }`, which is the opposite condition to the tuple above.
    app.add_systems(Update, cleanup_spacetime_3d_when_inactive);
}

/// Run condition: the twintrack spacetime display has something to draw.
///
/// The same question [`twintrack_is_active`] answers, in the shape a run
/// condition needs. Kept beside it so the two cannot drift.
fn twintrack_display_is_live(
    roots: Query<&ambition_platformer2d::runtime::demo_fixture::ActiveRoomMetadata>,
) -> bool {
    twintrack_is_active(&roots)
}

fn twintrack_is_active(
    roots: &Query<&ambition_platformer2d::runtime::demo_fixture::ActiveRoomMetadata>,
) -> bool {
    roots
        .iter()
        .any(|metadata| metadata.0.mode.as_deref() == Some(TWINTRACK_EXPERIENCE))
}

fn track_color(label: &str) -> Color {
    match label {
        "traveler" => Color::srgb(0.20, 1.0, 0.92),
        "laboratory" => Color::srgb(0.96, 0.98, 1.0),
        "Courier" => Color::srgb(0.30, 0.90, 1.0),
        "Drifter" => Color::srgb(0.86, 0.48, 1.0),
        "Spinner" => Color::srgb(1.0, 0.62, 0.16),
        "DJ Blue Shift" => Color::srgb(0.25, 0.52, 1.0),
        "Photon Fox" => Color::srgb(1.0, 0.25, 0.18),
        _ => Color::srgb(0.72, 0.76, 0.84),
    }
}

fn unlit_material(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        unlit: true,
        ..default()
    })
}

fn translucent_material(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: true,
        ..default()
    })
}

/// `Option<ResMut<Assets<..>>>`, NOT `ResMut`, and this is not a style
/// preference. A plain `ResMut<Assets<Mesh>>` is a system-parameter VALIDATION
/// failure in any app without the render plugins — and under Bevy 0.18 that is a
/// panic through `Main::run_main`, not a skipped system. Every headless test in
/// the workspace that steps the schedule dies, whatever it was about: fifteen
/// boss, actor-phase and reachability tests went down together, none of them
/// twintrack's.
///
/// the same shape the presentation crates already use —
/// `ambition_render::rendering::unauthored_volumes` and
/// `ambition_portal2d_presentation::visuals` both take the assets optionally for
/// this reason. A 3D overlay is presentation; a simulation test must be able to
/// run without one.
fn spawn_spacetime_3d(
    mut commands: Commands,
    roots: Query<&ambition_platformer2d::runtime::demo_fixture::ActiveRoomMetadata>,
    existing: Query<(), With<TwinTrackSpacetime3d>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    if !twintrack_is_active(&roots) || !existing.is_empty() {
        return;
    }
    // No render app, no meshes to build into. The overlay simply does not exist
    // in a headless composition, which is the honest answer rather than a panic.
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };

    let layer = RenderLayers::layer(SPACETIME_LAYER);
    let overlay_layer = RenderLayers::layer(SPACETIME_OVERLAY_LAYER);
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    commands.spawn((
        TwinTrackSpacetime3d,
        SpacetimeCamera3d,
        Camera3d::default(),
        Tonemapping::None,
        Msaa::Sample4,
        Camera {
            order: 4,
            is_active: true,
            // Like the Lunex pause cube, this is a higher-order Camera3d
            // composited over the live Camera2d. Unlike Lunex, a physical
            // viewport constrains its clear to the minimap rectangle.
            clear_color: ClearColorConfig::Custom(Color::srgb(0.006, 0.008, 0.020)),
            ..default()
        },
        layer.clone(),
        Projection::Perspective(PerspectiveProjection {
            fov: 52.0_f32.to_radians(),
            ..default()
        }),
        Transform::from_xyz(560.0, 170.0, 620.0).looking_at(Vec3::ZERO, Vec3::Y),
        Name::new("TwinTrack 3D spacetime camera"),
    ));

    commands.spawn((
        TwinTrackSpacetime3d,
        SpacetimeOverlayCamera,
        Camera2d,
        Camera {
            order: 5,
            is_active: true,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        overlay_layer.clone(),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: MINIMAP_LOGICAL_WIDTH,
                height: MINIMAP_LOGICAL_HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::default(),
        Name::new("TwinTrack 3D spacetime overlay camera"),
    ));

    let bead_material = unlit_material(&mut materials, Color::srgb(1.0, 0.95, 0.52));
    for label in TRACK_LABELS {
        let material = unlit_material(&mut materials, track_color(label));
        for slot in 0..TRACK_SEGMENTS {
            commands.spawn((
                TwinTrackSpacetime3d,
                WorldlineSegment3d { track: label, slot },
                Mesh3d(cube.clone()),
                MeshMaterial3d(material.clone()),
                Visibility::Hidden,
                Transform::default(),
                layer.clone(),
                Name::new(format!("TwinTrack 3D worldline {label} {slot}")),
            ));
        }
        for slot in 0..CLOCK_BEADS {
            commands.spawn((
                TwinTrackSpacetime3d,
                ClockBead3d { track: label, slot },
                Mesh3d(cube.clone()),
                MeshMaterial3d(bead_material.clone()),
                Visibility::Hidden,
                Transform::default(),
                layer.clone(),
                Name::new(format!("TwinTrack 3D clock bead {label} {slot}")),
            ));
        }
        commands.spawn((
            TwinTrackSpacetime3d,
            CurrentEvent3d(label),
            Mesh3d(cube.clone()),
            MeshMaterial3d(material),
            Visibility::Hidden,
            Transform::default(),
            layer.clone(),
            Name::new(format!("TwinTrack 3D current event {label}")),
        ));
    }

    let signal_material = unlit_material(&mut materials, Color::srgb(1.0, 0.90, 0.16));
    for slot in 0..SIGNAL_SEGMENTS {
        commands.spawn((
            TwinTrackSpacetime3d,
            SignalSegment3d(slot),
            Mesh3d(cube.clone()),
            MeshMaterial3d(signal_material.clone()),
            Visibility::Hidden,
            Transform::default(),
            layer.clone(),
            Name::new(format!("TwinTrack 3D light signal {slot}")),
        ));
    }

    let cone_material = unlit_material(&mut materials, Color::srgba(0.20, 0.72, 1.0, 0.86));
    for index in 0..CONE_SPOKES {
        commands.spawn((
            TwinTrackSpacetime3d,
            ConeSpoke3d(index),
            Mesh3d(cube.clone()),
            MeshMaterial3d(cone_material.clone()),
            Visibility::Hidden,
            Transform::default(),
            layer.clone(),
            Name::new(format!("TwinTrack 3D past-light-cone spoke {index}")),
        ));
    }
    for ring in 0..CONE_RINGS {
        for segment in 0..CONE_RING_SEGMENTS {
            commands.spawn((
                TwinTrackSpacetime3d,
                ConeRing3d { ring, segment },
                Mesh3d(cube.clone()),
                MeshMaterial3d(cone_material.clone()),
                Visibility::Hidden,
                Transform::default(),
                layer.clone(),
                Name::new(format!(
                    "TwinTrack 3D past-light-cone ring {ring}:{segment}"
                )),
            ));
        }
    }

    let lab_plane = translucent_material(&mut materials, Color::srgba(0.92, 0.96, 1.0, 0.10));
    let observer_plane = translucent_material(&mut materials, Color::srgba(0.18, 1.0, 0.92, 0.13));
    for (kind, material) in [
        (PlaneKind3d::LaboratoryNow, lab_plane),
        (PlaneKind3d::ObserverNow, observer_plane),
    ] {
        commands.spawn((
            TwinTrackSpacetime3d,
            SpacetimePlane3d(kind),
            Mesh3d(cube.clone()),
            MeshMaterial3d(material),
            Visibility::Hidden,
            Transform::default(),
            layer.clone(),
            Name::new(match kind {
                PlaneKind3d::LaboratoryNow => "TwinTrack 3D laboratory now plane",
                PlaneKind3d::ObserverNow => "TwinTrack 3D observer now plane",
            }),
        ));
    }

    for (index, color) in [
        Color::srgb(0.25, 0.96, 0.96),
        Color::srgb(0.92, 0.42, 1.0),
        Color::srgb(1.0, 0.76, 0.18),
    ]
    .into_iter()
    .enumerate()
    {
        let material = unlit_material(&mut materials, color);
        commands.spawn((
            TwinTrackSpacetime3d,
            SpacetimeAxis3d(index),
            Mesh3d(cube.clone()),
            MeshMaterial3d(material),
            Visibility::Hidden,
            Transform::default(),
            layer.clone(),
            Name::new(format!("TwinTrack 3D axis {index}")),
        ));
    }

    let legends = [
        (
            "3D SPACETIME MAP   •   M / SPECIAL: HIDE",
            Vec2::new(0.0, 142.0),
            Color::srgb(0.82, 0.92, 1.0),
        ),
        (
            "GOLD = TIME   •   GOLD BEADS = 1 s ON EACH CHARACTER'S CLOCK",
            Vec2::new(0.0, -132.0),
            Color::srgb(1.0, 0.90, 0.48),
        ),
        (
            "BLUE = PAST LIGHT CONE   •   CYAN PLANE = YOUR SIMULTANEOUS NOW",
            Vec2::new(0.0, -150.0),
            Color::srgb(0.54, 0.94, 1.0),
        ),
    ];
    for (index, (text, position, color)) in legends.into_iter().enumerate() {
        commands.spawn((
            TwinTrackSpacetime3d,
            SpacetimeLegend3d(index),
            Text2d::new(text),
            TextFont {
                font_size: if index == 0 { 18.0 } else { 12.0 },
                ..default()
            },
            TextColor(color),
            Visibility::Hidden,
            Transform::from_translation(position.extend(25.0)),
            overlay_layer.clone(),
            Name::new(format!("TwinTrack 3D legend {index}")),
        ));
    }
}

/// `Option<Res<ButtonInput<..>>>` — a headless app installs no input
/// plugin, and under Bevy 0.18 a missing `Res` is a system-parameter validation
/// failure that panics through `Main::run_main` rather than skipping the system.
/// The KEYBOARD shortcut is a convenience beside the traveler's own `special`
/// press, so its absence costs the shortcut and nothing else. See
/// `spawn_spacetime_3d` for the same note at more length.
fn toggle_spacetime_minimap(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    slots: Res<ambition_platformer2d::characters::control::SlotControls>,
    traveler: Query<
        &ambition_platformer2d::characters::control::DrivingParticipant,
        With<TravelerTwin>,
    >,
    mut special_was_pressed: Local<bool>,
    mut minimap: ResMut<SpacetimeMinimapState>,
) {
    // Presentation/UI input belongs to the participant slot, not to body-semantic
    // ActorControl. Follow whichever slot currently drives the traveler instead
    // of assuming the primary slot or manufacturing an input component on it.
    let special = traveler
        .single()
        .ok()
        .is_some_and(|driver| slots.get(driver.0).special_pressed);
    let special_edge = special && !*special_was_pressed;
    *special_was_pressed = special;
    let key_edge = keys.is_some_and(|keys| keys.just_pressed(KeyCode::KeyM));
    if special_edge || key_edge {
        minimap.visible = !minimap.visible;
    }
}

fn minimap_viewport(window: &Window) -> Option<Viewport> {
    let width = window.physical_width();
    let height = window.physical_height();
    if width < MINIMAP_MIN_PX + MINIMAP_MARGIN_PX * 2
        || height < MINIMAP_MIN_PX + MINIMAP_TOP_PX + MINIMAP_MARGIN_PX
    {
        return None;
    }
    let size = ((width as f32 * 0.34).min(height as f32 * 0.42) as u32)
        .max(MINIMAP_MIN_PX)
        .min(width.saturating_sub(MINIMAP_MARGIN_PX * 2))
        .min(height.saturating_sub(MINIMAP_TOP_PX + MINIMAP_MARGIN_PX));
    Some(Viewport {
        physical_position: UVec2::new(
            width.saturating_sub(size + MINIMAP_MARGIN_PX),
            MINIMAP_TOP_PX,
        ),
        physical_size: UVec2::splat(size),
        ..default()
    })
}

fn sync_spacetime_cameras(
    minimap: Res<SpacetimeMinimapState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cameras: ParamSet<(
        Query<&mut Camera, With<SpacetimeCamera3d>>,
        Query<&mut Camera, With<SpacetimeOverlayCamera>>,
    )>,
) {
    let viewport = windows.single().ok().and_then(minimap_viewport);
    let active = minimap.visible && viewport.is_some();
    for mut camera in &mut cameras.p0() {
        camera.is_active = active;
        camera.viewport = viewport.clone();
    }
    for mut camera in &mut cameras.p1() {
        camera.is_active = active;
        camera.viewport = viewport.clone();
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

fn selected_time(
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

fn display_window(experiment: &TwinTrackExperiment) -> f64 {
    if experiment.phase == TwinTrackPhase::Complete {
        REPLAY_WINDOW_SECONDS
    } else {
        LIVE_WINDOW_SECONDS
    }
}

fn track_samples<'a>(
    history: &'a WorldlineHistoryView2d,
    label: &str,
) -> Option<&'a std::collections::VecDeque<WorldlineSample2d>> {
    history
        .tracks
        .iter()
        .find(|(track, _)| track.as_str() == label)
        .map(|(_, samples)| samples)
}

fn sample_at_or_before<'a>(
    samples: &'a std::collections::VecDeque<WorldlineSample2d>,
    time: f64,
) -> Option<&'a WorldlineSample2d> {
    samples
        .iter()
        .rev()
        .find(|sample| sample.coordinate_time <= time)
        .or_else(|| samples.front())
}

fn graph_point(position: Vec2, age: f64) -> Vec3 {
    let room_center = Vec2::new(ROOM_WIDTH * 0.5, ROOM_HEIGHT * 0.5);
    let offset = (position - room_center) * SPACE_SCALE;
    Vec3::new(
        offset.x,
        -(age as f32) * INVARIANT_SPEED * SPACE_SCALE,
        offset.y,
    )
}

fn set_line_3d(transform: &mut Transform, start: Vec3, end: Vec3, thickness: f32) -> bool {
    let delta = end - start;
    let length = delta.length();
    if !length.is_finite() || length <= 1.0e-4 {
        return false;
    }
    transform.translation = (start + end) * 0.5;
    transform.rotation = Quat::from_rotation_arc(Vec3::X, delta / length);
    transform.scale = Vec3::new(length, thickness, thickness);
    true
}

fn sampled_worldline_points(
    samples: &std::collections::VecDeque<WorldlineSample2d>,
    now: f64,
    window: f64,
    max_points: usize,
) -> Vec<Vec3> {
    let earliest = now - window;
    let rows: Vec<_> = samples
        .iter()
        .filter(|sample| sample.coordinate_time >= earliest && sample.coordinate_time <= now)
        .collect();
    if rows.len() <= max_points {
        return rows
            .into_iter()
            .map(|sample| graph_point(sample.position, now - sample.coordinate_time))
            .collect();
    }
    let last = rows.len() - 1;
    (0..max_points)
        .map(|index| {
            let source = index * last / (max_points - 1).max(1);
            let sample = rows[source];
            graph_point(sample.position, now - sample.coordinate_time)
        })
        .collect()
}

fn update_worldlines(
    history: Res<WorldlineHistoryView2d>,
    signals: Res<RelativitySignalView2d>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut segments: Query<(&WorldlineSegment3d, &mut Transform, &mut Visibility)>,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    let active = true;
    let now = selected_time(experiment, &history, signals.coordinate_time);
    let window = display_window(experiment);

    let mut points_by_track: std::collections::BTreeMap<&'static str, Vec<Vec3>> =
        std::collections::BTreeMap::new();
    if active {
        for label in TRACK_LABELS {
            if let Some(samples) = track_samples(&history, label) {
                points_by_track.insert(
                    label,
                    sampled_worldline_points(samples, now, window, TRACK_SEGMENTS + 1),
                );
            }
        }
    }

    for (segment, mut transform, mut visibility) in &mut segments {
        let Some(points) = points_by_track.get(segment.track) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let Some((&start, &end)) = points.get(segment.slot).zip(points.get(segment.slot + 1))
        else {
            *visibility = Visibility::Hidden;
            continue;
        };
        *visibility = if set_line_3d(&mut transform, start, end, LINE_THICKNESS) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn proper_time_beads(
    samples: &std::collections::VecDeque<WorldlineSample2d>,
    now: f64,
    window: f64,
) -> Vec<Vec3> {
    let earliest = now - window;
    let rows: Vec<_> = samples
        .iter()
        .filter(|sample| sample.coordinate_time >= earliest && sample.coordinate_time <= now)
        .filter_map(|sample| sample.proper_time.map(|proper| (sample, proper)))
        .collect();
    let Some((first, first_proper)) = rows.first().copied() else {
        return Vec::new();
    };
    let mut next_second = first_proper.ceil().max(0.0);
    let mut beads = Vec::new();
    let mut previous = (first, first_proper);
    for current in rows.into_iter().skip(1) {
        while next_second <= current.1 && beads.len() < CLOCK_BEADS {
            if next_second >= previous.1 {
                let denom = (current.1 - previous.1).max(1.0e-12);
                let mix = ((next_second - previous.1) / denom).clamp(0.0, 1.0) as f32;
                let position = previous.0.position.lerp(current.0.position, mix);
                let time = previous.0.coordinate_time
                    + (current.0.coordinate_time - previous.0.coordinate_time) * f64::from(mix);
                beads.push(graph_point(position, now - time));
            }
            next_second += 1.0;
        }
        previous = current;
        if beads.len() >= CLOCK_BEADS {
            break;
        }
    }
    beads
}

fn update_clock_beads(
    history: Res<WorldlineHistoryView2d>,
    signals: Res<RelativitySignalView2d>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut beads: Query<(&ClockBead3d, &mut Transform, &mut Visibility)>,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    let active = true;
    let now = selected_time(experiment, &history, signals.coordinate_time);
    let window = display_window(experiment);
    let mut positions: std::collections::BTreeMap<&'static str, Vec<Vec3>> =
        std::collections::BTreeMap::new();
    if active {
        for label in TRACK_LABELS {
            if let Some(samples) = track_samples(&history, label) {
                positions.insert(label, proper_time_beads(samples, now, window));
            }
        }
    }
    for (bead, mut transform, mut visibility) in &mut beads {
        let Some(position) = positions
            .get(bead.track)
            .and_then(|rows| rows.get(bead.slot))
            .copied()
        else {
            *visibility = Visibility::Hidden;
            continue;
        };
        transform.translation = position;
        transform.rotation = Quat::IDENTITY;
        transform.scale = Vec3::splat(9.5);
        *visibility = Visibility::Visible;
    }
}

fn update_current_events(
    history: Res<WorldlineHistoryView2d>,
    signals: Res<RelativitySignalView2d>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut events: Query<(&CurrentEvent3d, &mut Transform, &mut Visibility)>,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    let active = true;
    let now = selected_time(experiment, &history, signals.coordinate_time);
    for (event, mut transform, mut visibility) in &mut events {
        if !active {
            *visibility = Visibility::Hidden;
            continue;
        }
        let Some(sample) =
            track_samples(&history, event.0).and_then(|rows| sample_at_or_before(rows, now))
        else {
            *visibility = Visibility::Hidden;
            continue;
        };
        transform.translation = graph_point(sample.position, now - sample.coordinate_time);
        transform.rotation = Quat::IDENTITY;
        transform.scale = Vec3::splat(if event.0 == "traveler" { 12.0 } else { 9.0 });
        *visibility = Visibility::Visible;
    }
}

fn arrival_start_position(
    history: &WorldlineHistoryView2d,
    emitter_tag: u64,
    payload: u64,
    emission_time: f64,
) -> Option<Vec2> {
    let label = if emitter_tag == crate::TRAVELER_TAG {
        "traveler"
    } else {
        let (_, actor_id, _) = crate::payload_parts(payload);
        match actor_id {
            crate::COURIER_ID => "Courier",
            crate::DRIFTER_ID => "Drifter",
            crate::SPINNER_ID => "Spinner",
            crate::DJ_ID => "DJ Blue Shift",
            crate::TAGGER_ID => "Photon Fox",
            _ => return None,
        }
    };
    let samples = track_samples(history, label)?;
    sample_at_or_before(samples, emission_time).map(|sample| sample.position)
}

fn update_signal_paths(
    history: Res<WorldlineHistoryView2d>,
    signals: Res<RelativitySignalView2d>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut segments: Query<(&SignalSegment3d, &mut Transform, &mut Visibility)>,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    let active = true;
    let now = selected_time(experiment, &history, signals.coordinate_time);
    let earliest = now - display_window(experiment);
    let mut rows: Vec<(Vec3, Vec3)> = Vec::new();
    if active {
        if experiment.phase == TwinTrackPhase::Complete {
            for arrival in signals.recent_arrivals.iter().filter(|arrival| {
                arrival.coordinate_time <= now && arrival.coordinate_time >= earliest
            }) {
                let Some(start_position) = arrival_start_position(
                    &history,
                    arrival.emitter_tag,
                    arrival.payload,
                    arrival.signal_emission_time,
                ) else {
                    continue;
                };
                rows.push((
                    graph_point(start_position, now - arrival.signal_emission_time),
                    graph_point(arrival.position, now - arrival.coordinate_time),
                ));
            }
        } else {
            for signal in &signals.active_signals {
                rows.push((
                    graph_point(signal.emission_position, signal.age),
                    graph_point(signal.position, 0.0),
                ));
            }
        }
    }
    if rows.len() > SIGNAL_SEGMENTS {
        rows.drain(0..rows.len() - SIGNAL_SEGMENTS);
    }
    for (segment, mut transform, mut visibility) in &mut segments {
        let Some((start, end)) = rows.get(segment.0).copied() else {
            *visibility = Visibility::Hidden;
            continue;
        };
        *visibility = if set_line_3d(&mut transform, start, end, 4.6) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn observer_sample(history: &WorldlineHistoryView2d, now: f64) -> Option<&WorldlineSample2d> {
    track_samples(history, "traveler").and_then(|samples| sample_at_or_before(samples, now))
}

fn update_light_cone(
    history: Res<WorldlineHistoryView2d>,
    signals: Res<RelativitySignalView2d>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut spokes: Query<(&ConeSpoke3d, &mut Transform, &mut Visibility)>,
    mut rings: Query<(&ConeRing3d, &mut Transform, &mut Visibility), Without<ConeSpoke3d>>,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    let active = true;
    let now = selected_time(experiment, &history, signals.coordinate_time);
    let Some(observer) = observer_sample(&history, now) else {
        for (_, _, mut visibility) in &mut spokes {
            *visibility = Visibility::Hidden;
        }
        for (_, _, mut visibility) in &mut rings {
            *visibility = Visibility::Hidden;
        }
        return;
    };
    let age = display_window(experiment).min(2.2);
    let apex = graph_point(observer.position, now - observer.coordinate_time);

    for (spoke, mut transform, mut visibility) in &mut spokes {
        if !active {
            *visibility = Visibility::Hidden;
            continue;
        }
        let angle = spoke.0 as f32 / CONE_SPOKES as f32 * std::f32::consts::TAU;
        let past_position =
            observer.position + Vec2::from_angle(angle) * INVARIANT_SPEED * age as f32;
        let end = graph_point(past_position, age);
        *visibility = if set_line_3d(&mut transform, apex, end, 2.2) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    for (ring, mut transform, mut visibility) in &mut rings {
        if !active {
            *visibility = Visibility::Hidden;
            continue;
        }
        let ring_age = age * (ring.ring + 1) as f64 / CONE_RINGS as f64;
        let a0 = ring.segment as f32 / CONE_RING_SEGMENTS as f32 * std::f32::consts::TAU;
        let a1 = (ring.segment + 1) as f32 / CONE_RING_SEGMENTS as f32 * std::f32::consts::TAU;
        let radius = INVARIANT_SPEED * ring_age as f32;
        let start = graph_point(observer.position + Vec2::from_angle(a0) * radius, ring_age);
        let end = graph_point(observer.position + Vec2::from_angle(a1) * radius, ring_age);
        *visibility = if set_line_3d(&mut transform, start, end, 2.0) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn update_simultaneity_planes(
    history: Res<WorldlineHistoryView2d>,
    signals: Res<RelativitySignalView2d>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut planes: Query<(&SpacetimePlane3d, &mut Transform, &mut Visibility)>,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    let active = true;
    let now = selected_time(experiment, &history, signals.coordinate_time);
    let observer = observer_sample(&history, now);
    let plane_size = Vec3::new(
        ROOM_WIDTH * SPACE_SCALE * 1.55,
        0.65,
        ROOM_HEIGHT * SPACE_SCALE * 1.55,
    );

    for (plane, mut transform, mut visibility) in &mut planes {
        if !active {
            *visibility = Visibility::Hidden;
            continue;
        }
        transform.scale = plane_size;
        match plane.0 {
            PlaneKind3d::LaboratoryNow => {
                transform.translation = Vec3::ZERO;
                transform.rotation = Quat::IDENTITY;
            }
            PlaneKind3d::ObserverNow => {
                let Some(observer) = observer else {
                    *visibility = Visibility::Hidden;
                    continue;
                };
                let observer_graph = graph_point(observer.position, now - observer.coordinate_time);
                transform.translation = Vec3::new(observer_graph.x, 0.5, observer_graph.z);
                let slope_scale = 1.0 / INVARIANT_SPEED;
                let normal = Vec3::new(
                    -observer.coordinate_velocity.x * slope_scale,
                    1.0,
                    -observer.coordinate_velocity.y * slope_scale,
                )
                .normalize_or_zero();
                transform.rotation = if normal == Vec3::ZERO {
                    Quat::IDENTITY
                } else {
                    Quat::from_rotation_arc(Vec3::Y, normal)
                };
            }
        }
        *visibility = Visibility::Visible;
    }
}

fn update_axes(
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut axes: Query<(&SpacetimeAxis3d, &mut Transform, &mut Visibility)>,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    let active = true;
    let half_x = ROOM_WIDTH * SPACE_SCALE * 0.56;
    let half_z = ROOM_HEIGHT * SPACE_SCALE * 0.56;
    let past = display_window(experiment) as f32 * INVARIANT_SPEED * SPACE_SCALE;
    for (axis, mut transform, mut visibility) in &mut axes {
        if !active {
            *visibility = Visibility::Hidden;
            continue;
        }
        let (start, end) = match axis.0 {
            0 => (
                Vec3::new(-half_x, 0.0, -half_z),
                Vec3::new(half_x, 0.0, -half_z),
            ),
            1 => (
                Vec3::new(-half_x, 0.0, -half_z),
                Vec3::new(-half_x, 0.0, half_z),
            ),
            _ => (
                Vec3::new(-half_x, -past, -half_z),
                Vec3::new(-half_x, 60.0, -half_z),
            ),
        };
        *visibility = if set_line_3d(&mut transform, start, end, 4.2) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn update_spacetime_camera(
    time: Res<Time>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut cameras: Query<&mut Transform, With<SpacetimeCamera3d>>,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    let height = display_window(experiment) as f32 * INVARIANT_SPEED * SPACE_SCALE;
    let center = Vec3::new(0.0, -height * 0.43, 0.0);
    let radius = (height * 0.86).max(520.0);
    let angle = 0.55 + time.elapsed_secs() * 0.13;
    let eye = center + Vec3::new(angle.cos() * radius, radius * 0.38, angle.sin() * radius);
    for mut transform in &mut cameras {
        *transform = Transform::from_translation(eye).looking_at(center, Vec3::Y);
    }
}

fn update_legend(
    history: Res<WorldlineHistoryView2d>,
    signals: Res<RelativitySignalView2d>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut legends: Query<(&SpacetimeLegend3d, &mut Text2d, &mut Visibility)>,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    let active = true;
    let now = selected_time(experiment, &history, signals.coordinate_time);
    let observer = observer_sample(&history, now);
    for (legend, mut text, mut visibility) in &mut legends {
        *visibility = if active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if legend.0 == 0 {
            let speed = observer.map_or(0.0, |sample| sample.coordinate_velocity.length());
            text.0 = format!(
                "3D SPACETIME MAP   •   SPEED {:.0}% c   •   M / SPECIAL: HIDE",
                (100.0 * speed / INVARIANT_SPEED).clamp(0.0, 99.9),
            );
        }
    }
}

fn cleanup_spacetime_3d_when_inactive(
    mut commands: Commands,
    roots: Query<&ambition_platformer2d::runtime::demo_fixture::ActiveRoomMetadata>,
    visuals: Query<Entity, With<TwinTrackSpacetime3d>>,
    mut minimap: ResMut<SpacetimeMinimapState>,
) {
    if twintrack_is_active(&roots) {
        return;
    }
    minimap.visible = true;
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
}
