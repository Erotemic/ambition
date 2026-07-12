//! Deterministic in-memory load coordination and barrier derivation.

use std::collections::BTreeMap;

use bevy::prelude::Resource;

use crate::{
    BarrierReadiness, DiscoveryForecast, EstimateBasis, LoadBarrierId, LoadBarrierSnapshot,
    LoadBarrierSpec, LoadCommand, LoadCommitRejection, LoadEvent, LoadId, LoadPlanSpec,
    LoadPlanState, LoadWorkId, LoadWorkSpec, LoadWorkState, ProgressEstimate,
};

#[derive(Clone, Debug)]
struct WorkRecord {
    spec: LoadWorkSpec,
    state: LoadWorkState,
}

#[derive(Clone, Debug)]
struct BarrierRecord {
    spec: LoadBarrierSpec,
    commit_authorized: bool,
}

#[derive(Clone, Debug)]
struct PlanRecord {
    spec: LoadPlanSpec,
    state: LoadPlanState,
    barriers: BTreeMap<LoadBarrierId, BarrierRecord>,
    work: BTreeMap<LoadWorkId, WorkRecord>,
}

#[derive(Resource, Default)]
pub struct LoadCoordinator {
    plans: BTreeMap<LoadId, PlanRecord>,
}

impl LoadCoordinator {
    pub fn contains(&self, load: &LoadId) -> bool {
        self.plans.contains_key(load)
    }

    pub fn plan_state(&self, load: &LoadId) -> Option<LoadPlanState> {
        self.plans.get(load).map(|plan| plan.state)
    }

    pub fn apply(&mut self, command: LoadCommand) -> Vec<LoadEvent> {
        let mut events = Vec::new();
        match command {
            LoadCommand::Begin(spec) => {
                if let Some(old) = spec.supersedes.clone() {
                    if let Some(plan) = self.plans.get_mut(&old) {
                        plan.state = LoadPlanState::Superseded;
                        events.push(LoadEvent::PlanSuperseded {
                            load_id: old,
                            replacement: spec.id.clone(),
                        });
                    }
                }
                let id = spec.id.clone();
                self.plans.insert(
                    id.clone(),
                    PlanRecord {
                        spec,
                        state: LoadPlanState::Active,
                        barriers: BTreeMap::new(),
                        work: BTreeMap::new(),
                    },
                );
                events.push(LoadEvent::PlanChanged { load_id: id });
            }
            LoadCommand::DeclareBarrier { load_id, spec } => {
                if let Some(plan) = self.active_plan_mut(&load_id) {
                    plan.barriers.insert(
                        spec.id.clone(),
                        BarrierRecord {
                            spec,
                            commit_authorized: false,
                        },
                    );
                    events.push(LoadEvent::PlanChanged { load_id });
                }
            }
            LoadCommand::SetDiscovery {
                load_id,
                barrier_id,
                open,
                forecast,
            } => {
                if let Some(barrier) = self
                    .active_plan_mut(&load_id)
                    .and_then(|plan| plan.barriers.get_mut(&barrier_id))
                {
                    barrier.spec.discovery_open = open;
                    barrier.spec.forecast = forecast;
                    events.push(LoadEvent::PlanChanged { load_id });
                }
            }
            LoadCommand::UpsertWork { load_id, spec } => {
                if let Some(plan) = self.active_plan_mut(&load_id) {
                    let state = plan
                        .work
                        .get(&spec.id)
                        .map(|record| record.state.clone())
                        .unwrap_or(LoadWorkState::Planned);
                    plan.work
                        .insert(spec.id.clone(), WorkRecord { spec, state });
                    events.push(LoadEvent::PlanChanged { load_id });
                }
            }
            LoadCommand::SetWorkState {
                load_id,
                work_id,
                state,
            } => {
                if let Some(work) = self
                    .active_plan_mut(&load_id)
                    .and_then(|plan| plan.work.get_mut(&work_id))
                {
                    work.state = state;
                    events.push(LoadEvent::PlanChanged { load_id });
                }
            }
            LoadCommand::RemoveWork { load_id, work_id } => {
                if let Some(plan) = self.active_plan_mut(&load_id) {
                    if plan.work.remove(&work_id).is_some() {
                        events.push(LoadEvent::PlanChanged { load_id });
                    }
                }
            }
            LoadCommand::SetWorkPriority {
                load_id,
                work_id,
                priority,
            } => {
                if let Some(work) = self
                    .active_plan_mut(&load_id)
                    .and_then(|plan| plan.work.get_mut(&work_id))
                {
                    work.spec.priority = priority;
                    events.push(LoadEvent::PlanChanged { load_id });
                }
            }
            LoadCommand::PromoteWork {
                load_id,
                work_id,
                barrier_id,
            } => {
                if let Some(work) = self
                    .active_plan_mut(&load_id)
                    .and_then(|plan| plan.work.get_mut(&work_id))
                {
                    work.spec.requirement.add_barrier(barrier_id);
                    events.push(LoadEvent::PlanChanged { load_id });
                }
            }
            LoadCommand::Cancel { load_id } => {
                if let Some(plan) = self.plans.get_mut(&load_id) {
                    plan.state = LoadPlanState::Cancelled;
                    events.push(LoadEvent::PlanCancelled { load_id });
                }
            }
            LoadCommand::RequestCommit {
                load_id,
                barrier_id,
            } => match self.request_commit(&load_id, &barrier_id) {
                Ok(()) => events.push(LoadEvent::CommitAuthorized {
                    load_id,
                    barrier_id,
                }),
                Err(reason) => events.push(LoadEvent::CommitRejected {
                    load_id,
                    barrier_id,
                    reason,
                }),
            },
        }
        events
    }

