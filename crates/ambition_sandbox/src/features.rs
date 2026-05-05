//! Runtime feature probes for the basement sandbox rooms.
//!
//! The engine owns the reusable data vocabulary. This module is deliberately a
//! sandbox-side adapter: it turns authored `World::objects` into a small playable
//! proving ground for hazards, enemies, bosses, breakables, pickups, chests, and
//! NPC interactions without committing final production behavior yet.

use ambition_engine as ae;
use ambition_engine::AabbExt;
use bevy::prelude::*;

use crate::platforms::MovingPlatformState;

const ENEMY_GRAVITY: f32 = 1450.0;
const ENEMY_MAX_FALL: f32 = 760.0;
const ENEMY_PATROL_SPEED: f32 = 105.0;
const ENEMY_CHASE_SPEED: f32 = 155.0;
const ENEMY_ATTACK_RANGE: f32 = 150.0;
const ENEMY_ATTACK_COOLDOWN: f32 = 1.05;
const BOSS_ATTACK_COOLDOWN: f32 = 1.35;
const BREAK_ON_STAND_SECONDS: f32 = 0.85;

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
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpcDialogueRequest {
    pub npc_id: String,
    pub npc_name: String,
    pub dialogue_id: String,
}

#[derive(Clone, Debug)]
pub struct FeatureRuntime {
    pub hazards: Vec<HazardRuntime>,
    pub enemies: Vec<EnemyRuntime>,
    pub bosses: Vec<BossRuntime>,
    pub breakables: Vec<BreakableRuntime>,
    pub pickups: Vec<PickupRuntime>,
    pub chests: Vec<ChestRuntime>,
    pub npcs: Vec<NpcRuntime>,
    pub switches: Vec<SwitchRuntime>,
    pub water_volumes: Vec<WaterVolumeRuntime>,
    pub banner: String,
    pub banner_timer: f32,
}

/// Live state for one water volume in the active room. Position +
/// AABB are stable; the runtime exists as a layer over the engine
/// spec so the swim system can query "is the player submerged"
/// without iterating `World::objects` every frame.
#[derive(Clone, Debug)]
pub struct WaterVolumeRuntime {
    pub id: String,
    pub aabb: ae::Aabb,
    pub spec: ae::WaterVolumeSpec,
}

/// Runtime state of a `Switch` interactable. The custom payload comes
/// from the LDtk `Switch` entity via `entity_to_runtime`; the
/// encounter system parses it on activation.
#[derive(Clone, Debug)]
pub struct SwitchRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub interactable: ae::Interactable,
    /// The `Custom("switch:...")` payload string. Cached here so the
    /// activation event doesn't have to re-pattern-match `kind`.
    pub custom_payload: String,
    /// Live on/off state for color rendering. The encounter system
    /// keeps this in sync with the persisted save state + the live
    /// encounter phase: `on = true` means the encounter is `Cleared`
    /// or has been disabled by the user; `on = false` means the
    /// encounter is armed (will fire when the player enters).
    pub on: bool,
}

impl SwitchRuntime {
    fn new(object: &ae::RoomObject, interactable: ae::Interactable, payload: String) -> Self {
        Self {
            id: object.id.clone(),
            name: object.name.clone(),
            pos: object.aabb.center(),
            size: object.aabb.half_size() * 2.0,
            interactable,
            custom_payload: payload,
            on: false,
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }
}

impl FeatureRuntime {
    /// Idempotently sync save-derived state onto the live runtime.
    /// Called every frame so a freshly-loaded room reflects boss
    /// defeats, NPC hostility, etc. Cheap (linear in feature counts).
    pub fn apply_save(&mut self, save: &ae::SandboxSaveData) {
        // Convert any NPC the save remembers as hostile into a real
        // enemy. Hostility is a one-way trip — once flipped, the NPC
        // is replaced by an `EnemyRuntime` that uses the same chase
        // / attack AI as authored enemies. Reusing the enemy
        // pipeline keeps the AI in one place and means hostile NPCs
        // automatically get patrol/aggro/attack telegraph behavior
        // for free.
        let mut to_convert: Vec<usize> = Vec::new();
        for (idx, npc) in self.npcs.iter_mut().enumerate() {
            let flag_id = npc.flag_id();
            let flagged = save.flag(&flag_id);
            if flagged || npc.hostile {
                npc.hostile = true;
                to_convert.push(idx);
            }
        }
        for idx in to_convert.into_iter().rev() {
            let npc = self.npcs.remove(idx);
            // If we already killed this hostile NPC in a prior
            // session, leave them dead — don't re-spawn the enemy.
            if save.flag(&format!("enemy_{}_dead", npc.id)) {
                continue;
            }
            // Spawn a striker-style enemy with the same id so any
            // quest hooks / save flags keyed on the NPC id still
            // resolve. The enemy id mirrors the NPC id so the
            // replacement is idempotent.
            self.spawn_enemy(
                npc.id.clone(),
                ae::EnemyBrain::Custom("medium_striker".into()),
                npc.pos,
                ae::Vec2::new(npc.size.x.max(22.0), npc.size.y.max(38.0)),
            );
            self.banner = format!("{} attacks!", npc.name);
            self.banner_timer = 1.5;
        }
        // Authored enemies (LDtk EnemySpawn) that were killed and
        // recorded in the save should also stay dead.
        for enemy in &mut self.enemies {
            if save.flag(&format!("enemy_{}_dead", enemy.id))
                && !enemy.id.starts_with("encounter:")
            {
                enemy.alive = false;
                enemy.health.current = 0;
            }
        }
        // Boss defeats: hide already-cleared bosses by marking the
        // runtime dead. New `BossSpawn` instances from the LDtk
        // file all start `alive=true`, so this is the gate.
        for boss in &mut self.bosses {
            if matches!(
                save.boss(&boss.id),
                ae::PersistedEncounterState::Cleared
            ) {
                boss.alive = false;
                boss.health.current = 0;
            }
        }
    }

    /// Set the on/off rendering state for a named switch (no-op if the
    /// id doesn't exist). The encounter system calls this whenever
    /// the persisted switch state or live encounter phase changes.
    pub fn set_switch_on(&mut self, id: &str, on: bool) {
        if let Some(switch) = self.switches.iter_mut().find(|s| s.id == id) {
            switch.on = on;
        }
    }

    /// Spawn a fresh enemy at runtime. Used by the encounter system's
    /// wave loop to introduce mobs after the camera + music intro
    /// rather than placing them as static LDtk EnemySpawn entities.
    /// Bypasses the patrol-path lookup since encounter mobs use chase
    /// AI by default.
    pub fn spawn_enemy(
        &mut self,
        id: String,
        brain: ae::EnemyBrain,
        pos: ae::Vec2,
        size: ae::Vec2,
    ) {
        let archetype = EnemyArchetype::from_brain(&brain);
        let aabb = ae::Aabb::new(pos, size * 0.5);
        let object = ae::RoomObject::new(
            id.clone(),
            id.clone(),
            aabb,
            ae::RoomObjectKind::EnemySpawn(brain.clone()),
        );
        let mut runtime = EnemyRuntime::new(&object, brain, &[]);
        runtime.archetype = archetype;
        runtime.health = ae::Health::new(archetype.max_health());
        // Encounter spawns shouldn't auto-respawn even if they happen
        // to be a sandbag archetype — set respawn timer to a value
        // longer than any reasonable encounter so the wave clears.
        runtime.respawn_timer = 999_999.0;
        self.enemies.push(runtime);
    }

    /// Remove all enemies whose id starts with `encounter:<id>:` —
    /// called when the encounter is reset via the switch so a fresh
    /// attempt doesn't inherit half-dead carryover mobs from the
    /// previous attempt.
    pub fn despawn_encounter_enemies(&mut self, encounter_id: &str) {
        let prefix = format!("encounter:{encounter_id}:");
        self.enemies.retain(|e| !e.id.starts_with(&prefix));
    }

    /// Remove the encounter-spawned chest, if any. The encounter
    /// system uses a fixed `encounter_chest_<id>` id when it drops the
    /// victory chest, so the matching chest is the one to drop on
    /// switch reset. Authored chests (different ids) are untouched.
    pub fn despawn_encounter_chest(&mut self, encounter_id: &str) {
        let target = format!("encounter_chest_{encounter_id}");
        self.chests.retain(|c| c.id != target);
    }

