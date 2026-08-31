//! Side-by-side observer panes resolved in separate reference frames.
//!
//! Both panes read `TwinTrackDualObserverView`; neither owns or mutates
//! simulation state. World-space presentation is duplicated per pane on private
//! render layers so each camera can resolve the same events independently.
//!
//! These demo-owned cameras intentionally bypass `LocalView`: production still
//! assumes one local view and has no per-view rectangle policy for split-screen
//! composition.

use bevy::camera::visibility::RenderLayers;
use bevy::camera::Viewport;
use bevy::camera::{ClearColorConfig, OrthographicProjection, Projection, ScalingMode};
use bevy::prelude::*;

use crate::dual_observer::{
    beacon_midpoint, observer_frame_offset, EventOrdering, ObserverOrderingReport, TwinTrackBeacon,
    TwinTrackDualObserverView, BEACON_HALF_SEPARATION,
};
use crate::light_pulse::{
    pulse_frame_sample, PulseObserverReport, PulseRay, TwinTrackLightPulseView,
    PULSE_REST_FREQUENCY_THZ,
};
use crate::{
    LaboratoryTwin, TwinTrackExperiment, TwinTrackViewMode, INVARIANT_SPEED, TWINTRACK_EXPERIENCE,
};

const SPLIT_LAB_LAYER: usize = 26;
const SPLIT_TRAVELER_LAYER: usize = 27;

/// Logical pane size. The ratio tracks a half-width 16:9 window, so a pane's
/// square instrument stays square there.
const PANE_WIDTH: f32 = 720.0;
const PANE_HEIGHT: f32 = 810.0;

/// World units to pane units. Sized so the whole beacon axis uses a little
/// under half the pane, leaving room for the observer to wander off it.
const WORLD_TO_PANE: f32 = 0.30;

/// Below this a pane rectangle is too small to read, so the exhibit declines to
/// draw rather than showing an illegible half.
const MINIMUM_TARGET_PX: UVec2 = UVec2::new(320, 240);

const BEACON_ROW_Y: f32 = 210.0;
const STRIP_HALF_WIDTH: f32 = 250.0;
const SEEN_STRIP_Y: f32 = -20.0;
const FRAME_STRIP_Y: f32 = -210.0;

/// Light crossing time of the beacon axis: the natural full scale for the
/// "light reached me" row, whose split can never exceed it.
const AXIS_LIGHT_CROSSING_SECONDS: f64 =
    2.0 * BEACON_HALF_SEPARATION as f64 / INVARIANT_SPEED as f64;
/// The "in my frame" split is `gamma * beta` times the crossing time, and
/// `gamma * beta` reaches about two at the demo's terminal speed. Ticks stay on
/// the axis until then and clamp visibly past it rather than sliding off.
const FRAME_STRIP_FULL_SCALE_SECONDS: f64 = 2.0 * AXIS_LIGHT_CROSSING_SECONDS;

/// The light-pulse instrument's own furniture, below the ordering strips.
const COMPASS_CENTER: Vec2 = Vec2::new(-238.0, -332.0);
const COMPASS_RADIUS: f32 = 54.0;
const PULSE_READOUT_CENTER: Vec2 = Vec2::new(112.0, -332.0);

/// Half-extent of the region a pulse front may be drawn in. A front that has
/// left the drawn map is hidden rather than pinned to the edge, because a
/// stopped dot would read as a pulse that stopped.
const FRONT_VISIBLE_HALF_EXTENT: Vec2 = Vec2::new(PANE_WIDTH * 0.5 - 14.0, 128.0);

const ALPHA_COLOR: Color = Color::srgb(0.34, 0.76, 1.0);
const OMEGA_COLOR: Color = Color::srgb(1.0, 0.70, 0.24);
const NEUTRAL_TEXT: Color = Color::srgb(0.92, 0.96, 1.0);
const DISAGREEMENT_TEXT: Color = Color::srgb(1.0, 0.86, 0.35);

/// Which observer a pane is drawn for.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitObserverPane {
    /// The laboratory twin: at rest, equidistant from both beacons.
    Laboratory,
    /// The controlled traveler.
    Traveler,
}

impl SplitObserverPane {
    const ALL: [Self; 2] = [Self::Laboratory, Self::Traveler];

