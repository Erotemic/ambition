//! Spawn bundles + enemy spawn-time data (moved out of the combat kit at
//! E2: spawn machinery is features-side; the bundles compose combat
//! components through the legal features → combat arrow).

use bevy::prelude::*;

use crate::combat::components::*;
use crate::combat::*;
use crate::features::ecs::actor_tuning::{ActorTuning, CharacterBrainSpec};
use ambition_characters::actor::BodyCombat;
use ambition_engine_core::CenteredAabb;
use ambition_platformer_primitives::lifecycle::FeatureSimEntity;

/// Simulation-only base for a feature entity: the marker that identifies it
/// to feature-system queries, its room-scoped lifecycle, and its authored
/// identity/shape. Does NOT include any rendering components, so it is the
/// right base for headless features, AI scratch entities, and future
/// presentation-lazy spawns.
#[derive(Bundle)]
pub struct FeatureLifecycleBundle {
    pub sim_entity: FeatureSimEntity,
    pub room_scoped: ambition_platformer_primitives::lifecycle::RoomScopedEntity,
    pub id: FeatureId,
    pub name: FeatureName,
    pub aabb: CenteredAabb,
}

impl FeatureLifecycleBundle {
    pub fn new(id: impl Into<String>, name: impl Into<String>, aabb: CenteredAabb) -> Self {
        Self {
            sim_entity: FeatureSimEntity,
            room_scoped: ambition_platformer_primitives::lifecycle::RoomScopedEntity,
            id: FeatureId(id.into()),
            name: FeatureName(name.into()),
            aabb,
        }
    }
}

/// Rendered feature base: lifecycle bundle plus `RoomVisual` (a
/// presentation-side component private to `crate::presentation::rendering`).
/// Use this for features that should be drawn by the presentation systems
/// (the default for every authored feature today). Headless/sim-only spawns
/// should reach for [`FeatureLifecycleBundle`] instead.
#[derive(Bundle)]
pub struct FeatureRenderedBundle {
    pub lifecycle: FeatureLifecycleBundle,
    pub room_visual: ambition_platformer_primitives::lifecycle::RoomVisual,
}

impl FeatureRenderedBundle {
    pub fn new(id: impl Into<String>, name: impl Into<String>, aabb: CenteredAabb) -> Self {
        Self {
            lifecycle: FeatureLifecycleBundle::new(id, name, aabb),
            room_visual: ambition_platformer_primitives::lifecycle::RoomVisual,
        }
    }
}

/// Backwards-compatible alias for [`FeatureRenderedBundle`]. New code should
/// pick the explicit `Lifecycle` or `Rendered` bundle.
pub type FeatureBaseBundle = FeatureRenderedBundle;

/// Bundle for pickup feature entities.
#[derive(Bundle)]
pub struct PickupBundle {
    pub base: FeatureBaseBundle,
    pub pickup: PickupFeature,
}

impl PickupBundle {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: CenteredAabb,
        pickup: ambition_interaction::Pickup,
    ) -> Self {
        Self {
            base: FeatureBaseBundle::new(id, name, aabb),
            pickup: PickupFeature::new(pickup),
        }
    }
}

/// Bundle for chest feature entities.
#[derive(Bundle)]
pub struct ChestBundle {
    pub base: FeatureBaseBundle,
    pub chest: ChestFeature,
}

impl ChestBundle {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: CenteredAabb,
        chest: ambition_interaction::Chest,
    ) -> Self {
        Self {
            base: FeatureBaseBundle::new(id, name, aabb),
            chest: ChestFeature::new(chest),
        }
    }
}

/// Bundle for enemy actor entities. `base` is the rendered feature bundle
/// today; once headless feature spawning lands, swap it for
/// [`FeatureLifecycleBundle`] and add `RoomVisual` only on the rendered
/// path. The behavior brain + cluster are still added separately.
#[derive(Bundle)]
pub struct EnemyActorBundle {
    pub base: FeatureBaseBundle,
    pub identity: ActorIdentity,
    pub disposition: ActorDisposition,
    /// Combat-side faction tag (`ActorFaction::Enemy` for encounter
    /// mobs, `ActorFaction::Npc` for peaceful actors). Future
    /// projectile-faction merge / multiplayer targeting will
    /// dispatch on this rather than pattern-matching on
    /// the actor cluster. See OVERNIGHT-TODO #17.2 / #17.3.
    pub faction: ActorFaction,
    /// Per-frame "who is this actor looking at" pointer. Populated
    /// by `select_actor_targets` to the nearest alive player-faction
    /// entity (OVERNIGHT-TODO #17.8). Defaults to "no target",
    /// updated each tick.
    pub target: ActorTarget,
    pub pose: ActorPose,
    pub combat_kit: CombatKit,
    pub aggression: ActorAggression,
    // Health (`BodyHealth`) spawns with the actor CLUSTER (`into_components`), the
    // one health authority — not on this combat bundle.
    pub combat: BodyCombat,
    pub intent: ActorIntent,
    pub cooldowns: ActorCooldowns,
    pub damageable_volumes: DamageableVolumes,
    pub pogo_policy: PogoPolicy,
    pub pogo_target_volumes: PogoTargetVolumes,
    /// Movement-driven presentation overlay timers, shared with the player. Armed
    /// each frame by `advance_actor_anim_overlays` (landing / dash-startup) and
    /// read by `pick_actor_anim`, so an AI fighter shows those poses too (fable
    /// review §A9). Defaults inert.
    pub anim: crate::player::BodyAnimFacts,
}

impl EnemyActorBundle {
    /// Construct a spawn bundle, filling the four fields that are identical at
    /// every spawn site — `target` (no target until `select_actor_targets`
    /// runs), `damageable_volumes` (derived from the sheet), `pogo_policy =
    /// FromDamageable`, and `pogo_target_volumes`. Each `spawn_*` site supplies
    /// only the fields that actually vary, so adding a new defaulted bundle field
    /// is a one-line change here instead of an edit at all six call sites
    /// (`spawn_actors.rs` ×4, `spawn_mounts.rs` ×2). Every parameter is a
    /// distinct type, so a mis-ordered argument is a compile error, not a silent
    /// spawn bug.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        base: FeatureBaseBundle,
        identity: ActorIdentity,
        disposition: ActorDisposition,
        faction: ActorFaction,
        pose: ActorPose,
        combat_kit: CombatKit,
        aggression: ActorAggression,
        combat: BodyCombat,
        intent: ActorIntent,
        cooldowns: ActorCooldowns,
    ) -> Self {
        Self {
            base,
            identity,
            disposition,
            faction,
            target: ActorTarget::default(),
            pose,
            combat_kit,
            aggression,
            combat,
            intent,
            cooldowns,
            damageable_volumes: DamageableVolumes::default(),
            pogo_policy: PogoPolicy::FromDamageable,
            pogo_target_volumes: PogoTargetVolumes::default(),
            anim: crate::player::BodyAnimFacts::default(),
        }
    }
}