    /// Spawn a chest at runtime. Used by the encounter system to drop
    /// a victory reward when an arena clears, and available as a
    /// general utility for code-built rooms / cutscenes.
    pub fn spawn_chest(
        &mut self,
        id: String,
        reward: Option<ae::PickupKind>,
        pos: ae::Vec2,
        size: ae::Vec2,
    ) {
        let aabb = ae::Aabb::new(pos, size * 0.5);
        let object = ae::RoomObject::new(
            id.clone(),
            id.clone(),
            aabb,
            ae::RoomObjectKind::Chest(ae::Chest::new(id.clone(), reward)),
        );
        let chest = match object.kind.clone() {
            ae::RoomObjectKind::Chest(c) => c,
            _ => unreachable!(),
        };
        // Don't double-spawn — if a chest with the same id already
        // exists (e.g. authored chest in the room), leave it alone.
        if self.chests.iter().any(|c| c.id == id) {
            return;
        }
        self.chests.push(ChestRuntime::new(&object, chest));
    }
}

impl FeatureRuntime {
    pub fn from_world(world: &ae::World) -> Self {
        let paths = room_paths(world);
        let mut runtime = Self {
            hazards: Vec::new(),
            enemies: Vec::new(),
            bosses: Vec::new(),
            breakables: Vec::new(),
            pickups: Vec::new(),
            chests: Vec::new(),
            npcs: Vec::new(),
            switches: Vec::new(),
            water_volumes: Vec::new(),
            banner: String::new(),
            banner_timer: 0.0,
        };

        for object in &world.objects {
            match &object.kind {
                ae::RoomObjectKind::DamageVolume(volume) => {
                    runtime
                        .hazards
                        .push(HazardRuntime::new(object, volume.clone()));
                }
                ae::RoomObjectKind::Pickup(pickup) => {
                    runtime
                        .pickups
                        .push(PickupRuntime::new(object, pickup.clone()));
                }
                ae::RoomObjectKind::Chest(chest) => {
                    runtime
                        .chests
                        .push(ChestRuntime::new(object, chest.clone()));
                }
                ae::RoomObjectKind::Breakable(breakable) => {
                    runtime
                        .breakables
                        .push(BreakableRuntime::new(object, breakable.clone()));
                }
                ae::RoomObjectKind::Interactable(interactable) => {
                    if matches!(interactable.kind, ae::InteractionKind::Npc { .. }) {
                        runtime
                            .npcs
                            .push(NpcRuntime::new(object, interactable.clone()));
                    } else if let ae::InteractionKind::Custom(payload) = &interactable.kind {
                        if payload.starts_with("switch:") {
                            runtime.switches.push(SwitchRuntime::new(
                                object,
                                interactable.clone(),
                                payload.clone(),
                            ));
                        }
                    }
                }
                ae::RoomObjectKind::EnemySpawn(brain) => {
                    runtime
                        .enemies
                        .push(EnemyRuntime::new(object, brain.clone(), &paths));
                }
                ae::RoomObjectKind::BossSpawn(brain) => {
                    runtime.bosses.push(BossRuntime::new(object, brain.clone()));
                }
                ae::RoomObjectKind::WaterVolume(spec) => {
                    runtime.water_volumes.push(WaterVolumeRuntime {
                        id: object.id.clone(),
                        aabb: object.aabb,
                        spec: *spec,
                    });
                }
                ae::RoomObjectKind::Actor(_)
                | ae::RoomObjectKind::KinematicPath(_)
                | ae::RoomObjectKind::DebugLabel(_)
                | ae::RoomObjectKind::DestinationLabel(_) => {}
            }
        }
        runtime
    }

    pub fn update(
        &mut self,
        world: &ae::World,
        player: &ae::Player,
        interact_pressed: bool,
        player_vulnerable: bool,
        tuning: FeatureCombatTuning,
        dt: f32,
    ) -> FeatureEvents {
        let mut events = FeatureEvents::default();
        self.banner_timer = (self.banner_timer - dt).max(0.0);
        if self.banner_timer <= 0.0 {
            self.banner.clear();
        }

        let player_body = player.aabb();

        for hazard in &mut self.hazards {
            hazard.update(dt);
            if player_vulnerable && hazard.active() && hazard.aabb().strict_intersects(player_body)
            {
                events
                    .messages
                    .push(format!("{} forced a safe respawn", hazard.name));
                events.impacts.push(player.pos);
                events.player_damage.push(PlayerDamageEvent {
                    mode: PlayerDamageMode::SafeRespawn,
                    source: PlayerDamageSource::Hazard,
                    source_pos: hazard.pos,
                    impact_pos: player.pos,
                    knockback_dir: 0.0,
                    strength: 1.0,
                    amount: hazard.volume.damage.amount.max(1),
                });
            }
        }

        for breakable in &mut self.breakables {
            if breakable.broken() {
                breakable.stand_timer = 0.0;
                if let ae::RespawnPolicy::AfterSeconds(_) = breakable.breakable.respawn {
                    breakable.respawn_timer = (breakable.respawn_timer - dt).max(0.0);
                    if breakable.respawn_timer <= 0.0 {
                        breakable.breakable.state = ae::BreakableState::Intact;
                        breakable.breakable.health.reset();
                        events
                            .messages
                            .push(format!("{} respawned", breakable.name));
                        events.bursts.push(breakable.pos);
                    }
                }
                continue;
            }

            if breakable.breaks_on_stand() && player_is_standing_on(player_body, breakable.aabb()) {
                breakable.stand_timer += dt;
                if breakable.stand_timer >= BREAK_ON_STAND_SECONDS {
                    let broke = breakable
                        .breakable
                        .apply_damage(breakable.breakable.health.current.max(1));
                    if broke {
                        breakable.start_respawn_timer();
                        events
                            .messages
                            .push(format!("{} collapsed under weight", breakable.name));
                        events.bursts.push(breakable.pos);
                        events.physics_bursts.push(FeaturePhysicsBurst {
                            pos: breakable.pos,
                            cue: FeaturePhysicsCue::Breakable,
                        });
                    }
                }
            } else {
                breakable.stand_timer = (breakable.stand_timer - dt * 2.0).max(0.0);
            }
        }

        for pickup in &mut self.pickups {
            if pickup.visible && pickup.aabb().strict_intersects(player_body) {
                pickup.visible = false;
                events.messages.push(format!("picked up {}", pickup.name));
                if let ae::PickupKind::Health { amount } = pickup.pickup.kind {
                    events.player_heal += amount;
                }
                events.bursts.push(pickup.pos);
            }
        }

        if interact_pressed {
            for chest in &mut self.chests {
                if !chest.opened && chest.aabb().strict_intersects(player_body) {
                    chest.opened = true;
                    events.consumed_interaction = true;
                    events.messages.push(format!("opened {}", chest.name));
                    events.bursts.push(chest.pos);
                    // Persist the looted state so save+reload re-spawns
                    // the chest in its opened state. Encounter chests
                    // are keyed `encounter_chest_<id>`; the matching
                    // looted flag is `encounter_<id>_reward_dropped`
                    // (`crate::encounter::encounter_reward_looted_flag`).
                    if let Some(encounter_id) = chest.id.strip_prefix("encounter_chest_") {
                        events.flag_writes.push((
                            format!("encounter_{encounter_id}_reward_dropped"),
                            true,
                        ));
                    }
                }
            }
            for npc in &mut self.npcs {
                if npc.aabb().strict_intersects(player_body) {
                    events.consumed_interaction = true;
                    events.messages.push(npc.message());
                    if !npc.hostile {
                        events.dialogue_request = Some(npc.dialogue_request());
                        // Quest hook: "talked to NPC" + a generic
                        // "met any hub NPC" flag the tutorial quest
                        // listens for.
                        events
                            .quest_advance
                            .push(ae::QuestAdvanceEvent::NpcTalked(npc.id.clone()));
                        events
                            .flag_writes
                            .push(("met_any_hub_npc".into(), true));
                    }
                    events.bursts.push(npc.pos);
                }
            }
            for switch in &mut self.switches {
                if switch.aabb().strict_intersects(player_body) {
                    events.consumed_interaction = true;
                    events.messages.push(format!("activated {}", switch.name));
                    events
                        .switch_activations
                        .push(switch.custom_payload.clone());
                    events.bursts.push(switch.pos);
                }
            }
        }

        for enemy in &mut self.enemies {
            enemy.update(world, player, tuning, dt);
            if player_vulnerable && enemy.alive {
                if let Some(damage) = enemy.player_damage(player_body) {
                    events
                        .messages
                        .push(format!("{} hit the player", enemy.name));
                    events.impacts.push(damage.impact_pos);
                    events.player_damage.push(damage);
                }
            }
        }

        for boss in &mut self.bosses {
            boss.update(world, player, tuning, dt);
            if player_vulnerable && boss.alive {
                if let Some(damage) = boss.player_damage(player_body) {
                    events
                        .messages
                        .push(format!("{} pattern hit the player", boss.name));
                    events.impacts.push(damage.impact_pos);
                    events.player_damage.push(damage);
                }
            }
        }

        // Hostile NPCs are converted to `EnemyRuntime` instances by
        // `apply_save`, so peaceful NPCs only need a hit-flash decay
        // here — combat damage flows through the enemy path.
        for npc in &mut self.npcs {
            npc.hit_flash = (npc.hit_flash - dt).max(0.0);
        }
        let _ = player_vulnerable;

        self.accept_events(&events);
        events
    }

