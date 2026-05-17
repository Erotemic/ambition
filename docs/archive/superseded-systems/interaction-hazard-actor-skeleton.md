# Archived: interaction-hazard-actor-skeleton.md

Superseded by current LDtk/data-driven ECS docs and concept pages.

Original path: `docs/systems/interaction-hazard-actor-skeleton.md`

---

# Interaction / hazard / actor architecture skeleton

Patch 2 establishes the shared engine vocabulary for gameplay objects before the sandbox grows basement feature rooms. The intent is to keep hazards, enemies, bosses, breakables, chests, pickups, NPCs, and debug labels composable rather than implementing each as a one-off sandbox system.

The engine now has reusable data/mechanics modules for actors, combat/damage volumes, interaction objects, and debug labels. `ambition_engine::World` keeps collision `Block`s as the room geometry language and adds `objects: Vec<RoomObject>` for authored gameplay entities layered on top of that geometry. This lets future RON room data describe spike balls, pickup rewards, chest rewards, breakables, enemy spawns, boss spawns, kinematic paths, and labels without changing the low-level block collision model.

The sandbox data loader accepts an optional `objects` list on each room. Existing rooms do not need to change because the field is `#[serde(default)]`. The current patch only builds the object graph; it does not yet spawn visible gameplay entities for every object kind. That follow-up should be small because the Bevy sandbox can iterate `world.objects` and choose rendering/simulation systems by `RoomObjectKind`.

Recommended follow-up order:

1. Render/debug the new room objects as simple colored outlines or labels.
2. Move legacy `BlockKind::Hazard` toward `DamageVolume` objects while keeping block hazards as a compatibility path.
3. Add basement test rooms that use `objects` for hazards, breakables, pickups/chests, enemies, bosses, and NPC/dialogue placeholders.
4. Introduce `seldom_state` only after the placeholder `EnemyBrain` / `BossBrain` types prove where per-entity state machines are needed.
