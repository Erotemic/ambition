//! Combat-kit message/event vocabulary + small shared value types.
//!
//! Holds the feature-visual taxonomy (`FeatureVisualKind`, `BoundFeatureKind`,
//! the `FeatureView` render snapshot, `FeatureCombatTuning`), the hit model
//! (`HitMode`, `HitKnockback`, `ActorStimulus`), the typed gameplay-effect
//! messages consumed in [`bus`](super::bus) (`SetFlagRequested`,
//! `QuestAdvanceRequested`, `SwitchActivated`, `GameplaySfxRequested`), the
//! room-reset signals (`RoomResetReason`, `ResetRoomFeaturesEvent`), and the
//! `GameplayBanner` HUD resource. Pure data/messages — no systems.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureVisualKind {
    Hazard,
    Enemy,
    /// A passive practice target (struck to test damage/feedback). Rendered with
    /// a sandbag sprite — the depiction is content; the kit kind is generic.
    TrainingDummy,
    Boss,
    Breakable,
    Chest,
    Pickup,
    Npc,
    /// Latched switch. Renders as a colored block whose color depends
    /// on `FeatureView::switch_on` (red = off, green = on).
    Switch,
}

/// Marker binding a feature visual to its kind + collision size (moved here from
/// the render layer so the mount gameplay can remove it without importing presentation).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct BoundFeatureKind {
    pub kind: FeatureVisualKind,
    pub collision_size: ae::Vec2,
}

impl BoundFeatureKind {
    pub fn new(kind: FeatureVisualKind, collision: bevy::math::Vec2) -> Self {
        Self {
            kind,
            collision_size: ae::Vec2::new(collision.x, collision.y),
        }
    }

    pub fn matches(&self, kind: FeatureVisualKind, collision_size: ae::Vec2) -> bool {
        self.kind == kind && (self.collision_size - collision_size).length_squared() <= 0.25
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FeatureView {
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub kind: FeatureVisualKind,
    pub visible: bool,
    pub flash: bool,
    /// For `FeatureVisualKind::Switch`: true when the switch reads as
    /// "on" (encounter cleared / reset path armed). Renders green when
    /// true, red when false. Ignored for other kinds.
    pub switch_on: bool,
    /// Z-axis rotation to apply to the rendered sprite, in radians
    /// (Bevy frame; +π/2 is CCW). Non-zero for surface-walking
    /// archetypes that crawl on walls/ceilings; everyone else
    /// reports 0.0 and renders axis-aligned. Uses the engine → Bevy
    /// rotation mapping shared by actor rendering.
    pub rotation_rad: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct FeatureCombatTuning {
    pub enemy_attack_windup: f32,
    pub enemy_attack_active: f32,
    pub boss_attack_windup: f32,
    pub boss_attack_active: f32,
}

impl Default for FeatureCombatTuning {
    fn default() -> Self {
        Self {
            enemy_attack_windup: 0.36,
            enemy_attack_active: 0.20,
            boss_attack_windup: 0.52,
            boss_attack_active: 0.32,
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

/// Feed a structured quest event into `QuestRegistry`.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct QuestAdvanceRequested(pub crate::quest::QuestAdvanceEvent);

/// A Switch interactable was activated. Carries the parsed
/// `SwitchActivation` (private to `crate::encounter`) directly — the
/// `switch:<id>:<action>:<target>` wire string lives only at the engine
/// `InteractionKind::Custom` boundary and is parsed once at LDtk spawn time.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct SwitchActivated {
    pub activation: crate::encounter::SwitchActivation,
    pub pos: ae::Vec2,
}

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
    PlayerProjectile {
        kind: crate::projectile::ProjectileKind,
    },
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
                | HitSource::PlayerProjectile { .. }
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
    /// hits this is the broadcast / orb AABB; for resolved single-
    /// victim hits this is the AABB at the impact location.
    pub volume: ae::Aabb,
    /// Damage to apply.
    pub damage: i32,
    /// Who or what dealt the hit.
    pub source: HitSource,
    /// Attacker entity, if the producer knows it. Every player-
    /// attacker source (slash, pogo, player projectile) stamps the
    /// player whose attack landed — `apply_feature_hit_events`
    /// uses it to attribute hitstop / flash to the correct player.
    /// Hostile sources (Hazard / Enemy* / Boss*) leave this as
    /// `None` since the relevant "attacker identity" for the
    /// player-side reader is the source enum, not an Entity.
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
