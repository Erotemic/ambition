//! Combat-kit message/event vocabulary + small shared value types.
//!
//! Holds `FeatureCombatTuning`, the hit model
//! (`HitMode`, `HitKnockback`, `ActorStimulus`), the typed gameplay-effect
//! messages consumed in [`bus`](super::bus) (`SetFlagRequested`,
//! `QuestAdvanceRequested`, `SwitchActivated`, `GameplaySfxRequested`), the
//! room-reset signals (`RoomResetReason`, `ResetRoomFeaturesEvent`), and the
//! `GameplayBanner` HUD resource. Pure data/messages — no systems.

use super::*;

// The feature-visual taxonomy (`FeatureVisualKind`, `BoundFeatureKind`) moved
// to `ambition_platformer_primitives::feature_kind` and the `FeatureView`
// read-model row to `ambition_sim_view` (recon C2): the taxonomy is foundation
// vocabulary and the row is the read-model's own — neither is combat model.

#[derive(Clone, Copy, Debug)]
pub struct FeatureCombatTuning {
    pub enemy_attack_windup: f32,
    pub enemy_attack_active: f32,
    pub boss_attack_windup: f32,
    pub boss_attack_active: f32,
}

/// Default attack-phase timings (seconds). Single source of truth, shared by
/// [`FeatureCombatTuning::default`] and `SandboxFeelTuning::default` (which
/// projects them back out via `SandboxFeelTuning::feature_combat_tuning`).
pub const DEFAULT_ENEMY_ATTACK_WINDUP: f32 = 0.36;
pub const DEFAULT_ENEMY_ATTACK_ACTIVE: f32 = 0.20;
pub const DEFAULT_BOSS_ATTACK_WINDUP: f32 = 0.52;
pub const DEFAULT_BOSS_ATTACK_ACTIVE: f32 = 0.32;

impl Default for FeatureCombatTuning {
    fn default() -> Self {
        Self {
            enemy_attack_windup: DEFAULT_ENEMY_ATTACK_WINDUP,
            enemy_attack_active: DEFAULT_ENEMY_ATTACK_ACTIVE,
            boss_attack_windup: DEFAULT_BOSS_ATTACK_WINDUP,
            boss_attack_active: DEFAULT_BOSS_ATTACK_ACTIVE,
        }
    }
}

/// Victim reaction mode for `HitEvent`s landing on a player. Ignored
/// for non-player targets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HitMode {
    /// Normal combat damage: preserve the room and apply knockback
    /// plus hitstun.
    #[default]
    Knockback,
    /// Lava / spike-pit style recovery: put the player back on the
    /// last safe platform.
    SafeRespawn,
}

/// Knockback impulse carried by a `HitEvent`. Producers fill this on
/// hits that should push the victim around (enemy melee, enemy
/// projectile, boss swing); leave `None` for impulse-free hits
/// (player slash, player projectile into a feature, pogo).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitKnockback {
    /// Horizontal impulse direction (±1).
    pub dir: f32,
    /// Strength multiplier. 1.0 is "standard".
    pub strength: f32,
    /// World-space attacker position — used for VFX direction.
    pub source_pos: ae::Vec2,
    /// World-space impact position — used for VFX position.
    pub impact_pos: ae::Vec2,
    /// Authored launch DIRECTION in the victim's gravity frame (CM1): `x` =
    /// lateral (mirrored to point away from the source by the resolver's
    /// side sign), `y` = upward against gravity. `None` = the feel-tuned
    /// default diagonal (today's launch exactly).
    pub launch_dir: Option<ae::Vec2>,
}

/// Relationship/AI stimuli observed by actors.
///
/// Damage systems should emit facts such as "this actor was damaged by that
/// entity". Aggression/relationship systems decide whether that means fight,
/// flee, ignore, call for help, or future faction-specific behavior.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorStimulus {
    DamagedBy {
        actor: Entity,
        source: Option<Entity>,
        damage: i32,
    },
    /// The actor was explicitly challenged to a fight (e.g. the player picked
    /// the "challenge" dialogue option). Provokes the actor into combat
    /// unconditionally — bypassing the strike-threshold gate that `DamagedBy`
    /// respects — because the challenge IS the deliberate consent to fight.
    /// `challenger` is who threw down the gauntlet (the player), used as the
    /// initial chase target.
    Challenged {
        actor: Entity,
        challenger: Option<Entity>,
    },
}

// Typed cross-system gameplay effects emitted by feature code.
//
// Each is its own Bevy [`Message`] with a focused consumer in `bus.rs`.
// This deliberately replaces the former single `GameplayEffect` enum bus:
// unrelated domain events (save flags, quest advances, switch activations,
// audio) no longer share one channel, so a consumer only declares a reader
// for the message it actually handles.
//
// Do not reintroduce a generic effect enum or side-channel `Vec`s for
// progression/save/audio routing. Add a typed message + a focused consumer
// system instead.

