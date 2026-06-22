//! Boss damage routing into the encounter phase machine.
//!
//! `record_boss_damage` / `force_boss_death` look up the encounter by its
//! linked boss-runtime id in [`BossEncounterRegistry`], apply the delta to the
//! phase machine (the source of truth for boss HP), and publish the resulting
//! events via [`events::publish_events`]. Returns a [`BossDamageOutcome`]
//! (post-hit HP + `killed`/`applied` flags) so the caller can fire same-frame
//! death VFX. `force_boss_death` bypasses phase invulnerability (environmental
//! kills); `record_boss_damage` honors it.

use crate::cutscene_trigger::CutsceneTriggerQueue;

use super::{events::publish_events, BossEncounterRegistry};

/// Outcome of a single `record_boss_damage` call. Returned so the
/// caller (today: the ECS damage system) can fire death VFX / banner
/// immediately on the same tick the kill landed, instead of waiting
/// for the boss-runtime mirror to observe `boss.alive = false` on the
/// following frame.
#[derive(Clone, Copy, Debug)]
pub struct BossDamageOutcome {
    /// Engine state's HP after the damage was applied (clamped to 0).
    pub hp_remaining: i32,
    /// True iff this damage event drove the engine state's HP to 0
    /// (i.e. the boss just died on this hit). False during invulnerable
    /// phases (Intro / Transition / Stagger) where the engine rejected
    /// the damage outright.
    pub killed: bool,
    /// True iff the engine state accepted the damage delta (false when
    /// the phase was invulnerable). Lets the caller suppress hit VFX
    /// during invulnerable beats so the player gets the right read.
    pub applied: bool,
}

/// Feed a damage delta into the engine boss-encounter state machine.
///
/// Returns `None` when `boss_runtime_id` does not map to a registered
/// encounter (no boss spawned in this room with that id). Returns
/// `Some(BossDamageOutcome)` with the post-hit HP and a `killed` flag
/// otherwise — the caller uses those to drive immediate VFX / banner /
/// debris bursts on the same frame the kill landed.
///
/// The engine `BossEncounterState` is now the source of truth for boss
/// HP. The sandbox-side `BossRuntime.health` mirror is updated each
/// frame by `update_boss_encounters` (see
/// `crate::boss_encounter::systems`), so callers should treat
/// `boss.health` as read-only after the inversion landed in
/// OVERNIGHT-TODO #8.
pub fn record_boss_damage(
    registry: &mut BossEncounterRegistry,
    music_request: &mut crate::encounter::BossEncounterMusicRequest,
    cutscene_queue: &mut CutsceneTriggerQueue,
    banner: &mut crate::features::GameplayBanner,
    boss_runtime_id: &str,
    damage: i32,
) -> Option<BossDamageOutcome> {
    // Live encounter state is keyed per-entity by the boss runtime id, so this
    // is a direct lookup — damage routes to exactly one boss instance (two of
    // the same archetype no longer share an HP pool). `state.spec.id` carries
    // the archetype id for the event publisher (music / save records).
    let state = registry.encounters.get_mut(boss_runtime_id)?;
    let id = state.spec.id.clone();
    let prev_hp = state.hp;
    let evs = state.apply_player_damage(damage);
    let applied = evs.iter().any(|ev| {
        matches!(
            ev,
            crate::boss_encounter::BossEncounterEvent::DamageApplied { .. }
        )
    });
    let hp_remaining = state.hp;
    let killed = applied && hp_remaining == 0 && prev_hp > 0;
    publish_events(&id, &evs, music_request, cutscene_queue, banner);
    Some(BossDamageOutcome {
        hp_remaining,
        killed,
        applied,
    })
}

