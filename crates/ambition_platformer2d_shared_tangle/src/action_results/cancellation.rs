//! Cancellation follows the native domain's declared suppression policy.

use super::*;

#[derive(Clone, Copy)]
pub enum ActionCancellationPolicy {
    Unsupported,
    /// Each consumer checks the pending ticket immediately before mutation.
    PendingTicket,
    /// The native owner suppresses all remaining work before it returns true.
    Handler(fn(&mut World, &ActionReply) -> bool),
}

/// Immutable installation data. Domain queues and mailboxes own rollback state.
#[derive(Resource, Default)]
pub struct ActionCancellationPolicies(
    std::collections::BTreeMap<String, ActionCancellationPolicy>,
);

pub fn register_action_domain(app: &mut App, domain: &str, policy: ActionCancellationPolicy) {
    app.init_resource::<ActionCancellationPolicies>();
    let mut policies = app.world_mut().resource_mut::<ActionCancellationPolicies>();
    if let Some(old) = policies.0.insert(domain.into(), policy) {
        let same = match (old, policy) {
            (ActionCancellationPolicy::Unsupported, ActionCancellationPolicy::Unsupported)
            | (ActionCancellationPolicy::PendingTicket, ActionCancellationPolicy::PendingTicket) => true,
            (ActionCancellationPolicy::Handler(a), ActionCancellationPolicy::Handler(b)) => std::ptr::fn_addr_eq(a, b),
            _ => false,
        };
        assert!(same, "an action domain has two cancellation policies: {domain}");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionCancellationRefusal {
    UnknownTicket,
    AlreadyTerminal,
    UnknownDomain,
    Unsupported,
    ApplicationStarted,
    OwnerRefused,
}
impl ActionCancellationRefusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::UnknownTicket => "cancel_unknown_ticket",
            Self::AlreadyTerminal => "cancel_already_terminal",
            Self::UnknownDomain => "cancel_unknown_domain",
            Self::Unsupported => "cancel_unsupported",
            Self::ApplicationStarted => "cancel_application_started",
            Self::OwnerRefused => "cancel_owner_refused",
        }
    }
}

/// Success means that this action cannot cause a later native mutation.
pub fn cancel_action(world: &mut World, scope: Entity, ticket: &ActionTicket)
    -> Result<(), ActionCancellationRefusal> {
    use ActionCancellationPolicy as Policy;
    use ActionCancellationRefusal as Refusal;
    let mailbox = world.get::<ActionMailbox>(scope).ok_or(Refusal::UnknownTicket)?;
    let pending = mailbox.producers.iter().flat_map(|producer| &producer.pending)
        .find(|pending| &pending.ticket == ticket);
    let Some(pending) = pending else {
        return Err(if mailbox.journal.iter().any(|row| &row.result.ticket == ticket && row.result.terminal) {
            Refusal::AlreadyTerminal
        } else { Refusal::UnknownTicket });
    };
    let stage = pending.stage;
    let policy = world.get_resource::<ActionCancellationPolicies>()
        .and_then(|policies| policies.0.get(&ticket.domain)).copied().ok_or(Refusal::UnknownDomain)?;
    let recipient = ActionRecipient::of(world, scope).ok_or(Refusal::UnknownTicket)?;
    let reply = ActionReply { recipient, ticket: ticket.clone() };
    match policy {
        Policy::Unsupported => return Err(Refusal::Unsupported),
        Policy::PendingTicket => {
            if !matches!(stage, ActionStage::Submitted | ActionStage::Admitted) {
                return Err(Refusal::ApplicationStarted);
            }
        }
        Policy::Handler(handler) => {
            if !handler(world, &reply) { return Err(Refusal::OwnerRefused); }
        }
    }
    let published = publish_action_result(world, Some(&reply), ActionStage::Canceled, true,
        "caller_canceled", "", Vec::new());
    assert!(published, "a cancellation owner changed its recipient or ticket");
    Ok(())
}
