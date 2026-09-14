# Architecture consolidation census

This directory is the current architecture consolidation map for Ambition.
It answers one question: which independent truths exist now, and which of them can later become one owner plus projections?

The census baseline was read against `662a9b56096a`. Planning-control status was
refreshed against `2dbd81abc50f` during the documentation consolidation; the
architecture census itself was not rerun. It did not compile or run Rust. The
generated `.agent` inventory is used for counts and navigation. Source and active
planning own semantic claims.

## What this is

- `architecture-census.md` is the human current-state map.
- `consolidation-ledger.json` is the stable-ID machine ledger. Update an item instead of adding a second prose account of the same fact.
- `consolidation-plan.md` ranks removal and ownership work by architectural leverage.
- `campaign-metrics.md` records repeatable baseline counts.
- `scripts/architecture_census.py` refreshes static package, LOC, optional-resource, editor-domain, and ledger measurements.

This census does not set complexity limits. A count can rise when a valid capability is added. The counts exist so a later campaign can state what changed.

## Evidence words

- `SOURCE_CONFIRMED`: explicit source establishes the claim.
- `SOURCE_INFERRED`: source shape supports the claim, but static inspection does not prove the full execution path.
- `DOC_CLAIM`: a current planning or architecture document states the claim, and this census did not prove it from source.
- `NEEDS_COMPILED_VERIFICATION`: Rust type checking, macro expansion, trait resolution, or resolved features are needed.
- `NEEDS_RUNTIME_VERIFICATION`: schedule execution, rollback behavior, two-App behavior, or another runtime fact is needed.

The complexity classes are `ESSENTIAL_COMPLEXITY`, `ACCIDENTAL_COMPLEXITY`, `TRANSITIONAL_COMPLEXITY`, and `UNCERTAIN`.
They describe the reason for a mechanism. They are not scores.

## Active work boundary

A10 is active work at this snapshot. ⚠ **THE LINE HERE SAID `ROOM_CANDIDATE_BRACKET = false` AND THAT WAS TRUE UNTIL 2026-09-14.**
It is `true` now: the normal room road builds every root hidden, stages the whole world replacement, verifies a projected
post-publication roster, and publishes or drops. The ROOM switch IS the normal road; the SESSION switch is not.
See `docs/planning/queue.md`'s A10 row for what each half covers.
Do not use this census as a second A10 implementation plan.

The peer-stable identity work is also a separate active campaign. This census maps the local and canonical identities but does not change them.
The shell/content activation gate is atomic at this snapshot. Its A-supersedes-B hold race still needs its named witness.
Mechanical edit admission is established as an implementation foundation: six production domains use the shared proposal, admission, and publication protocol.

## Refresh rule

Run:

```bash
python3 scripts/architecture_census.py
python3 scripts/architecture_census.py --crate-table
python3 scripts/architecture_census.py --json > /tmp/architecture-census.json
```

Then re-read the source behind any semantic ledger item that changed.
Do not replace a source conclusion with a text-pattern count.
Keep current state here. Keep investigation history in Git.
