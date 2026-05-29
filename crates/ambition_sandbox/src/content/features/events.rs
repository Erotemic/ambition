use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureVisualKind {
    Hazard,
    Enemy,
    Sandbag,
    Boss,
    Breakable,
    Chest,
    Pickup,
    Npc,
    /// Latched switch. Renders as a colored block whose color depends
    /// on `FeatureView::switch_on` (red = off, green = on).
    Switch,
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
    /// reports 0.0 and renders axis-aligned. See
    /// `EnemyRuntime::rotation_rad` for the engine → Bevy mapping.
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

/// Typed cross-system gameplay effects emitted by feature code.
///
/// `GameplayEffect` is a Bevy [`Message`], not a payload hidden behind a
/// custom resource bus. Systems should write concrete effects with
/// [`MessageWriter<GameplayEffect>`] and consume them with small
/// domain-specific readers.
///
/// Do not add side-channel `Vec`s for progression/save/audio routing. Add a
/// typed message/effect and a focused consumer system instead.
#[derive(Message, Clone, Debug, PartialEq)]
pub enum GameplayEffect {
    /// Set a save/quest flag. Consumers mirror `on == true` into a
    /// `QuestAdvanceEvent::FlagSet` so flag-driven quest steps advance in the
    /// same frame as the save write.
    SetFlag { id: String, on: bool },
    /// Feed a structured quest event into `QuestRegistry`.
    AdvanceQuest(crate::quest::QuestAdvanceEvent),
    /// A Switch interactable was activated. Carries the parsed
    /// `SwitchActivation` (private to `crate::encounter`) directly — the
    /// `switch:<id>:<action>:<target>` wire string lives only at the
    /// engine `InteractionKind::Custom` boundary and is parsed once at
    /// LDtk spawn time.
    ActivateSwitch {
        activation: crate::encounter::SwitchActivation,
        pos: ae::Vec2,
    },
    /// Route damage into the boss encounter state machine.
    DamageBoss { boss_id: String, amount: i32 },
    /// Record that an NPC was struck. Today this is trace/reporting glue;
    /// hostility is flipped at the emit site.
    StrikeNpc { npc_id: String, pos: ae::Vec2 },
    /// SFX-only effect. Use typed presentation vectors for sounds that also
    /// imply VFX/progression, and this variant for standalone audio.
    PlaySfx {
        id: ambition_sfx::SfxId,
        pos: ae::Vec2,
    },
}

impl GameplayEffect {
    pub fn switch_activation(&self) -> Option<(&crate::encounter::SwitchActivation, ae::Vec2)> {
        match self {
            Self::ActivateSwitch { activation, pos } => Some((activation, *pos)),
            _ => None,
        }
    }

    pub fn sfx_play(&self) -> Option<(ambition_sfx::SfxId, ae::Vec2)> {
        match self {
            Self::PlaySfx { id, pos } => Some((*id, *pos)),
            _ => None,
        }
    }
}

/// Reset request for ECS-owned room features.
///
/// Same-room resets and full sandbox resets emit this once, and
/// `reset_ecs_room_features` consumes it through Bevy's message stream.
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResetRoomFeaturesEvent;

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
///   `PogoBounce`) — consumed by the feature-damage system to apply
///   damage to enemies / bosses / breakables.
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
    PlayerProjectile { kind: crate::projectile::ProjectileKind },
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
    /// Contact with a boss body (touched the boss itself).
    BossBody,
    /// Hit by a boss melee swing.
    BossAttack,
}

impl HitSource {
    /// True iff the source is attacker-side (player → feature). The
    /// feature-damage consumer filters by this; the player-damage
    /// consumer filters by the complement.
    pub fn is_attacker_side(&self) -> bool {
        matches!(
            self,
            HitSource::PlayerSlash { .. }
                | HitSource::PlayerProjectile { .. }
                | HitSource::PogoBounce
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
