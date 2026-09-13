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

use ambition_combat::components::{CenteredAabb, FeatureId, FeatureName, PickupFeature};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;

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

/// The weapon drop's identity segment — see [`SimId::death_drop`]. Derived from
/// the same `(parent, kind)` its provenance is, so the two can never disagree
/// about which drop this is.
const DROP_KIND_WEAPON: &str = "weapon";

/// A drop states the body it fell out of.
///
/// without this a drop is never DRAWN. `rebuild_dynamic_feature_views` discovers loot the
/// running simulation minted by construction PROVENANCE — "this pickup was not in the room spec" is
/// exactly the condition under which the room-load visual pass could not have seen it. An authored
/// pickup carries [`SpawnOrigin::Authored`] and is filtered out there; these carried NO provenance
/// at all, so the query skipped them, no render family ever claimed them, and
/// `draw_unclaimed_feature_views` gave each one a magenta diagnostic stand-in.
///
/// this is provenance, NOT identity, for the three drops that grant a
/// QUANTITY. A `SimId` would enrol the coin in `TransactionBaseline::capture`,
/// whose roster a room-scoped entity leaves mid-transition — and it would be a
/// second authority over a fact `OwnedItems` already settles, since a checkpoint
/// restores the bag wholesale (`OwnedItemsBaseline`).
///
/// ⭐ [`drop_held_weapon`] IS THE EXCEPTION, and the line is what a drop
/// BECOMES rather than which function spawned it: a weapon becomes an object the
/// player CARRIES, and every durable road that could give it back is keyed by
/// `SimId`. It mints [`SimId::death_drop`]; the other three still mint nothing.
fn dynamic_drop_origin(parent: &SimId, sequence: u64) -> SpawnOrigin {
    SpawnOrigin::Dynamic {
        parent: parent.clone(),
        sequence,
    }
}

/// What EVERY runtime death drop is, regardless of what it drops.
///
/// ⛔⛤ **`A7` SAYS THIS PACKET'S ACCEPTANCE CLAUSE — *"reward policy … does not
/// become an alternative item minting path"* — IS NOT MET, AND THIS IS WHY.**
/// `GroundItem::at_rest` sealed the COMPONENT (`#[non_exhaustive]`,
/// poison-verified at `8ce24653d`); nothing sealed the OCCURRENCE. Each of the
/// four drop sites assembled the same four facts by hand:
///
/// ```text
/// session scope        who retires it when the session ends
/// RoomScopedEntity     which sweep takes it back
/// SpawnOrigin          how a provenance-keyed rebuild discovers it
/// SpawnedThisAttempt   whether an attempt reset un-drops it
/// ```
///
/// ⇒ **AND THE COST OF FORGETTING ONE IS RECORDED RIGHT HERE RATHER THAN
/// HYPOTHETICAL** — [`drop_held_weapon`]'s own doc enumerates what two missing
/// halves cost, because they WERE missing: a session-scoped weapon follows you
/// into the next room at its old coordinates, and a drop stating no parent is a
/// drop nothing can say where it came from. A fifth drop site was one omission
/// away from either.
///
/// ⭐⭐ **THE IDENTITY IS A PARAMETER WITH NO DEFAULT, AND THAT IS DELIBERATE.**
/// Only the weapon mints one today — a weapon becomes an object the player
/// CARRIES, and every durable road that could give it back is keyed by `SimId` —
/// and whether the other three should is a design question this does not answer.
/// What it removes is the ability to answer it by ACCIDENT: a caller states
/// `DropIdentity::Anonymous` or names an id, exactly as a producer of
/// `InCustodyOf` must state its `CustodyDurability` rather than inherit one.
/// Same packet family, same fix, and A4 recorded the reasoning first.
enum DropIdentity {
    /// This drop is not an occurrence: no checkpoint describes it, and an
    /// attempt reset destroys it with nothing able to rebuild it. True of the
    /// coin, the heart and the ability pickup today.
    Anonymous,
    /// This drop IS an occurrence, under an id DERIVED from its parent — see
    /// [`SimId::death_drop`] for why a counter would be wrong.
    Occurrence(SimId),
}

