//! Naming a shared autonomous-controller policy, in the two forms an
//! authored reference and a resolved identity need to be.
//!
//! They live here, beside `CharacterId`, not with `BrainProfile`, because two
//! authoring surfaces name a policy and only one can see the brain crate. A
//! `CharacterDefinition` names one, and so does an `EnemySpawn` placement
//! (`ambition_platformer2d_world`, which must not depend on
//! `ambition_characters`). The shared names go in the crate both depend on;
//! the policy value stays with the brains.

/// A provider-relative reference to a shared `BrainProfile`, as a
/// character authored it.
///
/// Content writes the local name. A definition in provider `ambition` authors
/// `medium_striker`, not `ambition::medium_striker`, because whether the
/// catalog is already namespaced is an assembly detail. Preparation resolves
/// it against the definition's own provider.
///
/// An already-qualified reference is kept: a `::` in the authored text means
/// "this exact policy", so a character can name another provider's policy.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct BrainProfileRef(String);

impl BrainProfileRef {
    pub fn new(reference: impl Into<String>) -> Self {
        Self(reference.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Resolve to the canonical identity, namespacing with `provider` unless the
    /// author already qualified it.
    pub fn resolve_in(&self, provider: &str) -> BrainProfileId {
        if self.0.contains("::") {
            BrainProfileId(self.0.clone())
        } else {
            BrainProfileId(format!("{provider}::{}", self.0))
        }
    }
}

impl std::fmt::Display for BrainProfileRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The canonical identity of a shared `BrainProfile` — `provider::name`,
/// the key the assembled registry actually holds.
///
/// A distinct type from `BrainProfileRef` on purpose. Both are strings with
/// different meanings: a raw authored spelling passed to a lookup that wants a
/// resolved one returns `None` silently.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BrainProfileId(String);

impl BrainProfileId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BrainProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod reference_tests {
    use super::*;

    /// An author writes the local name and preparation namespaces it.
    #[test]
    fn a_provider_relative_reference_resolves_against_its_own_provider() {
        let authored = BrainProfileRef::new("medium_striker");
        assert_eq!(
            authored.resolve_in("ambition").as_str(),
            "ambition::medium_striker"
        );
    }

    /// A deliberate cross-provider reference survives, so a demo may reuse
    /// the flagship's policy by naming it.
    #[test]
    fn an_already_qualified_reference_is_not_namespaced_twice() {
        let authored = BrainProfileRef::new("ambition::medium_striker");
        assert_eq!(
            authored.resolve_in("mary_o").as_str(),
            "ambition::medium_striker",
            "double-namespacing produces a key that exists nowhere, which is \
             exactly the silent `None` the typed pair exists to stop"
        );
    }
}
