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

⛔⛤ **RE-DERIVE A ROW BEFORE COSTING IT. THIS IS NOT ADVICE — IT IS THE RESULT OF
RE-DERIVING EIGHT OF THEM ON 2026-09-16 — SIX CAME BACK DIFFERENT, ONE HELD, AND ONE
(C07) ONLY *LOOKED* DIFFERENT UNTIL I USED THE REPOSITORY'S OWN INSTRUMENT.**

| what the row said | what source said |
| --- | --- |
| C03 starts from 32 session-owned App resources | **36** — `SessionScopedResources` is 29, not 25 |
| C03 can lift out "reset-only process storage" | **no such member exists**; all 29 have a reader outside their reset |
| C03 has "separate reset lists" to merge | the two lists' intersection is **EMPTY** — a partition, not two copies |
| C04: a live-construction fallback is an "accidental missing-resource branch" | it is a **DECLARED** decision — `for_live_session` REFUSES a shell-routed session with no generation, discriminated by `SessionGatedSimulation` |
| C08: "facade crates" hold compatibility re-exports | it is **ONE** crate — `ambition_platformer2d` holds 168 of the workspace's 300 cross-crate `pub use` statements |
| C09: the largest package has 104,962 nonblank Rust lines | **108,646** — and it is 60,228 SRC + 48,418 TESTS; second place is 76% tests |
| C07 counts 732 optional Res/ResMut accesses over 196 spellings | **726 / 196 today** — essentially FLAT. ⛔ This cell claimed +16% for an hour, from a hand-written scan that counted inline `#[cfg(test)]` modules; `scripts/architecture_census.py` is the instrument this row cites |
| C05: six values are separate queued writes | **five of six are Components on ONE entity from ONE lowering** (`PlatformerSessionWorld`, a Bundle on the session root); only `SessionMechanics` is an App global |

⚠ **THE DRIFT IS NOT NEGLECT, WHICH IS WHY IT WILL HAPPEN AGAIN.** Each of these
rows was true when written and the tree moved under it — C03's four extra
resources arrived because the peer-identity campaign made the `MatchInstance`
-stamped ones MEMBERS of that grouping rather than moving them, which is the
correct outcome and which C03 had no way to notice. A census pinned to a commit
describes that commit.

⭐ **AND C06 WAS SPOT-CHECKED THE SAME DAY AND HOLDS** — its materializer is
already one primitive, but the room-transition road really does keep its own
commit wrapper. That row is in this section deliberately: a run of stale rows
makes the next one look stale too, and that is how a CORRECT row gets rewritten.

⇒ So: a number in one of these rows is a MEASUREMENT WITH A DATE, not a fact.
Two of the four above are now guarded mechanically — the session-owner census
against `teardown.rs`, and every item's storage kind against its declaration —
and the rest are not. Re-derive, then cost.

The peer-stable identity work is also a separate active campaign. This census maps the local and canonical identities but does not change them.
The shell/content activation gate is atomic at this snapshot, and its
A-supersedes-B hold race is **WITNESSED IN PRODUCTION — the gate is CLOSED as of
2026-09-16.** ⚠ This paragraph has been wrong twice today in opposite directions;
the ⛔ at the end keeps both, because the method errors are the transferable part.

⭐ **THE SESSION HALF.** `a_candidate_session_replaced_while_pending_is_discarded`
(`game/ambition_app/tests/an_edit_reaches_the_shipped_game.rs:1320`) boots
`build_visible_app`, issues `ShellCommand::ReplaceWith` for `ambition_gameplay`,
waits THREE frames so the second request arrives while the first is still
PENDING, and asserts the superseded candidate is DISCARDED: no candidate gate
registration outlives its candidate, the route holds are released, and
`session_root_for_scope(SessionScopeId(1))` finds the live session — itself the
premise that the two requests overlapped, because the scope allocator is
sequential. `the_shipped_app_never_holds_two_session_roots_across_a_handoff`
(`:435`) drives the same road counting roots every frame.

⭐ **THE TRANSACTION HALF, ADDED 2026-09-16.**
`a_superseded_transaction_cannot_publish_in_the_shipped_app` drives the same
overlap and asserts what the three unit witnesses assert, against the composed
host: a SECOND and DIFFERENT transaction is minted (the premise — "cannot
publish" is otherwise satisfied by "never existed"), the first is ended and NAMED
with `TransactionEnd::Superseded`, and asking the production
`PreparedSessionRegistry` to publish it returns `None`.

⛔ **AND THE REFUSAL IS SPECIFIC, WHICH IS THE ONLY VERSION OF THAT CLAIM WORTH
ANYTHING.** POISONED: asking the SAME registry to publish the SUPERSEDING
transaction returns `Some(PreparedSessionIdentity { publication_id: 1, .. })`. So
the `None` is a refusal of supersession, not a registry that refuses everything —
which is exactly the failure mode an assertion on a lookup would have hidden. The
arm calls `publish`, not `prepared`, for that reason.

⚠ **WHAT IS STILL UNIT-ONLY, stated so nobody reads more into this than it
says:** the three router witnesses in `crates/ambition_game_shell/src/tests.rs`
and `crates/ambition_load/src/tests.rs` also pin the FAILED-retry road and the
load-commit authorization, and those two roads have no composed-host arm. The
gate this campaign named — an A superseded by a B while pending — does.

⛔⛤ **TWO METHOD ERRORS, BOTH MINE, BOTH ON THIS PARAGRAPH, IN ONE DAY.** First
it said the gate was entirely open: I listed the `app_it` files matching
`supersed` and `ShellActivationId`, listed the arms issuing `ShellCommand::GoTo`,
and concluded absence WITHOUT OPENING THEM — the arm that does it uses
`ReplaceWith`, and its file had matched my grep all along. Then it said only the
transaction half was missing, which was true and was fixable in one arm rather
than a campaign. ⚠ I had written the rule that same afternoon — a number counting
MENTIONS is vigilance, not safety, so do not report one you have not opened — and
then reported a NEGATIVE from the same kind of scan. **The rule about counts
applies to ABSENCES too, and absences are where it is hardest to notice: a
positive claim invites "which ones?", a negative one does not.** A negative grep
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
