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

/// **Where this instance goes and what it is called at runtime** — the part of
/// a spawn request that is universal to instantiating a character.
///
/// ⛔⛔ **DELIBERATELY SMALLER THAN THE FIRST CALLER NEEDS, and it briefly was
/// not.** It also carried the authored display NAME, the FACTION and the room's
/// kinematic PATHS, because the authored enemy path has all three in hand. Each
/// is a real placement decision, so the rule *"every member is something the
/// placement decided"* admitted them — and that rule turns out to be necessary
/// but not sufficient. A placement can decide plenty that belongs to ONE
/// authoring surface rather than to the shared constructor:
///
/// ```text
/// display name   presentation / debug label
/// faction        relationship policy
/// room paths     autonomous-controller placement input
/// ```
///
/// A match seat, a summon and a programmatic spawn should not have to
/// manufacture a room-style name or an empty path list to use the common
/// constructor. Those facts stay at their own call sites until a SECOND caller
/// shows they are shared — at which point they want their own contextual types
/// (`InitialRelations`, `AutonomousControllerContext`, presentation
/// attachments), not more members here.
pub(crate) struct SpawnContext<'a> {
    /// The authored feature id — stable across rebuilds, and the join key for
    /// save state and debug.
    pub(crate) feature_id: &'a str,
    /// Where the body starts, in world space.
    pub(crate) aabb: ae::Aabb,
}

/// One request to instantiate a character.
pub(crate) struct CharacterSpawnPlan<'a> {
    /// Which `CharacterDefinition` to instantiate.
    ///
    /// ⛔⛔ **REQUIRED, since AC6** (2026-08-14). This was `Option` for one
    /// reason: *"a placement that has not named a character falls back to its
    /// legacy archetype, and that gap must stay VISIBLE"*. The gap closed from
    /// the other end — the archetype ontology is deleted, so an unnamed
    /// character does not fall back to anything, and an `Option` here would
    /// describe a road that no longer exists while quietly reintroducing the
    /// question *what builds a body that names nobody*. There is no answer, and
    /// the type is now the one that says so.
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

    /// The prepared definition this plan names.
    ///
    /// ⭐ the ONE place construction asks "which character is this body?".
    ///
    /// ⛔ **TWO OUTCOMES, and the third one is now a TYPE.** This returned
    /// `Result<Option<_>, _>` so that *"this placement has not been migrated"*
    /// could be told apart from *"this placement names a character that is not
    /// registered"* — the distinction whose collapse once let a spawn authored
    /// as `IronMary` keep its shark-rider archetype silently. The first of those
    /// two is no longer a state a plan can be in ([`Self::character`]), so what
    /// remains is the fault:
    ///
    /// ```text
    /// Ok(d)     prepared      → the character decides
    /// Err(id)   NOT prepared  → a configuration fault
    /// ```
    pub(crate) fn definition<'r>(
        &self,
        registry: &'r crate::character_runtime::PreparedCharacterRegistry,
    ) -> Result<&'r crate::character_runtime::PreparedCharacterDefinition, &'a CharacterId> {
        registry.get(self.character.as_str()).ok_or(self.character)
    }
}

