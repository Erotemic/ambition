//! Immutable catalog brain references and provider namespace rules.

/// Stable id of a brain preset — a key into the catalog `brain_presets` map.
///
/// A newtype over `String` so a preset id can't be silently confused with a
/// character id or a bare string in a signature.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BrainPresetId(pub String);

impl BrainPresetId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for BrainPresetId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for BrainPresetId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for BrainPresetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Authored, provider-relative reference to a brain profile.
///
/// A reference such as `"combatant"` is qualified against the character's
/// namespace before becoming the canonical [`BrainPresetId`] such as
/// `"hall::combatant"`. Resolution currently occurs at spawn.
/// TODO(compat-remove): resolve brain references during preparation and carry
/// `BrainPresetId` in prepared definitions.
///
/// Distinct from `content_schema::BrainPresetRef`, the zero-sized validator tag.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BrainPresetRef(pub String);

impl BrainPresetRef {
    pub fn new(reference: impl Into<String>) -> Self {
        Self(reference.into())
    }

    /// The authored text, exactly as content wrote it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for BrainPresetRef {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for BrainPresetRef {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for BrainPresetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable catalog id for a boss's autonomous mode.
/// Control stores the id so the encounter owner can rebuild that mode after
/// temporary control ends.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BossAutonomyId(pub String);

impl BossAutonomyId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for BossAutonomyId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for BossAutonomyId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for BossAutonomyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Qualify a (possibly raw) local brain-preset name into the namespace of
/// `reference` — a fully-qualified preset name like `"provider::name"`.
///
/// The assembled [`super::CharacterCatalog`] namespaces every provider's brain presets
/// by owner (`"default::stand_still"`), and rewrites each character's
/// `default_brain` to match. Authoring surfaces (an LDtk `brain_override`, a
/// `<<use_brain>>` command) name a RAW local preset, so we qualify it into the
/// SAME namespace the character's `default_brain` lives in — passing `reference =
/// entry.default_brain` (or an actor's `BrainBinding::default_preset`). If `local`
/// is already qualified, or `reference` is un-namespaced (a raw / single-fragment
/// catalog, e.g. in tests), `local` is returned unchanged.
///
/// This is the whole namespace rule: a fully-qualified override is used exactly;
/// a raw override resolves ONLY within the character's own provider namespace.
/// There is no silent cross-provider fallback — a raw override the character's
/// provider does not own is a content error, not a lookup into some other
/// provider's presets.
pub fn qualify_preset_like(reference: &str, local: &str) -> String {
    if local.contains("::") {
        return local.to_string();
    }
    match reference.rsplit_once("::") {
        Some((provider, _)) => format!("{provider}::{local}"),
        None => local.to_string(),
    }
}

/// Qualify a local brain-profile reference into a PROVIDER's namespace.
///
///  the same rule as [`qualify_preset_like`], stated against the authority
/// that actually owns the namespace. That function infers the namespace from
/// a neighbouring key (`entry.default_brain`), which works only where a catalog
/// row is in hand — so a character's own prepared default needed a parallel
/// catalog row merely to learn its own provider. A `CharacterDefinition` states
/// its provider directly, and the assembled catalog namespaces every preset as
/// `provider::name` with the SAME provider id a definition carries. That
/// equality is guarded, not assumed: see
/// `character_definitions_and_catalog_fragments_share_one_provider_namespace`.
///
/// An already-qualified reference is used exactly, so a character may point at
/// another provider's profile deliberately; a raw one never silently falls
/// through to some other provider's presets.
pub fn qualify_in_provider(provider: &str, local: &str) -> String {
    if local.contains("::") || provider.is_empty() {
        return local.to_string();
    }
    format!("{provider}::{local}")
}