    pub fn apply_player_attack(
        &mut self,
        attack: ae::Aabb,
        damage: i32,
        knock_x: f32,
    ) -> FeatureEvents {
        let mut events = FeatureEvents::default();

        for enemy in &mut self.enemies {
            if enemy.alive && attack.strict_intersects(enemy.aabb()) {
                enemy.hit_flash = 0.16;
                enemy.vel.x += knock_x;
                enemy.vel.y = (enemy.vel.y - 90.0).max(-280.0);
                let killed = if enemy.archetype == EnemyArchetype::InfiniteSandbag {
                    false
                } else {
                    enemy.health.damage(damage)
                };
                let hit_pos = midpoint(attack.center(), enemy.pos);
                events.impacts.push(hit_pos);
                if killed {
                    enemy.alive = false;
                    if enemy.archetype == EnemyArchetype::FiniteSandbag {
                        enemy.respawn_timer = 0.85;
                        events
                            .messages
                            .push(format!("{} dropped; respawning", enemy.name));
                    } else {
                        events.messages.push(format!("defeated {}", enemy.name));
                        // Persist non-respawning enemy deaths so the
                        // room load doesn't bring them back. Encounter
                        // mobs (id prefix "encounter:") skip this —
                        // their lifecycle is owned by the encounter
                        // state machine, which will re-spawn them on
                        // re-trigger.
                        if !enemy.id.starts_with("encounter:")
                            && enemy.archetype != EnemyArchetype::InfiniteSandbag
                            && enemy.archetype != EnemyArchetype::FiniteSandbag
                        {
                            events
                                .flag_writes
                                .push((format!("enemy_{}_dead", enemy.id), true));
                        }
                    }
                    events.bursts.push(enemy.pos);
                    events.physics_bursts.push(FeaturePhysicsBurst {
                        pos: enemy.pos,
                        cue: FeaturePhysicsCue::EnemyRagdoll,
                    });
                }
            }
        }

        for boss in &mut self.bosses {
            if boss.alive && attack.strict_intersects(boss.aabb()) {
                boss.hit_flash = 0.18;
                let amount = damage.max(1);
                let killed = boss.health.damage(amount);
                events.impacts.push(midpoint(attack.center(), boss.pos));
                events.boss_damage.push((boss.id.clone(), amount));
                if killed {
                    boss.alive = false;
                    events.messages.push(format!("defeated boss {}", boss.name));
                    events.bursts.push(boss.pos);
                    events.physics_bursts.push(FeaturePhysicsBurst {
                        pos: boss.pos,
                        cue: FeaturePhysicsCue::BossRagdoll,
                    });
                }
            }
        }

        // NPC strikes — non-hostile NPCs accumulate hits; once they
        // cross `NPC_HOSTILE_STRIKE_THRESHOLD` they flip hostile and
        // begin acting like a striker enemy. Already-hostile NPCs
        // take real damage like any other enemy. The save flag write
        // (`npc_<id>_hostile`) is queued via `flag_writes` so the
        // sandbox runtime persists it.
        for npc in &mut self.npcs {
            if !attack.strict_intersects(npc.aabb()) {
                continue;
            }
            npc.hit_flash = 0.18;
            events.impacts.push(midpoint(attack.center(), npc.pos));
            events.npc_struck.push((npc.id.clone(), npc.pos));
            if npc.hostile {
                // Hostile NPC takes a damage tick; if it dies, fall
                // through and let the death cleanup happen on the
                // next room enter.
                npc.strikes = npc.strikes.saturating_add(1);
                if npc.strikes >= NPC_HOSTILE_STRIKE_THRESHOLD * 2 {
                    events
                        .messages
                        .push(format!("{} flees the room", npc.name));
                }
            } else {
                npc.strikes = npc.strikes.saturating_add(1);
                if npc.strikes >= NPC_HOSTILE_STRIKE_THRESHOLD {
                    npc.hostile = true;
                    events.messages.push(format!("{} turns hostile", npc.name));
                    events.flag_writes.push((npc.flag_id(), true));
                    events.bursts.push(npc.pos);
                }
            }
        }

        for breakable in &mut self.breakables {
            // Breakable pogo orbs take damage exclusively through the pogo
            // bounce path (`on_pogo_bounce`). Slashing or pogoing onto one
            // would otherwise apply two damage in a single frame — once
            // here via the slash hitbox and once via the pogo callback —
            // making a 3hp orb die in 2 bounces.
            if breakable.breakable.pogo_refresh {
                continue;
            }
            if !breakable.broken()
                && breakable.breaks_on_hit()
                && attack.strict_intersects(breakable.aabb())
            {
                let broke = breakable.breakable.apply_damage(damage.max(1));
                events
                    .impacts
                    .push(midpoint(attack.center(), breakable.pos));
                if broke {
                    breakable.start_respawn_timer();
                    events.messages.push(format!("broke {}", breakable.name));
                    events.bursts.push(breakable.pos);
                    events.physics_bursts.push(FeaturePhysicsBurst {
                        pos: breakable.pos,
                        cue: FeaturePhysicsCue::Breakable,
                    });
                }
            }
        }

        self.accept_events(&events);
        events
    }

    /// Apply pogo-bounce damage to any breakable pogo orb whose runtime
    /// AABB matches `orb_aabb` (engine-reported bounce source). Returns a
    /// `FeatureEvents` describing impacts/messages/physics so the caller
    /// can route them through the same audio/VFX/debris pipeline that
    /// player-attack hits use.
    pub fn on_pogo_bounce(&mut self, orb_aabb: ae::Aabb, damage: i32) -> FeatureEvents {
        let mut events = FeatureEvents::default();
        for breakable in &mut self.breakables {
            if breakable.broken() {
                continue;
            }
            if !breakable.breakable.pogo_refresh {
                continue;
            }
            if !approximately_same_aabb(breakable.aabb(), orb_aabb) {
                continue;
            }
            let broke = breakable.breakable.apply_damage(damage.max(1));
            events.impacts.push(breakable.pos);
            if broke {
                breakable.start_respawn_timer();
                events
                    .messages
                    .push(format!("shattered {}", breakable.name));
                events.bursts.push(breakable.pos);
                events.physics_bursts.push(FeaturePhysicsBurst {
                    pos: breakable.pos,
                    cue: FeaturePhysicsCue::Breakable,
                });
            }
        }
        events
    }

    pub fn view(&self, id: &str) -> Option<FeatureView> {
        for hazard in &self.hazards {
            if hazard.id == id {
                return Some(FeatureView {
                    pos: hazard.pos,
                    size: hazard.size,
                    kind: FeatureVisualKind::Hazard,
                    visible: hazard.active(),
                    flash: false,
                    switch_on: false,
                });
            }
        }
        for enemy in &self.enemies {
            if enemy.id == id {
                return Some(FeatureView {
                    pos: enemy.pos,
                    size: enemy.size,
                    kind: enemy.visual_kind(),
                    visible: enemy.alive,
                    flash: enemy.hit_flash > 0.0
                        || enemy.attack_windup_timer > 0.0
                        || enemy.attack_timer > 0.0,
                    switch_on: false,
                });
            }
        }
        for boss in &self.bosses {
            if boss.id == id {
                return Some(FeatureView {
                    pos: boss.pos,
                    size: boss.size,
                    kind: FeatureVisualKind::Boss,
                    visible: boss.alive,
                    flash: boss.hit_flash > 0.0
                        || boss.attack_windup_timer > 0.0
                        || boss.attack_timer > 0.0,
                    switch_on: false,
                });
            }
        }
        for breakable in &self.breakables {
            if breakable.id == id {
                return Some(FeatureView {
                    pos: breakable.pos,
                    size: breakable.size,
                    kind: FeatureVisualKind::Breakable,
                    visible: !breakable.broken(),
                    flash: breakable.breakable.state == ae::BreakableState::Cracking,
                    switch_on: false,
                });
            }
        }
        for pickup in &self.pickups {
            if pickup.id == id {
                return Some(FeatureView {
                    pos: pickup.pos,
                    size: pickup.size,
                    kind: FeatureVisualKind::Pickup,
                    visible: pickup.visible,
                    flash: false,
                    switch_on: false,
                });
            }
        }
        for chest in &self.chests {
            if chest.id == id {
                return Some(FeatureView {
                    pos: chest.pos,
                    size: chest.size,
                    kind: FeatureVisualKind::Chest,
                    visible: true,
                    flash: chest.opened,
                    switch_on: false,
                });
            }
        }
        for npc in &self.npcs {
            if npc.id == id {
                return Some(FeatureView {
                    pos: npc.pos,
                    size: npc.size,
                    kind: FeatureVisualKind::Npc,
                    visible: true,
                    flash: false,
                    switch_on: false,
                });
            }
        }
        for switch in &self.switches {
            if switch.id == id {
                return Some(FeatureView {
                    pos: switch.pos,
                    size: switch.size,
                    kind: FeatureVisualKind::Switch,
                    visible: true,
                    flash: false,
                    switch_on: switch.on,
                });
            }
        }
        None
    }

