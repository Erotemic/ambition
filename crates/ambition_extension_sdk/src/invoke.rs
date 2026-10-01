//! The invocation an entry receives.
//!
//! ```text
//! read cut + own records  ->  entry  ->  staged record writes + staged requests
//! ```
//!
//! An entry reads its trigger, the observations it declared and its own
//! records. It changes only staged copies of its own records and only stages
//! requests. The host checks the staged output after the entry returns. If the
//! entry faults, the host discards ALL of its staged writes and requests.

use std::any::Any;
use std::cell::OnceCell;

use crate::module::EntryDescriptor;
use crate::port::{Port, PortKey, PortRole};
use crate::schema::{Record, SchemaError, SchemaKey};
use crate::Name;

/// Why an invocation failed. A fault discards the invocation's staged output.
#[derive(Clone, Debug, PartialEq)]
pub enum Fault {
    /// The entry read an observation port it did not declare.
    UndeclaredRead(PortKey),
    /// The entry declared the read, but the host gave no value for this scope.
    MissingObservation(PortKey),
    /// The entry asked for records of a schema it did not declare.
    UndeclaredState(SchemaKey),
    /// The entry submitted to a request port it did not declare.
    UndeclaredRequest(PortKey),
    /// The entry asked for its trigger as the wrong port.
    WrongTrigger { asked: PortKey, actual: PortKey },
    /// A typed port's role does not match the use.
    WrongRole { port: PortKey, role: PortRole },
    /// The entry submitted more requests than its declared limit.
    RequestLimit { limit: u32 },
    /// A record does not match its schema.
    Schema { schema: SchemaKey, error: SchemaError },
    /// The module reports its own deterministic failure.
    Module(Name),
    /// The entry declares session-attached state and the host has no one
    /// session to keep it in (no session, or more than one).
    NoSession(SchemaKey),
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UndeclaredRead(p) => write!(f, "read of undeclared observation port {p}"),
            Self::MissingObservation(p) => write!(f, "no value for observation port {p}"),
            Self::UndeclaredState(s) => write!(f, "access to undeclared state {s}"),
            Self::UndeclaredRequest(p) => write!(f, "submit to undeclared request port {p}"),
            Self::WrongTrigger { asked, actual } => {
                write!(f, "asked for trigger {asked}; the trigger is {actual}")
            }
            Self::WrongRole { port, role } => write!(f, "port {port} is not a {role:?} port"),
            Self::RequestLimit { limit } => write!(f, "more than {limit} requests"),
            Self::Schema { schema, error } => write!(f, "state {schema}: {error}"),
            Self::Module(msg) => write!(f, "module fault: {msg}"),
            Self::NoSession(s) => write!(f, "state {s} is session-attached and there is no one session"),
        }
    }
}

/// A request that an entry staged. The host gives it to the request port's
/// domain adapter only if the whole invocation succeeds.
pub struct StagedRequest {
    pub port: PortKey,
    pub value: Box<dyn Any + Send + Sync>,
    /// Encodes `value` for a guest that must hand it back as bytes.
    pub encode: crate::port::EncodeFn,
}

impl std::fmt::Debug for StagedRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StagedRequest")
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

/// A port value as the invocation holds it: a Rust value in this process,
/// or bytes that a loaded module decodes when it first asks.
pub enum Payload<'a> {
    Native(&'a (dyn Any + Send + Sync)),
    Wire(&'a [u8]),
}

/// One observation value in the read cut.
pub struct Observation {
    pub port: PortKey,
    pub value: OwnedPayload,
}

/// An owned [`Payload`].
pub enum OwnedPayload {
    Native(Box<dyn Any + Send + Sync>),
    Wire(Vec<u8>),
}

impl OwnedPayload {
    pub fn as_payload(&self) -> Payload<'_> {
        match self {
            OwnedPayload::Native(v) => Payload::Native(v.as_ref()),
            OwnedPayload::Wire(b) => Payload::Wire(b),
        }
    }
}

/// The parts the host gives to [`Invocation::from_host`].
pub struct HostParts<'a> {
    pub entry: &'a EntryDescriptor,
    /// The logical simulation tick.
    pub tick: u64,
    /// The gameplay seconds this tick advances: zero while paused, scaled
    /// by bullet time (the engine's `WorldTime::sim_dt`).
    pub dt: f32,
    /// The use of a move that asked for this invocation, if a move asked.
    pub occurrence: Option<u32>,
    pub trigger_port: &'a PortKey,
    pub trigger: Payload<'a>,
    pub observations: &'a [Observation],
    /// Staged copies of the scope's records, one for each declared schema.
    pub state: &'a mut [(SchemaKey, Record)],
    pub requests: &'a mut Vec<StagedRequest>,
}

