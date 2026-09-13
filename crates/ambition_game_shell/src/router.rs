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

    /// Every registered route id, in id order.
    ///
    /// Exists so a refusal can NAME what was available. This repo's binding
    /// boundary makes that the rule rather than a courtesy: "unknown route" is
    /// a puzzle, and "unknown route, here are the eight that exist" is a typo
    /// somebody can fix without a debugger.
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
    pub route_id: ShellRouteId,
    pub push_history: bool,
    pub barrier: LoadBarrierRef,
    pub requires_prepared_session: bool,
    pub terminal_reported: bool,
    /// Whose request started this route, so a transaction that ends WITHOUT
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

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShellScopedEntity {
    pub activation_id: ShellActivationId,
}

/// **WHICH CALLER REQUEST CAUSED THIS TRANSACTION** — minted by the caller,
/// before it writes the command.
///
/// ⛔⛤ **THE ROUTER'S `LoadId` ANSWERS A DIFFERENT QUESTION, AND A CALLER THAT
/// NEEDED THIS ONE WAS INFERRING IT FROM A ROUTE NAME.** `LoadId` is minted
/// INSIDE `start_route`, in a later system than the request, and
/// `ShellCommand::ReplaceWith` carried no slot for a correlator — so
/// `ambition_content::reload` adopted the first `PreparationRequested` whose
/// ROUTE matched its own, and its own comment said *"a route name is not a
/// transaction identity"* while using one.
///
/// ⇒ TWO `ReplaceWith("game")` QUEUED IN ONE FRAME mint `shell.game.N` and
/// `shell.game.N+1`, and the second SUPERSEDES the first. A caller adopting by
/// route can therefore take a transaction it did not issue, or one already
/// cancelled in the same frame — and then wait forever for an activation that
/// cannot come.
///
/// ⭐ **THIS IS GENERIC ON PURPOSE.** It answers "whose request was this" for any
/// caller; `LoadId` stays router-owned and answers "which load is this". Two
/// questions, two identities, neither inferring the other.
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
    /// Replace the active route. `request` is the CALLER's own correlation id,
    /// carried through to [`crate::ProviderLoadTransaction`] so the caller can
    /// recognise the transaction its own command produced.
    ///
    /// ⚠ `None` MEANS NOBODY IS CORRELATING, which is the right answer for
    /// ordinary navigation and the wrong one for a caller that must publish at
    /// the activation of ITS transaction. See [`ShellRequestId`].
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
    /// Cancel the pending transaction THIS request produced, because the
    /// condition it was authorized under no longer holds.
    ///
    /// ⛔⛤ **THE ONE THING A CORRELATED REQUESTER COULD NOT DO.** A caller that
    /// issues `ReplaceWith { request }` owns half of a two-halved transaction —
    /// the shell prepares and activates a route, and the caller publishes its own
    /// material at that activation. If the caller's half becomes ILLEGAL while
    /// the transaction is in flight, its only previous options were to publish
    /// anyway or to drop its half silently and let the route activate without
    /// it. Both are the split the whole road exists to prevent: a route at
    /// generation N+1 with content still at N.
    ///
    /// ⇒ **BREAKING THE AUTHORIZATION EARLY CANCELS BOTH HALVES**, which is what
    /// `Q118` asks for in as many words.
    ///
    /// ⛔ **IT NAMES A REQUEST, NOT A ROUTE, AND IT IS REFUSED IF THE PENDING
    /// TRANSACTION IS SOMEBODY ELSE'S.** Two generations can target one route —
    /// that is exactly what a reload does — so a route-matching cancel could tear
    /// down a transaction the caller did not issue. Same rule, and the same
    /// reason, as `PendingGenerationInputs`' load-id claim.
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
    /// carries the coordinator's well-worded [`LoadFailure`] reasons when
    /// `readiness` is [`BarrierReadiness::Failed`] (empty for cancellation and
    /// supersession, which carry no per-work failure) — without it a headless
    /// host only ever saw "Failed" and the underlying provider reason (e.g.
    /// "provider registered no explicit audio fragment") was discarded, so the
    /// route appeared to stall forever with no diagnosable cause.
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
    /// ⭐⭐ **A TRANSACTION ENDED WITHOUT ACTIVATING, AND THIS IS THE WORD THE
    /// VOCABULARY DID NOT HAVE.**
    ///
    /// ⛔⛤ **BEFORE THIS, SUPERSESSION EMITTED NOTHING ABOUT THE LOAD IT
    /// CANCELLED.** `start_route` takes `self.pending`, calls
    /// `prepared.cancel(&previous.barrier)` and returns events about the NEW
    /// route only; `PreparedSessionRegistry::cancel` is a `records.remove`
    /// returning `bool` — a state mutation, not an observable event. And of the
    /// seven other variants only `PreparationRequested` and `WaitingForLoad`
    /// carry a barrier, both naming the NEW one; `CommandRejected(LoadFailed
    /// {..})` carries a readiness and a failure list but no load id, and
    /// `ExperienceFailed` carries an activation id.
    ///
    /// ⇒ So a caller waiting on its own transaction had NOTHING to match on for
    /// the one terminal state that produces no error at all, and had to infer
    /// ownership from `ShellRouter.pending` — which by then names the SUPERSEDING
    /// route. A pending generation could be stranded with no signal, and every
    /// later save answered `AlreadyPending`.
    ///
    /// ⚠ IT NAMES BOTH IDENTITIES ON PURPOSE: the `barrier` for anything
    /// correlating on the router's load, and `request` for the caller that minted
    /// the command. Neither is derivable from the other.
    TransactionEnded {
        route_id: ShellRouteId,
        barrier: LoadBarrierRef,
        request: Option<ShellRequestId>,
        reason: TransactionEnd,
    },
}