    /// Snapshot the enemy state needed to drive its sprite animation.
    /// Returns `None` if no enemy with that id exists.
    pub fn enemy_anim_state(&self, id: &str) -> Option<crate::character_sprites::EnemyAnimState> {
        for enemy in &self.enemies {
            if enemy.id == id {
                return Some(crate::character_sprites::EnemyAnimState {
                    vel: enemy.vel,
                    facing: enemy.facing,
                    alive: enemy.alive,
                    attack_active: enemy.attack_timer > 0.0,
                    attack_windup: enemy.attack_windup_timer > 0.0,
                    hit_flash: enemy.hit_flash > 0.0,
                });
            }
        }
        None
    }

    /// Look up a breakable's current state by feature id (LDtk iid).
    pub fn breakable_state(&self, id: &str) -> Option<ae::BreakableState> {
        self.breakables
            .iter()
            .find(|b| b.id == id)
            .map(|b| b.breakable.state)
    }

    /// Look up a chest's opened-flag by feature id.
    pub fn chest_opened(&self, id: &str) -> Option<bool> {
        self.chests.iter().find(|c| c.id == id).map(|c| c.opened)
    }

    /// Snapshot the boss state used to drive its spritesheet animation.
    pub fn boss_anim_state(&self, id: &str) -> Option<crate::boss_sprites::BossAnimState> {
        self.bosses
            .iter()
            .find(|b| b.id == id)
            .map(|b| crate::boss_sprites::BossAnimState {
                alive: b.alive,
                attack_active: b.attack_timer > 0.0,
                attack_windup: b.attack_windup_timer > 0.0,
                hit_flash: b.hit_flash > 0.0,
                pattern_timer: b.pattern_timer,
            })
    }

    pub fn feature_summary(&self) -> String {
        format!(
            "features: hazards {} enemies {}/{} bosses {}/{} breakables {}/{} chests {}/{} pickups {}/{} npcs {}",
            self.hazards.len(),
            self.enemies.iter().filter(|e| e.alive).count(),
            self.enemies.len(),
            self.bosses.iter().filter(|b| b.alive).count(),
            self.bosses.len(),
            self.breakables.iter().filter(|b| !b.broken()).count(),
            self.breakables.len(),
            self.chests.iter().filter(|c| c.opened).count(),
            self.chests.len(),
            self.pickups.iter().filter(|p| p.visible).count(),
            self.pickups.len(),
            self.npcs.len(),
        )
    }

    fn accept_events(&mut self, events: &FeatureEvents) {
        if let Some(message) = events.messages.last() {
            self.banner = message.clone();
            self.banner_timer = 2.6;
        }
    }
}

#[derive(Clone, Debug)]
pub struct HazardRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub volume: ae::DamageVolume,
    pub motion: Option<PathMotion>,
}

impl HazardRuntime {
    fn new(object: &ae::RoomObject, volume: ae::DamageVolume) -> Self {
        Self {
            id: object.id.clone(),
            name: object.name.clone(),
            pos: object.aabb.center(),
            size: object.aabb.half_size() * 2.0,
            motion: volume.motion.clone().map(PathMotion::new),
            volume,
        }
    }

    fn update(&mut self, dt: f32) {
        if let Some(motion) = &mut self.motion {
            self.pos = motion.advance(self.pos, dt);
        }
    }

    pub fn active(&self) -> bool {
        self.volume.enabled
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }
}

#[derive(Clone, Debug)]
pub struct EnemyRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub spawn: ae::Vec2,
    pub size: ae::Vec2,
    pub vel: ae::Vec2,
    pub health: ae::Health,
    pub brain: ae::EnemyBrain,
    pub archetype: EnemyArchetype,
    pub motion: Option<PathMotion>,
    pub alive: bool,
    pub facing: f32,
    pub attack_windup_timer: f32,
    pub attack_timer: f32,
    pub attack_cooldown: f32,
    pub respawn_timer: f32,
    pub hit_flash: f32,
    /// Last-evaluated `CharacterAiMode`. Updated by `update`. Read by
    /// HUD / rendering / debug overlay so they can branch on a single
    /// vocabulary instead of inferring it from the timer fields.
    pub ai_mode: ae::CharacterAiMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyArchetype {
    Combatant,
    SmallSkitter,
    MediumStriker,
    LargeBrute,
    AggressiveSeeker,
    InfiniteSandbag,
    FiniteSandbag,
}

impl EnemyArchetype {
    fn from_brain(brain: &ae::EnemyBrain) -> Self {
        match brain {
            ae::EnemyBrain::Custom(name) if name == "small_skitter" => Self::SmallSkitter,
            ae::EnemyBrain::Custom(name) if name == "medium_striker" => Self::MediumStriker,
            ae::EnemyBrain::Custom(name) if name == "large_brute" => Self::LargeBrute,
            ae::EnemyBrain::Custom(name) if name == "gradient_seeker" => Self::AggressiveSeeker,
            ae::EnemyBrain::Custom(name) if name == "sandbag_infinite" => Self::InfiniteSandbag,
            ae::EnemyBrain::Custom(name) if name == "sandbag_finite" => Self::FiniteSandbag,
            _ => Self::Combatant,
        }
    }

    pub(crate) fn is_sandbag(self) -> bool {
        matches!(self, Self::InfiniteSandbag | Self::FiniteSandbag)
    }

    fn max_health(self) -> i32 {
        match self {
            Self::SmallSkitter => 2,
            Self::Combatant | Self::AggressiveSeeker => 4,
            Self::MediumStriker => 5,
            Self::LargeBrute => 9,
            Self::InfiniteSandbag => 9999,
            Self::FiniteSandbag => 6,
        }
    }

    fn patrol_speed(self) -> f32 {
        match self {
            Self::SmallSkitter => 150.0,
            Self::LargeBrute => 72.0,
            Self::AggressiveSeeker => 130.0,
            _ => ENEMY_PATROL_SPEED,
        }
    }

    fn chase_speed(self) -> f32 {
        match self {
            Self::SmallSkitter => 210.0,
            Self::LargeBrute => 118.0,
            Self::AggressiveSeeker => 225.0,
            Self::MediumStriker => 170.0,
            _ => ENEMY_CHASE_SPEED,
        }
    }

    fn aggro_radius(self) -> f32 {
        match self {
            Self::SmallSkitter => 320.0,
            Self::MediumStriker | Self::Combatant => 460.0,
            Self::LargeBrute => 380.0,
            Self::AggressiveSeeker => 900.0,
            Self::InfiniteSandbag | Self::FiniteSandbag => 0.0,
        }
    }

    fn attack_range(self) -> f32 {
        match self {
            Self::SmallSkitter => 105.0,
            Self::LargeBrute => 205.0,
            _ => ENEMY_ATTACK_RANGE,
        }
    }

    fn contact_strength(self) -> f32 {
        match self {
            Self::SmallSkitter => 0.55,
            Self::LargeBrute => 1.25,
            Self::AggressiveSeeker => 0.80,
            _ => 0.70,
        }
    }

    fn damage_amount(self) -> i32 {
        match self {
            Self::LargeBrute => 2,
            _ => 1,
        }
    }
}

impl EnemyRuntime {
    fn new(
        object: &ae::RoomObject,
        brain: ae::EnemyBrain,
        paths: &[(String, ae::KinematicPath)],
    ) -> Self {
        let archetype = EnemyArchetype::from_brain(&brain);
        let motion = match &brain {
            ae::EnemyBrain::Patrol {
                path_id: Some(path_id),
            } if !archetype.is_sandbag() => paths
                .iter()
                .find(|(id, _)| id == path_id)
                .map(|(_, path)| PathMotion::new(path.clone())),
            _ => None,
        };
        Self {
            id: object.id.clone(),
            name: object.name.clone(),
            pos: object.aabb.center(),
            spawn: object.aabb.center(),
            size: object.aabb.half_size() * 2.0,
            vel: ae::Vec2::ZERO,
            health: ae::Health::new(archetype.max_health()),
            brain,
            archetype,
            motion,
            alive: true,
            facing: -1.0,
            attack_windup_timer: 0.0,
            attack_timer: 0.0,
            attack_cooldown: 0.2,
            respawn_timer: 0.0,
            hit_flash: 0.0,
            ai_mode: ae::CharacterAiMode::Idle,
        }
    }

