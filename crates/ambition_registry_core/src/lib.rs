//! The protocol every canonical registry in the workspace repeats, written once.
//!
//! Thirty-one `*Registry` types answer the same four questions independently
//! — what counts as identity, whether a second registration of the same key
//! is a no-op or a refusal, what enters a fingerprint, and whether a conflict
//! leaves the old registry untouched — and the 2026-09-02 inventory
//! (`docs/planning/triage/ambition-registry-core.md`) found three conflict
//! protocols coexisting, seven registries overwriting silently, and only one
//! (`ConstructionRegistry`) answering all four on purpose. This crate is that
//! one's answers, extracted so the next registry inherits them instead of
//! re-deciding them.
//!
//! ⛔ NOT A GENERIC REGISTRY. Domain crates keep their key and value types,
//! their maps, their executable dispatch, their override policy and their
//! Bevy resources. What lives here is the part that must not drift between
//! them: the stable declaration metadata, the classification of a second
//! registration, and the canonical row grammar a deterministic dump and a
//! fingerprint both read. A registry whose policy is genuinely different
//! (silent replacement by design, for instance) says so by not using
//! [`classify`] — and then has to say why in place.
//!
//! ⛔ NOTHING PROCESS-LOCAL ENTERS IDENTITY. A function address, a `TypeId`, an
//! allocation order — none of them can be a registration's identity, because
//! two builds of the same content must fingerprint equal. The types here carry
//! strings for that reason and validate them for the same one.

use std::fmt;

/// Who declared a registration, from where, under which stable schema.
///
/// Equality is semantic and stable across builds: three non-empty strings and
/// nothing else. A registry that needs more (a kind, a detail string) keeps its
/// own entry type and puts one of these inside it, or compares its own entry
/// with [`classify`] directly — the classification does not care what the
/// entry is, only whether the two are equal.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegistrationMeta {
    pub owner: String,
    pub source: String,
    pub schema_id: String,
}

impl RegistrationMeta {
    /// Validated: every field non-empty after trimming, because an empty owner
    /// or schema is a registration nobody can be asked about later.
    pub fn new(
        owner: impl Into<String>,
        source: impl Into<String>,
        schema_id: impl Into<String>,
    ) -> Result<Self, EmptyField> {
        let meta = Self {
            owner: owner.into(),
            source: source.into(),
            schema_id: schema_id.into(),
        };
        require_non_empty(&[
            ("owner", &meta.owner),
            ("source", &meta.source),
            ("schema id", &meta.schema_id),
        ])?;
        Ok(meta)
    }
}

/// A required identity field was blank. The field is named so the message a
/// content author reads says which of the strings they left out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmptyField {
    pub field: &'static str,
}

impl fmt::Display for EmptyField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "registration {} must not be empty", self.field)
    }
}

impl std::error::Error for EmptyField {}

/// Refuse a registration whose identity has a blank field. Trimmed, so a
/// string of spaces is as empty as no string.
pub fn require_non_empty(fields: &[(&'static str, &str)]) -> Result<(), EmptyField> {
    for (field, value) in fields {
        if value.trim().is_empty() {
            return Err(EmptyField { field });
        }
    }
    Ok(())
}

/// What a second registration under an already-known key IS.
///
/// The three answers every registry has to give, given once: nothing was
/// there (insert it), the same thing was there (do nothing, and say so), or
/// something else was there (refuse, and hand back both so the message can
/// name them). ⛔ There is deliberately no fourth answer — "replace" is a
/// policy a registry may adopt, but not through this function, so a silent
/// overwrite cannot be the accidental default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Classification<'a, E> {
    /// No entry under this key yet.
    New,
    /// The same entry is already registered.
    Idempotent,
    /// A different entry holds the key.
    Conflict { existing: &'a E },
}

pub fn classify<'a, E: PartialEq>(existing: Option<&'a E>, incoming: &E) -> Classification<'a, E> {
    match existing {
        None => Classification::New,
        Some(existing) if existing == incoming => Classification::Idempotent,
        Some(existing) => Classification::Conflict { existing },
    }
}

/// What a successful registration did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationOutcome {
    Inserted,
    Idempotent,
}

