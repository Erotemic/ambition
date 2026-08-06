//! Visible TwinTrack observatory: an observer-local sky plus spacetime strip.
//!
//! The main viewport remains the laboratory coordinate chart. This camera
//! renders only optical read-model proxies on a private layer, so no visual
//! system can mutate simulation state or masquerade as a second world.

use std::collections::{BTreeMap, BTreeSet};

use ambition_platformer2d::platformer::camera_layers::MainCamera;
use ambition_platformer2d::relativity::{observe_photon_direction, InvariantSpeed};
use ambition_platformer2d::relativity2d::{
    RelativisticOpticalView2d, RelativisticTargetingView2d, RelativitySignalView2d,
    WorldlineHistoryView2d,
};
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, OrthographicProjection, Projection, ScalingMode, Viewport};
use bevy::prelude::*;

use crate::{
    LaboratoryTwin, TwinTrackExperiment, TwinTrackPhase, TwinTrackViewMode, TWINTRACK_EXPERIENCE,
};

const OPTICAL_LAYER: usize = 28;
const PANEL_WIDTH: f32 = 720.0;
const PANEL_HEIGHT: f32 = 720.0;
const SKY_CENTER: Vec2 = Vec2::new(0.0, 115.0);
const SKY_RADIUS: f32 = 250.0;
const DIAGRAM_MIN: Vec2 = Vec2::new(-315.0, -315.0);
const DIAGRAM_MAX: Vec2 = Vec2::new(315.0, -90.0);
const TRACE_WINDOW_SECONDS: f64 = 8.0;
const STAR_COUNT: usize = 224;
const TRACE_DOT_COUNT: usize = 180;
const SIGNAL_LINE_COUNT: usize = 12;

#[derive(Component)]
struct ObservatoryVisual;

/// The observer viewport's camera.
///
/// ⭐ **public because a CAPTURE has to find it.** `ambition_render::capture`
/// adopts the main and HUD cameras — every game has those — and knows nothing
/// about a third camera one demo added. A picture of TwinTrack that omitted the
/// observatory would be a picture of the half the observatory exists to
/// complement, so `capture_twintrack` points this one at the same texture.
#[derive(Component)]
pub struct ObservatoryCamera;

#[derive(Component)]
struct ObservatoryStar {
    rest_source_direction: Vec2,
    radial_fraction: f32,
    base_size: f32,
    rest_intensity: f32,
}

#[derive(Component)]
struct ObservatorySourceProxy {
    label: String,
}

#[derive(Component)]
struct ObservatoryTraceDot(usize);

#[derive(Component)]
struct ObservatorySignalLine(usize);

