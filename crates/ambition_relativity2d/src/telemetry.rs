//! Bounded rollback-aware worldline telemetry for relativity instruments.

use std::collections::{BTreeMap, VecDeque};

use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::lifecycle::SessionRoot;
use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith;
use ambition_time::SimTick;
use bevy::ecs::entity::Entity;
use bevy::ecs::schedule::InternedScheduleLabel;
use bevy::prelude::{
    App, Component, IntoScheduleConfigs, Query, Res, ResMut, Resource, Update, Vec2, With,
};

use crate::{ActiveSpacetime2d, ProperTimeElapsed, Relativity2dSet, SpacetimeCoordinateTime2d};

pub const DEFAULT_WORLDLINE_HISTORY_SAMPLES: usize = 720;

/// A track's stable identity — never displayed.
///
/// This is separate from the display label so renaming presentation does not
/// move telemetry history and two bodies may share the same caption.
///
/// It remains a `String` because not every tracked body has a `SimId`; the
/// requirement is a stable, non-presentational track key.
#[derive(Component, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorldlineTrackId(pub String);

impl WorldlineTrackId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Opt one body into bounded worldline telemetry.
///
///  `label` is PRESENTATION and nothing keys on it. See [`WorldlineTrackId`].
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct WorldlineTracked2d {
    pub track: WorldlineTrackId,
    pub label: String,
}

impl WorldlineTracked2d {
    /// The common case: a body whose caption and its identity are the same
    /// string TODAY. They are still two values — renaming the caption later is
    /// a one-field edit that does not move the history.
    pub fn new(id_and_label: impl Into<String>) -> Self {
        let value: String = id_and_label.into();
        Self {
            track: WorldlineTrackId(value.clone()),
            label: value,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
}

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
    pub tracks: BTreeMap<WorldlineTrackId, VecDeque<WorldlineSample2d>>,
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

/// A stable hash of a label, for the rollback value probes below.
///
///  `DefaultHasher` is not stable ACROSS Rust releases, and that is fine here:
/// a checksum compares two peers running the same binary, which is already the
/// premise of every other projection in the registry.
pub(crate) fn hash_label(label: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    label.hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn register_rollback_state(registrar: &mut impl ambition_platformer2d_core::snapshot::RollbackRegistrar) {
    registrar.declare_rollback_derived_resource::<WorldlineHistoryView2d>(
        "ambition_relativity2d",
        "relativity.worldline_history_view_2d",
        "bounded tick-keyed telemetry that truncates abandoned rollback futures and rebuilds on resimulation",
    );
    registrar.rollback_component_clone_probed::<WorldlineTracked2d>(
        "ambition_relativity2d",
        "relativity.worldline_tracked_2d",
        |tracked| hash_label(tracked.track.as_str()),
    );
}

pub(crate) fn install_telemetry_systems(app: &mut App, sim: InternedScheduleLabel) {
    app.init_resource::<WorldlineHistoryView2d>();
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
        Entity,
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

    // A track has one deterministic owner. Sort by stable track id and entity,
    // then accept the first claimant and refuse duplicates so query iteration
    // order cannot choose which worldline survives.
    let mut rows: Vec<_> = tracked.iter().collect();
    rows.sort_by(|(lhs_entity, lhs, ..), (rhs_entity, rhs, ..)| {
        lhs.track
            .cmp(&rhs.track)
            .then_with(|| lhs_entity.cmp(rhs_entity))
    });
    let mut claimed: std::collections::BTreeSet<&str> = Default::default();
    for (entity, tracked, body, proper_time) in rows {
        if !claimed.insert(tracked.track.as_str()) {
            bevy::log::warn_once!(
                "worldline TRACK ID {:?} is claimed by more than one entity ({entity:?} is not \
                 the owner); its samples are refused rather than overwriting the owner's. Two \
                 bodies may share a display label ({:?}) — they may not share an identity.",
                tracked.track,
                tracked.label
            );
            continue;
        }
        let samples = history.tracks.entry(tracked.track.clone()).or_default();
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

    /// Renaming what a body is CALLED does not move its history.
    ///
    /// The display label is independent of the stable track id used as the
    /// history address.
    #[test]
    fn a_label_is_presentation_and_the_track_id_is_the_address() {
        let before = WorldlineTracked2d::new("traveler");
        assert_eq!(before.track, WorldlineTrackId("traveler".into()));
        assert_eq!(before.label, "traveler");

        let renamed = before.clone().with_label("The Traveller (ship clock)");
        assert_eq!(
            renamed.track, before.track,
            "renaming the caption moved the track identity, so the body's history \
             is now addressed somewhere nothing has written"
        );
        assert_ne!(renamed.label, before.label);
    }

    /// Two bodies may share a caption; they may not share an identity.
    ///
    /// The first was a real limitation of the string-keyed map — one of the two
    /// silently received no telemetry at all — and it is the one that should
    /// never have been a limitation.
    #[test]
    fn two_bodies_may_share_a_caption() {
        let one = WorldlineTracked2d::new("clock_a").with_label("Clock");
        let two = WorldlineTracked2d::new("clock_b").with_label("Clock");
        assert_eq!(one.label, two.label);
        assert_ne!(
            one.track, two.track,
            "two distinctly-identified bodies collided, which is the case the \
             ownership refusal is FOR — it should not be reachable by captioning"
        );

        let mut tracks: BTreeMap<WorldlineTrackId, VecDeque<WorldlineSample2d>> =
            Default::default();
        tracks.entry(one.track.clone()).or_default();
        tracks.entry(two.track.clone()).or_default();
        assert_eq!(
            tracks.len(),
            2,
            "the two bodies shared one history because they shared a caption"
        );
    }

    #[test]
    fn default_history_is_bounded() {
        let history = WorldlineHistoryView2d::default();
        assert!(history.capacity_per_track >= 2);
        assert_eq!(history.coordinate_epoch, None);
        assert!(history.tracks.is_empty());
    }
}
