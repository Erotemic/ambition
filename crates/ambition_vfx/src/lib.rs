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
    /// The summoner BOARDS what it just made, if the pair is a legal one.
    ///
    /// ⭐ ON THE SUMMON RATHER THAN A FOLLOW-UP, because the only moment the
    /// spawned entity and its summoner are both in hand is inside the executor's
    /// own exclusive command. A caller that wanted to board afterwards would
    /// have to be told which entity was made — a channel this needed no other
    /// consumer for — and would then be boarding a mount that had already been
    /// simulated for a tick without a rider.
    ///
    /// ⛔ IT IS A REQUEST, NOT A GUARANTEE. The class check is `mount::board`'s
    /// and it refuses an illegal pair; a summon that names no mountable thing
    /// simply spawns it and nobody gets on. `None` for every summon that existed
    /// before this field, which is every minion a boss drops.
    ///
    /// ⭐ IT CARRIES THE RIDE'S LENGTH, so the whole ride is ONE transaction.
    /// The first version installed the lease from the technique and left the
    /// board to the executor, so a REFUSED board left an orphan lease on a body
    /// that was not riding anything — and the dismount consumer skips a rider
    /// with no link, so the orphan never went away.
    pub ridden_by_summoner: Option<SummonedRide>,
    /// Health for THIS occurrence, overriding the character's authored vitals.
    ///
    /// ⛔⛔ THE SAME CREATURE IS NOT THE SAME THING IN TWO GAMES. The burning
    /// flying shark authors 6 HP, which is a fair pool for the game it was
    /// written for; the pirate's up-B drops it into a platform fighter whose own
    /// move table runs 2–17, so most single connections delete it — and it is
    /// summoned exactly where its rider is, which in a fight is exactly where
    /// the hits are. A recovery that dies to one stray hit is not a recovery.
    ///
    /// ⭐ ON THE SUMMON, NOT ON THE CHARACTER. Raising the authored number would
    /// re-tune a creature that appears in another game entirely; the summoner is
    /// the one that knows what THIS occurrence is for. `None` leaves the
    /// character's own vitals alone, which is every summon that existed before.
    pub health: Option<u32>,
}

/// The summoner rides what it makes, for this long.
///
/// Seconds rather than ticks, because the lease it becomes counts on the sim
/// clock like every other gameplay countdown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SummonedRide {
    /// How long the ride lasts once the summoner is aboard.
    pub seconds: f32,
    /// How close the mount must be to its summoner before they get on.
    ///
    /// ⭐ A DISTANCE, NOT A FLAG, so "it appears underneath me" and "it flies in
    /// from off-screen" are the same request with different numbers. A mount
    /// summoned at the rider's own position satisfies this on the tick it
    /// exists. See `ambition_mount::MountReservedFor`.
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