    fn layer(self) -> usize {
        match self {
            Self::Laboratory => SPLIT_LAB_LAYER,
            Self::Traveler => SPLIT_TRAVELER_LAYER,
        }
    }

    /// Above the optical and spacetime cameras (4, 5) so the panes own the
    /// window while they are up, and below the front HUD camera (9) so the
    /// objective and teacher readouts still sit on top of them.
    fn camera_order(self) -> isize {
        match self {
            Self::Traveler => 6,
            Self::Laboratory => 7,
        }
    }

    /// Which half of the window this pane draws into.
    ///
    /// participant order, not exhibit order. The traveler is seat zero and
    /// takes the left pane, because that is the pane its gameplay camera is
    /// already drawing into (`participants::place_the_two_views`) — an
    /// instrument that reported the traveler's numbers over the laboratory
    /// twin's picture would be worse than no instrument.
    fn column(self) -> u32 {
        match self {
            Self::Traveler => 0,
            Self::Laboratory => 1,
        }
    }

    fn clear_color(self) -> Color {
        match self {
            Self::Laboratory => Color::srgb(0.020, 0.028, 0.052),
            Self::Traveler => Color::srgb(0.042, 0.024, 0.036),
        }
    }

    fn side_caption(self) -> &'static str {
        match self {
            Self::Traveler => "LEFT",
            Self::Laboratory => "RIGHT",
        }
    }

    fn report<'view>(
        self,
        view: &'view TwinTrackDualObserverView,
    ) -> Option<&'view ObserverOrderingReport> {
        match self {
            Self::Laboratory => view.laboratory.as_ref(),
            Self::Traveler => view.traveler.as_ref(),
        }
    }

    fn pulse_report<'view>(
        self,
        view: &'view TwinTrackLightPulseView,
    ) -> Option<&'view PulseObserverReport> {
        match self {
            Self::Laboratory => view.laboratory.as_ref(),
            Self::Traveler => view.traveler.as_ref(),
        }
    }
}

/// The camera that draws one observer pane.
#[derive(Component, Clone, Copy, Debug)]
pub struct SplitObserverCamera(pub SplitObserverPane);

/// Which ordering question a strip answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OrderingRow {
    /// Which flash's light reached this observer first — a light-delay fact.
    LightReachedMe,
    /// Which flash happened first in this observer's own frame — the
    /// relativistic fact.
    InMyFrame,
}

impl OrderingRow {
    const ALL: [Self; 2] = [Self::LightReachedMe, Self::InMyFrame];

    fn axis_y(self) -> f32 {
        match self {
            Self::LightReachedMe => SEEN_STRIP_Y,
            Self::InMyFrame => FRAME_STRIP_Y,
        }
    }

    fn caption(self) -> &'static str {
        match self {
            Self::LightReachedMe => "LIGHT REACHED ME IN THIS ORDER",
            Self::InMyFrame => "IN MY FRAME THEY HAPPENED IN THIS ORDER",
        }
    }

    fn full_scale_seconds(self) -> f64 {
        match self {
            Self::LightReachedMe => AXIS_LIGHT_CROSSING_SECONDS,
            Self::InMyFrame => FRAME_STRIP_FULL_SCALE_SECONDS,
        }
    }

    /// The two times this row places on its axis, alpha first.
    fn times(self, report: &ObserverOrderingReport) -> (f64, f64) {
        match self {
            Self::LightReachedMe => (
                report.alpha.compared_arrival_time,
                report.omega.compared_arrival_time,
            ),
            Self::InMyFrame => (
                report.alpha.compared_frame_time,
                report.omega.compared_frame_time,
            ),
        }
    }

    fn ordering(self, report: &ObserverOrderingReport) -> EventOrdering {
        match self {
            Self::LightReachedMe => report.seen_order,
            Self::InMyFrame => report.frame_order,
        }
    }
}

