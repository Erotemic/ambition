//! Opt-in 2D spacetime integration.
//!
//! The adapter is provider-based rather than SR-hardcoded: TwinTrack installs
//! a Minkowski provider, while future analytic, sampled, or evolved GR
//! providers can implement [`SpacetimeMetric2d`] without changing clocks,
//! observations, or game entities.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use ambition_platformer2d_core::snapshot::{
    put_f32, put_u64, put_u8, put_vec2, Reader, RollbackRegistrar, SnapshotState, StateHasher,
};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::lifecycle::SessionRoot;
use ambition_platformer2d_shared_tangle::schedule::{
    Platformer2dSimulationPhaseMonolith, SimScheduleExt, WorldPrepSet,
};
use ambition_relativity::{ClockRateResult, IntervalKind, InvariantSpeed};
use ambition_time::{ProperTimeScale, WorldTime};
use bevy::prelude::*;

mod optics;
mod signals;
mod targeting;
mod telemetry;

pub use optics::{
    solve_retarded_source_event, ObserverOpticalView2d, OpticalObserverObservation2d,
    OpticalSource2d, OpticalSourceObservation2d, RelativisticObserver2d, RelativisticOpticalView2d,
    RetardedSourceEvent2d,
};
pub use signals::{
    LightEmissionRequest2d, LightEmitter2d, LightEmitterObservation2d, LightReceiver2d,
    LightReceiverMode2d, LightReceiverObservation2d, LightSignal2d, LightSignalObservation2d,
    LightSignalPoolSlot2d, ProperTimeCooldown2d, RelativitySignalView2d, SignalArrival2d,
    SignalArrivalHistory2d, SignalArrivalRecord2d, SpacetimeCoordinateTime2d,
};
pub use targeting::{
    ObserverTargetingView2d, RelativisticTarget2d, RelativisticTargetObservation2d,
    RelativisticTargetingView2d,
};
pub use telemetry::{
    WorldlineHistoryView2d, WorldlineSample2d, WorldlineTrackId, WorldlineTracked2d,
    DEFAULT_WORLDLINE_HISTORY_SAMPLES,
};

/// A 2D spacetime provider sampled by engine entities.
///
/// Proper-time measurement is intentionally the first narrow contract. Curved
/// providers may later add metric samples, tetrads, derivatives, and geodesics
/// through additive traits while preserving this consumer boundary.
pub trait SpacetimeMetric2d: Send + Sync + 'static {
    fn model_id(&self) -> &'static str;
    fn invariant_speed(&self) -> InvariantSpeed;
    fn deterministic_fingerprint(&self) -> u64;
    fn measure_clock(&self, position: Vec2, coordinate_velocity: Vec2) -> ClockMeasurement2d;

    /// Return the invariant speed only when this provider exposes one global
    /// Minkowski chart in which straight analytic null rays and global Lorentz
    /// boosts are exact. Curved providers keep the default `None` and add their
    /// own geodesic/tetrad optical capability rather than inheriting flat optics
    /// accidentally.
    fn minkowski_optics_invariant_speed(&self) -> Option<InvariantSpeed> {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClockMeasurement2d {
    pub relative_velocity: Vec2,
    pub rate: ClockRateResult,
}

/// Transform one coordinate velocity into an inertial observer frame moving at
/// `frame_velocity` in flat spacetime.
///
/// This is deliberately separate from the spacetime provider used for clock
/// accumulation: selecting an observer must not redefine the coordinate time
/// advanced by the simulation.
pub fn minkowski_relative_velocity_2d(
    coordinate_velocity: Vec2,
    frame_velocity: Vec2,
    invariant_speed: InvariantSpeed,
) -> Option<Vec2> {
    if !coordinate_velocity.is_finite() || !frame_velocity.is_finite() {
        return None;
    }
    let frame_speed_squared = frame_velocity.length_squared() as f64;
    if frame_speed_squared == 0.0 {
        return Some(coordinate_velocity);
    }
    let beta_squared = frame_speed_squared / invariant_speed.squared();
    if !(0.0..1.0).contains(&beta_squared) {
        return None;
    }

    let frame_speed = frame_speed_squared.sqrt() as f32;
    let direction = frame_velocity / frame_speed;
    let parallel = direction * coordinate_velocity.dot(direction);
    let transverse = coordinate_velocity - parallel;
    let gamma = 1.0 / (1.0 - beta_squared).sqrt();
    let denominator =
        1.0 - frame_velocity.dot(coordinate_velocity) as f64 / invariant_speed.squared();
    if !denominator.is_finite() || denominator.abs() <= 64.0 * f64::EPSILON {
        return None;
    }

    let transformed = (parallel - frame_velocity) / denominator as f32
        + transverse / (gamma * denominator) as f32;
    transformed.is_finite().then_some(transformed)
}

/// Flat spacetime in the engine's coordinate chart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinkowskiSpacetime2d {
    invariant_speed: InvariantSpeed,
}

