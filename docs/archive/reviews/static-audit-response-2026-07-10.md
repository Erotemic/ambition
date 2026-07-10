# Static audit response and correction order — 2026-07-10

**Source audited:** `8622af7f`  
**Scope:** static review only; no Rust toolchain or Cargo execution was available.  
**Normative status:** this file records the investigation and agreed repair order. Live status remains in `docs/planning/tracks.md`; the relevant design contracts remain in their engine docs.

## Locked corrections

### H1 — required rooms can disappear from the gate

Use this exact framing:

> Required rooms can disappear from the gate; loaded rooms do not silently pass incorrect canary/replay results.

The canary and replay checks still assert the expected tick count and hash equality once a room loads. Their defect is that a fixture-load failure becomes a skip/return. The debt ledgers are more exposed: if all required rooms fail to load, their measured peak can collapse to zero. All required fixture construction must therefore return `Result` and be asserted with room-specific context.

### N3.1 — valuable keystone, incomplete exactness

Use this exact framing:

> The N3.1 keystone exists and is valuable; the surrounding prose overstates what that keystone proves, while the exactness work is properly N3.2 and remains open.

N3.1 landed the registry, `SimId` vocabulary, shared snapshot/hash bytes, take/restore mechanics, coverage measurement, and a replay oracle. It did not establish exact rollback. Uniqueness, complete mutable-state coverage, codec failure semantics, room ownership, dynamic-spawn reconstruction, and bounded rollback remain N3.2 work.

### `portal_lab` — confirmed diagnosis (leak hypothesis SUPERSEDED)

> **Superseded 2026-07-10.** This section originally led with a cross-room *leak* /
> active-room-ownership *defect* as the diagnosis of `placement:NpcSpawn-0017`. That
> hypothesis was **traced and refuted**. The confirmed diagnosis is a **transition-spanning
> rollback window**, not a leak. The paragraphs below are rewritten to the confirmed
> finding; the original wording is preserved only in git history.

`placement:NpcSpawn-0017` is authored by `central_hub_main`, not by `portal_lab`; both
levels live in the same `sandbox.ldtk`. The ownership invariant was enforced per-tick
(`every_placement_entity_is_owned_by_the_active_room_every_tick`) and **it PASSES**: there
is no leak. The traversal policy bounces the player through a shared loading zone between
`portal_lab` and `central_hub_complex`; while `central_hub_complex` is active its
`NpcSpawn-0017` is legitimately alive, and it is despawned the instant the player
transitions away (confirmed by trace — it lives only while its room is active).

What makes the *rewind* dirty is separate: the rollback window SPANS a room transition. The
snapshot is taken while one room is active and `restore` runs while another is; the active
room was not restored sim state, so restore would reconcile against the wrong `RoomSpec`.
The fix that shipped is not to rebuild another room's entity against the active room, but to
**capture the active room in the snapshot and REFUSE a window that crosses it**
(`RestoreError::CrossRoomBoundary`, `restore_refuses_a_snapshot_that_spans_a_room_transition`).
Restoring only `RoomSet.active` + geometry, without the room-scoped entities/platforms/clocks
a transition rebuilds, would be *more* inconsistent than the refusal — so full atomic
room-context restore (what moves `portal_lab` from a REFUSED window to CLEAN) is named
remaining work, not this fix.

The per-tick invariant that holds:

> Every `placement:<iid>` in a room snapshot is authored by the active room, unless it is
> a dynamically-spawned child (no room authors it — an identity-vocabulary concern) or
> carried/persistent state (`slot:` — the player).

## Three-series execution order

### Series 1 — guardrail credibility and baseline normalization

Goal: establish that a green instrument means the intended fixtures ran and the intended debt was measured.