/// **What answers a missing character on the ONE road that still has a fallback
/// to name.**
///
/// ⛔⛔ **its population was four roads and is now one** (AC6, 2026-08-14), and
/// that is the honest reading rather than a loss. This rule exists to decide
/// between warning and refusing, and warning is only defensible when something
/// else will build the body. Three of its four callers had nothing:
///
/// ```text
/// authored enemy    the plan REFUSES at preparation (`preflight_planned_bodies`)
/// boss summon       the same, on the summon batch
/// encounter wave    passed `Some("its archetype")` — and the archetype is deleted
/// peaceful NPC      ← the fallback is REAL: the catalog row's body, kit borrowed
/// ```
///
/// The NPC road is different in kind, not merely unmigrated: an NPC's BODY comes
/// from its catalog row, so an unregistered-but-cataloged character still gets
/// the right body and only the KIT is borrowed. What has no fallback even there
/// is a character in NEITHER, where the road drops to a display-name match — a
/// person built by resembling somebody — and that is what this refuses.
///
/// The rule has three outcomes and the middle one is the reason it is not just a
/// panic:
///
/// ```text
/// no cast published at all  → ONE warning about the COMPOSITION, never the content
/// a fallback can build it   → warn: correct only for a BORROWED character
/// nothing can build it      → REFUSE
/// ```
///
/// ⚠ **`prepared.is_empty()` is not defensive padding, it is a second defect kept
/// out of this one's way.** Several hosts — the multi-game shell, the rollback
/// door fixture — reach construction with a prepared registry containing ZERO
/// characters, measured. There EVERY placement's character is "missing", and
/// refusing would blame the content for a composition gap.
///
/// ⛔ **`fallback` is what will build the body INSTEAD, not whether one exists in
/// principle.** Pass `None` only when the answer is *a generic body wearing this
/// character's name*, because that is the original Iron Mary defect and it looks
/// exactly like a working spawn.
///
/// # Panics
/// When this composition published a cast, this character is not in it, and
/// `fallback` is `None`.
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
    // ⭐ **TWO DIFFERENT FACTS, SAID DIFFERENTLY** (ledger D75). A per-placement
    // warning about a missing character reads as *this content is wrong*, and in
    // a host that published NO CAST AT ALL that is a lie repeated once per
    // placement: the composition is what is incomplete, and every character in
    // the room is equally "missing".
    //
    // ⚠ absence is legitimate — `CharacterPreparationPlugin` is installed by
    // `try_register_character`, so a host that registers nobody never publishes,
    // and "no cast" is exactly what that means. What must not happen is a room
    // full of character-named placements quietly becoming generics with nothing
    // said about WHY.
    if prepared.is_empty() {
        bevy::log::warn!(
            target: "ambition_platformer2d_actor_monolith::spawn",
            "this composition published NO prepared cast at all, and {placement} \
             names character `{missing}` — so it, and every other character-named \
             placement in this room, falls back to {}. The room expects a cast \
             this host does not register; that is a COMPOSITION gap, not a \
             content one.",
            fallback.unwrap_or("whatever generic is at hand"),
        );
    } else {
        bevy::log::warn!(
            target: "ambition_platformer2d_actor_monolith::spawn",
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

    /// **A plan resolves the character it NAMES, and nothing else.**
    ///
    /// ⛔ **the other half of this test is now a COMPILE ERROR** and that is the
    /// stronger form. It used to assert that a plan naming no character resolved
    /// none *"however its placement is labelled"* — the guard against a body
    /// being matched by display name. `CharacterSpawnPlan::character` is
    /// required since AC6, so a plan that names nobody cannot be constructed to
    /// be asked; there is no fallback for it to resolve TO. What survives here
    /// is the positive half, which the assertion above was measured against.
    #[test]
    fn a_plan_resolves_the_character_it_names() {
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

    /// **An authored character that is not prepared is a FAULT, not a
    /// fallback.**
    ///
    /// The state this separates out: content says `IronMary`, her registration
    /// is missing for any reason, and the body quietly keeps the archetype it
    /// would have had — which is the exact defect this campaign exists to
    /// remove, reproduced by the machinery meant to remove it.
    ///
    /// ⛔ poison: make `definition` return a plain `Option` and this stops
    /// compiling — which is the point. The fault is in the type, so it cannot be
    /// lost by a caller forgetting to check. ⚠ it used to have to be
    /// distinguishable from an UNMIGRATED placement because only one of the two
    /// could fall back; there is nothing to fall back to now, so the type
    /// carries one distinction instead of two.
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
            crate::character_runtime::CharacterDefinition::new("npc_somebody", "Somebody", "test"),
            &crate::character_runtime::CharacterBindings::default(),
        );
        registry.insert_prepared(finalized.prepared);
        registry
    }

    /// **A named character nobody registered, with nothing else able to build
    /// the body, is REFUSED.**
    ///
    /// This is the whole of P0.1's second half: the type has distinguished the
    /// three outcomes for a while, and the CALLER only warned. A body that would
    /// come out generic wearing somebody's name is the original defect, and the
    /// campaign's rule is that it must not be reachable quietly.
    #[test]
    #[should_panic(expected = "nothing else can build this body")]
    fn a_named_character_with_no_fallback_refuses_the_spawn() {
        report_unprepared_character("iron_mary", "enemy `EnemySpawn-9`", &a_cast_of_one(), None);
    }

    /// **…and the two states that must NOT refuse, so the guard above is a rule
    /// rather than a panic.**
    ///
    /// ⛔ this half is the poison for the half above. A refusal that also fires
    /// on a borrowed character or on a host with no cast would be indistinguishable
    /// from one that fires correctly — and it would refuse two shipping
    /// compositions, which is how a guard gets deleted instead of obeyed.
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
