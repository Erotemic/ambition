//! Feature-entity components: identity, geometry, and the pickup/chest/
//! breakable/switch/pogo feature families.

use super::super::*;

/// Stable authored/runtime identity for a feature entity.
///
/// Use this for save keys, traces, and entity lookup. It intentionally mirrors
/// the IDs currently embedded in `PickupRuntime`, `ChestRuntime`,
/// `BreakableRuntime`, and `SwitchRuntime` so migration patches can move one
/// family without changing persistence vocabulary.
#[derive(Component, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FeatureId(pub String);

impl FeatureId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Human-facing authored name for debug overlays / inspectors.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct FeatureName(pub String);

impl FeatureName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// World-space collision / interaction shape for a feature entity.
///
/// Re-export of the canonical machinery-layer center+half box
/// [`ae::CenteredAabb`]. ECS systems prefer this single component so collection,
/// interaction, damage, and overlay systems query one canonical shape — and it
/// is the same type the engine uses everywhere, so there is no per-layer box
/// conversion.
pub use ae::CenteredAabb;

// `ActorPose` moved to `crate::actor::pose` (actor-system vocabulary;
// Stage 22 unified-actor work). Re-exported below so kit paths keep working.
pub use crate::actor::pose::ActorPose;

/// Explicit persistence key. Kept separate from `FeatureId` so migrated features
/// can choose when authored identity and save identity differ.
#[derive(Component, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PersistKey(pub String);

impl PersistKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// ECS-native pickup payload.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct PickupFeature {
    pub pickup: ambition_interaction::Pickup,
}

impl PickupFeature {
    pub fn new(pickup: ambition_interaction::Pickup) -> Self {
        Self { pickup }
    }

    pub fn kind(&self) -> &ambition_interaction::PickupKind {
        &self.pickup.kind
    }
}

/// Marker inserted when a pickup has been collected in the current room/world.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Collected;

/// ECS-native chest payload.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ChestFeature {
    pub chest: ambition_interaction::Chest,
}

impl ChestFeature {
    pub fn new(chest: ambition_interaction::Chest) -> Self {
        Self { chest }
    }

    pub fn reward(&self) -> Option<&ambition_interaction::PickupKind> {
        self.chest.reward.as_ref()
    }
}

/// Marker inserted once a chest is opened.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Opened;

/// Marker/state component for chests that are falling toward the room floor.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct FallingChest {
    pub vel_y: f32,
}

impl FallingChest {
    pub fn new(vel_y: f32) -> Self {
        Self { vel_y }
    }
}

/// ECS-native breakable payload.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct BreakableFeature {
    pub breakable: ambition_interaction::Breakable,
}

impl BreakableFeature {
    pub fn new(breakable: ambition_interaction::Breakable) -> Self {
        Self { breakable }
    }

    pub fn broken(&self) -> bool {
        self.breakable.state == ambition_interaction::BreakableState::Broken
    }
}

/// Respawn timer for breakables that come back after being destroyed.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct RespawnTimer(pub f32);

/// Stand-to-crumble timer for breakables with an `OnStand` trigger.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct StandTimer(pub f32);

/// Marker for ECS features that should contribute collision to the sandbox
/// world overlay while active.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SandboxSolidContributor;

/// Volumes that can currently receive player-side attack damage.
///
/// This is intentionally a per-frame ECS read model rather than a type-specific
/// helper call: actors can publish their current body AABB, bosses can publish
/// sprite-authored hurtboxes, and breakables can publish authored trigger
/// volumes. Systems that care about "what can the player hit?" should consume
/// this component instead of rediscovering family-specific geometry.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct DamageableVolumes {
    pub volumes: Vec<ae::Aabb>,
}

impl DamageableVolumes {
    pub fn clear(&mut self) {
        self.volumes.clear();
    }

    pub fn set_single(&mut self, aabb: ae::Aabb) {
        self.volumes.clear();
        self.volumes.push(aabb);
    }
}

/// Per-feature pogo derivation policy.
///
/// The default game rule is that things the player can damage are also valid
/// downslash/pogo refresh targets. `Disabled` is the escape hatch for puzzle
/// targets or hazardous objects that should take damage without granting a
/// bounce, while `Custom` leaves `PogoTargetVolumes` to a domain-specific
/// system.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PogoPolicy {
    #[default]
    FromDamageable,
    Custom,
    Disabled,
}

/// Volumes that should be bridged into the engine collision world as
/// non-solid `PogoOrb` blocks.
///
/// `rebuild_feature_ecs_world_overlay` consumes this generic component instead
/// of hard-coding "enemy body" or "boss body" branches. That keeps composite
/// bosses such as GNU-ton free to expose only their active hurtboxes as pogo
/// targets.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct PogoTargetVolumes {
    pub volumes: Vec<ae::Aabb>,
}

/// Legacy marker for ECS features that can refresh pogo when struck/bounced.
///
/// Prefer `DamageableVolumes` + `PogoPolicy` + `PogoTargetVolumes` for new
/// gameplay. This marker remains for authored stand-to-crumble surfaces whose
/// pogo affordance is not a player-damage target.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PogoTargetContributor;

/// ECS-native switch.
///
/// Carries a typed `SwitchActivation` (private to `crate::encounter`)
/// instead of the raw `"switch:<id>:<action>:<target>"` wire string. The parse happens
/// once at LDtk-to-ECS spawn time in
/// `crate::features::ecs::spawn_room_feature_entity`; the
/// activation-queue / switch-index / interact-emit paths all read
/// `feature.activation` directly.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct SwitchFeature {
    pub activation: crate::encounter::SwitchActivation,
}

impl SwitchFeature {
    pub fn new(activation: crate::encounter::SwitchActivation) -> Self {
        Self { activation }
    }
}

/// Live switch state used by rendering and encounter reset logic.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SwitchOn(pub bool);
