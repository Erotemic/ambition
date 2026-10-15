//! Ambition's own tuning: the pack's `data/ambition_rules.ron` (schema
//! `ambition_rules`, owned by `ambition_content_schemas`). The idle bark
//! cadence of a boss and the GNU-ton duel read it here; a content reload
//! restates it (`reload::publish_ambition_tuning`).

use ambition_content_schemas::rules::{lowered_ambition_rules, AmbitionRules};
use bevy::prelude::{App, Resource};

/// The tuning the selected pack states.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct AmbitionTuning(pub AmbitionRules);

/// The tuning of the shipped pack, for a unit test with no App.
#[cfg(test)]
pub(crate) fn shipped() -> AmbitionRules {
    lowered_ambition_rules(crate::pack::shipped()).expect("Ambition's pack states its tuning (`data/ambition_rules.ron`)")
}

/// Insert the tuning the selected pack states.
pub(super) fn register(app: &mut App) {
    let pack = crate::pack::select(app.world_mut());
    let rules = lowered_ambition_rules(&pack).expect("Ambition's pack states its tuning (`data/ambition_rules.ron`)");
    app.insert_resource(AmbitionTuning(rules));
}
