//! Scoped replies from domain actions. Submission does not prove application.

use crate::script::ScriptValue;
use crate::structure::{CaseShape, Field, Shape};
use crate::wire::{self, WireError, WireReader};
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

/// The domain reports the stage it actually reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ActionStage {
    Submitted,
    Admitted,
    Applied,
    Completed,
    Rejected,
    Canceled,
    Failed,
}

impl ActionStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Submitted => "submitted",
            Self::Admitted => "admitted",
            Self::Applied => "applied",
            Self::Completed => "completed",
            Self::Rejected => "rejected",
            Self::Canceled => "canceled",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "submitted" => Ok(Self::Submitted),
            "admitted" => Ok(Self::Admitted),
            "applied" => Ok(Self::Applied),
            "completed" => Ok(Self::Completed),
            "rejected" => Ok(Self::Rejected),
            "canceled" => Ok(Self::Canceled),
            "failed" => Ok(Self::Failed),
            _ => Err(format!("unknown action stage {value:?}")),
        }
    }
}

impl Field<ActionStage> for ActionStage {
    fn shape() -> Shape {
        Shape::Choice(
            [
                "submitted",
                "admitted",
                "applied",
                "completed",
                "rejected",
                "canceled",
                "failed",
            ]
            .into_iter()
            .map(|name| CaseShape {
                name,
                payload: None,
                script: name,
            })
            .collect(),
        )
    }
    fn put(value: &ActionStage, out: &mut Vec<u8>) {
        wire::put_u8(out, *value as u8);
    }
    fn read(r: &mut WireReader<'_>) -> Result<ActionStage, WireError> {
        match r.u8()? {
            0 => Ok(Self::Submitted),
            1 => Ok(Self::Admitted),
            2 => Ok(Self::Applied),
            3 => Ok(Self::Completed),
            4 => Ok(Self::Rejected),
            5 => Ok(Self::Canceled),
            6 => Ok(Self::Failed),
            tag => Err(WireError::BadTag(tag)),
        }
    }
    fn to_script(value: &ActionStage) -> ScriptValue {
        ScriptValue::Text(value.as_str().into())
    }
    fn from_script(value: &ScriptValue) -> Result<ActionStage, String> {
        Self::parse(value.as_text()?)
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
