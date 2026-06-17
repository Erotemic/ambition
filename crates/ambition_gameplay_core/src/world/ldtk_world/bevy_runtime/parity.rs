//! Runtime-spine parity diagnostic: ECS indices vs. the JSON collision world.
//!
//! `check_ldtk_runtime_spine_parity` (Bevy system) compares per-kind counts
//! from sibling `indices` against the JSON-derived `ae::World::blocks`
//! (the current collision authority) and records the result in
//! `LdtkRuntimeSpineParity`. A mismatch signals the typed component path has
//! drifted from the validated JSON; pure diagnostic, drives no gameplay.

use bevy::prelude::{debug, warn, Res, ResMut, Resource};

use crate::engine_core as ae;

use super::indices::{LdtkRuntimeDamageIndex, LdtkRuntimeOneWayIndex, LdtkRuntimeSolidIndex};

/// Parity diagnostic for the runtime-spine migration.
///
/// `expected_*` is the count of matching `ae::Block::kind` entries in the
/// JSON-derived `ae::World::blocks` (collision authority); `runtime_*` is
/// the count seen on the ECS side via the runtime indices. A mismatch is
/// the signal that the typed component path has drifted from the
/// authoritative one — either the LDtk file is missing entities the JSON
/// adapter is synthesizing, or the typed-component spawn is missing a
/// case. The sandbox HUD/debug overlay logs the mismatch warning rather
/// than crashing so the migration can proceed in steps.
#[derive(Resource, Default, Clone, Debug)]
pub struct LdtkRuntimeSpineParity {
    pub expected_solids: usize,
    pub runtime_solids: usize,
    pub expected_one_way: usize,
    pub runtime_one_way: usize,
    pub expected_damage: usize,
    pub runtime_damage: usize,
    pub last_warn: Option<String>,
}

impl LdtkRuntimeSpineParity {
    pub fn solid_match(&self) -> bool {
        self.expected_solids == self.runtime_solids
    }
    pub fn one_way_match(&self) -> bool {
        self.expected_one_way == self.runtime_one_way
    }
    pub fn damage_match(&self) -> bool {
        self.expected_damage == self.runtime_damage
    }
    pub fn all_match(&self) -> bool {
        self.solid_match() && self.one_way_match() && self.damage_match()
    }

    pub fn summary(&self) -> String {
        format!(
            "solids {}/{}  one-way {}/{}  damage {}/{}  match={}",
            self.runtime_solids,
            self.expected_solids,
            self.runtime_one_way,
            self.expected_one_way,
            self.runtime_damage,
            self.expected_damage,
            self.all_match()
        )
    }
}
/// Compare runtime-spine index counts to the JSON-derived collision
/// world. Logs a tracing warning the first time a mismatch appears so
/// the parity bug is visible without spamming every frame; clears the
/// warning when counts converge.
///
/// This is the verification gate for the LDtk runtime-spine roadmap.
/// Once parity holds for a meaningful number of sandbox sessions and
/// hot-reload edits, the JSON adapter's collision arms can retire.
pub fn check_ldtk_runtime_spine_parity(
    world: Res<crate::GameWorld>,
    solid_index: Res<LdtkRuntimeSolidIndex>,
    one_way_index: Res<LdtkRuntimeOneWayIndex>,
    damage_index: Res<LdtkRuntimeDamageIndex>,
    mut parity: ResMut<LdtkRuntimeSpineParity>,
) {
    let mut expected_solids = 0;
    let mut expected_one_way = 0;
    let mut expected_damage = 0;
    for block in &world.0.blocks {
        match block.kind {
            ae::BlockKind::Solid => expected_solids += 1,
            ae::BlockKind::OneWay => expected_one_way += 1,
            ae::BlockKind::Hazard => expected_damage += 1,
            _ => {}
        }
    }
    let next = LdtkRuntimeSpineParity {
        expected_solids,
        runtime_solids: solid_index.count(),
        expected_one_way,
        runtime_one_way: one_way_index.count(),
        expected_damage,
        runtime_damage: damage_index.count(),
        last_warn: parity.last_warn.clone(),
    };
    if next.all_match() {
        if parity.last_warn.is_some() {
            // Counts have converged; clear the warning.
            parity.last_warn = None;
        }
    } else {
        let summary = next.summary();
        if parity.last_warn.as_deref() != Some(summary.as_str()) {
            if std::env::var_os("AMBITION_LDTK_SPINE_WARN").is_some() {
                warn!(target: "ambition::ldtk_runtime_spine", "{}", summary);
            } else {
                debug!(target: "ambition::ldtk_runtime_spine", "{}", summary);
            }
            parity.last_warn = Some(summary);
        }
    }
    parity.expected_solids = next.expected_solids;
    parity.runtime_solids = next.runtime_solids;
    parity.expected_one_way = next.expected_one_way;
    parity.runtime_one_way = next.runtime_one_way;
    parity.expected_damage = next.expected_damage;
    parity.runtime_damage = next.runtime_damage;
}