    fn update(
        &mut self,
        world: &ae::World,
        player: &ae::Player,
        tuning: FeatureCombatTuning,
        dt: f32,
    ) {
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        if !self.alive {
            self.respawn_timer = (self.respawn_timer - dt).max(0.0);
            if self.archetype == EnemyArchetype::FiniteSandbag && self.respawn_timer <= 0.0 {
                self.alive = true;
                self.health.reset();
                self.pos = self.spawn;
                self.vel = ae::Vec2::ZERO;
                self.hit_flash = 0.24;
            }
            self.ai_mode = ae::CharacterAiMode::Dead;
            return;
        }
        let was_winding_up = self.attack_windup_timer > 0.0;
        self.attack_windup_timer = (self.attack_windup_timer - dt).max(0.0);
        self.attack_timer = (self.attack_timer - dt).max(0.0);
        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);
        if was_winding_up && self.attack_windup_timer <= 0.0 {
            self.attack_timer = tuning.enemy_attack_active.max(0.01);
        }

        // Evaluate the AI mode FIRST so motion / trigger logic can
        // branch on a single vocabulary. The recover_remaining
        // surrogate (using post-attack cooldown) signals that the
        // actor just finished an attack and shouldn't be moving
        // until the cooldown elapses — this is what makes Recover
        // legible to consumers.
        let recover_remaining = if self.attack_cooldown > 0.0
            && self.attack_windup_timer <= 0.0
            && self.attack_timer <= 0.0
        {
            self.attack_cooldown.min(0.30)
        } else {
            0.0
        };
        self.ai_mode = ae::evaluate_character_ai(ae::CharacterAiSnapshot {
            actor_pos: self.pos,
            player_pos: player.pos,
            aggro_radius: self.archetype.aggro_radius(),
            attack_range: self.archetype.attack_range(),
            attack_windup_remaining: self.attack_windup_timer,
            attack_active_remaining: self.attack_timer,
            attack_recover_remaining: recover_remaining,
            stun_remaining: 0.0,
            alive: self.alive,
            patrol_enabled: !self.archetype.is_sandbag()
                && !matches!(self.brain, ae::EnemyBrain::Passive),
        });
        // Whether the actor is committed to an attack frame this
        // tick — Telegraph / Attack / Recover all hold position.
        let committed = self.ai_mode.is_committed();

        if let Some(motion) = &mut self.motion {
            let old = self.pos;
            self.pos = motion.advance(self.pos, dt);
            self.facing = (self.pos.x - old.x).signum_or(self.facing);
        } else {
            let delta_to_player = player.pos - self.pos;
            let distance_to_player = delta_to_player.length();
            // Movement now reads `ai_mode`: Stunned/committed → hold,
            // Chase → pursue at chase_speed, Patrol → pace at
            // patrol_speed, Idle → stand. Sandbags + Passive bypass
            // the chase line entirely.
            let desired_x = if matches!(
                self.ai_mode,
                ae::CharacterAiMode::Stunned | ae::CharacterAiMode::Telegraph | ae::CharacterAiMode::Attack
            ) {
                0.0
            } else if committed {
                // Recover: drift to a stop.
                self.vel.x * 0.4
            } else if self.archetype.is_sandbag() {
                0.0
            } else {
                match self.brain {
                    ae::EnemyBrain::Guard { leash_radius }
                        if distance_to_player <= leash_radius =>
                    {
                        delta_to_player.x.signum() * self.archetype.chase_speed()
                    }
                    ae::EnemyBrain::Custom(_)
                        if distance_to_player <= self.archetype.aggro_radius() =>
                    {
                        delta_to_player.x.signum() * self.archetype.chase_speed()
                    }
                    ae::EnemyBrain::Passive => 0.0,
                    _ => match self.ai_mode {
                        ae::CharacterAiMode::Patrol => {
                            self.facing * self.archetype.patrol_speed()
                        }
                        ae::CharacterAiMode::Chase => {
                            delta_to_player.x.signum() * self.archetype.chase_speed()
                        }
                        _ => 0.0,
                    },
                }
            };
            self.vel.x = approach(self.vel.x, desired_x, 650.0 * dt);
            self.vel.y = (self.vel.y + ENEMY_GRAVITY * dt).min(ENEMY_MAX_FALL);
            let old_x = self.pos.x;
            self.pos.x += self.vel.x * dt;
            if blocked(world, self.aabb()) {
                self.pos.x = old_x;
                self.facing *= -1.0;
                self.vel.x = 0.0;
            }
            let old_y = self.pos.y;
            self.pos.y += self.vel.y * dt;
            if blocked_y(world, self.aabb()) {
                self.pos.y = old_y;
                self.vel.y = 0.0;
            }
        }

        let to_player = player.pos - self.pos;
        if to_player.x.abs() > 4.0 {
            self.facing = to_player.x.signum();
        }
        // Attack trigger: only fire from Chase. Telegraph/Attack/
        // Recover/Stunned/Idle/Patrol/Dead all skip — they each
        // mean "not the right tick to start a swing."
        if matches!(self.ai_mode, ae::CharacterAiMode::Chase)
            && !self.archetype.is_sandbag()
            && to_player.length() <= self.archetype.attack_range()
            && self.attack_cooldown <= 0.0
        {
            self.attack_windup_timer = tuning.enemy_attack_windup.max(0.01);
            self.attack_cooldown = ENEMY_ATTACK_COOLDOWN
                * if self.archetype == EnemyArchetype::SmallSkitter {
                    0.75
                } else if self.archetype == EnemyArchetype::LargeBrute {
                    1.35
                } else {
                    1.0
                };
            // Re-evaluate so the snapshot reflects "I just started
            // a windup" rather than holding stale Chase.
            self.ai_mode = ae::CharacterAiMode::Telegraph;
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }

    pub fn visual_kind(&self) -> FeatureVisualKind {
        if self.archetype.is_sandbag() {
            FeatureVisualKind::Sandbag
        } else {
            FeatureVisualKind::Enemy
        }
    }

    pub fn attack_aabb(&self) -> ae::Aabb {
        ae::Aabb::new(
            self.pos + ae::Vec2::new(self.facing * (self.size.x * 0.55 + 24.0), -4.0),
            ae::Vec2::new(34.0, 28.0),
        )
    }

    pub fn attack_telegraph_aabb(&self) -> ae::Aabb {
        self.attack_aabb()
    }

    /// Always-on body contact damage volume for normal enemies.
    ///
    /// Sandbags intentionally opt out: they are hit-confirm / tuning targets,
    /// not hostile actors. Their body AABB remains their player-attack hurtbox.
    pub fn body_damage_aabb(&self) -> Option<ae::Aabb> {
        if self.archetype.is_sandbag() {
            None
        } else {
            Some(self.aabb())
        }
    }

    fn player_damage(&self, player_body: ae::Aabb) -> Option<PlayerDamageEvent> {
        if self.attack_timer > 0.0 && self.attack_aabb().strict_intersects(player_body) {
            return Some(PlayerDamageEvent {
                mode: PlayerDamageMode::Knockback,
                source: PlayerDamageSource::EnemyAttack,
                source_pos: self.pos,
                impact_pos: midpoint(player_body.center(), self.attack_aabb().center()),
                knockback_dir: (player_body.center().x - self.pos.x).signum_or(self.facing),
                strength: 1.0,
                amount: 1,
            });
        }
        if let Some(body_damage) = self.body_damage_aabb() {
            if body_damage.strict_intersects(player_body) {
                return Some(PlayerDamageEvent {
                    mode: PlayerDamageMode::Knockback,
                    source: PlayerDamageSource::EnemyBody,
                    source_pos: self.pos,
                    impact_pos: midpoint(player_body.center(), body_damage.center()),
                    knockback_dir: (player_body.center().x - self.pos.x).signum_or(self.facing),
                    strength: self.archetype.contact_strength(),
                    amount: self.archetype.damage_amount(),
                });
            }
        }
        None
    }
}

#[derive(Clone, Debug)]
pub struct BossRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub spawn: ae::Vec2,
    pub size: ae::Vec2,
    pub health: ae::Health,
    pub brain: ae::BossBrain,
    pub alive: bool,
    pub pattern_timer: f32,
    pub movement_timer: f32,
    pub attack_windup_timer: f32,
    pub attack_timer: f32,
    pub attack_cooldown: f32,
    pub hit_flash: f32,
}

impl BossRuntime {
    fn new(object: &ae::RoomObject, brain: ae::BossBrain) -> Self {
        Self {
            id: object.id.clone(),
            name: object.name.clone(),
            pos: object.aabb.center(),
            spawn: object.aabb.center(),
            size: object.aabb.half_size() * 2.0,
            health: ae::Health::new(18),
            brain,
            alive: true,
            pattern_timer: 0.0,
            movement_timer: 0.0,
            attack_windup_timer: 0.0,
            attack_timer: 0.0,
            attack_cooldown: 0.35,
            hit_flash: 0.0,
        }
    }

