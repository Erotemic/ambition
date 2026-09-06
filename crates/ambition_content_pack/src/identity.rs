//! The stable identities a prepared pack assigns.
//!
//! Everything downstream — semantic actions, causal facts, diagnostics, the
//! inspector's display — quotes these rather than reconstructing a name from a
//! runtime internal. That is the one rule that keeps three programs sharing one
//! vocabulary instead of three.

use std::fmt;

/// A pack's stable identity. Two packs with the same id are the same pack at
/// two versions, never two different packs.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct PackId(pub String);

/// The pack's version. Compared as an opaque string: the compiler never orders
/// versions, it only records which one produced a fingerprint.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct PackVersion(pub String);

/// The namespace every content identity in this pack is minted under. Two packs
/// may both define `goblin` as long as their namespaces differ; that is what
/// makes a third-party pack safe to install beside the shipped one.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct ModuleNamespace(pub String);

/// A schema's stable identity — the authored `kind`, owned by exactly one
/// capability.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct SchemaId(pub String);

/// A schema's version. Unlike [`PackVersion`] this IS compared: a source
/// declaring version 2 against an installed version 1 handler is a hard error,
/// because the handler would read fields that mean something else.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Deserialize,
    serde::Serialize,
)]
#[serde(transparent)]
pub struct SchemaVersion(pub u32);

/// A capability's stable identity. A pack declares the capabilities it requires;
/// a composition declares the ones it installs; the compiler refuses the
/// difference BEFORE anything is assembled.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct CapabilityId(pub String);

/// One piece of authored content, canonically identified.
///
/// Ordering is `(namespace, schema, name)` and it is the pack's canonical
/// content ordering: a `BTreeMap` keyed by this is deterministic across
/// platforms, runs, and filesystem enumeration order — which is what makes the
/// fingerprint mean anything.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentId {
    pub namespace: ModuleNamespace,
    pub schema: SchemaId,
    pub name: String,
}

impl ContentId {
    pub fn new(
        namespace: &ModuleNamespace,
        schema: &SchemaId,
        name: impl Into<String>,
    ) -> Self {
        Self {
            namespace: namespace.clone(),
            schema: schema.clone(),
            name: name.into(),
        }
    }
}

impl fmt::Display for ContentId {
    /// `namespace:schema/name` — the form that appears in every diagnostic and
    /// every causal fact, so a reader can grep one string across the compiler,
    /// the runtime and the inspector.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}/{}", self.namespace.0, self.schema.0, self.name)
    }
}

macro_rules! display_newtype {
    ($($t:ty),*) => {$(
        impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    )*};
}
display_newtype!(PackId, PackVersion, ModuleNamespace, SchemaId, CapabilityId);

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl SchemaId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl CapabilityId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}
