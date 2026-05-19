//! ECS-native feature components.
//!
//! Gameplay feature families are represented as normal Bevy entities/components,
//! paired with typed messages for cross-system effects.

use super::*;

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
/// The legacy runtimes store `pos` + full `size`. ECS systems should prefer
/// this single component so collection, interaction, damage, and overlay systems
/// can query one canonical shape.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct FeatureAabb {
    pub center: ae::Vec2,
    pub half_size: ae::Vec2,
}

impl FeatureAabb {
    pub fn new(center: ae::Vec2, half_size: ae::Vec2) -> Self {
        Self { center, half_size }
    }

    pub fn from_center_size(center: ae::Vec2, size: ae::Vec2) -> Self {
        Self {
            center,
            half_size: size * 0.5,
        }
    }

    pub fn from_aabb(aabb: ae::Aabb) -> Self {
        Self {
            center: aabb.center(),
            half_size: aabb.half_size(),
        }
    }

    pub fn size(self) -> ae::Vec2 {
        self.half_size * 2.0
    }

    pub fn aabb(self) -> ae::Aabb {
        ae::Aabb::new(self.center, self.half_size)
    }
}

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
    pub pickup: ae::Pickup,
}

impl PickupFeature {
    pub fn new(pickup: ae::Pickup) -> Self {
        Self { pickup }
    }

    pub fn kind(&self) -> &ae::PickupKind {
        &self.pickup.kind
    }
}

/// Marker inserted when a pickup has been collected in the current room/world.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Collected;

/// ECS-native chest payload.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ChestFeature {
    pub chest: ae::Chest,
}

impl ChestFeature {
    pub fn new(chest: ae::Chest) -> Self {
        Self { chest }
    }

    pub fn reward(&self) -> Option<&ae::PickupKind> {
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
    pub breakable: ae::Breakable,
}

impl BreakableFeature {
    pub fn new(breakable: ae::Breakable) -> Self {
        Self { breakable }
    }

    pub fn broken(&self) -> bool {
        self.breakable.state == ae::BreakableState::Broken
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

/// Marker for ECS features that can refresh pogo when struck/bounced.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PogoTargetContributor;

/// ECS-native switch.
///
/// Carries a typed [`crate::encounter::SwitchActivation`] instead of the
/// raw `"switch:<id>:<action>:<target>"` wire string. The parse happens
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


/// Actor-specific authored/runtime identity.
///
/// `FeatureId` remains the canonical entity lookup key. This component exposes
/// actor-facing identity directly so rendering, save sync, and debug systems do
/// not have to pattern-match through the behavior runtime to ask who the actor
/// is or which authored NPC sheet a hostile actor should keep using.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct ActorIdentity {
    pub id: String,
    pub name: String,
    pub sprite_override_npc_name: Option<String>,
}

impl ActorIdentity {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            sprite_override_npc_name: None,
        }
    }

    pub fn with_sprite_override(mut self, name: Option<String>) -> Self {
        self.sprite_override_npc_name = name;
        self
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// High-level actor disposition. Peaceful actors talk/patrol; hostile actors
/// chase/attack. Hostility is data now, not an enum arm callers must discover
/// by inspecting `ActorRuntime`.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorDisposition {
    Peaceful,
    Hostile,
}

impl ActorDisposition {
    pub fn is_hostile(self) -> bool {
        matches!(self, Self::Hostile)
    }
}

/// ECS-visible actor health. The behavior runtime is still the temporary home
/// for AI details, but shared systems should read/write this component for HP.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorHealth {
    pub health: ae::Health,
}

impl ActorHealth {
    pub fn new(health: ae::Health) -> Self {
        Self { health }
    }

    pub fn alive(self) -> bool {
        self.health.alive()
    }
}

/// ECS-visible combat/presentation state shared by NPCs and enemies.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ActorCombatState {
    pub alive: bool,
    pub hit_flash: f32,
    pub strike_count: i32,
    pub attack_windup_timer: f32,
    pub attack_timer: f32,
    pub training_dummy: bool,
}

impl ActorCombatState {
    pub fn peaceful(strike_count: i32, hit_flash: f32) -> Self {
        Self {
            alive: true,
            hit_flash,
            strike_count,
            attack_windup_timer: 0.0,
            attack_timer: 0.0,
            training_dummy: false,
        }
    }

    pub fn hostile(
        alive: bool,
        hit_flash: f32,
        attack_windup_timer: f32,
        attack_timer: f32,
        training_dummy: bool,
    ) -> Self {
        Self {
            alive,
            hit_flash,
            strike_count: 0,
            attack_windup_timer,
            attack_timer,
            training_dummy,
        }
    }
}

/// ECS-visible actor AI intent. Mirrors `ae::CharacterAiMode` so rendering and
/// HUD systems can branch on actor state without pattern-matching `ActorRuntime`.
/// Synced from the runtime each frame by `update_ecs_actors`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActorIntent(pub ae::CharacterAiMode);

impl ActorIntent {
    pub fn new(mode: ae::CharacterAiMode) -> Self { Self(mode) }
    pub fn mode(self) -> ae::CharacterAiMode { self.0 }
    pub fn is_dangerous(self) -> bool { self.0.is_dangerous() }
}

/// ECS-visible actor cooldown timers. Exposes timing state that rendering and
/// encounter systems need without reaching into `ActorRuntime`.
/// Synced from the runtime each frame by `update_ecs_actors`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct ActorCooldowns {
    pub attack_cooldown: f32,
    pub respawn_timer: f32,
}

