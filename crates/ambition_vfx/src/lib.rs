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

// The kernel, not the platformer. This crate takes `Vec2`, `Aabb`,
// `CombatVolume` and `VolumeShape` — shapes and boxes, nothing with a
// genre in it — and used to reach them through
// `ambition_platformer2d_core`, which made a presentation-neutral VFX
// vocabulary declare a platformer dependency it did not have.
use ambition_geometry as ae;
use ambition_projectile_spec::ProjectileSpawn;

pub mod fx;
pub mod vfx;
pub use fx::FxId;
pub use vfx::{
    FireworksRequest, FxPose, FxRequest, HitBurst, HurtFeedback, ImpactMaterial, ParticleKind, VfxMessage,
};

// ===================================================================
// The side an effect is emitted BY.
// ===================================================================
//
// ⚠ **this is the one piece of combat vocabulary that stayed**, and the reason
// is the orphan rule rather than taste. `Effect::DamageBox` and
// `Effect::Summon` both carry a side, and `ambition_projectiles` — which sits
// BELOW `ambition_combat` — names `Effect`. Moving the tag up would drag the
// whole effect-request vocabulary with it and hand a projectile crate a
// dependency on the combat crate. The authoritative components it used to sit
// beside are gone; what is left is a small enum on a message.
// ===================================================================
// Hitbox — the world-anchored damage volume an effect spawns.
// ===================================================================

/// Presentation-neutral side tag carried by effect messages and hitboxes.
///
/// This intentionally mirrors the combat-facing actor faction vocabulary
/// without depending on the character crate: effect producers map their richer
/// game-side faction into this small fact at the emit site, and combat
/// resolvers map it back when they need faction relations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HitSide {
    /// Local or remote player-controlled side.
    #[default]
    Player,
    /// Encounter-spawned hostile side.
    Enemy,
    /// Authored NPC side. Peaceful NPCs do not emit combat effects, but
    /// provoked NPCs keep this side so personal-grudge combat stays expressible.
    Npc,
    /// Boss-tier hostile side.
    Boss,
    /// Inert/non-combatant side.
    Neutral,
}

impl HitSide {
    pub fn is_player_side(self) -> bool {
        matches!(self, Self::Player)
    }

    pub fn is_hostile_side(self) -> bool {
        matches!(self, Self::Enemy | Self::Boss)
    }
}

// ===================================================================
// Effect vocabulary + the message seam + the executor.
// ===================================================================

/// The payload of an [`Effect::DamageBox`]: a world-anchored damage volume.
/// `center` + `faction` are explicit — the emitter resolves its own position and
/// knows its faction, so the executor needs no actor queries.
pub struct DamageBoxEffect {
    pub center: ae::Vec2,
    pub faction: HitSide,
    pub half_extent: ae::Vec2,
    pub damage: i32,
    /// Dimensionless multiplier over the victim's standard feel-tuned launch.
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
    pub character_id: String,
    pub encounter_id: String,
    pub faction: HitSide,
}

/// A composable effect an actor *technique* emits. [`apply_effects`] executes
/// `DamageBox`; `Summon` and the enemy-pool `Projectiles` are materialized by
/// lib-side executors next to their substrate (so the shared `ProjectileSeq`
/// ordering is preserved).
pub enum Effect {
    DamageBox(DamageBoxEffect),
    Summon(SummonSpec),
    Projectiles { shots: Vec<ProjectileSpawn> },
}

/// "This `owner` emitted this `effect`." Written by a technique, drained by
/// [`apply_effects`] (and the lib-side Summon/Projectiles executors).
#[derive(Message)]
pub struct EffectRequest {
    pub owner: Entity,
    pub effect: Effect,
}
