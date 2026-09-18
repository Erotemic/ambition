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

- [`architecture-census.md`](architecture-census.md) is the human current-state map.
- [`consolidation-ledger.json`](consolidation-ledger.json) is the stable-ID machine ledger. Update an item instead of adding a second prose account of the same fact.
- [`consolidation-plan.md`](consolidation-plan.md) ranks removal and ownership work by architectural leverage.
- [`campaign-metrics.md`](campaign-metrics.md) records repeatable baseline counts.
- `scripts/architecture_census.py` refreshes static package, LOC, optional-resource, editor-domain, and ledger measurements. ⭐ **RUN IT RATHER THAN WRITING YOUR OWN SCAN** — a hand-rolled one overstated the optional-resource population by 17% on 2026-09-16 by keeping inline `#[cfg(test)]` modules, which this helper strips.
- `scripts/multi_writer_resource_census.py` is the WRITER-side shortlist: which `Resource`s are written from more than one production file. ⚠ **NOT a finding list** — the discriminator is on the reader's side, and the script's own docstring carries two measured cases that came out opposite ways. `scripts/check_multi_writer_resources_are_adjudicated.py` ratchets the population on every `--maintenance` run so a new one cannot land unnoticed; **33 of the 121 carry a written verdict as of 2026-09-18 and it prints the other 88 as debt** rather than implying they were read — a count derived by the run itself, not by this page, so re-derive it with `python3 scripts/check_multi_writer_resources_are_adjudicated.py` instead of quoting this sentence. ⚠ **THE 8-OF-102 THIS LINE USED TO CARRY WAS TRUE ON 2026-09-17 AND WRONG BY THE NEXT AFTERNOON IN BOTH HALVES** — the population moved 102 → 121 when the census learned the `ResMut<'w, T>` and `world.resource_mut::<T>()` spellings and stopped counting whole test files, and the verdicts moved 8 → 33 by being written, and since 2026-09-17 the guard also refuses a verdict whose subject is no longer a multi-writer type, because that subtraction is what the debt line asserts. ⛔⛔ **THAT POPULATION WENT 85 -> 82 -> 83 -> 102 IN ONE DAY AND EVERY MOVE WAS AN INSTRUMENT DEFECT, NOT A TREE CHANGE** — see [campaign-metrics.md](campaign-metrics.md) for the four counting rules and their measured deltas. The largest: the census read `ResMut<T>` and not `ResMut<'w, T>` — how every `#[derive(SystemParam)]` bundle spells it, and a bundle is exactly the shape a resource takes when several systems share one accessor — nor `world.resource_mut::<T>()`, so it was also blind to the systems that SPEND the state it was auditing — a commit executor is an exclusive-world system, and `rollback_ggrs/lifecycle_commit.rs` and `session/reset/mod.rs` between them clear or reach `PendingLifecycleCommit`, `RoomTransitionLoadState`, `AmbitionGameSave`, `AuthoredOccurrences`, `QuestRegistry` and `GameplayBanner`. ⭐ **AND IT NOW CARRIES A SECOND POPULATION**: session world components reached through `SessionWorldMut<T>`, which A10 moved OUT of the resource space — none has `#[derive(Resource)]`, so the resource census was silent about them by construction while four have more than one writer and `EncounterMusicRequest` has eight. Ratcheted separately, because a session boundary reclaims them and a resource's lifetime is the App's. ⭐ **WHERE TO SPEND THE NEXT ONE IS MEASURED: THE GUARD PRINTS THE SHORTLIST ITSELF NOW, so this page carries no copy of it** — its last output line names the multi-writer types that are also rollback-registered and still carry no verdict, which is where a second writer is a DIVERGENCE rather than a design smell. ⛔⛤ It was prose in two places and went stale FIVE TIMES on 2026-09-18 alone (*"21 of the 102"*, then 31, eight, seven, six, five, four) while the intersection itself never moved once — the number counts VERDICTS, and verdicts are what this campaign spends, so the only stable place for it is the run. ✅ **THAT QUEUE WAS SPENT ON 2026-09-18**: every rollback-registered multi-writer type the parse can see now carries a verdict, ten of them written that day. ⚠ SPENT IS NOT CLEAN — the parse is a LOWER BOUND (turbofish registrations only, ~396 names against 491 registry rows), so a row registered through any other form was never in the queue; and the unadjudicated majority that remains is simply not rollback-registered, which is a weaker reason to read it rather than no reason. ⇒ The next populations with a stated discriminator are the SESSION WORLD COMPONENTS (four multi-writer of eight carrying a `SessionWorldMut<T>` accessor, ratcheted separately) and the durable-save writers (re-derived 2026-09-18 with the snippet in the guard's docstring; this said *"18 of the 85"*, a ratio about a smaller population and a blinder census), and joining their writers' WRITE TARGETS narrows it again — `--shared-targets` prints which field or method two of a type's writer files both reach for, splitting the files that TOUCH a target from the subset whose access is mutation-shaped, because a call through a `ResMut` binding may read or write and no regex can tell. ⛔⛤ **THAT MODE EXISTS BECAUSE THE HAND-WRITTEN TABLE IT REPLACED WAS WRONG IN FOUR PLACES, FOUND 2026-09-17 WHILE CITING IT** — its column was a per-target count wearing the word *writers*, so `AmbitionGameSave 14 writers` sat beside a baseline of 17; `OwnedItems 4 -> grant()` is 2 of 10; and *nothing shared* was true of one of the three types it claimed, when both of `VersusMatch`'s writers replace the whole resource with `*state = ..`. A table nobody can re-derive with one command is a table that goes stale between the paragraph and the reader. ⭐ **AND A VERDICT COSTS A POISON, NOT A READING** — `ClockState` was adjudicated on 2026-09-17 by breaking the coupling its three writers depend on (`apply_suspended_time_scale_system` zeroes the REQUESTED scale as well as the live one, so the smoother cannot ramp back up) and watching `suspended_frame_zeros_world_time_scaled_dt` go red. An entry without that step is an opinion in a machine-readable list.
  ⛔⛤ **AND THE POISON CAN COME BACK GREEN, WHICH IS A FINDING ABOUT THE ARMS RATHER THAN A CLEAN RESULT.** Adjudicating `SlotInteractionState` on 2026-09-17 meant deleting the `slot_gestures.primary_mut().clear()` that `detect_room_transition_system` calls *consuming the gesture* — and **1,207 crate arms and all 11 room-transition integration arms stayed green**, because every authored door arm HOLDS interact for thirty frames and the producer refills the buffer underneath the clear. The verdict cost a new arm (`a_door_crossing_consumes_the_buffered_press_rather_than_letting_it_decay`, a single TAP plus an out-of-zone control) before it could be written down. ⇒ Budget an adjudication as *one poison, and a witness if the poison passes*.
  ⛔⛤ **BOTH OF ITS NUMBERS WERE WRONG UNTIL 2026-09-17, IN OPPOSITE DIRECTIONS.** It counted whole test FILES as writers (no `#[cfg(test)]` line to cut at) and it threw away everything after a file's FIRST `#[cfg(test)]` — which in this tree is usually a `mod tests;` declaration near the top, so 40 files were hiding 77 `ResMut<T>` occurrences of real production code. 83 → 75 → **85**: a census can be stale in both directions at once, and the net looked like a small drift.

⚠ **THOSE WERE BACKTICKS AND NOT LINKS UNTIL 2026-09-16**, so `consolidation-plan.md` and `campaign-metrics.md` were reachable from NO document in the repository — the whole campaign tree was navigable only by knowing the filenames. A page nobody can reach is the dual of a pointer that lands nowhere, and nothing checked for it.

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
| C03 starts from 32 session-owned App resources | **37** — `SessionScopedResources` holds 30, not 25 (it read 29, then 30; re-derived 2026-09-17) |
| C03 can lift out "reset-only process storage" | **no such member exists**; all 30 have a reader outside their reset |
| C03 has "separate reset lists" to merge | the two lists' intersection is **EMPTY** — a partition, not two copies |
| C04: a live-construction fallback is an "accidental missing-resource branch" | it is a **DECLARED** decision — `for_live_session` REFUSES a shell-routed session with no generation, discriminated by `SessionGatedSimulation` |
| C08: "facade crates" hold compatibility re-exports | it is **ONE** crate — `ambition_platformer2d` holds 168 of the workspace's 300 cross-crate `pub use` statements |
| C09: the largest package has 104,962 nonblank Rust lines | **108,646** — and it is 60,228 SRC + 48,418 TESTS; second place is 76% tests |
| C07 counts 732 optional Res/ResMut accesses over 196 spellings | **820 / 206 (2026-09-17)** — and the movement is the INSTRUMENT, not the tree. ⛔ This cell said `726 / 196 — essentially FLAT` and before that claimed +16%; the census was cutting every file from its first `#[cfg(test)]` to the end, which in this tree is usually production code, so it was hiding 89 occurrences. The hand scan this cell overruled was closer to the population than the tool was; its error was comparing against a baseline built by a different rule |
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
(`game/ambition_app/tests/an_edit_reaches_the_shipped_game.rs:1404`) boots
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