/// ECS-visible boss pattern timer. Mirrors `BossRuntime::pattern_timer`
/// so sprite animation systems can read it without accessing `BossFeature`.
/// Synced from the runtime each frame by `update_ecs_bosses`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BossPatternTimer(pub f32);

/// ECS-visible boss combat phase.
///
/// Synced from `BossFeature::boss.alive` each frame by `update_ecs_bosses`:
/// - `Active`   — the boss entity exists and is still alive.
/// - `Defeated` — the boss entity exists but health reached zero.
///
/// A boss entity is only ever spawned when an authored `BossSpawn` exists
/// in the active room, so there is no separate "dormant" reading: the
/// absence of a `BossPhase` component is itself the dormant signal.
/// (Engine-side cinematic phasing — Intro / Phase 2 etc. — lives in the
/// seldom_state `ae::state_machines::BossPhase` machine on the boss
/// runtime; this read-model intentionally does not duplicate it.)
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossPhase {
    Active,
    Defeated,
}

impl BossPhase {
    pub fn from_alive(alive: bool) -> Self {
        if alive {
            Self::Active
        } else {
            Self::Defeated
        }
    }

    pub fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    pub fn is_defeated(self) -> bool {
        matches!(self, Self::Defeated)
    }
}

/// Marker for hostile actors spawned dynamically by an encounter wave.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct EncounterMob {
    pub encounter_id: String,
}

impl EncounterMob {
    pub fn new(encounter_id: impl Into<String>) -> Self {
        Self {
            encounter_id: encounter_id.into(),
        }
    }
}

/// Marker for encounter reward chests spawned after a mob encounter clears.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct EncounterRewardChest {
    pub encounter_id: String,
}

impl EncounterRewardChest {
    pub fn new(encounter_id: impl Into<String>) -> Self {
        Self {
            encounter_id: encounter_id.into(),
        }
    }
}

/// Marker for boss reward chests spawned after a boss encounter clears.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct BossRewardChest {
    pub encounter_id: String,
}

impl BossRewardChest {
    pub fn new(encounter_id: impl Into<String>) -> Self {
        Self {
            encounter_id: encounter_id.into(),
        }
    }
}

// ── Bundles ───────────────────────────────────────────────────────────────
//
// Each bundle groups the components that always appear together when a feature
// entity is spawned. Spawn calls in features/ecs.rs use these bundles so the
// required components are expressed in one place and tests/editors can match
// the exact shape without rediscovering the tuple.

/// Shared base for every feature simulation entity (pickup, chest, hazard,
/// actor, breakable …). Combines the marker, visual tag, and authored identity
/// components that appear on every feature spawn.
///
/// Includes [`crate::presentation::rendering::RoomVisual`] because every authored feature
/// today both *is* a simulation entity and *renders*; the rendering systems
/// query `With<RoomVisual>` and the room-load / reset path despawns the same
/// marker to wipe the previous room. See the doc comment on `RoomVisual` for
/// the dual-role rationale and the planned split into a separate
/// `RoomScopedEntity` lifecycle marker once a presentation observer makes the
/// visual lazy.
#[derive(Bundle)]
pub struct FeatureBaseBundle {
    pub sim_entity: FeatureSimEntity,
    pub room_visual: crate::presentation::rendering::RoomVisual,
    pub id: FeatureId,
    pub name: FeatureName,
    pub aabb: FeatureAabb,
}

impl FeatureBaseBundle {
    pub fn new(id: impl Into<String>, name: impl Into<String>, aabb: FeatureAabb) -> Self {
        Self {
            sim_entity: FeatureSimEntity,
            room_visual: crate::presentation::rendering::RoomVisual,
            id: FeatureId(id.into()),
            name: FeatureName(name.into()),
            aabb,
        }
    }
}

/// Bundle for pickup feature entities.
#[derive(Bundle)]
pub struct PickupBundle {
    pub base: FeatureBaseBundle,
    pub pickup: PickupFeature,
}

impl PickupBundle {
    pub fn new(id: impl Into<String>, name: impl Into<String>, aabb: FeatureAabb, pickup: ae::Pickup) -> Self {
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
    pub fn new(id: impl Into<String>, name: impl Into<String>, aabb: FeatureAabb, chest: ae::Chest) -> Self {
        Self {
            base: FeatureBaseBundle::new(id, name, aabb),
            chest: ChestFeature::new(chest),
        }
    }
}

/// Bundle for enemy actor entities. Presentation rendering and behavior
/// runtime (ActorRuntime) are added separately so headless builds can omit
/// the visual components.
#[derive(Bundle)]
pub struct EnemyActorBundle {
    pub base: FeatureBaseBundle,
    pub identity: ActorIdentity,
    pub disposition: ActorDisposition,
    pub health: ActorHealth,
    pub combat: ActorCombatState,
    pub intent: ActorIntent,
    pub cooldowns: ActorCooldowns,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_aabb_round_trips_center_and_size() {
        let feature = FeatureAabb::from_center_size(
            ae::Vec2::new(10.0, 20.0),
            ae::Vec2::new(8.0, 6.0),
        );

        assert_eq!(feature.center, ae::Vec2::new(10.0, 20.0));
        assert_eq!(feature.half_size, ae::Vec2::new(4.0, 3.0));
        assert_eq!(feature.size(), ae::Vec2::new(8.0, 6.0));
        assert_eq!(
            feature.aabb(),
            ae::Aabb::new(ae::Vec2::new(10.0, 20.0), ae::Vec2::new(4.0, 3.0))
        );
    }
}
