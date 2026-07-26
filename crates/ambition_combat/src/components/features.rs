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

// `ActorPose` moved to `ambition_characters::actor::pose` (actor-system vocabulary;
// Stage 22 unified-actor work). Re-exported below so kit paths keep working.
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
/// Since damage resolution reads this component, it must distinguish **three**
/// states, not two:
///
/// * *never published* (`published == false`) — nobody has spoken for this body
///   yet, so a consumer falls back to the body's coarse box. A body spawned this
///   tick, or a bare test fixture that does not run the publisher, lives here.
/// * *published, non-empty* — these volumes ARE the body's silhouette.
/// * *published, empty* — the body is deliberately **intangible**: an authored
///   invulnerable window, or a corpse the publisher cleared.
///
/// Collapsing the first and third is how an authored invulnerability silently
/// becomes a hittable rectangle, and collapsing them the other way makes every
/// freshly spawned body a ghost. Both were live possibilities the first time
/// damage started reading this component, and one of them broke a projectile test
/// immediately.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct DamageableVolumes {
    pub volumes: Vec<ae::Aabb>,
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

    /// Publish "intangible": this body can be hit nowhere.
    pub fn clear(&mut self) {
        self.volumes.clear();
        self.published = true;
    }

    pub fn set_single(&mut self, aabb: ae::Aabb) {
        self.volumes.clear();
        self.volumes.push(aabb);
        self.published = true;
    }

    /// Publish an explicit list — an authored silhouette, or a boss's active parts.
    pub fn publish(&mut self, volumes: Vec<ae::Aabb>) {
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

// `SwitchFeature`/`SwitchOn` moved to `crate::encounter::switches` (E2):
// they carry encounter vocabulary (`SwitchActivation`).
