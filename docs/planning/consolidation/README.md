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

Post-A10 demolition CLOSED on 2026-09-16 on both the symbol and public-surface
axes, with a mostly negative result (see the A10 row). It is no longer a lane.
Use this census to find roads whose replacement now exists, not as an
implementation plan.

The peer-stable identity work is also a separate active campaign. This census maps the local and canonical identities but does not change them.
The shell/content activation gate is atomic at this snapshot. Its A-supersedes-B
hold race is **HALF WITNESSED**, MEASURED 2026-09-16 — and this paragraph is a
CORRECTION of one written earlier the same day, which said the gate was entirely
open. It was not. See the ⛔ at the end for how that happened, because the method
error matters more than the verdict.

⭐ **THE SHELL/SESSION HALF HAS ITS NAMED PRODUCTION WITNESS, AND IT PASSES.**
`a_candidate_session_replaced_while_pending_is_discarded`
(`game/ambition_app/tests/an_edit_reaches_the_shipped_game.rs:1320`) boots
`build_visible_app` — the shipped visible composition, not a hand-built router —
issues `ShellCommand::ReplaceWith` for `ambition_gameplay`, waits THREE frames so
the second request arrives while the first is still PENDING, and asserts the
superseded candidate is DISCARDED: no candidate gate registration outlives its
candidate, the route holds are released, and
`session_root_for_scope(SessionScopeId(1))` finds the live session — which is
itself the premise that the two requests actually overlapped, because the scope
allocator is sequential. `the_shipped_app_never_holds_two_session_roots_across_a_handoff`
(`:435`) drives the same road and counts roots every frame. MEASURED at `c78cc725e` on
this box: both pass.

⛔ **THE CONTENT/TRANSACTION HALF DOES NOT.** What the three unit witnesses pin
is the TRANSACTION: that a retry supersedes the failed transaction and the stale
one cannot publish
(`provider_retry_supersedes_the_failed_transaction_and_rejects_stale_publication`,
`crates/ambition_game_shell/src/tests.rs:918`), that the superseded transaction
NAMES the request it cancelled (`:1350`), and that a superseded load cannot
authorize a commit (`superseded_load_cannot_authorize_commit`,
`crates/ambition_load/src/tests.rs:131`). All three build a
`ShellRouter::default()` by hand and register their own catalog. MEASURED: NO
`app_it` arm reads `ShellEvent::PreparationRequested` or a transaction's
`barrier.load_id` at all — the three `ShellEvent::` mentions in the whole
integration suite are in comments. ⇒ The composed host is never asked whether a
superseded transaction can still publish.

⇒ **SO THE REMAINING GAP IS ONE ARM AND IT IS SMALLER THAN THE ROW SAID.** The
session lifecycle under supersession is covered in production; the transaction
identity under supersession is covered only in a unit fixture. The missing arm
re-requests the live route in `build_visible_app` and asserts the FIRST
transaction cannot publish afterwards — the unit witnesses say exactly what to
assert.

⛔⛤ **HOW THE EARLIER PARAGRAPH GOT IT WRONG, because the method is the
transferable part.** I listed the `app_it` files matching `supersed` and
`ShellActivationId`, listed the arms issuing `ShellCommand::GoTo`, and concluded
"no `app_it` arm launches the same route twice without quitting" WITHOUT OPENING
THEM. The arm that does it uses `ReplaceWith`, not `GoTo`, and its file matched
my grep the whole time. ⚠ I had written the rule that same afternoon — a number
counting mentions is VIGILANCE, not safety, so do not report one you have not
opened — and then reported a NEGATIVE from the same kind of scan. A negative grep
is a claim about the QUERY.
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
