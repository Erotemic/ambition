//! App-local boss-encounter spec access.
//!
//! Named encounter data is assembled in [`super::BossCatalog`]. This module
//! exposes a read-only catalog view used by validation and registry code;
//! it owns no process-global content state.

use super::BossCatalog;

/// Boss specs authored by the providers linked into this App.
pub fn default_boss_specs(catalog: &BossCatalog) -> Vec<crate::boss_encounter::BossEncounterSpec> {
    catalog.encounter_specs().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_encounter_has_an_authored_behavior() {
        let catalog = super::super::test_boss_catalog();
        let orphans: Vec<String> = catalog
            .encounter_specs()
            .filter(|spec| catalog.behavior(&spec.id).is_none())
            .map(|spec| spec.id.clone())
            .collect();
        assert!(
            orphans.is_empty(),
            "boss encounters have no matching authored behavior: {orphans:?}"
        );
    }

    #[test]
    fn assembled_boss_specs_have_unique_ids() {
        let specs = default_boss_specs(super::super::test_boss_catalog());
        let mut seen = std::collections::BTreeSet::new();
        let dupes: Vec<String> = specs
            .iter()
            .filter_map(|spec| (!seen.insert(spec.id.clone())).then(|| spec.id.clone()))
            .collect();
        assert!(
            dupes.is_empty(),
            "duplicate assembled boss spec ids: {dupes:?}"
        );
    }
}
