//! Reusable effect vocabulary + executor.
//!
//! A *technique* (a boss special, a wielded ability, an authored hazard) decides
//! *what and when*; an **effect** is the consequence it spawns. Effects are
//! faction-tagged and carry their own geometry, so the player, an enemy, and a
//! boss all drive the same execution path. The message seam ([`EffectRequest`])
//! is what makes the effect system removable: drop the consumer and techniques
//! emit into the void — nothing spawns, the rest of the game still runs.
//!
//! This crate owns the world-anchored [`Hitbox`] damage-box component and the
//! `DamageBox` executor; damage *resolution* (`apply_hitbox_damage`) and the
//! `Summon` / `Projectiles` executors live in the game lib next to their
//! substrate (the enemy roster, the projectile pool), reading this crate's
//! [`Effect`] enum.

use bevy::prelude::*;

use ambition_characters::actor::ActorFaction;
use ambition_engine_core as ae;
use ambition_platformer_primitives::projectile::EnemyProjectileSpawn;

pub mod vfx;
pub use vfx::{
    explosion_sfx, ExplosionKind, ExplosionRequest, FireworksRequest, ParticleKind, VfxMessage,
};

// ===================================================================
// Hitbox — the world-anchored damage volume an effect spawns.
// ===================================================================

/// One in-flight strike's damage volume. Spawned on the windup → active edge of
/// an attack (or by a `DamageBox` effect); despawned when its [`HitboxLifetime`]
/// expires. Damage resolution (`apply_hitbox_damage`) lives in the game lib.
#[derive(Component, Clone, Debug)]
pub struct Hitbox {
    /// Entity that spawned the hitbox (skip self-hits; look up the follow
    /// anchor's world position each tick).
    pub owner: Entity,
    /// Whose attack is this? Picks the target query in damage resolution.
    pub source: ActorFaction,
    /// `FollowOwner` re-resolves the AABB each tick from the owner's
    /// authoritative position; `World` is a fixed world-space rectangle.
    pub anchor: HitboxAnchor,
    pub half_extent: ae::Vec2,
    /// Optional non-box strike shape (rotated box, disc, convex arc), in local
    /// space (`+x` = forward) and placed at the anchor center. `None` falls back
    /// to the `half_extent` box, so existing strikes are unchanged.
    pub shape: Option<ae::VolumeShape>,
    /// Facing baked at spawn — mirrors an asymmetric `shape`. `1.0` for boxes /
    /// discs (mirror-invariant).
    pub facing: f32,
    pub damage: i32,
    pub knockback_strength: f32,
    /// Signed horizontal slash impulse (gravity-relative, victim-local x) carried
    /// by a Player-faction melee strike — the unified analogue of the old
    /// per-frame `HitSource::PlayerSlash { knock_x }`. `0.0` for aggressor strikes
    /// (Enemy/Boss/Npc), which knock back via position-derived `knockback_strength`.
    pub knock_x: f32,
    /// The owner's gravity "down" baked at spawn — the frame a non-box `shape`
    /// is placed in, so an authored slash arc / cone rotates with the body's
    /// gravity instead of pinning to screen-down (fable review 2026-07-02 §B10).
    /// World-anchored hazards author in world space and pass screen-down.
    pub frame_down: ae::Vec2,
}

#[derive(Clone, Copy, Debug)]
pub enum HitboxAnchor {
    /// Melee swing — the hitbox tracks the owner's `pos` each tick with a
    /// per-strike local offset baked at spawn (facing encoded in
    /// `local_offset.x`'s sign, so a flipped attacker needs no re-spawn).
    FollowOwner { local_offset: ae::Vec2 },
    /// Arena hazard / boss special — fixed world-space rectangle.
    #[allow(dead_code)]
    World { center: ae::Vec2 },
}

#[derive(Component, Clone, Copy, Debug)]
pub struct HitboxLifetime {
    pub remaining_s: f32,
}

/// Hit-once set: targets the hitbox already damaged this strike, so a long
/// active window can't re-hit a stationary target every frame.
#[derive(Component, Default, Debug)]
pub struct HitboxHits {
    pub hit: std::collections::HashSet<Entity>,
}

impl Hitbox {
    /// Re-resolve this hitbox's world-space volume. Computed every tick rather
    /// than mirrored on the entity so a moving owner needs no per-frame update.
    /// Uses the authored `shape` when present (placed + facing-mirrored at the
    /// anchor center); otherwise the `half_extent` box.
    pub fn world_volume(&self, owner_pos: ae::Vec2) -> ae::CombatVolume {
        let center = match self.anchor {
            HitboxAnchor::FollowOwner { local_offset } => owner_pos + local_offset,
            HitboxAnchor::World { center } => center,
        };
        match &self.shape {
            Some(shape) => shape.place_at(center, self.facing, self.frame_down),
            None => ae::CombatVolume::aabb(ae::Aabb::new(center, self.half_extent)),
        }
    }

    /// Conservative axis-aligned bounds of [`Self::world_volume`] — for the
    /// debug overlay and any broad-phase callers.
    pub fn world_aabb(&self, owner_pos: ae::Vec2) -> ae::Aabb {
        self.world_volume(owner_pos).bounds()
    }
}

// ===================================================================
// DamageBox primitive — spawn a world-anchored Hitbox.
// ===================================================================

