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

/// Data/authored combat capabilities for an actor.
///
/// `ActionSet` remains the hot per-frame resolver consumed by the brain/action
/// pipeline. `CombatKit` is the durable ECS/gameplay source of capability: what
/// the actor can do innately, before current held-item overlays are applied.
/// That distinction lets a peaceful NPC carry a sword/bow/bomb without being
/// aggressive yet, and lets aggression changes re-enable attacks without
/// swapping the actor's identity or archetype.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct CombatKit {
    pub innate_melee: Option<crate::brain::MeleeActionSpec>,
    pub innate_ranged: Option<crate::brain::RangedActionSpec>,
    pub move_style: crate::brain::MoveStyleSpec,
}

impl CombatKit {
    pub fn from_action_set(actions: &crate::brain::ActionSet) -> Self {
        Self {
            innate_melee: actions.melee,
            innate_ranged: actions.ranged,
            move_style: actions.move_style,
        }
    }

    pub fn to_action_set(
        &self,
        held_item: Option<&crate::brain::HeldItemSpec>,
    ) -> crate::brain::ActionSet {
        let mut actions = crate::brain::ActionSet {
            melee: self.innate_melee,
            ranged: self.innate_ranged,
            move_style: self.move_style,
            ..Default::default()
        };
        if let Some(item) = held_item {
            item.apply_to_action_set(&mut actions);
        }
        actions
    }

    pub fn can_melee(&self, held_item: Option<&crate::brain::HeldItemSpec>) -> bool {
        self.to_action_set(held_item).melee.is_some()
    }

    pub fn can_ranged(&self, held_item: Option<&crate::brain::HeldItemSpec>) -> bool {
        self.to_action_set(held_item).ranged.is_some()
    }
}

/// Relationship/hostility state for actor-like entities.
///
/// This is deliberately separate from `ActorFaction`: faction says what the
/// actor is authored as, while aggression says who the actor is currently
/// willing to fight. The first slice supports the current player-retaliation
/// game; future faction/allied-NPC behavior can add more targets without
/// rewriting the brain/action pipeline.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorAggression {
    pub mode: AggressionMode,
    pub target: Option<Entity>,
}

impl ActorAggression {
    pub fn passive() -> Self {
        Self {
            mode: AggressionMode::Passive,
            target: None,
        }
    }

    pub fn retaliates_when_hit(strike_threshold: u8) -> Self {
        Self {
            mode: AggressionMode::RetaliatesWhenHit { strike_threshold },
            target: None,
        }
    }

    pub fn hostile_to_player() -> Self {
        Self {
            mode: AggressionMode::HostileToPlayer,
            target: None,
        }
    }

    pub fn is_aggressive(self) -> bool {
        matches!(self.mode, AggressionMode::HostileToPlayer)
    }

    /// Who this actor wants to look at / chase this frame, derived from
    /// its aggression mode rather than its [`ActorFaction`]. This is the
    /// seam [`select_actor_targets`](crate::features::ecs::select_actor_targets)
    /// reads: faction no longer decides targeting.
    ///
    /// Intentionally minimal today — every non-passive actor tracks the
    /// nearest player, which reproduces the previous
    /// `faction.needs_target()` behavior for all hostile / retaliating
    /// actors. The richer relationship policies sketched in
    /// `dev/reviews/ecs-cleanup-plan.md` #3 (HostileToFaction, ally-of-
    /// player, lock onto the specific `target` entity) slot in here as
    /// new [`AggressionTarget`] variants without touching the brains or
    /// combat systems.
    pub fn target_policy(self) -> AggressionTarget {
        match self.mode {
            AggressionMode::Passive => AggressionTarget::None,
            AggressionMode::RetaliatesWhenHit { .. } | AggressionMode::HostileToPlayer => {
                AggressionTarget::NearestPlayer
            }
        }
    }
}

impl Default for ActorAggression {
    fn default() -> Self {
        Self::passive()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AggressionMode {
    Passive,
    RetaliatesWhenHit { strike_threshold: u8 },
    HostileToPlayer,
}

/// Resolved targeting policy for one frame, produced by
/// [`ActorAggression::target_policy`] and consumed by
/// [`select_actor_targets`](crate::features::ecs::select_actor_targets).
/// Keeps target selection aggression-driven instead of branching on
/// [`ActorFaction`]. New relationship policies (target a specific
/// entity, nearest hostile faction member, ...) extend this enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AggressionTarget {
    /// No combat target this frame — passive actor. The selector points
    /// the actor at itself so downstream facing math reads a zero
    /// direction (keep current facing) instead of snapping toward the
    /// world origin.
    None,
    /// Track the nearest alive player-faction entity.
    NearestPlayer,
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

/// Melee attack timing + aim, grouped out of the flat `EnemyRuntime`
/// field list so the four interdependent values move as one coherent
/// unit and combat systems can read strike progress from a single
/// place. Today it lives as the `EnemyRuntime::attack` field; it is the
/// destined home for the ECS-component promotion tracked in
/// `dev/reviews/ecs-cleanup-plan.md` (#1, "Extract ActorAttackState").
///
/// Timeline of a strike: `begin_attack` arms `windup_timer` + `cooldown`
/// and commits `pending_axis`; `tick` counts windup down and, on the
/// windup→active edge, arms `active_timer` (the hitbox window); the
/// active window then counts down while `cooldown` keeps ticking so a
/// fresh attack can't start until recovery passes.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ActorAttackState {
    /// Telegraph/windup remaining before the strike goes active.
    pub windup_timer: f32,
    /// Active hitbox window remaining once windup completes.
    pub active_timer: f32,
    /// Recovery remaining before another attack can begin.
    pub cooldown: f32,
    /// Direction of the in-flight melee attack — committed on
    /// `begin_attack`, read on the windup→active edge to place the
    /// hitbox. `(facing, 0)` forward, `(0, -1)` up, `(0, +1)` down-air,
    /// `(-facing, 0)` back-air. Persists across the whole strike so the
    /// swing doesn't re-aim mid-windup.
    pub pending_axis: ae::Vec2,
}

impl Default for ActorAttackState {
    fn default() -> Self {
        Self {
            windup_timer: 0.0,
            active_timer: 0.0,
            cooldown: 0.2,
            pending_axis: ae::Vec2::new(-1.0, 0.0),
        }
    }
}

impl ActorAttackState {
    pub fn is_winding_up(self) -> bool {
        self.windup_timer > 0.0
    }

