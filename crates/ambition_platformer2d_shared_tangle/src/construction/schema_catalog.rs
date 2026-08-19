//! Descriptor-only federation for independently typed construction domains.
//!
//! A room may be built by several [`ConstructionDomain`](super::ConstructionDomain)
//! implementations, but executable recipe dispatch deliberately stays closed and
//! typed inside each domain. This catalog composes only their stable metadata so
//! prepared-content fingerprints still cover every installed construction schema.
//! It is not an executable registry and cannot choose, replace, or order recipe
//! functions.

use std::collections::BTreeMap;

use bevy::prelude::Resource;

/// Canonical construction-schema descriptions contributed by installed domains.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct ConstructionSchemaCatalog {
    domains: BTreeMap<String, String>,
}

/// A domain tried to publish two different schema descriptions under one id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstructionSchemaCatalogError {
    EmptyDomain,
    InvalidDomain { domain: String },
    ConflictingDomain { domain: String },
}

impl std::fmt::Display for ConstructionSchemaCatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyDomain => f.write_str("construction schema domain id must not be empty"),
            Self::InvalidDomain { domain } => write!(
                f,
                "construction schema domain `{domain}` contains a tab or newline and cannot be canonicalized"
            ),
            Self::ConflictingDomain { domain } => write!(
                f,
                "construction schema domain `{domain}` was contributed twice with different metadata"
            ),
        }
    }
}

impl std::error::Error for ConstructionSchemaCatalogError {}

impl ConstructionSchemaCatalog {
    /// Publish one domain's stable registry dump.
    ///
    /// Repeating an identical contribution is idempotent so normal Bevy plugin
    /// composition remains harmless. A different dump under the same domain id
    /// is an authority conflict and is refused rather than last-writer-wins.
    pub fn try_contribute(
        &mut self,
        domain: impl Into<String>,
        deterministic_registry_dump: impl Into<String>,
    ) -> Result<(), ConstructionSchemaCatalogError> {
        let domain = domain.into();
        if domain.trim().is_empty() {
            return Err(ConstructionSchemaCatalogError::EmptyDomain);
        }
        if domain.chars().any(|ch| matches!(ch, '\t' | '\n' | '\r')) {
            return Err(ConstructionSchemaCatalogError::InvalidDomain { domain });
        }
        let dump = deterministic_registry_dump.into();
        if let Some(existing) = self.domains.get(&domain) {
            return if existing == &dump {
                Ok(())
            } else {
                Err(ConstructionSchemaCatalogError::ConflictingDomain { domain })
            };
        }
        self.domains.insert(domain, dump);
        Ok(())
    }

    pub fn contains_domain(&self, domain: &str) -> bool {
        self.domains.contains_key(domain)
    }

    /// Byte-stable fingerprint material for every installed construction domain.
    ///
    /// The byte count makes nested registry dumps unambiguous even though they
    /// contain their own newlines. `BTreeMap` order makes plugin installation
    /// order irrelevant.
    pub fn deterministic_dump(&self) -> String {
        use std::fmt::Write as _;

        let mut out = String::from("construction-schema-catalog-v1\n");
        for (domain, dump) in &self.domains {
            let _ = writeln!(out, "domain\t{domain}\t{}", dump.len());
            out.push_str(dump);
            if !dump.ends_with('\n') {
                out.push('\n');
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{ConstructionSchemaCatalog, ConstructionSchemaCatalogError};

    #[test]
    fn catalog_is_order_independent_and_idempotent() {
        let mut a = ConstructionSchemaCatalog::default();
        a.try_contribute("actor", "recipe\ta\n").unwrap();
        a.try_contribute("portal-gun", "recipe\tp\n").unwrap();
        a.try_contribute("actor", "recipe\ta\n").unwrap();

        let mut b = ConstructionSchemaCatalog::default();
        b.try_contribute("portal-gun", "recipe\tp\n").unwrap();
        b.try_contribute("actor", "recipe\ta\n").unwrap();

        assert_eq!(a.deterministic_dump(), b.deterministic_dump());
    }

    #[test]
    fn conflicting_domain_is_refused() {
        let mut catalog = ConstructionSchemaCatalog::default();
        catalog.try_contribute("actor", "v1").unwrap();
        assert_eq!(
            catalog.try_contribute("actor", "v2"),
            Err(ConstructionSchemaCatalogError::ConflictingDomain {
                domain: "actor".to_string(),
            })
        );
    }
}