/// A refused registration, carrying both sides so the domain's error can say
/// what was there and what arrived without re-deriving either.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict<K, E> {
    pub key: K,
    pub existing: E,
    pub incoming: E,
}

impl<K: fmt::Display, E: fmt::Debug> fmt::Display for Conflict<K, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "conflicting registration '{}': existing {:?}, incoming {:?}",
            self.key, self.existing, self.incoming
        )
    }
}

/// One canonical row: tab-separated fields and a trailing newline.
///
/// The grammar every deterministic dump in the workspace already used by
/// convention, made a function so a row cannot quietly grow a field that the
/// fingerprint reads and the diagnostic dump omits (or the reverse), and so
/// the separator is never ambiguous. A field containing a tab or a newline
/// would forge a column or a row; that is a programming error, not content,
/// and it panics rather than escaping — an escaped byte would change every
/// fingerprint that row enters.
pub fn canonical_row(fields: &[&str]) -> String {
    for field in fields {
        assert!(
            !field.contains(['\t', '\n']),
            "a canonical row field may not contain a tab or a newline: {field:?}"
        );
    }
    let mut row = fields.join("\t");
    row.push('\n');
    row
}

/// A canonical section: an optional header line, then rows in the order the
/// caller's ORDERED map yields them. The caller iterates a `BTreeMap` (or
/// sorts); this function does not sort, because a section may have several
/// row kinds whose relative order is itself part of the grammar
/// (construction emits recipes before relations).
pub fn canonical_section<'a>(header: Option<&str>, rows: impl IntoIterator<Item = &'a str>) -> String {
    let mut out = String::new();
    if let Some(header) = header {
        out.push_str(header);
        out.push('\n');
    }
    for row in rows {
        out.push_str(row);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_registration_is_new_idempotent_or_conflict_and_nothing_else() {
        let a = RegistrationMeta::new("engine", "src/a.rs", "v1").unwrap();
        let b = RegistrationMeta::new("content", "src/b.rs", "v1").unwrap();
        assert_eq!(classify::<RegistrationMeta>(None, &a), Classification::New);
        assert_eq!(classify(Some(&a), &a.clone()), Classification::Idempotent);
        assert_eq!(classify(Some(&a), &b), Classification::Conflict { existing: &a });
    }

    /// Poison for the validation: every blank field is named, and a string of
    /// spaces is blank.
    #[test]
    fn a_blank_identity_field_is_refused_by_name() {
        assert_eq!(
            RegistrationMeta::new("", "s", "v").unwrap_err(),
            EmptyField { field: "owner" }
        );
        assert_eq!(
            RegistrationMeta::new("o", "   ", "v").unwrap_err(),
            EmptyField { field: "source" }
        );
        assert_eq!(
            RegistrationMeta::new("o", "s", "\t").unwrap_err(),
            EmptyField { field: "schema id" }
        );
        assert!(RegistrationMeta::new("o", "s", "v").is_ok());
    }

    #[test]
    fn rows_are_tab_separated_and_a_section_keeps_the_callers_order() {
        let rows = [
            canonical_row(&["recipe", "door", "engine", "src/x.rs", "v3"]),
            canonical_row(&["relation", "hinge", "engine", "src/y.rs", "v1"]),
        ];
        assert_eq!(rows[0], "recipe\tdoor\tengine\tsrc/x.rs\tv3\n");
        let section = canonical_section(Some("schema-v2"), rows.iter().map(String::as_str));
        assert_eq!(
            section,
            "schema-v2\nrecipe\tdoor\tengine\tsrc/x.rs\tv3\nrelation\thinge\tengine\tsrc/y.rs\tv1\n"
        );
        assert_eq!(canonical_section(None, rows.iter().map(String::as_str)).lines().count(), 2);
    }

    /// A field that would forge a column is a bug, and escaping it would
    /// silently move every fingerprint its row enters.
    #[test]
    #[should_panic(expected = "may not contain a tab")]
    fn a_tab_in_a_field_is_a_programming_error_not_content() {
        let _ = canonical_row(&["a\tb"]);
    }
}
