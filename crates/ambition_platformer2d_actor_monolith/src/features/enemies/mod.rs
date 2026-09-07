//! Actor spawn/surface state and shared movement integration for brain-driven actors.
//!
//! Grounded, aerial, and adhesive actors all integrate through `ae::step_motion`.

use super::*;

mod integration;
pub use integration::ContactAttack;
pub(crate) use integration::ActorMutIntegrationExt;
#[cfg(test)]
pub(crate) use integration::SeedActorIntegrationTestExt;



/// Shared suffix for persistent `_dead_until_rest` flags.
pub const ENEMY_DEAD_UNTIL_REST_SUFFIX: &str = "_dead_until_rest";

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
