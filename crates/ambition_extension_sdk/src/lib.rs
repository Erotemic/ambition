//! The portable procedural extension contract.
//!
//! A procedural module is a bounded algorithm with its own state. It reads
//! facts through observation ports, changes its own state records and submits
//! typed requests to the domains that own the engine facts. It does not get a
//! `World`, a `Commands`, an `Entity` or a component setter.
//!
//! This crate has no dependencies. The Bevy host adapter is
//! `ambition_extension_host`. Each domain owns the pure values of its own
//! ports (for example `ambition_projectile_spec` owns the projectile spawn
//! request). See `docs/planning/engine/extension-model.md` (D3, D4, D5) and
//! `docs/planning/engine/extension-state-and-execution.md`.
//!
//! The parts:
//!
//! * [`schema`] — state schemas, logical values, records and their canonical
//!   encoding and digest.
//! * [`port`] — port keys and the typed [`port::Port`] trait that a domain
//!   implements for its pure values.
//! * [`module`] — the module and entry descriptors that admission reads.
//! * [`invoke`] — the [`invoke::Invocation`] that an entry receives.

pub mod digest;
pub mod invoke;
pub mod module;
pub mod port;
pub mod schema;
pub mod typed;
pub mod wire;
pub mod abi;

pub use invoke::{Fault, HostParts, Invocation, Observation, OwnedPayload, Payload, StagedRequest};
pub use module::{
    phases, CodeIdentity, EntryCode, EntryDescriptor, EntryFn, IdlePolicy, Limits, ModuleDescriptor, ModuleKey, Phase,
    TriggerBinding,
};
pub use port::{decode_erased, encode_erased, DecodeFn, EncodeFn, Port, PortKey, PortRole};
pub use schema::{
    Attachment, FieldDecl, FieldKind, FieldRef, Record, SaveEligibility, SchemaKey, StateSchema,
    Value,
};

/// A semantic API version. The host supports one major version and a range of
/// minor versions. Do not infer compatibility from Rust crate versions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApiVersion {
    pub major: u16,
    pub minor: u16,
}

/// The API version of this SDK. A module records the version it was built
/// against in its descriptor.
pub const API_VERSION: ApiVersion = ApiVersion { major: 0, minor: 1 };

/// A name in a descriptor. Static modules use `&'static str`; a loaded module
/// can own its strings.
pub type Name = std::borrow::Cow<'static, str>;
