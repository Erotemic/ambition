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

### `portal_lab` — corrected diagnosis

`placement:NpcSpawn-0017` is authored by `central_hub_main`, not by `portal_lab`. Both levels live in the same `sandbox.ldtk` and one sim loads that world. Its appearance in a `portal_lab` snapshot is therefore evidence of a cross-room roster or active-room-ownership defect, not evidence that `RoomSpec` needs a fourth authored-list arm.

Do **not** teach `respawn_authored_entity` to rebuild another room's entity against the active room. First trace why the entity remains snapshot-eligible across room behavior or transition. The highest-value probes are:

1. room transition and teardown;
2. per-tick snapshot-roster construction;
3. initial roster construction.

The first hypothesis to confirm or refute is that `portal_lab` behavior over its 60-tick run causes a transition/despawn path to expose an entity leaked from `central_hub_main`. The invariant must ultimately hold at every captured tick, not merely immediately after room load:

> Every `placement:<iid>` in a room snapshot is authored by the active room, unless it is explicitly classified as carried/persistent state.

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
