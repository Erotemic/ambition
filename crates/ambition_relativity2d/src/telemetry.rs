//! Bounded rollback-aware worldline telemetry for relativity instruments.

use std::collections::{BTreeMap, VecDeque};

use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_runtime::rollback::AmbitionRollbackApp;
use ambition_platformer2d_shared_tangle::lifecycle::SessionRoot;
use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith;
use ambition_time::SimTick;
use bevy::ecs::schedule::InternedScheduleLabel;
use bevy::prelude::{
    App, Component, IntoScheduleConfigs, Query, Res, ResMut, Resource, Update, Vec2, With,
};

use crate::{
    ActiveSpacetime2d, ProperTimeElapsed, Relativity2dSet, SpacetimeCoordinateTime2d,
};

pub const DEFAULT_WORLDLINE_HISTORY_SAMPLES: usize = 720;

/// Opt one body into bounded worldline telemetry.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct WorldlineTracked2d(pub String);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldlineSample2d {
    pub sim_tick: u64,
    pub coordinate_time: f64,
    pub proper_time: Option<f64>,
    pub position: Vec2,
    pub coordinate_velocity: Vec2,
}

/// Rollback-aware derived telemetry. Samples from an abandoned future are
/// truncated when the current simulation tick moves backward, then rebuilt by
/// deterministic resimulation.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct WorldlineHistoryView2d {
    pub capacity_per_track: usize,
    pub coordinate_epoch: Option<u64>,
    pub tracks: BTreeMap<String, VecDeque<WorldlineSample2d>>,
}

impl Default for WorldlineHistoryView2d {
    fn default() -> Self {
        Self {
            capacity_per_track: DEFAULT_WORLDLINE_HISTORY_SAMPLES,
            coordinate_epoch: None,
            tracks: BTreeMap::new(),
        }
    }
}

impl WorldlineHistoryView2d {
    pub fn clear(&mut self) {
        self.coordinate_epoch = None;
        self.tracks.clear();
    }
}

pub(crate) fn install_telemetry_systems(app: &mut App, sim: InternedScheduleLabel) {
    app.init_resource::<WorldlineHistoryView2d>();
    app.declare_rollback_derived_resource::<WorldlineHistoryView2d>(
        "ambition_relativity2d",
        "relativity.worldline_history_view_2d",
        "bounded tick-keyed telemetry that truncates abandoned rollback futures and rebuilds on resimulation",
    );
    app.rollback_component_clone::<WorldlineTracked2d>(
        "ambition_relativity2d",
        "relativity.worldline_tracked_2d",
    );

    app.add_systems(
        sim,
        publish_worldline_history
            .run_if(crate::spacetime_is_active)
            .in_set(Relativity2dSet::PublishView)
            .in_set(Platformer2dSimulationPhaseMonolith::FeatureViewSync),
    )
    .add_systems(Update, clear_worldlines_without_live_spacetime);
}

fn publish_worldline_history(
    tick: Res<SimTick>,
    coordinate_time: Query<
        &SpacetimeCoordinateTime2d,
        (With<ActiveSpacetime2d>, With<SessionRoot>),
    >,
    tracked: Query<(
        &WorldlineTracked2d,
        &BodyKinematics,
        Option<&ProperTimeElapsed>,
    )>,
    mut history: ResMut<WorldlineHistoryView2d>,
) {
    let Ok(coordinate_time) = coordinate_time.single() else {
        return;
    };
    let current_tick = tick.get();
    if history.coordinate_epoch != Some(coordinate_time.epoch) {
        history.tracks.clear();
        history.coordinate_epoch = Some(coordinate_time.epoch);
    }
    let capacity = history.capacity_per_track.max(2);

    for (label, body, proper_time) in &tracked {
        let samples = history.tracks.entry(label.0.clone()).or_default();
        while samples
            .back()
            .is_some_and(|sample| sample.sim_tick >= current_tick)
        {
            samples.pop_back();
        }
        samples.push_back(WorldlineSample2d {
            sim_tick: current_tick,
            coordinate_time: coordinate_time.seconds,
            proper_time: proper_time.map(|clock| clock.seconds),
            position: body.pos,
            coordinate_velocity: body.vel,
        });
        while samples.len() > capacity {
            samples.pop_front();
        }
    }
}

fn clear_worldlines_without_live_spacetime(
    spacetime: Query<(), (With<ActiveSpacetime2d>, With<SessionRoot>)>,
    mut history: ResMut<WorldlineHistoryView2d>,
) {
    if spacetime.is_empty() && !history.tracks.is_empty() {
        history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_history_is_bounded() {
        let history = WorldlineHistoryView2d::default();
        assert!(history.capacity_per_track >= 2);
        assert_eq!(history.coordinate_epoch, None);
        assert!(history.tracks.is_empty());
    }
}