    fn update(
        &mut self,
        world: &ae::World,
        player: &ae::Player,
        tuning: FeatureCombatTuning,
        dt: f32,
    ) {
        if !self.alive {
            return;
        }
        self.pattern_timer += dt;
        self.movement_timer += dt;
        // AMBITION_REVIEW(spatial): this is still a cheap authored boss movement
        // prototype, but it now computes chase from the stable spawn anchor and
        // moves toward the target with an axis-separated collision guard. The
        // previous version used current position as feedback in the chase term,
        // which could flip sign every frame near the player and visually split
        // the boss into two flickering locations.
        let anchor_to_player = player.pos - self.spawn;
        let chase = (anchor_to_player.x * 0.18).clamp(-70.0, 70.0);
        let target = ae::Vec2::new(
            self.spawn.x + (self.movement_timer * 0.72).sin() * 130.0 + chase,
            self.spawn.y - (self.movement_timer * 1.10).sin().abs() * 18.0,
        );
        self.move_toward_target(world, target, dt);
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        let was_winding_up = self.attack_windup_timer > 0.0;
        self.attack_windup_timer = (self.attack_windup_timer - dt).max(0.0);
        self.attack_timer = (self.attack_timer - dt).max(0.0);
        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);
        if was_winding_up && self.attack_windup_timer <= 0.0 {
            self.attack_timer = tuning.boss_attack_active.max(0.01);
        }
        if self.attack_cooldown <= 0.0
            && self.attack_windup_timer <= 0.0
            && self.attack_timer <= 0.0
        {
            self.attack_windup_timer = tuning.boss_attack_windup.max(0.01);
            self.attack_cooldown = BOSS_ATTACK_COOLDOWN;
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }

    pub fn attack_volumes(&self) -> Vec<ae::Aabb> {
        if self.attack_timer <= 0.0 {
            return Vec::new();
        }
        self.pattern_volumes()
    }

    pub fn attack_telegraph_volumes(&self) -> Vec<ae::Aabb> {
        if self.attack_windup_timer <= 0.0 {
            return Vec::new();
        }
        self.pattern_volumes()
    }

    pub fn body_damage_aabb(&self) -> ae::Aabb {
        self.aabb()
    }

    fn move_toward_target(&mut self, world: &ae::World, target: ae::Vec2, dt: f32) {
        let half = self.size * 0.5;
        let margin = 8.0;
        let max_x = (world.size.x - half.x - margin).max(half.x + margin);
        let max_y = (world.size.y - half.y - margin).max(half.y + margin);
        let clamped_target = ae::Vec2::new(
            target.x.clamp(half.x + margin, max_x),
            target.y.clamp(half.y + margin, max_y),
        );
        let delta = clamped_target - self.pos;
        let max_step = 220.0 * dt.max(0.0);
        let step = if delta.length() > max_step && max_step > 0.0 {
            delta.normalize_or_zero() * max_step
        } else {
            delta
        };

        let try_x = ae::Vec2::new(self.pos.x + step.x, self.pos.y);
        if boss_space_is_free(world, try_x, self.size) {
            self.pos.x = try_x.x;
        }
        let try_y = ae::Vec2::new(self.pos.x, self.pos.y + step.y);
        if boss_space_is_free(world, try_y, self.size) {
            self.pos.y = try_y.y;
        }
    }

    fn pattern_volumes(&self) -> Vec<ae::Aabb> {
        let phase = ((self.pattern_timer / BOSS_ATTACK_COOLDOWN) as i32).rem_euclid(3);
        match phase {
            0 => vec![ae::Aabb::new(
                self.pos + ae::Vec2::new(0.0, self.size.y * 0.5 + 22.0),
                ae::Vec2::new(self.size.x * 0.75, 18.0),
            )],
            1 => vec![
                ae::Aabb::new(
                    self.pos + ae::Vec2::new(-self.size.x * 0.75, 0.0),
                    ae::Vec2::new(22.0, self.size.y * 0.72),
                ),
                ae::Aabb::new(
                    self.pos + ae::Vec2::new(self.size.x * 0.75, 0.0),
                    ae::Vec2::new(22.0, self.size.y * 0.72),
                ),
            ],
            _ => vec![ae::Aabb::new(self.pos, self.size * 0.70)],
        }
    }

    fn player_damage(&self, player_body: ae::Aabb) -> Option<PlayerDamageEvent> {
        if self.attack_timer > 0.0 {
            if let Some(volume) = self
                .attack_volumes()
                .into_iter()
                .find(|volume| volume.strict_intersects(player_body))
            {
                return Some(PlayerDamageEvent {
                    mode: PlayerDamageMode::Knockback,
                    source: PlayerDamageSource::BossAttack,
                    source_pos: self.pos,
                    impact_pos: midpoint(player_body.center(), volume.center()),
                    knockback_dir: (player_body.center().x - self.pos.x).signum_or(1.0),
                    strength: 1.25,
                    amount: 2,
                });
            }
        }
        let body_damage = self.body_damage_aabb();
        if body_damage.strict_intersects(player_body) {
            return Some(PlayerDamageEvent {
                mode: PlayerDamageMode::Knockback,
                source: PlayerDamageSource::BossBody,
                source_pos: self.pos,
                impact_pos: midpoint(player_body.center(), body_damage.center()),
                knockback_dir: (player_body.center().x - self.pos.x).signum_or(1.0),
                strength: 1.0,
                amount: 1,
            });
        }
        None
    }
}

#[derive(Clone, Debug)]
pub struct BreakableRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub breakable: ae::Breakable,
    pub respawn_timer: f32,
    pub stand_timer: f32,
}

impl BreakableRuntime {
    fn new(object: &ae::RoomObject, breakable: ae::Breakable) -> Self {
        Self {
            id: object.id.clone(),
            name: object.name.clone(),
            pos: object.aabb.center(),
            size: object.aabb.half_size() * 2.0,
            breakable,
            respawn_timer: 0.0,
            stand_timer: 0.0,
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }

    pub fn broken(&self) -> bool {
        self.breakable.state == ae::BreakableState::Broken
    }

    fn start_respawn_timer(&mut self) {
        self.stand_timer = 0.0;
        if let ae::RespawnPolicy::AfterSeconds(seconds) = self.breakable.respawn {
            self.respawn_timer = seconds;
        }
    }

    fn breaks_on_stand(&self) -> bool {
        self.breakable.collision.blocks_movement() && self.breakable.trigger.allows_stand()
    }

    fn breaks_on_hit(&self) -> bool {
        self.breakable.trigger.allows_hit()
    }
}

#[cfg(test)]
mod conversion_tests {
    use super::*;

    fn world_with_npc(id: &str) -> (ae::World, NpcRuntime) {
        let world = ae::World::new(
            String::from("npc_test"),
            ae::Vec2::new(800.0, 600.0),
            ae::Vec2::new(100.0, 100.0),
            vec![ae::Block::solid(
                String::from("floor"),
                ae::Vec2::new(0.0, 560.0),
                ae::Vec2::new(800.0, 40.0),
            )],
        );
        let aabb = ae::Aabb::new(ae::Vec2::new(200.0, 540.0), ae::Vec2::new(11.0, 19.0));
        let object = ae::RoomObject::new(
            id.to_string(),
            id.to_string(),
            aabb,
            ae::RoomObjectKind::Interactable(ae::Interactable::new(
                id.to_string(),
                String::from("Talk"),
                aabb,
                ae::InteractionKind::Npc {
                    dialogue_id: Some(id.to_string()),
                },
            )),
        );
        let interactable = match object.kind.clone() {
            ae::RoomObjectKind::Interactable(it) => it,
            _ => unreachable!(),
        };
        let npc = NpcRuntime::new(&object, interactable);
        (world, npc)
    }

    #[test]
    fn striking_npc_three_times_flips_them_hostile() {
        let mut features = FeatureRuntime {
            hazards: Vec::new(),
            enemies: Vec::new(),
            bosses: Vec::new(),
            breakables: Vec::new(),
            pickups: Vec::new(),
            chests: Vec::new(),
            npcs: vec![world_with_npc("guide").1],
            switches: Vec::new(),
            water_volumes: Vec::new(),
            banner: String::new(),
            banner_timer: 0.0,
        };
        let attack = ae::Aabb::new(ae::Vec2::new(200.0, 540.0), ae::Vec2::new(20.0, 20.0));
        for _ in 0..3 {
            let _ = features.apply_player_attack(attack, 1, 0.0);
        }
        assert_eq!(features.npcs.len(), 1);
        assert!(features.npcs[0].hostile);
    }

