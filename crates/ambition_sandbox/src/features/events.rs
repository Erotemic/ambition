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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeaturePhysicsCue {
    Breakable,
    EnemyRagdoll,
    BossRagdoll,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeaturePhysicsBurst {
    pub pos: ae::Vec2,
    pub cue: FeaturePhysicsCue,
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
    BossBody,
    BossAttack,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerDamageEvent {
    pub mode: PlayerDamageMode,
    pub source: PlayerDamageSource,
    pub source_pos: ae::Vec2,
    pub impact_pos: ae::Vec2,
    pub knockback_dir: f32,
    pub strength: f32,
    pub amount: i32,
}

#[derive(Default, Clone, Debug)]
pub struct FeatureEvents {
    pub dialogue_request: Option<NpcDialogueRequest>,
    /// Legacy room-reset flag. New player damage should prefer `player_damage`
    /// so lava-like hazards and enemy attacks can resolve differently.
    pub reset_player: bool,
    pub consumed_interaction: bool,
    pub messages: Vec<String>,
    pub impacts: Vec<ae::Vec2>,
    pub bursts: Vec<ae::Vec2>,
    pub physics_bursts: Vec<FeaturePhysicsBurst>,
    pub player_damage: Vec<PlayerDamageEvent>,
    pub player_heal: i32,
    /// Switch interactables the player activated this frame. Drained
    /// by the encounter system into `SwitchActivationQueue`. The
    /// payload is the Custom string from `Interactable::Custom("switch:...")`
    /// — the consumer parses it via `SwitchActivation::parse_custom`.
    pub switch_activations: Vec<String>,
    /// Per-boss damage events from this frame's player attack(s).
    /// `(boss_runtime_id, damage_amount)`. Drained by
    /// `boss_encounter::record_boss_damage` so the encounter phase
    /// machine can react.
    pub boss_damage: Vec<(String, i32)>,
    /// NPCs the player struck this frame. Used by the hostile-NPC
    /// system to convert peaceful NPCs into combat encounters.
    /// `(npc_id, npc_pos)`.
    pub npc_struck: Vec<(String, ae::Vec2)>,
    /// Quest advance events surfaced from gameplay this frame
    /// (e.g. NPC talked, item collected). Drained by
    /// `quest::apply_quest_advance_events`. Kept as `String` payloads
    /// so the engine doesn't need a Bevy-aware enum here.
    pub quest_advance: Vec<ae::QuestAdvanceEvent>,
    /// Save flags to set this frame. `(flag_id, on)`. Routed by the
    /// sandbox runtime into `SandboxSave`.
    pub flag_writes: Vec<(String, bool)>,
    /// Position of every chest the player opened this frame. The
    /// presentation layer maps these to `world.treasure_chest.open`
    /// SFX. Sim-only callers (headless / RL) ignore this.
    pub chests_opened: Vec<ae::Vec2>,
    /// `(kind, pos)` of every pickup collected this frame. The
    /// presentation layer dispatches per-kind SFX (coin / health).
    pub pickups_collected: Vec<(ae::PickupKind, ae::Vec2)>,
    /// Position of every breakable destroyed this frame (in addition
    /// to the existing `physics_bursts.Breakable` entry which already
    /// drives debris). Drives the `world.crate.break` SFX.
    pub breakables_destroyed: Vec<ae::Vec2>,
    /// Position of every switch activated this frame. Pairs 1:1 with
    /// `switch_activations` (which carries the payload string). Drives
    /// the `world.switch.toggle` SFX.
    pub switches_activated_pos: Vec<ae::Vec2>,
    /// Generic SFX-only events: `(SfxId, pos)`. The presentation layer
    /// drains these into `SfxMessage::Play`. Use this for events that
    /// are *only* audible — anything that also drives VFX, persistence,
    /// or quest hooks should go through a typed event vec above so
    /// every consumer can subscribe independently.
    ///
    /// Helper: `events.play_sfx(id, pos)`.
    pub sfx_plays: Vec<(ambition_sfx::SfxId, ae::Vec2)>,
}

impl FeatureEvents {
    pub fn merge(&mut self, mut other: Self) {
        self.reset_player |= other.reset_player;
        self.consumed_interaction |= other.consumed_interaction;
        self.messages.append(&mut other.messages);
        if other.dialogue_request.is_some() {
            self.dialogue_request = other.dialogue_request;
        }
        self.impacts.append(&mut other.impacts);
        self.bursts.append(&mut other.bursts);
        self.physics_bursts.append(&mut other.physics_bursts);
        self.player_damage.append(&mut other.player_damage);
        self.player_heal += other.player_heal;
        self.switch_activations
            .append(&mut other.switch_activations);
        self.boss_damage.append(&mut other.boss_damage);
        self.npc_struck.append(&mut other.npc_struck);
        self.quest_advance.append(&mut other.quest_advance);
        self.flag_writes.append(&mut other.flag_writes);
        self.chests_opened.append(&mut other.chests_opened);
        self.pickups_collected.append(&mut other.pickups_collected);
        self.breakables_destroyed
            .append(&mut other.breakables_destroyed);
        self.switches_activated_pos
            .append(&mut other.switches_activated_pos);
        self.sfx_plays.append(&mut other.sfx_plays);
    }

    /// Enqueue a one-shot SFX at a position. Cheap helper for sim-side
    /// code that wants to play a sound without adding a dedicated
    /// typed event vec. Drained by `handle_feature_events` into
    /// `SfxMessage::Play`.
    pub fn play_sfx(&mut self, id: ambition_sfx::SfxId, pos: ae::Vec2) {
        self.sfx_plays.push((id, pos));
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
/// New damage sources should add a variant here rather than building
/// a parallel `apply_*_attack` method on `FeatureRuntime`. The unified
/// path is `apply_damage_event`.
#[derive(Clone, Debug, PartialEq)]
pub enum DamageSource {
    /// Player melee slash. `knock_x` is the horizontal impulse to
    /// add to a hit enemy's velocity (sign tied to player facing).
    /// Slashes also nudge enemies upward.
    PlayerSlash { knock_x: f32 },
    /// Player projectile (Fireball / Hadouken). No knockback today;
    /// the projectile's own velocity carries the visual feedback.
    PlayerProjectile { kind: ae::ProjectileKind },
    /// Pogo bounce on a breakable orb. Routed via the legacy
    /// `on_pogo_bounce` path because it's pogo-orb-specific (only
    /// `pogo_refresh` breakables react). Listed here so future
    /// damage-source consumers see the full set in one place.
    PogoBounce,
}

/// One damage event in world space: an AABB volume to test against
/// every damageable feature, the amount to apply, and the source. The
/// caller produces these once per frame; `FeatureRuntime::apply_damage_event`
/// resolves them across all target collections in a single pass and
/// returns a `DamageReport` so the caller can decide what to do with
/// the source (despawn projectile, reduce durability, …).
#[derive(Clone, Debug)]
pub struct DamageEvent {
    pub volume: ae::Aabb,
    pub damage: i32,
    pub source: DamageSource,
}

/// What `apply_damage_event` did. Carries the side-effect bundle
/// (sounds, VFX, save-flag writes — already accepted into the runtime
/// by the time the report returns) plus a structured tally of who
/// was hit so callers can branch.
///
/// Counts are usize because a wide damage volume can clip multiple
/// enemies at once (think: a spell AOE). For projectile expiry the
/// caller usually only checks `any_actor_hit()`.
#[derive(Clone, Debug, Default)]
pub struct DamageReport {
    pub events: FeatureEvents,
    pub enemies_hit: usize,
    pub bosses_hit: usize,
    pub breakables_hit: usize,
    pub npcs_hit: usize,
    pub kills: usize,
}

impl DamageReport {
    /// True iff any damageable feature was hit. Used by projectile
    /// resolution to decide whether to despawn the body.
    pub fn any_actor_hit(&self) -> bool {
        self.enemies_hit + self.bosses_hit + self.breakables_hit + self.npcs_hit > 0
    }
}