/// What one drawn thing in a pane is.
///
/// one enum instead of a dozen marker components, deliberately. Every
/// element below is repositioned from the same read model in the same frame, so
/// a marker per element would need a hand-kept `Without<..>` matrix on each of a
/// dozen `&mut Transform` queries — a list that goes stale by ADDING an element
/// and fails at runtime rather than at compile time.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum PaneElement {
    Title,
    WaitingNotice,
    BeaconLamp(TwinTrackBeacon),
    BeaconLabel(TwinTrackBeacon),
    BeaconAxis,
    ObserverMarker,
    ObserverLabel,
    StripTick(OrderingRow, TwinTrackBeacon),
    StripVerdict(OrderingRow),
    /// One arm of the aberration compass: the direction THIS observer measures
    /// the named ray travelling in, tinted with the colour it measures.
    CompassArm(PulseRay),
    /// The speed / angle / Doppler table for the current pulse.
    PulseReadout,
    /// The pulse front itself, plotted on this observer's own map at one
    /// instant of the observer's own time.
    PulseFront(PulseRay),
    /// Static furniture: axis rules, captions, direction keys, the divider.
    Fixture,
}

pub(crate) fn install(app: &mut App) {
    // ⭐ The third instance of the same shape in this crate — and the fact that
    // `twintrack_is_active` is HAND-COPIED into three files is the tell that the
    // gate was always wanted and never written.
    app.add_systems(
        Update,
        (
            spawn_split_observer_panes,
            sync_split_observer_cameras,
            update_split_observer_panes,
        )
            .run_if(twintrack_display_is_live),
    );

    // ⛔ Ungated on purpose: `cleanup_split_observer_panes_when_inactive` opens
    // with `if twintrack_is_active { return; }` and tears the panes down when the
    // experience ENDS. Gating it with the rest leaves them on screen forever.
    app.add_systems(Update, cleanup_split_observer_panes_when_inactive);
}

/// Run condition: the split observer panes have something to show.
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

fn beacon_color(beacon: TwinTrackBeacon) -> Color {
    match beacon {
        TwinTrackBeacon::Alpha => ALPHA_COLOR,
        TwinTrackBeacon::Omega => OMEGA_COLOR,
    }
}