/// Set a save/quest flag. The consumer mirrors `on == true` into a
/// `QuestAdvanceEvent::FlagSet` so flag-driven quest steps advance in the
/// same frame as the save write.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct SetFlagRequested {
    pub id: String,
    pub on: bool,
}

// `QuestAdvanceRequested` moved to `crate::quest` (E2): quest owns its
// advance vocabulary; combat must not name it.

// `SwitchActivated` moved to `crate::encounter::switches` (E2): it names
// encounter vocabulary; combat must not.

/// Standalone audio-only gameplay effect. Use typed presentation vectors for
/// sounds that also imply VFX/progression, and this message for bare audio.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct GameplaySfxRequested {
    pub id: ambition_sfx::SfxId,
    pub pos: ae::Vec2,
}

/// Why a room reset fired. Lets a consumer treat a player DEATH differently from
/// a deliberate MANUAL reset — e.g. the portal adapter preserves the player's gun
/// portals across a death but clears them on a manual reset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoomResetReason {
    /// The player died / fell out of the world (engine-raised reset). Portals are
    /// PRESERVED so a death doesn't wipe the player's gun-portal setup.
    PlayerDeath,
    /// A deliberate reset: the manual delete-key reset or a scripted room replay.
    /// The gun's portals are cleared (authored level portals are always spared by
    /// `clear_portals_on_reset`). Default so any plain construction clears.
    #[default]
    Manual,
}

/// Reset request for ECS-owned room features.
///
/// Same-room resets and full sandbox resets emit this once, and
/// `reset_ecs_room_features` consumes it through Bevy's message stream.
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResetRoomFeaturesEvent {
    pub reason: RoomResetReason,
}

/// Runtime HUD banner state owned directly by Bevy ECS.
///
/// Gameplay systems either mutate this resource directly or emit
/// [`GameplayBannerRequested`] when their parameter list is already large.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct GameplayBanner {
    pub text: String,
    pub timer: f32,
}

impl GameplayBanner {
    pub fn show(&mut self, text: impl Into<String>, duration: f32) {
        self.text = text.into();
        self.timer = duration.max(0.0);
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.timer = 0.0;
    }

    pub fn visible(&self) -> bool {
        self.timer > 0.0 && !self.text.is_empty()
    }

    pub fn tick(&mut self, dt: f32) {
        self.timer = (self.timer - dt).max(0.0);
        if self.timer <= 0.0 {
            self.text.clear();
        }
    }
}

/// Message form for systems that cannot cheaply acquire `ResMut<GameplayBanner>`
/// without bloating an already-large system signature.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct GameplayBannerRequested {
    pub text: String,
    pub duration: f32,
}