/// Why a transaction ended without activating.
///
/// ⛔ SUPERSEDED IS NOT A FAILURE, and conflating them is what made the old
/// silence defensible: nothing went wrong, the caller simply asked for something
/// else first. A caller still has to hear about it, because its own generation is
/// waiting on a load that will never activate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransactionEnd {
    /// A PERSON cancelled the load, through the load-presentation screen's
    /// cancel or quit.
    ///
    /// ⛔⛤ **THE LAST TERMINAL ROAD THAT NAMED NOTHING.**
    /// `ShellRouter::cancel_pending` was a bare `self.pending.take()` returning
    /// the route for its caller's hold bookkeeping, so a player who cancelled a
    /// load while a content reload was in flight stranded that reload's
    /// generation — and because `ReloadRequest::AlreadyPending` refuses while one
    /// is in flight, every later save was refused too.
    ///
    /// ⚠ A SEPARATE REASON FROM `Superseded` ON PURPOSE: nobody asked for
    /// anything else, so a caller that retries on supersession must NOT retry
    /// here.
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
                // ⛔ ONLY THE ISSUER'S OWN TRANSACTION. A `None` request on the
                // pending transaction means nobody was correlating, so nobody
                // can be cancelling it by name either.
                match self.pending.as_ref().map(|pending| pending.request.clone()) {
                    Some(Some(pending)) if pending == request => {
                        self.cancel_pending(loads, prepared)
                    }
                    // ⚠ NOT AN ERROR AND NOT A REJECTION. A transaction that has
                    // already ended — activated, failed, superseded — is the
                    // ordinary race: the authorization broke on the same frame
                    // the transaction finished. Silence is the honest answer,
                    // and the caller's own half is discarded either way.
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

    /// Abandon the pending transaction and SAY SO.
    ///
    /// ⭐⭐ **IT RETURNS EVENTS RATHER THAN THE ROUTE BECAUSE A CALLER MUST NOT
    /// BE ABLE TO CANCEL SILENTLY.** The previous signature handed back
    /// `Option<PendingShellRoute>` and emitted nothing; both callers used it for
    /// `route_id` alone and neither could have known a correlating caller was
    /// waiting. Now the route id arrives INSIDE
    /// [`ShellEvent::TransactionEnded`], so the bookkeeping a caller wants and
    /// the signal a waiter needs are the same value — a caller cannot keep the
    /// first and drop the second.
    ///
    /// ⚠ A SMALLER STEP THAN IT COULD BE, DELIBERATELY. Routing cancellation
    /// through a `ShellCommand` would leave `apply`/`advance_pending` as the ONLY
    /// producers of `ShellEvent`, which is the better shape; it also defers the
    /// hold release and the presentation clear by a frame, and both are currently
    /// synchronous with the person's click. That collapse is its own change.
    /// End the pending transaction because a PERSON cancelled it: retire its
    /// preparation and its load authority, clear the router, and say so.
    ///
    /// ⛔⛤ **THIS USED TO BE `self.pending.take()` AND THE EVENT, AND NOTHING
    /// ELSE — SO "CANCELLED" DESCRIBED THE ROUTER AND NOT THE TRANSACTION.**
    /// The announcement half was correct and the lifecycle half was missing:
    /// the [`PreparedSessionRegistry`] record and the [`LoadCoordinator`] plan
    /// both survived a cancel, so
    ///
    /// * the abandoned provider work kept going and could still PUBLISH a
    ///   prepared session for a transaction the shell had already declared
    ///   ended — and with `self.pending` gone it could never activate, so that
    ///   record had no consumer at all;
    /// * `start → cancel` repeated accumulated resident preparation and load
    ///   authority, one abandoned record and one abandoned plan per cancel.
    ///
    /// ⭐⭐ **THE TELL WAS THE SIBLING.** `start_route`'s SUPERSESSION path
    /// already did the whole job — `prepared.cancel(..)` and
    /// `loads.retire(..)` — so the two terminal roads out of one pending
    /// transaction disagreed about what ending it means. ⇒ The two now do the
    /// same three things; only the `reason` differs, which is the one thing
    /// that really is different.
    ///
    /// ⚠ **`retire` RATHER THAN `LoadCommand::Cancel` THEN `retire`, AND THE
    /// REASON IS NOT TIDINESS.** `Cancel` flips the plan to
    /// `LoadPlanState::Cancelled` and returns a `PlanCancelled` event; retiring
    /// immediately afterwards removes the plan that flip just marked, and this
    /// caller would drop the event. A state change nobody can observe followed
    /// by a delete is ceremony, and supersession already established `retire`
    /// as the terminal operation. The observable announcement is the
    /// [`ShellEvent::TransactionEnded`] below.
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
                    // ⭐⭐ **THIS IS THE ROAD A PRODUCTION LOAD ACTUALLY DIES
                    // ON**, and until now it named nothing. `start_route`'s
                    // terminal check only fires for a barrier that is ALREADY
                    // terminal when the command arrives; a load that fails while
                    // the shell waits arrives here, one frame at a time. A
                    // correlating caller that watched only `start_route` would
                    // wait forever on the normal failure.
                    //
                    // ⛔ BOTH EVENTS, NOT ONE. `CommandRejected(LoadFailed)` is
                    // what the shell's own readers already handle and carries the
                    // per-failure list; `TransactionEnded` carries the identity
                    // and no detail. Collapsing them would either strand those
                    // readers or bloat the identity event into a second copy of
                    // the failure report.
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
        // ⛔ THREADED, NOT INVENTED HERE. The router cannot know whose request
        // this was; only the caller that wrote the command does.
        request: Option<ShellRequestId>,
        catalog: &ShellRouteCatalog,
        loads: &mut LoadCoordinator,
        prepared: &mut PreparedSessionRegistry,
    ) -> Vec<ShellEvent> {
        let Some(route) = catalog.get(&route_id) else {
            // ⚠ BEFORE `self.pending` IS TAKEN, so there is nothing superseded
            // to report: an unknown route cancels nothing.
            return vec![ShellEvent::CommandRejected(
                ShellCommandRejection::UnknownRoute(route_id),
            )];
        };

        let previous_pending = self.pending.take();
        let supersedes = previous_pending
            .as_ref()
            .map(|pending| pending.barrier.load_id.clone());
        // ⛔⛔ **SAY SO. THE CANCELLED TRANSACTION USED TO VANISH IN SILENCE.**
        // A caller waiting on it has a generation staged against a load that will
        // never activate, and no other variant names a cancelled barrier.
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
                    // ⛔ AND THE NEW TRANSACTION'S OWN TERMINAL STATE NAMES
                    // ITSELF TOO. `CommandRejected(LoadFailed { .. })` carries a
                    // readiness and a failure list and NO load id, so a caller
                    // correlating on its own request had nothing to match.
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
        events.extend(self.activate(route_id, push_history, catalog, None, None));
        events
    }

    fn activate(
        &mut self,
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
        self.next_activation = self.next_activation.saturating_add(1);
        let active = ActiveShellExperience {
            activation_id: ShellActivationId(self.next_activation),
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
