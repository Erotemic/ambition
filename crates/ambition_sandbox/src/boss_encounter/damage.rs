use ambition_engine as ae;

use crate::presentation::cutscene::CutsceneTriggerQueue;

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
    music_request: &mut crate::encounter::EncounterMusicRequest,
    cutscene_queue: &mut CutsceneTriggerQueue,
    banner: &mut crate::features::GameplayBanner,
    boss_runtime_id: &str,
    damage: i32,
) -> Option<BossDamageOutcome> {
    let (id, _) = registry
        .runtime_ids
        .iter()
        .find(|(_id, runtime_id)| runtime_id.as_str() == boss_runtime_id)
        .map(|(id, runtime_id)| (id.clone(), runtime_id.clone()))?;
    let state = registry.encounters.get_mut(&id)?;
    let prev_hp = state.hp;
    let evs = state.apply_player_damage(damage);
    let applied = evs
        .iter()
        .any(|ev| matches!(ev, ae::BossEncounterEvent::DamageApplied { .. }));
    let hp_remaining = state.hp;
    let killed = applied && hp_remaining == 0 && prev_hp > 0;
    publish_events(&id, &evs, music_request, cutscene_queue, banner);
    Some(BossDamageOutcome {
        hp_remaining,
        killed,
        applied,
    })
}
