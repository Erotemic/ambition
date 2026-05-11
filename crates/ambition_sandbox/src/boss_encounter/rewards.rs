use ambition_engine as ae;

use super::{BossEncounterRegistry, MOCKINGBIRD_ENCOUNTER_ID};

/// World-space y-offset applied to the chest's *spawn point*,
/// relative to the mockingbird's spawn anchor. The bird is airborne,
/// so the chest is born just below the bird and then lets gravity
/// (`features::CHEST_FALL_GRAVITY`) carry it the rest of the way to
/// whatever solid block the arena puts under it. A small offset keeps
/// the spawn from clipping the bird's own collision while still
/// reading as "the chest tumbled out of the bird's grip".
const MOCKINGBIRD_CHEST_DROP_OFFSET_Y: f32 = 24.0;
/// Pirate hoard footprint. Deliberately oversized — narratively a
/// pirate galleon's hoard is a *big* chest, and visually we want a
/// satisfying target the player can't miss in the arena.
const MOCKINGBIRD_CHEST_SIZE: ae::Vec2 = ae::Vec2::new(56.0, 56.0);

/// Idempotent sync: when the mockingbird encounter is `Cleared`, make
/// sure the pirate-hoard chest exists in the live arena. Runs each
/// frame from `update_boss_encounters`.
///
/// GENERALIZATION PLAN (see `MOCKINGBIRD_ENCOUNTER_ID` for the broader
/// picture): the body here is not actually mockingbird-specific —
/// "find a boss by encounter id, and if its save state is `Cleared`,
/// drop a chest at its spawn anchor + offset" works for any boss
/// fight. Generalize by:
///   1. Introducing a `BossDeathReward` table keyed by encounter id.
///   2. Replacing the single early-return on `MOCKINGBIRD_ENCOUNTER_ID`
///      with a loop over the table.
///   3. Letting the per-entry `PickupKind` and offset feed in from
///      that table instead of constants.
/// We're keeping the special-case while there's exactly one entry —
/// adding the table for a single user is premature abstraction.
///
/// In its current form the sync ensures the chest:
///   - drops the moment the boss dies (encounter flips to `Cleared`),
///   - re-appears on room re-entry (FeatureRuntime is rebuilt empty by
///     `from_world`, so without this sync the chest would vanish on
///     reload),
///   - mirrors the persisted "looted" flag onto `chest.opened` so a
///     re-spawned chest reads as already-opened.
///
/// Cheap when the encounter isn't cleared, when there is no live
/// mockingbird in the room, or when the chest already exists
/// (`spawn_chest` short-circuits on duplicate id).
pub fn sync_mockingbird_treasure_chest(
    features: &mut crate::features::FeatureRuntime,
    save: &ae::SandboxSaveData,
    registry: &BossEncounterRegistry,
    world: &ae::World,
) {
    // Source of truth is the save: the engine's phase stays at
    // `Death` post-defeat, while the save bumps the boss to `Cleared`
    // once the death animation completes. The save reading also
    // survives room reloads where the encounter state machine is
    // freshly built each time.
    if !matches!(
        save.boss(MOCKINGBIRD_ENCOUNTER_ID),
        ae::PersistedEncounterState::Cleared
    ) {
        return;
    }
    // Pull the on-floor chest position from the boss's authored spawn
    // anchor. We use `spawn` (not `pos`) so the chest lands at the
    // same place on first kill and on every subsequent room reload —
    // the BossRuntime is rebuilt at its LDtk-authored spawn each time.
    let runtime_id = registry
        .runtime_ids
        .get(MOCKINGBIRD_ENCOUNTER_ID)
        .cloned()
        .unwrap_or_else(|| MOCKINGBIRD_ENCOUNTER_ID.to_string());
    let Some(boss) = features.bosses.iter().find(|b| b.id == runtime_id) else {
        return;
    };
    let chest_pos = ae::Vec2::new(boss.spawn.x, boss.spawn.y + MOCKINGBIRD_CHEST_DROP_OFFSET_Y);
    let chest_id = format!("encounter_chest_{MOCKINGBIRD_ENCOUNTER_ID}");
    // Detect first-spawn so we can kick off the falling-chest physics
    // *only* when the chest is genuinely new. `spawn_chest` is
    // idempotent on id, so once the chest is in the runtime, we leave
    // its falling/settled state alone — otherwise the per-frame sync
    // would teleport it back up every tick.
    let just_spawned = features.chests.iter().all(|c| c.id != chest_id);
    features.spawn_chest(
        chest_id.clone(),
        // The "real" payout is the admiral's quest reward; this
        // chest is mostly diegetic, so a small heal is fine. The
        // narrative reward (data chips, batteries, potions) lands
        // in the inventory when the player returns to the admiral
        // — see `quest::grant_pirate_treasure_reward`.
        Some(ae::PickupKind::Custom("pirate_hoard".to_string())),
        chest_pos,
        MOCKINGBIRD_CHEST_SIZE,
    );
    // Mirror the persisted "looted" flag onto the live chest so a
    // save+reload after looting doesn't show the chest as closed.
    let looted = save.flag(&crate::encounter::encounter_reward_looted_flag(
        MOCKINGBIRD_ENCOUNTER_ID,
    ));
    if let Some(chest) = features.chests.iter_mut().find(|c| c.id == chest_id) {
        chest.opened = looted;
        if just_spawned {
            // Born above the floor — let gravity do the rest. After
            // it lands, `chest.falling` clears and the chest behaves
            // like any other static interactable.
            chest.falling = true;
            chest.vel_y = 0.0;
            // If the player has already looted this chest in a prior
            // session, the dramatic drop animation is no longer
            // appropriate — they expect the chest to be sitting open
            // where they left it. Run the same physics tick the live
            // update loop runs, but in a tight in-line loop, so the
            // chest reaches its resting position *this frame* before
            // the renderer ever sees it. Capped iteration count is a
            // safety net against pathological geometry (e.g. a chest
            // spawn over an open pit) — at ~16 ms per virtual tick,
            // 240 iterations covers 3.84 s of fall before bailing.
            if looted {
                let virtual_dt = 1.0 / 60.0;
                for _ in 0..240 {
                    if !chest.falling {
                        break;
                    }
                    crate::features::tick_chest_fall(chest, world, virtual_dt);
                }
            }
        }
    }
}
