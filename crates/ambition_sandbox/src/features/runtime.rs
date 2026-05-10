use super::*;

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
    pub banner: String,
    pub banner_timer: f32,
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
    pub(super) fn new(
        object: &ae::RoomObject,
        interactable: ae::Interactable,
        payload: String,
    ) -> Self {
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
            // Sprite-override gate. Only the Kernel Guide has the
            // dedicated "turns into a goblin" beat (it's a deliberate
            // joke about the hub guide growing tusks); every other
            // hostile NPC keeps their own spritesheet so their
            // authored slash / hit rows actually drive the visual.
            // `None` here means "use the default `Enemy` sheet
            // (goblin)" — the kernel guide's name shadows that.
            if npc.name != "Kernel Guide NPC" {
                if let Some(enemy) = self.enemies.last_mut() {
                    enemy.sprite_override_npc_name = Some(npc.name.clone());
                }
            }
            self.banner = format!("{} attacks!", npc.name);
            self.banner_timer = 1.5;
        }
        // Authored enemies (LDtk EnemySpawn) that were killed and
        // recorded in the save should also stay dead.
        for enemy in &mut self.enemies {
            if save.flag(&format!("enemy_{}_dead", enemy.id)) && !enemy.id.starts_with("encounter:")
            {
                enemy.alive = false;
                enemy.health.current = 0;
            }
        }
        // Boss defeats: hide already-cleared bosses by marking the
        // runtime dead. New `BossSpawn` instances from the LDtk
        // file all start `alive=true`, so this is the gate.
        for boss in &mut self.bosses {
            if matches!(save.boss(&boss.id), ae::PersistedEncounterState::Cleared) {
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

/// Run one fixed-`dt` gravity step on a falling chest, sub-stepped so
/// a fast-falling chest can't tunnel through a thin floor. Clears
/// `falling` (and `vel_y`) on the first solid contact below.
///
/// Pulled out of `FeatureRuntime::update` so the boss-encounter
/// "spawn-and-fast-settle" path can run the same physics in a tight
/// loop (see `sync_mockingbird_treasure_chest`) — guaranteeing a
/// looted chest re-spawns at the *same* settled y the live tick
/// would have produced, instead of dropping again on every room load.
pub fn tick_chest_fall(chest: &mut ChestRuntime, world: &ae::World, dt: f32) {
    chest.vel_y = (chest.vel_y + CHEST_FALL_GRAVITY * dt).min(CHEST_FALL_MAX_SPEED);
    let step = chest.vel_y * dt;
    if step <= 0.0 {
        return;
    }
    let max_substep = (chest.size.y * 0.5).max(2.0);
    let mut remaining = step;
    while remaining > 0.0 {
        let advance = remaining.min(max_substep);
        let try_pos = ae::Vec2::new(chest.pos.x, chest.pos.y + advance);
        let try_aabb = ae::Aabb::new(try_pos, chest.size * 0.5);
        let blocked = world.body_overlaps_any(try_aabb, |block| {
            matches!(
                block.kind,
                ae::BlockKind::Solid | ae::BlockKind::OneWay | ae::BlockKind::BlinkWall { .. }
            )
        });
        if blocked {
            chest.falling = false;
            chest.vel_y = 0.0;
            break;
        }
        chest.pos = try_pos;
        remaining -= advance;
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
                let verb = match hazard.mode {
                    PlayerDamageMode::SafeRespawn => "forced a safe respawn",
                    PlayerDamageMode::Knockback => "knocked the player back",
                };
                events.messages.push(format!("{} {}", hazard.name, verb));
                events.impacts.push(player.pos);
                // AMBITION_REVIEW(spatial): horizontal-only knockback —
                // collapses the 2D player↔hazard offset to its X sign.
                // OK while `apply_player_knockback` itself only consumes
                // a horizontal direction; revisit if knockback ever
                // gains a vertical component.
                let knockback_dir = (player.pos.x - hazard.pos.x).signum();
                events.player_damage.push(PlayerDamageEvent {
                    mode: hazard.mode,
                    source: PlayerDamageSource::Hazard,
                    source_pos: hazard.pos,
                    impact_pos: player.pos,
                    knockback_dir,
                    strength: 1.0,
                    amount: hazard.volume.damage.amount.max(1),
                });
                events.play_sfx(hazard_sfx_id(&hazard.name), player.pos);
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
                        events.breakables_destroyed.push(breakable.pos);
                    }
                }
            } else {
                breakable.stand_timer = (breakable.stand_timer - dt * 2.0).max(0.0);
            }
        }

        // Falling-chest physics. Chests spawned mid-air (today: the
        // pirate-hoard drop from a defeated mockingbird) integrate
        // gravity until they touch a solid block below, then settle
        // and behave like any other static chest. Authored / encounter
        // chests have `falling = false` and skip this loop entirely.
        for chest in &mut self.chests {
            if !chest.falling {
                continue;
            }
            tick_chest_fall(chest, world, dt);
        }

        for pickup in &mut self.pickups {
            if pickup.visible && pickup.aabb().strict_intersects(player_body) {
                pickup.visible = false;
                events.messages.push(format!("picked up {}", pickup.name));
                if let ae::PickupKind::Health { amount } = pickup.pickup.kind {
                    events.player_heal += amount;
                }
                events.bursts.push(pickup.pos);
                events
                    .pickups_collected
                    .push((pickup.pickup.kind.clone(), pickup.pos));
            }
        }

        if interact_pressed {
            for chest in &mut self.chests {
                if !chest.opened && chest.aabb().strict_intersects(player_body) {
                    chest.opened = true;
                    events.consumed_interaction = true;
                    events.messages.push(format!("opened {}", chest.name));
                    events.bursts.push(chest.pos);
                    events.chests_opened.push(chest.pos);
                    // Persist the looted state so save+reload re-spawns
                    // the chest in its opened state. Encounter chests
                    // are keyed `encounter_chest_<id>`; the matching
                    // looted flag is `encounter_<id>_reward_dropped`
                    // (`crate::encounter::encounter_reward_looted_flag`).
                    if let Some(encounter_id) = chest.id.strip_prefix("encounter_chest_") {
                        events
                            .flag_writes
                            .push((format!("encounter_{encounter_id}_reward_dropped"), true));
                    }
                }
            }
            for npc in &mut self.npcs {
                if npc.aabb().strict_intersects(player_body) {
                    events.consumed_interaction = true;
                    events.messages.push(npc.message());
                    if !npc.hostile {
                        let dialogue_request = npc.dialogue_request();
                        // Quest hook: "talked to NPC" + a generic
                        // "met any hub NPC" flag the tutorial quest
                        // listens for, plus a per-dialogue flag so
                        // quests can key on specific NPCs without
                        // depending on the LDtk-issued iid.
                        events
                            .quest_advance
                            .push(ae::QuestAdvanceEvent::NpcTalked(npc.id.clone()));
                        events.flag_writes.push(("met_any_hub_npc".into(), true));
                        events.flag_writes.push((
                            format!("npc_{}_talked", dialogue_request.dialogue_id),
                            true,
                        ));
                        events.dialogue_request = Some(dialogue_request);
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
                    events.switches_activated_pos.push(switch.pos);
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
                    events.play_sfx(ambition_sfx::ids::PLAYER_DAMAGE, damage.impact_pos);
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
                    events.play_sfx(ambition_sfx::ids::PLAYER_DAMAGE, damage.impact_pos);
                    events.impacts.push(damage.impact_pos);
                    events.player_damage.push(damage);
                }
            }
        }

        // Hostile NPCs are converted to `EnemyRuntime` instances by
        // `apply_save`, so the NPC tick here is purely peaceful
        // physics + patrol AI. Hit-flash decay happens inside
        // `npc.update`.
        for npc in &mut self.npcs {
            npc.update(world, player, dt);
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
        // Slash is one specialization of the unified damage path —
        // see `apply_damage_event` for the canonical entry point.
        // Keeping this signature stable so existing call sites
        // (sandbox_update's slash phase, attack tests) don't change.
        let report = self.apply_damage_event(&DamageEvent {
            volume: attack,
            damage,
            source: DamageSource::PlayerSlash { knock_x },
        });
        report.events
    }

    /// Apply a damage event against every damageable feature whose
    /// AABB strictly overlaps `event.volume`. Single source of truth
    /// for "AABB + damage hits the world": slashes, projectiles, and
    /// any future tool / hazard / spell that produces a damage volume
    /// route through here.
    ///
    /// Returns a `DamageReport` with:
    /// - `events`: the side-effect bundle (already merged into the
    ///   runtime's internal state — caller only needs to forward
    ///   audio/VFX/quest/flag writes to its own bus).
    /// - hit counts per target type so projectile resolution can
    ///   decide whether to expire the source.
    ///
    /// Per-target behavior:
    /// - Enemies: damage + optional slash-style knockback. Encounter
    ///   mobs (`enemy.id` starting with `"encounter:"`) skip the
    ///   "enemy_<id>_dead" save flag because the encounter state
    ///   machine owns their lifecycle.
    /// - Bosses: damage routes to the boss-encounter machine via
    ///   `events.boss_damage`.
    /// - NPCs: any hit increments `strikes`; crossing the threshold
    ///   flips hostility (regardless of source — projectiles provoke
    ///   too).
    /// - Breakables: take damage UNLESS they're pogo-refresh orbs
    ///   (those only react to the dedicated `on_pogo_bounce` path).
    pub fn apply_damage_event(&mut self, event: &DamageEvent) -> DamageReport {
        let mut report = DamageReport::default();
        let attack = event.volume;
        let damage = event.damage;
        let knock = match event.source {
            DamageSource::PlayerSlash { knock_x } => Some(knock_x),
            _ => None,
        };

        for enemy in &mut self.enemies {
            if enemy.alive && attack.strict_intersects(enemy.aabb()) {
                enemy.hit_flash = 0.16;
                if let Some(knock_x) = knock {
                    enemy.vel.x += knock_x;
                    enemy.vel.y = (enemy.vel.y - 90.0).max(-280.0);
                }
                let killed = if enemy.archetype == EnemyArchetype::InfiniteSandbag {
                    false
                } else {
                    enemy.health.damage(damage)
                };
                let hit_pos = midpoint(attack.center(), enemy.pos);
                report.events.impacts.push(hit_pos);
                report.enemies_hit += 1;
                if killed {
                    enemy.alive = false;
                    report.kills += 1;
                    if enemy.archetype == EnemyArchetype::FiniteSandbag {
                        enemy.respawn_timer = 0.85;
                        report
                            .events
                            .messages
                            .push(format!("{} dropped; respawning", enemy.name));
                    } else {
                        report
                            .events
                            .messages
                            .push(format!("defeated {}", enemy.name));
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
                            report
                                .events
                                .flag_writes
                                .push((format!("enemy_{}_dead", enemy.id), true));
                        }
                    }
                    report.events.bursts.push(enemy.pos);
                    report.events.physics_bursts.push(FeaturePhysicsBurst {
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
                report
                    .events
                    .impacts
                    .push(midpoint(attack.center(), boss.pos));
                report.events.boss_damage.push((boss.id.clone(), amount));
                report.bosses_hit += 1;
                if killed {
                    boss.alive = false;
                    report.kills += 1;
                    report
                        .events
                        .messages
                        .push(format!("defeated boss {}", boss.name));
                    report.events.bursts.push(boss.pos);
                    report.events.physics_bursts.push(FeaturePhysicsBurst {
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
        // sandbox runtime persists it. Any damage source provokes —
        // projectiles count as strikes too.
        for npc in &mut self.npcs {
            if !attack.strict_intersects(npc.aabb()) {
                continue;
            }
            npc.hit_flash = 0.18;
            report
                .events
                .impacts
                .push(midpoint(attack.center(), npc.pos));
            report.events.npc_struck.push((npc.id.clone(), npc.pos));
            report.npcs_hit += 1;
            if npc.hostile {
                npc.strikes = npc.strikes.saturating_add(1);
                if npc.strikes >= NPC_HOSTILE_STRIKE_THRESHOLD * 2 {
                    report
                        .events
                        .messages
                        .push(format!("{} flees the room", npc.name));
                }
            } else {
                npc.strikes = npc.strikes.saturating_add(1);
                if npc.strikes >= NPC_HOSTILE_STRIKE_THRESHOLD {
                    npc.hostile = true;
                    report
                        .events
                        .messages
                        .push(format!("{} turns hostile", npc.name));
                    report.events.flag_writes.push((npc.flag_id(), true));
                    report.events.bursts.push(npc.pos);
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
                report
                    .events
                    .impacts
                    .push(midpoint(attack.center(), breakable.pos));
                report.breakables_hit += 1;
                if broke {
                    breakable.start_respawn_timer();
                    report
                        .events
                        .messages
                        .push(format!("broke {}", breakable.name));
                    report.events.bursts.push(breakable.pos);
                    report.events.physics_bursts.push(FeaturePhysicsBurst {
                        pos: breakable.pos,
                        cue: FeaturePhysicsCue::Breakable,
                    });
                    report.events.breakables_destroyed.push(breakable.pos);
                }
            }
        }

        self.accept_events(&report.events);
        report
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
                events.breakables_destroyed.push(breakable.pos);
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
                    size: boss.render_size(),
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

    /// Authored display name for an NPC by feature id (LDtk iid).
    /// The renderer uses this to pick a faction-specific spritesheet
    /// (see `CharacterSpriteAssets::npc_asset_for_name`); ids are not
    /// human-meaningful, the LDtk `name` field is.
    pub fn npc_name(&self, id: &str) -> Option<&str> {
        self.npcs
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.name.as_str())
    }

    /// If this enemy was spawned by migrating a hostile NPC, return
    /// the LDtk display name of the original NPC so the renderer can
    /// keep the NPC's authored spritesheet. Falls through to `None`
    /// for authored / encounter-spawned enemies, which use the
    /// default goblin sheet.
    pub fn enemy_sprite_override(&self, id: &str) -> Option<&str> {
        self.enemies
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.sprite_override_npc_name.as_deref())
    }

    /// Snapshot the NPC state needed to drive its sprite animation.
    /// Returns `None` if no NPC with that id exists. Mirrors
    /// `enemy_anim_state` so the animation system can fall through
    /// to NPCs after enemies (a feature id is only ever in one of
    /// the two lists at a time).
    pub fn npc_anim_state(&self, id: &str) -> Option<crate::character_sprites::NpcAnimState> {
        self.npcs
            .iter()
            .find(|n| n.id == id)
            .map(|n| crate::character_sprites::NpcAnimState {
                vel: n.vel,
                facing: n.facing,
                hit_flash: n.hit_flash > 0.0,
            })
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

    /// Look up a boss's authored display name by feature id. Used by
    /// the rendering layer to pick the right per-boss spritesheet
    /// (mockingbird vs gradient sentinel) at sprite-bind time.
    pub fn boss_name(&self, id: &str) -> Option<&str> {
        self.bosses
            .iter()
            .find(|b| b.id == id)
            .map(|b| b.name.as_str())
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

    pub(super) fn accept_events(&mut self, events: &FeatureEvents) {
        if let Some(message) = events.messages.last() {
            self.banner = message.clone();
            self.banner_timer = 2.6;
        }
    }
}