/// Spawn one runtime death drop, carrying everything a drop IS.
///
/// ⛔⛤ **ONE SPAWN ROAD, SO A DROP SITE CANNOT BUILD A PARTIAL ONE.** A bundle
/// helper would still have let a fifth site call `spawn_session_scoped` directly
/// and assemble three of the four facts; this owns the spawn, so the facts are
/// not something a caller supplies at all.
///
/// ⚠ IT TAKES THE SEQUENCE RATHER THAN DERIVING IT: the `DROP_SEQUENCE_*`
/// constants distinguish two drops from ONE parent in the same frame, so they
/// belong to the caller that knows which drop it is making.
fn spawn_death_drop(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    parent: &SimId,
    sequence: u64,
    identity: DropIdentity,
    payload: impl bevy::prelude::Bundle,
) {
    let mut entity = commands.spawn_session_scoped(
        session_scope,
        (
            payload,
            // ⛔ THE THREE FACTS EVERY DROP CARRIES, and the cost of each
            // omission is recorded on `DropIdentity`: which sweep takes it back,
            // how a provenance-keyed rebuild discovers it, and whether an
            // attempt reset un-drops it.
            dynamic_drop_origin(parent, sequence),
            RoomScopedEntity,
            super::attempt::SpawnedThisAttempt,
        ),
    );
    // ⛔ `Option<SimId>` IS NOT A `Bundle` IN THIS BEVY, so the identity is a
    // second insert rather than a fourth field — which is why this owns the
    // SPAWN and not merely the bundle: a caller that assembled the bundle
    // itself would have to remember this line, and remembering is the failure
    // this removes.
    if let DropIdentity::Occurrence(id) = identity {
        entity.insert(id);
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
    spawn_death_drop(
        commands,
        session_scope,
        parent,
        DROP_SEQUENCE_COIN,
        DropIdentity::Anonymous,
        (
            FeatureSimEntity,
            FeatureId::new(format!("coin:{id}")),
            FeatureName::new("Coin"),
            CenteredAabb::from_center_size(pos, ae::Vec2::new(12.0, 12.0)),
            PickupFeature::new(ambition_interaction::Pickup::new(
                format!("coin:{id}"),
                ambition_interaction::PickupKind::Currency { amount },
            )),
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
/// [`Hitbox`](ambition_combat::hitbox::Hitbox) centered on the corpse. Enemy faction, so
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
    prepared: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
    session_scope: SessionSpawnScope,
    parent_id: &str,
    pos: ae::Vec2,
    // AC5.4: WHAT it splits into, from the parent character's own `divides_into`.
    offspring: &str,
) {
    let empty_cast = ambition_characters::prepared::PreparedCharacterRegistry::default();
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
    spawn_death_drop(
        commands,
        session_scope,
        parent,
        DROP_SEQUENCE_HEALTH,
        DropIdentity::Anonymous,
        (
            FeatureSimEntity,
            FeatureId::new(format!("heart:{id}")),
            FeatureName::new("Health"),
            CenteredAabb::from_center_size(pos, ae::Vec2::new(12.0, 12.0)),
            PickupFeature::new(ambition_interaction::Pickup::new(
                format!("heart:{id}"),
                ambition_interaction::PickupKind::Health { amount },
            )),
            // Ambition's OWN combat drops keep the loot magnet, and now say so.
            super::pickups::PickupMagnet::classic(),
        ),
    );
}

/// Spawn a collectible ability pickup at `pos` — a defeated boss's reward. Reuses
/// the standard pickup entity shape so [`super::collect_ecs_pickups`] grants the
/// ability to the player's catalog ([`ambition_items::OwnedItems`]) on overlap.
pub fn drop_ability_pickup(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    parent: &SimId,
    boss_id: &str,
    pos: ae::Vec2,
    ability_id: &str,
    ability_name: &str,
) {
    spawn_death_drop(
        commands,
        session_scope,
        parent,
        DROP_SEQUENCE_ABILITY,
        DropIdentity::Anonymous,
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
/// ⭐ AND IT MINTS AN IDENTITY, alone among the four drops. Measured
/// 2026-09-04: without one this object is invisible to
/// `capture_minted_item_baseline` (`(&SimId, &SpawnOrigin, &GroundItem,
/// &ItemCustody)`), to `capture_custody_baseline` (`(&SimId, &InCustodyOf)`)
/// and to `TransactionBaseline::capture` — so a checkpoint taken while the
/// player holds it has no description of it, and the `SpawnedThisAttempt`
/// sweep a death runs destroys it with nothing able to rebuild it. Every
/// boss's signature gauntlet arrives this way, and the IDENTICAL gauntlet
/// authored as a room placement does carry an identity: the same object was an
/// occurrence or not depending on how it was acquired.
///
/// The id is DERIVED from `(parent, kind)`, exactly as the provenance beside it
/// is — [`SimId::death_drop`] holds why a counter would be wrong here and why
/// the `drop` segment is not decoration.
pub fn drop_held_weapon(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    parent: &SimId,
    pos: ae::Vec2,
    spec: ambition_characters::brain::HeldItemSpec,
    half_extent: ae::Vec2,
    name: &str,
) {
    // ⭐ THE ONLY DROP THAT IS AN OCCURRENCE, and it SAYS SO rather than being
    // the one that happens to carry a `SimId`. Room scope, provenance and
    // attempt state come with it — a weapon that carried an identity and not
    // room scope would FOLLOW YOU into the next room, and one with no
    // provenance is a drop nothing can rebuild. Those two were missing once;
    // now they cannot be.
    spawn_death_drop(
        commands,
        session_scope,
        parent,
        DROP_SEQUENCE_WEAPON,
        DropIdentity::Occurrence(SimId::death_drop(parent, DROP_KIND_WEAPON)),
        (
            ambition_held_items::GroundItem::at_rest(spec, pos, half_extent),
            bevy::prelude::Name::new(name.to_string()),
        ),
    );
}

#[cfg(test)]
mod tests;
