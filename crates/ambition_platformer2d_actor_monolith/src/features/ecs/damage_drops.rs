//! Loot / drop spawners for the damage path.
//!
//! The pure helpers `apply_actor_hit` / `apply_boss_hit` (in `damage/`) call
//! these when something dies — currency coins, health hearts, ability pickups,
//! the exploding-mite death blast, and the dividing-mite split. Sibling of
//! `damage/` (which owns hit application) and `damage_predicates`.

use ambition_platformer2d_shared_tangle::construction::SpawnOrigin;
use ambition_platformer2d_shared_tangle::lifecycle::{
    RoomScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::{Commands, Entity};

use super::FeatureSimEntity;
use ambition_combat::components::{CenteredAabb, FeatureId, FeatureName, PickupFeature};
use ambition_platformer2d_core as ae;

/// Which of a body's death drops this is — the `sequence` half of the drop's
/// [`SpawnOrigin::Dynamic`].
///
/// DERIVED, not counted, and for the same reason `SimId::strike_volume` is
/// derived rather than sequenced: `(parent, kind)` determines a death drop
/// completely. A body dies once and drops at most one coin, one heart and one
/// ability pickup, so a counter would number a thing that cannot repeat.
///
/// and now that a counter EXISTS, the choice is worth stating rather than
/// inherited: a counter is rollback state and a derivation is not, so deriving
/// keeps these ordinals stable across a rewind for free.
///
/// these ordinals reach snapshots inside `SpawnOrigin`. Append, never
/// renumber.
const DROP_SEQUENCE_COIN: u64 = 0;
const DROP_SEQUENCE_HEALTH: u64 = 1;
const DROP_SEQUENCE_ABILITY: u64 = 2;
/// The weapon a defeated body was holding. a body drops at most one, so this
/// stays a derivation like its siblings above.
const DROP_SEQUENCE_WEAPON: u64 = 3;

/// A drop states the body it fell out of.
///
/// without this a drop is never DRAWN. `rebuild_dynamic_feature_views` discovers loot the
/// running simulation minted by construction PROVENANCE — "this pickup was not in the room spec" is
/// exactly the condition under which the room-load visual pass could not have seen it. An authored
/// pickup carries [`SpawnOrigin::Authored`] and is filtered out there; these carried NO provenance
/// at all, so the query skipped them, no render family ever claimed them, and
/// `draw_unclaimed_feature_views` gave each one a magenta diagnostic stand-in.
///
/// this is provenance, NOT identity: no `SimId` is minted here. A `SimId`
/// would enrol the coin in `TransactionBaseline::capture`, whose roster a
/// room-scoped entity leaves mid-transition; giving drops durable identity is a
/// step of the reconstruction migration, and drawing them does not wait on it.
fn dynamic_drop_origin(parent: &SimId, sequence: u64) -> SpawnOrigin {
    SpawnOrigin::Dynamic {
        parent: parent.clone(),
        sequence,
    }
}

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
/// (`Pickup::new` defaults to [`ambition_interaction::RespawnPolicy::Never`]).
///
/// `parent` is the identity of the body or prop it fell out of — see
/// [`dynamic_drop_origin`].
pub fn drop_currency_coin(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    parent: &SimId,
    id: &str,
    pos: ae::Vec2,
    amount: i32,
) {
    commands.spawn_session_scoped(
        session_scope,
        (
            FeatureSimEntity,
            FeatureId::new(format!("coin:{id}")),
            FeatureName::new("Coin"),
            CenteredAabb::from_center_size(pos, ae::Vec2::new(12.0, 12.0)),
            PickupFeature::new(ambition_interaction::Pickup::new(
                format!("coin:{id}"),
                ambition_interaction::PickupKind::Currency { amount },
            )),
            // The sim kept publishing a Pickup view for an entity nothing was drawing, so
            // `draw_unclaimed_feature_views` spawned a stand-in for it in the NEW room, every
            // transition, forever.
            RoomScopedEntity,
            dynamic_drop_origin(parent, DROP_SEQUENCE_COIN),
            super::reset::SpawnedThisAttempt,
            // Ambition's OWN combat drops keep the loot magnet, and now say so.
            super::pickups::PickupMagnet::classic(),
        ),
    );
}

/// Half-extent (px) of an `ExplodingMite`'s death blast — a wide, readable boom.
const EXPLODER_BLAST_HALF: f32 = 64.0;
/// Damage the blast deals (more than the mite's contact, so a point-blank kill
/// genuinely punishes).
pub(super) const EXPLODER_BLAST_DAMAGE: i32 = 3;
const EXPLODER_BLAST_KNOCKBACK: f32 = 1.6;
/// A brief flash — the box exists just long enough to register one hit.
const EXPLODER_BLAST_LIFETIME_S: f32 = 0.14;

/// Spawn the death blast of a volatile mite: a one-shot Enemy-faction
/// [`Hitbox`](crate::features::Hitbox) centered on the corpse. Enemy faction, so
/// `apply_hitbox_damage` routes it at the *player* (not other enemies — the blast
/// doesn't chain), and the player's shield/parry can still negate it. `owner` is
/// the dying mite (moot for ignore-self, since the blast never hits its own side).
///
/// Calls the executor DIRECTLY (not via `Effect::DamageBox`) on purpose: this
/// runs in the hit-resolution stage, AFTER `apply_effects`, so a fire-and-forget
/// `EffectRequest` would land a frame late. Spawning the box here keeps it
/// same-frame (and replay-identical).
pub(super) fn spawn_death_explosion(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    owner: Entity,
    pos: ae::Vec2,
) {
    let entity = ambition_combat::strike::spawn_damage_box(
        commands,
        owner,
        ambition_vfx::HitSide::Enemy,
        pos,
        ambition_combat::strike::DamageBox {
            half_extent: ae::Vec2::splat(EXPLODER_BLAST_HALF),
            shape: None,
            damage: EXPLODER_BLAST_DAMAGE,
            knockback: EXPLODER_BLAST_KNOCKBACK,
            lifetime_s: EXPLODER_BLAST_LIFETIME_S,
            name: Some("Exploding mite blast"),
        },
    );
    let mut entity_commands = commands.entity(entity);
    session_scope.apply_to(&mut entity_commands);
}

/// Lateral offset (px) each split offspring spawns from the parent's corpse.
const SPLIT_OFFSET_X: f32 = 30.0;
/// Half-size of a split offspring.
const SPLIT_OFFSPRING_HALF: ae::Vec2 = ae::Vec2::new(15.0, 20.0);

/// A `DividingMite` splits into two offspring on death — one to each side —
/// through the runtime-minion spawner. The children do not divide, so the split
/// is exactly one level deep: no runaway recursion, just "kill the slow parent,
/// then handle two quick children."
///
/// THE CHILDREN ARE PUPPY SLUGS. Skitters are Puppy Slug."*
///
/// the repo had already answered this once: the proving grounds' placement
/// literally named `pg_skitter` is cast as `npc_puppy_slug` today. The split was
/// the site that had not caught up.
///
/// an engine module still names an Ambition creature, and that is AC5.4's
/// remainder rather than this line's. What a character splits into is a CONTENT
/// fact and belongs on the parent's definition; casting it correctly first is
/// what makes moving it a move rather than a decision.
pub(super) fn spawn_split_offspring(
    commands: &mut Commands,
    character_catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    // The offspring are a CHARACTER when one is registered for them — the same
    // resolution every other spawn road does.
    prepared: Option<&crate::character_runtime::PreparedCharacterRegistry>,
    session_scope: SessionSpawnScope,
    parent_id: &str,
    pos: ae::Vec2,
    // AC5.4: WHAT it splits into, from the parent character's own `divides_into`.
    offspring: &str,
) {
    let empty_cast = crate::character_runtime::PreparedCharacterRegistry::default();
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        crate::features::spawn_runtime_minion(
            commands,
            character_catalog,
            authored_sheets,
            prepared.unwrap_or(&empty_cast),
            session_scope,
            format!("{parent_id}:split{i}"),
            "Divided cell",
            pos + ae::Vec2::new(side * SPLIT_OFFSET_X, 0.0),
            SPLIT_OFFSPRING_HALF,
            offspring,
            format!("{parent_id}:split"),
            ambition_combat::components::ActorFaction::Enemy,
            ambition_combat::components::ActorAggression::hostile(),
        );
    }
}