fn spawn_split_observer_panes(
    mut commands: Commands,
    roots: Query<&ambition_platformer2d::runtime::demo_fixture::ActiveRoomMetadata>,
    existing: Query<(), With<SplitObserverCamera>>,
) {
    if !twintrack_is_active(&roots) || !existing.is_empty() {
        return;
    }

    for pane in SplitObserverPane::ALL {
        let layer = RenderLayers::layer(pane.layer());
        let side = pane.side_caption();

        commands.spawn((
            SplitObserverCamera(pane),
            pane,
            Camera2d,
            Camera {
                order: pane.camera_order(),
                is_active: false,
                clear_color: ClearColorConfig::Custom(pane.clear_color()),
                ..default()
            },
            layer.clone(),
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::Fixed {
                    width: PANE_WIDTH,
                    height: PANE_HEIGHT,
                },
                ..OrthographicProjection::default_2d()
            }),
            Transform::default(),
            Name::new(format!("TwinTrack {side} observer pane camera")),
        ));

        commands.spawn((
            pane,
            PaneElement::Title,
            Text2d::new(""),
            TextFont {
                font_size: FontSize::Px(22.0),
                ..default()
            },
            TextColor(Color::srgb(0.86, 0.94, 1.0)),
            Transform::from_xyz(0.0, PANE_HEIGHT * 0.5 - 46.0, 20.0),
            layer.clone(),
            Name::new(format!("TwinTrack {side} pane title")),
        ));

        commands.spawn((
            pane,
            PaneElement::WaitingNotice,
            Text2d::new("WAITING FOR THE FIRST FLASH PAIR\nTO REACH THIS OBSERVER"),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::srgb(0.70, 0.78, 0.92)),
            Visibility::Hidden,
            Transform::from_xyz(0.0, 60.0, 21.0),
            layer.clone(),
            Name::new(format!("TwinTrack {side} pane waiting notice")),
        ));

        commands.spawn((
            pane,
            PaneElement::BeaconAxis,
            Sprite::from_color(Color::srgba(0.62, 0.72, 0.88, 0.42), Vec2::ONE),
            Transform::from_xyz(0.0, BEACON_ROW_Y, 4.0),
            layer.clone(),
            Name::new(format!("TwinTrack {side} pane beacon axis")),
        ));

        for beacon in TwinTrackBeacon::ALL {
            let color = beacon_color(beacon);
            commands.spawn((
                pane,
                PaneElement::BeaconLamp(beacon),
                Sprite::from_color(color, Vec2::splat(30.0)),
                Transform::from_xyz(0.0, BEACON_ROW_Y, 8.0),
                layer.clone(),
                Name::new(format!("TwinTrack {side} pane {} lamp", beacon.label())),
            ));
            commands.spawn((
                pane,
                PaneElement::BeaconLabel(beacon),
                Text2d::new(beacon.label()),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(color),
                Transform::from_xyz(0.0, BEACON_ROW_Y + 40.0, 9.0),
                layer.clone(),
                Name::new(format!("TwinTrack {side} pane {} label", beacon.label())),
            ));
        }

        commands.spawn((
            pane,
            PaneElement::ObserverMarker,
            Sprite::from_color(Color::srgb(0.30, 1.0, 0.86), Vec2::splat(20.0)),
            Transform::from_xyz(0.0, BEACON_ROW_Y, 10.0),
            layer.clone(),
            Name::new(format!("TwinTrack {side} pane observer marker")),
        ));
        commands.spawn((
            pane,
            PaneElement::ObserverLabel,
            Text2d::new(""),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::srgb(0.55, 1.0, 0.92)),
            Transform::from_xyz(0.0, BEACON_ROW_Y - 30.0, 11.0),
            layer.clone(),
            Name::new(format!("TwinTrack {side} pane observer label")),
        ));

        for row in OrderingRow::ALL {
            commands.spawn((
                pane,
                PaneElement::Fixture,
                Sprite::from_color(
                    Color::srgba(0.72, 0.82, 0.96, 0.55),
                    Vec2::new(STRIP_HALF_WIDTH * 2.0, 3.0),
                ),
                Transform::from_xyz(0.0, row.axis_y(), 5.0),
                layer.clone(),
                Name::new(format!("TwinTrack {side} pane {row:?} axis rule")),
            ));
            commands.spawn((
                pane,
                PaneElement::Fixture,
                Text2d::new(row.caption()),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.76, 0.86, 1.0)),
                Transform::from_xyz(0.0, row.axis_y() + 54.0, 6.0),
                layer.clone(),
                Name::new(format!("TwinTrack {side} pane {row:?} caption")),
            ));
            commands.spawn((
                pane,
                PaneElement::Fixture,
                Text2d::new("<-- EARLIER          LATER -->"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgba(0.66, 0.76, 0.90, 0.85)),
                Transform::from_xyz(0.0, row.axis_y() - 24.0, 6.0),
                layer.clone(),
                Name::new(format!("TwinTrack {side} pane {row:?} direction key")),
            ));
            commands.spawn((
                pane,
                PaneElement::StripVerdict(row),
                Text2d::new(""),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(NEUTRAL_TEXT),
                Transform::from_xyz(0.0, row.axis_y() - 52.0, 7.0),
                layer.clone(),
                Name::new(format!("TwinTrack {side} pane {row:?} verdict")),
            ));
            for beacon in TwinTrackBeacon::ALL {
                commands.spawn((
                    pane,
                    PaneElement::StripTick(row, beacon),
                    Sprite::from_color(beacon_color(beacon), Vec2::new(7.0, 34.0)),
                    Transform::from_xyz(0.0, row.axis_y(), 6.5),
                    layer.clone(),
                    Name::new(format!(
                        "TwinTrack {side} pane {row:?} {} tick",
                        beacon.label()
                    )),
                ));
            }
        }

        // ---- the light-pulse instrument -------------------------------
        commands.spawn((
            pane,
            PaneElement::Fixture,
            Text2d::new("THE PULSE, IN MY FRAME"),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::srgb(0.76, 0.86, 1.0)),
            Transform::from_xyz(COMPASS_CENTER.x, COMPASS_CENTER.y + 78.0, 6.0),
            layer.clone(),
            Name::new(format!("TwinTrack {side} pane pulse caption")),
        ));
        commands.spawn((
            pane,
            PaneElement::Fixture,
            Sprite::from_color(Color::srgba(0.62, 0.72, 0.88, 0.35), Vec2::splat(12.0)),
            Transform::from_translation(COMPASS_CENTER.extend(5.0)),
            layer.clone(),
            Name::new(format!("TwinTrack {side} pane compass hub")),
        ));
        for ray in PulseRay::ALL {
            commands.spawn((
                pane,
                PaneElement::CompassArm(ray),
                Sprite::from_color(Color::WHITE, Vec2::new(COMPASS_RADIUS, 7.0)),
                Transform::from_translation(COMPASS_CENTER.extend(6.0)),
                layer.clone(),
                Name::new(format!("TwinTrack {side} pane {} compass arm", ray.label())),
            ));
            commands.spawn((
                pane,
                PaneElement::PulseFront(ray),
                Sprite::from_color(Color::WHITE, Vec2::splat(13.0)),
                Visibility::Hidden,
                Transform::from_xyz(0.0, BEACON_ROW_Y, 12.0),
                layer.clone(),
                Name::new(format!("TwinTrack {side} pane {} pulse front", ray.label())),
            ));
        }
        commands.spawn((
            pane,
            PaneElement::PulseReadout,
            Text2d::new(""),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            // left-justified on purpose: the readout is a THREE-COLUMN table
            // whose whole job is letting a viewer compare the same column
            // across two panes, and a centred block breaks the columns.
            bevy::text::TextLayout::justify(bevy::text::Justify::Left),
            TextColor(NEUTRAL_TEXT),
            Transform::from_translation(PULSE_READOUT_CENTER.extend(7.0)),
            layer.clone(),
            Name::new(format!("TwinTrack {side} pane pulse readout")),
        ));

        // A rule down the pane's inner edge, so the two halves read as two
        // instruments rather than one wide one.
        let divider_x = match pane {
            SplitObserverPane::Laboratory => PANE_WIDTH * 0.5 - 3.0,
            SplitObserverPane::Traveler => -PANE_WIDTH * 0.5 + 3.0,
        };
        commands.spawn((
            pane,
            PaneElement::Fixture,
            Sprite::from_color(
                Color::srgba(0.55, 0.68, 0.88, 0.75),
                Vec2::new(4.0, PANE_HEIGHT),
            ),
            Transform::from_xyz(divider_x, 0.0, 25.0),
            layer.clone(),
            Name::new(format!("TwinTrack {side} pane divider")),
        ));
    }
}

