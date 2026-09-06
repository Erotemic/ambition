//! Shared LDtk entity-authoring contract.
//!
//! `ldtk_entity_contract.json` is the single required-field/grammar table used
//! by both authoring paths. Rust proves it against the real conversion code in
//! both directions; Python reads the same data for validation without Cargo.

use std::sync::OnceLock;

use serde::Deserialize;

/// The contract document, baked in at compile time so the runtime and the law it
/// obeys ship together.
pub const CONTRACT_JSON: &str = include_str!("../ldtk_entity_contract.json");

/// Every entity the engine's standard converters understand, and the rules each
/// one's authored fields obey.
#[derive(Clone, Debug, Deserialize)]
pub struct LdtkAuthoringContract {
    pub version: u32,
    #[serde(default)]
    pub entities: Vec<EntityContract>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EntityContract {
    pub identifier: String,
    /// Width/height the prover gives a probe instance. Surface-shaped entities
    /// need a positive size to compile at all.
    #[serde(default = "default_probe_size")]
    pub probe_size: [i32; 2],
    /// Cargo feature the converter needs. Absent = always compiled in.
    #[serde(default)]
    pub feature: Option<String>,
    /// Read off the raw project by its own consumer; the converter emits nothing,
    /// so the converter is not the authority for this entity's fields.
    #[serde(default)]
    pub consumed_elsewhere: bool,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub fields: Vec<FieldContract>,
}

fn default_probe_size() -> [i32; 2] {
    [16, 16]
}

/// What the converter does when the field is ABSENT.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Presence {
    /// The converter returns `Err`. Authoring: an error.
    Required,
    /// The converter accepts absence, but the result is not what any author
    /// means. Authoring: a warning.
    Recommended,
    /// Absence is ordinary. Authoring: silent.
    #[default]
    Optional,
}

/// What the converter does with a value the field's declared grammar REJECTS.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OnInvalid {
    /// The converter returns `Err`. Authoring: an error, agreeing with the
    /// runtime.
    Refused,
    /// The converter accepts it and quietly substitutes `default`.
    ///
    /// The authoring loop is deliberately stricter than the runtime here, and that asymmetry is
    /// the point rather than an oversight.
    SilentDefault,
    /// The fallthrough is a real extension point with real consumers
    /// (`CharacterBrain::Custom("mary_o_snake")`, a `PropRegistry` id). Authoring
    /// says nothing.
    Open,
}

/// A presence that only applies when a sibling field holds a particular value.
#[derive(Clone, Debug, Deserialize)]
pub struct RequiresValueOf {
    pub field: String,
    pub equals: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FieldContract {
    pub name: String,
    #[serde(default)]
    pub presence: Presence,
    pub on_invalid: OnInvalid,
    /// Closed literal set. Empty + empty `patterns` = the field has no declared
    /// grammar and only its presence is ruled.
    #[serde(default)]
    pub values: Vec<String>,
    /// Regex grammars completing `values` (`InPlace(0.85)`, `health:3`).
    #[serde(default)]
    pub patterns: Vec<String>,
    /// Spellings the converter refuses OUT LOUD — a retired convention, not an
    /// unknown value. Each needs a literal in `refused_samples` so the prover can
    /// poke it.
    #[serde(default)]
    pub refused_patterns: Vec<String>,
    #[serde(default)]
    pub refused_samples: Vec<String>,
    /// How the converter folds an authored value before matching it against
    /// `values`. spelled as the parser spells it, not as "case insensitive":
    /// `parse_path_mode` and the camera policies do
    /// `trim().to_ascii_lowercase().replace('-', "_")` (`lowercase_underscore`)
    /// while `PortalChannelColorSpec::from_name` only lowercases (`lowercase`) —
    /// and the difference is real, because `c-1` is a legal colour under one rule
    /// and refused under the other.
    #[serde(default)]
    pub normalize: Option<String>,
    /// Semicolon-separated `x,y` polyline needing at least this many points.
    #[serde(default)]
    pub min_points: Option<usize>,
    /// Numeric field the converter requires to be `> 0`.
    #[serde(default)]
    pub positive: bool,
    /// Numeric field the converter requires to be non-zero (a signed span).
    #[serde(default)]
    pub nonzero: bool,
    #[serde(default)]
    pub requires_value_of: Option<RequiresValueOf>,
    /// Authoring this field without those is refused.
    #[serde(default)]
    pub requires_fields: Vec<String>,
    /// Authoring this field beside any of those is refused — two answers to one
    /// question.
    #[serde(default)]
    pub conflicts_with: Vec<String>,
    /// A native LDtk `EntityRef` that must name an entity of this identifier.
    #[serde(default)]
    pub entity_ref_target: Option<String>,
    /// `active_area` = the target must live in the SAME active area, matching the
    /// runtime lookup table's scope exactly.
    #[serde(default)]
    pub entity_ref_scope: Option<String>,
    /// What `silent_default` substitutes — quoted back to the author.
    #[serde(default)]
    pub default: Option<String>,
    /// Prover-only: a value that must convert.
    #[serde(default)]
    pub sample: Option<String>,
    /// Prover-only: overrides the illegal value derived from the grammar.
    #[serde(default)]
    pub poison: Option<String>,
    /// The author-facing sentence, used verbatim as a fix hint.
    #[serde(default)]
    pub note: Option<String>,
}

impl EntityContract {
    pub fn field(&self, name: &str) -> Option<&FieldContract> {
        self.fields.iter().find(|field| field.name == name)
    }
}

/// The parsed contract. Panics on a malformed document, which is a build-time
/// fact rather than a runtime input.
pub fn contract() -> &'static LdtkAuthoringContract {
    static CONTRACT: OnceLock<LdtkAuthoringContract> = OnceLock::new();
    CONTRACT.get_or_init(|| {
        serde_json::from_str(CONTRACT_JSON).expect(
            "ldtk_entity_contract.json is malformed — it is the single source of \
             truth for LDtk authoring and the crate cannot start without it",
        )
    })
}

/// The rules for one entity identifier, if the engine declares any.
pub fn contract_for(identifier: &str) -> Option<&'static EntityContract> {
    contract()
        .entities
        .iter()
        .find(|entity| entity.identifier == identifier)
}

#[cfg(test)]
mod prover;