/// Spawn a collectible health heart at `pos` (a sometimes-drop on enemy defeat),
/// same pickup path as the coin so `collect_ecs_pickups` heals the player on
/// overlap via `PlayerHealRequested`.
pub fn drop_health_pickup(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    parent: &SimId,
    id: &str,
    pos: ae::Vec2,
    amount: i32,
) {
    commands.spawn_session_scoped(
        session_scope,
        (
            FeatureSimEntity,
            FeatureId::new(format!("heart:{id}")),
            FeatureName::new("Health"),
            CenteredAabb::from_center_size(pos, ae::Vec2::new(12.0, 12.0)),
            PickupFeature::new(ambition_interaction::Pickup::new(
                format!("heart:{id}"),
                ambition_interaction::PickupKind::Health { amount },
            )),
            // Room-scoped for the same reason as the coin above.
            RoomScopedEntity,
            dynamic_drop_origin(parent, DROP_SEQUENCE_HEALTH),
            super::reset::SpawnedThisAttempt,
            // Ambition's OWN combat drops keep the loot magnet, and now say so.
            super::pickups::PickupMagnet::classic(),
        ),
    );
}

/// Spawn a collectible ability pickup at `pos` — a defeated boss's reward. Reuses
/// the standard pickup entity shape so [`super::collect_ecs_pickups`] grants the
/// ability to the player's catalog ([`crate::items::OwnedItems`]) on overlap.
pub fn drop_ability_pickup(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    parent: &SimId,
    boss_id: &str,
    pos: ae::Vec2,
    ability_id: &str,
    ability_name: &str,
) {
    commands.spawn_session_scoped(
        session_scope,
        (
            FeatureSimEntity,
            FeatureId::new(format!("ability_drop:{boss_id}")),
            FeatureName::new(ability_name.to_string()),
            CenteredAabb::from_center_size(pos, ae::Vec2::new(16.0, 16.0)),
            PickupFeature::new(ambition_interaction::Pickup::new(
                format!("ability_drop:{boss_id}"),
                ambition_interaction::PickupKind::Ability {
                    ability_id: ability_id.to_string(),
                },
            )),
            // Room-scoped for the same reason as the coin above, and it is the
            // reason that matters rather than the design question. Whether a
            // boss's reward SHOULD survive the room is arguable; that its
            // picture is a `RoomVisual` — and therefore room-scoped — is not.
            // Session-scoped here meant the sim kept publishing a Pickup view
            // for an entity nothing was drawing, which is the stand-in loop the
            // coin's comment describes, on the longest-lived drop in the game.
            RoomScopedEntity,
            dynamic_drop_origin(parent, DROP_SEQUENCE_ABILITY),
            super::reset::SpawnedThisAttempt,
        ),
    );
}

