//! Actor combat components: identity/disposition/target,
//! aggression, health, attack/combat state, cooldowns, and boss phase state.

use super::super::*;
use ambition_characters::actor::control::{BlockReason, IntentOutcome};

/// Who an actor is: its stable id and display name.
///
/// The ONE runtime owner of both. It is materialized with the body's cluster
/// (`ActorClusterSeed::into_components`, or a boss's spawn) and nothing writes
/// it afterwards, so there is no second copy for a per-tick pass to reconcile.
/// `FeatureId` remains the canonical entity lookup key.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct ActorIdentity {
    pub id: String,
    pub name: String,
}

impl ActorIdentity {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Marks a body whose death consequences are owned by a ruleset.
///
/// Damage still reaches zero health, but exploration respawn and enemy death
/// economy are suppressed so the owning ruleset can resolve the death.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RulesetOwnsDeath;

/// Marks a body that is actively participating in combat.
///
/// Rulesets attach and remove this at combat-entry/exit boundaries. It is
/// independent of [`ActorDisposition`], which describes AI/social hostility.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActiveCombatant;

/// Per-body lives/stocks remaining in a ruleset-owned death model.
///
/// Stocks are asymmetric per fighter and compose with unbounded damage plus
/// [`RulesetOwnsDeath`]; reaching zero eliminates the fighter.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct FighterStocks {
    /// Stocks left. `0` means ELIMINATED — out of the match, not respawning.
    pub remaining: u32,
    /// What it started with, kept so a HUD can draw "2 of 3" rather than
    /// inferring a maximum it was never told.
    pub started_with: u32,
}

impl FighterStocks {
    pub fn new(stocks: u32) -> Self {
        Self {
            remaining: stocks,
            started_with: stocks,
        }
    }

    /// Spend one stock and return whether the fighter is now eliminated.
    /// Saturation makes duplicate reports harmless.
    pub fn spend(&mut self) -> bool {
        self.remaining = self.remaining.saturating_sub(1);
        self.is_eliminated()
    }

    pub fn is_eliminated(self) -> bool {
        self.remaining == 0
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

/// Combat participation independent of AI/social disposition.
///
/// Active combatants remain damageable regardless of hostility; otherwise
/// hostile actors use the full damage path and bystanders use provocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatStanding {
    /// A ruleset put this body in a fight and owns its death. Damageable
    /// whatever its brain is doing.
    Combatant,
    /// Socially hostile: it chases and attacks, and it takes the full damage
    /// path.
    Hostile,
    /// A bystander. Striking one PROVOKES it — barks, strikes, the flip to
    /// hostile — rather than hurting it.
    Bystander,
}

impl CombatStanding {
    /// Derive standing from social disposition and the [`ActiveCombatant`] marker.
    pub fn of(disposition: ActorDisposition, active_combatant: bool) -> Self {
        if active_combatant {
            Self::Combatant
        } else if disposition.is_hostile() {
            Self::Hostile
        } else {
            Self::Bystander
        }
    }

    /// Whether this body is in a fight — so it keeps its attack state, holds its
    /// place on the anti-clump board, and does not stand down for want of an AI
    /// target it was never going to have.
    pub fn is_combatant(self) -> bool {
        matches!(self, Self::Combatant)
    }