1. Make required-room construction a hard failure in the desync canary, replay oracle, SimId ledger, and snapshot coverage ledger.
2. Add the missing-room poison test in the same commit as that hard-failure behavior.
3. Add a coverage-sensitivity poison test using a fake mutable resource and message channel. The existing ledger machinery must observe the added debt, and the gate must react rather than preserve a count-only false green.
4. Expand N0.3's determinism scan to every simulation-bearing source root, including `game/ambition_content` and any demo rules that schedule simulation systems.
5. Reopen D-B. Regenerate stale `MODULES.md` files and correct the workspace count, but do not call that sufficient: add an executable module-size check.
6. Encode module-size exceptions as a **named waiver list with one reason per path** inside the enforcing script. Do not infer “generated” or “declarative” categories heuristically. Adding a new waiver must be a visible review event.
7. Land the pass-1 planning correction now: status labels and diagnoses must stop steering work incorrectly before implementation begins.

Series 1 must end green. It contains only poison tests for behavior that Series 1 itself enforces.

### Series 2 — N3.2 exact-restore substrate

Land these as small, bisectable commits in dependency order.

1. **Identity invariant.** Reject duplicate live `SimId`s, duplicate snapshot row identities, and duplicate registry names before building lookup maps. Report every collision with enough entity/archetype/entry context to diagnose it. Land the duplicate-`SimId` poison test in this same commit.
2. **Active-room ownership invariant.** Trace roster construction plus room transition/teardown, confirm the `portal_lab` mechanism, then enforce per-tick ownership of `placement:<iid>` rows. Explicitly modeled carried/persistent entities are the only exception.
3. **Reconciliation and stale-state accounting.** Reconcile first, then compute stale components and unidentified survivors against the post-reconciliation roster. Report restored, reconstructed, intentionally persistent, removed-as-stale, unidentified-survivor, and decode-failure outcomes separately.
4. **Codec failure semantics.** A registered component/resource/message codec failure is a restore error in every build mode. Land the corrupted-blob poison test in this same commit.
5. **Coverage and `lossless()` contract.** Define losslessness positively: unique identity, component coverage, mutable-resource coverage, relevant message/event coverage, successful decode, no unexplained survivors, no unaccounted stale state, and no naked reconstruction outside an explicit policy. Intentional exclusions must be named policy, not silence.
6. **Dynamic-spawn reconstruction and bounded rollback.** Add spawn recipes where exact reconstruction is required, or constrain the rollback window so it cannot span unsupported births. Then add presentation confirmation and side-effect deduplication.

### Series 3 — evidence ledger and planning normalization

This has two passes.

**Pass 1 — now:** correct every overclaimed status and the false `portal_lab` diagnosis. Keep normative status terse and link here for investigative history.

**Pass 2 — after Series 1 and Series 2 are green:** update completion evidence. A `COMPLETE` label must point to an enforced invariant or gate, not merely a diagnostic mechanism or a pinned count.

## Poison-test atomicity rule

A poison test and the invariant that makes it pass belong in the same commit unless the invariant already exists before the series starts. Do not:

- land a known-red test for behavior scheduled later;
- write a test that asserts the current incorrect behavior merely to keep the tree green;
- accept a red interval whose cause is “planned work” and therefore indistinguishable from an accidental regression.

The accepted placement is:

| Poison test | Series / commit |
|---|---|
| required room missing | Series 1, with hard-fail fixture construction |
| unregistered mutable resource/channel | Series 1, with coverage-sensitivity enforcement |
| duplicate `SimId` | Series 2 identity-invariant commit |
| corrupted snapshot blob | Series 2 codec-failure commit |

## Re-audit checkpoints

1. **Guardrail audit:** each poison test demonstrably detects its intended failure, required rooms cannot disappear, and the workspace/source scope is explicit.
2. **Substrate audit:** identity, ownership, reconciliation ordering, codec semantics, coverage, and losslessness are reviewed independently.
3. **Ledger audit:** every `COMPLETE` status points to enforced evidence; diagnostics and measurements are labeled as such.

---

## Execution evidence (Series 1 + Series 2, 2026-07-10)

