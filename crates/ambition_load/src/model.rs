//! Contributor-neutral load plans, work states, forecasts, and snapshots.

use std::collections::BTreeSet;
use std::ops::RangeInclusive;

use crate::{LoadBarrierId, LoadId, LoadWorkId};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LoadPriority {
    Immediate,
    High,
    #[default]
    Normal,
    Low,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActivationRequirement {
    RequiredFor(BTreeSet<LoadBarrierId>),
    Degradable,
    Speculative,
}

impl ActivationRequirement {
    pub fn required_for(barrier: impl Into<LoadBarrierId>) -> Self {
        Self::RequiredFor(BTreeSet::from([barrier.into()]))
    }

    pub fn barriers(&self) -> impl Iterator<Item = &LoadBarrierId> {
        let slice = match self {
            Self::RequiredFor(barriers) => Some(barriers),
            Self::Degradable | Self::Speculative => None,
        };
        slice.into_iter().flatten()
    }

    pub fn add_barrier(&mut self, barrier: LoadBarrierId) {
        match self {
            Self::RequiredFor(barriers) => {
                barriers.insert(barrier);
            }
            Self::Degradable | Self::Speculative => {
                *self = Self::RequiredFor(BTreeSet::from([barrier]));
            }
        }
    }

    pub fn is_required_for(&self, barrier: &LoadBarrierId) -> bool {
        matches!(self, Self::RequiredFor(barriers) if barriers.contains(barrier))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitProgress {
    pub completed: f32,
    pub total: f32,
}

impl UnitProgress {
    pub fn new(completed: f32, total: f32) -> Self {
        assert!(
            total.is_finite() && total > 0.0,
            "progress total must be positive"
        );
        assert!(completed.is_finite(), "progress completed must be finite");
        Self {
            completed: completed.clamp(0.0, total),
            total,
        }
    }

    pub fn fraction(self) -> f32 {
        (self.completed / self.total).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LoadWorkState {
    Planned,
    Running { progress: Option<UnitProgress> },
    Complete,
    Failed(LoadFailure),
    Cancelled,
    Skipped,
}

impl LoadWorkState {
    pub fn fraction(&self) -> f32 {
        match self {
            Self::Complete | Self::Skipped => 1.0,
            Self::Running {
                progress: Some(progress),
            } => progress.fraction(),
            Self::Planned
            | Self::Running { progress: None }
            | Self::Failed(_)
            | Self::Cancelled => 0.0,
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete | Self::Skipped)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadFailure {
    pub player_message: String,
    pub developer_detail: String,
    pub retryable: bool,
}

impl LoadFailure {
    pub fn new(player_message: impl Into<String>, developer_detail: impl Into<String>) -> Self {
        Self {
            player_message: player_message.into(),
            developer_detail: developer_detail.into(),
            retryable: false,
        }
    }

    pub fn retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EstimateConfidence {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DiscoveryForecast {
    pub additional_steps: Option<RangeInclusive<usize>>,
    pub additional_weight: Option<f32>,
    pub confidence: EstimateConfidence,
    pub provenance: String,
}

impl DiscoveryForecast {
    pub fn new(provenance: impl Into<String>) -> Self {
        Self {
            additional_steps: None,
            additional_weight: None,
            confidence: EstimateConfidence::Low,
            provenance: provenance.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadWorkSpec {
    pub id: LoadWorkId,
    pub label: String,
    pub requirement: ActivationRequirement,
    pub priority: LoadPriority,
    pub estimated_weight: Option<f32>,
}

impl LoadWorkSpec {
    pub fn required(
        id: impl Into<LoadWorkId>,
        label: impl Into<String>,
        barrier: impl Into<LoadBarrierId>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            requirement: ActivationRequirement::required_for(barrier),
            priority: LoadPriority::High,
            estimated_weight: None,
        }
    }

    pub fn streamable(id: impl Into<LoadWorkId>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            requirement: ActivationRequirement::Degradable,
            priority: LoadPriority::Normal,
            estimated_weight: None,
        }
    }

    pub fn speculative(id: impl Into<LoadWorkId>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            requirement: ActivationRequirement::Speculative,
            priority: LoadPriority::Low,
            estimated_weight: None,
        }
    }

    pub fn with_weight(mut self, weight: f32) -> Self {
        assert!(
            weight.is_finite() && weight > 0.0,
            "work weight must be positive"
        );
        self.estimated_weight = Some(weight);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadBarrierSpec {
    pub id: LoadBarrierId,
    pub label: String,
    pub discovery_open: bool,
    pub forecast: Option<DiscoveryForecast>,
}

impl LoadBarrierSpec {
    pub fn new(id: impl Into<LoadBarrierId>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            discovery_open: true,
            forecast: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadPlanSpec {
    pub id: LoadId,
    pub label: String,
    pub supersedes: Option<LoadId>,
}

impl LoadPlanSpec {
    pub fn new(id: impl Into<LoadId>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            supersedes: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoadPlanState {
    Active,
    Cancelled,
    Superseded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BarrierReadiness {
    Preparing,
    Ready,
    Failed,
    Cancelled,
    Superseded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EstimateBasis {
    EqualSteps,
    AuthoredWeights,
    MixedWeights,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProgressEstimate {
    pub fraction: f32,
    pub confidence: EstimateConfidence,
    pub basis: EstimateBasis,
    pub provenance: String,
    pub may_decrease: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadBarrierSnapshot {
    pub load_id: LoadId,
    pub barrier_id: LoadBarrierId,
    pub label: String,
    pub readiness: BarrierReadiness,
    pub discovery_open: bool,
    pub completed_steps: usize,
    pub active_steps: usize,
    pub known_remaining_steps: usize,
    pub failed_steps: usize,
    pub cancelled_steps: usize,
    pub estimated_additional_steps: Option<RangeInclusive<usize>>,
    pub estimated_total_remaining_steps: Option<RangeInclusive<usize>>,
    pub completed_labels: Vec<String>,
    pub active_labels: Vec<String>,
    pub remaining_labels: Vec<String>,
    pub streamable_labels: Vec<String>,
    pub speculative_labels: Vec<String>,
    pub failures: Vec<LoadFailure>,
    pub estimate: Option<ProgressEstimate>,
}

impl LoadBarrierSnapshot {
    pub fn ready(&self) -> bool {
        self.readiness == BarrierReadiness::Ready
    }
}
