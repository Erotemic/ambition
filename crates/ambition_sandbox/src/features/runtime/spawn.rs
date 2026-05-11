use super::*;

impl FeatureRuntime {
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
