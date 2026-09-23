//! The save-flag spellings of a body's durable FATE — a death its respawn
//! policy remembers, and a provocation construction rebuilds.
//!
//! ⛔ NOT A FEATURE MODULE, AND THAT IS WHY IT IS HERE. Two layers name these:
//! the feature systems that WRITE a fate (a death, a provocation) and room
//! construction, which READS it to build the body in that state
//! (`construction::PersistedFates`). Construction must not reach up into
//! `features` (`test_actor_construction_inversion.py`), so the vocabulary both
//! share lives below both.

/// Shared suffix for persistent `_dead_until_rest` flags.
///
/// ⭐ RE-EXPORTED, NOT DECLARED. The save module owns the spelling because it is
/// the one that has to RECOGNISE these ids when a rest clears them, and this
/// crate already depends on it. Two `const`s with a comment asking a reader to
/// keep them in sync is not synchronisation.
pub use ambition_persistence::save_data::DEAD_UNTIL_REST_SUFFIX as ENEMY_DEAD_UNTIL_REST_SUFFIX;

/// The save flag a `DeadStaysDead` placement's death writes.
pub fn enemy_dead_flag(id: &str) -> String {
    format!("enemy_{id}_dead")
}

/// The save flag an `OnRest` placement's death writes — cleared by a rest.
pub fn enemy_dead_until_rest_flag(id: &str) -> String {
    format!("enemy_{id}{ENEMY_DEAD_UNTIL_REST_SUFFIX}")
}

/// The flag a death under `policy` WRITES, or `None` for a policy that keeps no
/// record.
///
/// ⭐ THE POLICY DECIDES THE FLAG IN ONE PLACE, and the match is exhaustive, so a
/// fifth `RespawnPolicy` does not compile until somebody says whether a death
/// under it is remembered. Before this, the death path spelled
/// `format!("enemy_{{}}_dead", ..)` and the load path spelled
/// `format!("enemy_{{id}}_dead")` — four literals for two flags across two files,
/// and the drift is silent in the worst direction: the writer keeps stamping a
/// flag the reader no longer looks for, so a permanent casualty comes back to
/// life on the next load with every test green.
///
/// ⛔⛔ ONLY A POLICY THAT WRITES A FLAG MAY READ ONE. `OnRoomReenter` and
/// `InPlace` keep no record at all, and a body under either used to have its
/// liveness decided by a record its own kind never keeps — which is how ONE
/// summoned shark's death zeroed every later summon sharing its `config.id`.
pub fn enemy_death_flag(
    policy: ambition_entity_catalog::placements::RespawnPolicy,
    id: &str,
) -> Option<String> {
    use ambition_entity_catalog::placements::RespawnPolicy as P;
    match policy {
        P::OnRoomReenter | P::InPlace(_) => None,
        P::OnRest => Some(enemy_dead_until_rest_flag(id)),
        P::DeadStaysDead => Some(enemy_dead_flag(id)),
    }
}

#[cfg(test)]
mod death_flag_tests {
    use super::*;
    use ambition_entity_catalog::placements::RespawnPolicy as P;

    /// ⚠ THE ROUND TRIP IS THE CONTRACT: the flag a death writes is a flag a load
    /// looks for. It cannot fail now that one function feeds both, which is the
    /// point — before this it was a claim about four literals staying in step.
    #[test]
    fn a_policy_that_writes_a_flag_writes_one_a_load_looks_for() {
        assert_eq!(
            enemy_death_flag(P::DeadStaysDead, "EnemySpawn-1"),
            Some(enemy_dead_flag("EnemySpawn-1"))
        );
        assert_eq!(
            enemy_death_flag(P::OnRest, "EnemySpawn-1"),
            Some(enemy_dead_until_rest_flag("EnemySpawn-1"))
        );
        // ⭐ AND THE TWO THAT KEEP NO RECORD SAY SO, which is the half that stops a
        // summon from inheriting a predecessor's death.
        assert_eq!(enemy_death_flag(P::OnRoomReenter, "EnemySpawn-1"), None);
        assert_eq!(enemy_death_flag(P::InPlace(2.0), "EnemySpawn-1"), None);
    }
}

/// The save flag that says this NPC was provoked and stays hostile TO THE
/// PLAYER.
///
/// ⭐ ONE SPELLING. Room construction reads it (`PersistedFates::npc_fate`) and
/// [`record_npc_provocations`](crate::features::record_npc_provocations) writes it; anything else that needs to name the
/// fact — a test, a dev tool — asks here rather than re-deriving the format,
/// because a second `format!` for the same flag is a rename waiting to go
/// silently one-sided.
///
/// ⚠ It is a boolean with no faction in it, and construction rebuilds the person
/// with `Grudge::Faction(Player)`. So it MEANS "persistently hostile to the
/// player", and only a provocation the player caused may set it.
pub fn npc_flag_id(id: &str) -> String {
    format!("npc_{id}_hostile")
}
