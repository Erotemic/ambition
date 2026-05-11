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

/// Typed cross-system gameplay effects emitted by feature code.
///
/// `FeatureEvents` still owns presentation-facing vectors such as impacts,
/// bursts, chest-open cues, and pickup-collected cues. Anything that changes
/// progression state or needs to fan out to another gameplay system should go
/// through this table instead of adding another parallel stringly typed vector.
#[derive(Clone, Debug, PartialEq)]
pub enum GameplayEffect {
    /// Set a save/quest flag. Consumers mirror `on == true` into a
    /// `QuestAdvanceEvent::FlagSet` so flag-driven quest steps advance in the
    /// same frame as the save write.
    SetFlag { id: String, on: bool },
    /// Feed a structured quest event into `QuestRegistry`.
    AdvanceQuest(ae::QuestAdvanceEvent),
    /// A Switch interactable was activated. `payload` is still the authored
    /// Custom string for now, but it is contained in one typed effect variant
    /// instead of a standalone side-channel.
    ActivateSwitch { payload: String, pos: ae::Vec2 },
    /// Route damage into the boss encounter state machine.
    DamageBoss { boss_id: String, amount: i32 },
    /// Record that an NPC was struck. Today this is trace/reporting glue;
    /// hostility is flipped at the emit site.
    StrikeNpc { npc_id: String, pos: ae::Vec2 },
    /// SFX-only effect. Use typed presentation vectors for sounds that also
    /// imply VFX/progression, and this variant for standalone audio.
    PlaySfx { id: ambition_sfx::SfxId, pos: ae::Vec2 },
}

impl GameplayEffect {
    pub fn switch_activation(&self) -> Option<(&str, ae::Vec2)> {
        match self {
            Self::ActivateSwitch { payload, pos } => Some((payload.as_str(), *pos)),
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
    /// Typed gameplay effects for progression, persistence, switch routing,
    /// boss routing, and standalone audio.
    pub effects: Vec<GameplayEffect>,
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
        self.effects.append(&mut other.effects);
        self.chests_opened.append(&mut other.chests_opened);
        self.pickups_collected.append(&mut other.pickups_collected);
        self.breakables_destroyed
            .append(&mut other.breakables_destroyed);
    }

    pub fn push_effect(&mut self, effect: GameplayEffect) {
        self.effects.push(effect);
    }

    pub fn set_flag(&mut self, id: impl Into<String>, on: bool) {
        self.push_effect(GameplayEffect::SetFlag { id: id.into(), on });
    }

    pub fn advance_quest(&mut self, event: ae::QuestAdvanceEvent) {
        self.push_effect(GameplayEffect::AdvanceQuest(event));
    }

    pub fn activate_switch(&mut self, payload: impl Into<String>, pos: ae::Vec2) {
        self.push_effect(GameplayEffect::ActivateSwitch {
            payload: payload.into(),
            pos,
        });
    }

    pub fn damage_boss(&mut self, boss_id: impl Into<String>, amount: i32) {
        self.push_effect(GameplayEffect::DamageBoss {
            boss_id: boss_id.into(),
            amount,
        });
    }

    pub fn strike_npc(&mut self, npc_id: impl Into<String>, pos: ae::Vec2) {
        self.push_effect(GameplayEffect::StrikeNpc {
            npc_id: npc_id.into(),
            pos,
        });
    }

    /// Enqueue a one-shot SFX at a position. Cheap helper for sim-side
    /// code that wants to play a sound without adding a dedicated
    /// typed presentation vector. Drained by `handle_feature_events` into
    /// `SfxMessage::Play`.
    pub fn play_sfx(&mut self, id: ambition_sfx::SfxId, pos: ae::Vec2) {
        self.push_effect(GameplayEffect::PlaySfx { id, pos });
    }

    pub fn switch_activations(&self) -> impl Iterator<Item = (&str, ae::Vec2)> + '_ {
        self.effects.iter().filter_map(GameplayEffect::switch_activation)
    }

    pub fn sfx_plays(&self) -> impl Iterator<Item = (ambition_sfx::SfxId, ae::Vec2)> + '_ {
        self.effects.iter().filter_map(GameplayEffect::sfx_play)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gameplay_effect_table_carries_cross_system_side_effects() {
        let mut events = FeatureEvents::default();
        events.set_flag("met_any_hub_npc", true);
        events.advance_quest(ae::QuestAdvanceEvent::NpcTalked("guide".into()));
        events.activate_switch("switch:mob_lab", ae::Vec2::new(1.0, 2.0));
        events.damage_boss("mockingbird", 3);
        events.strike_npc("pirate_admiral", ae::Vec2::new(5.0, 6.0));
        events.play_sfx(ambition_sfx::ids::PLAYER_DAMAGE, ae::Vec2::new(7.0, 8.0));

        assert!(matches!(
            events.effects[0],
            GameplayEffect::SetFlag { ref id, on: true } if id == "met_any_hub_npc"
        ));
        assert_eq!(
            events.switch_activations().collect::<Vec<_>>(),
            vec![("switch:mob_lab", ae::Vec2::new(1.0, 2.0))]
        );
        assert_eq!(
            events.sfx_plays().collect::<Vec<_>>(),
            vec![(ambition_sfx::ids::PLAYER_DAMAGE, ae::Vec2::new(7.0, 8.0))]
        );
    }

    #[test]
    fn merging_feature_events_preserves_effect_order() {
        let mut first = FeatureEvents::default();
        first.set_flag("first", true);
        let mut second = FeatureEvents::default();
        second.set_flag("second", true);

        first.merge(second);

        let ids = first
            .effects
            .iter()
            .filter_map(|effect| match effect {
                GameplayEffect::SetFlag { id, .. } => Some(id.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["first", "second"]);
    }
}
