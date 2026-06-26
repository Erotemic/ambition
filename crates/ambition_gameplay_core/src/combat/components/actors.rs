//! Actor combat components: identity/disposition/target, combat kit +
//! aggression, health, attack/combat state, cooldowns, and boss phase state.

use super::super::*;
use ambition_characters::actor::control::{BlockReason, IntentOutcome};

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
/// by inspecting an actor-type tag.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorDisposition {
    Peaceful,
    Hostile,
}

impl ActorDisposition {
    pub fn is_hostile(self) -> bool {
        matches!(self, Self::Hostile)
    }

    pub fn is_peaceful(self) -> bool {
        matches!(self, Self::Peaceful)
    }
}

/// Explicit sprite render-quad size for an actor whose collision box was derived
/// from published sprite `body_metrics` (so `kin.size` is the visible-body
/// hitbox, not a scaled placeholder). The renderer draws the sprite at THIS
/// size instead of re-deriving `collision * collision_scale`, which would
/// double-scale once the collision already equals the body.
///
/// A SHARED actor component (not on `NpcConfig`/`ActorConfig`) precisely so it
/// survives a peaceful→hostile flip: when an NPC turns hostile the NPC-only
/// cluster is swapped for the enemy cluster, but this component stays attached,
/// so the actor keeps rendering at its authored size instead of ballooning.
/// Absent ⇒ the actor uses the legacy `collision_scale` render path.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ActorRenderSize(pub ae::Vec2);

/// Optional dialogue/interaction payload for a *talkable* actor.
///
/// Lifted off `NpcConfig` so "can be talked to" is a SHARED actor capability,
/// not an NPC-type trait: a peaceful NPC today, a parley-able enemy or ally
/// tomorrow, all carry the same component. Presence = "this actor can be talked
/// to"; the interact / proximity-highlight / dialogue-bubble systems key off the
/// component instead of an actor-type tag.
///
/// `talk_radius` is the world-pixel range at which a patrolling actor stops to
/// face the player so the interact is reachable.
#[derive(Component, Clone, Debug)]
pub struct ActorInteraction {
    pub interactable: ambition_interaction::Interactable,
    pub talk_radius: f32,
}
// `ActorFaction` moved to `ambition_characters::actor::pose` with `ActorPose`.
pub use ambition_characters::actor::pose::ActorFaction;

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
    pub innate_melee: Option<ambition_characters::brain::MeleeActionSpec>,
    pub innate_ranged: Option<ambition_characters::brain::RangedActionSpec>,
    pub move_style: ambition_characters::brain::MoveStyleSpec,
}

impl CombatKit {
    pub fn from_action_set(actions: &ambition_characters::brain::ActionSet) -> Self {
        Self {
            innate_melee: actions.melee,
            innate_ranged: actions.ranged,
            move_style: actions.move_style,
        }
    }

