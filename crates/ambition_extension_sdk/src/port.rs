//! Ports: the typed doors between a module and a domain.
//!
//! A domain owns the pure values of each of its ports, in a crate that does
//! not depend on Bevy. It implements [`Port`] for a marker type. The host
//! installs an offer for the port only together with the adapter that answers
//! it, so a port key in the installed set means that something answers it.

use crate::Name;

/// The identity of a port: a namespaced name and an exact version.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PortKey {
    pub name: Name,
    pub version: u16,
}

impl PortKey {
    pub const fn new(name: &'static str, version: u16) -> Self {
        Self {
            name: Name::Borrowed(name),
            version,
        }
    }
}

impl std::fmt::Display for PortKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.name, self.version)
    }
}

/// What a port does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PortRole {
    /// The domain starts an invocation for one scope and gives its value as
    /// the invocation's trigger.
    Trigger,
    /// The domain gives a read-only projection inside an invocation.
    Observation,
    /// The module submits a value; the domain validates and applies it.
    Request,
}

/// A port, typed for the in-process Rust binding.
///
/// `Value` is a plain Rust value that the domain owns. A portable binding
/// lowers the same value to a bounded wire encoding; the semantics do not
/// change.
pub trait Port: 'static {
    const KEY: PortKey;
    const ROLE: PortRole;
    type Value: 'static + Send + Sync;
}
