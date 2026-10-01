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

impl PortKey {
    pub fn put(&self, out: &mut Vec<u8>) {
        crate::wire::put_str(out, &self.name);
        crate::wire::put_u16(out, self.version);
    }

    pub fn read(r: &mut crate::wire::WireReader<'_>) -> Result<Self, crate::wire::WireError> {
        Ok(Self {
            name: r.str()?.to_owned().into(),
            version: r.u16()?,
        })
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

    /// The value's wire encoding (see [`crate::wire`]). A loaded module and
    /// the host exchange these bytes; the encoding is part of the port's
    /// version.
    fn encode(value: &Self::Value, out: &mut Vec<u8>);

    /// The inverse of [`Port::encode`]. It must read exactly the bytes
    /// `encode` wrote.
    fn decode(r: &mut crate::wire::WireReader<'_>) -> Result<Self::Value, crate::wire::WireError>;
}

/// Encode a type-erased value of port `P`. The host and the guest glue keep
/// one of these beside each value whose type they no longer name.
pub fn encode_erased<P: Port>(value: &(dyn std::any::Any + Send + Sync), out: &mut Vec<u8>) {
    let value = value
        .downcast_ref::<P::Value>()
        .unwrap_or_else(|| panic!("a value stored under port {} is not its type", P::KEY));
    P::encode(value, out);
}

/// Decode and box a value of port `P`.
pub fn decode_erased<P: Port>(
    bytes: &[u8],
) -> Result<Box<dyn std::any::Any + Send + Sync>, crate::wire::WireError> {
    crate::wire::decode_all(bytes, P::decode).map(|v| Box::new(v) as Box<_>)
}

/// A type-erased encoder for one port's values.
pub type EncodeFn = fn(&(dyn std::any::Any + Send + Sync), &mut Vec<u8>);

/// A type-erased decoder for one port's values.
pub type DecodeFn =
    fn(&[u8]) -> Result<Box<dyn std::any::Any + Send + Sync>, crate::wire::WireError>;
