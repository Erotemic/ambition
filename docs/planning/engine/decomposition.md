# Decomposition doctrine — durable authority moved

> ⛔⛔ **DO NOT RETIRE THIS PAGE WITHOUT REPOINTING EIGHT POLICY ROWS FIRST — I
> deleted it on 2026-09-03 and my own guard caught it.** It has zero inbound
> links from `docs/planning` and its standing rule IS absorbed by
> [`../../architecture/package-and-capability-boundaries.md`](../../architecture/package-and-capability-boundaries.md),
> so both halves of the usual retirement test passed. ⚠ **The test misses a
> third reference class: `source_doc` fields in the workspace-policy TOMLs.**
> Eight rows cite this path — five in `engine.toml`, two in `module_size.toml`,
> one each in `game.toml` and `repository.toml` — and
> `every_source_doc_names_a_real_file_and_heading` went red in the feature-union
> job within the hour. ⇒ Before deleting any planning page, run
> `grep -rn "<path>" tests/ambition_workspace_policy/policies/*.toml` as well as
> the prose sweep. This is the same reason `engine/architecture.md` earns its
> keep, met a second time by a different road.


**State:** doctrine distilled; no independent execution queue lives here.

The durable package/dependency rules formerly maintained in this file now live
in
[`../../architecture/package-and-capability-boundaries.md`](../../architecture/package-and-capability-boundaries.md).

Use the focused plans for current work:

- [`actor-monolith-decomposition.md`](actor-monolith-decomposition.md) — the
  measured residual actor-kernel frontier;
- [`capability-and-runtime-composition.md`](capability-and-runtime-composition.md)
  — product/capability dependency closure;
- [`public-sdk-1.0.md`](public-sdk-1.0.md) — consumer-facing semantic API pressure.

Standing rule: **decompose by ownership and dependency value, not by line count.**
A carve should move one coherent authority with its registration and remove a
real dependency/change-fanout path, or provide another concrete isolation/SDK
benefit. Runtime performance is not assumed; measure it separately.

The previous execution ledger is available in git history. Do not rebuild that
ledger in live planning.