    pub fn to_action_set(
        &self,
        held_item: Option<&ambition_characters::brain::HeldItemSpec>,
    ) -> ambition_characters::brain::ActionSet {
        let mut actions = ambition_characters::brain::ActionSet {
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

    pub fn can_melee(&self, held_item: Option<&ambition_characters::brain::HeldItemSpec>) -> bool {
        self.to_action_set(held_item).melee.is_some()
    }

    pub fn can_ranged(&self, held_item: Option<&ambition_characters::brain::HeldItemSpec>) -> bool {
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
    /// Accumulated provocation count. Lives here (next to the
    /// `RetaliatesWhenHit { strike_threshold }` mode it feeds) rather than on a
    /// per-family status component, so the provoke accumulator survives the
    /// NPC→one-actor cluster merge and the in-place hostile flip.
    pub strikes: i32,
}

impl ActorAggression {
    pub fn passive() -> Self {
        Self {
            mode: AggressionMode::Passive,
            target: None,
            strikes: 0,
        }
    }

    pub fn retaliates_when_hit(strike_threshold: u8) -> Self {
        Self {
            mode: AggressionMode::RetaliatesWhenHit { strike_threshold },
            target: None,
            strikes: 0,
        }
    }

    pub fn hostile_to_player() -> Self {
        Self {
            mode: AggressionMode::HostileToPlayer,
            target: None,
            strikes: 0,
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
    pub health: ambition_characters::actor::Health,
}

impl ActorHealth {
    pub fn new(health: ambition_characters::actor::Health) -> Self {
        Self { health }
    }

    pub fn alive(self) -> bool {
        self.health.alive()
    }
}

/// Melee attack timing + aim. The four interdependent values move as one
/// coherent unit so combat systems can read strike progress from a single
/// place.
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
    /// Body-side ranged refire cooldown remaining (s). The ranged analogue of
    /// `cooldown`: it is the *body's* fire-rate floor (invariant I3), not the
    /// brain's cadence. A controller may attempt `fire` every tick; the body
    /// accepts a shot only when this is `<= 0` and re-arms it on each accepted
    /// shot, so a spam controller and a human produce the same weapon rate.
    /// Ranged has no windup/active timeline (it spawns instantly), so it needs
    /// only this one timer rather than the melee strike's three.
    pub ranged_cooldown: f32,
    /// Body-side blink refire cooldown remaining (s). The physical floor on the
    /// `blink` intent (invariant I3): a controller may *attempt* a blink every
    /// tick, but the body teleports at most once per refire and re-arms this on
    /// each accepted blink — so an AI brain and a possessing human blink at the
    /// same rate. Gated together with `CombatCapabilities::can_blink`.
    pub blink_cooldown: f32,
}

impl Default for ActorAttackState {
    fn default() -> Self {
        Self {
            windup_timer: 0.0,
            active_timer: 0.0,
            cooldown: 0.2,
            pending_axis: ae::Vec2::new(-1.0, 0.0),
            ranged_cooldown: 0.0,
            blink_cooldown: 0.0,
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
        self.ranged_cooldown = (self.ranged_cooldown - dt).max(0.0);
        self.blink_cooldown = (self.blink_cooldown - dt).max(0.0);
        if was_winding_up && self.windup_timer <= 0.0 {
            self.active_timer = active_seconds.max(0.01);
        }
    }

    /// Body-side ranged fire-rate enforcement (invariant I3).
    ///
    /// A controller attempts a shot; the body accepts it only when the ranged
    /// weapon is off cooldown, re-arming the cooldown to `refire_seconds` on an
    /// accepted shot. The controller is free to attempt every tick — this is the
    /// floor that turns attempts into the body's weapon rate, identical for an AI
    /// spam controller, a tactical brain, and a human. Returns the per-intent
    /// outcome so the seam can route `Blocked`/`Accepted` feedback back to the
    /// controller.
    pub fn try_fire_ranged(&mut self, refire_seconds: f32) -> IntentOutcome {
        if self.ranged_cooldown > 0.0 {
            return IntentOutcome::Blocked(BlockReason::Cooldown);
        }
        self.ranged_cooldown = refire_seconds.max(0.0);
        IntentOutcome::Accepted
    }

    /// Body-side blink enforcement (invariant I3), the movement analogue of
    /// [`Self::try_fire_ranged`]. Accepts a blink only when off cooldown, arming
    /// the refire on acceptance. The caller still gates on the body's
    /// `CombatCapabilities::can_blink` (does this body have blink at all) and
    /// performs the actual collision-clamped teleport on `Accepted`.
    pub fn try_blink(&mut self, refire_seconds: f32) -> IntentOutcome {
        if self.blink_cooldown > 0.0 {
            return IntentOutcome::Blocked(BlockReason::Cooldown);
        }
        self.blink_cooldown = refire_seconds.max(0.0);
        IntentOutcome::Accepted
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

/// ECS-visible actor AI intent. Mirrors `ambition_characters::actor::ai::CharacterAiMode` so rendering and
/// HUD systems can branch on actor state without a per-family runtime.
/// Synced from the runtime each frame by `update_ecs_actors`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActorIntent(pub ambition_characters::actor::ai::CharacterAiMode);

impl ActorIntent {
    pub fn new(mode: ambition_characters::actor::ai::CharacterAiMode) -> Self {
        Self(mode)
    }
    pub fn mode(self) -> ambition_characters::actor::ai::CharacterAiMode {
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