/// Force a registered boss encounter to die from an environmental rule.
///
/// This bypasses player-damage invulnerability phases while still routing
/// the resulting `Defeated` / phase-change events through the normal boss
/// event publisher.
pub fn force_boss_death(
    registry: &mut BossEncounterRegistry,
    music_request: &mut crate::encounter::BossEncounterMusicRequest,
    cutscene_queue: &mut CutsceneTriggerQueue,
    banner: &mut crate::features::GameplayBanner,
    boss_runtime_id: &str,
) -> Option<BossDamageOutcome> {
    let state = registry.encounters.get_mut(boss_runtime_id)?;
    let id = state.spec.id.clone();
    let prev_hp = state.hp;
    let evs = state.force_death();
    publish_events(&id, &evs, music_request, cutscene_queue, banner);
    Some(BossDamageOutcome {
        hp_remaining: 0,
        killed: prev_hp > 0,
        applied: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss_encounter::roster::BossSpecRoster;
    use crate::encounter::BossEncounterMusicRequest;
    use crate::features::GameplayBanner;

    fn fixture(max_hp: i32) -> BossEncounterRegistry {
        // Reuse the gradient sentinel spec as the template — gives us
        // realistic phase thresholds (0.66 / 0.22) and music ids, then
        // override id + max_hp for the test.
        let mut spec = crate::boss_encounter::BossEncounterSpec::gradient_sentinel();
        spec.id = "test_boss".into();
        spec.name = "Test Boss".into();
        spec.max_hp = max_hp;
        let mut registry = BossEncounterRegistry::default();
        // Live state is keyed by the boss RUNTIME id (per-entity); the spec keeps
        // the archetype id ("test_boss") for event/save routing.
        registry.encounters.insert(
            "test_boss_runtime".into(),
            crate::boss_encounter::BossEncounterState::new(spec),
        );
        registry.link_runtime("test_boss", "test_boss_runtime");
        // Skip Intro -> Phase1 so the boss is damageable.
        let state = registry.encounters.get_mut("test_boss_runtime").unwrap();
        state.phase = crate::boss_encounter::BossEncounterPhase::Phase1;
        state.hp = max_hp;
        registry
    }

    #[test]
    fn record_boss_damage_returns_none_for_unknown_runtime_id() {
        let mut registry = fixture(10);
        let mut music = BossEncounterMusicRequest::default();
        let mut cutscene = CutsceneTriggerQueue::default();
        let mut banner = GameplayBanner::default();
        let outcome = record_boss_damage(
            &mut registry,
            &mut music,
            &mut cutscene,
            &mut banner,
            "nobody",
            5,
        );
        assert!(outcome.is_none());
    }

    #[test]
    fn record_boss_damage_decreases_hp_and_reports_applied() {
        let mut registry = fixture(10);
        let mut music = BossEncounterMusicRequest::default();
        let mut cutscene = CutsceneTriggerQueue::default();
        let mut banner = GameplayBanner::default();
        let outcome = record_boss_damage(
            &mut registry,
            &mut music,
            &mut cutscene,
            &mut banner,
            "test_boss_runtime",
            3,
        )
        .expect("registered boss returns Some");
        assert_eq!(outcome.hp_remaining, 7);
        assert!(outcome.applied);
        assert!(!outcome.killed);
    }

    #[test]
    fn record_boss_damage_kills_boss_when_hp_hits_zero() {
        let mut registry = fixture(4);
        let mut music = BossEncounterMusicRequest::default();
        let mut cutscene = CutsceneTriggerQueue::default();
        let mut banner = GameplayBanner::default();
        let outcome = record_boss_damage(
            &mut registry,
            &mut music,
            &mut cutscene,
            &mut banner,
            "test_boss_runtime",
            10,
        )
        .expect("registered boss returns Some");
        assert_eq!(outcome.hp_remaining, 0);
        assert!(outcome.applied);
        assert!(outcome.killed);
    }

    /// Pin the `prev_hp > 0` guard on `killed`. Re-damaging a boss
    /// whose engine state is already at 0 HP (e.g. because the engine
    /// just transitioned to `Death`) must NOT re-fire the kill flag —
    /// otherwise the caller would route VFX / quest events / save
    /// updates twice for the same death.
    #[test]
    fn record_boss_damage_does_not_re_fire_killed_when_already_dead() {
        let mut registry = fixture(4);
        let mut music = BossEncounterMusicRequest::default();
        let mut cutscene = CutsceneTriggerQueue::default();
        let mut banner = GameplayBanner::default();
        // First hit kills.
        let first = record_boss_damage(
            &mut registry,
            &mut music,
            &mut cutscene,
            &mut banner,
            "test_boss_runtime",
            10,
        )
        .expect("registered boss returns Some");
        assert!(first.killed, "first lethal hit should report killed=true");
        // Second hit on the already-dead boss.
        let second = record_boss_damage(
            &mut registry,
            &mut music,
            &mut cutscene,
            &mut banner,
            "test_boss_runtime",
            5,
        )
        .expect("registered boss returns Some");
        assert!(
            !second.killed,
            "killed must only fire once per encounter death; \
             follow-up damage during Death phase should report killed=false \
             so quest/save side effects don't double-fire"
        );
        assert_eq!(second.hp_remaining, 0);
    }

    #[test]
    fn record_boss_damage_reports_not_applied_during_invulnerable_phase() {
        let mut registry = fixture(10);
        // Flip to an invulnerable phase (Intro / Transition / Stagger). The live
        // state is keyed by the per-entity runtime id.
        registry.encounters.get_mut("test_boss_runtime").unwrap().phase =
            crate::boss_encounter::BossEncounterPhase::Transition;
        let mut music = BossEncounterMusicRequest::default();
        let mut cutscene = CutsceneTriggerQueue::default();
        let mut banner = GameplayBanner::default();
        let outcome = record_boss_damage(
            &mut registry,
            &mut music,
            &mut cutscene,
            &mut banner,
            "test_boss_runtime",
            5,
        )
        .expect("registered boss returns Some");
        // Engine state rejected the damage; HP unchanged.
        assert_eq!(outcome.hp_remaining, 10);
        assert!(!outcome.applied);
        assert!(!outcome.killed);
    }
}