/// Half a render target each, side by side. `None` when the target is too small
/// to give the pane a usable rectangle.
fn pane_viewport(pane: SplitObserverPane, target: UVec2) -> Option<Viewport> {
    if target.x < MINIMUM_TARGET_PX.x || target.y < MINIMUM_TARGET_PX.y {
        return None;
    }
    let half = target.x / 2;
    Some(match pane.column() {
        0 => Viewport {
            physical_position: UVec2::ZERO,
            physical_size: UVec2::new(half, target.y),
            ..default()
        },
        _ => Viewport {
            physical_position: UVec2::new(half, 0),
            physical_size: UVec2::new(target.x - half, target.y),
            ..default()
        },
    })
}

/// each pane splits ITS OWN render target, never the primary window.
/// TwinTrack's offscreen capture has no window at all, and a rectangle measured
/// from a window that does not exist would leave the panes stacked on top of
/// each other while still reporting two active cameras.
fn sync_split_observer_cameras(
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut cameras: Query<(&SplitObserverCamera, &mut Camera)>,
) {
    let split_requested = experiment
        .single()
        .is_ok_and(|experiment| experiment.view_mode == TwinTrackViewMode::SplitObservers);
    for (pane, mut camera) in &mut cameras {
        if !split_requested {
            if camera.is_active {
                camera.is_active = false;
            }
            continue;
        }
        let viewport = camera
            .physical_target_size()
            .and_then(|target| pane_viewport(pane.0, target));
        // A pane with no rectangle yet stays dark rather than covering the
        // whole target with one observer's half of the exhibit.
        camera.is_active = viewport.is_some();
        camera.viewport = viewport;
    }
}

/// Pane-local position of a frame-relative world offset.
fn pane_point(frame_offset: Vec2) -> Vec2 {
    Vec2::new(
        frame_offset.x * WORLD_TO_PANE,
        BEACON_ROW_Y + frame_offset.y * WORLD_TO_PANE,
    )
}