impl MinkowskiSpacetime2d {
    pub fn new(invariant_speed: f64) -> Result<Self, ambition_relativity::InvariantSpeedError> {
        Ok(Self {
            invariant_speed: InvariantSpeed::new(invariant_speed)?,
        })
    }
}

impl SpacetimeMetric2d for MinkowskiSpacetime2d {
    fn model_id(&self) -> &'static str {
        "minkowski"
    }

    fn invariant_speed(&self) -> InvariantSpeed {
        self.invariant_speed
    }

    fn deterministic_fingerprint(&self) -> u64 {
        let mut hasher = StateHasher::default();
        hasher.write(b"ambition.minkowski2d.v1");
        hasher.write(&self.invariant_speed.get().to_bits().to_le_bytes());
        hasher.finish()
    }

    fn measure_clock(&self, _position: Vec2, coordinate_velocity: Vec2) -> ClockMeasurement2d {
        ClockMeasurement2d {
            relative_velocity: coordinate_velocity,
            rate: ambition_relativity::minkowski_clock_rate(
                coordinate_velocity.length_squared() as f64,
                self.invariant_speed,
            ),
        }
    }

    fn minkowski_optics_invariant_speed(&self) -> Option<InvariantSpeed> {
        Some(self.invariant_speed)
    }
}

/// The exact spacetime model owned by a live gameplay session.
#[derive(Component, Clone)]
pub struct ActiveSpacetime2d {
    model: Arc<dyn SpacetimeMetric2d>,
}

impl ActiveSpacetime2d {
    pub fn new(model: impl SpacetimeMetric2d) -> Self {
        Self {
            model: Arc::new(model),
        }
    }

    pub fn minkowski(
        invariant_speed: f64,
    ) -> Result<Self, ambition_relativity::InvariantSpeedError> {
        Ok(Self::new(MinkowskiSpacetime2d::new(invariant_speed)?))
    }

    pub fn model(&self) -> &dyn SpacetimeMetric2d {
        self.model.as_ref()
    }

    pub fn deterministic_fingerprint(&self) -> u64 {
        self.model.deterministic_fingerprint()
    }
}

impl fmt::Debug for ActiveSpacetime2d {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActiveSpacetime2d")
            .field("model_id", &self.model.model_id())
            .field("invariant_speed", &self.model.invariant_speed().get())
            .finish()
    }
}

fn active_spacetime_checksum(spacetime: &ActiveSpacetime2d) -> u64 {
    spacetime.deterministic_fingerprint()
}

/// Opt one canonical 2D body into spacetime-derived proper time.
#[derive(Component, Clone, Copy, Debug, Default)]
#[require(ProperTimeScale, ProperTimeElapsed, RelativityState2d)]
pub struct RelativisticClock2d;

/// Authoritative proper time accumulated along one worldline.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct ProperTimeElapsed {
    pub seconds: f64,
}

impl ProperTimeElapsed {
    pub const ZERO: Self = Self { seconds: 0.0 };

    pub fn reset(&mut self) {
        self.seconds = 0.0;
    }
}

impl SnapshotState for ProperTimeElapsed {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.seconds.to_bits());
    }

    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            seconds: f64::from_bits(reader.u64()?),
        })
    }
}

/// Stable presentation name for a published clock.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct RelativityClockLabel(pub String);

/// Derived diagnostic state from the most recent spacetime sample.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct RelativityState2d {
    pub relative_velocity: Vec2,
    pub invariant_speed: f32,
    pub beta_squared: f32,
    pub lorentz_factor: f32,
    pub proper_time_rate: f32,
    pub interval_kind: IntervalKind,
}

impl Default for RelativityState2d {
    fn default() -> Self {
        Self {
            relative_velocity: Vec2::ZERO,
            invariant_speed: 1.0,
            beta_squared: 0.0,
            lorentz_factor: 1.0,
            proper_time_rate: 1.0,
            interval_kind: IntervalKind::Timelike,
        }
    }
}

