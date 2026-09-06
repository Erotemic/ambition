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
        })
        .expect("the three engine prefabs are distinct literal keys in a fresh registry");
        reg.register("simple_ranged", |p| {
            Ok(simple_ranged(&p.hydrate().map_err(|e| e.to_string())?))
        })
        .expect("the three engine prefabs are distinct literal keys in a fresh registry");
        reg.register("simple_charge", |p| {
            Ok(simple_charge(&p.hydrate().map_err(|e| e.to_string())?))
        })
        .expect("the three engine prefabs are distinct literal keys in a fresh registry");
        reg
    }

    /// Register a prefab builder under `key`. REFUSES a key that is already
    /// taken, naming it.
    ///
    /// ⛔⛔ IT USED TO SAY "register (or override)" AND OVERRIDE SILENTLY, and
    /// nothing overrode anything: measured 2026-09-05, the only three
    /// registrations in the workspace are the engine seeds below, under three
    /// distinct literal keys. ⇒ The override was a capability no caller used and
    /// a hazard every caller inherited — two content packs naming one prefab key
    /// would have had the second quietly win, and the moves the first pack
    /// authored would expand into somebody else's shape with no error anywhere.
    ///
    /// ⭐ ANY SECOND REGISTRATION IS A CONFLICT, AND THERE IS DELIBERATELY NO
    /// IDEMPOTENT CASE — which is why this does NOT adopt
    /// `ambition_registry_core::classify` the way a keyed table normally should.
    /// `classify` decides its three answers with `PartialEq`, and the value here
    /// is a `fn` POINTER: the compiler is free to merge identical functions and
    /// free not to, so two registrations of the same builder may compare equal
    /// or unequal depending on optimisation settings. A registry that cannot
    /// soundly recognise "the same entry" has no honest Idempotent arm, and
    /// pretending otherwise would make the answer depend on the build.
    ///
    /// ⚠ THE HAZARD IS CURRENTLY UNREACHABLE AND THAT IS WORTH KNOWING RATHER
    /// THAN HIDING: every `expand` call in the workspace is in this module's own
    /// tests, so no shipped code has ever asked this registry for a move. Fixed
    /// anyway, because the seam exists to be used and the first content pack to
    /// use it should not be the one that discovers this.
    pub fn register(
        &mut self,
        key: impl Into<String>,
        builder: MovePrefabBuilder,
    ) -> Result<(), String> {
        let key = key.into();
        if self.builders.contains_key(&key) {
            return Err(format!(
                "move prefab '{key}' is already registered — a prefab key is what \
                 an authored roster names to mint a move, so a second builder \
                 under it would silently change what every existing row expands to"
            ));
        }
        self.builders.insert(key, builder);
        Ok(())
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

#[cfg(test)]
mod refusal_tests {
    use super::*;

    /// ⛔⛔ A SECOND BUILDER UNDER ONE PREFAB KEY IS REFUSED, NOT ADOPTED.
    ///
    /// A prefab key is what an authored roster names to mint a move. Letting a
    /// second registration win silently means every row that already named the
    /// key expands into somebody else's shape, with no error at startup and no
    /// error at expansion — the move simply becomes a different move.
    ///
    /// ⭐ THE SECOND HALF IS THE ONE THAT MATTERS: the refusal must not have
    /// EATEN the original. A registry that rejects the newcomer and also drops
    /// the incumbent is a worse failure than the overwrite, and both look like
    /// "register returned an error".
    #[test]
    fn a_second_builder_under_one_key_is_refused_and_the_first_survives() {
        let mut reg = MovePrefabRegistry::with_engine_prefabs();
        let err = reg
            .register("simple_melee", |_| Err("the impostor".to_string()))
            .expect_err("a taken prefab key accepted a second builder");
        assert!(
            err.contains("simple_melee"),
            "the refusal does not name the key it refused ({err}) — a startup \
             error nobody can locate is barely better than a silent overwrite"
        );

        // The incumbent still expands, and still expands as ITSELF.
        let params = ambition_entity_catalog::ParamValue::default();
        let expanded = reg.expand("simple_melee", &params, "probe", |_| true);
        assert!(
            expanded.is_ok(),
            "the refused registration destroyed the builder that was already \
             there: {expanded:?}"
        );

        // ⛔ POISON GUARD. Every assertion above would also hold on a registry
        // that had no `simple_melee` at all, if `expand` were tolerant.
        assert!(
            reg.register("a_key_nobody_took", |_| Err("x".to_string())).is_ok(),
            "poison: this registry refuses EVERY registration, so the refusal \
             above says nothing about duplicate keys"
        );
    }
}
