# Decomposition doctrine — durable authority moved

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
