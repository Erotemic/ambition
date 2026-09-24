//! Host-relative top-level route lifecycle, pending loads, focus, and scoped cleanup.

use std::collections::BTreeMap;

use ambition_load::{
    BarrierReadiness, LoadBarrierId, LoadBarrierRef, LoadCommitRejection, LoadCoordinator,
    LoadFailure, LoadId,
};
use bevy::prelude::{Component, Message, Resource};

use crate::{
    PreparedSessionIdentity, PreparedSessionRegistry, ProviderLoadTransaction,
    ProviderPreparationPlan, ShellActivationId, ShellExperienceId, ShellHoldId, ShellRouteId,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShellCompletionPolicy {
    Stay,
    GoTo(ShellRouteId),
    ReturnHome,
    ExitProcess,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShellRouteSpec {
    pub id: ShellRouteId,
    pub experience: ShellExperienceId,
    pub required_barrier: Option<LoadBarrierRef>,
    /// Fresh provider-authored work minted for every route request.
    pub preparation: Option<ProviderPreparationPlan>,
    pub on_complete: ShellCompletionPolicy,
    pub parameters: BTreeMap<String, String>,
}

impl ShellRouteSpec {
    pub fn new(id: impl Into<ShellRouteId>, experience: impl Into<ShellExperienceId>) -> Self {
        Self {
            id: id.into(),
            experience: experience.into(),
            required_barrier: None,
            preparation: None,
            on_complete: ShellCompletionPolicy::Stay,
            parameters: BTreeMap::new(),
        }
    }

    pub fn requiring(mut self, load_id: LoadId, barrier_id: LoadBarrierId) -> Self {
        self.required_barrier = Some(LoadBarrierRef::new(load_id, barrier_id));
        self
    }

    pub fn preparing_with(mut self, plan: ProviderPreparationPlan) -> Self {
        self.preparation = Some(plan);
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

    /// Every registered route id, in id order. A refusal uses this to list the
    /// routes that exist.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.routes.keys().map(ShellRouteId::as_str)
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
    pub load_authorization: Option<LoadBarrierRef>,
    pub prepared_session: Option<PreparedSessionIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingShellRoute {
    /// The activation id, reserved when the route goes pending, so a
    /// participant can prepare material (e.g. a candidate session) for an
    /// activation before it happens.
    ///
    /// It is used for host-local correlation only. The candidate slot is keyed
    /// on it: `pending_activation() != candidate.activation_id` finds an
    /// abandoned candidate, and a match reuses a prepared one. It is not part of
    /// the session root `SimId`.
    ///
    /// A pending route that never activates uses up an id. Gaps are legal.
    pub reserved_activation: ShellActivationId,
    pub route_id: ShellRouteId,
    pub push_history: bool,
    pub barrier: LoadBarrierRef,
    pub requires_prepared_session: bool,
    pub terminal_reported: bool,
    /// Whose request started this route, so a transaction that ends without
    /// activating can name its owner. See [`ShellEvent::TransactionEnded`].
    pub request: Option<ShellRequestId>,
}

#[derive(Resource, Default)]
pub struct ShellRouter {
    pub active: Option<ActiveShellExperience>,
    pub pending: Option<PendingShellRoute>,
    pub history: Vec<ShellRouteId>,
    pub exit_requested: bool,
    initialized: bool,
    next_activation: u64,
    next_load_transaction: u64,
}

#[derive(Resource, Default)]
pub struct ShellRouteHolds {
    holds: BTreeMap<ShellRouteId, std::collections::BTreeSet<ShellHoldId>>,
}

/// What an activation gate answers when the router asks it.
///
/// The router asks inside the activation, to avoid a check-then-act race
/// (`Q118`): a gate never releases itself. [`ShellActivationGates`] evaluation
/// and the activation that follows run in one exclusive operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellGateVerdict {
    /// Not yet — stay pending and ask again next time.
    Hold,
    /// Go ahead. The hold is consumed by this activation and by nothing else.
    Admit,
    /// This transaction must not activate at all: cancel it.
    Refuse,
}

/// The prerequisites a pending shell transaction must satisfy at activation.
///
/// The shell stays generic: a gate is a registered system that returns a
/// [`ShellGateVerdict`]. Rollback publication is the first user.
///
/// Keyed by hold id, not by route. A transaction-specific hold id
/// (`"content-publication:<request-id>"`) keeps a stale transaction's cleanup
/// from releasing its successor's hold on the same route.
#[derive(Resource, Default)]
pub struct ShellActivationGates {
    gates: BTreeMap<ShellHoldId, bevy::ecs::system::SystemId<(), ShellGateVerdict>>,
}

impl ShellActivationGates {
    /// Register the evaluator for a hold id, replacing any previous one.
    pub fn register(
        &mut self,
        hold_id: ShellHoldId,
        system: bevy::ecs::system::SystemId<(), ShellGateVerdict>,
    ) {
        self.gates.insert(hold_id, system);
    }

    pub fn evaluator(
        &self,
        hold_id: &ShellHoldId,
    ) -> Option<bevy::ecs::system::SystemId<(), ShellGateVerdict>> {
        self.gates.get(hold_id).copied()
    }

    pub fn forget(&mut self, hold_id: &ShellHoldId) {
        self.gates.remove(hold_id);
    }
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

    /// This route's holds, in id order — what the activation gate iterates.
    pub fn held(&self, route_id: &ShellRouteId) -> Vec<ShellHoldId> {
        self.holds
            .get(route_id)
            .map(|holds| holds.iter().cloned().collect())
            .unwrap_or_default()
    }
}

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShellScopedEntity {
    pub activation_id: ShellActivationId,
}

/// Which caller request caused this transaction. The caller mints it before
/// it writes the command.
///
/// `LoadId` is different: the router mints it later, inside `start_route`, and
/// it names the load. A route name is not a transaction identity: two
/// `ReplaceWith("game")` in one frame give two transactions, and the second
/// supersedes the first.
#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct ShellRequestId(String);

impl ShellRequestId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ShellRequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Message, Clone, Debug, Eq, PartialEq)]
pub enum ShellCommand {
    Initialize,
    GoTo(ShellRouteId),
    /// Replace the active route. `request` is the caller's correlation id,
    /// carried to [`crate::ProviderLoadTransaction`]. `None` means nobody
    /// correlates, which is correct for ordinary navigation. See
    /// [`ShellRequestId`].
    ReplaceWith {
        route: ShellRouteId,
        request: Option<ShellRequestId>,
    },
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
    /// Cancel the pending transaction this request produced, because the
    /// condition it was authorized under no longer holds.
    ///
    /// The caller owns half of the transaction (it publishes material at
    /// activation). Cancelling ends both halves, so a route never activates
    /// with stale content (`Q118`).
    ///
    /// It names a request, not a route, and does nothing if the pending
    /// transaction belongs to another request. A reload can target the same
    /// route twice. Same rule as `PendingGenerationInputs`' load-id claim.
    CancelPending {
        request: ShellRequestId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShellCommandRejection {
    HostNotConfigured,
    UnknownRoute(ShellRouteId),
    StaleActivation(ShellActivationId),
    /// A route's required load reached a terminal non-ready state. `failures`
    /// carries the [`LoadFailure`] reasons when `readiness` is
    /// [`BarrierReadiness::Failed`], and is empty for cancellation and
    /// supersession.
    LoadFailed {
        readiness: BarrierReadiness,
        failures: Vec<LoadFailure>,
    },
    LoadCommitRejected(LoadCommitRejection),
    PreparedSessionUnavailable(LoadBarrierRef),
}

#[derive(Message, Clone, Debug, Eq, PartialEq)]
pub enum ShellEvent {
    PreparationRequested(ProviderLoadTransaction),
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
    /// A transaction ended without activating.
    ///
    /// This is the only event that names the ended transaction's barrier and
    /// request. Other events name the new route, or carry no load id. Without
    /// it, a caller waiting on a superseded or cancelled transaction gets no
    /// signal, and its pending generation is stranded.
    ///
    /// It carries both identities: `barrier` for code that correlates on the
    /// router's load, and `request` for the caller that minted the command.
    TransactionEnded {
        route_id: ShellRouteId,
        barrier: LoadBarrierRef,
        request: Option<ShellRequestId>,
        reason: TransactionEnd,
    },
}

/// Why a transaction ended without activating.
///
/// Superseded is not a failure, but the caller must still hear about it: its
/// generation waits on a load that will never activate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransactionEnd {
    /// A person cancelled the load (the load screen's cancel or quit), or the
    /// requester sent `ShellCommand::CancelPending`.
    ///
    /// Separate from `Superseded`: nothing else was requested, so a caller that
    /// retries on supersession must not retry here.
    Cancelled,
    /// Another `start_route` began while this one was still pending.
    Superseded,
    /// Its barrier reached a terminal non-ready state.
    Failed,
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
        prepared: &mut PreparedSessionRegistry,
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
                    self.start_route(
                        spec.initial_route.clone(),
                        false,
                        None,
                        catalog,
                        loads,
                        prepared,
                    )
                } else {
                    vec![ShellEvent::CommandRejected(
                        ShellCommandRejection::HostNotConfigured,
                    )]
                }
            }
            ShellCommand::GoTo(route) => {
                self.start_route(route, true, None, catalog, loads, prepared)
            }
            ShellCommand::ReplaceWith { route, request } => {
                self.start_route(route, false, request, catalog, loads, prepared)
            }
            ShellCommand::CancelPending { request } => {
                // Only the issuer's own transaction. A pending transaction with
                // no request cannot be cancelled by name.
                match self.pending.as_ref().map(|pending| pending.request.clone()) {
                    Some(Some(pending)) if pending == request => {
                        self.cancel_pending(loads, prepared)
                    }
                    // Not an error: the transaction already ended in the same
                    // frame. The caller discards its own half either way.
                    _ => Vec::new(),
                }
            }
            ShellCommand::Return => {
                let route = self
                    .history
                    .pop()
                    .or_else(|| host.spec.as_ref().map(|spec| spec.home_route.clone()));
                match route {
                    Some(route) => self.start_route(route, false, None, catalog, loads, prepared),
                    None => vec![ShellEvent::CommandRejected(
                        ShellCommandRejection::HostNotConfigured,
                    )],
                }
            }
            ShellCommand::QuitToHome => match host.spec.as_ref() {
                Some(spec) => {
                    self.history.clear();
                    self.start_route(
                        spec.home_route.clone(),
                        false,
                        None,
                        catalog,
                        loads,
                        prepared,
                    )
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
                        self.start_route(route, false, None, catalog, loads, prepared)
                    }
                    ShellCompletionPolicy::ReturnHome => {
                        self.apply(ShellCommand::QuitToHome, catalog, host, loads, prepared)
                    }
                    ShellCompletionPolicy::ExitProcess => {
                        self.apply(ShellCommand::ExitProcess, catalog, host, loads, prepared)
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

    /// End the pending transaction because it was cancelled: cancel its
    /// [`PreparedSessionRegistry`] record, retire its [`LoadCoordinator`] plan,
    /// clear the router, and return [`ShellEvent::TransactionEnded`].
    ///
    /// This does the same cleanup as supersession in `start_route`; only the
    /// `reason` differs. Without it, abandoned provider work could still publish
    /// a prepared session, and repeated cancels would leak records and plans.
    ///
    /// It returns events, not the route, so a caller cannot cancel silently.
    /// It calls `retire` without `LoadCommand::Cancel` first: the cancel event
    /// would be dropped, and `TransactionEnded` is the observable signal.
    pub fn cancel_pending(
        &mut self,
        loads: &mut LoadCoordinator,
        prepared: &mut PreparedSessionRegistry,
    ) -> Vec<ShellEvent> {
        self.pending
            .take()
            .map(|pending| {
                prepared.cancel(&pending.barrier);
                loads.retire(&pending.barrier.load_id);
                vec![ShellEvent::TransactionEnded {
                    route_id: pending.route_id,
                    barrier: pending.barrier,
                    request: pending.request,
                    reason: TransactionEnd::Cancelled,
                }]
            })
            .unwrap_or_default()
    }

    /// Would the pending route activate on this call if nothing were holding it?
    ///
    /// Ask the activation gates only when this is true. A gate's `Admit`
    /// consumes its hold, so an early ask would leave the route unheld until
    /// readiness (the `Q118` race).
    pub fn ready_but_for_holds(
        &self,
        loads: &LoadCoordinator,
        prepared: &PreparedSessionRegistry,
    ) -> bool {
        let Some(pending) = self.pending.as_ref() else {
            return false;
        };
        let readiness = loads
            .snapshot(&pending.barrier.load_id, &pending.barrier.barrier_id)
            .map(|snapshot| snapshot.readiness);
        if !matches!(readiness, Some(BarrierReadiness::Ready)) {
            return false;
        }
        !(pending.requires_prepared_session && prepared.prepared(&pending.barrier).is_none())
    }

    pub fn advance_pending(
        &mut self,
        catalog: &ShellRouteCatalog,
        loads: &mut LoadCoordinator,
        prepared: &mut PreparedSessionRegistry,
        holds: &ShellRouteHolds,
    ) -> Vec<ShellEvent> {
        let Some(pending) = self.pending.clone() else {
            return Vec::new();
        };
        let snapshot = loads.snapshot(&pending.barrier.load_id, &pending.barrier.barrier_id);
        let readiness = snapshot.as_ref().map(|snapshot| snapshot.readiness);

        // A hold delays activation but never suppresses a terminal barrier
        // result; failed/cancelled/superseded routes must still be reported.
        if holds.is_held(&pending.route_id)
            && !matches!(
                readiness,
                Some(
                    BarrierReadiness::Failed
                        | BarrierReadiness::Cancelled
                        | BarrierReadiness::Superseded
                )
            )
        {
            return Vec::new();
        }

        match readiness {
            Some(BarrierReadiness::Ready) => {
                if pending.requires_prepared_session
                    && prepared.prepared(&pending.barrier).is_none()
                {
                    return Vec::new();
                }
                match loads.request_commit(&pending.barrier.load_id, &pending.barrier.barrier_id) {
                    Ok(()) => {
                        let prepared_session = if pending.requires_prepared_session {
                            let Some(identity) = prepared.consume(&pending.barrier) else {
                                return vec![ShellEvent::CommandRejected(
                                    ShellCommandRejection::PreparedSessionUnavailable(
                                        pending.barrier,
                                    ),
                                )];
                            };
                            Some(identity)
                        } else {
                            None
                        };
                        self.pending = None;
                        self.activate(
                            // The id the pending route reserved.
                            pending.reserved_activation,
                            pending.route_id,
                            pending.push_history,
                            catalog,
                            Some(pending.barrier),
                            prepared_session,
                        )
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
                    let failures = snapshot
                        .map(|snapshot| snapshot.failures)
                        .unwrap_or_default();
                    // A load that fails while the shell waits ends here (the
                    // usual case). Send both events: `TransactionEnded` names
                    // the transaction, and `CommandRejected(LoadFailed)` carries
                    // the failure list that existing readers handle.
                    vec![
                        ShellEvent::TransactionEnded {
                            route_id: pending.route_id.clone(),
                            barrier: pending.barrier.clone(),
                            request: pending.request.clone(),
                            reason: TransactionEnd::Failed,
                        },
                        ShellEvent::CommandRejected(
                            ShellCommandRejection::LoadFailed {
                                readiness: state,
                                failures,
                            },
                        ),
                    ]
                }
            }
            Some(BarrierReadiness::Preparing) | None => Vec::new(),
        }
    }

    fn start_route(
        &mut self,
        route_id: ShellRouteId,
        push_history: bool,
        // Passed through from the command; the router cannot know it.
        request: Option<ShellRequestId>,
        catalog: &ShellRouteCatalog,
        loads: &mut LoadCoordinator,
        prepared: &mut PreparedSessionRegistry,
    ) -> Vec<ShellEvent> {
        let Some(route) = catalog.get(&route_id) else {
            // Before `self.pending` is taken: an unknown route cancels nothing.
            return vec![ShellEvent::CommandRejected(
                ShellCommandRejection::UnknownRoute(route_id),
            )];
        };

        // Reserve once, before the branches, so every branch uses the same id.
        // See `PendingShellRoute::reserved_activation`.
        let reserved = self.reserve_activation();
        let previous_pending = self.pending.take();
        let supersedes = previous_pending
            .as_ref()
            .map(|pending| pending.barrier.load_id.clone());
        // Announce the superseded transaction; its caller is waiting on it.
        let mut events: Vec<ShellEvent> = Vec::new();
        if let Some(previous) = previous_pending.as_ref() {
            prepared.cancel(&previous.barrier);
            events.push(ShellEvent::TransactionEnded {
                route_id: previous.route_id.clone(),
                barrier: previous.barrier.clone(),
                request: previous.request.clone(),
                reason: TransactionEnd::Superseded,
            });
        }

        if let Some(plan) = route.preparation.as_ref() {
            self.next_load_transaction = self.next_load_transaction.saturating_add(1);
            let load_id = LoadId::new(format!(
                "shell.{}.{}",
                route.id.as_str(),
                self.next_load_transaction,
            ));
            for command in plan.begin_commands(load_id.clone(), supersedes.clone()) {
                loads.apply(command);
            }
            if let Some(old_load) = supersedes.as_ref() {
                loads.retire(old_load);
            }
            let barrier = LoadBarrierRef::new(load_id, plan.barrier.id.clone());
            let transaction = ProviderLoadTransaction {
                request: request.clone(),
                route_id: route.id.clone(),
                experience_id: route.experience.clone(),
                barrier: barrier.clone(),
            };
            prepared.request(transaction.clone());
            self.pending = Some(PendingShellRoute {
                reserved_activation: reserved,
                request: request.clone(),
                route_id: route_id.clone(),
                push_history,
                barrier: barrier.clone(),
                requires_prepared_session: true,
                terminal_reported: false,
            });
            events.extend([
                ShellEvent::PreparationRequested(transaction),
                ShellEvent::WaitingForLoad { route_id, barrier },
            ]);
            return events;
        }

        if let Some(previous) = previous_pending {
            loads.apply(ambition_load::LoadCommand::Cancel {
                load_id: previous.barrier.load_id.clone(),
            });
            loads.retire(&previous.barrier.load_id);
        }

        if let Some(barrier) = route.required_barrier.clone() {
            let snapshot = loads.snapshot(&barrier.load_id, &barrier.barrier_id);
            let readiness = snapshot.as_ref().map(|snapshot| snapshot.readiness);
            match readiness {
                Some(BarrierReadiness::Ready) => {
                    if let Err(reason) = loads.request_commit(&barrier.load_id, &barrier.barrier_id)
                    {
                        events.push(ShellEvent::CommandRejected(
                            ShellCommandRejection::LoadCommitRejected(reason),
                        ));
                        return events;
                    }
                    events.extend(self.activate(
                        reserved,
                        route_id,
                        push_history,
                        catalog,
                        Some(barrier),
                        None,
                    ));
                    return events;
                }
                Some(
                    state @ (BarrierReadiness::Failed
                    | BarrierReadiness::Cancelled
                    | BarrierReadiness::Superseded),
                ) => {
                    self.pending = Some(PendingShellRoute {
                        reserved_activation: reserved,
                        request: request.clone(),
                        route_id: route_id.clone(),
                        push_history,
                        barrier: barrier.clone(),
                        requires_prepared_session: false,
                        terminal_reported: true,
                    });
                    let failures = snapshot
                        .map(|snapshot| snapshot.failures)
                        .unwrap_or_default();
                    // Also send `TransactionEnded`: `LoadFailed` has no load id
                    // for a correlating caller to match.
                    events.extend([
                        ShellEvent::WaitingForLoad {
                            route_id: route_id.clone(),
                            barrier: barrier.clone(),
                        },
                        ShellEvent::TransactionEnded {
                            route_id,
                            barrier,
                            request,
                            reason: TransactionEnd::Failed,
                        },
                        ShellEvent::CommandRejected(ShellCommandRejection::LoadFailed {
                            readiness: state,
                            failures,
                        }),
                    ]);
                    return events;
                }
                Some(BarrierReadiness::Preparing) | None => {
                    self.pending = Some(PendingShellRoute {
                        reserved_activation: reserved,
                        request: request.clone(),
                        route_id: route_id.clone(),
                        push_history,
                        barrier: barrier.clone(),
                        requires_prepared_session: false,
                        terminal_reported: false,
                    });
                    events.push(ShellEvent::WaitingForLoad { route_id, barrier });
                    return events;
                }
            }
        }
        events.extend(self.activate(reserved, route_id, push_history, catalog, None, None));
        events
    }

    /// Reserve the identity the next pending route will activate under.
    ///
    /// See [`PendingShellRoute::reserved_activation`].
    fn reserve_activation(&mut self) -> ShellActivationId {
        self.next_activation = self.next_activation.saturating_add(1);
        ShellActivationId(self.next_activation)
    }

    /// The identity the pending route will activate under, if one is pending.
    pub fn pending_activation(&self) -> Option<ShellActivationId> {
        self.pending
            .as_ref()
            .map(|pending| pending.reserved_activation)
    }

    fn activate(
        &mut self,
        // Passed in, not read from `self.pending`: `advance_pending` clears
        // `self.pending` before it calls this.
        activation_id: ShellActivationId,
        route_id: ShellRouteId,
        push_history: bool,
        catalog: &ShellRouteCatalog,
        load_authorization: Option<LoadBarrierRef>,
        prepared_session: Option<PreparedSessionIdentity>,
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
        let active = ActiveShellExperience {
            activation_id,
            route_id,
            experience_id: route.experience.clone(),
            parameters: route.parameters.clone(),
            load_authorization,
            prepared_session,
        };
        self.active = Some(active.clone());
        self.pending = None;
        events.push(ShellEvent::RouteActivated(active));
        events
    }
}
