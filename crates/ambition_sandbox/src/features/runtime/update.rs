use super::*;

impl FeatureRuntime {
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
                        events.set_flag(format!("encounter_{encounter_id}_reward_dropped"), true);
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
                        events.advance_quest(ae::QuestAdvanceEvent::NpcTalked(npc.id.clone()));
                        events.set_flag("met_any_hub_npc", true);
                        events
                            .set_flag(format!("npc_{}_talked", dialogue_request.dialogue_id), true);
                        events.dialogue_request = Some(dialogue_request);
                    }
                    events.bursts.push(npc.pos);
                }
            }
            // Switch interaction is now ECS-owned. `self.switches` remains as a
            // compatibility mirror for encounter arming until those helpers query
            // `SwitchFeature` components directly.
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
}
