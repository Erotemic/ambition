# Triage — diagnosed, not yet scheduled

Each page here is one finding that was measured and written down, and that nobody
has scheduled. They are not proposals and they are not a queue: a page arrives
when the diagnosis is complete and the decision about whether to act is somebody
else's, or when the cost of acting is not yet known.

A page here can become stale. Before you act on it, read its status line and
date, and re-derive its measurement; see
[`../../recipes/re-measuring-a-planning-claim.md`](../../recipes/re-measuring-a-planning-claim.md).

## Open findings

- [`a-composition-acceptance-that-only-fails-in-company.md`](a-composition-acceptance-that-only-fails-in-company.md)
  — an isolation test that fails intermittently in a full run and passes alone.
  It is a class with at least two instances. The next step is
  `--test-threads=1`; the `TEST-LANES` row in [`../queue.md`](../queue.md) links
  it.
- [`a-prose-path-inside-a-doc-comment-is-not-checked.md`](a-prose-path-inside-a-doc-comment-is-not-checked.md)
  — paths, quotations and bare names in source comments that no check resolves.
  The path half is closed (`check_planning_citations.py --comment-paths`
  reports in the default lane). The page keeps the measurements for the
  quotation, vanished-name and stranded-doc-comment sweeps, which are opt-in and
  not wired as gates.

## Design and scope pages awaiting a decision

- [`ambition-registry-core.md`](ambition-registry-core.md) — registry protocol,
  identity, and the deliberate-duplicate policy.
- [`ambition-test-support.md`](ambition-test-support.md) — sequester harness
  boilerplate so behavioural tests are cheap to write.
- [`bevy-system-parameter-architecture.md`](bevy-system-parameter-architecture.md)
  — system parameters and mutation boundaries.
- [`declared-id-resolution-checks.md`](declared-id-resolution-checks.md) —
  remaining authoring diagnostics for declared-id resolution.
- [`gameplay-presentation-profiles.md`](gameplay-presentation-profiles.md) —
  remaining presentation-profile work.
- [`stable-identifier-centralization.md`](stable-identifier-centralization.md) —
  semantic scope before shared syntax. ⚠ `tracks.md` gates this on *"concrete
  identity families that share actual operations"*.
- [`unused-dependency-census.md`](unused-dependency-census.md) — a
  compiler-verified census of unused dependency declarations.

## Deferred on an external or product condition

- [`character-dialogue-from-suggestions.md`](character-dialogue-from-suggestions.md)
  — SHELVED 2026-07-26 pending a design settle.
- [`leafwing-clash-scan-patch-2026-07-23.md`](leafwing-clash-scan-patch-2026-07-23.md)
  — a deferred UPSTREAM patch. `tracks.md` gates it on a dependency/version
  change or a measured CPU cost that matters to a declared profile.
