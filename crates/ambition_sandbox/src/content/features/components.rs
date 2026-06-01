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

/// Gameplay-space pose for an actor-like feature.
///
/// `FeatureAabb` remains the authoritative collision body; `ActorPose` is the
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
    pub fn from_aabb(aabb: FeatureAabb, facing: f32) -> Self {
        Self {
            center: aabb.center,
            feet: ae::Vec2::new(aabb.center.x, aabb.center.y + aabb.half_size.y),
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
    pub pickup: crate::interaction::Pickup,
}

impl PickupFeature {
    pub fn new(pickup: crate::interaction::Pickup) -> Self {
        Self { pickup }
    }

    pub fn kind(&self) -> &crate::interaction::PickupKind {
        &self.pickup.kind
    }
}

/// Marker inserted when a pickup has been collected in the current room/world.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Collected;

/// ECS-native chest payload.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ChestFeature {
    pub chest: crate::interaction::Chest,
}

impl ChestFeature {
    pub fn new(chest: crate::interaction::Chest) -> Self {
        Self { chest }
    }

    pub fn reward(&self) -> Option<&crate::interaction::PickupKind> {
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
    pub breakable: crate::interaction::Breakable,
}

impl BreakableFeature {
    pub fn new(breakable: crate::interaction::Breakable) -> Self {
        Self { breakable }
    }

