//! Provider-authored fresh preparation plans and exact prepared-session identity.

use std::collections::BTreeMap;

use ambition_load::{
    DiscoveryForecast, LoadBarrierId, LoadBarrierSpec, LoadCommand, LoadFailure, LoadId,
    LoadPlanSpec, LoadPriority, LoadWorkId, LoadWorkSpec, LoadWorkState,
};
use bevy::prelude::Resource;

use crate::{LoadBarrierRef, ShellExperienceId, ShellRouteId};

pub const PREPARE_BARRIER_ID: &str = "activation-ready";
pub const PREPARE_CATALOGS_WORK_ID: &str = "validate-catalogs";
pub const PREPARE_WORLD_WORK_ID: &str = "prepare-world";
pub const PREPARE_SPRITES_WORK_ID: &str = "resolve-sprite-manifest";
pub const PREPARE_AUDIO_WORK_ID: &str = "validate-audio";
pub const PREPARE_SESSION_WORK_ID: &str = "publish-prepared-session";
pub const PREPARE_PACKED_SFX_WORK_ID: &str = "stream-packed-sfx";

pub fn standard_platformer_preparation_plan(
    label: impl Into<String>,
) -> ProviderPreparationPlan {
    ProviderPreparationPlan::new(label, PREPARE_BARRIER_ID, "Ready to play")
        .required(PREPARE_CATALOGS_WORK_ID, "Validate authored catalogs")
        .required(PREPARE_WORLD_WORK_ID, "Prepare world data")
        .required(PREPARE_SPRITES_WORK_ID, "Resolve sprite manifest")
        .required(PREPARE_AUDIO_WORK_ID, "Validate music and sound effects")
        .required(PREPARE_SESSION_WORK_ID, "Build prepared session")
        .streamable(PREPARE_PACKED_SFX_WORK_ID, "Stream packed sound bank")
}

pub fn standard_preparation_succeeded_commands(
    transaction: &ProviderLoadTransaction,
) -> Vec<LoadCommand> {
    let load_id = transaction.barrier.load_id.clone();
    let mut commands = [
        PREPARE_CATALOGS_WORK_ID,
        PREPARE_WORLD_WORK_ID,
        PREPARE_SPRITES_WORK_ID,
        PREPARE_AUDIO_WORK_ID,
        PREPARE_SESSION_WORK_ID,
    ]
    .into_iter()
    .map(|work_id| LoadCommand::SetWorkState {
        load_id: load_id.clone(),
        work_id: LoadWorkId::new(work_id),
        state: LoadWorkState::Complete,
    })
    .collect::<Vec<_>>();
    commands.push(LoadCommand::SetWorkState {
        load_id: load_id.clone(),
        work_id: LoadWorkId::new(PREPARE_PACKED_SFX_WORK_ID),
        state: LoadWorkState::Running { progress: None },
    });
    commands.push(LoadCommand::SetDiscovery {
        load_id,
        barrier_id: transaction.barrier.barrier_id.clone(),
        open: false,
        forecast: None,
    });
    commands
}

