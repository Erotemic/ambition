//! Move-prefab registry: `key + params -> MoveSpec` at roster installation.
//!
//! Expansion validates authored presentation ids through a caller-supplied
//! oracle, keeping combat independent of presentation assets and usable in
//! headless builds. Engine-provided prefab builders remain in the lower
//! character authoring layer and are registered here.

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
    /// unknown (a roster typo), the authored params don't hydrate, or the move
    /// names a cosmetic effect `vfx_known` does not recognize.
    ///
    /// `vfx_known` is the caller's answer to *"what can actually be drawn?"* —
    /// pass `ambition_sprite_sheet::fx::is_authored_effect` from anywhere that
    /// links presentation, which answers it from the shipped sheets themselves.
    /// See the module header for why this crate does not name that function.
    pub fn expand(
        &self,
        key: &str,
        params: &ambition_entity_catalog::ParamValue,
        move_id: &str,
        vfx_known: impl Fn(&str) -> bool,
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
        // missing effect.
        let problems = spec.presentation_problems(vfx_known);
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
