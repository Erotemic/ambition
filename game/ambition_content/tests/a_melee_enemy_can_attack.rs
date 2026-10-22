//! A character that fights in melee by itself must be able to ATTACK.
//!
//! A row that states its `abilities` states all of them: a body without
//! `Attack` never starts a swing, whatever its brain presses. The Attack Hound
//! and the Searchlight Crawler stated `[Walk, Jump]` and `[Walk]` and never
//! bit or struck anyone; they chased you, stood beside you and did nothing
//! (measured 2026-10-22, while the Spectral Wolf was built from the hound).

use ambition_platformer2d::engine_core::AbilityGrant;

#[test]
fn every_autonomous_melee_fighter_can_attack() {
    let catalog = ambition_content::character_catalog::shipped_catalog();
    let data = catalog.data();
    let mut broken = Vec::new();
    for (id, entry) in &data.characters {
        let Some(abilities) = entry.abilities.as_ref() else {
            continue;
        };
        if entry.autonomous_profile.is_none() || abilities.contains(&AbilityGrant::Attack) {
            continue;
        }
        let key = entry.default_action_set.as_str();
        let melee = data
            .action_set_presets
            .iter()
            .find(|(name, _)| name.as_str() == key || name.ends_with(&format!(":{key}")))
            .is_some_and(|(_, set)| set.melee.is_some());
        if melee {
            broken.push(format!("{id} ({key}): {abilities:?}"));
        }
    }
    assert!(broken.is_empty(), "these fight in melee but cannot attack: {broken:#?}");
}
