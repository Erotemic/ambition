//! Scoped replies from domain actions. Submission does not prove application.

use crate::wire::{WireError, WireReader};
use crate::{Port, PortKey, PortRole};

/// A caller supplies a new request number for each action in one producer.
/// The native adapter supplies the recipient. A script cannot redirect it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplyTo {
    pub request: u32,
    pub selector: String,
}

/// Native action admission data from the request's owning port.
pub struct ActionSubmission<'a> {
    pub domain: &'a str,
    pub reply: &'a ReplyTo,
    value: &'a (dyn std::any::Any + Send + Sync),
    fingerprint: fn(&(dyn std::any::Any + Send + Sync)) -> u64,
}
impl<'a> ActionSubmission<'a> {
    pub fn encoded<P: Port>(value: &'a P::Value, domain: &'a str, reply: &'a ReplyTo) -> Self {
        fn fingerprint<P: Port>(value: &(dyn std::any::Any + Send + Sync)) -> u64 {
            let mut bytes = Vec::new();
            P::encode(value.downcast_ref::<P::Value>().expect("the port's value type is unique"), &mut bytes);
            crate::digest::digest_bytes(&bytes)
        }
        Self {domain, reply, value, fingerprint: fingerprint::<P>}
    }
    /// Hash only when the native owner reserves or refuses the request.
    pub fn signature(&self) -> u64 { (self.fingerprint)(self.value) }
}

/// The host boundary that refused a correlated submission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionRefusal {
    InvalidReply,
    InvocationCost,
}

/// A request is scoped to its recipient, submitting entry, and native domain.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ActionTicket {
    pub entry: String,
    pub domain: String,
    pub request: u32,
    pub occurrence: u64,
    pub room: Option<u32>,
    pub generation: u64,
}

crate::port_choices! {
    /// The domain reports the stage it actually reached.
    pub enum ActionStage {
        Submitted = 0 => "submitted",
        Admitted = 1 => "admitted",
        Applied = 2 => "applied",
        Completed = 3 => "completed",
        Rejected = 4 => "rejected",
        Canceled = 5 => "canceled",
        Failed = 6 => "failed",
    }
}

/// A semantic domain key, with an exact live address when the subject is an object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionSubject {
    pub key: String,
    pub object: Option<crate::ObjectRef>,
}
impl ActionSubject {
    pub fn named(key: impl Into<String>) -> Self { Self {key: key.into(), object: None} }
}
crate::port_record! { ActionSubject { key: String, object: Option<crate::ObjectRef> } }

/// A native domain publishes this fact after its own admission or mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionResult {
    pub ticket: ActionTicket,
    pub stage: ActionStage,
    pub terminal: bool,
    pub tick: u64,
    pub code: String,
    pub message: String,
    pub subjects: Vec<ActionSubject>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionJournalEntry {
    pub sequence: u64,
    pub result: ActionResult,
    pub handler: String,
    pub delivery: ActionDelivery,
    pub attempts: u32,
    pub last_attempt: Option<u64>,
    pub last_error: Option<String>,
}

crate::port_choices! {
    /// Delivery progress is separate from the native action's result.
    pub enum ActionDelivery {
        Waiting = 0 => "waiting",
        Queued = 1 => "queued",
        Invoked = 2 => "invoked",
        Acknowledged = 3 => "acknowledged",
        Faulted = 4 => "faulted",
        Quarantined = 5 => "quarantined",
        Unbound = 6 => "unbound",
        Retired = 7 => "retired",
    }
}

/// A refused submission has no reserved ticket and cannot authorize work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionSubmissionRefusal {
    pub sequence: u64,
    pub entry: String,
    pub domain: String,
    pub request: u32,
    pub handler: String,
    pub signature: u64,
    pub tick: u64,
    pub code: String,
}
crate::port_record! {
    ActionSubmissionRefusal { sequence: u64, entry: String, domain: String, request: u32,
        handler: String, signature: u64, tick: u64, code: String }
}