impl SnapshotState for RelativityState2d {
    fn encode(&self, out: &mut Vec<u8>) {
        put_vec2(out, self.relative_velocity);
        put_f32(out, self.invariant_speed);
        put_f32(out, self.beta_squared);
        put_f32(out, self.lorentz_factor);
        put_f32(out, self.proper_time_rate);
        put_u8(
            out,
            match self.interval_kind {
                IntervalKind::Timelike => 0,
                IntervalKind::Null => 1,
                IntervalKind::Spacelike => 2,
            },
        );
    }

    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            relative_velocity: reader.vec2()?,
            invariant_speed: reader.f32()?,
            beta_squared: reader.f32()?,
            lorentz_factor: reader.f32()?,
            proper_time_rate: reader.f32()?,
            interval_kind: match reader.u8()? {
                0 => IntervalKind::Timelike,
                1 => IntervalKind::Null,
                2 => IntervalKind::Spacelike,
                _ => return None,
            },
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RelativityClockObservation2d {
    pub entity: Entity,
    pub position: Vec2,
    pub coordinate_velocity: Vec2,
    pub relative_velocity: Vec2,
    pub invariant_speed: f32,
    pub beta_squared: f32,
    pub lorentz_factor: f32,
    pub proper_time_rate: f32,
    pub proper_time_seconds: f64,
    pub interval_kind: IntervalKind,
}

/// Presentation-facing read model keyed by provider-authored clock labels.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct RelativityClockView2d {
    pub model_id: Option<&'static str>,
    pub clocks: BTreeMap<String, RelativityClockObservation2d>,
}

#[derive(SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Relativity2dSet {
    AdvanceCoordinateTime,
    ResolveClocks,
    AdvanceProperCooldowns,
    PublishView,
    PublishOptics,
    PublishTargeting,
}

/// Declare every rollback-relevant Relativity2d value through the backend-neutral registrar.
/// Composition may run this once for schema metadata and again through a concrete
/// rollback backend without the domain depending on that backend.
pub fn register_rollback_state(registrar: &mut impl RollbackRegistrar) {
    //  DERIVED, not snapshot state. `publish_clock_view` clears the map
    // and rebuilds every entry each tick out of `RelativityState2d` and
    // `ProperTimeElapsed`, both registered below — so a rewind that restores
    // those restores this on the next publish, and snapshotting it would
    // store a second copy of state the schema already owns.
    //
    //  this is the shape worth being careful about: a "derived" resource
    // that also carried an accumulator or an already-applied gate would be
    // rollback state wearing a cache's name. This one has neither — the
    // first statement in its writer is `clocks.clear()`, and the only other
    // writer resets it wholesale when no spacetime is live.
    registrar.declare_rollback_derived_resource::<RelativityClockView2d>(
        "ambition_relativity2d",
        "relativity.clock_view_2d",
        "presentation read model rebuilt every tick from RelativityState2d + ProperTimeElapsed",
    );
    registrar
        .rollback_component_canonical::<ProperTimeElapsed>(
            "ambition_relativity2d",
            "relativity.proper_time_elapsed",
        )
        .rollback_component_clone_checksum_with_schema_detail::<ActiveSpacetime2d>(
            "ambition_relativity2d",
            "relativity.active_spacetime_2d",
            "provider-authored spacetime model and invariant-speed fingerprint",
            active_spacetime_checksum,
        )
        // A value probe over the LABEL. The label is the whole
        // component and the identity a clock readout is joined by; a presence
        // probe saw none of it.
        .rollback_component_clone_probed::<RelativityClockLabel>(
            "ambition_relativity2d",
            "relativity.clock_label_2d",
            |label| crate::telemetry::hash_label(&label.0),
        )
        .rollback_component_clone::<RelativisticClock2d>(
            "ambition_relativity2d",
            "relativity.clock_marker_2d",
        )
        .declare_rollback_derived_component_state::<RelativityState2d>(
            "ambition_relativity2d",
            "relativity.state_2d",
            "recomputed from canonical body kinematics and the session spacetime",
        );

    signals::register_rollback_state(registrar);
    telemetry::register_rollback_state(registrar);
    optics::register_rollback_state(registrar);
    targeting::register_rollback_state(registrar);
}

pub struct Relativity2dPlugin;

impl Plugin for Relativity2dPlugin {
    fn build(&self, app: &mut App) {
        {
            let mut registrar =
                ambition_platformer2d_runtime::rollback::SchemaRollbackRegistrar::new(app);
            register_rollback_state(&mut registrar);
        }
        app.init_resource::<RelativityClockView2d>();

        let sim = app.sim_schedule();
        signals::install_signal_systems(app, sim);
        telemetry::install_telemetry_systems(app, sim);
        optics::install_optics_systems(app, sim);
        targeting::install_targeting_systems(app, sim);
        app.add_systems(
            sim,
            resolve_and_advance_clocks
                .run_if(spacetime_is_active)
                .in_set(Relativity2dSet::ResolveClocks)
                .in_set(WorldPrepSet::BeforeIntegrate)
                .after(Relativity2dSet::AdvanceCoordinateTime),
        )
        .add_systems(
            sim,
            publish_clock_view
                .run_if(spacetime_is_active)
                .in_set(Relativity2dSet::PublishView)
                .in_set(Platformer2dSimulationPhaseMonolith::FeatureViewSync),
        )
        .add_systems(Update, clear_view_without_live_spacetime);
    }
}

