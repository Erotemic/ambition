//! Reusable effect vocabulary + executor.
//!
//! A *technique* (a boss special, a wielded ability, an authored hazard) decides
//! *what and when*; an effect is the consequence it spawns. Effects are
//! faction-tagged and carry their own geometry, so the player, an enemy, and a
//! boss all drive the same execution path. The message seam ([`EffectRequest`])
//! is what makes the effect system removable: drop the consumer and techniques
//! emit into the void — nothing spawns, the rest of the game still runs.
//!
//! This crate owns the world-anchored [`Hitbox`] damage-box component and the
//! `DamageBox` executor; damage *resolution* (`apply_hitbox_damage`) and the
//! `Summon` executor live beside their substrate. Projectile spawning is a
//! separate authoritative domain and uses its own `ProjectileSpawnRequest`
//! directly rather than passing simulation work through a VFX enum.

use bevy::prelude::*;

// The kernel, not the platformer. Gameplay geometry stays on `ambition_geometry`; the only direct
// core edge now is the backend-neutral rollback declaration vocabulary in `rollback_registration`.
use ambition_geometry as ae;

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
// this is the one piece of combat vocabulary that stayed, and the reason is the orphan rule
// rather than taste. `Effect::DamageBox` and `Effect::Summon` both carry a side. Projectile
// spawning no longer names this enum at all; its request vocabulary is owned by the projectile
// domain.

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
    /// A hazard: it belongs to no side and hurts every fighter, including the
    /// one who placed it.
    ///
    /// This is the presentation-side name for `DamageTeam::Environment`, not a
    /// new idea. `placements.rs` asserts `Environment.can_damage(Player)` and
    /// `Environment.can_damage(Enemy)`. It is a separate variant because the
    /// `Hitbox` road carries `source: HitSide` and no `DamageTeam`.
    ///
    /// It is not `Neutral` with damage: `Neutral` takes no part, and the resolver
    /// never spawns a damaging hitbox for it.
    Environment,
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
    /// The summoner boards what it just made, if the pair is legal.
    ///
    /// This is on the summon, not a follow-up, because only the executor's
    /// exclusive command holds both the new entity and its summoner. A later
    /// board would need a channel back and would find a mount that already
    /// simulated a tick without a rider.
    ///
    /// It is a request: `mount::board` refuses an illegal pair, and then the
    /// entity spawns with no rider. `None` means no ride (every boss minion).
    ///
    /// It carries the ride length so the lease and the board are one transaction.
    /// A refused board must not leave a lease on a body that rides nothing.
    pub ridden_by_summoner: Option<SummonedRide>,
    /// Health for this occurrence, overriding the character's authored vitals.
    ///
    /// The same creature can need different health in different games: a summon
    /// used as a recovery must survive a stray hit in a fighter whose moves deal
    /// 2 to 17. This is on the summon, not the character, so other games keep
    /// the authored value. `None` keeps the character's own vitals.
    pub health: Option<u32>,
    /// Whether this occurrence keeps the character's authored contact hazard.
    ///
    /// "Neutral" is not "harmless": `damage_lands` is true for `Foe | Neutral`,
    /// so an opponent can gimp a recovery. Without this flag, a neutral summon
    /// avoids contact damage only because it acquires no target. `true` keeps the
    /// character's own answer.
    pub keeps_contact_damage: bool,
}

/// The summoner rides what it makes, for this long.
///
/// Seconds rather than ticks, because the lease it becomes counts on the sim
/// clock like every other gameplay countdown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SummonedRide {
    /// How long the ride lasts once the summoner is aboard.
    pub seconds: f32,
    /// A distance, not a flag: "it appears underneath me" and "it flies in from
    /// off-screen" are the same request with different numbers. A mount summoned
    /// at the rider's position satisfies this on its first tick. See
    /// `ambition_mount::MountReservedFor`.
    pub board_within: f32,
    /// How long the mount waits to be reached before giving up and telling the
    /// ruleset. See `ambition_mount::MountReservedFor::expires_in`.
    pub board_deadline_s: f32,
}

/// A composable non-projectile effect an actor *technique* emits.
/// [`apply_effects`] executes `DamageBox`; `Summon` is materialized beside the
/// actor-construction substrate. Projectile requests deliberately use their own
/// domain-owned message instead of entering this enum.
pub enum Effect {
    DamageBox(DamageBoxEffect),
    Summon(SummonSpec),
}

/// "This `owner` emitted this `effect`." Written by a technique, drained by
/// [`apply_effects`] (and the lib-side Summon executor).
#[derive(Message)]
pub struct EffectRequest {
    pub owner: Entity,
    pub effect: Effect,
}

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