    pub fn takes_damage(self) -> bool {
        !matches!(self, Self::Bystander)
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
/// Absent  the actor uses the legacy `collision_scale` render path.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ActorRenderSize(pub ae::Vec2);

/// Where to draw an actor's sprite quad RELATIVE to its body centre.
///
/// The companion to [`ActorRenderSize`]: that one says how big the quad is,
/// this one says where it goes. Both exist because a sheet frame is not its
/// character — the art sits somewhere inside the frame, usually off-centre, and
/// a quad centred on the body draws the character wherever the padding happens
/// to put it. Non-zero  shift the quad so the ART lands on the collision box.
///
/// Absent (the overwhelming majority)  the quad is centred on the body, which
/// is right for every sheet whose art is already frame-centred. Written by
/// `character_sprites::posed_body`, whose whole job is deriving this and
/// `ActorRenderSize` from the same authored rectangle so they cannot disagree.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ActorSpriteOffset(pub ae::Vec2);

/// The body's GROSS coarse-hurtbox footprint — the full-size box the published
/// `CenteredAabb` covers, distinct from `kin.size` (the collision box the
/// movement seam sweeps against walls) and from [`ActorRenderSize`] (the sprite
/// draw quad). Present  the shared body integrator publishes `CenteredAabb`
/// from THIS size; absent  it publishes from `kin.size` (the ordinary actor,
/// whose collision box IS its footprint).
///
/// This is the envelope split: a giant boss has
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
/// Reach is geometric: an interact press reaches this actor when the acting
/// body's box overlaps the actor's `CenteredAabb`
/// (`interact_ecs_actors_and_switches`). A peaceful patroller stops and faces
/// the player through its patrol brain's own `aggro_radius`
/// (`PatrolCfg::NPC_DEFAULT`); this component carries no range of its own.
#[derive(Component, Clone, Debug)]
pub struct ActorInteraction {
    pub interactable: ambition_interaction::Interactable,
}
// TODO(compat-remove): migrate remaining combat callers to
// `ambition_characters::actor::pose::ActorFaction`, then delete this re-export.
pub use ambition_characters::actor::pose::ActorFaction;

/// Per-actor current targeting read-model.
///
/// `select_actor_targets` derives `entity` each tick from the targeting policy. `pos` is
/// retained as rollback state so a no-candidate frame can preserve the previous aim.
/// The `Entity` handle is therefore a transient query handle rather than durable identity;
/// consumers must tolerate it becoming invalid after despawn/restore.
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
    /// Per-actor grudge: whom this actor has decided to oppose beyond its
    /// `FactionRelations` baseline (see [`Grudge`]). Targeting treats whoever it
    /// names as a foe just like a relational faction-foe, so a provoked NPC
    /// chases its attacker without mutating its `ActorFaction` identity. `None` =
    /// no grudge; the actor fights purely along faction lines. Grudges only form
    /// against real attackers — with friendly-fire off, `can_damage` blocks
    /// ally-on-ally hits, so no spurious `DamagedBy` stimulus (and no grudge)
    /// ever forms.
    pub grudge: Option<Grudge>,
}

/// Whom one actor opposes personally.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grudge {
    /// One body: the attacker that provoked it, or an authored feud.
    Body(Entity),
    /// Every body standing for a faction. A provocation read back from the save
    /// is this: the body that struck the blow is gone, and the room is built
    /// before any player body exists to name.
    Faction(ActorFaction),
}

impl Grudge {
    /// Does this grudge name `entity`, whose EFFECTIVE faction is `faction`?
    pub fn names(self, entity: Entity, faction: ActorFaction) -> bool {
        match self {
            Self::Body(body) => body == entity,
            Self::Faction(opposed) => opposed == faction,
        }
    }

    /// The one body this grudge names, if it names one.
    pub fn body(self) -> Option<Entity> {
        match self {
            Self::Body(body) => Some(body),
            Self::Faction(_) => None,
        }
    }
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

    /// Has this actor been provoked past its own threshold?
    ///
    /// ⛔⛔ THE ONE AUTHORITY, and it exists because there were two. The
    /// per-body `strike_threshold` in [`AggressionMode::RetaliatesWhenHit`] is
    /// what canonical aggression resolution asks, but the damage road asked a
    /// GLOBAL DEFAULT constant instead — `NPC_HOSTILE_STRIKE_THRESHOLD` — when
    /// deciding to write the persistent hostile flag, play the hostile bark and
    /// raise the banner.
    ///
    /// ⚠ AND THE TWO AGREED BY COINCIDENCE, which is why nothing caught it:
    /// ordinary NPC spawn policy sets `strike_threshold` FROM that same constant,
    /// so both sides computed 3 and matched. Author a body with a threshold of 5
    /// and the third hit turns it hostile in the save file and in the speech
    /// bubble while the mechanics say it is still peaceful; author 1 and the
    /// mechanics turn hostile two hits before anything tells the player.
    ///
    /// ⇒ The constant remains correct as the DEFAULT SPAWN VALUE. It stopped
    /// being the answer the moment it was copied into a per-body field.
    pub fn provoked(self) -> bool {
        match self.mode {
            AggressionMode::RetaliatesWhenHit { strike_threshold } => {
                self.strikes >= i32::from(strike_threshold)
            }
            AggressionMode::Hostile => true,
            AggressionMode::Passive => false,
        }
    }