#[derive(Component)]
struct ObservatoryLightConeLine(i8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ObservatoryVectorKind {
    Apparent,
    CoordinateNow,
    Intercept,
    Aim,
}

#[derive(Component)]
struct ObservatoryVectorLine(ObservatoryVectorKind);

#[derive(Component)]
struct ObservatoryReticle {
    kind: ObservatoryVectorKind,
    vertical: bool,
}

pub(crate) fn install(app: &mut App) {
    app.add_systems(
        Update,
        (
            spawn_observatory_rig,
            sync_observatory_viewport,
            sync_laboratory_viewport,
            update_observatory_stars,
            update_observatory_targeting,
            sync_observatory_source_proxies,
            update_observatory_trace,
            update_observatory_signal_lines,
            update_observatory_light_cone,
            cleanup_observatory_when_inactive,
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

fn spawn_observatory_rig(
    mut commands: Commands,
    roots: Query<&ambition_platformer2d::runtime::demo_fixture::ActiveRoomMetadata>,
    cameras: Query<(), With<ObservatoryCamera>>,
) {
    if !twintrack_is_active(&roots) || !cameras.is_empty() {
        return;
    }

    let layer = RenderLayers::layer(OPTICAL_LAYER);
    commands.spawn((
        ObservatoryVisual,
        ObservatoryCamera,
        Camera2d,
        // ⛔ **`None`, and the panel paints its own backdrop below.** A `Custom`
        // clear color here erased the ENTIRE frame — not just this camera's
        // viewport — because a clear is a load op on the whole attachment and
        // the scissor rect does not bound it. This camera has `order: 4`, so it
        // ran after the laboratory chart and wiped it: the first capture came
        // back as an observatory floating on a black rectangle where the
        // experiment used to be, in the windowed build too.
        Camera {
            order: 4,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        layer.clone(),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: PANEL_WIDTH,
                height: PANEL_HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
        // ⛔ **z = 0, not Bevy's default 1000.** `default_2d()` spans
        // `near = -1000 .. far = 1000` RELATIVE to the camera, so a camera parked
        // at `z = 1000` sees `0 ..= 2000` — and every backdrop this file draws
        // behind its content sits at a NEGATIVE z. The sky disk and the
        // spacetime-diagram backing were culled from the first frame the
        // observatory ever drew; only the stars and vectors (all `z >= 0`) came
        // through, which is why the panel read as a black void with lines on it.
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new("TwinTrack observer optical camera"),
    ));

    // The panel's own backdrop, which is what the camera's clear used to be.
    // Opaque and exactly the projection's size, so the laboratory chart behind
    // it is hidden where the panel sits and untouched everywhere else.
    commands.spawn((
        ObservatoryVisual,
        Sprite::from_color(
            Color::srgb(0.008, 0.012, 0.028),
            Vec2::new(PANEL_WIDTH, PANEL_HEIGHT),
        ),
        Transform::from_xyz(0.0, 0.0, -20.0),
        layer.clone(),
        Name::new("TwinTrack observatory panel backdrop"),
    ));

    commands.spawn((
        ObservatoryVisual,
        Sprite::from_color(
            Color::srgba(0.02, 0.035, 0.075, 0.92),
            Vec2::splat(SKY_RADIUS * 2.08),
        ),
        Transform::from_translation(SKY_CENTER.extend(-8.0)),
        layer.clone(),
        Name::new("TwinTrack optical sky disk"),
    ));
    commands.spawn((
        ObservatoryVisual,
        Sprite::from_color(
            Color::srgba(0.025, 0.045, 0.085, 0.96),
            Vec2::new(DIAGRAM_MAX.x - DIAGRAM_MIN.x, DIAGRAM_MAX.y - DIAGRAM_MIN.y),
        ),
        Transform::from_translation(((DIAGRAM_MIN + DIAGRAM_MAX) * 0.5).extend(-8.0)),
        layer.clone(),
        Name::new("TwinTrack spacetime diagram backing"),
    ));

    for (text, position, size) in [
        ("TRAVELER PAST-LIGHT-CONE VIEW", Vec2::new(0.0, 340.0), 19.0),
        (
            "aberration · Doppler · retarded events",
            Vec2::new(0.0, 315.0),
            13.0,
        ),
        (
            "SPACETIME STRIP   x →     past ↓",
            Vec2::new(0.0, -72.0),
            13.0,
        ),
    ] {
        commands.spawn((
            ObservatoryVisual,
            Text2d::new(text),
            TextFont {
                font_size: size,
                ..default()
            },
            TextColor(Color::srgb(0.72, 0.88, 1.0)),
            Transform::from_translation(position.extend(8.0)),
            layer.clone(),
            Name::new(format!("TwinTrack observatory label: {text}")),
        ));
    }

    commands.spawn((
        ObservatoryVisual,
        Text2d::new("RED apparent   YELLOW coordinate-now   GREEN intercept   CYAN aim   SPECIAL cycles views"),
        TextFont {
            font_size: 11.0,
            ..default()
        },
        TextColor(Color::srgb(0.80, 0.88, 1.0)),
        Transform::from_translation(Vec2::new(0.0, 292.0).extend(8.0)),
        layer.clone(),
        Name::new("TwinTrack observatory targeting legend"),
    ));

    for kind in [
        ObservatoryVectorKind::Apparent,
        ObservatoryVectorKind::CoordinateNow,
        ObservatoryVectorKind::Intercept,
        ObservatoryVectorKind::Aim,
    ] {
        commands.spawn((
            ObservatoryVisual,
            ObservatoryVectorLine(kind),
            Sprite::from_color(vector_color(kind), Vec2::ONE),
            Visibility::Hidden,
            Transform::from_xyz(0.0, 0.0, 6.0),
            layer.clone(),
            Name::new(format!("TwinTrack observatory vector {kind:?}")),
        ));
        for vertical in [false, true] {
            commands.spawn((
                ObservatoryVisual,
                ObservatoryReticle { kind, vertical },
                Sprite::from_color(vector_color(kind), Vec2::ONE),
                Visibility::Hidden,
                Transform::from_xyz(0.0, 0.0, 7.0),
                layer.clone(),
                Name::new(format!(
                    "TwinTrack observatory reticle {kind:?} {}",
                    if vertical { "vertical" } else { "horizontal" }
                )),
            ));
        }
    }

    for index in 0..STAR_COUNT {
        let hash = splitmix64(index as u64 + 0x8b5a_2d19_74c3_61e7);
        let angle = unit_from_u64(hash) * core::f32::consts::TAU;
        let radial_fraction = 0.18 + 0.80 * unit_from_u64(hash.rotate_left(21)).sqrt();
        let rest_intensity = 0.35 + 0.65 * unit_from_u64(hash.rotate_left(37));
        let base_size = 1.3 + 2.8 * unit_from_u64(hash.rotate_left(49));
        commands.spawn((
            ObservatoryVisual,
            ObservatoryStar {
                rest_source_direction: Vec2::from_angle(angle),
                radial_fraction,
                base_size,
                rest_intensity,
            },
            Sprite::from_color(Color::WHITE, Vec2::splat(base_size)),
            Transform::from_translation(SKY_CENTER.extend(0.0)),
            layer.clone(),
            Name::new(format!("TwinTrack optical star {index}")),
        ));
    }

    for index in 0..TRACE_DOT_COUNT {
        commands.spawn((
            ObservatoryVisual,
            ObservatoryTraceDot(index),
            Sprite::from_color(Color::WHITE, Vec2::splat(3.0)),
            Visibility::Hidden,
            Transform::from_xyz(0.0, 0.0, 2.0),
            layer.clone(),
            Name::new(format!("TwinTrack worldline dot {index}")),
        ));
    }
    for index in 0..SIGNAL_LINE_COUNT {
        commands.spawn((
            ObservatoryVisual,
            ObservatorySignalLine(index),
            Sprite::from_color(Color::srgb(0.18, 0.88, 1.0), Vec2::ONE),
            Visibility::Hidden,
            Transform::from_xyz(0.0, 0.0, 3.0),
            layer.clone(),
            Name::new(format!("TwinTrack null-signal line {index}")),
        ));
    }
    for sign in [-1_i8, 1_i8] {
        commands.spawn((
            ObservatoryVisual,
            ObservatoryLightConeLine(sign),
            Sprite::from_color(Color::srgba(0.35, 0.72, 1.0, 0.48), Vec2::ONE),
            Transform::from_xyz(0.0, 0.0, 1.0),
            layer.clone(),
            Name::new(format!("TwinTrack past light cone {sign}")),
        ));
    }
}

/// The full physical size of a camera's render target, in pixels.
///
/// ⛔ **not the primary WINDOW's resolution.** Both split systems below asked the
/// window how big it was, so the whole two-viewport layout silently collapsed to
/// nothing whenever the game rendered to a texture instead — which is what an
/// offscreen capture is, and therefore what `capture_twintrack` would have
/// photographed. The camera already knows the size of whatever it draws into.
///
/// ⚠ `physical_target_size`, NOT `physical_viewport_size`: the second answers
/// the viewport these very systems are about to set, so a layout derived from it
/// would shrink toward zero one frame at a time.
fn target_size(camera: &Camera) -> Option<(u32, u32)> {
    let size = camera.physical_target_size()?;
    (size.x >= 2 && size.y >= 2).then_some((size.x, size.y))
}

fn sync_observatory_viewport(
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut cameras: Query<&mut Camera, With<ObservatoryCamera>>,
) {
    let (Ok(experiment), Ok(mut camera)) = (experiment.single(), cameras.single_mut()) else {
        return;
    };
    let Some((width, height)) = target_size(&camera) else {
        return;
    };
    let margin_x = (width as f32 * 0.015).round() as u32;
    let margin_y = (height as f32 * 0.035).round() as u32;
    let desired = match experiment.view_mode {
        TwinTrackViewMode::Dual => {
            let panel_size = ((width as f32 * 0.39).round() as u32)
                .min(height.saturating_sub(margin_y.saturating_mul(2)))
                .max(2);
            Viewport {
                physical_position: UVec2::new(
                    width.saturating_sub(panel_size + margin_x),
                    height.saturating_sub(panel_size) / 2,
                ),
                physical_size: UVec2::splat(panel_size),
                ..default()
            }
        }
        TwinTrackViewMode::OpticalFocus => {
            let panel_size = ((width as f32 * 0.74).round() as u32)
                .min(height.saturating_sub(margin_y.saturating_mul(2)))
                .max(2);
            Viewport {
                physical_position: UVec2::new(
                    width.saturating_sub(panel_size + margin_x),
                    height.saturating_sub(panel_size) / 2,
                ),
                physical_size: UVec2::splat(panel_size),
                ..default()
            }
        }
        TwinTrackViewMode::LaboratoryFocus => {
            let panel_size = ((width as f32 * 0.27).round() as u32)
                .min((height as f32 * 0.42).round() as u32)
                .max(2);
            Viewport {
                physical_position: UVec2::new(
                    width.saturating_sub(panel_size + margin_x),
                    margin_y,
                ),
                physical_size: UVec2::splat(panel_size),
                ..default()
            }
        }
    };
    if !viewport_matches(camera.viewport.as_ref(), &desired) {
        camera.viewport = Some(desired);
    }
}

fn sync_laboratory_viewport(
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut cameras: Query<&mut Camera, (With<MainCamera>, Without<ObservatoryCamera>)>,
) {
    let Ok(experiment) = experiment.single() else {
        return;
    };
    for mut camera in &mut cameras {
        // ⚠ per camera, because the size is the CAMERA's target rather than one
        // window every camera was assumed to share.
        let Some((width, height)) = target_size(&camera) else {
            continue;
        };
        let desired = if experiment.view_mode == TwinTrackViewMode::OpticalFocus {
            let margin = (width as f32 * 0.015).round() as u32;
            let size = ((width as f32 * 0.215).round() as u32)
                .min((height as f32 * 0.34).round() as u32)
                .max(2);
            Some(Viewport {
                physical_position: UVec2::new(margin, height.saturating_sub(size + margin)),
                physical_size: UVec2::splat(size),
                ..default()
            })
        } else {
            None
        };
        match desired.as_ref() {
            Some(desired) if !viewport_matches(camera.viewport.as_ref(), desired) => {
                camera.viewport = Some(desired.clone());
            }
            None if camera.viewport.is_some() => camera.viewport = None,
            _ => {}
        }
    }
}

fn viewport_matches(current: Option<&Viewport>, desired: &Viewport) -> bool {
    current.is_some_and(|current| {
        current.physical_position == desired.physical_position
            && current.physical_size == desired.physical_size
    })
}

fn update_observatory_stars(
    optical: Res<RelativisticOpticalView2d>,
    mut stars: Query<
        (&ObservatoryStar, &mut Sprite, &mut Transform),
        Without<ObservatorySourceProxy>,
    >,
) {
    let Some(observer) = optical.observer.as_ref() else {
        return;
    };
    let Ok(c) = InvariantSpeed::new(observer.invariant_speed) else {
        return;
    };
    let observer_velocity = [
        f64::from(observer.coordinate_velocity.x),
        f64::from(observer.coordinate_velocity.y),
        0.0,
    ];
    for (star, mut sprite, mut transform) in &mut stars {
        let source_direction = screen_to_physics_direction(star.rest_source_direction);
        let photon_direction = -source_direction;
        let Some(observation) = observe_photon_direction(
            [
                f64::from(photon_direction.x),
                f64::from(photon_direction.y),
                0.0,
            ],
            observer_velocity,
            c,
        ) else {
            continue;
        };
        let apparent = physics_to_screen_direction(Vec2::new(
            observation.apparent_source_direction[0] as f32,
            observation.apparent_source_direction[1] as f32,
        ))
        .normalize_or_zero();
        transform.translation =
            (SKY_CENTER + apparent * SKY_RADIUS * star.radial_fraction).extend(0.0);
        let luminance = (star.rest_intensity * observation.beaming_factor as f32)
            .sqrt()
            .clamp(0.18, 3.5);
        let size = star.base_size * luminance.clamp(0.65, 2.8);
        sprite.custom_size = Some(Vec2::splat(size));
        sprite.color = spectral_color(observation.doppler_factor, luminance);
    }
}

fn update_observatory_targeting(
    optical: Res<RelativisticOpticalView2d>,
    targeting: Res<RelativisticTargetingView2d>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    // ⛔ **the `Without` halves are load-bearing, not decoration.** Two `&mut`
    // queries over the same components are a PARAMETER CONFLICT: Bevy does not
    // infer disjointness from the fact that one asks for `ObservatoryVectorLine`
    // and the other for `ObservatoryReticle`, and it panics at run time rather
    // than failing to compile — B0001, from inside the shipped host, in every
    // composition that runs this file. It cost four `ambition_app` tests.
    mut lines: Query<
        (
            &ObservatoryVectorLine,
            &mut Sprite,
            &mut Transform,
            &mut Visibility,
        ),
        Without<ObservatoryReticle>,
    >,
    mut reticles: Query<
        (
            &ObservatoryReticle,
            &mut Sprite,
            &mut Transform,
            &mut Visibility,
        ),
        Without<ObservatoryVectorLine>,
    >,
) {
    let (Some(observer), Some(target), Ok(experiment)) = (
        optical.observer.as_ref(),
        targeting
            .targets
            .iter()
            .find(|target| target.label == "chase_beacon"),
        experiment.single(),
    ) else {
        for (_, _, _, mut visibility) in &mut lines {
            *visibility = Visibility::Hidden;
        }
        for (_, _, _, mut visibility) in &mut reticles {
            *visibility = Visibility::Hidden;
        }
        return;
    };
    let Ok(c) = InvariantSpeed::new(observer.invariant_speed) else {
        return;
    };
    let coordinate_now =
        (target.coordinate_position - targeting.observer_position).normalize_or_zero();
    let coordinate_now_local = if coordinate_now == Vec2::ZERO {
        None
    } else {
        observe_photon_direction(
            [
                -f64::from(coordinate_now.x),
                -f64::from(coordinate_now.y),
                0.0,
            ],
            [
                f64::from(observer.coordinate_velocity.x),
                f64::from(observer.coordinate_velocity.y),
                0.0,
            ],
            c,
        )
        .map(|observation| {
            Vec2::new(
                observation.apparent_source_direction[0] as f32,
                observation.apparent_source_direction[1] as f32,
            )
            .normalize_or_zero()
        })
    };

    let direction_for = |kind: ObservatoryVectorKind| -> Option<Vec2> {
        (match kind {
            ObservatoryVectorKind::Apparent => target.apparent_source_direction,
            ObservatoryVectorKind::CoordinateNow => coordinate_now_local,
            ObservatoryVectorKind::Intercept => Some(target.observer_local_emission_direction),
            ObservatoryVectorKind::Aim => (experiment.phase == TwinTrackPhase::Pursuit)
                .then(|| experiment.aim_direction.normalize_or_zero()),
        })
        .map(physics_to_screen_direction)
        .filter(|direction| *direction != Vec2::ZERO)
    };
    let radius_for = |kind: ObservatoryVectorKind| match kind {
        ObservatoryVectorKind::Apparent => 165.0,
        ObservatoryVectorKind::CoordinateNow => 190.0,
        ObservatoryVectorKind::Intercept => 218.0,
        ObservatoryVectorKind::Aim => 244.0,
    };

    for (line, mut sprite, mut transform, mut visibility) in &mut lines {
        let Some(direction) = direction_for(line.0) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let end = SKY_CENTER + direction * radius_for(line.0);
        set_line_sprite(&mut sprite, &mut transform, SKY_CENTER, end, 1.7);
        sprite.color = vector_color(line.0);
        *visibility = Visibility::Visible;
    }
    for (reticle, mut sprite, mut transform, mut visibility) in &mut reticles {
        let Some(direction) = direction_for(reticle.kind) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let center = SKY_CENTER + direction * radius_for(reticle.kind);
        sprite.custom_size = Some(if reticle.vertical {
            Vec2::new(2.2, 18.0)
        } else {
            Vec2::new(18.0, 2.2)
        });
        sprite.color = vector_color(reticle.kind);
        transform.translation = center.extend(transform.translation.z);
        transform.rotation = Quat::IDENTITY;
        *visibility = Visibility::Visible;
    }
}

fn sync_observatory_source_proxies(
    mut commands: Commands,
    optical: Res<RelativisticOpticalView2d>,
    mut proxies: Query<(
        Entity,
        &ObservatorySourceProxy,
        &mut Sprite,
        &mut Transform,
        &mut Visibility,
    )>,
) {
    let mut existing = BTreeMap::new();
    for (entity, proxy, _, _, _) in &proxies {
        existing.insert(proxy.label.clone(), entity);
    }
    let mut live = BTreeSet::new();
    for source in &optical.sources {
        live.insert(source.label.clone());
        let radial =
            72.0 + (source.apparent_range.max(0.0).ln_1p() as f32 / 8.0).clamp(0.0, 1.0) * 155.0;
        let position = SKY_CENTER
            + physics_to_screen_direction(source.apparent_source_direction).normalize_or_zero()
                * radial;
        let size = (source.apparent_radius
            * (source.beaming_factor as f32).sqrt().clamp(0.65, 2.7))
        .clamp(7.0, 34.0);
        let color = spectral_color(source.doppler_factor, source.rest_intensity.max(0.4));
        if let Some(entity) = existing.get(&source.label).copied() {
            if let Ok((_, _, mut sprite, mut transform, mut visibility)) = proxies.get_mut(entity) {
                sprite.custom_size = Some(Vec2::splat(size));
                sprite.color = color;
                transform.translation = position.extend(5.0);
                *visibility = Visibility::Visible;
            }
        } else {
            commands.spawn((
                ObservatoryVisual,
                ObservatorySourceProxy {
                    label: source.label.clone(),
                },
                Sprite::from_color(color, Vec2::splat(size)),
                Transform::from_translation(position.extend(5.0)),
                RenderLayers::layer(OPTICAL_LAYER),
                Name::new(format!("TwinTrack retarded image: {}", source.label)),
            ));
        }
    }
    for (_, proxy, _, _, mut visibility) in &mut proxies {
        if !live.contains(&proxy.label) {
            *visibility = Visibility::Hidden;
        }
    }
}

fn update_observatory_trace(
    optical: Res<RelativisticOpticalView2d>,
    worldlines: Res<WorldlineHistoryView2d>,
    mut dots: Query<(
        &ObservatoryTraceDot,
        &mut Sprite,
        &mut Transform,
        &mut Visibility,
    )>,
) {
    let Some(observer) = optical.observer.as_ref() else {
        return;
    };
    let mut points = Vec::new();
    // ⚠ **these are TRACK IDS, not captions.** The history is keyed by identity
    // since 2026-08-06, so an observatory that drew by label would have gone
    // blank the first time somebody renamed one on screen.
    for (track, samples) in &worldlines.tracks {
        if !matches!(track.as_str(), "traveler" | "laboratory" | "chase_beacon") {
            continue;
        }
        let color = match track.as_str() {
            "traveler" => Color::srgb(0.22, 1.0, 0.46),
            "laboratory" => Color::srgb(1.0, 0.80, 0.16),
            _ => Color::srgb(1.0, 0.30, 0.72),
        };
        for sample in samples.iter().rev().step_by(3) {
            let age = observer.coordinate_time - sample.coordinate_time;
            if !(0.0..=TRACE_WINDOW_SECONDS).contains(&age) {
                continue;
            }
            points.push((diagram_position(sample.position.x, age), color));
        }
    }
    for (dot, mut sprite, mut transform, mut visibility) in &mut dots {
        if let Some((position, color)) = points.get(dot.0).copied() {
            transform.translation = position.extend(2.0);
            sprite.color = color;
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn update_observatory_signal_lines(
    signals: Res<RelativitySignalView2d>,
    mut lines: Query<(
        &ObservatorySignalLine,
        &mut Sprite,
        &mut Transform,
        &mut Visibility,
    )>,
) {
    for (line, mut sprite, mut transform, mut visibility) in &mut lines {
        let Some(signal) = signals.active_signals.get(line.0) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let start_age = signal.age;
        let end_age = 0.0;
        let start_x = signal.emission_position.x;
        let end_x = signal.position.x;
        let start = diagram_position(start_x, start_age);
        let end = diagram_position(end_x, end_age);
        set_line_sprite(&mut sprite, &mut transform, start, end, 2.6);
        sprite.color = spectral_color(signal.coordinate_frequency / 100.0, 1.1);
        *visibility = Visibility::Visible;
    }
}

fn update_observatory_light_cone(
    optical: Res<RelativisticOpticalView2d>,
    mut lines: Query<(&ObservatoryLightConeLine, &mut Sprite, &mut Transform)>,
) {
    let Some(observer) = optical.observer.as_ref() else {
        return;
    };
    for (line, mut sprite, mut transform) in &mut lines {
        let start = diagram_position(observer.position.x, 0.0);
        let past_x = observer.position.x
            + f32::from(line.0) * observer.invariant_speed as f32 * TRACE_WINDOW_SECONDS as f32;
        let end = diagram_position(past_x, TRACE_WINDOW_SECONDS);
        set_line_sprite(&mut sprite, &mut transform, start, end, 1.4);
    }
}

fn cleanup_observatory_when_inactive(
    mut commands: Commands,
    roots: Query<&ambition_platformer2d::runtime::demo_fixture::ActiveRoomMetadata>,
    visuals: Query<Entity, With<ObservatoryVisual>>,
    mut main_cameras: Query<&mut Camera, (With<MainCamera>, Without<ObservatoryCamera>)>,
) {
    if twintrack_is_active(&roots) {
        return;
    }
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    for mut camera in &mut main_cameras {
        camera.viewport = None;
    }
}

/// One `(world x, age)` pair placed inside the spacetime strip.
///
/// ⛔ **the x fraction used to clamp to `-0.2 ..= 1.2`**, so every line this
/// function places was allowed to run a fifth of the strip's width past its own
/// backing rectangle on each side — the past-light-cone rays crossed the panel
/// edge and kept going. The age already clamped to `0 ..= 1`, so the overflow
/// was horizontal only, which is why it read as a drawing bug rather than as a
/// window. A bounded strip that draws outside its bounds is not bounded.
fn diagram_position(world_x: f32, age: f64) -> Vec2 {
    let x = DIAGRAM_MIN.x + (world_x / 1_680.0).clamp(0.0, 1.0) * (DIAGRAM_MAX.x - DIAGRAM_MIN.x);
    let y = DIAGRAM_MAX.y
        - (age / TRACE_WINDOW_SECONDS).clamp(0.0, 1.0) as f32 * (DIAGRAM_MAX.y - DIAGRAM_MIN.y);
    Vec2::new(x, y)
}

fn set_line_sprite(
    sprite: &mut Sprite,
    transform: &mut Transform,
    start: Vec2,
    end: Vec2,
    thickness: f32,
) {
    let delta = end - start;
    sprite.custom_size = Some(Vec2::new(delta.length().max(0.1), thickness));
    transform.translation = ((start + end) * 0.5).extend(transform.translation.z);
    transform.rotation = Quat::from_rotation_z(delta.y.atan2(delta.x));
}

fn physics_to_screen_direction(direction: Vec2) -> Vec2 {
    Vec2::new(direction.x, -direction.y)
}

fn screen_to_physics_direction(direction: Vec2) -> Vec2 {
    Vec2::new(direction.x, -direction.y)
}

fn vector_color(kind: ObservatoryVectorKind) -> Color {
    match kind {
        ObservatoryVectorKind::Apparent => Color::srgb(1.0, 0.24, 0.30),
        ObservatoryVectorKind::CoordinateNow => Color::srgb(1.0, 0.82, 0.20),
        ObservatoryVectorKind::Intercept => Color::srgb(0.25, 1.0, 0.45),
        ObservatoryVectorKind::Aim => Color::srgb(0.18, 0.80, 1.0),
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
