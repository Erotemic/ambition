# Benchmark candidate index

Use this index before refactors or when planning validation. Benchmark candidates are distilled invariant traps from real Ambition mistakes.

## Rust module/refactor invariants

| Failure class | Read |
|---|---|
| Facade re-exports after splitting a large Rust module | [`rust-questions.md`](rust-questions.md) |
| Private child-module visibility and sibling helper access | [`rust-questions.md`](rust-questions.md), [`rust-module-split-subtle-review-question-2026-05-11.md`](rust-module-split-subtle-review-question-2026-05-11.md) |
| `pub use` of a `pub(crate)` item silently widens API surface (E0364) | [`rust-pub-use-pub-crate-mismatch-2026-05-19.md`](rust-pub-use-pub-crate-mismatch-2026-05-19.md) |
| Extension-trait imports and `Self: Sized` trait default methods | [`rust-questions.md`](rust-questions.md), [`movement-refactor-questions-2026-05-11.md`](movement-refactor-questions-2026-05-11.md) |
| Attributes/doc comments/derive macros drifting from extracted items | [`rust-questions.md`](rust-questions.md), [`rust-attribute-drift-resource-derive-question-2026-05-12.md`](rust-attribute-drift-resource-derive-question-2026-05-12.md) |
| `include_str!` / file-location invariants and test moves | [`rust-questions.md`](rust-questions.md), [`rl-sim-module-split-question-2026-05-11.md`](rl-sim-module-split-question-2026-05-11.md) |
| Multi-invariant module splits | [`compositions.md`](compositions.md) |

## Bevy / ECS / event invariants

| Failure class | Read |
|---|---|
| System tuple chain size/order and `.chain()` trait-bound failures | [`bevy-system-tuple-chain-limit-question-2026-05-12.md`](bevy-system-tuple-chain-limit-question-2026-05-12.md) |
| Removing sync systems leaves stale ECS components | [`bevy-ecs-stale-component-after-sync-removal-2026-05-15.md`](bevy-ecs-stale-component-after-sync-removal-2026-05-15.md) |
| Resource derives/attributes drift during extraction | [`rust-attribute-drift-resource-derive-question-2026-05-12.md`](rust-attribute-drift-resource-derive-question-2026-05-12.md) |
| Typed event/message API clobbered by stale overlays | [`overlay-stale-feature-events-api-question-2026-05-12.md`](overlay-stale-feature-events-api-question-2026-05-12.md) |
| Bevy feature graph transitively re-enables `bevy_window` / `bevy_winit` even after removing `default_app` from the base dep | [`bevy-feature-graph-headless-2026-05-20.md`](bevy-feature-graph-headless-2026-05-20.md) |
| Sandbox runtime mirror + engine state machine both own the same gameplay invariant; double-write yields one-frame off-by-ones | [`boss-runtime-mirror-vs-engine-state-2026-05-20.md`](boss-runtime-mirror-vs-engine-state-2026-05-20.md) |
| Deciding whether a `register_*_systems` helper moves to a domain module or stays in the app orchestrator | [`module-local-bevy-plugin-extraction-2026-05-20.md`](module-local-bevy-plugin-extraction-2026-05-20.md) |
| Per-player-component mirrors: only readers that run AFTER the sync system see this-frame data; mid-chain readers must stay on the source resource | [`per-player-component-mirror-schedule-boundary-2026-05-20.md`](per-player-component-mirror-schedule-boundary-2026-05-20.md) |

## Movement / collision invariants

| Failure class | Read |
|---|---|
| Edge-touch side contact misclassified as vertical landing | [`movement-edge-touch-y-sweep-question-2026-05-11.md`](movement-edge-touch-y-sweep-question-2026-05-11.md) |
| Collision refactor replaces guarded semantics with raw shape-cast normals | [`movement-refactor-questions-2026-05-11.md`](movement-refactor-questions-2026-05-11.md) |
| Grounded attack/pogo intent and intercept semantics | [`grounded-attack-intent-pogo-intercept-question-2026-05-13.md`](grounded-attack-intent-pogo-intercept-question-2026-05-13.md) |
| Runtime LDtk collision insertion and ledge snap world-bound validation | [`ldtk-runtime-collision-questions.md`](ldtk-runtime-collision-questions.md) |

## LDtk / assets / editor interop

| Failure class | Read |
|---|---|
| LDtk entity insertion does not acquire runtime collision | [`ldtk-runtime-collision-questions.md`](ldtk-runtime-collision-questions.md) |
| Sprite generator schema overlays clobber fields from earlier patches | [`sprite-generator-schema-questions.md`](sprite-generator-schema-questions.md) |
| Procedural audio debugging without listening | [`procedural-audio-questions.md`](procedural-audio-questions.md) |
| Music director module split needs item-complete extraction and re-export-visible helpers | [`music-director-module-split-question-2026-05-11.md`](music-director-module-split-question-2026-05-11.md) |

## UI / input / process

| Failure class | Read |
|---|---|
| Bevy UI helper extraction creates overlapping mutable borrows | [`ui-nav-refactor-questions.md`](ui-nav-refactor-questions.md) |
| UI label helper refactor breaks alignment gutters | [`ui-nav-test-questions.md`](ui-nav-test-questions.md) |
| Warning cleanup adds undeclared dependencies | [`warning-cleanup-questions.md`](warning-cleanup-questions.md) |
| Cargo test command grammar / single filter rule | [`cargo-test-single-filter-question-2026-05-11.md`](cargo-test-single-filter-question-2026-05-11.md) |
| Meta-process loop derails artifact delivery | [`meta-process-derailment-loop-tentative-2026-05-12.md`](meta-process-derailment-loop-tentative-2026-05-12.md) |

## Adding a benchmark candidate

Read [`README.md`](README.md) first. Tag by transferable invariant, not just surface subsystem. Add a row here when a new candidate becomes useful for future pre-flight checks.