fn clamp_to_pane(point: Vec2) -> Vec2 {
    Vec2::new(
        point
            .x
            .clamp(-PANE_WIDTH * 0.5 + 30.0, PANE_WIDTH * 0.5 - 30.0),
        point
            .y
            .clamp(BEACON_ROW_Y - 130.0, PANE_HEIGHT * 0.5 - 90.0),
    )
}

/// Where one of a pair of times sits on its ordering axis.
///
/// The axis is centred on the PAIR's midpoint rather than on an absolute time,
/// so what a viewer reads off it is the split and its sign — which is the only
/// thing an ordering claim is about.
fn tick_x(row: OrderingRow, time: f64, other: f64) -> f32 {
    let midpoint = 0.5 * (time + other);
    let offset = (time - midpoint) / row.full_scale_seconds();
    if !offset.is_finite() {
        return 0.0;
    }
    (offset.clamp(-1.0, 1.0) as f32) * STRIP_HALF_WIDTH
}

fn verdict_text(row: OrderingRow, report: &ObserverOrderingReport) -> String {
    let (alpha, omega) = row.times(report);
    match row.ordering(report) {
        EventOrdering::Simultaneous => "SIMULTANEOUS".to_owned(),
        ordering => format!("{} BY {:.2} s", ordering.caption(), (omega - alpha).abs()),
    }
}

/// A rough visible-spectrum ramp, so a Doppler factor becomes a COLOUR rather
/// than one more number to read.
///
/// the ends are deliberately not clamped to red and violet. At 0.9c the
/// chased ray falls to about 124 THz and the head-on ray climbs past 2 300 THz,
/// both far outside anything an eye responds to; a ramp that clamped would show
/// them as ordinary red and violet and quietly understate the effect.
fn spectral_color(frequency_thz: f64) -> Color {
    // `(frequency, red, green, blue)`, ascending.
    const STOPS: [(f64, f32, f32, f32); 6] = [
        (120.0, 0.30, 0.02, 0.02),
        (400.0, 1.00, 0.20, 0.16),
        (520.0, 0.30, 1.00, 0.45),
        (620.0, 0.30, 0.55, 1.00),
        (790.0, 0.72, 0.40, 1.00),
        (2_400.0, 0.94, 0.90, 1.00),
    ];
    if !frequency_thz.is_finite() {
        return NEUTRAL_TEXT;
    }
    let first = STOPS[0];
    let last = STOPS[STOPS.len() - 1];
    if frequency_thz <= first.0 {
        return Color::srgb(first.1, first.2, first.3);
    }
    for window in STOPS.windows(2) {
        let (low, high) = (window[0], window[1]);
        if frequency_thz <= high.0 {
            let t = ((frequency_thz - low.0) / (high.0 - low.0)) as f32;
            return Color::srgb(
                low.1 + (high.1 - low.1) * t,
                low.2 + (high.2 - low.2) * t,
                low.3 + (high.3 - low.3) * t,
            );
        }
    }
    Color::srgb(last.1, last.2, last.3)
}

/// Where this pane draws a pulse front, or `None` when the front has aged out
/// or left the drawn map.
///
/// anchored on the observer marker, not on the pane origin. The exact
/// boost gives the front's offset FROM THE OBSERVER in the observer's frame,
/// and the marker is where a viewer's eye already puts that observer. The
/// anchor is re-derived from the pulse report rather than borrowed from the
/// ordering report, so a pane can draw a pulse before the first flash pair has
/// reached its observer.
fn front_pane_point(pulse: &PulseObserverReport, ray: PulseRay) -> Option<Vec2> {
    if !pulse.front_is_visible() {
        return None;
    }
    let invariant_speed =
        ambition_platformer2d::relativity::InvariantSpeed::new(f64::from(INVARIANT_SPEED)).ok()?;
    let sample = pulse_frame_sample(
        pulse.pulse_index,
        ray,
        pulse.coordinate_time,
        pulse.position,
        pulse.velocity,
        invariant_speed,
    )?;
    let anchor = observer_frame_offset(
        pulse.position - beacon_midpoint(),
        pulse.velocity,
        pulse.lorentz_factor,
    );
    let point = clamp_to_pane(pane_point(anchor)) + sample.offset * WORLD_TO_PANE;
    let inside = point.x.abs() <= FRONT_VISIBLE_HALF_EXTENT.x
        && (point.y - BEACON_ROW_Y).abs() <= FRONT_VISIBLE_HALF_EXTENT.y;
    inside.then_some(point)
}

