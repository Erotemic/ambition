# Architecture consolidation census

This directory is the current architecture consolidation map for Ambition.
It answers one question: which independent truths exist now, and which of them can later become one owner plus projections?

The census baseline was read against `662a9b56096a`. Planning-control status was
refreshed against `2dbd81abc50f` during the documentation consolidation; the
architecture census itself was not rerun.

⛔⛤ **A REFRESHED COUNT IS NOT A REFRESHED CLAIM, AND THE LEDGER NOW CARRIES THE
TWO SEPARATELY.** `static_measurement_refresh` records when the numbers were
re-measured (2026-09-16 at `09629b060f65`); `source_commit` still names the
commit the 114 SEMANTIC items were read at, and they have NOT been re-read.
Stamping one commit on the whole file would say the claims were re-verified
because the counts were.

⚠ **AND `generated_inventory_counts` HAS ITS OWN PROVENANCE AGAIN.** It is read
out of `.agent/`, which is regenerated separately and was 50 commits behind the
rest of the snapshot when this was found. Without `_generated_from_commit` beside
it a reader compares source-measured numbers from one commit against inventory
numbers from another and reads the difference as architecture. It did not compile or run Rust. The
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

**A10 is CLOSED (2026-09-15) at BOTH scopes.** The room road builds every root
hidden, stages the whole world replacement, verifies a projected post-publication
roster, and publishes or drops; the session road prepares a candidate session off
to the side and admits it through the shell gate. There is no mode flag — the
candidate-bracket selector is deleted, not frozen.

Post-A10 demolition is the active lane. Do not use this census as an
implementation plan for it; use it to find roads whose replacement now exists.

The peer-stable identity work is also a separate active campaign. This census maps the local and canonical identities but does not change them.
The shell/content activation gate is atomic at this snapshot. Its A-supersedes-B
hold race still needs its named witness — and MEASURED 2026-09-16, that blocker
is REAL rather than stale, which is worth stating because it gates C03 and C05.

⭐ **WHAT ALREADY EXISTS, so nobody re-derives it:** three supersession witnesses,
all in `crates/ambition_game_shell/src/tests.rs` and
`crates/ambition_load/src/tests.rs` —
`provider_retry_supersedes_the_failed_transaction_and_rejects_stale_publication`,
`a_superseded_transaction_names_the_request_it_cancelled`, and
`superseded_load_cannot_authorize_commit`. Together they pin that a retry
supersedes a failed transaction, that the superseded one NAMES the request it
cancelled, and that a superseded load cannot authorise a commit.

⛔ **WHAT IS MISSING IS THE WORD "PRODUCTION".** All three build a
`ShellRouter::default()` by hand and register their own catalog, so they witness
the ROUTER'S LOGIC rather than the shell/content activation gate in a composed
host. No `app_it` arm launches the same route twice without quitting — the shape
`a_re_requested_active_route_gets_a_fresh_transaction` exercises at unit level.
⇒ The gap is one integration arm, not a mechanism; the unit coverage says what
the arm should assert.
Mechanical edit admission is established as an implementation foundation: six production domains use the shared proposal, admission, and publication protocol.

## What is checked mechanically, and what is not

`scripts/check_consolidation_ledger_still_resolves.py` answers the two questions
about this ledger that a machine can answer: every cited `source_paths` /
evidence source is a file that exists, and every CamelCase name in a
`current_truth` sentence resolves to a definition in the tracked Rust sources.

MEASURED 2026-09-16, and the result is a NEGATIVE one worth recording: **114
items, 378 cited paths all exist, 63 names all resolve.** The ledger is not stale
by either mechanical measure, which bounds the worry that its items were read at
`662a9b56096a` and never re-read.

⚠ **AND THE FIRST RUN REPORTED FOUR UNRESOLVED NAMES, ALL FOUR FALSE.**
`ResMut` and `TypeId` are Bevy's and std's; `LoadId` is `ambition_load`'s and
`RunGgrsSystems` is `bevy_ggrs`'s, both live and used in dozens of places. A name
the check cannot resolve is a name whose definition it cannot SEE — a claim about
the scan's reach, not about the ledger. Triage every finding by hand.

⛔⛤ **NEITHER CHECK SAYS THE CLAIMS ARE STILL TRUE, AND A GREEN RUN MUST NOT BE
CITED AS A REFRESH.** A `current_truth` sentence can go completely stale while
every path exists and every type still compiles: the owner changes, a second
writer appears, a road is deleted and the sentence describing it survives. That
is the refresh rule below, and it needs a human reading the source behind the
item. ⇒ The check bounds the CHEAP failure and is silent about the expensive one.

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
