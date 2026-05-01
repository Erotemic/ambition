# Engine architecture

`ambition_engine` is the reusable mechanics crate. It is allowed to depend on Bevy-adjacent crates such as `bevy_math` when those crates provide battle-tested primitives that Ambition should not maintain itself.

The current intent is:

- `ambition_engine` owns movement rules, collision semantics, abilities, combat hitboxes, enemy/test-target behavior, generated music/audio specs, and reusable mechanics.
- `ambition_sandbox` owns the Bevy app shell, runtime presentation, input wiring, debug tooling, and sandbox-specific RON data.
- Future story crates should look more like thin content crates: they select mechanics and provide room/story data without reimplementing player movement or collision details.

Important source modules:

- `abilities.rs` — ability flags and compatibility warnings.
- `movement.rs` — player movement, blink, dash, fly, wall, rebound, pogo, and symbolic operation traces.
- `world.rs` — reusable world/block data structures and collision query entry points.
- `geometry.rs` — Bevy `Aabb2d` helpers plus Ambition-specific strict overlap and Parry-backed shape casts.
- `combat.rs` — slash/pogo hitbox computation.
- `enemy.rs` — test dummy and knockback behavior.
- `music.rs` — data structures shared by generated audio/music authoring.
- `scalar.rs` — small Ambition-specific scalar helpers such as `approach()`.

The old public `build_endgame_sandbox()` hard-coded room builder has been removed. Room layout now comes from RON data in the sandbox crate, and engine tests use small explicit fixture worlds that describe only the geometry needed by each class of test.