/// The pane's speed / angle / Doppler table for the current pulse.
///
/// The first number is the exhibit: whatever this observer's own speed is, the
/// line reads `1.000 c`.
fn pulse_readout_text(report: Option<&PulseObserverReport>) -> String {
    let Some(report) = report else {
        return "LIGHT PULSE — WAITING FOR THE LAB TO FIRE".to_owned();
    };
    let speed = report.ray(PulseRay::TowardOmega);
    let mut lines = format!(
        "LIGHT PULSE #{} · AGE {:.2} s · EMITTED AT {PULSE_REST_FREQUENCY_THZ:.0} THz\n\
         SPEED I MEASURE: {:.1} u/s = {:.3} c\n\
         RAY         ANGLE  DOPPLER    COLOUR",
        report.pulse_index,
        report.age_seconds.max(0.0),
        speed.measured_speed,
        speed.measured_speed_fraction,
    );
    for ray in PulseRay::ALL {
        let measurement = report.ray(ray);
        lines.push_str(&format!(
            "\n{:<10} {:>5.0}   x{:>5.2} {:>6.0} THz",
            ray.label(),
            measurement.apparent_angle_degrees,
            measurement.doppler_factor,
            measurement.observed_frequency_thz,
        ));
    }
    lines
}

fn title_text(pane: SplitObserverPane, report: Option<&ObserverOrderingReport>) -> String {
    let side = pane.side_caption();
    match report {
        Some(report) if report.beta > 0.005 => format!(
            "{side} · OBSERVER: {} — {:.0}% OF LIGHT",
            report.label,
            100.0 * report.beta,
        ),
        Some(report) => format!("{side} · OBSERVER: {} — AT REST", report.label),
        None => format!("{side} · OBSERVER OFFLINE"),
    }
}

