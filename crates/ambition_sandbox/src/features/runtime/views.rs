use super::*;

impl FeatureRuntime {
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
}
