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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerDamageMode {
    /// Lava/spike-pit style recovery: put the player back on the last safe platform.
    SafeRespawn,
    /// Normal combat damage: preserve the room and apply knockback plus hitstun.
    Knockback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerDamageSource {
    Hazard,
    EnemyBody,
    EnemyAttack,
    /// Hit by a projectile fired by an enemy (pirate volley, etc).
    /// Distinct from `EnemyAttack` so the HUD/trace can tell the
    /// player whether they took a contact swing or a ranged shot.
    EnemyProjectile,
    BossBody,
    BossAttack,
}

#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct PlayerDamageEvent {
    pub mode: PlayerDamageMode,
    pub source: PlayerDamageSource,
    pub source_pos: ae::Vec2,
    pub impact_pos: ae::Vec2,
    pub knockback_dir: f32,
    pub strength: f32,
    pub amount: i32,
    /// Player entity the damage targets. `None` keeps the legacy
    /// "primary player takes it" routing (used by enemy contact / boss
    /// patterns whose AI already targets primary via `PrimaryPlayerOnly`).
    /// `Some(entity)` is what iterate-all-players producers (hazards,
    /// enemy projectiles) stamp once per overlapping player so the
    /// reader-side per-player damage routing (OVERNIGHT-TODO #17.6) can
    /// land them on the right player rather than amplifying onto the
    /// primary. The reader honors the field where the per-entity apply
    /// path is wired up; until then the field documents producer intent.
    pub target: Option<bevy::prelude::Entity>,
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
    AdvanceQuest(ae::QuestAdvanceEvent),
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

/// Pogo hit against an ECS-owned breakable/rebound target.
///
/// The engine reports the orb AABB immediately during player motion; the ECS
/// breakable damage system resolves the actual target after the main sim tick.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct PogoBounceEvent {
    pub orb_aabb: ae::Aabb,
    pub damage: i32,
}

impl PogoBounceEvent {
    pub fn new(orb_aabb: ae::Aabb, damage: i32) -> Self {
        Self { orb_aabb, damage }
    }
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

/// Source of a damage event. Lets per-target damage logic branch on
/// the originator (slash applies upward + horizontal knockback;
/// projectile doesn't push the target around) and lets the trace /
/// HUD label hits.
///
/// New damage sources should add a variant here rather than building a
/// parallel `apply_*_attack` path. The unified path is `DamageEvent` consumed
/// by ECS feature-damage systems.
#[derive(Clone, Debug, PartialEq)]
pub enum DamageSource {
    /// Player melee slash. `knock_x` is the horizontal impulse to
    /// add to a hit enemy's velocity (sign tied to player facing).
    /// Slashes also nudge enemies upward.
    PlayerSlash { knock_x: f32 },
    /// Player projectile (Fireball / Hadouken). No knockback today;
    /// the projectile's own velocity carries the visual feedback.
    PlayerProjectile { kind: ae::ProjectileKind },
    /// Pogo bounce on a breakable orb. Kept here so future damage-source
    /// consumers see the full set in one place; pogo-orb resolution itself
    /// uses [`PogoBounceEvent`].
    PogoBounce,
}

/// One damage event in world space: an AABB volume to test against every
/// damageable feature, the amount to apply, and the source. Producers emit
/// these as Bevy messages; ECS feature-damage systems resolve them against
/// actor, boss, and breakable components.
#[derive(Message, Clone, Debug)]
pub struct DamageEvent {
    pub volume: ae::Aabb,
    pub damage: i32,
    pub source: DamageSource,
    /// Target keys that have already been hit by this one-hit-per-target
    /// source. Empty for ordinary one-frame projectiles / hazards.
    pub ignored_targets: Vec<String>,
}
