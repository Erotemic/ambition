# Engine architecture

`ambition_engine` is the reusable mechanics crate. It is allowed to depend on Bevy-adjacent crates such as `bevy_math` when those crates provide battle-tested primitives that Ambition should not maintain itself.

The current intent is:

- `ambition_engine` owns movement rules, collision semantics, abilities, combat hitboxes, enemy/test-target behavior, generated music/audio specs, and reusable mechanics.
- `ambition_sandbox` owns the Bevy app shell, runtime presentation, input wiring, debug tooling, and sandbox-specific RON data.
- Future story crates should look more like thin content crates: they select mechanics and provide room/story data without reimplementing player movement or collision details.

Important source modules:

- `abilities.rs` — ability flags (`AbilitySet`) and compatibility warnings.
- `actor.rs` — `Actor`, `Health`, `EnemyBrain`, `BossBrain`, `KinematicPath`.
- `boss_encounter.rs` — `BossEncounterSpec`, `BossEncounterState`, phase events.
- `boss_patterns.rs` — `BossAttackKind`, `BossPatternSchedule` for telegraph timing.
- `character_ai.rs` — backend-neutral character AI evaluator; sandbox feeds `CharacterAiSnapshot`.
- `combat.rs` — `Hitbox`, `Hurtbox`, slash/pogo hitbox computation.
- `cutscene.rs` — `CutsceneScript`, `CutsceneRuntime` for scripted sequences.
- `debug.rs` — `DebugLabel`, `DestinationLabel` for sandbox-side debug overlays.
- `enemy.rs` — test dummy / `Dummy` / `DummyKind` and knockback behavior.
- `geometry.rs` — Bevy `Aabb2d` helpers plus Ambition-specific strict overlap and Parry-backed shape casts, including contact normals for swept hits.
- `interaction.rs` — `Breakable`, `Chest`, `Pickup`, `Interactable` primitives.
- `ledge_grab.rs` — `probe_ledge_grab`, `LedgeContact` for ledge-grab AABB lookup.
- `movement.rs` / `movement/` — player movement facade plus child modules for blink, collision/sweeps, control actions, velocity integration, simulation clocks, player state, input, ops/events, tuning, and tests.
- `music.rs` — data structures shared by generated audio/music authoring.
- `physics.rs` — `PhysicsBodyKind`, `PhysicsMaterial`, `PhysicsShape`, `RagdollSpec` (data, not Avian wiring).
- `player_state.rs` — `BodyMode`, `BodyShape`, `LocomotionState`, `ResourceMeter`, collision-safe `try_change_body_mode`.
- `projectile.rs` — `ProjectileSpec`, `ProjectileBody`, `MotionInputBuffer` (Hadouken).
- `quest.rs` — `QuestSpec`, `QuestState`, `QuestStepCondition`.
- `save.rs` — versioned `SandboxSaveData` with quests/encounters/switches/flags.
- `scalar.rs` — small Ambition-specific scalar helpers such as `approach()`.
- `state_machines.rs` — seldom_state markers (`EnemyIdle`, `EncounterActive`, `BossDormant`, etc.).
- `world.rs` — reusable world/block data structures and collision query entry points.

The old public `build_endgame_sandbox()` hard-coded room builder has been removed. Room layout now comes from LDtk + RON data in the sandbox crate, and engine tests use small explicit fixture worlds that describe only the geometry needed by each class of test.
