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

/// The body's GROSS coarse-hurtbox footprint — the full-size box the published
/// `CenteredAabb` covers, distinct from `kin.size` (the collision box the
/// movement seam sweeps against walls) and from [`ActorRenderSize`] (the sprite
/// draw quad). Present ⇒ the shared body integrator publishes `CenteredAabb`
/// from THIS size; absent ⇒ it publishes from `kin.size` (the ordinary actor,
/// whose collision box IS its footprint).
///
/// This is the envelope split (fable-review-2026-07-04 AJ5.1): a giant boss has
/// a composite render/whole-creature envelope much larger than its collision
/// box, and that envelope — not the collision box — is the coarse hurtbox a
/// duelist's swing must overlap. Making the divergence an explicit component
/// (instead of a bespoke boss integrate arm publishing a render-sized box) is
/// what lets the boss body flow through the SAME `integrate_actor_body` every
/// actor uses. Only bosses carry it today; the fine per-part hurtboxes still
/// come from `damageable_volumes`, not this coarse box.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct BodyEnvelope(pub ae::Vec2);

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
///
/// # Snapshot story
///
/// `docs/planning/engine/netcode.md` N3.1: *"sim components are plain data; anything
/// holding `Entity` references or interior mutability documents its snapshot story at
/// the definition site."* This is that document.
///
/// **`entity` is an `Entity`, which N3.1 decision (2) forbids in sim state.** An
/// entity index is a slot in an allocator, not an identity: it does not survive a
/// restore that respawns anything, and it means nothing in a desync report. It is
/// here because every consumer needs to *query* the target (its health, its body),
/// and replacing it with a `SimId` needs a `SimId -> Entity` index rebuilt each tick.
/// **That is the named migration slice**, listed in `tracks.md`.
///
/// Until then: `entity` is **derived**, rewritten every tick by
/// [`select_actor_targets`](crate::targeting::select_actor_targets) whenever any
/// candidate exists, and a rollback lets that system rebuild it. `pos` is **state** —
/// on the one frame where no candidates exist, the selector leaves the whole
/// component untouched on purpose, so `pos` carries the previous frame's aim into the
/// brain's math. `ambition_runtime::rollback` therefore registers `ActorTarget` as a
/// `SnapshotCursor` over `pos` alone.
///
/// A rollback that spans the frame a target *died* can leave `entity` pointing at a
/// despawned body for one tick. `Query::get` returns `Err` and every consumer already
/// treats that as "no target", which is the behaviour they take when the target dies
/// anyway. That is a survivable one-tick lie, not a correct design, and it is the
/// third reason the migration is worth doing.
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
    /// Per-actor grudge: a SPECIFIC entity this actor has decided to oppose
    /// beyond its `FactionRelations` baseline (set when provoked — the attacker
    /// that struck it past its threshold). Targeting treats a grudge entity as a
    /// foe just like a relational faction-foe, so a provoked NPC chases its
    /// attacker without mutating its `ActorFaction` identity. `None` = no grudge;
    /// the actor fights purely along faction lines. Grudges only form against
    /// real attackers — with friendly-fire off, `can_damage` blocks ally-on-ally
    /// hits, so no spurious `DamagedBy` stimulus (and no grudge) ever forms.
    pub grudge: Option<Entity>,
}

impl ActorAggression {
    pub fn passive() -> Self {
        Self {
            mode: AggressionMode::Passive,
            target: None,
            strikes: 0,
            grudge: None,
        }
    }

    pub fn retaliates_when_hit(strike_threshold: u8) -> Self {
        Self {
            mode: AggressionMode::RetaliatesWhenHit { strike_threshold },
            target: None,
            strikes: 0,
            grudge: None,
        }
    }

    /// Actively hostile: targets the nearest body of any faction it opposes
    /// (`FactionRelations`) PLUS any entity it holds a grudge against. Whom that
    /// turns out to be — the player (a born Enemy whose faction opposes Player),
    /// a faction-foe in a duel (Enemy vs Boss, the observing player spared because
    /// relations don't make it a foe), or a specific attacker (a provoked NPC's
    /// grudge) — is decided by relations + faction + grudge, never named here.
    pub fn hostile() -> Self {
        Self {
            mode: AggressionMode::Hostile,
            target: None,
            strikes: 0,
            grudge: None,
        }
    }