    /// Who this actor wants to look at / chase this frame, derived from
    /// its aggression mode rather than its [`ActorFaction`]. This is the
    /// seam [`select_actor_targets`](crate::targeting::select_actor_targets)
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
/// [`select_actor_targets`](crate::targeting::select_actor_targets).
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

/// The melee swing a body is performing NOW, as a read-only view.
///
/// ⛔ NOT STORED. It is derived on demand from the body's live
/// [`MovePlayback`](crate::moveset::MovePlayback) by
/// [`melee_swing_of`](crate::moveset::melee_swing_of). It used to be
/// `BodyMelee::swing`, a copy the moveset projected every tick and the rollback
/// codec also restored, and two damage sites wrote hit keys into the copy that
/// the next projection threw away. The move owns every one of these facts;
/// this only reshapes them for the anim picker, the HUD, the gizmos and the
/// brain snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct MeleeSwing {
    /// The swing's timing and read-model direction. Every geometry and impulse
    /// field is inert: the real strike is the move's own hitbox.
    pub spec: crate::AttackSpec,
    /// Seconds since the swing began (the move's `t`).
    pub elapsed: f32,
    /// `prefix:id` keys of every target this swing has struck (the move's
    /// `hit_targets`).
    pub hit_targets: Vec<String>,
}

impl MeleeSwing {
    pub fn new(spec: crate::AttackSpec) -> Self {
        Self {
            spec,
            elapsed: 0.0,
            hit_targets: Vec::new(),
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

    pub fn is_winding_up(&self) -> bool {
        matches!(self.phase(), Some(crate::AttackPhase::Startup))
    }

    pub fn is_active(&self) -> bool {
        matches!(self.phase(), Some(crate::AttackPhase::Active))
    }

    /// Seconds of windup (startup) remaining, for the AI telegraph snapshot.
    pub fn windup_remaining(&self) -> f32 {
        if self.is_winding_up() {
            (self.spec.startup_seconds - self.elapsed).max(0.0)
        } else {
            0.0
        }
    }

    /// Seconds of active (hitbox) window remaining, for the AI snapshot.
    pub fn active_remaining(&self) -> f32 {
        if self.is_active() {
            (self.spec.startup_seconds + self.spec.active_seconds - self.elapsed).max(0.0)
        } else {
            0.0
        }
    }
}

/// The body's melee pacing floor — the one melee fact the body itself holds.
///
/// `cooldown` is the floor a brain reads to time its next swing. A melee swing
/// a brain starts arms it from that brain's authored
/// `BrainProfile::attack_cooldown_s`; a participant-driven swing never does.
/// The swing in flight is not stored here: see [`MeleeSwing`]. The ranged
/// fire-rate floor is not melee state: it is [`RangedRefire`].
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct BodyMelee {
    /// Recovery/AI pacing floor before another swing may begin (s).
    pub cooldown: f32,
}

impl BodyMelee {
    pub fn on_cooldown(&self) -> bool {
        self.cooldown > 0.0
    }

    /// Advance the cooldown floor by `dt`.
    pub fn tick(&mut self, dt: f32) {
        self.cooldown = (self.cooldown - dt.max(0.0)).max(0.0);
    }
}

/// The body's ranged fire-rate floor (invariant I3).
///
/// Not the brain's cadence: a controller may attempt `fire` every tick; the
/// body accepts a shot only when the floor is spent and re-arms it on each
/// accepted shot, so a spam controller and a human produce the same weapon
/// rate. Rollback state (`actor.ranged_refire`).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct RangedRefire {
    /// Seconds until the next shot may leave the weapon.
    pub remaining: f32,
}

impl RangedRefire {
    /// May a shot leave the weapon now?
    pub fn ready(&self) -> bool {
        self.remaining <= 0.0
    }

