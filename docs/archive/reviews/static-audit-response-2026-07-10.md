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

> **First pass.** Two further re-audit passes refined these — see [second pass](#re-audit-response-second-pass-2026-07-10)
> and [third pass](#re-audit-response-third-pass-2026-07-10) below. Among the changes: the
> identity `roster` became AUTHORITATIVE and HASHED, a mutation-free `validate_snapshot` phase
> was added, cursor/resolved codecs report an `ApplyOutcome` (so `lossless()` denies unapplied
> rows), resource cursors gained a presence tag, the cross-room refusal compares the full
> `Option`, and the coverage ledger is pinned by TYPE NAME. Where a name below differs from
> current code, the later re-audit sections are authoritative.

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
- ~~**Boss-hand identity**~~ **RESOLVED 2026-07-11.** `giant_gnu_hand_{left,right}_7` no longer derive their id from `giant.index()` (an allocator slot). `spawn_giant_hand_limbs` derives the `FeatureId` from the giant's authored id and mints `SimId::spawned(giant_placement, ordinal)` at spawn, so each hand is a deterministic spawned child (`placement:<giant_iid>/<ordinal>`) and `ensure_sim_id` (`Without<SimId>`) skips it. Pinned by `giant_hand_identity_tests` in `spawn_actors.rs`.
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

## Re-audit response (third pass, 2026-07-10)

The third-pass re-audit accepted the six second-pass fixes as closed but withheld "N3.2
substrate LANDED," naming six further exactness gaps — most of them exposed by the new
identity roster the second pass had introduced but not yet made authoritative. All six are
fixed. The code (findings 1–6, runtime substrate + the app coverage-ledger gate — coupled
through the `CrossRoomBoundary` API change, so they land together to keep every commit green)
is one commit, `9e0365e5`, with a per-finding red-before demonstration in the message; this
documentation is a second. Executed by Opus 4.8.

| # | Third-pass finding | Fix |
|---|---|---|
| 1 | `roster` captured but restore and the hash still derived existence from component ROWS — a zero-component `SimId` entity was despawned if it survived, dropped if it died, and invisible to the hash | `sim_ids()` reads the authoritative `roster`; restore reconciles against it (no other change needed); `roster` added as a named hash pseudo-entry in `hash_world` + `hash_by_entry` |
| 2 | "Robust to a deserialized snapshot" overstated — `duplicate_ids` assumed a sorted roster; unknown entries and kind mismatches were silently skipped | order-robust `duplicate_ids` (sorts a clone); mutation-free `validate_snapshot` establishes canonical order, registry/kind agreement, unique rows, roster membership, and no-missing-entry BEFORE the first despawn → `RestoreError::MalformedSnapshot` |
| 3 | Cursor and resolved codecs returned bare `true` when they applied NOTHING (no live target / vanished content) → false `lossless()` | `insert` returns `ApplyOutcome::{Applied, DecodeFailed, Unapplied}`; restore counts `unapplied_rows`; `lossless()` denies any unapplied row |
| 4 | Resource cursor encoded absence and an empty cursor identically as `[]`; `CombatSlotsRes::apply_cursor` silently zipped mismatched shapes | presence-tagged resource-cursor blob (`Some`/`None`); apply reports `ApplyOutcome`; an absent-tagged resource is removed to match; a shape mismatch refuses loudly (`None` → `DecodeFailed`) |
| 5 | Cross-room refusal only fired when BOTH sides carried a `RoomSet` | `CrossRoomBoundary` carries `Option<String>`; restore compares the full options, so a `Some`/`None` presence mismatch is refused |
| 6 | Coverage gate was count-only (`resources.len() <= 181`, `worst <= 59`) — a substitution that holds the count constant passes | debt pinned by TYPE NAME against reviewed inventory files (`tests/known_{resource,component}_debt.txt`); a subset check makes a new type a review event; counts kept as summaries |

**Red-before demonstrations:**

- (1) restore off the component-derived set → the zero-component ghost is despawned as a future birth, and the hash does not move when it is added → `a_zero_component_sim_id_entity_is_covered_by_the_roster` FAILS.
- (2) `duplicate_ids` without the sort → the split `["dup", "other", "dup"]` roster returns empty → FAILS; a component blob placed under a resource entry is skipped → `restore_refuses_a_snapshot_with_a_kind_mismatched_entry` FAILS.
- (3) bare-`true` resolved insert → the dropped `Playing` row reports lossless → `a_resolved_component_that_names_missing_content_is_dropped_and_denies_lossless` FAILS.
- (4) untagged empty blob → an absent-at-snapshot resource survives, and a present-at-snapshot / absent-now cursor passes → `a_resource_cursor_tags_presence_and_reports_an_absent_target` FAILS.
- (5) both-`Some` guard → a `Some`/`None` presence mismatch is not refused (the runtime `Option` comparison is the fix; the existing `Some`/`Some` app test still refuses).
- (6) empty inventory → the current 177 resource / 70 component debt names are all "new" → the subset assertion FAILS. (This is literally how the reviewed inventories were captured — from the failing run's `--nocapture` dump.)

**Revised status after the third pass:** N0.3 = LANDED. N0.4 = LANDED as the canary mechanism
(the stale skip-path paragraph in netcode.md is corrected). Identity substrate = the roster is
now AUTHORITATIVE and HASHED, and `validate_snapshot` guards the deserialized path. Ordinary
codec preflight = LANDED. Cursor/resolved codec semantics = report an `ApplyOutcome` and deny
`lossless()` on any unapplied row; RESIDUAL: `SnapshotResolve::resolve` → `Result` to separate
a resolved decode failure from authored absence. Losslessness = self-measured, census-gated,
and denies unapplied rows / unresolved cursors. Room-boundary refusal = compares the full
`Option`. Dynamic reconstruction = one explicit refusal. **N3.2 overall is still OPEN**: the
full atomic room-context restore (→ `portal_lab` CLEAN), per-spawner reconstruction recipes,
the `resolve` → `Result` contract, and the boss-hand `SimId::spawned` identity remain named,
not hidden.

## Re-audit response (fourth pass, 2026-07-10)

The fourth pass accepted the six third-pass fixes and named three codec-completeness gaps —
cases where "successful decode" was not yet guaranteed merely because a `RestoreReport` exists.
All three fixed, each with a red-before poison test, in one commit.

| # | Fourth-pass finding | Fix |
|---|---|---|
| 1 | The resource-cursor ABSENCE path removed the resource and returned `Applied` without exhausting the reader, so `false` + trailing bytes was accepted; and `Reader::bool` treated every nonzero byte (e.g. tag `2`) as `true` | the absence path checks `r.finish()` before removing; `Reader::bool` decodes only canonical `0`/`1` (→ `a_resource_cursor_absence_blob_rejects_trailing_bytes_and_a_bad_tag`) |
| 2 | `register_resolved` treated `resolve → Some` as `Applied` without `r.finish()`, so a valid prefix + trailing garbage was applied and could report lossless — NOT covered by the `resolve → Result` residual | the insert closure (which holds the reader after `resolve` returns) checks `r.finish()`; trailing bytes are `DecodeFailed` (→ `a_resolved_blob_with_trailing_bytes_is_rejected`) |
| 3 | `validate_snapshot` accepted any PERMUTATION of entries, though restore iterates `snapshot.entries` directly — a resolved codec inspects other components, so a reorder could resolve before a dependency is restored | `validate_snapshot` requires the snapshot's entry order to match the registry's non-diagnostic order exactly (subsuming unknown/missing/duplicate) (→ `a_reordered_snapshot_is_rejected`) |

**Red-before demonstrations:**

- (1) no `finish()` on the absence path → `[false, 0xAB]` removes-and-succeeds; old `bool` → `[2]` is `true` and the present path passes → the test's `DecodeFailed` expectations FAIL.
- (2) no `finish()` after `resolve` → the boss's `playing` row with a trailing `0xFF` applies and returns `Ok` → the test's `DecodeFailed` expectation FAILS.
- (3) order-independent validation → a `swap(0, 1)` snapshot validates and restores → the test's `MalformedSnapshot` expectation FAILS.

**Revised status after the fourth pass:** "successful decode" is now guaranteed by a
`RestoreReport` existing — every codec path (plain, cursor, resolved, resource-cursor, and the
presence tag) either consumes its whole blob or returns `DecodeFailed`, and `validate_snapshot`
fixes the entry order restore depends on. The named residual narrows accordingly: `resolve →
Result` is now ONLY about distinguishing a resolved decode failure from authored absence (both
still map to `Unapplied`, which denies `lossless()`), not about trailing bytes. **N3.2 overall
remains OPEN** — the full atomic room-context restore, per-spawner recipes, the `resolve →
Result` contract, and the boss-hand `SimId::spawned` identity are unchanged from above.