    pub fn is_aggressive(self) -> bool {
        matches!(self.mode, AggressionMode::Hostile)
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
            // One relational policy: track the nearest foe — any faction this actor
            // opposes (`FactionRelations`) OR its grudge entity. A peaceful
            // `RetaliatesWhenHit` actor (faction Npc, no grudge) has no foe, so this
            // is inert until it's provoked into `Hostile` with a grudge.
            AggressionMode::RetaliatesWhenHit { .. } | AggressionMode::Hostile => {
                AggressionTarget::Foe
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
    /// Never fights — no combat target, ignores being hit.
    Passive,
    /// Peaceful until struck past `strike_threshold`, then provoked into `Hostile`
    /// with a grudge against the attacker (see [`ActorAggression::grudge`]).
    RetaliatesWhenHit { strike_threshold: u8 },
    /// Actively hostile — one relational policy. Targets the nearest body of any
    /// faction it opposes (`FactionRelations`) plus its grudge entity. There is no
    /// player-named mode: a born Enemy hunts the player because its faction opposes
    /// Player; a duel fighter hunts its faction-foe; a provoked NPC hunts its
    /// grudge — all the same mode, the difference is in relations/faction/grudge.
    Hostile,
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
    /// Track the nearest alive FOE: any faction this actor opposes
    /// (`FactionRelations`) OR its grudge entity. The player is a relational
    /// candidate like any other faction — never an unconditional special case.
    Foe,
}

/// One in-flight melee swing, driven by the player's [`AttackSpec`] model.
///
/// This is THE swing state for EVERY body — the human player and every
/// brain-driven actor — so a swing's lifecycle (startup → active → recovery),
/// per-swing hit dedup, and pogo bookkeeping have a single definition. The spec
/// is stored already rotated into the body's world frame (the moveset resolves
/// the move's authored volumes into the gravity frame at spawn), so
/// `phase_at(elapsed)` and the hitbox geometry are read directly without
/// re-rotating.
#[derive(Clone, Debug, PartialEq)]
pub struct MeleeSwing {
    /// Resolved swing parameters in WORLD frame (timing, reach, knockback, art).
    pub spec: crate::AttackSpec,
    /// Seconds since the swing began.
    pub elapsed: f32,
    /// `prefix:id` keys of every target already struck this swing, so an
    /// every-active-frame hitbox only damages each target once. Used by the
    /// universal hit resolver (`apply_feature_hit_events`).
    pub hit_targets: Vec<String>,
    /// True once the active window has begun (first-active-frame edge latch).
    pub active_started: bool,
    /// True once a downward/pogo active-frame attack has produced its bounce, so
    /// one long active window can't bounce every frame.
    pub pogo_applied: bool,
}

impl MeleeSwing {
    pub fn new(spec: crate::AttackSpec) -> Self {
        Self {
            spec,
            elapsed: 0.0,
            hit_targets: Vec::new(),
            active_started: false,
            pogo_applied: false,
        }
    }

    pub fn phase(&self) -> Option<crate::AttackPhase> {
        self.spec.phase_at(self.elapsed)
    }

    pub fn done(&self) -> bool {
        self.phase().is_none()
    }

    pub fn progress(&self) -> f32 {
        (self.elapsed / self.spec.total_seconds().max(0.001)).clamp(0.0, 1.0)
    }
}

/// Unified body melee state — the ONE component every body (player + actors)
/// carries for melee. The in-flight [`MeleeSwing`] is the player's spec model;
/// `cooldown` is the AI/recovery pacing floor a brain reads to time its next
/// swing (independent of the swing so a body can be in recovery with no swing
/// armed); `ranged_cooldown` is the body-side ranged fire-rate floor (invariant
/// I3, orthogonal to melee); `pending_axis` is the last committed aim for anim
/// selection. (ONE BODY ONE PATH: this REPLACES the former parallel
/// `PlayerAttackState`/`ActivePlayerAttack` and the timer-based actor state.)
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct BodyMelee {
    pub swing: Option<MeleeSwing>,
    /// Recovery/AI pacing floor before another swing may begin (s).
    pub cooldown: f32,
    /// Body-side ranged refire cooldown remaining (s). The body's fire-rate
    /// floor (invariant I3), not the brain's cadence: a controller may attempt
    /// `fire` every tick; the body accepts a shot only when this is `<= 0` and
    /// re-arms it on each accepted shot, so a spam controller and a human
    /// produce the same weapon rate.
    pub ranged_cooldown: f32,
    /// Direction of the in-flight melee attack, committed when the swing begins
    /// (`(facing,0)` forward, `(0,-1)` up, `(0,+1)` down-air, `(-facing,0)`
    /// back-air). Persists across the swing so it doesn't re-aim mid-windup.
    pub pending_axis: ae::Vec2,
}

impl BodyMelee {
    /// Begin a swing: commit the world-frame `spec`, aim, and recovery floor.
    pub fn begin(&mut self, spec: crate::AttackSpec, pending_axis: ae::Vec2, cooldown: f32) {
        self.pending_axis = pending_axis;
        self.cooldown = cooldown.max(0.0);
        self.swing = Some(MeleeSwing::new(spec));
    }

    pub fn phase(&self) -> Option<crate::AttackPhase> {
        self.swing.as_ref().and_then(|s| s.phase())
    }

    pub fn is_winding_up(&self) -> bool {
        matches!(self.phase(), Some(crate::AttackPhase::Startup))
    }

    pub fn is_active(&self) -> bool {
        matches!(self.phase(), Some(crate::AttackPhase::Active))
    }

    /// True while ANY swing is in flight (startup, active, OR recovery) — the
    /// "is the body mid-swing" signal the player mirrors onto `BodyCombat`
    /// (distinct from `is_active`, which is only the hitbox window).
    pub fn is_swinging(&self) -> bool {
        self.swing.is_some()
    }

    /// Cancel any in-flight swing (room transition / reset).
    pub fn clear(&mut self) {
        self.swing = None;
    }

    pub fn on_cooldown(&self) -> bool {
        self.cooldown > 0.0
    }

    /// Seconds of windup (startup) remaining, for the AI telegraph snapshot.
    pub fn windup_remaining(&self) -> f32 {
        match &self.swing {
            Some(s) if self.is_winding_up() => (s.spec.startup_seconds - s.elapsed).max(0.0),
            _ => 0.0,
        }
    }

    /// Seconds of active (hitbox) window remaining, for the AI snapshot.
    pub fn active_remaining(&self) -> f32 {
        match &self.swing {
            Some(s) if self.is_active() => {
                (s.spec.startup_seconds + s.spec.active_seconds - s.elapsed).max(0.0)
            }
            _ => 0.0,
        }
    }

    /// Advance the swing + the cooldown floors by `dt`. Drops a spent swing once
    /// it passes recovery (the cooldown floor keeps ticking independently).
    pub fn tick(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        self.cooldown = (self.cooldown - dt).max(0.0);
        self.ranged_cooldown = (self.ranged_cooldown - dt).max(0.0);
        if let Some(swing) = &mut self.swing {
            swing.elapsed += dt;
            if swing.phase().is_none() {
                self.swing = None;
            }
        }
    }

    /// Body-side ranged fire-rate enforcement (invariant I3).
    ///
    /// A controller attempts a shot; the body accepts it only when the ranged
    /// weapon is off cooldown, re-arming the cooldown to `refire_seconds` on an
    /// accepted shot. Identical for an AI spam controller, a tactical brain, and
    /// a human. Returns the per-intent outcome for the seam to route back.
    pub fn try_fire_ranged(&mut self, refire_seconds: f32) -> IntentOutcome {
        if self.ranged_cooldown > 0.0 {
            return IntentOutcome::Blocked(BlockReason::Cooldown);
        }
        self.ranged_cooldown = refire_seconds.max(0.0);
        IntentOutcome::Accepted
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

/// Marker for hostile actors spawned imperatively at room load OUTSIDE the
/// authored `RoomSpec` lists (today: the spectator-duel fighters staged by
/// the content duel stager). The authored render pass only spawns visuals for
/// `spec.enemy_spawns`, and the dynamic pass only for [`EncounterMob`] / reward
/// chests, so a directly-staged actor would render invisibly. This marker lets
/// the renderer's runtime-visual discovery give it the same sprite pipeline every
/// other hostile actor gets — so "spawning a character" always shows the
/// character. Carries no lifecycle of its own (unlike `EncounterMob`); room-scope
/// despawn cleans it up.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimeStagedActor;

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

impl bevy::ecs::entity::MapEntities for ActorTarget {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        if let Some(entity) = self.entity.as_mut() {
            *entity = mapper.get_mapped(*entity);
        }
    }
}

impl bevy::ecs::entity::MapEntities for ActorAggression {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        if let Some(entity) = self.target.as_mut() {
            *entity = mapper.get_mapped(*entity);
        }
        if let Some(entity) = self.grudge.as_mut() {
            *entity = mapper.get_mapped(*entity);
        }
    }
}