pub(crate) fn spacetime_is_active(
    spacetime: Query<(), (With<ActiveSpacetime2d>, With<SessionRoot>)>,
) -> bool {
    !spacetime.is_empty()
}

fn resolve_and_advance_clocks(
    time: Res<WorldTime>,
    spacetime: Query<&ActiveSpacetime2d, With<SessionRoot>>,
    mut clocks: Query<
        (
            &BodyKinematics,
            &mut ProperTimeScale,
            &mut ProperTimeElapsed,
            &mut RelativityState2d,
        ),
        With<RelativisticClock2d>,
    >,
) {
    let Ok(spacetime) = spacetime.single() else {
        return;
    };
    let model = spacetime.model();
    let invariant_speed = model.invariant_speed().get() as f32;
    for (body, mut scale, mut clock, mut state) in &mut clocks {
        let measurement = model.measure_clock(body.pos, body.vel);
        let result = measurement.rate;
        let rate64 = result.proper_time_rate.get();
        let rate = rate64 as f32;
        scale.0 = rate;
        clock.seconds += f64::from(time.sim_dt()) * rate64;
        *state = RelativityState2d {
            relative_velocity: measurement.relative_velocity,
            invariant_speed,
            beta_squared: result.beta_squared as f32,
            lorentz_factor: result.lorentz_factor.unwrap_or(f64::INFINITY) as f32,
            proper_time_rate: rate,
            interval_kind: result.interval_kind,
        };
    }
}

fn publish_clock_view(
    spacetime: Query<&ActiveSpacetime2d, With<SessionRoot>>,
    clocks: Query<
        (
            Entity,
            &RelativityClockLabel,
            &BodyKinematics,
            &ProperTimeElapsed,
            &RelativityState2d,
        ),
        With<RelativisticClock2d>,
    >,
    mut view: ResMut<RelativityClockView2d>,
) {
    view.clocks.clear();
    let Ok(spacetime) = spacetime.single() else {
        view.model_id = None;
        return;
    };
    view.model_id = Some(spacetime.model().model_id());
    for (entity, label, body, clock, state) in &clocks {
        view.clocks.insert(
            label.0.clone(),
            RelativityClockObservation2d {
                entity,
                position: body.pos,
                coordinate_velocity: body.vel,
                relative_velocity: state.relative_velocity,
                invariant_speed: state.invariant_speed,
                beta_squared: state.beta_squared,
                lorentz_factor: state.lorentz_factor,
                proper_time_rate: state.proper_time_rate,
                proper_time_seconds: clock.seconds,
                interval_kind: state.interval_kind,
            },
        );
    }
}

fn clear_view_without_live_spacetime(
    spacetime: Query<(), (With<ActiveSpacetime2d>, With<SessionRoot>)>,
    mut view: ResMut<RelativityClockView2d>,
) {
    if spacetime.is_empty() && (view.model_id.is_some() || !view.clocks.is_empty()) {
        *view = RelativityClockView2d::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observer_velocity_transform_is_lorentzian() {
        let c = InvariantSpeed::new(100.0).unwrap();
        let transformed =
            minkowski_relative_velocity_2d(Vec2::new(80.0, 0.0), Vec2::new(50.0, 0.0), c).unwrap();
        assert!((transformed.x - 50.0).abs() < 1.0e-4);
        assert!(transformed.y.abs() < 1.0e-5);
    }

    #[test]
    fn observer_frame_must_be_timelike() {
        let c = InvariantSpeed::new(100.0).unwrap();
        assert!(minkowski_relative_velocity_2d(Vec2::ZERO, Vec2::new(100.0, 0.0), c).is_none());
    }

    #[test]
    fn minkowski_provider_uses_engine_coordinate_time() {
        let metric = MinkowskiSpacetime2d::new(100.0).unwrap();
        let measured = metric.measure_clock(Vec2::ZERO, Vec2::new(80.0, 0.0));
        assert_eq!(measured.relative_velocity, Vec2::new(80.0, 0.0));
        assert!((measured.rate.proper_time_rate.get() - 0.6).abs() < 1.0e-12);
    }
}
