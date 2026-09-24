//! `ActorFaction`, the shared allegiance handle every actor family carries.

use bevy::prelude::Component;

/// Combat-side faction tag (OVERNIGHT-TODO #17.2/17.3 — shared actor
/// facets). Distinct from `ActorDisposition` (`ambition_combat`, which this
/// crate sits BELOW and cannot link to): disposition is the
/// per-tick hostility flag NPCs can toggle into (a guide can become
/// `Hostile` when struck); faction is the structural "which side
/// owns this actor" tag that damage routing, projectile hit policy,
/// and enemy AI targeting all dispatch on.
///
/// Initially attached as a read-model / identity tag only — none of
/// today's combat / projectile code consults it. The point is to
/// give per-family components (`PlayerEntity`, `BossFeature`,
/// the actor cluster, etc.) a single shared "faction" handle so
/// multiplayer-aware targeting (#17.8) and the unified projectile
/// faction merge (#17.7) can move off type-pattern-matching onto a
/// uniform query filter.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ActorFaction {
    /// Local or remote player-controlled actor.
    #[default]
    Player,
    /// Encounter-spawned hostile actor (enemy, miniboss).
    Enemy,
    /// Authored story-content NPC (peaceful by default; can flip to
    /// hostile via `ActorDisposition` without changing faction).
    Npc,
    /// Boss-tier hostile actor. Distinct from `Enemy` because boss
    /// encounters carry phase / cutscene / save state that regular
    /// enemies don't.
    Boss,
    /// Neutral non-combatant (currently unused; reserved for future
    /// breakables that act like actors for hit detection without
    /// participating in the player-vs-enemy combat loop).
    Neutral,
}

impl ActorFaction {
    /// True iff `self` is on the player's side. Projectile damage routes
    /// off the firer's `ActorFaction` (looked up from the projectile's
    /// owner entity): player-fired projectiles damage non-player factions,
    /// enemy-fired projectiles damage player factions only.
    pub fn is_player_side(self) -> bool {
        matches!(self, Self::Player)
    }

    /// True iff `self` participates in the active combat loop
    /// (`Enemy` / `Boss`). Useful for nearest-target queries that
    /// ignore peaceful NPCs and neutrals.
    pub fn is_hostile_side(self) -> bool {
        matches!(self, Self::Enemy | Self::Boss)
    }
}