    /// Authorize the single atomic activation associated with a ready barrier.
    ///
    /// Shells and headless hosts call this at the final handoff point. Merely
    /// observing `Ready` is not authorization: cancellation, supersession, and
    /// duplicate activation are rejected here.
    pub fn request_commit(
        &mut self,
        load_id: &LoadId,
        barrier_id: &LoadBarrierId,
    ) -> Result<(), LoadCommitRejection> {
        let snapshot = self
            .snapshot(load_id, barrier_id)
            .ok_or(LoadCommitRejection::UnknownBarrier)?;
        if !snapshot.ready() {
            return Err(LoadCommitRejection::BarrierNotReady(snapshot.readiness));
        }
        let barrier = self
            .plans
            .get_mut(load_id)
            .and_then(|plan| plan.barriers.get_mut(barrier_id))
            .ok_or(LoadCommitRejection::UnknownBarrier)?;
        if barrier.commit_authorized {
            return Err(LoadCommitRejection::AlreadyAuthorized);
        }
        barrier.commit_authorized = true;
        Ok(())
    }

    pub fn snapshot(
        &self,
        load_id: &LoadId,
        barrier_id: &LoadBarrierId,
    ) -> Option<LoadBarrierSnapshot> {
        let plan = self.plans.get(load_id)?;
        let barrier = plan.barriers.get(barrier_id)?;
        let required: Vec<_> = plan
            .work
            .values()
            .filter(|record| record.spec.requirement.is_required_for(barrier_id))
            .collect();

        let completed_steps = required
            .iter()
            .filter(|work| work.state.is_complete())
            .count();
        let active_steps = required
            .iter()
            .filter(|work| matches!(&work.state, LoadWorkState::Running { .. }))
            .count();
        let known_remaining_steps = required
            .iter()
            .filter(|work| {
                !work.state.is_complete()
                    && !matches!(
                        &work.state,
                        LoadWorkState::Failed(_) | LoadWorkState::Cancelled
                    )
            })
            .count();
        let failures: Vec<_> = required
            .iter()
            .filter_map(|work| match &work.state {
                LoadWorkState::Failed(failure) => Some(failure.clone()),
                _ => None,
            })
            .collect();
        let failed_steps = failures.len();
        let cancelled_steps = required
            .iter()
            .filter(|work| matches!(&work.state, LoadWorkState::Cancelled))
            .count();

        let readiness = match plan.state {
            LoadPlanState::Cancelled => BarrierReadiness::Cancelled,
            LoadPlanState::Superseded => BarrierReadiness::Superseded,
            LoadPlanState::Active if failed_steps > 0 => BarrierReadiness::Failed,
            LoadPlanState::Active if cancelled_steps > 0 => BarrierReadiness::Cancelled,
            LoadPlanState::Active
                if !barrier.spec.discovery_open
                    && required.iter().all(|work| work.state.is_complete()) =>
            {
                BarrierReadiness::Ready
            }
            LoadPlanState::Active => BarrierReadiness::Preparing,
        };

        let estimated_additional_steps = barrier
            .spec
            .forecast
            .as_ref()
            .and_then(|forecast| forecast.additional_steps.clone());
        let estimated_total_remaining_steps = estimated_additional_steps.as_ref().map(|range| {
            range.start().saturating_add(known_remaining_steps)
                ..=range.end().saturating_add(known_remaining_steps)
        });

        Some(LoadBarrierSnapshot {
            load_id: load_id.clone(),
            barrier_id: barrier_id.clone(),
            label: barrier.spec.label.clone(),
            readiness,
            discovery_open: barrier.spec.discovery_open,
            completed_steps,
            active_steps,
            known_remaining_steps,
            failed_steps,
            cancelled_steps,
            estimated_additional_steps,
            estimated_total_remaining_steps,
            active_labels: required
                .iter()
                .filter(|work| matches!(&work.state, LoadWorkState::Running { .. }))
                .map(|work| work.spec.label.clone())
                .collect(),
            remaining_labels: required
                .iter()
                .filter(|work| !work.state.is_complete())
                .map(|work| work.spec.label.clone())
                .collect(),
            failures,
            estimate: estimate_progress(
                &required,
                barrier.spec.discovery_open,
                barrier.spec.forecast.as_ref(),
            ),
        })
    }