    #[test]
    fn apply_save_with_hostile_flag_replaces_npc_with_enemy() {
        let mut features = FeatureRuntime {
            hazards: Vec::new(),
            enemies: Vec::new(),
            bosses: Vec::new(),
            breakables: Vec::new(),
            pickups: Vec::new(),
            chests: Vec::new(),
            npcs: vec![world_with_npc("guide").1],
            switches: Vec::new(),
            water_volumes: Vec::new(),
            banner: String::new(),
            banner_timer: 0.0,
        };
        let mut save = ae::SandboxSaveData::new();
        save.set_flag("npc_guide_hostile", true);
        features.apply_save(&save);
        assert!(features.npcs.is_empty(), "NPC should be removed");
        assert_eq!(features.enemies.len(), 1, "An enemy should replace the NPC");
        assert_eq!(features.enemies[0].id, "guide");
    }

    #[test]
    fn apply_save_with_dead_flag_keeps_npc_dead_no_respawn() {
        let mut features = FeatureRuntime {
            hazards: Vec::new(),
            enemies: Vec::new(),
            bosses: Vec::new(),
            breakables: Vec::new(),
            pickups: Vec::new(),
            chests: Vec::new(),
            npcs: vec![world_with_npc("guide").1],
            switches: Vec::new(),
            water_volumes: Vec::new(),
            banner: String::new(),
            banner_timer: 0.0,
        };
        let mut save = ae::SandboxSaveData::new();
        save.set_flag("npc_guide_hostile", true);
        save.set_flag("enemy_guide_dead", true);
        features.apply_save(&save);
        assert!(features.npcs.is_empty(), "NPC was hostile, removed");
        assert!(
            features.enemies.is_empty(),
            "Dead flag should suppress the conversion respawn"
        );
    }

    #[test]
    fn spawn_chest_appends_a_chest_runtime() {
        let mut features = FeatureRuntime {
            hazards: Vec::new(),
            enemies: Vec::new(),
            bosses: Vec::new(),
            breakables: Vec::new(),
            pickups: Vec::new(),
            chests: Vec::new(),
            npcs: Vec::new(),
            switches: Vec::new(),
            water_volumes: Vec::new(),
            banner: String::new(),
            banner_timer: 0.0,
        };
        features.spawn_chest(
            "encounter_chest_mob_lab".into(),
            Some(ae::PickupKind::Health { amount: 2 }),
            ae::Vec2::new(400.0, 300.0),
            ae::Vec2::new(28.0, 28.0),
        );
        assert_eq!(features.chests.len(), 1);
        assert_eq!(features.chests[0].id, "encounter_chest_mob_lab");
        // Same id again → no double-spawn.
        features.spawn_chest(
            "encounter_chest_mob_lab".into(),
            None,
            ae::Vec2::new(0.0, 0.0),
            ae::Vec2::new(28.0, 28.0),
        );
        assert_eq!(features.chests.len(), 1, "should not double-spawn");
    }

    #[test]
    fn apply_save_marks_authored_enemy_dead_when_save_says_so() {
        // Authored enemy from a regular EnemySpawn (no encounter
        // prefix). The save flag should mark it dead on load.
        let world = ae::World::new(
            String::from("enemy_test"),
            ae::Vec2::new(800.0, 600.0),
            ae::Vec2::new(100.0, 100.0),
            vec![ae::Block::solid(
                String::from("floor"),
                ae::Vec2::new(0.0, 560.0),
                ae::Vec2::new(800.0, 40.0),
            )],
        )
        .with_objects(vec![ae::RoomObject::new(
            String::from("spider"),
            String::from("spider"),
            ae::Aabb::new(ae::Vec2::new(400.0, 540.0), ae::Vec2::new(11.0, 19.0)),
            ae::RoomObjectKind::EnemySpawn(ae::EnemyBrain::Custom(String::from("medium_striker"))),
        )]);
        let mut features = FeatureRuntime::from_world(&world);
        assert_eq!(features.enemies.len(), 1);
        assert!(features.enemies[0].alive);
        let mut save = ae::SandboxSaveData::new();
        save.set_flag("enemy_spider_dead", true);
        features.apply_save(&save);
        assert!(!features.enemies[0].alive);
    }
}

fn player_is_standing_on(player: ae::Aabb, platform: ae::Aabb) -> bool {
    let horizontally_overlaps =
        player.right() > platform.left() + 2.0 && player.left() < platform.right() - 2.0;
    let near_top = (player.bottom() - platform.top()).abs() <= 8.0;
    horizontally_overlaps && near_top
}

#[derive(Clone, Debug)]
pub struct PickupRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub pickup: ae::Pickup,
    pub visible: bool,
}

impl PickupRuntime {
    fn new(object: &ae::RoomObject, pickup: ae::Pickup) -> Self {
        Self {
            id: object.id.clone(),
            name: object.name.clone(),
            pos: object.aabb.center(),
            size: object.aabb.half_size() * 2.0,
            pickup,
            visible: true,
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }
}

#[derive(Clone, Debug)]
pub struct ChestRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub chest: ae::Chest,
    pub opened: bool,
}

impl ChestRuntime {
    fn new(object: &ae::RoomObject, chest: ae::Chest) -> Self {
        Self {
            id: object.id.clone(),
            name: object.name.clone(),
            pos: object.aabb.center(),
            size: object.aabb.half_size() * 2.0,
            chest,
            opened: false,
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }
}

#[derive(Clone, Debug)]
pub struct NpcRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub interactable: ae::Interactable,
    /// Hostility flag. Becomes true after the player strikes the NPC
    /// enough times to provoke them. The save flag mirrors this so
    /// hostility persists across rooms / saves. Once flipped, the
    /// NPC is removed from `FeatureRuntime::npcs` and replaced by an
    /// `EnemyRuntime` carrying the same id; this flag exists only for
    /// the brief window between the strike that flipped hostility
    /// and the next call to `apply_save`.
    pub hostile: bool,
    /// Hits the NPC has taken since the last reset. Crosses
    /// `HOSTILE_THRESHOLD` to flip hostile.
    pub strikes: i32,
    /// Brief flash after a strike — used by the renderer to flicker
    /// the NPC red without changing the dialog system.
    pub hit_flash: f32,
}

/// Apply save-derived state (NPC hostility, boss defeats) onto the
/// live `FeatureRuntime`. Public free function so room-load paths
/// that already hold the save can apply it inline; a Bevy system
/// (`sync_features_with_save`) calls it each frame as a safety net.
pub fn apply_save_to_features(features: &mut FeatureRuntime, save: &ae::SandboxSaveData) {
    features.apply_save(save);
}

/// Bevy system: keep the feature runtime in sync with the save
/// resource. Runs each frame; cheap idempotent linear pass.
pub fn sync_features_with_save(
    mut runtime: ResMut<crate::SandboxRuntime>,
    save: Res<crate::save::SandboxSave>,
) {
    apply_save_to_features(&mut runtime.features, save.data());
}

/// Cross-system bus for feature events that need to fan out to
/// resources `sandbox_update` doesn't hold. Refilled each frame in
/// `feature_runtime_phase`; drained by `drain_feature_event_bus`.
#[derive(Resource, Default)]
pub struct FeatureEventBus {
    pub boss_damage: Vec<(String, i32)>,
    pub npc_struck: Vec<(String, ae::Vec2)>,
    pub quest_advance: Vec<ae::QuestAdvanceEvent>,
    pub flag_writes: Vec<(String, bool)>,
}

impl FeatureEventBus {
    pub fn ingest(&mut self, events: &FeatureEvents) {
        self.boss_damage.extend(events.boss_damage.iter().cloned());
        self.npc_struck.extend(events.npc_struck.iter().cloned());
        self.quest_advance
            .extend(events.quest_advance.iter().cloned());
        self.flag_writes.extend(events.flag_writes.iter().cloned());
    }
}

