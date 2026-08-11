//! **The one thing every authoring surface lowers to before a character body
//! is built.**
//!
//! `NpcSpawn`, `EnemySpawn`, an encounter mob, a summon, a match participant and
//! a programmatic spawn are six ways to ask for the same thing. They stay
//! distinct as AUTHORING, and they converge here:
//!
//! ```text
//! authoring surface
//!         ↓
//! CharacterSpawnPlan { character, controller, context }   ← controller: see below
//!         +
//! PreparedCharacterDefinition
//!         ↓
//! one character-body construction
//!         ↓
//! generic runtime ECS components
//! ```
//!
//! ⚠ **this is an UPSTREAM layer, not a merge of the two existing plans.**
//! `EnemyActorSpawnPlan` and `NpcActorSpawnPlan` are already-LOWERED results:
//! by the time either exists the brain is built, the action set resolved and
//! the seed constructed from the archetype. They share nine of their twelve
//! fields, and every one of the nine is an OUTPUT of resolution — which is
//! evidence of a shared CONSTRUCTOR, not of a shared plan. Merging them would
//! produce one struct that still asks the archetype what the body is.
//!
//! ⚠ **it carries `character` + `context` and NOT YET `controller` or an
//! autonomous-profile override, deliberately.** Those are the plan's other two
//! members and the second axis is the whole point of the design — but the
//! authored-enemy path is the only caller so far, and it has neither: an
//! `EnemySpawn` authors no brain override, and every enemy is autonomous. A
//! vocabulary invented ahead of its callers is how the thing this replaces grew
//! to forty-nine fields. They arrive with the NPC path (which authors an
//! override) and the match path (which authors a controller), each with a
//! reader in the same change.
//!
//! ⛔ **`SpawnContext` is the member that will rot if unwatched.** The rule that
//! keeps it honest: every member is a decision the PLACEMENT made, never a fact
//! the CHARACTER states. A field that a character could author belongs on the
//! definition, and putting it here makes this the next god-object.
//!
//! See `docs/planning/character-template-architecture-2026-08-10.md`, appendix E.

use ambition_entity_catalog::CharacterId;
use ambition_platformer2d_core as ae;

/// What the PLACEMENT decided, as opposed to what the character is.
///
/// Respawn policy, encounter membership and disposition join this as their
/// authoring surfaces migrate; today it carries what the authored enemy path
/// genuinely has in hand.
pub(crate) struct SpawnContext<'a> {
    /// The authored feature id — stable across rebuilds, and the join key for
    /// save state and debug.
    pub(crate) feature_id: &'a str,
    /// The authored display name. ⛔ presentation only: it is not an identity,
    /// and nothing may resolve a character from it.
    pub(crate) feature_name: &'a str,
    /// Where the body starts, in world space.
    pub(crate) aabb: ae::Aabb,
    /// Which side it fights for. A placement decision — the same character can
    /// be an ally in one room and an enemy in another.
    pub(crate) faction: crate::features::ActorFaction,
    /// The room's kinematic paths, for a placement that rides one.
    pub(crate) paths: &'a [(String, ambition_platformer2d_core::KinematicPath)],
}

/// One request to instantiate a character.
pub(crate) struct CharacterSpawnPlan<'a> {
    /// Which `CharacterDefinition` to instantiate.
    ///
    /// ⚠ `Option` only while the migration runs: a placement that has not named
    /// a character falls back to its legacy archetype, and that gap must stay
    /// VISIBLE rather than be filled by guessing from a display name. When the
    /// authored content is migrated this becomes required.
    character: Option<&'a CharacterId>,
    context: SpawnContext<'a>,
}

impl<'a> CharacterSpawnPlan<'a> {
    pub(crate) fn new(character: Option<&'a CharacterId>, context: SpawnContext<'a>) -> Self {
        Self { character, context }
    }

    pub(crate) fn context(&self) -> &SpawnContext<'a> {
        &self.context
    }

    /// The prepared definition this plan names, when the registry has it.
    ///
    /// ⭐ the ONE place construction asks "which character is this body?", so
    /// there is one answer to change when the fallback is removed.
    pub(crate) fn definition<'r>(
        &self,
        registry: &'r crate::character_runtime::PreparedCharacterRegistry,
    ) -> Option<&'r crate::character_runtime::PreparedCharacterDefinition> {
        registry.get(self.character?.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> SpawnContext<'static> {
        SpawnContext {
            feature_id: "EnemySpawn-1",
            feature_name: "Busy Beaver",
            aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0)),
            faction: crate::features::ActorFaction::Enemy,
            paths: &[],
        }
    }

    /// **A plan that names no character resolves no definition, even when its
    /// display name matches one.**
    ///
    /// The same invariant the authored-enemy path is guarded on, pinned at the
    /// layer that will own it once every surface lowers here — so widening the
    /// plan to the NPC, encounter and match paths cannot reintroduce the
    /// name-matching route one caller at a time.
    #[test]
    fn an_unnamed_plan_resolves_nothing() {
        let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
        let finalized = crate::character_runtime::prepare_and_finalize_for_test(
            crate::character_runtime::CharacterDefinition::new(
                "npc_busy_beaver",
                "Busy Beaver",
                "test",
            ),
            &crate::character_runtime::CharacterBindings::default(),
        );
        registry.insert_prepared(finalized.prepared);

        let plan = CharacterSpawnPlan::new(None, context());
        assert!(
            plan.definition(&registry).is_none(),
            "the context's feature_name is exactly this character's display \
             name, and it must not resolve one",
        );

        let named = CharacterId::new("npc_busy_beaver");
        let plan = CharacterSpawnPlan::new(Some(&named), context());
        assert!(
            plan.definition(&registry).is_some(),
            "and naming it does resolve, or the assertion above is vacuous",
        );
    }
}