> **First pass.** A second-pass re-audit (see [Re-audit response](#re-audit-response-second-pass-2026-07-10) below)
> refined several of these: `UnsupportedDynamicBirth` was renamed
> `UnsupportedDynamicReconstruction`, `lossless()` became argless and self-measured, the
> active room was added to the hash, and the identity roster and codec preflight were made
> robust. Where a name below differs from current code, the second-pass section is
> authoritative.

Executed by Opus 4.8. Every slice is one commit, green before commit, with its poison
test's red-before/green-after demonstration recorded in the commit message.

### The gate, run end to end (exact commands, all green)

```
cargo test -p ambition_actors --lib                         → 744 passed
cargo test -p ambition_engine_core -p ambition_runtime \
  -p ambition_host -p ambition_dialog -p ambition_sim_view \
  -p ambition_combat -p ambition_characters                 → all segments ok (356/100/24/282/… )
cargo test -p ambition_content --features portal            → 101 (+3/5/4/3/1) passed
cargo test -p ambition_content --features ui --test yarn_compile → 1 passed
cargo test -p ambition_app --features rl_sim                → all ok (139 lib + 67 arch-boundary + desync_canary 15 + …)
```

### Poison tests — each fails for its intended reason, passes after its fix

| Poison test | Commit | Red-before demonstration |
|---|---|---|
| `a_missing_required_room_is_a_hard_failure_not_a_skip` | d9c151bc | fallback-detection disabled → FAILS ("a room no world authors must not pass try_sim") |
| `the_coverage_ledger_reacts_to_a_new_unregistered_resource` | 4ddfd4cd | debt-insertion removed → count assertion FAILS (left 181, right 182) |
| N0.3 widened scan | 705b6695 | caught 2 real `falling_sand.rs` rule-3 violations |
| D-B `no_production_module_exceeds_the_size_limit_unwaived` | f2034a23 | a disabled waiver → FAILS naming `falling_sand.rs: 1588 lines` |
| `restore_refuses_a_world_with_two_entities_of_one_identity` | 3bedea4b | identity assert neutered → FAILS ("must refuse a duplicated identity") |
| `restore_refuses_a_snapshot_that_spans_a_room_transition` | 25ae1d3c | boundary check disabled → FAILS ("reconciled a cross-room snapshot") |
| `stale_state_is_measured_after_reconciliation_not_before` | 2fea4942 | measured before despawn → FAILS (despawned ghost's component leaks) |
| `restore_refuses_a_corrupted_blob_rather_than_leaving_stale_state` | 86ae239a | decode check disabled → FAILS (returns `Ok(RestoreReport{patched:2,…})`) |
| `the_sim_resource_universe_excludes_presentation_but_keeps_sim_state` | 17247e02 | teeth both ways: drops `sim_view`/`ldtk_map`, keeps `ClockState` |
| `restore_refuses_a_dynamically_spawned_entity_born_inside_the_window` | c44c4ebd | reject disabled → FAILS (returns `Ok(RestoreReport{respawned:1,…})`) |

### Mapping to the re-audit checkpoints

- **Guardrail audit:** required rooms hard-fail (silent room-fallback caught too); N0.3 scans content + demo rules with a non-vacuous self-check; the coverage ledger reacts to added debt; D-B has an executable line gate + bidirectional reasoned waiver list.
- **Substrate audit:** identity (`duplicate_live_ids` + panic), room ownership (per-tick invariant + `CrossRoomBoundary`), reconciliation ordering (stale computed last, H4), codec semantics (`DecodeFailed` in every build), coverage/`lossless()` (positive contract + named `SIM_RESOURCE_EXCLUSIONS`), and the dynamic-birth window bound (`UnsupportedDynamicBirth`) each have an independent enforced test.
- **Ledger audit:** `tracks.md` N0.3/N0.4 = LANDED, N3.2 substrate = LANDED with remainder named; `portal_lab` corrected to a transition-spanning window (not a leak).

### Genuinely remaining (named, not hidden)

- **Full atomic room-context restore** (entities + platforms + clocks) — what moves `portal_lab` from a REFUSED window to CLEAN. Restoring only `RoomSet.active`+geometry would be *worse* than the refusal (reviewer).
- **Per-spawner reconstruction recipes** — no suite room spawns AND kills a dynamic child inside a window, so the recipe path is unexercised; the `UnsupportedDynamicReconstruction` refusal is the honest bound until then. It is ONE reconstruction refusal, not a general bounded-rollback-window guarantee.
- **Boss-hand identity** — `giant_gnu_hand_{left,right}_7` get a `FeatureId`, so `ensure_sim_id` promotes them into the `placement:` namespace with a non-deterministic name; they should mint `SimId::spawned(parent, counter)`.
- **Resolved-codec decode failure is indistinguishable from authored absence** — `SnapshotResolve::resolve` returns `None` for both, so a resolved codec cannot be decode-preflighted and its failure can still leave the world partially restored. The corrupted-blob test covers the ordinary component/resource codec, which IS transactional.
- **The N3.2 codec relocation** (move `SnapshotState` down so each crate owns its codec) and the `snapshot.rs`/`moveset.rs` god-module splits (audit M9) remain D-B waivers.

---

## Re-audit response (second pass, 2026-07-10)

The re-auditor accepted the first-pass list-as-landed but withheld "full guardrail + N3.2
substrate green," naming six correctness gaps the tests did not expose. All six are fixed;
each is one bisectable commit, green before commit, with a red-before demonstration in the
message. Executed by Opus 4.8.

| # | Re-audit finding | Fix | Commit |
|---|---|---|---|
| 1 | N0.3 manifest-dependency scan was **vacuous** — `root/crates/<full-path>` opened zero manifests, passed green | `root.join(krate)`; PANIC on an unreadable listed manifest; assert one manifest read per listed crate | `1dfcad98` |
| 2 | Active room **captured but not hashed** — snapshot held state the N0.4 hash omitted | fold active room into `hash_world` + `hash_by_entry` + `size_bytes`; poison test on real rooms | `2bc0a662` |
| 3 | `duplicate_ids()` scanned **one component entry** — blind to a collision across disjoint/zero components; `take` never rejected live dups | full `roster` of every live `SimId`; `take` enforces uniqueness; `restore` validates the snapshot roster independently | `a26a681a` |
| 4 | `UnsupportedDynamicBirth` named the **wrong temporal event** (a death/reconstruction, not a birth) | renamed `UnsupportedDynamicReconstruction`; reworded to a single reconstruction refusal, NOT a general window bound | `e06cfea7` |
| 5 | Unsupported-dynamic + decode errors returned **after mutating** the world | dynamic refusal preflighted before any despawn; standalone (component/resource) codecs decode-preflighted before mutation; cursor/resolved apply-time path named as residual | `e06cfea7` + `e4eb4b55` |
| 6 | `lossless()` unreliable: caller-supplied count, spurious `0` under no-debug-names, message channels counted as false debt, broad `ldtk_map::` exclusion, silent cursor success | restore MEASURES the resource term; argless `lossless()` requires `resource_census_reliable`; message-channel `Messages<M>` TypeIds claimed; ldtk exclusions per-type; cursor-unresolved counted | `80186af6` |
| doc | The `portal_lab` "corrected diagnosis" still led with the leak hypothesis | section marked SUPERSEDED and rewritten to the confirmed transition-spanning diagnosis | this doc |

**Red-before demonstrations** (each is in its commit message):

- (1) old join path → 0 manifests read → the new `assert_eq!(manifests_read, len)` FAILS.
- (2) omit the active-room fold → the poison test's `before == after` → FAILS.
- (3) per-entry `duplicate_ids` → a collision sharing no component row returns empty → the surfacing test FAILS.
- (4/5) inline (post-despawn) refusal → the future-only canary entity is despawned before the refusal → the "world untouched" assertion FAILS.
- (5) decode check at apply time → the corrupted standalone blob despawns the future entity before `DecodeFailed` → FAILS.
- (6) caller-supplied `lossless(0)` / no census flag → a report with resource debt reports lossless under no-debug-names → the census assertion FAILS.

**Revised status after the second pass:** N0.3 = LANDED (dependency scan no longer vacuous);
N3.2 substrate advanced — identity/ownership/reconciliation/room-boundary LANDED; codec
semantics transactional for the ordinary path with the resolved path a named residual;
losslessness self-measured and reliable; dynamic reconstruction a single enforced refusal,
not a general window bound. The full atomic room restore, per-spawner recipes, boss-hand
identity, and the resolved-codec Result contract remain named above.
