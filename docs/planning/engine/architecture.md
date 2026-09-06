# Engine architecture — moved to durable architecture

**Status:** DISTILLED 2026-08-30.

The canonical current engine architecture now lives at
[`../../architecture/engine-architecture.md`](../../architecture/engine-architecture.md).

This planning-path receipt remains temporarily because policy metadata, code
comments, ADRs, and historical documents still link here. New architecture
references should use the durable document directly.

> ⭐ **AND "TEMPORARILY" NOW HAS A NUMBER AND AN EXIT CONDITION** (measured
> 2026-09-03, four days on). **8 live files** still point here, plus 7 archived
> ones that should never be repointed because an archive records what a document
> said at the time:
>
> | citer | rows | can it move? |
> |---|---|---|
> | `tests/ambition_workspace_policy/` — `policies/{engine,game,repository}.toml`, `src/custom/session_world.rs`, `tests/policy.rs` | 5 files, **16 citations** | yes — these are `reason`/doc fields naming where a rule is argued |
> | `scripts/check_agent_kb.py`, `dev/journals/code_smells.md`, `docs/planning/yardrat-open-measurements.md` | 3 files | yes |
>
> ⇒ **So the receipt is genuinely load-bearing, and the workspace policy engine
> is why** — `policies/engine.toml` alone cites this path 10 times. ⇒ The exit
> condition is therefore concrete rather than a feeling: repoint those 16
> citations at
> [`../../architecture/engine-architecture.md`](../../architecture/engine-architecture.md)
> and this file can be deleted. ⚠ Nobody should do that as a sed. A policy
> `reason` names the document that ARGUES the rule, and whether each argument
> actually survived the distillation is a per-row question — which is the work,
> not the repointing.
>
> ✔ **And the scope is bounded, which is the good news.** Every documentation
> path cited anywhere under `tests/ambition_workspace_policy/` resolves — **262
> citations across 13 distinct documents, 0 unresolved** — and of those 13, this
> receipt is the **only** one that is a receipt. The other twelve are live
> documents that still argue what they are cited for. ⇒ So these 16 rows are not
> the visible part of a larger rot; they are the whole of it.

Forward architecture gaps belong in:

- [`../status.md`](../status.md) for current orientation;
- [`../roadmap.md`](../roadmap.md) for strategic order;
- [`../queue.md`](../queue.md) for executable work;
- [`../tracks.md`](../tracks.md) for deferred work;
- focused plans in this directory for unresolved design.

Do not add new architectural doctrine to this receipt.