/// Spawn the weapon a defeated body was holding as a `GroundItem` at `pos` — a
/// pirate's gun-sword, a boss's signature gauntlet — so the player can pick it
/// up and wield it through the ordinary item road.
///
/// What the two missing halves cost, stated separately because they fail
/// differently:
///
/// ```text
/// RoomScopedEntity   the roster a room CHANGE retires is
///                    `(With<RoomScopedEntity>, Without<InCustodyOf>)`, so a
///                    session-scoped weapon on the floor is not in it — it
///                    FOLLOWS YOU into the next room, at its old coordinates,
///                    pickup-able there, while everything else that fell in
///                    that fight stays behind
/// SpawnOrigin        `rebuild_dynamic_feature_views` discovers runtime-minted
///                    loot by construction PROVENANCE; a drop that states no
///                    parent is a drop nothing can say where it came from
/// ```
///
/// provenance, NOT identity — deliberately, and for the same reason the
/// coin gives: no `SimId` is minted here. Giving death drops durable identity
/// is a step of the reconstruction migration and this is not it.
pub fn drop_held_weapon(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    parent: &SimId,
    pos: ae::Vec2,
    spec: ambition_characters::brain::HeldItemSpec,
    half_extent: ae::Vec2,
    name: &str,
) {
    commands.spawn_session_scoped(
        session_scope,
        (
            crate::items::pickup::GroundItem {
                spec,
                pos,
                vel: ae::Vec2::ZERO,
                half_extent,
            },
            bevy::prelude::Name::new(name.to_string()),
            // Room-scoped for the same reason as the coin above — and here the
            // symptom is the object itself rather than its picture.
            RoomScopedEntity,
            dynamic_drop_origin(parent, DROP_SEQUENCE_WEAPON),
            // The attempt produced it; the attempt's reset takes it back.
            //
            // the boss's gauntlet did NOT carry this and its sibling reward
            // did. Both fall out of one death, and an attempt reset that
            // un-fights the boss while leaving half its loot on the floor is two
            // answers to one question. They agree now.
            super::reset::SpawnedThisAttempt,
        ),
    );
}

#[cfg(test)]
mod tests;
