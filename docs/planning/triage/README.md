# Triage — diagnosed, not yet scheduled

Each page here is one diagnosed finding that nobody has scheduled. A page is not
a proposal and not a queue. Each page states:

- **Finding:** what is true and why it matters.
- **Evidence command:** how to re-measure it.
- **Owner:** the queue row, track or document that would take it, or "none".
- **Trigger to promote:** the condition that moves it to `queue.md` or
  `tracks.md`.

A page can go stale. Before you act on it, re-run its evidence command; see
[`../../recipes/re-measuring-a-planning-claim.md`](../../recipes/re-measuring-a-planning-claim.md).
When a finding is fixed or superseded, delete its page and check for inbound
references first (source comments and workspace policies cite some of these
pages).

## Open findings

- [`a-composition-acceptance-that-only-fails-in-company.md`](a-composition-acceptance-that-only-fails-in-company.md)
  — an `app_it` arm that failed once in a full run and passes alone. Owner:
  `TEST-LANES` in [`../queue.md`](../queue.md).
- [`gameplay-presentation-profiles.md`](gameplay-presentation-profiles.md) —
  `DisplaySafeAreaInsets` has no producer, so every layout uses zero insets.
- [`character-dialogue-from-suggestions.md`](character-dialogue-from-suggestions.md)
  — *interact* reaches a placeholder node while barks use the character's own
  lines. Design settled (2026-07-26); nothing built.
- [`unused-dependency-census.md`](unused-dependency-census.md) — the
  compiler-based method for finding unused dependency declarations, and its
  blind spots. Re-run after a carve.

## Design and scope pages

- [`ambition-registry-core.md`](ambition-registry-core.md) — registry
  protocol, identity and duplicate policy. Workspace policies cite it.
- [`ambition-test-support.md`](ambition-test-support.md) — a shared layer for
  test harness boilerplate. The dependency layering is not decided.
- [`stable-identifier-centralization.md`](stable-identifier-centralization.md)
  — semantic scope before shared id syntax. `tracks.md` gates it on concrete
  identity families that share actual operations.

## Deferred on an external condition

- [`leafwing-clash-scan-patch-2026-07-23.md`](leafwing-clash-scan-patch-2026-07-23.md)
  — a deferred upstream patch. `tracks.md` gates it on a dependency version
  change or a measured CPU cost that matters to a declared profile.