impl GameplayBannerRequested {
    pub fn new(text: impl Into<String>, duration: f32) -> Self {
        Self {
            text: text.into(),
            duration,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpcDialogueRequest {
    pub npc_id: String,
    pub npc_name: String,
    pub dialogue_id: String,
}

/// Source of a hit event. Lets per-target damage logic branch on the
/// originator and lets the trace / HUD label hits.
///
/// `HitSource` partitions naturally into two directions:
/// - **Attacker-side** (`PlayerSlash`, `PlayerProjectile`,
///   `PogoBounce`, self-destruct style enemy crashes) — consumed by
///   the feature-damage system to apply damage to enemies / bosses /
///   breakables.
/// - **Victim-side** (`Hazard`, `EnemyBody`, `EnemyAttack`,
///   `EnemyProjectile`, `BossBody`, `BossAttack`) — consumed by the
///   player-damage system to apply damage to players.
///
/// New attack sources should add a variant here rather than building
/// a parallel `apply_*_attack` path. The canonical channel is
/// [`HitEvent`].
#[derive(Clone, Debug, PartialEq)]
pub enum HitSource {
    /// Player melee slash. `knock_x` is the horizontal impulse to
    /// add to a hit enemy's velocity (sign tied to player facing).
    /// Slashes also nudge enemies upward.
    PlayerSlash { knock_x: f32 },
    /// Player projectile (Fireball / Hadouken). No knockback today;
    /// the projectile's own velocity carries the visual feedback.
    PlayerProjectile,
    /// Pogo bounce on a breakable orb. The carrying `HitEvent`'s
    /// `volume` field is the orb's authoritative AABB; the consumer
    /// matches it against `pogo_refresh` breakables via
    /// `approximately_same_aabb` rather than the broadcast
    /// `strict_intersects` used by every other source. Actor / boss
    /// targets are skipped under this source.
    PogoBounce,
    /// Environmental hazard (spike, lava, falling debris). Victim
    /// reaction depends on `HitEvent::mode` — `SafeRespawn` returns
    /// the player to the last safe platform; `Knockback` applies
    /// hitstun + knockback.
    Hazard,
    /// Contact with an enemy body (touched the enemy itself, not
    /// its swing). Always knockback mode.
    EnemyBody,
    /// Hit by an enemy melee swing.
    EnemyAttack,
    /// Hit by an enemy-fired projectile (pirate volley, etc).
    /// Distinct from `EnemyAttack` so the HUD / trace can tell the
    /// player whether they took a contact swing or a ranged shot.
    EnemyProjectile,
    /// Enemy self-crash / self-destruct hit. Used by special charge
    /// behaviors that intentionally ram a wall and explode.
    EnemyChargeCrash,
    /// Contact with a boss body (touched the boss itself).
    BossBody,
    /// Hit by a boss melee swing.
    BossAttack,
}

impl HitSource {
    /// True iff the source is attacker-side (player → feature, or a
    /// feature self-destructing into the world). The feature-damage
    /// consumer filters by this; the player-damage consumer filters
    /// by the complement.
    pub fn is_attacker_side(&self) -> bool {
        matches!(
            self,
            HitSource::PlayerSlash { .. }
                | HitSource::PlayerProjectile
                | HitSource::PogoBounce
                | HitSource::EnemyChargeCrash
        )
    }
}

/// How a hit event resolves its victim.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HitTarget {
    /// Broadcast: any feature / actor / boss whose AABB intersects
    /// `volume` takes the hit. Default for attacker-side broadcast
    /// hits (player slash, player projectile, hazard areas in
    /// authoring zones).
    #[default]
    Volume,
    /// Single pre-resolved player victim. Producers that already
    /// iterated players and picked the overlapping one stamp this
    /// so the reader doesn't re-pick the primary by default.
    Player(bevy::prelude::Entity),
    /// Single pre-resolved NON-player actor victim. Stamped by a producer that
    /// already resolved overlap + faction hostility (`FactionRelations`) and
    /// picked the actor to damage — the relational actor-vs-actor path (S3e). The
    /// actor-damage consumer applies it to exactly this entity; the player-damage
    /// consumer ignores it. This is how an Enemy-faction body's swing damages a
    /// Boss-faction body without the bipartite player/enemy assumption.
    Actor(bevy::prelude::Entity),
    /// Orb-AABB match (pogo). Only the breakable whose AABB
    /// approximately equals `volume` is hit; actors / bosses are
    /// skipped.
    OrbMatch,
}

/// One hit event in world space — the single canonical channel for
/// damage in either direction (attacker → feature, or anything →
/// player). Producers emit these as Bevy messages; the feature- and
/// player-damage systems filter by source-direction and apply.
///
/// Source-specific resolution:
/// - `PlayerSlash` / `PlayerProjectile`: broadcast match — every
///   feature whose AABB strict-intersects `volume` takes a hit.
/// - `PogoBounce`: orb-exact match — only the breakable whose AABB
///   approximately equals `volume` is hit; actors / bosses are skipped.
/// - `Hazard` / `Enemy*` / `Boss*` with `target = Player(e)`: the
///   pre-resolved player victim takes the hit (mode + knockback
///   applied). `target = Volume` falls back to the primary player.
#[derive(Message, Clone, Debug)]
pub struct HitEvent {
    /// World-space volume the hit covers. For broadcast / orb-match
    /// hits this is the broadcast / orb volume; for resolved single-
    /// victim hits this is the volume at the impact location. A
    /// [`CombatVolume`] so an attack can carry an effect-shaped (rotated
    /// or convex) hitbox; the common case is still an axis-aligned box
    /// (`Aabb` converts via `.into()`).
    pub volume: ae::CombatVolume,
    /// Damage to apply.
    pub damage: i32,
    /// Who or what dealt the hit.
    pub source: HitSource,
    /// Attacker entity, if the producer knows it. Player-attacker
    /// sources (slash, pogo, player projectile) stamp the player whose
    /// attack landed — `apply_feature_hit_events` uses it to attribute
    /// hitstop / flash to the correct player. Hostile sources stamp the
    /// attacking entity symmetrically where one exists: `BossAttack` /
    /// `BossBody` carry the boss, `EnemyBody` / `EnemyChargeCrash` the
    /// enemy — so the victim's `DeathCause` records who killed it (the
    /// compact causality seam for replay / RL / future netcode). Sources
    /// with no entity attacker stay `None`: `Hazard` (environmental) and
    /// `EnemyProjectile` (string-`ProjectileOwnerId`-owned, not entity-
    /// tracked).
    pub attacker: Option<bevy::prelude::Entity>,
    /// Hint for how the consumer resolves the victim. See
    /// [`HitTarget`].
    pub target: HitTarget,
    /// Reaction mode for player victims (`Knockback` / `SafeRespawn`).
    /// Ignored for non-player targets.
    pub mode: HitMode,
    /// Knockback impulse to apply to the victim. `None` for impulse-
    /// free sources (player slash uses its own per-source `knock_x`
    /// field on `HitSource::PlayerSlash`; pogo / player-projectile
    /// don't push their target around).
    pub knockback: Option<HitKnockback>,
    /// Target keys that have already been hit by this one-hit-per-
    /// target source. Empty for ordinary one-frame projectiles /
    /// hazards / pogos.
    pub ignored_targets: Vec<String>,
}
