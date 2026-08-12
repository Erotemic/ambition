//! **THE PREFAB REGISTRY** — `key + params -> MoveSpec`, expanded at roster
//! install, with its authored presentation ids validated against what renderers
//! can actually draw.
//!
//! ⛔⛔ **this was the last thing keeping `prefabs.rs` in this crate** (campaign
//! P1.7, 2026-08-12). That module is the build-time half of the Smash model —
//! `attack_move_from_melee`, `directional_attack_variants`, `build_actor_moveset`
//! — and character PREPARATION calls it, so it has to sit at or below
//! `ambition_characters`. Every type it touches already does, with ONE
//! exception: this registry's `expand` validates a move's presentation ids
//! through `ambition_vfx::move_vfx_kind`, and reaching a render-adjacent crate
//! from the character domain is the wrong direction.
//!
//! ⭐ so the registry moved rather than the validation being dropped. Building a
//! move from a spec and EXPANDING an authored prefab key are different jobs; the
//! second is the one that needs to know what a renderer can draw, and it belongs
//! up here with the rest of the combat runtime.
//!
//! ⚠ the engine prefabs it seeds itself with (`simple_melee` / `simple_ranged` /
//! `simple_charge`) stay in `prefabs.rs` and are called from here — a downward
//! call, which is the direction that was already fine.
use super::*;
use ambition_characters::moveset_prefabs::{simple_charge, simple_melee, simple_ranged};

/// A prefab builder: hydrate an authored [`ParamValue`] into the prefab's own
/// params and expand it into a [`MoveSpec`]. `fn`-pointer shaped so the registry
/// stays a plain data table.
pub type MovePrefabBuilder = fn(&ambition_entity_catalog::ParamValue) -> Result<MoveSpec, String>;

/// String-keyed prefab registry (A2 / R2.3): `key + params -> MoveSpec`, expanded
/// at roster install. The engine ships `simple_melee` / `simple_ranged` /
/// `simple_charge`; a content roster names a prefab + params to mint a move with
/// ZERO new code (`sword_slash = simple_melee` + sword params). Content may
/// register its own prefabs for richer shapes.
pub struct MovePrefabRegistry {
    builders: std::collections::BTreeMap<String, MovePrefabBuilder>,
}

impl MovePrefabRegistry {
    /// A registry pre-seeded with the engine-shipped prefabs.
    pub fn with_engine_prefabs() -> Self {
        let mut reg = Self {
            builders: std::collections::BTreeMap::new(),
        };
        reg.register("simple_melee", |p| {
            Ok(simple_melee(&p.hydrate().map_err(|e| e.to_string())?))
        });
        reg.register("simple_ranged", |p| {
            Ok(simple_ranged(&p.hydrate().map_err(|e| e.to_string())?))
        });
        reg.register("simple_charge", |p| {
            Ok(simple_charge(&p.hydrate().map_err(|e| e.to_string())?))
        });
        reg
    }

    /// Register (or override) a prefab builder under `key`.
    pub fn register(&mut self, key: impl Into<String>, builder: MovePrefabBuilder) {
        self.builders.insert(key.into(), builder);
    }

    /// Expand a prefab row into a move named `move_id`. Errors if the key is
    /// unknown (a roster typo) or the authored params don't hydrate.
    pub fn expand(
        &self,
        key: &str,
        params: &ambition_entity_catalog::ParamValue,
        move_id: &str,
    ) -> Result<MoveSpec, String> {
        let builder = self
            .builders
            .get(key)
            .ok_or_else(|| format!("unknown move prefab '{key}'"))?;
        let mut spec = builder(params)?;
        spec.id = move_id.to_string();
        // CM5: reject an unresolvable presentation id (a `Vfx`/`Sfx` typo) at
        // expansion time — the SAME startup-validation gate a bad prefab key or
        // param hits, so authored sound/vfx typos never survive to a silent
        // missing effect. The cosmetic-vfx vocabulary lives in `ambition_vfx`;
        // inject it (entity_catalog can't depend on the render-adjacent crate).
        let problems = spec.presentation_problems(|id| ambition_vfx::move_vfx_kind(id).is_some());
        if !problems.is_empty() {
            return Err(problems.join("; "));
        }
        Ok(spec)
    }

    /// True iff no prefab is registered.
    pub fn is_empty(&self) -> bool {
        self.builders.is_empty()
    }
}

impl Default for MovePrefabRegistry {
    fn default() -> Self {
        Self::with_engine_prefabs()
    }
}