    pub fn broken(&self) -> bool {
        self.breakable.state == crate::interaction::BreakableState::Broken
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
    /// (`crate::projectile::ProjectileFaction`) and actor faction agree on this:
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

    /// True iff `self` needs an `ActorTarget` (hostile-side combat
    /// actors and NPCs that face the player while idle).
    pub fn needs_target(self) -> bool {
        matches!(self, Self::Enemy | Self::Boss | Self::Npc)
    }
}

/// Per-actor "who am I looking at this frame" pointer. Populated by
/// [`select_actor_targets`](crate::features::ecs::select_actor_targets)
/// at the top of the simulation chain to the nearest alive
/// `ActorFaction::Player` entity.
///
/// Today's targeting policy is "single nearest player" (and there's
/// exactly one player in production, so the choice is trivial); the
/// component exists so the policy is a per-actor read, not a global
/// `player_query.single()` hard-coded into every actor update.
/// Co-op / split-screen builds can later swap in a per-actor policy
/// (sticky-target, role-based, distance-weighted) without touching
/// `enemy.update` / `npc.update` / `boss.update` signatures.
///
/// `entity` is `None` when no player-faction entities exist (pre-spawn,
/// post-death-of-all-players, headless probe). `pos` defaults to the
/// actor's own position in that case so a "no target" frame produces
/// a self-looking no-op rather than NaN-on-zero-direction crashes
/// in choreography or AI math.
#[derive(Component, Clone, Copy, Debug)]
pub struct ActorTarget {
    pub entity: Option<Entity>,
    pub pos: ae::Vec2,
}

impl Default for ActorTarget {
    fn default() -> Self {
        Self {
            entity: None,
            pos: ae::Vec2::ZERO,
        }
    }
}

/// ECS-visible actor health. The behavior runtime is still the temporary home
/// for AI details, but shared systems should read/write this component for HP.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorHealth {
    pub health: crate::actor::Health,
}

impl ActorHealth {
    pub fn new(health: crate::actor::Health) -> Self {
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

/// ECS-visible actor AI intent. Mirrors `crate::character_ai::CharacterAiMode` so rendering and
/// HUD systems can branch on actor state without pattern-matching `ActorRuntime`.
/// Synced from the runtime each frame by `update_ecs_actors`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActorIntent(pub crate::character_ai::CharacterAiMode);

impl ActorIntent {
    pub fn new(mode: crate::character_ai::CharacterAiMode) -> Self {
        Self(mode)
    }
    pub fn mode(self) -> crate::character_ai::CharacterAiMode {
        self.0
    }
    pub fn is_dangerous(self) -> bool {
        self.0.is_dangerous()
    }
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

/// Presentation lifetime for a defeated boss. `BossRuntime::alive` must flip
/// to false immediately so combat, rewards, and progression see the kill, but
/// the visual entity should remain visible long enough for the non-looping
/// death row to play.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct BossDeathAnimation {
    pub remaining_s: f32,
}

impl BossDeathAnimation {
    pub const DEFAULT_DURATION_S: f32 = 1.10;

    pub fn start(&mut self) {
        self.remaining_s = Self::DEFAULT_DURATION_S;
    }

    pub fn clear(&mut self) {
        self.remaining_s = 0.0;
    }

    pub fn tick(&mut self, dt: f32) {
        self.remaining_s = (self.remaining_s - dt.max(0.0)).max(0.0);
    }

    pub fn visible(self, alive: bool) -> bool {
        alive || self.remaining_s > 0.0
    }
}

impl Default for BossDeathAnimation {
    fn default() -> Self {
        Self { remaining_s: 0.0 }
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

/// Simulation-only base for a feature entity: the marker that identifies it
/// to feature-system queries, its room-scoped lifecycle, and its authored
/// identity/shape. Does NOT include any rendering components, so it is the
/// right base for headless features, AI scratch entities, and future
/// presentation-lazy spawns.
#[derive(Bundle)]
pub struct FeatureLifecycleBundle {
    pub sim_entity: FeatureSimEntity,
    pub room_scoped: crate::presentation::rendering::RoomScopedEntity,
    pub id: FeatureId,
    pub name: FeatureName,
    pub aabb: FeatureAabb,
}

impl FeatureLifecycleBundle {
    pub fn new(id: impl Into<String>, name: impl Into<String>, aabb: FeatureAabb) -> Self {
        Self {
            sim_entity: FeatureSimEntity,
            room_scoped: crate::presentation::rendering::RoomScopedEntity,
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
    pub room_visual: crate::presentation::rendering::RoomVisual,
}

impl FeatureRenderedBundle {
    pub fn new(id: impl Into<String>, name: impl Into<String>, aabb: FeatureAabb) -> Self {
        Self {
            lifecycle: FeatureLifecycleBundle::new(id, name, aabb),
            room_visual: crate::presentation::rendering::RoomVisual,
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
        aabb: FeatureAabb,
        pickup: crate::interaction::Pickup,
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
        aabb: FeatureAabb,
        chest: crate::interaction::Chest,
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
/// path. Behavior runtime (`ActorRuntime`) is still added separately.
#[derive(Bundle)]
pub struct EnemyActorBundle {
    pub base: FeatureBaseBundle,
    pub identity: ActorIdentity,
    pub disposition: ActorDisposition,
    /// Combat-side faction tag (`ActorFaction::Enemy` for encounter
    /// mobs, `ActorFaction::Npc` for peaceful actors). Future
    /// projectile-faction merge / multiplayer targeting will
    /// dispatch on this rather than pattern-matching on
    /// `ActorRuntime`. See OVERNIGHT-TODO #17.2 / #17.3.
    pub faction: ActorFaction,
    /// Per-frame "who is this actor looking at" pointer. Populated
    /// by `select_actor_targets` to the nearest alive player-faction
    /// entity (OVERNIGHT-TODO #17.8). Defaults to "no target",
    /// updated each tick.
    pub target: ActorTarget,
    pub pose: ActorPose,
    pub health: ActorHealth,
    pub combat: ActorCombatState,
    pub intent: ActorIntent,
    pub cooldowns: ActorCooldowns,
    pub damageable_volumes: DamageableVolumes,
    pub pogo_policy: PogoPolicy,
    pub pogo_target_volumes: PogoTargetVolumes,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_aabb_round_trips_center_and_size() {
        let feature =
            FeatureAabb::from_center_size(ae::Vec2::new(10.0, 20.0), ae::Vec2::new(8.0, 6.0));

        assert_eq!(feature.center, ae::Vec2::new(10.0, 20.0));
        assert_eq!(feature.half_size, ae::Vec2::new(4.0, 3.0));
        assert_eq!(feature.size(), ae::Vec2::new(8.0, 6.0));
        assert_eq!(
            feature.aabb(),
            ae::Aabb::new(ae::Vec2::new(10.0, 20.0), ae::Vec2::new(4.0, 3.0))
        );
    }

    #[test]
    fn actor_faction_player_is_player_side_others_are_not() {
        assert!(ActorFaction::Player.is_player_side());
        assert!(!ActorFaction::Enemy.is_player_side());
        assert!(!ActorFaction::Npc.is_player_side());
        assert!(!ActorFaction::Boss.is_player_side());
        assert!(!ActorFaction::Neutral.is_player_side());
    }

    #[test]
    fn actor_faction_enemy_and_boss_are_hostile_side() {
        assert!(ActorFaction::Enemy.is_hostile_side());
        assert!(ActorFaction::Boss.is_hostile_side());
        assert!(!ActorFaction::Player.is_hostile_side());
        assert!(!ActorFaction::Npc.is_hostile_side());
        assert!(!ActorFaction::Neutral.is_hostile_side());
    }

    #[test]
    fn actor_faction_default_is_player() {
        assert_eq!(ActorFaction::default(), ActorFaction::Player);
    }

    #[test]
    fn pogo_policy_defaults_to_damageable() {
        assert_eq!(PogoPolicy::default(), PogoPolicy::FromDamageable);
    }
}