    pub fn plan_label(&self, load_id: &LoadId) -> Option<&str> {
        self.plans.get(load_id).map(|plan| plan.spec.label.as_str())
    }

    fn active_plan_mut(&mut self, load_id: &LoadId) -> Option<&mut PlanRecord> {
        self.plans
            .get_mut(load_id)
            .filter(|plan| plan.state == LoadPlanState::Active)
    }
}

fn estimate_progress(
    required: &[&WorkRecord],
    discovery_open: bool,
    forecast: Option<&DiscoveryForecast>,
) -> Option<ProgressEstimate> {
    if required.is_empty() && !discovery_open {
        return Some(ProgressEstimate {
            fraction: 1.0,
            confidence: crate::EstimateConfidence::High,
            basis: EstimateBasis::EqualSteps,
            provenance: "empty closed barrier".to_owned(),
            may_decrease: false,
        });
    }
    if discovery_open && forecast.and_then(|item| item.additional_weight).is_none() {
        return None;
    }

    let weighted = required
        .iter()
        .filter(|work| work.spec.estimated_weight.is_some())
        .count();
    let basis = if weighted == 0 {
        EstimateBasis::EqualSteps
    } else if weighted == required.len() {
        EstimateBasis::AuthoredWeights
    } else {
        EstimateBasis::MixedWeights
    };
    let mut total_weight = 0.0;
    let mut completed_weight = 0.0;
    for work in required {
        let weight = work
            .spec
            .estimated_weight
            .filter(|weight| weight.is_finite() && *weight > 0.0)
            .unwrap_or(1.0);
        total_weight += weight;
        completed_weight += weight * work.state.fraction();
    }
    if let Some(additional) = forecast.and_then(|item| item.additional_weight) {
        if !additional.is_finite() || additional < 0.0 {
            return None;
        }
        total_weight += additional;
    }
    if total_weight <= f32::EPSILON {
        return None;
    }
    Some(ProgressEstimate {
        fraction: (completed_weight / total_weight).clamp(0.0, 1.0),
        confidence: forecast
            .map(|item| item.confidence)
            .unwrap_or(crate::EstimateConfidence::High),
        basis,
        provenance: forecast
            .map(|item| item.provenance.clone())
            .unwrap_or_else(|| "known required work".to_owned()),
        may_decrease: discovery_open,
    })
}