/// Bounded retained history. `dropped` makes a delivery gap explicit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActionResults {
    pub next_sequence: u64,
    pub dropped: u64,
    pub entries: Vec<ActionJournalEntry>,
    pub refusals: Vec<ActionSubmissionRefusal>,
}

crate::port_record! {
    ReplyTo {
        /// A monotonic request number in the submitting entry and domain.
        request: u32,
        /// The action_result entry selector. Delivery stays in the caller's scope.
        selector: String,
    }
}
crate::port_record! {
    ActionTicket {
        /// The submitting provider::module/entry.
        entry: String,
        /// The native action domain.
        domain: String,
        request: u32,
        /// A stable occurrence in this recipient's action mailbox.
        occurrence: u64,
        /// Allocation room, or absent for a session-lifetime caller.
        room: Option<u32>,
        /// A caller lifetime within that room. Recreated bodies get a new value.
        generation: u64,
    }
}
crate::port_record! {
    ActionResult {
        ticket: ActionTicket,
        stage: ActionStage,
        /// The domain has finished this action. Applied can be terminal for a one-step mutation.
        terminal: bool,
        /// Simulation tick when the domain published this stage.
        tick: u64,
        /// A domain-defined result or refusal code.
        code: String,
        /// The domain's diagnostic, when it supplies one.
        message: String,
        /// Domain keys and exact occurrence references. No native entity handles.
        subjects: Vec<ActionSubject>,
    }
}
crate::port_record! {
    ActionJournalEntry {
        sequence: u64,
        result: ActionResult,
        handler: String,
        delivery: ActionDelivery,
        attempts: u32,
        last_attempt: Option<u64>,
        last_error: Option<String>,
    }
}
crate::port_record! {
    ActionResults {
        next_sequence: u64,
        dropped: u64,
        entries: Vec<ActionJournalEntry>,
        refusals: Vec<ActionSubmissionRefusal>,
    }
}

/// Cancellation is a separate correlated action. Its refusal leaves the target unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CancelAction {
    pub ticket: ActionTicket,
    pub reply: ReplyTo,
}
crate::port_record! { CancelAction { ticket: ActionTicket, reply: ReplyTo } }

pub struct CancelActionPort;
pub struct ActionResultPort;
pub struct ActionResultsPort;

impl Port for ActionResultPort {
    const KEY: PortKey = PortKey::new("ambition.actions.result", 2);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = ActionResult;
    fn encode(value: &Self::Value, out: &mut Vec<u8>) {
        crate::structure::put(value, out);
    }
    fn decode(r: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
        crate::structure::read(r)
    }
}
impl Port for ActionResultsPort {
    const KEY: PortKey = PortKey::new("ambition.actions.history", 4);
    const ROLE: PortRole = PortRole::Observation;
    type Value = ActionResults;
    fn encode(value: &Self::Value, out: &mut Vec<u8>) {
        crate::structure::put(value, out);
    }
    fn decode(r: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
        crate::structure::read(r)
    }
}
crate::script_port!(ActionResultPort);
crate::script_port!(ActionResultsPort);

/// Ask the native owner to cancel work in this scope and module. The request
/// has its own result. A refusal does not change the target action.
impl Port for CancelActionPort {
    const KEY: PortKey = PortKey::new("ambition.actions.cancel", 3);
    const ROLE: PortRole = PortRole::Request;
    type Value = CancelAction;
    fn action(value: &Self::Value) -> Option<crate::action::ActionSubmission<'_>> {
        Some(crate::action::ActionSubmission::encoded::<Self>(value, "actions.cancel", &value.reply))
    }
    fn encode(value: &Self::Value, out: &mut Vec<u8>) {
        crate::structure::put(value, out);
    }
    fn decode(r: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
        crate::structure::read(r)
    }
}
crate::script_port!(CancelActionPort);