/// Bevy system: drain `FeatureEventBus` into the right downstream
/// systems. Splits the work across resources so `sandbox_update`
/// stays under the system-param limit.
pub fn drain_feature_event_bus(
    mut bus: ResMut<FeatureEventBus>,
    mut runtime: ResMut<crate::SandboxRuntime>,
    mut save: ResMut<crate::save::SandboxSave>,
    mut quests: ResMut<crate::quest::QuestRegistry>,
    mut boss_registry: ResMut<crate::boss_encounter::BossEncounterRegistry>,
    mut music_request: ResMut<crate::encounter::EncounterMusicRequest>,
    mut cutscene_queue: ResMut<crate::cutscene::CutsceneTriggerQueue>,
) {
    // Flag writes first so quest conditions that read flags see them
    // this same frame.
    let flags = std::mem::take(&mut bus.flag_writes);
    for (id, on) in flags {
        // Mirror flag write into a quest advance event so any quest
        // step keyed on this flag can react.
        if on {
            quests.push_event(ae::QuestAdvanceEvent::FlagSet(id.clone()));
        }
        save.data_mut().set_flag(id, on);
    }
    // Quest advance events from gameplay (NPC talked, etc.).
    let advances = std::mem::take(&mut bus.quest_advance);
    for ev in advances {
        quests.push_event(ev);
    }
    // Boss damage routes through the boss encounter machine.
    let boss_damage = std::mem::take(&mut bus.boss_damage);
    for (boss_id, amount) in boss_damage {
        crate::boss_encounter::record_boss_damage(
            &mut boss_registry,
            &mut music_request,
            &mut cutscene_queue,
            &mut runtime.features,
            &boss_id,
            amount,
        );
    }
    // NPC strikes are reportable for the trace; the actual hostility
    // flip happens inside `apply_player_attack`. Drain to avoid
    // accumulation.
    bus.npc_struck.clear();
}

/// Number of player attacks before a peaceful NPC turns hostile.
/// Three lets the player commit to the choice intentionally without
/// flipping by accident on a stray slash.
pub const NPC_HOSTILE_STRIKE_THRESHOLD: i32 = 3;

impl NpcRuntime {
    fn new(object: &ae::RoomObject, interactable: ae::Interactable) -> Self {
        Self {
            id: object.id.clone(),
            name: object.name.clone(),
            pos: object.aabb.center(),
            size: object.aabb.half_size() * 2.0,
            interactable,
            hostile: false,
            strikes: 0,
            hit_flash: 0.0,
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }

    pub fn flag_id(&self) -> String {
        format!("npc_{}_hostile", self.id)
    }

    fn message(&self) -> String {
        if self.hostile {
            return format!("{} attacks!", self.name);
        }
        match &self.interactable.kind {
            ae::InteractionKind::Npc {
                dialogue_id: Some(dialogue_id),
            } => {
                format!("{} opens dialogue {}", self.name, dialogue_id)
            }
            _ => format!("{} opens fallback dialogue", self.name),
        }
    }

    fn dialogue_request(&self) -> NpcDialogueRequest {
        let dialogue_id = match &self.interactable.kind {
            ae::InteractionKind::Npc {
                dialogue_id: Some(dialogue_id),
            } => dialogue_id.clone(),
            _ => "generic_npc".to_string(),
        };
        NpcDialogueRequest {
            npc_id: self.id.clone(),
            npc_name: self.name.clone(),
            dialogue_id,
        }
    }

    // Hostile NPCs are converted to `EnemyRuntime` instances in
    // `apply_save`. The legacy `hostile_damage` body-volume method
    // was removed because the spawned enemy now handles contact
    // damage through the standard `EnemyRuntime::player_damage`.
}

#[derive(Clone, Debug)]
pub struct PathMotion {
    path: ae::KinematicPath,
    segment: usize,
    dir: i32,
}

impl PathMotion {
    fn new(path: ae::KinematicPath) -> Self {
        Self {
            path,
            segment: 0,
            dir: 1,
        }
    }

    fn advance(&mut self, mut pos: ae::Vec2, dt: f32) -> ae::Vec2 {
        if !self.path.is_valid() || dt <= 0.0 {
            return pos;
        }
        let mut remaining = self.path.speed * dt;
        while remaining > 0.0 {
            let target_index = if self.dir >= 0 {
                self.segment + 1
            } else {
                self.segment
            };
            let Some(target) = self.path.points.get(target_index).copied() else {
                break;
            };
            let to_target = target - pos;
            let distance = to_target.length();
            if distance <= 0.001 {
                self.advance_segment();
                continue;
            }
            let step = remaining.min(distance);
            pos += to_target / distance * step;
            remaining -= step;
            if step >= distance - 0.001 {
                self.advance_segment();
            }
        }
        pos
    }

    fn advance_segment(&mut self) {
        let last_segment = self.path.points.len().saturating_sub(2);
        match self.path.mode {
            ae::KinematicPathMode::Once => {
                if self.dir >= 0 && self.segment < last_segment {
                    self.segment += 1;
                }
            }
            ae::KinematicPathMode::Loop => {
                if self.dir >= 0 {
                    self.segment = if self.segment >= last_segment {
                        0
                    } else {
                        self.segment + 1
                    };
                } else if self.segment == 0 {
                    self.segment = last_segment;
                } else {
                    self.segment -= 1;
                }
            }
            ae::KinematicPathMode::PingPong => {
                if self.dir >= 0 {
                    if self.segment >= last_segment {
                        self.dir = -1;
                    } else {
                        self.segment += 1;
                    }
                } else if self.segment == 0 {
                    self.dir = 1;
                } else {
                    self.segment -= 1;
                }
            }
        }
    }
}

pub fn world_with_sandbox_solids(
    world: &ae::World,
    platform: &MovingPlatformState,
    features: &FeatureRuntime,
) -> ae::World {
    let mut collision_world = crate::platforms::world_with_moving_platform(world, platform);
    for breakable in &features.breakables {
        if breakable.broken() {
            continue;
        }
        // Breakable pogo orbs contribute a pogo-orb block (no body collision)
        // while intact, so the engine's pogo-bounce logic finds them; the
        // bounce damage is routed back through `FeatureRuntime::on_pogo_bounce`
        // by the gameplay loop.
        if breakable.breakable.pogo_refresh {
            collision_world.blocks.push(ae::Block {
                name: format!("breakable-pogo {}", breakable.name),
                aabb: breakable.aabb(),
                kind: ae::BlockKind::PogoOrb,
            });
            continue;
        }
        let kind = match breakable.breakable.collision {
            ae::BreakableCollision::None => continue,
            // Solid breakables behave like a hard blink wall for blink
            // pathing: identical to BlockKind::Solid for ordinary movement
            // (BlinkWall is solid on both axes), but max-tier blink with
            // `blink_through_hard_walls` can teleport through. Lower-tier
            // blink is still blocked, so the breakable still gates progress
            // until the player either earns the upgrade or breaks it.
            ae::BreakableCollision::Solid => ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Hard,
            },
            ae::BreakableCollision::OneWayUp => ae::BlockKind::OneWay,
        };
        collision_world.blocks.push(ae::Block {
            name: format!("breakable {}", breakable.name),
            aabb: breakable.aabb(),
            kind,
        });
    }
    collision_world
}

fn boss_space_is_free(world: &ae::World, pos: ae::Vec2, size: ae::Vec2) -> bool {
    let aabb = ae::Aabb::new(pos, size * 0.5);
    if aabb.left() < 0.0
        || aabb.right() > world.size.x
        || aabb.top() < 0.0
        || aabb.bottom() > world.size.y
    {
        return false;
    }
    !world.body_overlaps_any(aabb, |block| {
        matches!(
            block.kind,
            ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. } | ae::BlockKind::OneWay
        )
    })
}

fn room_paths(world: &ae::World) -> Vec<(String, ae::KinematicPath)> {
    world
        .objects
        .iter()
        .filter_map(|object| match &object.kind {
            ae::RoomObjectKind::KinematicPath(path) => Some((object.id.clone(), path.clone())),
            _ => None,
        })
        .collect()
}

fn blocked(world: &ae::World, aabb: ae::Aabb) -> bool {
    world.body_overlaps_any(aabb, |block| {
        matches!(
            block.kind,
            ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
        )
    })
}

fn blocked_y(world: &ae::World, aabb: ae::Aabb) -> bool {
    world.body_overlaps_any(aabb, |block| {
        matches!(
            block.kind,
            ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. } | ae::BlockKind::OneWay
        )
    })
}

fn approach(value: f32, target: f32, delta: f32) -> f32 {
    if value < target {
        (value + delta).min(target)
    } else {
        (value - delta).max(target)
    }
}

fn approximately_same_aabb(a: ae::Aabb, b: ae::Aabb) -> bool {
    // Pogo-bounce routing matches an engine-reported orb AABB against
    // sandbox-side breakable AABBs. The two are derived from the same
    // entity placement so the values agree to floating-point tolerance,
    // but a tiny epsilon avoids spurious mismatches if a future codepath
    // recomputes one of the AABBs from rounded coordinates.
    let eps = 0.5;
    (a.center() - b.center()).length() <= eps && (a.half_size() - b.half_size()).length() <= eps
}

fn midpoint(a: ae::Vec2, b: ae::Vec2) -> ae::Vec2 {
    ae::Vec2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

trait SignumOr {
    fn signum_or(self, fallback: f32) -> f32;
}

impl SignumOr for f32 {
    fn signum_or(self, fallback: f32) -> f32 {
        if self.abs() <= 0.001 {
            fallback
        } else {
            self.signum()
        }
    }
}
