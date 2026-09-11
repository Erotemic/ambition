# Planning status

Live execution is in [queue.md](queue.md). Product rulings remain in
[maintainer-decisions.md](maintainer-decisions.md); unanswered choices remain in
[awaiting-maintainer-decision.md](awaiting-maintainer-decision.md).

**Architecture review baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`,
2026-09-08 committed source archive. Source inspection and Python architecture
checks were available; Rust compilation, gameplay, GPU and network execution
were not. The [coverage receipt](engine/architecture-review-coverage.md) separates
those evidence classes.

## Architecture posture

The [reassessment](engine/architecture-reassessment.md) replaces the mandatory
P2-P5 SCC sequence with ownership-based packets. Construction inversion,
backend-neutral rollback registration, published phase sets and the corrected
actor-spawn boundary remain supported. None establishes that all state, lifetime
or capability boundaries have converged.

The reusable substrate coexists with a distributed integration kernel across
actor-monolith, combat, characters, core and runtime. The
[responsibility map](engine/architecture-responsibility-map.md) distinguishes
logical authorities from present crate locations. A new crate is not an exit
criterion by itself.

## Next architectural action

⛔⛔ **THIS SECTION NAMED A1 UNTIL 2026-09-10, AND A1 WAS ALREADY FINISHED. IT SENT
THREE AGENTS AT COMPLETED WORK IN ONE DAY.**

**A1 is DONE.** Audited row by row, not inferred from the packets: the
[checkpoint protocol](engine/checkpoint-restoration-protocol.md)'s executable
acceptance matrix holds **18 rows, and all 18 carry a named witness** in the
audit table beneath it. A1a's admission latch, A1b's move of restoration into
session, and A1c's single immutable checkpoint are each closed there. The
frontier's readiness map marks A1a/b/c DONE. Subcommit 5's targets are gone —
`ResetToCheckpoint` has no raw restoration readers left, only writers and
registration.

⚠ **One item stays open and the protocol scopes it itself:** the terminal
outcome has no presentation consumer, *"when one is wanted"*. **That is a
deferral, not a defect, and it is not an architectural action.**

⇒ **THERE IS NO SINGLE NEXT ARCHITECTURAL ACTION NAMED HERE ANY MORE, and that is
deliberate.** A section that names one goes stale the moment that one lands, and
this one stayed wrong long enough to misdirect three agents on the day it was
noticed. **The open packets and their state are listed below; read those.**

⚠ **AND THE COUNT ABOVE IS 18, NOT 17.** The agent who audited the rows reported
17. Both a count of the matrix and a count of the audit table give 18. **The
audit's finding stands; its arithmetic did not, and a number that crosses a
document boundary travels with the method that produced it.**

The [projectile protocol](engine/projectile-contact-protocol.md) makes A2's
geometry sample, finite-shape obstruction, compound contacts and targeted delivery
explicit. A3 retains the valid world-to-construction lowering move. The
[authored-technique protocol](engine/authored-technique-admission.md) specifies
A11/A12's installed support, exhaustive reference validation, checked acyclic
flows, move-clock semantics and explicit prepared-revision activation.

⚠ **THIS PARAGRAPH SAID "these are implementation targets, not landed fixes"
UNTIL 2026-09-10, AND BY THEN IT WAS FALSE FOR THREE OF THE FOUR.** A protocol
document describing a target reads identically to one describing shipped
behaviour, so the distinction has to be maintained by hand here — and a blanket
sentence covering four packets goes stale the moment any one of them lands.
Stated per packet instead:

- **A2** — the RESOLVER half is met: one real swept projectile leg, exact
  `total_cmp` ordering, the selected world collider carried into physical
  response, and an identified victim ordered ahead of an unidentified one. ⛔
  **The CONSTRUCTION-IDENTITY row is NOT met and this line used to say
  otherwise.** The protocol calls a missing deterministic identity a
  construction failure rather than a sort fallback, and construction still
  permits one. ⚠ **The specific example a review supplied — the cut-rope victory
  NPC — was REFUTED**: it carries `FeatureId` + `BodyKinematics`, which is
  exactly the query `ensure_sim_id` serves, so it is identified a tick later.
  **What is open is the general invariant, not that case.** The COMPOUND SOLID
  row was deferred to Q96/A5 as a maintainer ruling rather than unfinished work.
  ⭐ **Q96 IS RULED 2026-09-10 — the compound contact**, so the row is now ordinary
  engineering work with contributor identity as its prerequisite.
- **A3** — done; the only residual is a file move that removes no edge.
- **A11** — **FOUR BLOCKERS CLOSED, AND A LANE HAS NOW SEEN THE FOURTH.** Green at
  `43059a46d`: 178 suites, 7714 passed, 0 failed, **78 Doc-tests phases** —
  **workspace test scope, doctests included, and nothing wider.** ⛔ That lane does
  NOT enforce `-D warnings`, does not cover the `--rust` set, and compiles nothing
  behind a **non-default** `cfg(feature = ...)`; `check_no_warnings.py`'s own note
  records three warnings that once lived in exactly that gap while its line read
  clean. ⇒ **A11 is CLOSABLE on that scope, not closed on a wider one.**
  ⭐ **It is the first run of the whole tree with the raw finalizer ABSENT** — the
  poison proved the boundary in isolation; this proved nothing else depended on it.
  ⛔⛤ **AND THAT SAME TREE HAD A RED GATE WHILE THE LANE WAS GREEN.**
  `check_absence_contracts.py` reports
  `capability-footprint-sentinel-lockfile-is-stale` — **1 of 39 contracts
  violated at `43059a46d`**, caused by that commit's `Cargo.toml` edit, because
  the sentinel keeps its OWN lockfile in `fixtures/minimal_game` that a workspace
  build never touches. ⇒ **The verdict is not wrong; its SCOPE is why.** *"Nothing
  wider"* is the sentence that keeps it honest, and an unqualified *"green at
  `43059a46d`"* would now be false.
  ⛔ **TWO STRUCTURAL DEFECTS MADE IT UNDISCOVERABLE**, both verified 2026-09-10:
  the checker **exits 0 over a violation** (`--check` is opt-in — *"exit 1 when a
  contract is violated"*), and **no lane invokes it at all** — 24 doc mentions,
  zero invocations outside its own tests. ⇒ **A 39-contract checker that no lane
  runs, whose default cannot fail, was red and nothing would have said so.**
  ⚠ Wiring it into a lane is a gate change and is **not made here**.

  ⛔⛤ **RE-DERIVED 2026-09-11 AND THE SECOND HALF OF THAT SENTENCE IS FALSE.**
  *"Zero invocations outside its own tests"* is literally true and its implication
  is not: **its own tests ARE a lane.**
  `scripts/tests/test_absence_contracts.py` parametrises all three pattern
  families over the LIVE tree (`test_every_contract_holds_against_the_live_tree`,
  `test_every_dependency_contract_holds_against_the_live_workspace`,
  `test_every_module_allowlist_holds_against_the_live_tree`) and carries separate
  live-tree tests for both baseline-backed contracts and for the footprint
  ratchet — and `scripts/tests` IS the `repo tooling (scripts/tests; repo-coupled)`
  job, which the default lane and `--rust` both run.
  ⭐ **The specific red is gated too.**
  `test_the_footprint_ratchet_holds_against_the_live_tree` calls
  `capability_footprint_violations`, which reaches `sentinel_linked_closure`,
  which RAISES `SentinelLockfileStale` on a stale sentinel lockfile — an uncaught
  raise, so the test errors and the job is red. **That test and that raise were
  both present at `43059a46d`** (checked with `git show`), so the pytest lane
  would have reddened there.
  ⇒ **The true sentence is narrower: the CLI's `--check` EXIT CODE runs in no lane;
  the CONTRACTS run in two.** What made `43059a46d` look clean is that the run
  stamping it green was a workspace CARGO lane, which executes no pytest at all —
  the lane-population lesson, not a missing gate.
  ⚠ **So the four questions above measure a gate that mostly exists.** Wiring the
  CLI is still open and still a gate change; MEASURED 2026-09-11, it would buy the
  **two** verdicts the CLI computes that no pytest test asserts —
  `the-featureless-facade-links-none-of-these` against the live tree (its three
  tests patch either the forbidden set or the closure) and
  `capability-footprint-baseline-is-stale` (the DEPARTURE direction) — not the
  39-contract surface.

⭐⭐ **MEASURED FOR WHOEVER DECIDES (`d398bcafc`), FOUR QUESTIONS, NO
RECOMMENDATION:**

1. **`--check` passes at HEAD — and only since one commit ago.** `exit 0`,
   39 of 39 hold. **It was red until `3275ba09c`.** ⇒ A gate proposal whose
   subject went green an hour ago is a proposal about a lane **that would have
   been red today.**
2. **9 seconds.** Cheaper than the citation lane, cheaper than one crate's tests,
   for the whole 39-contract surface.
3. ⭐⭐ **YES, IT WOULD HAVE CAUGHT TODAY'S RED AT INTRODUCTION**, and the diffs
   show the mechanism: `43059a46d` moved `ron` out of `[dependencies]` and left
   `fixtures/minimal_game/Cargo.lock` untouched; `3275ba09c` deleted exactly that
   one lockfile line. **The repair is the one line the break should have carried.**
   ⛔ **It went unnoticed for 11 commits / ~56 minutes — and the workspace lane
   stamped `43059a46d` GREEN inside that window** (178 suites, 7,714 passed).
   ⇒ **A 9-second check would have caught, at introduction, a defect a 20-minute
   workspace lane certified as clean.**
4. **All 39 print a plain `ok`; none qualifies its own pass.** ⚠ **STATED AS THE
   WEAK NEGATIVE IT IS:** that is ONE run at ONE commit, with no history of reds
   to sample. **A contract that is FLAKY rather than wrong looks identical to a
   clean one in a single green run.** *"No noisy contract in one observation"* is
   the finding; *"no noisy contracts"* is not.

⚠ **The Q3 argument is STRONG AND NARROW.** One example settles that this
contract, broken this way, would have been caught. **It says nothing about the
other 38, and a gate is justified by its whole surface, not its best case.**
Measured by ToothbrushAmbition; no recommendation made and nothing wired. ⚠ This line
  said *"FULLY re-closed"* after three, and **a completeness word is a hostage to
  the next review** — the fourth arrived hours later. ✅ **Closed in `95f0c1484`
  with a PROVEN boundary rather than an asserted one**: `cargo check -p ambition_app`
  ships **without** the function, `--workspace --all-targets` reaches all ten test
  callers, and a poisoned non-test call fails with `error[E0425]: cannot find
  function`. ⭐ **Passing the two checks showed only that nothing broke; the poison
  showed something is now impossible.** ⛔ **But that commit carries NO workspace
  test-scope claim** — two `cargo check`s and one poison, no suite. **A `Cargo.toml`
  change plus a `cfg` gate is exactly the shape that compiles everywhere and breaks
  a runtime path**, so this is not closed until a lane says so. It was true
  of the three blockers then known and became false the moment a fourth was found —
  `close_preparation_barrier_without_admission` WAS plain `pub`
  (`crates/ambition_characters/src/prepared.rs:2278`), whose path is
  `finalized = true` → mark unchecked → raw `finalize_cast` → publish with no
  refusal filtering, and once it runs the checked closer sees `finalized` and cannot
  correct the publication. ⚠ All ten current callers are tests, so this is a
  CONTRACT violation rather than a live defect — but the page's own trust boundary
  forbids "a second public path to activating unvalidated authored data". ⚠ A
  smaller item rides with it: an empty-support refusal sets
  `closed_with_admission = false` while `barrier_closed_with_admission` documents
  `false` as an UNCHECKED close, so the witness misreports the case that fix made
  legitimate. **Nothing in production reads that flag** — its only consumers are two
  lines of a test — so it misleads a reader, not the engine. Reopened 2026-09-10 by
  a review of the 125
  commits following `6a692b6`. ⛔⛤ **The shipped game had been closing its
  admission barrier UNCHECKED**: Bevy's runner does `finish()` before the first
  `update()`, `PreStartup` lives inside that update, and the unchecked backstop
  therefore won in every app reaching `App::run` — while every guard agreed,
  because they all drive `update()` by hand, which never runs `finish`. Fixed by
  ONE DECLARED AUTHORITY, and the witness that says so differs from its green
  sibling by three calls. **Transitive withholding is fixed too**: the one-pass
  filter published a summoner whose beast it had just refused.
  ✅ **CLOSED 2026-09-10 by `4fe4a1a27`, and this line said
  otherwise for half a day.** `admit_and_finalize_cast` now takes
  `support: &TechniqueSupport`, **not `Option<&_>`** — the comment on that
  parameter says *"NOT `Option`, AND THAT WAS THE LAST A11 BLOCKER."*
  `TechniqueSupport::admit_at` opens with `self.offers.get(&effect.key)` and
  returns `TechniqueRefusal::Unknown` on a miss, so an EMPTY table refuses every
  native effect. ⇒ **A composition that installs no technique handlers does
  not have an UNKNOWN support set. It has the EMPTY one.** Both production
  empty-table roads are deliberate and documented: the `finish()` backstop, which
  stands down on `ChecksAuthoredEffectsAtTheBarrier`, and the public
  `close_preparation_barrier` for a host that installs none.
  ⚠ **ALL THREE A11 BLOCKERS ARE CLOSED. A11 HAS NO OPEN BLOCKER.**
- **A12** — **PROPAGATION LANDED (`f9baa86e8`), THEN IDENTITY (`106c349b5`).
  ⛔⛤ THIS LINE SAID *"TWO BLOCKERS REMAIN OPEN"* AND THE TRUE COUNT WAS FOUR.**
  An outside review of 52 commits found two more, both of them roads the
  identity fix did not reach, and a reader of this row would have sized the
  remaining work at half of what it was. ⭐ **The correction is not that the
  number was stale — it is that a count of blockers is a claim about a
  POPULATION, and nothing re-derived the population when `106c349b5` changed
  what "fixed" meant.** ⇒ The four, and where each stands:

  | # | blocker | state |
  |---|---|---|
  | 1 | the boss start road inserted `MovePlayback::new_at` directly, never joining the body-owned mint | ✅ **CLOSED `051e95972`** |
  | 2 | provenance died on the `Effect → Special → projectile` road | ✅ **CLOSED `2ba3b6700`** |
  | 3 | reflection re-owns a shot and leaves `FiredByMoveInstance` untouched | ✅ **CLOSED `a7e6bcbf6`** |
  | 4 | `None` still credits the move playing at the time | ✅ **CLOSED `fa86d3c41`** |

  ⚠ **3 AND 4 WERE ONE QUESTION WEARING TWO FACES, AND THE ORDER MATTERED.**
  Clearing the stamp on reflection produces exactly the state blocker 4
  mishandled, so the clear had to come SECOND:

  | | stale stamp | cleared to `None` |
  |---|---|---|
  | `is_none_or` (before) | credited **iff the numbers collide** | credited **always** |
  | `== Some(pb.instance)` | collides (unchanged) | **nobody credited** ✅ |

  ⛔ **Clear-first converts an intermittent false credit into a guaranteed one.**
  Two readers — mine and a peer's — argued for clear-first from the same wrong
  premise, that the flip loosened `Some`. It never did: `is_none_or` already
  compared `Some(x)` with `==`, so the flip changes the `None` case ALONE. ⭐ The
  four-cell table settled in one reading what neither of us had settled by
  re-reading the code.

  ⭐⭐ **Q101 IS WHAT CLOSED 4, AND IT IS A RULING, NOT A REPAIR.** *"`Connected`
  is a fact owned by a specific move occurrence. `None` means no move claims the
  outcome, and therefore it cannot modify ANY move's `Connected` state."*
  Crediting *"whoever is playing right now"* is not an authority — it attributes
  by coincidence of timing. ⇒ Every ability becomes a LOCAL question: propagate
  the occurrence and the hit satisfies the move; propagate nothing and it
  satisfies nothing. The runtime learns about no ability.

  ⚠ **AND BLOCKER 3 WAS AN ATTRIBUTION QUESTION, NOT A GAMEPLAY ONE.** It was
  filed as needing a per-ability ruling. `MoveOccurrence` counts per entity from
  0, so a reflected shot's stale stamp COLLIDES with the new owner's own
  occurrence — most loudly at 0, which both bodies hold on their first use. No
  predicate can repair that; the number alone cannot say whose it is.

  ⭐ **Blocker 2's shape is the reusable part.** The bridge that dropped the
  occurrence sat beside the branch that carried it, in the same `match`, and the
  doc comment above the predicate *described the carrying branch as if it were
  the subject*. A correct sentence with an unstated scope closed the question
  for a reader standing on either road. **Both witnesses now name their road.**
  This line read *"NOT fixed"*, then *"LANDED"*, and both were wrong. What landed
  closes the DIRECT-REPLACEMENT case, where `succeeding(Some(prev))` genuinely
  increments. ✅ **IDENTITY FIXED IN `106c349b5`**: `MoveOccurrence(u32)` now lives
  on the BODY, advances at `start_move`, and is NEVER REMOVED, so an idle gap keeps
  its count. `MovePlayback::instance` copies it, and `succeeding()`/`replacing` were
  deleted in the same change. ⚠ It WAS a within-chain ordinal — `new_at` set `0` and
  `succeeding(None)` mapped back to `0` while the playback is removed when a move
  ends, so a move starting after an idle gap carried `0` again and took credit for
  a shot stamped `0` by an earlier one. ⭐ The same field stamped MELEE strike
  volumes, so the two roads had one defect between them. ⚠ Concretely reachable: `officer_the_draw`
  fires at 0.348s and ends at 0.696s while its projectile lives up to 2.4s.
  ⛔ **Reflection** re-owns a shot to the interceptor (`intercept.rs:82`) and
  leaves `FiredByMoveInstance` untouched, pairing a new owner with the old
  shooter's stamp. ⛔ **`None` still credits the current move**, so clearing the
  stamp on reflection is the same defect with an extra step. A12b's
  prepared-revision items remain open and always said so. A verdict carrying
  `attacker_move_instance: None` WAS credited to whatever move the fighter was
  playing at the time, so a projectile launched by move A and landing during
  move B marked B connected — the late-feedback defect A12 exists to eliminate. The
  predicate's own comment defends `None` for hazards and contact attrition and
  never addresses the projectile. **Both halves were needed and both
  landed:** the instance now travels `MovePlayback::instance` →
  `MoveEventMessage::move_instance` → `RangedCommitment::CommittedMove`
  → `ProjectileSpawnRequest` → `FiredByMoveInstance` on the shot
  → the damage result, and the
  predicate then requires the claim. ⚠ **The value is ABSENT, not zero**,
  for a shot no move fired — a `0` would name a first use that never
  played. `GGRS_ROLLBACK_SCHEMA_VERSION` 178 → 179.
- **A7** — RECLASSIFIED from an occurrence-ownership completion to a
  COMPONENT-CONSTRUCTION SEAL. `GroundItem` is sealed and that is real; *"seven
  minting authorities became one"* is not, and callers still mint identity,
  custody, provenance and attempt state themselves. ⇒ Calling a centralized
  component constructor does not transfer authority over the OCCURRENCE.
  Q97's policy half is still Jon's.
- **A9** — open, and its next question is which of the 49 crates a minimum
  profile has a RIGHT to expect for the crates the frontier does not name.

The frontier records prerequisites and holds; the queue selects priority rather
than a predicted SCC trajectory.

## Measured shape, not architectural acceptance

⚠ **RE-DERIVED IN FULL 2026-09-10 at `09467966b`.** Every number below had been
stamped to `300004d6` for 240+ commits, under a sentence telling readers to
reproduce it. The old values are kept beside the new ones: **they were true when
written, and overwriting an accurate record destroys the evidence that it moved.**

| measure | `300004d6` | `09467966b` | |
|---|---|---|---|
| module SCC size | 9 | **10** | ⛔ **+1, and membership moved** |
| second SCC | assets/character_sprites | assets/character_sprites | unchanged |
| workspace packages | 79 | **79** | unchanged |
| physical Rust lines under `src/` | 679,785 | **694,104** | +14,319 |
| …in the actor monolith | 98,464 | **104,545** | +6,081 |
| capability private orderings | 0 | **10** | ⚠ unit differs — see below |
| composition private orderings | 73 | **67** | −6 |
| foreign system installations | 174 | **213** | +39 |
| reducible installation blocks | 3 | **1** | ⭐ cause known |
| irreducible installation blocks | 38 | **38** | unchanged |
| facade non-optional parentage | 51 | **48** | re-derived `939d6aaa5`, holds here |

⛔⛔ **THE SCC IS TEN MODULES NOW, AND ONE LEFT WHILE TWO JOINED.**

    was   abilities, construction, control, features, items, projectile, session, shrine, world      (9)
    now   abilities, avatar, character_runtime, construction, control, features, items,
          projectile, session, world                                                                 (10)
    ⇒ OUT: shrine     IN: avatar, character_runtime

⇒ Reproduce with **`uv run --script scripts/measure_kernel_module_graph.py --scc`**.

⛔⛔ **AND THE FIRST VERSION OF THIS ROW SAID *"still nine, two members changed"*,
WHICH WAS WRONG, BECAUSE I DERIVED THE SCC FROM THE TOOL'S DISPLAY INSTEAD OF
ASKING THE TOOL.** Two compounding errors, both mine:

* **The instrument HAS an `--scc` flag.** I ran it with no arguments, saw no
  components, and wrote into this page that it *"does not report SCCs"*. ⇒ **Ran
  it one way and concluded it cannot do the thing.**
* **The printed table truncates.** Out-edges are `most_common(8)` per module, so
  the edge that keeps `projectile` in the cycle is not shown. My hand-rolled
  Tarjan over that table dropped `projectile` and produced a nine-module answer
  that looked like a confirmation of the stale number.

⚠ **THE TRUNCATION MADE THE WRONG ANSWER MORE PERSUASIVE, NOT LESS.** A derived
9 matching the published 9 read as *"the count held, the membership moved"* — a
tidy finding. **The real change is a count that moved, which the page's own
number would have flagged to anyone who re-ran the instrument properly.**

⭐ The membership finding survives and is the part worth keeping: **`shrine` left
and `avatar` and `character_runtime` joined.** But the headline is now *"nine
became ten"*, and a reader checking only the number **would** have caught it.

⚠ `scripts/module_graph.py` also has an SCC layer and needs `uv run --script`
(it fails under the tool venv with `ModuleNotFoundError: networkx`) — but it
condenses over **1,606 file-level modules**, not the kernel's 38, so it is **not
the instrument for this number.** Two tools, two granularities, one word.

⭐ **REDUCIBLE 3 → 1 HAS A NAMEABLE CAUSE**, which is worth more than the fresh
number: `61f297edf` — *"the latch drain installs itself; the other reducible block
is declined, with the reason"*.

⭐⭐ **THE ORDERING ROW IS RECONCILED, AND IT IS NOT A REGRESSION — IT IS MOSTLY
THE INSTRUMENT.** Resolved 2026-09-10 by running the BASELINE instrument (from
`300004d6`) against today's tree, which separates a change in the code from a
change in the measure:

| bucket | page @`300004d6` | baseline instr @today | current instr @today | tree effect | instrument effect |
|---|---|---|---|---|---|
| capability | 0 | 2 | **10** | +2 | **+8** |
| composition | 73 | 58 | **67** | **−15** | +9 |
| installing | 174 | 168 | **213** | −6 | **+45** |

⇒ **THE TREE IMPROVED WHERE THE PAGE APPEARS TO SHOW IT WORSENING.** Composition
orderings fell **73 → 58** on the same instrument. The published number moves only
73 → 67 because the instrument simultaneously got stricter and added nine back.
**A reader comparing the two published numbers sees −6 and would never learn that
the code closed fifteen.**

⛔ **AND `0 → 10` IS FOUR-FIFTHS INSTRUMENT.** Two of the ten are real
capability-written edges that did not exist at the baseline; eight are edges the
baseline instrument could not see. **It is not a regression and must not be
quoted as one.**

⚠ **WHY THE INSTRUMENT MOVED, from its own history:** `5f3f75508` keyed the
classifier on the SITE and made the ceilings count EDGES; `6db1dd495` found *"the
census counted an import style, not an architecture"*; and the current version
counts every foreign system named anywhere inside `add_systems(...)`, including
CHAINED TUPLES that a `.before(` regex misses entirely. The baseline classifier
has no `is_binary_root` at all, and reports raw occurrences where the current one
reports unique edges (77 edges across 93 occurrences).

⛔ **THE WORD *"private"* IS NOT THE INSTRUMENT'S.** It appears once, in the
script's docstring prose, and never in its output — the labels are *"written by a
capability / ruleset"* and *"written by a composition layer"*. This page inherited
a vocabulary the tool does not use.

⚠ **THE DECOMPOSITION IS APPROXIMATE AND THE MIDDLE COLUMN NEVER EXISTED.** The
baseline instrument against today's tree is a hypothetical run: no one ever took
that measurement at the time. It separates the two effects well enough to retire
"regression", not well enough to attribute single edges.

⇒ Reproduce, and name the unit every time:

```bash
cargo metadata --no-deps --format-version 1     # 79 packages; lines counted under each package's src/
uv run --script scripts/measure_kernel_module_graph.py --scc   # THE SCC; --scc is required
python3 scripts/measure_foreign_system_ordering.py       # ordering edges / installations
python3 scripts/measure_carveable_installations.py       # reducible / irreducible
cargo tree -e normal --no-default-features -p ambition_platformer2d   # 49 names, 48 others
```

The module-path instrument excludes several test forms and is a textual
heuristic, not a Rust semantic dependency graph.

The facade's nonoptional internal dependency traversal reaches 51 other workspace
packages even without selecting optional dependencies. Render is reachable through
host. That is a source-only lower bound, not Cargo's resolved feature closure,
link size, memory consumption or a runtime measurement. The SDK needs independently
verified minimal profiles rather than an opt-out claim based only on plugin flags.

⚠ **RE-MEASURED 2026-09-10: it is 48, at `939d6aaa5`.** The 51 is the
`300004d601af1e633cfaee969f079cf9bb368ca8` baseline. Three edges closed
between the two, all on 2026-09-09: the render path through host, five dead
dependency declarations, and the map capability. ⇒ Reproduce with
`cargo tree -e normal --no-default-features -p ambition_platformer2d`, count the
unique `ambition_*` names (49) and subtract the facade itself (48).
⛔ **THE UNIT IS THE TRAP.** 49 counts the facade, 48 does not, and this page's
51 is an *other-packages* count. A number that cannot say which it is cannot be
quoted. See `scripts/measure_minimum_profile_parentage.py`.

## Correctness and validation front

[Findings F1-F9](engine/architecture-review-findings.md) distinguish conditional
startup retry loss, divergent boss/projectile geometry, obstruction ordering,
inert authored fields, nonoptional render reachability, construction publication
limits, unwired technique validation, flow admission bounds and raw checkpoint
reset mutation outside room admission. Each has a counterexample or verification
task and a responsible packet. Do not represent them as reproduced Rust failures
from this review.

[Repository history](repository-history.md) explains the intentional, repeatable
Git epochs and recovery from cold storage. A locally unresolved old commit is
an evidence-availability result, not proof that its history was lost. Remote
archive metadata was inspected; full reconstruction was not executed here.

The already-landed mark-attribution, mark-stock lifetime, fuse timing, mark-clock,
body-owned portal publication, hit-flash release, body-clock public set,
map-visited lifetime, installer preflight, mount/possession arbitration and Mary-O
render-basis repairs remain closed absent new contrary evidence. This review does
not reopen them from stale reports.

## Standing constraints

Body-owned presentation follows simulation/read model, finalized drawable,
portal publication and pane composition in that order. Performance claims require
the intended scenario, cache state and hardware. Quality tier, device residency,
authored world dimensions and trim placement remain distinct contracts.

Fighter-brain policy stays generic over an authored move menu. Game product choices
are not resolved by a package move. No recommendation in this review is a new
maintainer ruling. Completed work leaves the live queue; Git retains its history.
