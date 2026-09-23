//! The authored vocabulary of actor resources: which resource, how much of it,
//! and what a body starts with.
//!
//! See `docs/planning/engine/composable-actor-resources.md`. This crate owns the
//! IDENTITY and the authored shapes; the per-actor bank that stores values is
//! `ambition_platformer2d_core::resources`, and what a resource MEANS (how it
//! fills, what spends it) belongs to the capability that declares it.
//!
//! A leaf crate because two layers that may not depend on each other both need
//! it: content data prices moves in it, and the body floor stores it.
//!
//! ⛔ There is no list of resources here. A resource is whatever some content
//! names, so the engine cannot enumerate them and must not special-case one.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

/// A stable authored resource identity: the name content wrote, and a digest of
/// it that is what the simulation compares.
///
/// The digest is derived from the name alone (FNV-1a), so two peers that load
/// the same content derive the same id without agreeing on a registration
/// order, and a save that names `"limit"` means the same resource in every
/// build. Equality and order are by digest, then name, so a digest collision
/// between two distinct names still compares unequal.
#[derive(Clone, Debug)]
pub struct ResourceId {
    digest: u64,
    name: Cow<'static, str>,
}

impl ResourceId {
    /// An identity for a name known at compile time — a capability's own
    /// resource, declared as a `const` beside the rule that owns it.
    pub const fn from_static(name: &'static str) -> Self {
        Self {
            digest: fnv1a(name.as_bytes()),
            name: Cow::Borrowed(name),
        }
    }

    /// An identity for a name read from content.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            digest: fnv1a(name.as_bytes()),
            name: Cow::Owned(name),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The content digest the simulation compares and a checksum folds.
    pub const fn digest(&self) -> u64 {
        self.digest
    }
}

impl PartialEq for ResourceId {
    fn eq(&self, other: &Self) -> bool {
        self.digest == other.digest && self.name == other.name
    }
}

impl Eq for ResourceId {}

impl PartialOrd for ResourceId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ResourceId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.digest
            .cmp(&other.digest)
            .then_with(|| self.name.cmp(&other.name))
    }
}

impl std::hash::Hash for ResourceId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.digest.hash(state);
    }
}

impl std::fmt::Display for ResourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

impl Serialize for ResourceId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.name)
    }
}

impl<'de> Deserialize<'de> for ResourceId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        if name.is_empty() {
            return Err(serde::de::Error::custom("a resource id cannot be empty"));
        }
        Ok(Self::new(name))
    }
}

const fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        i += 1;
    }
    hash
}

/// One term of a price: this much of this resource.
///
/// A move's price is a list of these, paid atomically: either every term is
/// affordable and all are paid, or nothing is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceCost {
    pub resource: ResourceId,
    pub amount: f32,
}

impl ResourceCost {
    pub const fn new(resource: ResourceId, amount: f32) -> Self {
        Self { resource, amount }
    }
}

/// What a body holds of a resource when it is built and when it is reset.
///
/// One value for both, because a reset that chose its own start is the
/// construction-then-correction this exists to remove: an EARNED resource (a
/// Limit) starts empty on spawn and on every respawn; a SPENT one (a Mana pool)
/// starts full on both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceStart {
    Empty,
    Full,
}

/// A body declares it holds this resource, with this capacity and start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceDeclaration {
    pub resource: ResourceId,
    pub capacity: f32,
    pub start: ResourceStart,
}

impl ResourceDeclaration {
    pub const fn new(resource: ResourceId, capacity: f32, start: ResourceStart) -> Self {
        Self {
            resource,
            capacity,
            start,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_static_and_a_content_read_name_are_the_same_identity() {
        const LIMIT: ResourceId = ResourceId::from_static("limit");
        assert_eq!(LIMIT, ResourceId::new("limit"));
        assert_ne!(LIMIT, ResourceId::new("mana"));
        let parsed: ResourceId = ron::from_str("\"limit\"").expect("parses");
        assert_eq!(parsed, LIMIT);
        assert_eq!(ron::to_string(&LIMIT).expect("writes"), "\"limit\"");
    }
}
