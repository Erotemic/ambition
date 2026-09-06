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

// Re-exported below so kit paths keep working.
pub use ambition_characters::actor::pose::ActorPose;

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

/// Per-frame volumes that can currently receive attack damage.
///
/// Consumers must preserve three states: unpublished falls back to the body's
/// coarse box; published non-empty uses the authored silhouette; published
/// empty is intentionally intangible. The list may contain multiple shaped
/// [`ae::CombatVolume`] parts because one hull cannot represent disjoint body
/// regions.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct DamageableVolumes {
    pub volumes: Vec<ae::CombatVolume>,
    published: bool,
}

impl DamageableVolumes {
    /// True once a publisher has spoken for this body this session.
    ///
    /// A consumer that needs geometry must fall back to the coarse box while this
    /// is false, and must respect an empty list once it is true.
    pub fn published(&self) -> bool {
        self.published
    }

    /// True when a publisher has explicitly made this body unhittable.
    ///
    /// Consumers must check this before any coarse-box fallback.
    pub fn intangible(&self) -> bool {
        self.published && self.volumes.is_empty()
    }

    /// Publish "intangible": this body can be hit nowhere.
    pub fn clear(&mut self) {
        self.volumes.clear();
        self.published = true;
    }

    pub fn set_single(&mut self, aabb: ae::Aabb) {
        self.volumes.clear();
        self.volumes.push(ae::CombatVolume::aabb(aabb));
        self.published = true;
    }

    /// Publish an explicit list — an authored silhouette, or a boss's active parts.
    pub fn publish(&mut self, volumes: Vec<ae::CombatVolume>) {
        self.volumes = volumes;
        self.published = true;
    }

    /// An already-published single volume, for fixtures that need a body which is
    /// hittable without running a publisher.
    pub fn single(aabb: ae::Aabb) -> Self {
        let mut out = Self::default();
        out.set_single(aabb);
        out
    }

    /// The published silhouette as coarse boxes.
    ///
    /// Pogo affordance readers and explicit world-surface contributors are box
    /// consumers. Damage itself must keep reading the original combat volumes;
    /// this named coarsening prevents a convenience projection from becoming the
    /// authoritative hurtbox by accident.
    pub fn bounds(&self) -> Vec<ae::Aabb> {
        self.volumes.iter().map(ae::CombatVolume::bounds).collect()
    }
}

/// Per-feature pogo derivation policy.
///
/// The default game rule is that things a body can damage are also valid
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

/// Entity-side geometry that may grant a pogo rebound.
///
/// Bodies publish this alongside their damageable silhouette so proximity/UI and
/// `PogoPolicy::Custom` can describe the pogoable part without losing body
/// identity. These volumes are not collision-world blocks by default. An ECS
/// feature becomes world rebound geometry only when it also carries
/// [`PogoTargetContributor`].
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct PogoTargetVolumes {
    pub volumes: Vec<ae::Aabb>,
}

/// Explicit marker for an ECS feature that contributes WORLD pogo geometry.
///
/// This is deliberately absent from combat bodies. A body is pogoed through its
/// resolved entity hit and [`PogoPolicy`]; a contributor is lowered into a
/// collision-world `PogoOrb` because it represents a surface with no body-victim
/// semantics (for example a stand-to-crumble or pogo-refresh platform).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PogoTargetContributor;
