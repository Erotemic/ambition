//! Effect primitives — the generic, actor-agnostic things a *technique* emits.
//!
//! A technique (a boss special, a wielded ability, an authored hazard) decides
//! *what and when*; an **effect** is the consequence it spawns. Effects are
//! faction-tagged and emitter-relative, so the player, an enemy, and a boss all
//! drive the same execution path — the actor-generic `ShockwaveSlam` is the
//! reference shape. Keeping the spawn here, instead of hand-rolled at each call
//! site, is what lets this logic eventually lift into a standalone
//! `ambition_effects` crate, and what makes "comment out effects → nothing
//! spawns, the rest of the game still runs" a real property.
//!
//! First primitive: [`DamageBox`] (a world-anchored damage volume) — the shared
//! shape behind boss hazards, AOE slams, beams, and death blasts. Damage
//! *resolution* stays in the combat layer (`apply_hitbox_damage`); an effect
//! only describes and spawns the box. The `FollowOwner` melee case keeps its
//! own `mechanics::combat::hitbox::spawn_melee_hitbox`.

use bevy::prelude::*;

use crate::engine_core as ae;
use crate::features::{ActorFaction, FeatureAabb, FeatureSimEntity, Hitbox, HitboxAnchor, HitboxHits, HitboxLifetime};
use crate::player::{BodyKinematics, PlayerEntity};

/// The `DamageBox` effect primitive: a world-anchored, time-limited damage
/// volume. `owner` + `source` faction are supplied at spawn (from the emitter),
/// so one shape serves player AOEs, boss hazards, and enemy death blasts.
pub struct DamageBox {
    pub half_extent: ae::Vec2,
    pub damage: i32,
    pub knockback: f32,
    /// Final lifetime in seconds — callers pass the already-clamped value (the
    /// helper does not re-clamp, to stay byte-identical to the old call sites).
    pub lifetime_s: f32,
    /// Optional inspector/debug name. `None` matches the call sites that spawn
    /// no `Name`; kept exact so the spawned archetype — and replay — is
    /// unchanged.
    pub name: Option<&'static str>,
}

/// Spawn a world-anchored [`DamageBox`] at `center`, owned by `owner` and tagged
/// with `source` faction. Returns the entity so callers that track the box (the
/// rotating-cross arms) can despawn it later.
///
/// The single spawn point for world-anchored damage boxes — consolidates five
/// formerly hand-rolled `(Hitbox, HitboxLifetime, HitboxHits[, Name])` sites.
pub fn spawn_damage_box(
    commands: &mut Commands,
    owner: Entity,
    source: ActorFaction,
    center: ae::Vec2,
    dbox: DamageBox,
) -> Entity {
    let mut e = commands.spawn((
        Hitbox {
            owner,
            source,
            anchor: HitboxAnchor::World { center },
            half_extent: dbox.half_extent,
            damage: dbox.damage,
            knockback_strength: dbox.knockback,
        },
        HitboxLifetime {
            remaining_s: dbox.lifetime_s,
        },
        HitboxHits::default(),
    ));
    if let Some(name) = dbox.name {
        e.insert(Name::new(name));
    }
    e.id()
}

/// Where a [`DamageBoxEffect`] is anchored.
pub enum DamageBoxAt {
    /// At the emitting actor's own position (player kinematics, or a feature
    /// actor's AABB center) — the resolution the shockwave consumer used.
    Emitter,
    /// At an explicit world point (e.g. a pit trap dropped at the player).
    World(ae::Vec2),
}

/// The payload of an [`Effect::DamageBox`]: a world-anchored damage volume an
/// emitter requests. `at` chooses the center (only `Emitter` resolves against
/// the emitter); `faction` is carried explicitly — the emitter knows whether
/// it's the player or a hostile actor, so we don't depend on the emitter being a
/// resolvable player/feature.
pub struct DamageBoxEffect {
    pub at: DamageBoxAt,
    pub faction: ActorFaction,
    pub half_extent: ae::Vec2,
    pub damage: i32,
    pub knockback: f32,
    pub lifetime_s: f32,
    pub name: Option<&'static str>,
}