/// The `DamageBox` effect primitive: a world-anchored, time-limited damage
/// volume. `owner` + `source` faction are supplied at spawn, so one shape serves
/// player AOEs, boss hazards, and enemy death blasts.
pub struct DamageBox {
    pub half_extent: ae::Vec2,
    /// Optional non-box shape (a disc for an explosion, a convex blast cone).
    /// `None` = the `half_extent` box. The world-anchored counterpart to the
    /// player/enemy melee `shape`.
    pub shape: Option<ae::VolumeShape>,
    pub damage: i32,
    pub knockback: f32,
    /// Final lifetime in seconds — callers pass the already-clamped value.
    pub lifetime_s: f32,
    /// Optional inspector/debug name. `None` matches the call sites that spawn
    /// no `Name` (kept exact so the spawned archetype — and replay — is unchanged).
    pub name: Option<&'static str>,
}

/// Spawn a world-anchored [`DamageBox`] at `center`, owned by `owner` and tagged
/// with `source` faction. Returns the entity so callers that track the box (the
/// rotating-cross arms) can despawn it later. The single spawn point for
/// world-anchored damage boxes.
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
            shape: dbox.shape.clone(),
            facing: 1.0,
            damage: dbox.damage,
            knockback_strength: dbox.knockback,
            knock_x: 0.0,
            // World-anchored volumes are authored in world space (arena
            // hazards); screen-down IS their frame.
            frame_down: ae::Vec2::new(0.0, 1.0),
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

// ===================================================================
// Effect vocabulary + the message seam + the executor.
// ===================================================================

/// The payload of an [`Effect::DamageBox`]: a world-anchored damage volume.
/// `center` + `faction` are explicit — the emitter resolves its own position and
/// knows its faction, so the executor needs no actor queries.
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
/// emitter. NOT necessarily a friendly minion — `faction` decides. `id` is
/// caller-supplied (stable across the encounter), so summons are deterministic
/// without a shared spawn counter. Executed lib-side (the enemy roster).
pub struct SummonSpec {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub half_size: ae::Vec2,
    pub archetype_id: String,
    pub encounter_id: String,
    pub faction: ActorFaction,
}

/// A composable effect an actor *technique* emits. [`apply_effects`] executes
/// `DamageBox`; `Summon` and the enemy-pool `Projectiles` are materialized by
/// lib-side executors next to their substrate (so the shared `ProjectileSeq`
/// ordering is preserved).
pub enum Effect {
    DamageBox(DamageBoxEffect),
    Summon(SummonSpec),
    Projectiles { shots: Vec<EnemyProjectileSpawn> },
}

/// "This `owner` emitted this `effect`." Written by a technique, drained by
/// [`apply_effects`] (and the lib-side Summon/Projectiles executors).
#[derive(Message)]
pub struct EffectRequest {
    pub owner: Entity,
    pub effect: Effect,
}

/// Generic effect executor: drains [`EffectRequest`]s and spawns each
/// `DamageBox`. Pure executor — every effect carries its own geometry, so this
/// needs no actor queries. Reads in message order (unsorted) to match the
/// per-consumer behavior it replaces.
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
                        shape: None,
                        damage: d.damage,
                        knockback: d.knockback,
                        lifetime_s: d.lifetime_s,
                        name: d.name,
                    },
                );
            }
            // Materialized by lib-side executors next to their substrate
            // (`apply_summon_effects` / `apply_projectile_effects`).
            Effect::Summon(_) | Effect::Projectiles { .. } => {}
        }
    }
}

#[cfg(test)]
mod hitbox_shape_tests {
    use super::*;

    #[test]
    fn shaped_hitbox_resolves_its_shape_at_the_anchor() {
        // A disc FollowOwner hitbox resolves to a Circle centered at owner+offset,
        // not the half_extent box.
        let hb = Hitbox {
            owner: Entity::PLACEHOLDER,
            source: ActorFaction::Player,
            anchor: HitboxAnchor::FollowOwner {
                local_offset: ae::Vec2::new(20.0, 0.0),
            },
            half_extent: ae::Vec2::splat(10.0),
            shape: Some(ae::VolumeShape::circle(15.0)),
            facing: 1.0,
            damage: 1,
            knockback_strength: 0.0,
            knock_x: 0.0,
        };
        match hb.world_volume(ae::Vec2::new(100.0, 50.0)) {
            ae::CombatVolume::Circle { center, radius } => {
                assert_eq!(center, ae::Vec2::new(120.0, 50.0));
                assert_eq!(radius, 15.0);
            }
            other => panic!("expected Circle, got {other:?}"),
        }
    }

    #[test]
    fn unshaped_hitbox_falls_back_to_the_half_extent_box() {
        let hb = Hitbox {
            owner: Entity::PLACEHOLDER,
            source: ActorFaction::Enemy,
            anchor: HitboxAnchor::World {
                center: ae::Vec2::new(5.0, 5.0),
            },
            half_extent: ae::Vec2::new(8.0, 12.0),
            shape: None,
            facing: 1.0,
            damage: 1,
            knockback_strength: 0.0,
            knock_x: 0.0,
        };
        assert!(matches!(
            hb.world_volume(ae::Vec2::ZERO),
            ae::CombatVolume::Aabb(_)
        ));
    }
}
