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
use crate::features::{ActorFaction, Hitbox, HitboxAnchor, HitboxHits, HitboxLifetime};

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

/// The payload of an [`Effect::DamageBox`]: a world-anchored damage volume an
/// emitter requests. `center` + `faction` are explicit — the emitter resolves
/// its own position and knows its faction, so the executor needs no actor
/// queries (keeping it substrate-free for the `ambition_effects` crate).
pub struct DamageBoxEffect {
    pub center: ae::Vec2,
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
/// emitting technique has already resolved aim, so the executor just
/// builds + spawns. (When `effects` becomes its own crate, the shot type swaps
/// to a substrate-neutral struct; in-lib it reuses the existing spawn request.)
pub enum Effect {
    DamageBox(DamageBoxEffect),
    Summon(SummonSpec),
    Projectiles {
        faction: crate::projectile::ProjectileFaction,
        shots: Vec<ambition_platformer_runtime::projectile::EnemyProjectileSpawn>,
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

/// Generic effect executor: drains [`EffectRequest`]s and makes each happen.
/// Pure executor — every effect carries its own geometry (center / shots /
/// pos), so this needs no actor queries (keeping it substrate-free for the
/// `ambition_effects` crate). Reads in message order (unsorted) to match the
/// per-consumer behavior it replaces; if a future multi-emit race needs a stable
/// order, sort by `owner`'s stable id here.
pub fn apply_effects(mut commands: Commands, mut requests: MessageReader<EffectRequest>) {
    for req in requests.read() {
        match &req.effect {
            Effect::DamageBox(d) => {
                spawn_damage_box(
                    &mut commands,
                    req.owner,
                    d.faction,
                    d.center,
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
