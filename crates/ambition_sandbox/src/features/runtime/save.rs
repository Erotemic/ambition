use super::*;

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
}