    pub fn is_active(self) -> bool {
        self.active_timer > 0.0
    }

    pub fn on_cooldown(self) -> bool {
        self.cooldown > 0.0
    }

    /// Advance all timers by `dt`. On the windup→active edge, arm the
    /// active window to `active_seconds` (the hitbox lifetime).
    pub fn tick(&mut self, dt: f32, active_seconds: f32) {
        let was_winding_up = self.windup_timer > 0.0;
        self.windup_timer = (self.windup_timer - dt).max(0.0);
        self.active_timer = (self.active_timer - dt).max(0.0);
        self.cooldown = (self.cooldown - dt).max(0.0);
        if was_winding_up && self.windup_timer <= 0.0 {
            self.active_timer = active_seconds.max(0.01);
        }
    }
}

/// ECS-visible combat/presentation state shared by NPCs, enemies, and bosses.
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

/// ECS-visible actor AI intent. Mirrors `crate::actor::ai::CharacterAiMode` so rendering and
/// HUD systems can branch on actor state without pattern-matching `ActorRuntime`.
/// Synced from the runtime each frame by `update_ecs_actors`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActorIntent(pub crate::actor::ai::CharacterAiMode);

impl ActorIntent {
    pub fn new(mode: crate::actor::ai::CharacterAiMode) -> Self {
        Self(mode)
    }
    pub fn mode(self) -> crate::actor::ai::CharacterAiMode {
        self.0
    }
    pub fn is_dangerous(self) -> bool {
        self.0.is_dangerous()
    }
}

/// ECS-visible actor cooldown timers. Exposes timing state that rendering and
/// encounter systems need without reaching into family-specific runtimes.
/// Synced from actor/boss runtime state each frame by feature systems.
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

/// Neutral marker for a runtime-spawned post-boss NPC.
///
/// Core room-reset cleanup and the presentation render-fallback both need to
/// treat these runtime NPCs generically (despawn them on a same-room reset,
/// give them an NPC sprite-fallback) without naming any specific boss. The
/// bespoke per-boss content (e.g. the Smirking Behemoth victory NPC in
/// `crate::ambition_content::bosses::cut_rope`) tags the entity with this
/// marker so the dependency points content -> core, never the reverse.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PostBossNpc;

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
    pub room_scoped: crate::platformer_runtime::lifecycle::RoomScopedEntity,
    pub id: FeatureId,
    pub name: FeatureName,
    pub aabb: FeatureAabb,
}

impl FeatureLifecycleBundle {
    pub fn new(id: impl Into<String>, name: impl Into<String>, aabb: FeatureAabb) -> Self {
        Self {
            sim_entity: FeatureSimEntity,
            room_scoped: crate::platformer_runtime::lifecycle::RoomScopedEntity,
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
    pub combat_kit: CombatKit,
    pub aggression: ActorAggression,
    pub health: ActorHealth,
    pub combat: ActorCombatState,
    pub intent: ActorIntent,
    pub cooldowns: ActorCooldowns,
    pub damageable_volumes: DamageableVolumes,
    pub pogo_policy: PogoPolicy,
    pub pogo_target_volumes: PogoTargetVolumes,
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
        health: ActorHealth,
        combat: ActorCombatState,
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
            health,
            combat,
            intent,
            cooldowns,
            damageable_volumes: DamageableVolumes::default(),
            pogo_policy: PogoPolicy::FromDamageable,
            pogo_target_volumes: PogoTargetVolumes::default(),
        }
    }
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

/// Per-actor combat capabilities, derived from the actor's authored
/// archetype DATA at spawn (`enemy_archetypes.ron`) and attached as a
/// component so generic combat systems can branch on capabilities
/// instead of matching named archetype enums. The content layer
/// derives it; the kit only defines the vocabulary.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct CombatCapabilities {
    /// Detonates at the corpse on death (Enemy-faction blast), so a
    /// point-blank kill is punished.
    pub explodes_on_death: bool,
    /// Splits into offspring on death.
    pub divides_on_death: bool,
    /// A fast charge stopped dead by a wall destroys this actor.
    pub charge_crash_explodes: bool,
    /// Damage never kills (training dummy with an effectively
    /// infinite pool).
    pub never_dies: bool,
    /// On death, respawns in place after this many seconds instead of
    /// counting as defeated.
    pub respawn_in_place_seconds: Option<f32>,
}
