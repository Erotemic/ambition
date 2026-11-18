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

/// A request is scoped to its recipient, submitting entry, and native domain.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ActionTicket {
    pub entry: String,
    pub domain: String,
    pub request: u32,
    pub occurrence: u64,
    pub room: u32,
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

/// A native domain publishes this fact after its own admission or mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionResult {
    pub ticket: ActionTicket,
    pub stage: ActionStage,
    pub terminal: bool,
    pub tick: u64,
    pub code: String,
    pub message: String,
    pub subjects: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionJournalEntry {
    pub sequence: u64,
    pub result: ActionResult,
}

/// Bounded retained history. `dropped` makes a delivery gap explicit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActionResults {
    pub next_sequence: u64,
    pub dropped: u64,
    pub entries: Vec<ActionJournalEntry>,
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
        /// The live room that minted this caller lifetime.
        room: u32,
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
        /// Stable semantic identities supplied by the domain. No native entity handles.
        subjects: Vec<String>,
    }
}
crate::port_record! { ActionJournalEntry { sequence: u64, result: ActionResult } }
crate::port_record! {
    ActionResults {
        next_sequence: u64,
        dropped: u64,
        entries: Vec<ActionJournalEntry>,
    }
}

pub struct CancelActionPort;
pub struct ActionResultPort;
pub struct ActionResultsPort;

impl Port for ActionResultPort {
    const KEY: PortKey = PortKey::new("ambition.actions.result", 1);
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
    const KEY: PortKey = PortKey::new("ambition.actions.history", 1);
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

/// Cancel pending work in the caller's scope and module. Applied terminal
/// work cannot be canceled. The domain checks the ticket before each change.
impl Port for CancelActionPort {
    const KEY: PortKey = PortKey::new("ambition.actions.cancel", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = ActionTicket;
    fn encode(value: &Self::Value, out: &mut Vec<u8>) {
        crate::structure::put(value, out);
    }
    fn decode(r: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
        crate::structure::read(r)
    }
}
crate::script_port!(CancelActionPort);
