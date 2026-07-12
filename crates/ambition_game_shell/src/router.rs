//! Host-relative top-level route lifecycle, pending loads, focus, and scoped cleanup.

use std::collections::BTreeMap;

use ambition_load::{
    BarrierReadiness, LoadBarrierId, LoadCommitRejection, LoadCoordinator, LoadId,
};
use bevy::prelude::{Component, Message, Resource};

use crate::{ShellActivationId, ShellExperienceId, ShellHoldId, ShellRouteId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadBarrierRef {
    pub load_id: LoadId,
    pub barrier_id: LoadBarrierId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShellCompletionPolicy {
    Stay,
    GoTo(ShellRouteId),
    ReturnHome,
    ExitProcess,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShellRouteSpec {
    pub id: ShellRouteId,
    pub experience: ShellExperienceId,
    pub required_barrier: Option<LoadBarrierRef>,
    pub on_complete: ShellCompletionPolicy,
    pub parameters: BTreeMap<String, String>,
}

impl ShellRouteSpec {
    pub fn new(id: impl Into<ShellRouteId>, experience: impl Into<ShellExperienceId>) -> Self {
        Self {
            id: id.into(),
            experience: experience.into(),
            required_barrier: None,
            on_complete: ShellCompletionPolicy::Stay,
            parameters: BTreeMap::new(),
        }
    }

    pub fn requiring(mut self, load_id: LoadId, barrier_id: LoadBarrierId) -> Self {
        self.required_barrier = Some(LoadBarrierRef {
            load_id,
            barrier_id,
        });
        self
    }

    pub fn on_complete(mut self, policy: ShellCompletionPolicy) -> Self {
        self.on_complete = policy;
        self
    }
}

#[derive(Resource, Default)]
pub struct ShellRouteCatalog {
    routes: BTreeMap<ShellRouteId, ShellRouteSpec>,
}

impl ShellRouteCatalog {
    pub fn register(&mut self, spec: ShellRouteSpec) -> Option<ShellRouteSpec> {
        self.routes.insert(spec.id.clone(), spec)
    }

    pub fn get(&self, id: &ShellRouteId) -> Option<&ShellRouteSpec> {
        self.routes.get(id)
    }

    pub fn contains(&self, id: &ShellRouteId) -> bool {
        self.routes.contains_key(id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShellHostSpec {
    pub initial_route: ShellRouteId,
    pub home_route: ShellRouteId,
}

impl ShellHostSpec {
    pub fn new(
        initial_route: impl Into<ShellRouteId>,
        home_route: impl Into<ShellRouteId>,
    ) -> Self {
        Self {
            initial_route: initial_route.into(),
            home_route: home_route.into(),
        }
    }
}

#[derive(Resource, Default)]
pub struct ShellHostConfiguration {
    pub spec: Option<ShellHostSpec>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveShellExperience {
    pub activation_id: ShellActivationId,
    pub route_id: ShellRouteId,
    pub experience_id: ShellExperienceId,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingShellRoute {
    pub route_id: ShellRouteId,
    pub push_history: bool,
    pub barrier: LoadBarrierRef,
    pub terminal_reported: bool,
}

#[derive(Resource, Default)]
pub struct ShellRouter {
    pub active: Option<ActiveShellExperience>,
    pub pending: Option<PendingShellRoute>,
    pub history: Vec<ShellRouteId>,
    pub exit_requested: bool,
    initialized: bool,
    next_activation: u64,
}

#[derive(Resource, Default)]
pub struct ShellRouteHolds {
    holds: BTreeMap<ShellRouteId, std::collections::BTreeSet<ShellHoldId>>,
}

impl ShellRouteHolds {
    pub fn hold(&mut self, route_id: ShellRouteId, hold_id: ShellHoldId) {
        self.holds.entry(route_id).or_default().insert(hold_id);
    }

    pub fn release(&mut self, route_id: &ShellRouteId, hold_id: &ShellHoldId) {
        let remove_route = if let Some(holds) = self.holds.get_mut(route_id) {
            holds.remove(hold_id);
            holds.is_empty()
        } else {
            false
        };
        if remove_route {
            self.holds.remove(route_id);
        }
    }

    pub fn clear_route(&mut self, route_id: &ShellRouteId) {
        self.holds.remove(route_id);
    }

    pub fn is_held(&self, route_id: &ShellRouteId) -> bool {
        self.holds
            .get(route_id)
            .is_some_and(|holds| !holds.is_empty())
    }
}

#[derive(Resource, Default, Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShellInputFocus {
    pub activation_id: Option<ShellActivationId>,
}

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShellScopedEntity {
    pub activation_id: ShellActivationId,
}

#[derive(Message, Clone, Debug, Eq, PartialEq)]
pub enum ShellCommand {
    Initialize,
    GoTo(ShellRouteId),
    ReplaceWith(ShellRouteId),
    Return,
    QuitToHome,
    ExitProcess,
    ExperienceCompleted {
        activation_id: ShellActivationId,
    },
    ExperienceFailed {
        activation_id: ShellActivationId,
        message: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShellCommandRejection {
    HostNotConfigured,
    UnknownRoute(ShellRouteId),
    StaleActivation(ShellActivationId),
    LoadFailed(BarrierReadiness),
    LoadCommitRejected(LoadCommitRejection),
}

#[derive(Message, Clone, Debug, Eq, PartialEq)]
pub enum ShellEvent {
    WaitingForLoad {
        route_id: ShellRouteId,
        barrier: LoadBarrierRef,
    },
    RouteActivated(ActiveShellExperience),
    RouteDeactivated(ActiveShellExperience),
    ExperienceFailed {
        activation_id: ShellActivationId,
        message: String,
    },
    ExitRequested,
    CommandRejected(ShellCommandRejection),
}

impl ShellRouter {
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn apply(
        &mut self,
        command: ShellCommand,
        catalog: &ShellRouteCatalog,
        host: &ShellHostConfiguration,
        loads: &mut LoadCoordinator,
    ) -> Vec<ShellEvent> {
        match command {
            ShellCommand::Initialize => {
                if self.initialized {
                    Vec::new()
                } else if let Some(spec) = &host.spec {
                    if !catalog.contains(&spec.initial_route) {
                        return vec![ShellEvent::CommandRejected(
                            ShellCommandRejection::UnknownRoute(spec.initial_route.clone()),
                        )];
                    }
                    self.initialized = true;
                    self.start_route(spec.initial_route.clone(), false, catalog, loads)
                } else {
                    vec![ShellEvent::CommandRejected(
                        ShellCommandRejection::HostNotConfigured,
                    )]
                }
            }
            ShellCommand::GoTo(route) => self.start_route(route, true, catalog, loads),
            ShellCommand::ReplaceWith(route) => self.start_route(route, false, catalog, loads),
            ShellCommand::Return => {
                let route = self
                    .history
                    .pop()
                    .or_else(|| host.spec.as_ref().map(|spec| spec.home_route.clone()));
                match route {
                    Some(route) => self.start_route(route, false, catalog, loads),
                    None => vec![ShellEvent::CommandRejected(
                        ShellCommandRejection::HostNotConfigured,
                    )],
                }
            }
            ShellCommand::QuitToHome => match host.spec.as_ref() {
                Some(spec) => {
                    self.history.clear();
                    self.start_route(spec.home_route.clone(), false, catalog, loads)
                }
                None => vec![ShellEvent::CommandRejected(
                    ShellCommandRejection::HostNotConfigured,
                )],
            },
            ShellCommand::ExitProcess => {
                self.exit_requested = true;
                vec![ShellEvent::ExitRequested]
            }
            ShellCommand::ExperienceCompleted { activation_id } => {
                let Some(active) = self.active.as_ref() else {
                    return vec![ShellEvent::CommandRejected(
                        ShellCommandRejection::StaleActivation(activation_id),
                    )];
                };
                if active.activation_id != activation_id {
                    return vec![ShellEvent::CommandRejected(
                        ShellCommandRejection::StaleActivation(activation_id),
                    )];
                }
                let policy = catalog
                    .get(&active.route_id)
                    .map(|route| route.on_complete.clone())
                    .unwrap_or(ShellCompletionPolicy::Stay);
                match policy {
                    ShellCompletionPolicy::Stay => Vec::new(),
                    ShellCompletionPolicy::GoTo(route) => {
                        self.start_route(route, false, catalog, loads)
                    }
                    ShellCompletionPolicy::ReturnHome => {
                        self.apply(ShellCommand::QuitToHome, catalog, host, loads)
                    }
                    ShellCompletionPolicy::ExitProcess => {
                        self.apply(ShellCommand::ExitProcess, catalog, host, loads)
                    }
                }
            }
            ShellCommand::ExperienceFailed {
                activation_id,
                message,
            } => {
                if self
                    .active
                    .as_ref()
                    .is_some_and(|active| active.activation_id == activation_id)
                {
                    vec![ShellEvent::ExperienceFailed {
                        activation_id,
                        message,
                    }]
                } else {
                    vec![ShellEvent::CommandRejected(
                        ShellCommandRejection::StaleActivation(activation_id),
                    )]
                }
            }
        }
    }

    pub fn cancel_pending(&mut self) -> Option<PendingShellRoute> {
        self.pending.take()
    }

    pub fn advance_pending(
        &mut self,
        catalog: &ShellRouteCatalog,
        loads: &mut LoadCoordinator,
        holds: &ShellRouteHolds,
    ) -> Vec<ShellEvent> {
        let Some(pending) = self.pending.clone() else {
            return Vec::new();
        };
        if holds.is_held(&pending.route_id) {
            return Vec::new();
        }
        let readiness = loads
            .snapshot(&pending.barrier.load_id, &pending.barrier.barrier_id)
            .map(|snapshot| snapshot.readiness);
        match readiness {
            Some(BarrierReadiness::Ready) => {
                match loads.request_commit(&pending.barrier.load_id, &pending.barrier.barrier_id) {
                    Ok(()) => {
                        self.pending = None;
                        self.activate(pending.route_id, pending.push_history, catalog)
                    }
                    Err(reason) => vec![ShellEvent::CommandRejected(
                        ShellCommandRejection::LoadCommitRejected(reason),
                    )],
                }
            }
            Some(
                state @ (BarrierReadiness::Failed
                | BarrierReadiness::Cancelled
                | BarrierReadiness::Superseded),
            ) => {
                if pending.terminal_reported {
                    Vec::new()
                } else {
                    if let Some(current) = self.pending.as_mut() {
                        current.terminal_reported = true;
                    }
                    vec![ShellEvent::CommandRejected(
                        ShellCommandRejection::LoadFailed(state),
                    )]
                }
            }
            Some(BarrierReadiness::Preparing) | None => Vec::new(),
        }
    }

    fn start_route(
        &mut self,
        route_id: ShellRouteId,
        push_history: bool,
        catalog: &ShellRouteCatalog,
        loads: &mut LoadCoordinator,
    ) -> Vec<ShellEvent> {
        let Some(route) = catalog.get(&route_id) else {
            return vec![ShellEvent::CommandRejected(
                ShellCommandRejection::UnknownRoute(route_id),
            )];
        };
        if let Some(barrier) = route.required_barrier.clone() {
            let readiness = loads
                .snapshot(&barrier.load_id, &barrier.barrier_id)
                .map(|snapshot| snapshot.readiness);
            match readiness {
                Some(BarrierReadiness::Ready) => {
                    if let Err(reason) = loads.request_commit(&barrier.load_id, &barrier.barrier_id)
                    {
                        return vec![ShellEvent::CommandRejected(
                            ShellCommandRejection::LoadCommitRejected(reason),
                        )];
                    }
                }
                Some(
                    state @ (BarrierReadiness::Failed
                    | BarrierReadiness::Cancelled
                    | BarrierReadiness::Superseded),
                ) => {
                    self.pending = Some(PendingShellRoute {
                        route_id: route_id.clone(),
                        push_history,
                        barrier: barrier.clone(),
                        terminal_reported: true,
                    });
                    return vec![
                        ShellEvent::WaitingForLoad { route_id, barrier },
                        ShellEvent::CommandRejected(ShellCommandRejection::LoadFailed(state)),
                    ];
                }
                Some(BarrierReadiness::Preparing) | None => {
                    self.pending = Some(PendingShellRoute {
                        route_id: route_id.clone(),
                        push_history,
                        barrier: barrier.clone(),
                        terminal_reported: false,
                    });
                    return vec![ShellEvent::WaitingForLoad { route_id, barrier }];
                }
            }
        }
        self.activate(route_id, push_history, catalog)
    }

    fn activate(
        &mut self,
        route_id: ShellRouteId,
        push_history: bool,
        catalog: &ShellRouteCatalog,
    ) -> Vec<ShellEvent> {
        let route = catalog
            .get(&route_id)
            .expect("route was checked before activation");
        let mut events = Vec::new();
        if let Some(old) = self.active.take() {
            if push_history {
                self.history.push(old.route_id.clone());
            }
            events.push(ShellEvent::RouteDeactivated(old));
        }
        self.next_activation = self.next_activation.saturating_add(1);
        let active = ActiveShellExperience {
            activation_id: ShellActivationId(self.next_activation),
            route_id,
            experience_id: route.experience.clone(),
            parameters: route.parameters.clone(),
        };
        self.active = Some(active.clone());
        self.pending = None;
        events.push(ShellEvent::RouteActivated(active));
        events
    }
}
