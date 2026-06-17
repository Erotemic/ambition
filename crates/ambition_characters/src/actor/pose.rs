//! Actor-system pose + faction vocabulary.
//!
//! `ActorPose` is the lightweight gameplay-space read model brains and
//! action systems use for attack origins and facing (the collision body
//! stays authoritative in the combat kit). `ActorFaction` is the shared
//! allegiance handle every actor family carries. Both moved here from
//! the combat kit because they are ACTOR vocabulary (Stage 22 unified
//! actor system); the kit re-exports them at the old paths.

use ambition_engine_core as ae;
use bevy::prelude::Component;

/// Gameplay-space pose for an actor-like feature.
///
/// `CenteredAabb` remains the authoritative collision body; `ActorPose` is the
/// lightweight read model that brain/action systems use for attack origins and
/// facing. This keeps gameplay action emission off Bevy `Transform`, which is a
/// rendering/spatial-hierarchy concern in this codebase.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ActorPose {
    pub center: ae::Vec2,
    pub feet: ae::Vec2,
    pub facing: f32,
}

impl ActorPose {
    /// Build a pose from a collision body's parts. (Was `from_aabb`,
    /// taking the kit's `CenteredAabb`; parts-based so this crate-level
    /// vocabulary doesn't depend on the combat kit's body type.)
    pub fn from_parts(center: ae::Vec2, half_size: ae::Vec2, facing: f32) -> Self {
        Self {
            center,
            feet: ae::Vec2::new(center.x, center.y + half_size.y),
            facing: normalized_facing(facing),
        }
    }

    pub fn origin(self) -> ae::Vec2 {
        self.center
    }
}

impl Default for ActorPose {
    fn default() -> Self {
        Self {
            center: ae::Vec2::ZERO,
            feet: ae::Vec2::ZERO,
            facing: 1.0,
        }
    }
}

fn normalized_facing(facing: f32) -> f32 {
    if facing < 0.0 {
        -1.0
    } else {
        1.0
    }
}

/// Combat-side faction tag (OVERNIGHT-TODO #17.2/17.3 — shared actor
/// facets). Distinct from [`ActorDisposition`]: disposition is the
/// per-tick hostility flag NPCs can toggle into (a guide can become
/// `Hostile` when struck); faction is the structural "which side
/// owns this actor" tag that damage routing, projectile hit policy,
/// and enemy AI targeting all dispatch on.
///
/// Initially attached as a read-model / identity tag only — none of
/// today's combat / projectile code consults it. The point is to
/// give per-family components (`PlayerEntity`, `BossFeature`,
/// `ActorRuntime`, etc.) a single shared "faction" handle so
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
    /// True iff `self` is on the player's side. Projectile faction
    /// (`ProjectileFaction` (ambition_gameplay_core)) and actor faction agree on this:
    /// player projectiles damage non-player factions, enemy
    /// projectiles damage player factions only.
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
