//! Load-foreground policy, semantic view model, and arbitrary activity protocol.

use std::collections::BTreeMap;
use std::time::Duration;

use ambition_load::{
    string_id, BarrierReadiness, LoadBarrierRef, LoadBarrierSnapshot, LoadFailure, ProgressEstimate,
};
use bevy::prelude::{Component, Message, Resource};

string_id!(LoadExperienceId);
string_id!(LoadActivityId);
string_id!(LoadPresentationOwnerId);

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
    pub input_owner: Option<ambition_input::InputOwner>,
    pub id: LoadExperienceId,
    pub reveal_after: Duration,
    pub ready_policy: ReadyTransitionPolicy,
    pub activity: Option<LoadActivityId>,
    pub show_estimated_percentage: bool,
}

impl LoadExperienceSpec {
    pub fn basic(id: impl Into<String>) -> Self {
        Self {
            input_owner: Some(ambition_input::InputOwner::AllParticipants),
            id: LoadExperienceId::new(id),
            reveal_after: Duration::from_millis(250),
            ready_policy: ReadyTransitionPolicy::AutoUnlessEngaged,
            activity: None,
            show_estimated_percentage: true,
        }
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
    pub owner: LoadPresentationOwnerId,
    pub barrier: LoadBarrierRef,
    pub spec: LoadExperienceSpec,
    pub phase: LoadForegroundPhase,
    pub elapsed: Duration,
    pub activity_activation_id: Option<u64>,
    pub engaged: bool,
    pub ready_released: bool,
}

impl ActiveLoadForeground {
    pub fn should_hold_ready(&self) -> bool {
        self.phase == LoadForegroundPhase::ReadyHold && !self.ready_released
    }
}

/// Each load owner has its own foreground, model, and activity lifetime.
#[derive(Resource, Default)]
pub struct LoadPresentations {
    pub(crate) entries: BTreeMap<LoadPresentationOwnerId, LoadPresentation>,
    pub(crate) next_activity_activation: u64,
}

pub struct LoadPresentation {
    pub foreground: ActiveLoadForeground,
    pub model: LoadPresentationModel,
    pub activity: Option<ActiveLoadActivity>,
    pub last_outcome: Option<LoadActivityOutcome>,
}

impl LoadPresentations {
    pub fn get(&self, owner: &LoadPresentationOwnerId) -> Option<&LoadPresentation> {
        self.entries.get(owner)
    }

    pub fn iter(&self) -> impl Iterator<Item = &LoadPresentation> {
        self.entries.values()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn begin(&mut self, foreground: ActiveLoadForeground) -> &mut LoadPresentation {
        let owner = foreground.owner.clone();
        self.entries.insert(owner.clone(), LoadPresentation {
            foreground, model: LoadPresentationModel::default(), activity: None, last_outcome: None,
        });
        self.entries.get_mut(&owner).unwrap()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadPresentationModel {
    pub visible: bool,
    pub ready_hold: bool,
    pub owner: Option<LoadPresentationOwnerId>,
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
            owner: None,
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
    pub fn from_snapshot(snapshot: LoadBarrierSnapshot, foreground: &ActiveLoadForeground) -> Self {
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
            ready_hold: foreground.should_hold_ready(),
            owner: Some(foreground.owner.clone()),
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
    pub input_owner: Option<ambition_input::InputOwner>,
    pub activation_id: u64,
    pub activity_id: LoadActivityId,
    pub owner: LoadPresentationOwnerId,
    pub barrier: LoadBarrierRef,
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

/// Contributor-neutral presentation lifecycle commands.
///
/// The caller owns retry, cancellation, routing, and commit semantics. This
/// crate owns only the visible foreground and its ready-hold policy.
#[derive(Message, Clone, Debug, PartialEq)]
pub enum LoadPresentationCommand {
    Begin {
        owner: LoadPresentationOwnerId,
        barrier: LoadBarrierRef,
        spec: LoadExperienceSpec,
    },
    Finish {
        owner: LoadPresentationOwnerId,
    },
    Cancel {
        owner: LoadPresentationOwnerId,
    },
}

#[derive(Message, Clone, Debug, Eq, PartialEq)]
pub enum LoadPresentationAction {
    Continue { owner: LoadPresentationOwnerId },
    Retry { owner: LoadPresentationOwnerId },
    Cancel { owner: LoadPresentationOwnerId },
    Quit { owner: LoadPresentationOwnerId },
}

/// Semantic requests routed back to the owner of the active presentation.
///
/// The generic presentation cannot recreate a failed plan, choose a previous
/// destination, or quit to a host route. Shell and room-transition adapters
/// interpret these messages independently.
#[derive(Message, Clone, Debug, Eq, PartialEq)]
pub enum LoadPresentationEvent {
    ContinueRequested {
        owner: LoadPresentationOwnerId,
    },
    RetryRequested {
        owner: LoadPresentationOwnerId,
        barrier: LoadBarrierRef,
    },
    CancelRequested {
        owner: LoadPresentationOwnerId,
    },
    QuitRequested {
        owner: LoadPresentationOwnerId,
    },
}