/// The payload of an [`Effect::Summon`]: bring an entity into being near the
/// emitter. NOT necessarily a friendly minion — a summon may be hostile,
/// neutral, a decoy, or a hazard-carrier; `faction`/`aggression` decide. `id` is
/// caller-supplied (stable across the encounter), so summons are deterministic
/// without a shared spawn counter.
pub struct SummonSpec {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub half_size: ae::Vec2,
    pub archetype_id: String,
    pub encounter_id: String,
    pub faction: ActorFaction,
    pub aggression: crate::features::ActorAggression,
}

/// A composable effect an actor *technique* emits. [`apply_effects`] executes
/// `DamageBox`/`Summon`; the enemy-pool `Projectiles` are materialized by the
/// projectile substrate's own executor (`apply_projectile_effects`, at the
/// projectile-spawn slot) so the shared `ProjectileSeq` ordering is preserved.
///
/// A `Projectiles` burst carries one [`EnemyProjectileSpawn`] per shot — the
/// emitting technique (a shaper) has already resolved aim, so the executor just
/// builds + spawns. (When `effects` becomes its own crate, the shot type swaps
/// to a substrate-neutral struct; in-lib it reuses the existing spawn request.)
pub enum Effect {
    DamageBox(DamageBoxEffect),
    Summon(SummonSpec),
    Projectiles {
        faction: crate::projectile::ProjectileFaction,
        shots: Vec<crate::enemy_projectile::EnemyProjectileSpawn>,
    },
}

/// "This `owner` emitted this `effect`." Written by a technique, drained by
/// [`apply_effects`]. This message seam is what makes the effect system
/// removable: drop the consumer and techniques emit into the void — nothing
/// spawns, the rest of the game still runs.
#[derive(Message)]
pub struct EffectRequest {
    pub owner: Entity,
    pub effect: Effect,
}

/// Resolve an emitter's center for a `DamageBoxAt::Emitter` box: the player by
/// kinematics, else a feature actor by its AABB center. (Faction is carried
/// explicitly on the effect; only the position is resolved here.)
fn resolve_emitter_center(
    owner: Entity,
    players: &Query<&BodyKinematics, With<PlayerEntity>>,
    features: &Query<&FeatureAabb, With<FeatureSimEntity>>,
) -> Option<ae::Vec2> {
    if let Ok(kin) = players.get(owner) {
        Some(kin.pos)
    } else if let Ok(aabb) = features.get(owner) {
        Some(aabb.center)
    } else {
        None
    }
}

/// Generic effect executor: drains [`EffectRequest`]s and makes each happen at
/// the emitter's position with the emitter's faction. The single home for "an
/// actor emitted an effect → it occurs in the world." Reads in message order
/// (unsorted) to match the per-consumer behavior it replaces; if a future
/// multi-emit race needs a stable order, sort by `owner`'s stable id here.
pub fn apply_effects(
    mut commands: Commands,
    mut requests: MessageReader<EffectRequest>,
    players: Query<&BodyKinematics, With<PlayerEntity>>,
    features: Query<&FeatureAabb, With<FeatureSimEntity>>,
) {
    for req in requests.read() {
        match &req.effect {
            Effect::DamageBox(d) => {
                // Faction is explicit on the effect; only an `Emitter`-anchored
                // box needs the emitter's position resolved.
                let center = match d.at {
                    DamageBoxAt::Emitter => {
                        match resolve_emitter_center(req.owner, &players, &features) {
                            Some(c) => c,
                            None => continue,
                        }
                    }
                    DamageBoxAt::World(p) => p,
                };
                spawn_damage_box(
                    &mut commands,
                    req.owner,
                    d.faction,
                    center,
                    DamageBox {
                        half_extent: d.half_extent,
                        damage: d.damage,
                        knockback: d.knockback,
                        lifetime_s: d.lifetime_s,
                        name: d.name,
                    },
                );
            }
            Effect::Summon(s) => {
                crate::features::spawn_runtime_minion(
                    &mut commands,
                    s.id.clone(),
                    s.name.clone(),
                    s.pos,
                    s.half_size,
                    &s.archetype_id,
                    s.encounter_id.clone(),
                    s.faction,
                    s.aggression.clone(),
                );
            }
            // Enemy-pool projectiles are materialized by the projectile
            // substrate's executor (`apply_projectile_effects`) at the spawn
            // slot, so the shared `ProjectileSeq` order is preserved.
            Effect::Projectiles { .. } => {}
        }
    }
}