/// What an entry receives.
pub struct Invocation<'a> {
    parts: HostParts<'a>,
    /// Decoded wire values, one cell for the trigger and one per observation.
    decoded_trigger: OnceCell<Box<dyn Any + Send + Sync>>,
    decoded_observations: Vec<OnceCell<Box<dyn Any + Send + Sync>>>,
}

/// Read a payload as port `P`'s value, decoding wire bytes once.
fn payload_value<'c, P: Port>(
    payload: Payload<'c>,
    cell: &'c OnceCell<Box<dyn Any + Send + Sync>>,
) -> Result<&'c P::Value, Fault> {
    match payload {
        Payload::Native(v) => v.downcast_ref::<P::Value>().ok_or(Fault::MissingObservation(P::KEY)),
        Payload::Wire(bytes) => {
            if cell.get().is_none() {
                let value = crate::wire::decode_all(bytes, P::decode).map_err(|e| {
                    Fault::Module(format!("port {} value does not decode: {e}", P::KEY).into())
                })?;
                let _ = cell.set(Box::new(value));
            }
            cell.get()
                .and_then(|v| v.downcast_ref::<P::Value>())
                .ok_or(Fault::MissingObservation(P::KEY))
        }
    }
}

impl<'a> Invocation<'a> {
    /// Host API: build an invocation. A module never calls this.
    pub fn from_host(parts: HostParts<'a>) -> Self {
        let decoded_observations = parts.observations.iter().map(|_| OnceCell::new()).collect();
        Self {
            parts,
            decoded_trigger: OnceCell::new(),
            decoded_observations,
        }
    }

    /// The logical simulation tick. There is no wall clock.
    pub fn tick(&self) -> u64 {
        self.parts.tick
    }

    /// The gameplay seconds this tick advances: zero while paused, scaled by
    /// bullet time. A timer a module keeps counts these, never wall time.
    pub fn dt(&self) -> f32 {
        self.parts.dt
    }

    /// The move use that asked for this invocation. The host attaches it to
    /// every request this invocation submits; a module cannot change it.
    pub fn occurrence(&self) -> Option<u32> {
        self.parts.occurrence
    }

    /// The trigger value, typed as port `P`.
    pub fn trigger<P: Port>(&self) -> Result<&P::Value, Fault> {
        if P::ROLE != PortRole::Trigger {
            return Err(Fault::WrongRole {
                port: P::KEY,
                role: PortRole::Trigger,
            });
        }
        if *self.parts.trigger_port != P::KEY {
            return Err(Fault::WrongTrigger {
                asked: P::KEY,
                actual: self.parts.trigger_port.clone(),
            });
        }
        let payload = match &self.parts.trigger {
            Payload::Native(v) => Payload::Native(*v),
            Payload::Wire(b) => Payload::Wire(b),
        };
        payload_value::<P>(payload, &self.decoded_trigger)
    }

    /// An observation from port `P`. The entry must declare `P` in `reads`.
    pub fn observe<P: Port>(&self) -> Result<&P::Value, Fault> {
        if P::ROLE != PortRole::Observation {
            return Err(Fault::WrongRole {
                port: P::KEY,
                role: PortRole::Observation,
            });
        }
        if !self.parts.entry.reads.contains(&P::KEY) {
            return Err(Fault::UndeclaredRead(P::KEY));
        }
        let index = self
            .parts
            .observations
            .iter()
            .position(|o| o.port == P::KEY)
            .ok_or(Fault::MissingObservation(P::KEY))?;
        payload_value::<P>(
            self.parts.observations[index].value.as_payload(),
            &self.decoded_observations[index],
        )
    }

    /// The staged record of a declared schema for this scope.
    pub fn state(&mut self, schema: &SchemaKey) -> Result<&mut Record, Fault> {
        self.parts
            .state
            .iter_mut()
            .find(|(key, _)| key == schema)
            .map(|(_, record)| record)
            .ok_or_else(|| Fault::UndeclaredState(schema.clone()))
    }

    /// Stages a request to port `P`. The entry must declare `P` in `requests`.
    /// A staged request is SUBMITTED, not applied: the domain decides.
    pub fn submit<P: Port>(&mut self, value: P::Value) -> Result<(), Fault> {
        if P::ROLE != PortRole::Request {
            return Err(Fault::WrongRole {
                port: P::KEY,
                role: PortRole::Request,
            });
        }
        if !self.parts.entry.requests.contains(&P::KEY) {
            return Err(Fault::UndeclaredRequest(P::KEY));
        }
        let limit = self.parts.entry.limits.max_requests;
        if self.parts.requests.len() >= limit as usize {
            return Err(Fault::RequestLimit { limit });
        }
        self.parts.requests.push(StagedRequest {
            port: P::KEY,
            value: Box::new(value),
            encode: crate::port::encode_erased::<P>,
        });
        Ok(())
    }
}
