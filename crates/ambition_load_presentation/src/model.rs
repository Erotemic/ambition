//! Load-foreground policy, semantic view model, and arbitrary activity protocol.

use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

use ambition_game_shell::{LoadBarrierRef, ShellRouteId};
use ambition_load::{BarrierReadiness, LoadBarrierSnapshot, LoadFailure, ProgressEstimate};
use bevy::prelude::{Component, Message, Resource};

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);
        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                let value = value.into();
                assert!(
                    !value.trim().is_empty(),
                    concat!(stringify!($name), " cannot be empty")
                );
                Self(value)
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }
        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

string_id!(LoadExperienceId);
string_id!(LoadActivityId);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadyTransitionPolicy {
    AutoAdvance,
    AwaitConfirmation,
    AutoUnlessEngaged,
}

impl ReadyTransitionPolicy {
    pub fn holds_ready(self, foreground_visible: bool, engaged: bool) -> bool {
        match self {
            Self::AutoAdvance => false,
            Self::AwaitConfirmation => foreground_visible,
            Self::AutoUnlessEngaged => engaged,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadExperienceSpec {
    pub id: LoadExperienceId,
    pub reveal_after: Duration,
    pub ready_policy: ReadyTransitionPolicy,
    pub activity: Option<LoadActivityId>,
    pub show_estimated_percentage: bool,
}

impl LoadExperienceSpec {
    pub fn basic(id: impl Into<String>) -> Self {
        Self {
            id: LoadExperienceId::new(id),
            reveal_after: Duration::from_millis(250),
            ready_policy: ReadyTransitionPolicy::AutoUnlessEngaged,
            activity: None,
            show_estimated_percentage: true,
        }
    }
}

#[derive(Resource)]
pub struct LoadPresentationCatalog {
    pub default: LoadExperienceSpec,
    pub by_route: BTreeMap<ShellRouteId, LoadExperienceSpec>,
}

impl Default for LoadPresentationCatalog {
    fn default() -> Self {
        Self {
            default: LoadExperienceSpec::basic("ambition.load.basic"),
            by_route: BTreeMap::new(),
        }
    }
}

impl LoadPresentationCatalog {
    pub fn for_route(&self, route: &ShellRouteId) -> &LoadExperienceSpec {
        self.by_route.get(route).unwrap_or(&self.default)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoadForegroundPhase {
    HiddenGrace,
    Visible,
    ReadyHold,
    Failed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActiveLoadForeground {
    pub route_id: ShellRouteId,
    pub barrier: LoadBarrierRef,
    pub spec: LoadExperienceSpec,
    pub phase: LoadForegroundPhase,
    pub elapsed: Duration,
    pub activity_activation_id: Option<u64>,
    pub engaged: bool,
}

#[derive(Resource, Default)]
pub struct LoadForegroundState {
    pub active: Option<ActiveLoadForeground>,
    next_activity_activation: u64,
}

impl LoadForegroundState {
    pub(crate) fn next_activity_activation(&mut self) -> u64 {
        self.next_activity_activation = self.next_activity_activation.saturating_add(1);
        self.next_activity_activation
    }
}

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct LoadPresentationModel {
    pub visible: bool,
    pub ready_hold: bool,
    pub route_id: Option<ShellRouteId>,
    pub readiness: Option<BarrierReadiness>,
    pub stage: String,
    pub completed_steps: usize,
    pub active_steps: usize,
    pub known_remaining_steps: usize,
    pub discovery_open: bool,
    pub estimated_additional_steps: Option<std::ops::RangeInclusive<usize>>,
    pub estimate: Option<ProgressEstimate>,
    pub completed_labels: Vec<String>,
    pub active_labels: Vec<String>,
    pub remaining_labels: Vec<String>,
    pub streamable_labels: Vec<String>,
    pub speculative_labels: Vec<String>,
    pub failures: Vec<LoadFailure>,
    pub activity: Option<LoadActivityId>,
    pub activity_engaged: bool,
}

impl Default for LoadPresentationModel {
    fn default() -> Self {
        Self {
            visible: false,
            ready_hold: false,
            route_id: None,
            readiness: None,
            stage: String::new(),
            completed_steps: 0,
            active_steps: 0,
            known_remaining_steps: 0,
            discovery_open: false,
            estimated_additional_steps: None,
            estimate: None,
            completed_labels: Vec::new(),
            active_labels: Vec::new(),
            remaining_labels: Vec::new(),
            streamable_labels: Vec::new(),
            speculative_labels: Vec::new(),
            failures: Vec::new(),
            activity: None,
            activity_engaged: false,
        }
    }
}

impl LoadPresentationModel {
    pub fn from_snapshot(
        route_id: ShellRouteId,
        snapshot: LoadBarrierSnapshot,
        foreground: &ActiveLoadForeground,
    ) -> Self {
        let ready = snapshot.ready();
        let mut estimate = foreground
            .spec
            .show_estimated_percentage
            .then_some(snapshot.estimate)
            .flatten();
        if let Some(estimate) = estimate.as_mut() {
            if !ready {
                estimate.fraction = estimate.fraction.min(0.999);
            }
        }
        Self {
            visible: foreground.phase != LoadForegroundPhase::HiddenGrace,
            ready_hold: foreground.phase == LoadForegroundPhase::ReadyHold,
            route_id: Some(route_id),
            readiness: Some(snapshot.readiness),
            stage: snapshot.label,
            completed_steps: snapshot.completed_steps,
            active_steps: snapshot.active_steps,
            known_remaining_steps: snapshot.known_remaining_steps,
            discovery_open: snapshot.discovery_open,
            estimated_additional_steps: snapshot.estimated_additional_steps,
            estimate,
            completed_labels: snapshot.completed_labels,
            active_labels: snapshot.active_labels,
            remaining_labels: snapshot.remaining_labels,
            streamable_labels: snapshot.streamable_labels,
            speculative_labels: snapshot.speculative_labels,
            failures: snapshot.failures,
            activity: foreground.spec.activity.clone(),
            activity_engaged: foreground.engaged,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActiveLoadActivity {
    pub activation_id: u64,
    pub activity_id: LoadActivityId,
    pub route_id: ShellRouteId,
    pub barrier: LoadBarrierRef,
}

#[derive(Resource, Default)]
pub struct LoadActivityState {
    pub active: Option<ActiveLoadActivity>,
    pub last_outcome: Option<LoadActivityOutcome>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LoadActivityOutcome {
    pub score: Option<u64>,
    pub completed: bool,
    pub telemetry: BTreeMap<String, String>,
}

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoadActivityScopedEntity {
    pub activation_id: u64,
}

#[derive(Message, Clone, Debug, Eq, PartialEq)]
pub enum LoadActivitySignal {
    Engaged {
        activation_id: u64,
    },
    Finished {
        activation_id: u64,
        outcome: LoadActivityOutcome,
    },
    Failed {
        activation_id: u64,
        message: String,
    },
}

#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoadPresentationAction {
    Continue,
    Retry,
    CancelToPrevious,
    QuitToHome,
}

/// Semantic requests that game-owned load contributors may handle.
///
/// The presentation layer cannot recreate a failed plan because retry policy
/// and destination parameters belong to the composing game.
#[derive(Message, Clone, Debug, Eq, PartialEq)]
pub enum LoadPresentationEvent {
    RetryRequested {
        route_id: ShellRouteId,
        barrier: LoadBarrierRef,
    },
}