    /// A committed shot spends the weapon for `refire_seconds`.
    pub fn arm(&mut self, refire_seconds: f32) {
        self.remaining = self.remaining.max(refire_seconds.max(0.0));
    }

    pub fn tick(&mut self, dt: f32) {
        self.remaining = (self.remaining - dt.max(0.0)).max(0.0);
    }

    /// An ATTEMPTED shot: accepted only when the weapon is ready, which
    /// re-arms it. Identical for an AI spam controller, a tactical brain and
    /// a human.
    pub fn try_fire(&mut self, refire_seconds: f32) -> IntentOutcome {
        if !self.ready() {
            return IntentOutcome::Blocked(BlockReason::Cooldown);
        }
        self.remaining = refire_seconds.max(0.0);
        IntentOutcome::Accepted
    }
}

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
        if let Some(Grudge::Body(entity)) = self.grudge.as_mut() {
            *entity = mapper.get_mapped(*entity);
        }
    }
}

#[cfg(test)]
mod actor_disposition_tests {
    use super::*;

    /// The state a platform fighter is in for most of a round.
    ///
    /// it was inexpressible while one field answered two questions: a fighter
    /// somebody entered into a match had to be socially `Hostile` merely to be
    /// damageable, and AI targeting hunts live foes for a BRAIN — so two
    /// human-driven fighters held no target, both stood down to `Peaceful`, and
    /// neither could hurt the other.
    #[test]
    fn a_combatant_is_damageable_however_its_brain_feels() {
        let standing = CombatStanding::of(ActorDisposition::Peaceful, true);
        assert_eq!(standing, CombatStanding::Combatant);
        assert!(
            standing.takes_damage(),
            "a human-driven fighter is socially hostile toward nobody and is \
             still in the fight"
        );
        assert!(
            standing.is_combatant(),
            "and it keeps its attack state — a body that can be hit and cannot \
             swing is half a fighter"
        );
    }

    /// A town NPC is unchanged, which is the half that must not regress:
    /// striking one PROVOKES it rather than hurting it.
    #[test]
    fn a_bystander_is_provoked_rather_than_hurt() {
        let standing = CombatStanding::of(ActorDisposition::Peaceful, false);
        assert_eq!(standing, CombatStanding::Bystander);
        assert!(!standing.takes_damage());
    }

    /// Being in a fight is not the same fact as whose business your death
    /// is, and the eliminated fighter is where the two come apart.
    ///
    /// An eliminated fighter still carries that marker — the match still owns its KO — and its body
    /// stays standing until a ruleset removes it, so it went on being a damageable combatant with
    /// no stocks left, holding attack state and a place on the anti-clump board.
    #[test]
    fn an_eliminated_fighter_is_no_longer_a_combatant() {
        // Elimination removes the participation marker and leaves death
        // ownership alone; this is the standing that results.
        let standing = CombatStanding::of(ActorDisposition::Peaceful, false);
        assert_ne!(
            standing,
            CombatStanding::Combatant,
            "a fighter that is OUT is not in the fight, however the match feels \
             about its corpse"
        );
    }

    /// Social hostility still stands on its own, for every AI body that
    /// joins a fight without a ruleset putting it in one.
    #[test]
    fn a_hostile_body_needs_no_ruleset() {
        let standing = CombatStanding::of(ActorDisposition::Hostile, false);
        assert_eq!(standing, CombatStanding::Hostile);
        assert!(standing.takes_damage());
    }
}
