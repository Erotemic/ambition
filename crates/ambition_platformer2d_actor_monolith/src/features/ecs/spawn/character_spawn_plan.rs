//! Common lowering target for character-body spawn requests.
//!
//! Authoring surfaces remain distinct, but converge on a character plus the
//! placement context required by the shared character-body constructor.
//! `SpawnContext` must contain placement decisions only; character-authored facts
//! belong on the prepared definition.

use ambition_entity_catalog::CharacterId;
use ambition_platformer2d_core as ae;

/// Placement context shared by every character-body construction path.
///
/// Keep surface-specific presentation, relationship, and controller inputs at
/// their own call sites unless the shared constructor genuinely requires them.
pub(crate) struct SpawnContext<'a> {
    /// The authored feature id — stable across rebuilds, and the join key for
    /// save state and debug.
    pub(crate) feature_id: &'a str,
    /// Where the body starts, in world space.
    pub(crate) aabb: ae::Aabb,
}

/// One request to instantiate a character.
pub(crate) struct CharacterSpawnPlan<'a> {
    /// Which prepared `CharacterDefinition` to instantiate.
    character: &'a CharacterId,
    context: SpawnContext<'a>,
}

impl<'a> CharacterSpawnPlan<'a> {
    pub(crate) fn new(character: &'a CharacterId, context: SpawnContext<'a>) -> Self {
        Self { character, context }
    }

    pub(crate) fn context(&self) -> &SpawnContext<'a> {
        &self.context
    }

    /// Resolve the required prepared definition, returning the missing id on
    /// configuration failure.
    pub(crate) fn definition<'r>(
        &self,
        registry: &'r crate::character_runtime::PreparedCharacterRegistry,
    ) -> Result<&'r crate::character_runtime::PreparedCharacterDefinition, &'a CharacterId> {
        registry.get(self.character.as_str()).ok_or(self.character)
    }
}

/// Report an unprepared character on a construction path with a real fallback.
///
/// An empty prepared cast is a composition-level warning. With a published cast,
/// a missing character may warn only when `fallback` will actually build the
/// body; otherwise construction refuses rather than producing a generic body.
///
/// # Panics
/// When a cast exists, `missing` is absent from it, and `fallback` is `None`.
pub(crate) fn report_unprepared_character(
    missing: &str,
    placement: &str,
    prepared: &crate::character_runtime::PreparedCharacterRegistry,
    fallback: Option<&str>,
) {
    assert!(
        prepared.is_empty() || fallback.is_some(),
        "{placement} names character `{missing}`, which this composition has not \
         registered, and nothing else can build this body — so it would spawn as \
         a generic body wearing that character's name. Register the character, or \
         author something that can build it."
    );
    // No published cast is a composition gap, not a per-character content fault.
    if prepared.is_empty() {
        bevy::log::warn!(
            target: "crate::spawn",
            "this composition published NO prepared cast at all, and {placement} \
             names character `{missing}` — so it, and every other character-named \
             placement in this room, falls back to {}. The room expects a cast \
             this host does not register; that is a COMPOSITION gap, not a \
             content one.",
            fallback.unwrap_or("whatever generic is at hand"),
        );
    } else {
        bevy::log::warn!(
            target: "crate::spawn",
            "{placement} names character `{missing}`, which this composition has \
             not registered; it falls back to {}. This is correct only for a \
             BORROWED character in a partial composition.",
            fallback.unwrap_or("whatever generic is at hand"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> SpawnContext<'static> {
        SpawnContext {
            feature_id: "EnemySpawn-1",
            aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0)),
        }
    }

    /// A plan resolves the character it NAMES, and nothing else.
    ///
    /// the other half of this test is now a COMPILE ERROR and that is the stronger form.
    /// `CharacterSpawnPlan::character` is required since AC6, so a plan that names nobody cannot be
    /// constructed to be asked; there is no fallback for it to resolve TO.
    #[test]
    fn a_plan_resolves_the_character_it_names() {
        let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
        let finalized = crate::character_runtime::prepare_and_finalize_for_test(
            ambition_characters::actor::definition::CharacterDefinition::new(
                "npc_busy_beaver",
                "Busy Beaver",
                "test",
            ),
            &crate::character_runtime::CharacterBindings::default(),
        );
        registry.insert_prepared(finalized.prepared);

        let named = CharacterId::new("npc_busy_beaver");
        let plan = CharacterSpawnPlan::new(&named, context());
        assert_eq!(
            plan.definition(&registry)
                .expect("the registry holds this character")
                .id
                .as_str(),
            "npc_busy_beaver",
        );

        let stranger = CharacterId::new("npc_nobody");
        let plan = CharacterSpawnPlan::new(&stranger, context());
        assert!(
            plan.definition(&registry).is_err(),
            "and a name the registry does not hold resolves nothing, or the \
             assertion above is vacuous",
        );
    }

    /// An authored character that is not prepared is a FAULT, not a
    /// fallback.
    ///
    /// poison: make `definition` return a plain `Option` and this stops compiling — which is the
    /// point.
    #[test]
    fn an_authored_character_that_is_not_prepared_is_an_error() {
        let registry = crate::character_runtime::PreparedCharacterRegistry::default();
        let named = CharacterId::new("iron_mary");
        let plan = CharacterSpawnPlan::new(&named, context());
        assert_eq!(
            plan.definition(&registry).err().map(CharacterId::as_str),
            Some("iron_mary"),
            "an empty registry must REPORT the missing character, not silently \
             hand the body back to its archetype",
        );
    }

    /// A registry with somebody in it, so `is_empty()` is false and the rule's
    /// composition carve-out does not apply.
    fn a_cast_of_one() -> crate::character_runtime::PreparedCharacterRegistry {
        let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
        let finalized = crate::character_runtime::prepare_and_finalize_for_test(
            ambition_characters::actor::definition::CharacterDefinition::new("npc_somebody", "Somebody", "test"),
            &crate::character_runtime::CharacterBindings::default(),
        );
        registry.insert_prepared(finalized.prepared);
        registry
    }

    /// A named character nobody registered, with nothing else able to build
    /// the body, is REFUSED.
    ///
    /// This is the whole of P0.1's second half: the type has distinguished the three outcomes
    /// for a while, and the CALLER only warned.
    #[test]
    #[should_panic(expected = "nothing else can build this body")]
    fn a_named_character_with_no_fallback_refuses_the_spawn() {
        report_unprepared_character("iron_mary", "enemy `EnemySpawn-9`", &a_cast_of_one(), None);
    }

    /// …and the two states that must NOT refuse, so the guard above is a rule
    /// rather than a panic.
    ///
    /// this half is the poison for the half above.
    #[test]
    fn a_fallback_or_an_empty_cast_warns_instead_of_refusing() {
        // Something else can build it: a borrowed character in a partial
        // composition, which is a real shipping arrangement.
        report_unprepared_character(
            "plane_swarm",
            "enemy `EnemySpawn-3`",
            &a_cast_of_one(),
            Some("its `patrol_cutter` archetype"),
        );
        // No cast at all: the COMPOSITION is incomplete, not the content, and
        // every character-named placement in the room is equally "missing".
        report_unprepared_character(
            "iron_mary",
            "enemy `EnemySpawn-9`",
            &crate::character_runtime::PreparedCharacterRegistry::default(),
            None,
        );
    }
}
