//! Loot / drop spawners, split out of `damage.rs` (Refactor 6 from TODO.md:
//! shrink the incremental rebuild unit + improve navigability). These are the
//! pure helpers `apply_actor_hit` / `apply_boss_hit` call when something dies —
//! currency coins, health hearts, ability pickups, the exploding-mite death
//! blast, and the dividing-mite split. A straight code move: no behavior change.

use bevy::prelude::{Commands, Entity};

use super::{CenteredAabb, FeatureId, FeatureName, FeatureSimEntity, PickupFeature};
use crate::engine_core as ae;

/// Deterministic (FNV-1a over the id) gate so ~1 in 4 enemy *kinds* drops a heart.
/// Deterministic, not random, so the headless sim stays reproducible — the same
/// enemy always drops or always doesn't.
pub fn id_drops_health(id: &str) -> bool {
    let h = id
        .bytes()
        .fold(2166136261u32, |a, b| (a ^ b as u32).wrapping_mul(16777619));
    h % 4 == 0
}

/// Spawn a collectible currency coin at `pos` — an enemy's death drop. Reuses the
/// exact pickup entity shape that LDtk-placed coins use, so the already-registered
/// [`super::collect_ecs_pickups`] grants it (and plays `WORLD_COIN_PICKUP`) when a
/// player overlaps it. The coin sits where the enemy fell and never respawns
/// (`Pickup::new` defaults to [`crate::interaction::RespawnPolicy::Never`]).
pub fn drop_currency_coin(commands: &mut Commands, id: &str, pos: ae::Vec2, amount: i32) {
    commands.spawn((
        FeatureSimEntity,
        FeatureId::new(format!("coin:{id}")),
        FeatureName::new("Coin"),
        CenteredAabb::from_center_size(pos, ae::Vec2::new(12.0, 12.0)),
        PickupFeature::new(crate::interaction::Pickup::new(
            format!("coin:{id}"),
            crate::interaction::PickupKind::Currency { amount },
        )),
    ));
}

/// Half-extent (px) of an `ExplodingMite`'s death blast — a wide, readable boom.
const EXPLODER_BLAST_HALF: f32 = 64.0;
/// Damage the blast deals (more than the mite's contact, so a point-blank kill
/// genuinely punishes).
pub(super) const EXPLODER_BLAST_DAMAGE: i32 = 3;
const EXPLODER_BLAST_KNOCKBACK: f32 = 1.6;
/// A brief flash — the box exists just long enough to register one hit.
const EXPLODER_BLAST_LIFETIME_S: f32 = 0.14;

/// Spawn the death blast of a volatile mite: a one-shot **Enemy-faction**
/// [`Hitbox`](crate::features::Hitbox) centered on the corpse. Enemy faction, so
/// `apply_hitbox_damage` routes it at the *player* (not other enemies — the blast
/// doesn't chain), and the player's shield/parry can still negate it. `owner` is
/// the dying mite (moot for ignore-self, since the blast never hits its own side).
///
/// Calls the executor DIRECTLY (not via `Effect::DamageBox`) on purpose: this
/// runs in the hit-resolution stage, AFTER `apply_effects`, so a fire-and-forget
/// `EffectRequest` would land a frame late. Spawning the box here keeps it
/// same-frame (and replay-identical).
pub(super) fn spawn_death_explosion(commands: &mut Commands, owner: Entity, pos: ae::Vec2) {
    crate::effects::spawn_damage_box(
        commands,
        owner,
        crate::features::ActorFaction::Enemy,
        pos,
        crate::effects::DamageBox {
            half_extent: ae::Vec2::splat(EXPLODER_BLAST_HALF),
            damage: EXPLODER_BLAST_DAMAGE,
            knockback: EXPLODER_BLAST_KNOCKBACK,
            lifetime_s: EXPLODER_BLAST_LIFETIME_S,
            name: Some("Exploding mite blast"),
        },
    );
}

/// Lateral offset (px) each split offspring spawns from the parent's corpse.
const SPLIT_OFFSET_X: f32 = 30.0;
/// Half-size of a split offspring (a small-skitter body).
const SPLIT_OFFSPRING_HALF: ae::Vec2 = ae::Vec2::new(15.0, 20.0);

/// A `DividingMite` splits into two fast `SmallSkitter` offspring on death — one
/// to each side — through the runtime-minion spawner. The children are plain
/// skitters (NOT dividers), so the split is exactly one level deep: no runaway
/// recursion, just "kill the slow parent, then handle two quick children."
pub(super) fn spawn_split_offspring(commands: &mut Commands, parent_id: &str, pos: ae::Vec2) {
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        crate::features::spawn_runtime_minion(
            commands,
            format!("{parent_id}:split{i}"),
            "Divided cell",
            pos + ae::Vec2::new(side * SPLIT_OFFSET_X, 0.0),
            SPLIT_OFFSPRING_HALF,
            "SmallSkitter",
            format!("{parent_id}:split"),
            crate::features::ActorFaction::Enemy,
            crate::features::ActorAggression::hostile_to_player(),
        );
    }
}

/// Spawn a collectible health heart at `pos` (a sometimes-drop on enemy defeat),
/// same pickup path as the coin so `collect_ecs_pickups` heals the player on
/// overlap via `PlayerHealRequested`.
pub fn drop_health_pickup(commands: &mut Commands, id: &str, pos: ae::Vec2, amount: i32) {
    commands.spawn((
        FeatureSimEntity,
        FeatureId::new(format!("heart:{id}")),
        FeatureName::new("Health"),
        CenteredAabb::from_center_size(pos, ae::Vec2::new(12.0, 12.0)),
        PickupFeature::new(crate::interaction::Pickup::new(
            format!("heart:{id}"),
            crate::interaction::PickupKind::Health { amount },
        )),
    ));
}

/// Spawn a collectible ability pickup at `pos` — a defeated boss's reward. Reuses
/// the standard pickup entity shape so [`super::collect_ecs_pickups`] grants the
/// ability to the player's catalog ([`crate::items::OwnedItems`]) on overlap.
pub fn drop_ability_pickup(
    commands: &mut Commands,
    boss_id: &str,
    pos: ae::Vec2,
    ability_id: &str,
    ability_name: &str,
) {
    commands.spawn((
        FeatureSimEntity,
        FeatureId::new(format!("ability_drop:{boss_id}")),
        FeatureName::new(ability_name.to_string()),
        CenteredAabb::from_center_size(pos, ae::Vec2::new(16.0, 16.0)),
        PickupFeature::new(crate::interaction::Pickup::new(
            format!("ability_drop:{boss_id}"),
            crate::interaction::PickupKind::Ability {
                ability_id: ability_id.to_string(),
            },
        )),
    ));
}