#[allow(clippy::type_complexity)]
fn update_split_observer_panes(
    view: Res<TwinTrackDualObserverView>,
    pulse: Res<TwinTrackLightPulseView>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut visuals: Query<(
        &SplitObserverPane,
        &PaneElement,
        &mut Transform,
        &mut Visibility,
        Option<&mut Sprite>,
        Option<&mut Text2d>,
        Option<&mut TextColor>,
    )>,
) {
    let active = experiment
        .single()
        .is_ok_and(|experiment| experiment.view_mode == TwinTrackViewMode::SplitObservers);
    if !active {
        return;
    }
    let coordinate_time = view.coordinate_time;
    let frames_disagree = view.frame_orders_disagree();

    for (pane, element, mut transform, mut visibility, sprite, text, text_color) in &mut visuals {
        let report = pane.report(&view);
        let pulse_report = pane.pulse_report(&pulse);
        match *element {
            PaneElement::Fixture => {}
            PaneElement::Title => {
                if let Some(mut text) = text {
                    text.0 = title_text(*pane, report);
                }
            }
            PaneElement::WaitingNotice => {
                *visibility = if report.is_some() {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                };
            }
            PaneElement::BeaconLamp(beacon) => {
                let Some(mut sprite) = sprite else {
                    continue;
                };
                let Some(report) = report else {
                    sprite.color = beacon_color(beacon).with_alpha(0.12);
                    continue;
                };
                let reading = report.reading(beacon);
                let point = pane_point(reading.frame_offset);
                transform.translation.x = point.x;
                transform.translation.y = point.y;
                let lit = reading.is_lit(coordinate_time);
                sprite.color = if lit {
                    Color::WHITE
                } else {
                    beacon_color(beacon).with_alpha(0.30)
                };
                sprite.custom_size = Some(Vec2::splat(if lit { 56.0 } else { 30.0 }));
            }
            PaneElement::BeaconLabel(beacon) => {
                let Some(report) = report else {
                    continue;
                };
                let point = pane_point(report.reading(beacon).frame_offset);
                transform.translation.x = point.x;
                transform.translation.y = point.y + 44.0;
            }
            PaneElement::BeaconAxis => {
                let (Some(mut sprite), Some(report)) = (sprite, report) else {
                    continue;
                };
                let start = pane_point(report.alpha.frame_offset);
                let end = pane_point(report.omega.frame_offset);
                let delta = end - start;
                sprite.custom_size = Some(Vec2::new(delta.length().max(1.0), 2.0));
                transform.translation = ((start + end) * 0.5).extend(4.0);
                transform.rotation = Quat::from_rotation_z(delta.y.atan2(delta.x));
            }
            PaneElement::ObserverMarker => {
                let Some(report) = report else {
                    continue;
                };
                let point = clamp_to_pane(pane_point(report.frame_offset));
                transform.translation.x = point.x;
                transform.translation.y = point.y;
            }
            PaneElement::ObserverLabel => {
                let Some(mut text) = text else {
                    continue;
                };
                let Some(report) = report else {
                    text.0 = String::new();
                    continue;
                };
                let point = clamp_to_pane(pane_point(report.frame_offset));
                transform.translation.x = point.x;
                transform.translation.y = point.y - 28.0;
                text.0 = format!(
                    "{}\nALPHA {:.0} · OMEGA {:.0}",
                    report.label, report.alpha.range, report.omega.range,
                );
            }
            PaneElement::StripTick(row, beacon) => {
                let Some(report) = report else {
                    continue;
                };
                let (alpha, omega) = row.times(report);
                let (mine, other) = match beacon {
                    TwinTrackBeacon::Alpha => (alpha, omega),
                    TwinTrackBeacon::Omega => (omega, alpha),
                };
                transform.translation.x = tick_x(row, mine, other);
                transform.translation.y = row.axis_y();
            }
            PaneElement::CompassArm(ray) => {
                let (Some(mut sprite), Some(pulse_report)) = (sprite, pulse_report) else {
                    continue;
                };
                let measurement = pulse_report.ray(ray);
                let direction = measurement.apparent_direction;
                transform.translation =
                    (COMPASS_CENTER + direction * COMPASS_RADIUS * 0.5).extend(6.0);
                transform.rotation = Quat::from_rotation_z(direction.to_angle());
                sprite.custom_size = Some(Vec2::new(COMPASS_RADIUS, 7.0));
                sprite.color = spectral_color(measurement.observed_frequency_thz);
            }
            PaneElement::PulseFront(ray) => {
                let (Some(mut sprite), Some(pulse_report)) = (sprite, pulse_report) else {
                    *visibility = Visibility::Hidden;
                    continue;
                };
                let Some(point) = front_pane_point(pulse_report, ray) else {
                    *visibility = Visibility::Hidden;
                    continue;
                };
                *visibility = Visibility::Visible;
                transform.translation = point.extend(12.0);
                let measurement = pulse_report.ray(ray);
                sprite.color = spectral_color(measurement.observed_frequency_thz);
            }
            PaneElement::PulseReadout => {
                let Some(mut text) = text else {
                    continue;
                };
                text.0 = pulse_readout_text(pulse_report);
            }
            PaneElement::StripVerdict(row) => {
                let Some(mut text) = text else {
                    continue;
                };
                let Some(report) = report else {
                    text.0 = String::new();
                    continue;
                };
                text.0 = verdict_text(row, report);
                if let Some(mut color) = text_color {
                    // The frame row carries the relativistic claim, so it is the
                    // one that lights up when the two panes actually disagree.
                    color.0 = if row == OrderingRow::InMyFrame && frames_disagree {
                        DISAGREEMENT_TEXT
                    } else {
                        NEUTRAL_TEXT
                    };
                }
            }
        }
    }
}

fn cleanup_split_observer_panes_when_inactive(
    mut commands: Commands,
    roots: Query<&ambition_platformer2d::runtime::demo_fixture::ActiveRoomMetadata>,
    cameras: Query<Entity, With<SplitObserverCamera>>,
    visuals: Query<Entity, With<PaneElement>>,
) {
    if twintrack_is_active(&roots) {
        return;
    }
    for entity in cameras.iter().chain(visuals.iter()) {
        commands.entity(entity).despawn();
    }
}
