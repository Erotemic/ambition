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
| A test finds an entity by the TEXT it displays, two entities carry that label, and archetype order picks the answer — so an unrelated plugin becomes a "load-bearing dependency" a bisect confirms and no grep can explain | [`global-label-search-makes-content-text-an-identity-2026-08-08.md`](global-label-search-makes-content-text-an-identity-2026-08-08.md) |

## Movement / collision invariants

| Failure class | Read |
|---|---|
| Zero-speed surface rider at a polyline joint selects a non-supporting branch and loses jump/crouch/walk | [`surface-joint-zero-speed-support-2026-07-11.md`](surface-joint-zero-speed-support-2026-07-11.md) |
| Tangent-continuous route fork ignores Up/Down or repeats a loop because routing scores only the immediate tangent / cannot cross chains | [`surface-route-junction-lookahead-2026-07-12.md`](surface-route-junction-lookahead-2026-07-12.md) |
| Edge-touch side contact misclassified as vertical landing (Y-sweep) | [`movement-edge-touch-y-sweep-question-2026-05-11.md`](movement-edge-touch-y-sweep-question-2026-05-11.md) |
| Swept parallel-graze far-edge de-penetration: a body sliding under a wide ceiling teleports out its far X edge (X analog of the edge-touch bug; an `immediate_contact`-gated defer misses the non-immediate graze) | [`swept-parallel-graze-far-edge-depenetration-2026-06-04.md`](swept-parallel-graze-far-edge-depenetration-2026-06-04.md) |
| A guarantee documented on a tuning FIELD is enforced inside one arm of a policy switch, so an earlier arm skips it and the shared clamp reads the raw authored value | [`documented-guarantee-enforced-inside-one-branch-2026-08-08.md`](documented-guarantee-enforced-inside-one-branch-2026-08-08.md) |
| Collision refactor replaces guarded semantics with raw shape-cast normals | [`movement-refactor-questions-2026-05-11.md`](movement-refactor-questions-2026-05-11.md) |
| Grounded attack/pogo intent and intercept semantics | [`grounded-attack-intent-pogo-intercept-question-2026-05-13.md`](grounded-attack-intent-pogo-intercept-question-2026-05-13.md) |
| Runtime LDtk collision insertion and ledge snap world-bound validation | [`ldtk-runtime-collision-questions.md`](ldtk-runtime-collision-questions.md) |

## LDtk / assets / editor interop

| Failure class | Read |
|---|---|
| LDtk entity insertion does not acquire runtime collision | [`ldtk-runtime-collision-questions.md`](ldtk-runtime-collision-questions.md) |
| LDtk area-spec `world_x` drifts from the live LDtk; `--replace-existing` silently re-anchors at the stale coord | [`ldtk-area-spec-drift-2026-05-21.md`](ldtk-area-spec-drift-2026-05-21.md) |
| Sprite generator schema overlays clobber fields from earlier patches | [`sprite-generator-schema-questions.md`](sprite-generator-schema-questions.md) |
| Atlas builder reinvents addressing math the manifest already encodes (silent `from_name` filter + grid math ignoring inter-frame padding) | [`sprite-atlas-grid-math-vs-authoritative-rects-2026-05-23.md`](sprite-atlas-grid-math-vs-authoritative-rects-2026-05-23.md) |
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
| Autonomous long-run session vs interactive-default safety rules ("stop and ask") | [`autonomous-long-run-never-stop-2026-05-21.md`](autonomous-long-run-never-stop-2026-05-21.md) |
| One question validated at two sites: the earlier one short-circuits, so fixing the later one repairs nothing and leaves a comment claiming it did (a green unit test covered the unreachable copy) | [`one-question-two-checkers-only-the-first-runs-2026-08-08.md`](one-question-two-checkers-only-the-first-runs-2026-08-08.md) |
| A checker ENUMERATES with `git ls-files` and VALIDATES with a filesystem walk, so anything gitignored-but-present (a nested `.gitignore`, `.goal/*.json`, the guard's own fixtures) is silently outside the population it reports on | [`enumerate-one-way-validate-another-2026-08-08.md`](enumerate-one-way-validate-another-2026-08-08.md) |
| A measurement cache keyed by the INPUT spec cannot see a change to the RENDERER, so the run that was meant to verify the change reports `0 fresh, 782 cached` and republishes the old numbers without a word | [`cache-keyed-by-input-cannot-see-a-changed-transform-2026-08-08.md`](cache-keyed-by-input-cannot-see-a-changed-transform-2026-08-08.md) |
| A capability EXISTS and WORKS and nothing calls it, so "the engine cannot do X" is corrected to "it can" and the adoption gap is never counted — six in one session, each degrading silently because the fallback still produces a plausible answer, and one of them BROKEN with zero callers so fixing it changed no shipped behaviour at all | [`a-capability-with-no-adopters-2026-08-09.md`](a-capability-with-no-adopters-2026-08-09.md) |
| N sites answer one question, N-1 agree and often carry a comment saying why, and the ODD ONE OUT is the defect — eleven in one session, five with the rule written a few lines away, and three of them found in DATA (a spritesheet row, an LDtk level, an audio manifest) rather than in code | [`the-odd-one-out-among-siblings-2026-08-09.md`](the-odd-one-out-among-siblings-2026-08-09.md) |
| A comment asserts a behaviour the code does not have, was TRUE when written, and forecloses the check that would catch it — five in one session, the worst being *"falls through to the ordinary face resolution below"* on an `else if` arm, which left a game mechanic dead and sent its diagnosis into the sprite pipeline | [`the-comment-asserts-what-the-code-does-not-2026-08-09.md`](the-comment-asserts-what-the-code-does-not-2026-08-09.md) |
| Suppressing a behaviour by REVOKING its verb also deletes the on-screen button that verb declared: the gameplay gate has a compensating exception and the touch overlay does not, so the weapon fires on a desktop and is untappable on a phone. Arbitrate the RESOLUTION, never the DECLARATION | [`revoking-a-verb-also-deletes-its-touch-button-2026-08-09.md`](revoking-a-verb-also-deletes-its-touch-button-2026-08-09.md) |
| Two things CONSTRUCT one population — a constructor that is told a fact and a reset that RE-DERIVES it from a proxy — and they agree until the proxy stops being honest. A revived boss stood still with a correct brain because the reset computed `fly_enabled` from an ability toggle nobody has ever toggled. ⭐ the tell is free: *"leaving the room and re-entering fixes it"* means the OTHER constructor builds a different body | [`two-constructors-for-one-population-2026-08-14.md`](two-constructors-for-one-population-2026-08-14.md) |
| An absent component reads as "no value" to every consumer, so a NARROW population's fallback arms are silently also serving "not covered" — four consumers of one presented pose, three degraded and one that only looked correct because it never fired, which went live and WRONG on the commit that widened the population | [`an-absent-component-reads-as-no-value-2026-08-14.md`](an-absent-component-reads-as-no-value-2026-08-14.md) |

## Adding a benchmark candidate

Read [`README.md`](README.md) first. Tag by transferable invariant, not just surface subsystem. Add a row here when a new candidate becomes useful for future pre-flight checks.