pub fn standard_preparation_failed_commands(
    transaction: &ProviderLoadTransaction,
    work_id: impl Into<LoadWorkId>,
    failure: LoadFailure,
) -> Vec<LoadCommand> {
    vec![
        LoadCommand::SetWorkState {
            load_id: transaction.barrier.load_id.clone(),
            work_id: work_id.into(),
            state: LoadWorkState::Failed(failure),
        },
        LoadCommand::SetDiscovery {
            load_id: transaction.barrier.load_id.clone(),
            barrier_id: transaction.barrier.barrier_id.clone(),
            open: false,
            forecast: None,
        },
    ]
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProviderPreparationPlan {
    pub label: String,
    pub barrier: LoadBarrierSpec,
    pub work: Vec<LoadWorkSpec>,
}

impl ProviderPreparationPlan {
    pub fn new(
        label: impl Into<String>,
        barrier_id: impl Into<LoadBarrierId>,
        barrier_label: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            barrier: LoadBarrierSpec::new(barrier_id, barrier_label),
            work: Vec::new(),
        }
    }

    pub fn required(mut self, id: impl Into<LoadWorkId>, label: impl Into<String>) -> Self {
        self.work
            .push(LoadWorkSpec::required(id, label, self.barrier.id.clone()));
        self
    }

    pub fn streamable(mut self, id: impl Into<LoadWorkId>, label: impl Into<String>) -> Self {
        self.work.push(LoadWorkSpec::streamable(id, label));
        self
    }

    pub fn speculative(mut self, id: impl Into<LoadWorkId>, label: impl Into<String>) -> Self {
        self.work.push(LoadWorkSpec::speculative(id, label));
        self
    }

    pub fn with_discovery_forecast(mut self, forecast: DiscoveryForecast) -> Self {
        self.barrier.forecast = Some(forecast);
        self
    }

    pub fn with_priority(mut self, id: &str, priority: LoadPriority) -> Self {
        if let Some(work) = self.work.iter_mut().find(|work| work.id.as_str() == id) {
            work.priority = priority;
        }
        self
    }

    pub(crate) fn begin_commands(
        &self,
        load_id: LoadId,
        supersedes: Option<LoadId>,
    ) -> Vec<LoadCommand> {
        let mut plan = LoadPlanSpec::new(load_id.clone(), self.label.clone());
        plan.supersedes = supersedes;
        let mut commands = vec![
            LoadCommand::Begin(plan),
            LoadCommand::DeclareBarrier {
                load_id: load_id.clone(),
                spec: self.barrier.clone(),
            },
        ];
        commands.extend(
            self.work
                .iter()
                .cloned()
                .map(|spec| LoadCommand::UpsertWork {
                    load_id: load_id.clone(),
                    spec,
                }),
        );
        commands
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderLoadTransaction {
    pub route_id: ShellRouteId,
    pub experience_id: ShellExperienceId,
    pub barrier: LoadBarrierRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedSessionIdentity {
    pub transaction: ProviderLoadTransaction,
    pub publication_id: u64,
}

#[derive(Clone, Debug)]
struct PreparationRecord {
    transaction: ProviderLoadTransaction,
    prepared: Option<PreparedSessionIdentity>,
    consumed: bool,
}

#[derive(Resource, Default)]
pub struct PreparedSessionRegistry {
    records: BTreeMap<LoadId, PreparationRecord>,
    next_publication: u64,
}

impl PreparedSessionRegistry {
    pub(crate) fn request(&mut self, transaction: ProviderLoadTransaction) {
        let load_id = transaction.barrier.load_id.clone();
        assert!(
            self.records
                .insert(
                    load_id.clone(),
                    PreparationRecord {
                        transaction,
                        prepared: None,
                        consumed: false,
                    },
                )
                .is_none(),
            "fresh provider load transaction {load_id} was reused",
        );
    }

    pub fn publish(
        &mut self,
        transaction: &ProviderLoadTransaction,
    ) -> Option<PreparedSessionIdentity> {
        let record = self.records.get_mut(&transaction.barrier.load_id)?;
        if record.transaction != *transaction || record.prepared.is_some() || record.consumed {
            return None;
        }
        self.next_publication = self.next_publication.saturating_add(1);
        let identity = PreparedSessionIdentity {
            transaction: transaction.clone(),
            publication_id: self.next_publication,
        };
        record.prepared = Some(identity.clone());
        Some(identity)
    }

    pub fn prepared(&self, barrier: &LoadBarrierRef) -> Option<&PreparedSessionIdentity> {
        let record = self.records.get(&barrier.load_id)?;
        if record.transaction.barrier != *barrier {
            return None;
        }
        record.prepared.as_ref()
    }

    pub(crate) fn consume(
        &mut self,
        barrier: &LoadBarrierRef,
    ) -> Option<PreparedSessionIdentity> {
        let record = self.records.get_mut(&barrier.load_id)?;
        if record.transaction.barrier != *barrier || record.consumed {
            return None;
        }
        let identity = record.prepared.clone()?;
        record.consumed = true;
        Some(identity)
    }

    pub fn retire_prepared(&mut self, identity: &PreparedSessionIdentity) -> bool {
        let load_id = &identity.transaction.barrier.load_id;
        let Some(record) = self.records.get(load_id) else {
            return false;
        };
        if !record.consumed
            || record.transaction != identity.transaction
            || record.prepared.as_ref() != Some(identity)
        {
            return false;
        }
        self.records.remove(load_id);
        true
    }

    pub fn cancel(&mut self, barrier: &LoadBarrierRef) -> bool {
        if !self.contains(barrier) {
            return false;
        }
        self.records.remove(&barrier.load_id).is_some()
    }

    pub fn contains(&self, barrier: &LoadBarrierRef) -> bool {
        self.records
            .get(&barrier.load_id)
            .is_some_and(|record| record.transaction.barrier == *barrier)
    }

    pub fn contains_load(&self, load_id: &LoadId) -> bool {
        self.records.contains_key(load_id)
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }
}
