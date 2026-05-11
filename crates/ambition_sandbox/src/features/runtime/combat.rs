use super::*;

impl FeatureRuntime {
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
}
