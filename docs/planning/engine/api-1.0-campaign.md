# API 1.0 campaign

**Status: slices A–G CLOSED; all three §4 conditions hold (2026-07-30). §4.2
regressed for exactly one commit when a new blind-run series first measured the
rollback surface, and the regression was the most useful thing here. Two
engine-work findings and slice H are carried out of the campaign — see
[§Slice G](#slice-g--closed-and-what-it-leaves-open).**

| §4 terminal condition | State |
|---|---|
| the allowlist ratchet is at ZERO | Outlander **0 of 18** · minimal_game **0** |
| a blind run opens NO engine file | Script A run 6: **zero** · Script B run 8: **zero** (run 7, the series' first: three) |
| every consumer-matrix category proven | **6 of 6**, each naming a test — the Smash row now states what its participants half does NOT prove |

**§4.2 went red because a new surface was measured for the first time, and that
is the gate working.** All six earlier runs used Script A, whose task is
standing a game up. Slice F published `ambition::rollback`, which Script A's
task cannot reach — a minimal game that boots never starts a session. So six
green runs said nothing about the newest public surface, and
[Script B](slice-evidence/blind-agent-runs/SCRIPT.md) was added as a NEW SERIES
rather than an edit, leaving runs 1–6 comparable.

Run 7's baseline was three engine files, which is what a first run of a new
series looks like — Script A's own first run opened eight. It also found five
reachable defects on the far side of `start()`, four of them silent, and one of
them ([finding a](slice-evidence/blind-agent-runs/2026-07-30-slice-g-run7.json))
is the sharpest thing this campaign has produced: a registration the SDK tells
you to make is *accepted, counted, and inert* on an entity your own game
spawned.

**Run 8 (Script B run 2) closed the regression: zero engine files.** The
single change that did it was documenting `require_rollback` at the point of
failure — the agent's verdict: *"that warning saved me; it is the single most
valuable paragraph in the SDK."* It also proved finding (a) closed with a
control run 7 could not make work: its own entity, spawned in `Startup`,
registered, held 1:1 with the frame count while the same binary with
registration toggled off over-counted ~4.95× — and the over-count is the
proof, because a 1:1 result alone is also what a frozen session produces. That
control is now the acceptance test
`a_rewound_counter_does_not_out_count_the_frames_it_ran` in
`fixtures/external_consumer/tests/rollback_is_a_promise.rs`.

⚠ **Run 7 also reported a severe engine desync that does not reproduce**, and
the next slice was nearly aimed at it; two of its five findings did not survive
checking. A subagent's conclusion is evidence to check, not a result to act on;
each check is recorded beside the claim in
[`slice-g-selection.json`](slice-evidence/slice-g-selection.json).

**The last three items were the same deferred thing, and it stayed deferred
until it was earned.** Nothing steered them there; each was reached
independently, and the destination was the boundary ADR 0031 drew before any of
this was built.

**Two standing reservations, at the top rather than in an appendix**, because
"all three conditions hold" is a claim about the three things this campaign
chose to measure and not about the API being finished:

* **Every consumer-matrix row is proven by a consumer written in this repo.**
  The matrix's own argument is that an API proven against one consumer is
  shaped like that consumer — and that applies to authorship as well as count.
  Outlander is external in *dependency shape* and internal in *authorship*.
  Nothing in §4 measures this, the blind-agent runs are a deliberate proxy for
  it, and it is not closeable by a slice.
* **The capability footprint never moved.** Depending on `ambition` links 41
  crates, 19 of which a movement-only game never asked for, and that number is
  identical to slice A's. It is the one §4 decomposition trigger that never
  fired. Seven slices of "no consumer has been unable to do something because
  of it" is now evidence rather than an absence of data — which is exactly what
  made it the slice-G candidate, and what run 7 outranked: it lost to five
  things a consumer felt in one session, and is now **slice H** rather than a
  footnote.

Slice F is the one the campaign could have faked. `ambition::rollback` could
have been curated and the baseline pruned in an afternoon, at any point across
slices C, D and E, and the ratchet would have read zero. The shortcut publishes
the same module name over the same implementation; what it omits is everything
that makes the name a promise. What closed it instead was ADR 0031's own
prescription — `SnapshotState` carved down to the floor so a consumer's types
can implement it without naming an engine crate, the six properties given a
test each, and Outlander's forty-line hand-ordered startup collapsed into one
call. The rejected shortcut is still recorded at the contract entry, because
"we reached zero" is not the same claim as "there is nothing left to leak".

Executable plan for
[ADR 0031](../../adr/0031-public-facade-is-the-compatibility-boundary.md) and
[ADR 0032](../../adr/0032-authoring-is-declarative.md), both **Accepted**
on 2026-07-30 once this campaign reached its terminal condition.

### What each slice closed

| Slice | Leak | Result |
|---|---|---|
| **A** | a consumer must know the engine's assembly order | `PlatformerApp`; ratchet 18 → 14 |
| **B** | a minimal game cannot reach the windowed face | `characters`/`no_characters`, `host_status`; a second consumer at 0 |
| **C** | the engine refuses a game and drops the reason | `HostStatus::Refused`; a route hold was suppressing failure reporting |
| **D** | a composition holds ONE experience; the shipped host has four | multi-experience drafts; matrix row 4 |
| **E** | the builder boots only into a game; the shipped host boots into a launcher | `start_at_launcher()`; matrix row 6 |
| **F** | rollback is not a public promise; `SnapshotState` sat above the domains it encodes | `ambition::rollback` + `PlatformerApp::rollback(n)`; the vocabulary carved to `ambition_engine_core::snapshot`; ratchet 1 → 0; ADRs 0031/0032 Accepted |
| **G** | the SDK stops at `start()` returning Ok | `sim_schedule` documented, `rollback::health`, `MatchSeat`; Script B run 8 at zero engine files |

### The shape that recurred three times

    single face      -> two faces      (A)
    one experience   -> four           (D)
    one host policy  -> two            (E)

Each time the SDK expressed ONE option while the shipped host needed another,
and each was invisible while only the games written against the SDK were
consulted. Three independent arrivals at the consumer matrix's argument, by
measurement rather than assertion — a stronger case for ADR 0031's sequencing
than the ADR itself makes.

### Slice F, derived and blocked

*(Derivation record — F LANDED 2026-07-30 in two parts; see §Deferred for what
delivery looked like.)*

**Rollback as a public promise.** The only candidate left, and the campaign's
§4 carve trigger FIRED while scoping it: rollback ownership cannot be federated
without moving code between crates, because `SnapshotState` and
`AmbitionRollbackApp` live in `ambition_runtime`, which depends on twenty domain
crates while none depends on it.

⚠ **The compiler then proved the sequencing.** Attempting the carve produced
E0117 — the 62 `impl SnapshotState for <foreign type>` blocks are legal only
while the trait is local, so **extracting the vocabulary and federating the 62
impls are ONE atomic change**, not two commits. The attempt was backed out
rather than landed half-done.

The instrument is already in place: the campaign's SECOND ratchet (319 stable
names + 62 codecs, frozen as a SET) was built one commit before the constraint
was measured, so the federation is measurable from its first commit.

Full derivation, ranking and the five things the next attempt inherits:
[`slice-evidence/slice-f-selection.json`](slice-evidence/slice-f-selection.json).

⚠ **The cheap version is available and would be fraud.** Curating
`ambition::rollback` as a closed module and pruning the baseline takes §4.1 to
zero in twenty minutes and makes, through the back door, exactly the promise
ADR 0031 reserves for its own slice. The allowlist contract carries that
reasoning at the entry, because "one away from zero" is when somebody reaches
for it.

### Slice G — closed, and what it leaves open

Derivation and per-finding verdicts:
[`slice-g-selection.json`](slice-evidence/slice-g-selection.json). The slice
was re-aimed the day it was derived: the ranking chose the capability footprint
BEFORE blind run 7 returned, and run 7 found five things a consumer felt in an
hour, so the footprint moved to slice H and G became **the far side of
`start()`** — what happens after a session exists.

**Closed (2026-07-30):** (e) where gameplay systems go — `app.sim_schedule()`,
never `Update`, now the phase-table section of `docs/sdk/api-reference.md`;
(c) session liveness — `ambition::rollback::health`, whose docs state the limit
a single sample cannot see (a frozen session reports Healthy forever, so
liveness is a property of TWO observations); (f) REFUTED —
`drive_control_frame` is correct under GGRS, proven by Outlander's
byte-identical parity walk; (a) closed as documentation — `require_rollback`
at the point of failure; (g) partly — `ambition::actor::MatchSeat` is the
query half of seating. Run 8's four findings, all documentation (a
`SnapshotState` worked example with the field-ORDER warning; "`rollback(n)`
does not create n characters"; `encoded_types()` is a DELTA assertion, not an
absolute; "spawn your rollback entities in `Startup`"), closed the same day.

**Open findings that need ENGINE work, carried out of the campaign:**

1. **(g) Seat-keyed input and query.** `session.participants()` is the
   DECLARATION — how many input streams GGRS checksum-compares. The seating
   comes from the stage and its devices. **Nothing reconciles them**: a
   composition can declare four participants and seat two, and no error says
   so. `MatchSeat` answers "which seat is this body" but no public seam drives
   input to a NAMED seat, so two independent input streams cannot be shown to
   reach two bodies and a couch-versus game is not yet expressible through the
   SDK. The Smash matrix row carries the limit explicitly
   (`⚠_what_the_participants_half_does_NOT_prove` in
   [`consumer-matrix.json`](slice-evidence/consumer-matrix.json)); the test
   claims only what it proves
   (`the_match_has_two_distinct_seats_and_simulates_with_both`).
2. **(a) An inert registration should be unrepresentable.**
   `rollback_component_canonical` on an entity the consumer spawned is
   accepted, counted by `encoded_types()`, and inert until
   `require_rollback::<T>` is also called. Closed as documentation, and the
   documentation demonstrably works (it is what took run 8 to zero) — but this
   campaign's own rule elsewhere is to make the wrong thing unrepresentable
   rather than warned about, e.g. by refusing at registration time when no
   entity family is declared.

⚠ **Three assertions passed for the wrong reason in one day**, none caught by
reading, all three by probing: a refusal whose precondition never occurs
(`ParticipantsDisagree` — deleted), `is_running()` used as a liveness check on
a sim that can freeze while reporting Running (both slice-F tests now require
the frame to ADVANCE), and a two-seat test that also passed with its gamepads
deleted, because the versus stage seats two fighters on its own. The instance
count is recorded on the matrix row rather than in commit messages nobody
greps.

### Slice H, derived — the facade's edges, made optional

Unchanged in content from the superseded slice-G ranking (the `slice_g` object
in [`slice-g-selection.json`](slice-evidence/slice-g-selection.json), including
its exit-criteria sketch and the failure mode to avoid — cutting edges until
the number looks good). Depending on `ambition` links 41 crates, 19 of which a
movement-only game never asked for; it is the one §4 decomposition trigger
that never fired, and it is still unfelt by any consumer after eight blind
runs. The reason to do it has not weakened; the reason to do it FIRST has,
twice.

**⚠ FIRST CUT EXISTS, UNMERGED AND UNVERIFIED (2026-07-30)** — branch
`worktree-agent-af39b56fa4add8fc2`, commit `26237cb3f`. 18 of 41 edges became
implicit crate features (`default = ["all_capabilities"]` preserves today's
facade exactly); `fixtures/minimal_game`'s measured closure moved **41 → 38**
(`inventory_ui`, `portal_presentation`, `touch_input` cut) and the footprint
ratchet was reworked to measure the sentinel's RESOLVED closure via
`cargo tree --locked` (the old static walk counted optional edges regardless
of features — it could never have moved) with the baseline pruned 41→38 /
19→15 in the same commit. Honest residue, recorded in the branch's evidence
files: `render` is optional in the facade but NOT for minimal_game (its
windowed boot is a slice-B exit criterion); `audio` stays unconditional
(`no_audio` still registers a fragment); and the other 14 unwanted crates
remain linked because **`ambition_actors` brings them — the §4 carve
condition, exactly as the baseline predicted.** NOT merged because three
verifications were still compiling at wrap-up: `cargo check -p ambition`
(default features), `cargo test` in both fixture workspaces, and the
red-probe of the new art-without-render refusal. Run those three before
merging; a green result closes most of the slice, a red one is a finding.

---

## Rules for every slice

Each is a scar, not an aspiration.

1. **One authority, migrate, delete, guard.** From
   [architecture-campaign-2026-07-28.md](../architecture-campaign-2026-07-28.md):
   *"Introduce one authority, migrate all production consumers, delete the
   displaced authority, and guard the absence. Every one of the five parts is
   required."*
2. **Name a test, not a doc marker.** Prose-asserted absences have gone red on
   prose three times here.
3. **Seen red before green.** A check that has never failed is a check whose
   subject you have not verified.
4. **A slice ends with one path, not two.** If the new surface and the old raw
   paths are both in use when the slice closes, the slice did not land.
5. **Every migration is a RATCHET, never a flag day and never a red main
   branch.** This campaign has three, and they are the same mechanism: a
   committed baseline set, a test that it may not grow, and work that shrinks
   it. See §Ratchets.

---

## Ratchets

Three, identical in shape. Each lands **green against a recorded baseline**, so
`main` is never failing, and each only ever shrinks.

| Ratchet | Baseline | Invariant | Zero means |
|---|---|---|---|
| **Module allowlist violations** | every `ambition::…` path production consumer code names that is not in the reviewed public surface | the set may not gain a member, **and may not keep one the consumer stopped naming** | consumers name only the SDK |
| **Central rollback registrations + codecs** — **BUILT 2026-07-30**, baseline **319 stable names + 62 codecs** in `slice-evidence/rollback-schema-baseline.json` | the explicit set of stable schema names in `register_engine_rollback_state` and the `impl SnapshotState` blocks in `rollback/codecs.rs` | `current ⊆ frozen`, **and** `frozen ⊆ current` so a federating migration must prune | rollback ownership is federated |
| **Undeleted compensating mechanisms** | ADR 0032's deletion criteria | the list may not gain a member | the seams took ownership |
| **Capability footprint** — **BUILT 2026-07-30**, baseline **41 crates, 19 unwanted** in `slice-evidence/capability-footprint-baseline.json` | the transitive `ambition_*` closure a consumer links by depending on the facade alone | the closure may not GROW (one invariant, not two — see below) | a game links only what it asked for |

⚠ **The footprint ratchet has ONE invariant on purpose.** The other two need
`baseline ⊆ current` as well, because a retired member leaves a SLOT that can be
silently reoccupied. A crate leaving a dependency closure leaves no slot — it
cannot be replaced without the count moving. Copying the second invariant here
would add a rule with no failure behind it, which is how a guard becomes
ceremony.

> ⚠ **This row cited the wrong file for three days.** It said "the codecs in
> `rollback/codecs.rs`", and that file's `pub fn`s are four reconciliation
> helpers — the codecs are its 65 `impl SnapshotState` blocks. Nobody re-read
> the citation between writing it and building the ratchet against it, which is
> precisely the class `check_roadmap_evidence.py` exists for and precisely what
> `feedback_docs_describe_nonexistent_smell` warns about.

⚠ **A count is not a ratchet.** Freezing only the *number* of central rollback
registrations permits deleting one and adding another. Freeze the **set**, by
stable schema name. Same for codecs — otherwise registration federates outward
while `ambition_runtime` remains the implementation owner of every domain's
snapshot, which is exactly the state
`impl SnapshotState for ambition_actors::…::MatchSeat` describes today.

⚠ **Freezing the set is only half of it — the set must also be PRUNED.** A1
found this while implementing the first ratchet. A frozen set whose entries
are never removed as they are migrated is still a budget: retire one member,
leave it listed, and the slot it vacates can be filled by something else
without the contract ever going red. So each ratchet carries a second
invariant — *the baseline may not keep a member the subject no longer has* —
which forces the prune into the migrating commit and makes re-adding
impossible. All three ratchets in this table want both halves.

---

## Slice A — host facade and external composition

**The leak:** a consumer must know the engine's assembly order.
`build_windowed_app` in `fixtures/external_consumer` is ~65 lines whose ordering
is load-bearing in at least four places, three of them recorded in that file's
own comments as leaks found the hard way.

**Bounded to host composition.** No content model, no character authority, no
capability staging, no rollback federation. The minimal experience definition is
whatever host assembly needs and no more.

### A1 — the public-surface allowlist, with a baseline — **LANDED 2026-07-30**

Extend `scripts/check_absence_contracts.py` to module-path granularity and add
an **allowlist** contract: production game/consumer code may name only reviewed
public SDK modules; everything else under `ambition::` is a violation.

⚠ **Allowlist, not denylist, and the numbers settle it.** Outlander names **18
distinct top-level `ambition::` modules** — `actors`, `asset_manager`, `audio`,
`characters`, `engine`, `engine_core`, `entity_catalog`, `game_assets`,
`game_shell`, `input`, `platformer`, `presentation`, `provider`, `runtime`,
`sprite_sheet`, `time`, `windowed_host`, `world`. The first draft of this
campaign forbade six. It would have gone green with **twelve leaks still
open**, which is worse than no contract because it would have been believed.

> ⚠ **Correction, 2026-07-30 — this row said NINETEEN and listed eighteen.**
> So did [ADR 0031](../../adr/0031-public-facade-is-the-compatibility-boundary.md).
> The fixture names **eighteen**: no brace-grouped `ambition::{…}` imports and
> no root-level type re-exports were hiding from the count. The number now comes
> from the instrument (`--allowlist-open-count`), never from this paragraph — a
> baseline transcribed out of prose is a ratchet nobody measured, and the
> baseline IS the contract's entire content.

Lands **green against the recorded baseline of 18**, non-increasing. Not red on
`main`.

Exact public module names stay **provisional** until A2 is accepted — `allowed`
is deliberately EMPTY, because populating it before the call sites exist would
be designing the API from the module list, which is the sequencing ADR 0031
rejects.

**What landed.** `MODULE_ALLOWLISTS` in `scripts/check_absence_contracts.py`,
scoped to `fixtures/external_consumer/` only (Jon, 2026-07-30: `game/` stays
out, because `ambition_content`'s dependency on the facade is a measurement
question ADR 0031 defers, and widening the paths would answer it by accident).

**Two invariants, and the second is the one that makes it a ratchet:**

| | Invariant | Without it |
|---|---|---|
| 1 | `named ⊆ allowed ∪ baseline` | the consumer can name a new module |
| 2 | `baseline ⊆ named` | the baseline is a *budget*: migrate `time` away, leave it listed, and the freed slot is occupied silently — a ratchet on a count, which §5 of the growth method says is not one |

Composed, they give the property being bought: a pruned module can never come
back, because invariant 1 then rejects it.

**Seen red before green**, all four ways: a module dropped from the baseline
reports `NEW`; an unpruned entry reports `STALE`; a brace-grouped
`use ambition::{combat::Strike, effects::Spark};` appended to the real fixture
took the contract red at `src/lib.rs:911` with exit 1; and prose naming
`ambition::runtime` stays silent. That third one is why the contract parses use
trees instead of matching a line regex — `\bambition::([a-z_]+)` sees `{` and
stops, so the obvious implementation would have been green, and wrong the first
time anyone wrote idiomatic Rust. Probes live in
`scripts/tests/test_absence_contracts.py`, including a non-vacuity assertion:
an instrument that silently measures nothing reports ZERO open leaks, which is
this campaign's success condition.

### A2 — `docs/sdk/api-prototype.md`, host call sites only

No implementation. A minimal visible game, the same game headless, and the
smallest experience/module declaration host assembly requires. Judged by
reading.

Two constraints, cheap now and expensive later:

* `GameModule` is `fn manifest(&self)` + `fn define(&self, …)`. Not because
  `Box<dyn GameModule>` is required — generic `mount(SanicModule { difficulty })`
  erased into `PreparedModule` is sufficient — but because a receiver-less
  `define` or an associated `const ID` forecloses parameterised modules for
  nothing.
* **Domain preludes, not one root prelude.** `ambition::character::prelude`,
  `ambition::world::prelude`. One enormous root prelude is a discovery problem
  for an agent, not a convenience.

### A3 — implement `PlatformerApp` — **LANDED 2026-07-30**

Over current machinery; no crate moves. It owns asset-source install,
foundation, simulation host, platformer runtime, window/device host, shell,
experience registration, asset preparation, presentation and optional audio in
the one correct order.

`Simulation`/`SessionMode` exposes **fixed-step only**. Rollback is not a public
knob in A — see §Deferred.

**Landed as `crates/ambition/src/app.rs`.** The umbrella is where it belongs and
there is precedent in the same crate: `game_assets` lives there because it spans
two layers that may not depend on each other, and its module docs say so. A
builder that sequences installs is assembly, not the leaf system ADR 0031 warns
about.

Acceptance: `fixtures/external_consumer/tests/composition.rs` — one mounted
module reaches BOTH faces, and a request a face cannot honor is a stated error
rather than a silent no-op.

Two decisions A2 §7 left open, resolved by building it:

* **The rollback variation** — resolved as proposed. `unstable_rollback_session`
  is `#[doc(hidden)]` and `SessionMode` still has exactly one public arm, so the
  promise is unchanged while the fixture's third composition goes through the
  same builder instead of staying a fork.
* **Asset preparation is POLICY, not a face** (`with_game_assets`). This one was
  got wrong twice before the fixture settled it — see the leak below.

### A4 — migrate Outlander, delete its composition path — **LANDED 2026-07-30**

Windowed and headless. **Delete, not deprecate.** Done: `build_outlander_app`,
`build_outlander_rollback_app` and `build_windowed_app` are one builder call
each; `compose_outlander_shell`, `register_outlander_asset_source` and
`RenderMode` are gone; the three test sites that rebuilt composition subsets by
hand now use the real thing; and `src/bin/dump.rs` — the last hand-ordered path,
which had been installing the WINDOWED host in a headless dump — went with them.

Guarded by `outlander-does-not-hand-order-its-own-composition`, and **seen red**:
reintroducing `add_headless_foundation` + `PlatformerHostPlugins` in the fixture
takes it red with exit 1, and takes the A1 ratchet red too, because those module
names are pruned and invariant 1 now rejects them. Two independent guards on one
regression, which is what pruning bought.

**Result: 18 → 14, exactly the four A2 §5 predicted** (`engine`, `game_assets`,
`presentation`, `windowed_host`). `ambition::app` is the first entry in the
allowlist's `allowed` set — the first name in this engine that is a promise
rather than a mirror of the crate list.

#### ⚠ §2d moved the WRONG WAY, and that is the finding

Slice A made **zero** of ADR 0032's six deletion criteria deletable. That is the
honest and expected result — all six are content or capability criteria, and
slice A was bounded to host composition. A slice reporting progress there would
have been reporting that it exceeded its own scope.

One of the six moved *away*:

> **`headless-and-visible-share-a-prepared-content-fingerprint`.**
> `PlatformerApp` gained `with_game_assets`, off by default on headless and
> always on for windowed, so the two faces now consume different prepared art
> unless the consumer says otherwise.

That knob is correct and was arrived at the hard way: the first implementation
installed assets on **both** faces *citing this very criterion*, and the
fixture's rollback parity test caught it — under GGRS the extra asset frames are
frames the sim does not advance. Preparing art is also not free (627MP/2.5GB at
boot). So the policy stays and the criterion is further off. **Slice B owns
closing it, and must close it without collapsing the policy back into the face.**

⚠ The collector originally reported this criterion as *deletable*. Its verdict
column was computed as `became_deletable = in_scope`, which is tautological —
an in-scope row could never be false, so `in_scope_but_not_deletable` was empty
BY CONSTRUCTION, and §2d calls that list *"the most valuable single signal this
method produces"*. Each criterion now carries its own verdict and reason, and
the collector asserts the column is not merely a restatement of the scope.
`provider-plugin-ordering-decides-content-completeness` was reclassified in the
same pass: it had been marked composition/deletable on the strength of the
host-ordering contract, but content completeness is still decided by
`Plugin::build` and the finish/`PreStartup` apparatus slice A never went near.

#### ⚠ The ninth leak, found BY the migration

A2 §1 inventoried eight rules. The migration found a ninth, and it is the
sharpest evidence in the slice for why migration is not a formality:

> **Under GGRS the frame dt must be integer nanoseconds, and
> `Time::<Fixed>::from_hz(60.0)` does not give them.** It rounds to
> `16_666_667`ns; GGRS needs the truncated `16_666_666`. Feeding it the rounded
> value costs real frames — the fixture's parity walk took **192 `update()`
> calls to reach a world state the fixed-tick host reached in 180**, while every
> checksum still agreed.

The rule existed, in a comment on the fixture's hand-composed rollback app
("the frame dt must be the tick dt exactly (integer nanos, no drift)"), and
nowhere else. A consumer who wrote the obvious thing got a host that runs,
simulates correctly, agrees on every checksum, and quietly needs 7% more frames.
That is the silent class §3a prices at triple, and it survived only because one
fixture had already been bitten.

It was found because the parity test went red on a change that looked
unrelated — which is what a canary is for.

### A5 — first blind agent run (baseline) — **LANDED 2026-07-30**

Fresh context, `docs/sdk/` + facade only: *stand up a new minimal game against
this engine.* Record completion, **which engine file it opened first**, and
elapsed context. This run establishes the baseline the later ones improve on;
it is not expected to succeed at authoring content, which does not exist yet.

Fixed script: `slice-evidence/blind-agent-runs/SCRIPT.md`. Record:
`slice-evidence/blind-agent-runs/2026-07-30-slice-a-baseline.json`.

**Result: headless booted, windowed was blocked by the environment, and a
minimal game was NOT reached.** The run was worth more than the rest of the
slice, and three of its findings are worth reading in full.

#### The binding constraint is not an API leak

> **The engine does not compile for an outside consumer at all.** A fresh
> lockfile resolves `bevy_ggrs` from crates.io and `ambition_runtime` dies with
> `cannot find type GgrsFrameTiming`; you must copy `[patch.crates-io]` out of
> the engine's workspace root, and nothing says so.

It precedes every API question. No amount of work on `ambition::app` reaches a
third party who cannot get past `cargo check`. Now documented in
`docs/sdk/README.md` — and see the slice-B derivation for why *removing* it is
not ours to decide.

#### Two of the overclaims were in documents this campaign wrote

* **Rule 7 was not enforced, and `ambition::app` claimed it was.** The module
  docs said the empty host was *"unreachable rather than merely documented"*.
  What was enforced is that a STRING had been supplied. The agent declared a
  gameplay route nothing served and got a host that built clean, ran 60 ticks
  and spawned zero entities — and found it only because it independently counted
  entities. **Now actually enforced**: `try_build` checks declared routes against
  the `ShellRouteCatalog` and names the routes that do exist. Seen red before
  green; test `a_declared_route_no_capability_registers_is_refused`.
* **`api-prototype.md` §2b said the two faces are interchangeable.** They are
  not: the visible face requires a `CharacterCatalog`. The agent recorded that
  the document *"actively told me the opposite would be true"*. Corrected in
  place, with the limitation stated rather than quietly dropped.

An overclaimed guarantee is worse than an absent one — it tells a consumer to
stop looking. Both were rule 2 violations (*a doc marker where a check belongs*)
committed by the campaign that wrote rule 2.

#### And one API bug A3 introduced

`ModuleDraft::capability` required `Clone`, and the engine's own
`CharacterCatalogPlugin` is not `Clone` — so an engine plugin could not go
through the engine's own capability slot. Fixed: installers are `FnOnce`,
drained through a `Mutex`.

#### ⚠ The headline number is misleading, and the agent said so first

It opened no `.rs` file under `crates/`. It also ran `cargo doc` over **seven**
engine crates and read the rendered API — *"the same information a leak would
have given me, just laundered through rustdoc"*. Scoring on `.rs` files opened
would have reported a pass. The record counts the rustdoc surfaces, because
ADR 0031's gate is *never opening a file under `crates/`* and rustdoc over a
crate is reading that crate.

Two contaminations are recorded rather than hidden, both biasing **toward**
competence: the subagent inherits `AGENTS.md`, and the evidence tree was still
under `docs/sdk/` when the run launched (moved mid-run).

### Slice A exit criteria

* [x] allowlist ratchet green, baseline 18 *(A1, 2026-07-30)*;
* [x] open-leak count **strictly lower** than 18
      (`scripts/check_absence_contracts.py --allowlist-open-count`) — **14**,
      retiring `engine`, `game_assets`, `presentation`, `windowed_host`. §5
      predicted exactly those four and exactly 14, recorded BEFORE A4 ran
      *(A4, 2026-07-30)*;
* [x] Outlander's composition is policy, not ordering *(A3/A4, 2026-07-30)*;
* [x] Outlander's manual composition path deleted, and the absence guarded
      *(A4, 2026-07-30)* — `src/bin/dump.rs` was the last one, and it installed
      the WINDOWED host in a headless dump, which nothing noticed because the
      registries it prints do not come from the host;
* [x] blind-agent baseline recorded *(A5, 2026-07-30)* — fresh subagent, fixed
      script, record in `slice-evidence/blind-agent-runs/`. First engine file
      opened: `fixtures/external_consumer/Cargo.lock`, for a BUILDABILITY
      question, before any API question could be asked;
* [x] §2 evidence collected per the growth method — all five, by
      `scripts/collect_slice_evidence.py` into
      `slice-evidence/slice-a-evidence.json`;
* [x] slice B **derived** from that evidence rather than invented —
      `slice-evidence/slice-b-selection.json`.

**Slice A is closed (2026-07-30).**

### What slice A reduced (§6's convergence check)

A slice that reduces none of the five counters did not close a leak. This one
reduced two, and left three honestly unmoved:

| Counter | Slice A |
|---|---|
| forbidden paths a consumer names | **18 → 14** |
| open fixture findings | **3 closed, 1 new** (the GGRS integer-nanos dt) |
| engine files a blind author must open | baseline set: 8 API surfaces + 1 lockfile |
| undeleted compensating mechanisms | 6 → 6, correctly — all six are content criteria |
| unwanted linked capabilities | 41 → 41, untouched; slice B is what makes it measurable |

### Slice B, derived

**The movement-only minimal game.** The campaign *sketched* B as declarative
content and character authority. The evidence confirms the domain and changes
the shape: the blind run did not fail for want of a namespace rule, it failed
because there is no empty content and no way to ask whether the host came up.
§3c sizes a slice by one leak closed end to end and says *split by consumer, not
by layer* — a `ContentPack` design with no consumer to migrate is a layer.

It also fills the consumer-matrix row Outlander structurally cannot, and it is
the row §4 needs before any decomposition can be argued: a carve authorised by a
sentinel consumer's footprint requires a sentinel consumer, and there is
currently one.

Full ranking, routing and the explicit NOT-in-this-slice list:
`slice-evidence/slice-b-selection.json`.

⚠ **The highest-cost leak is NOT slice B**, and that is §3b working. The
`bevy_ggrs` patch table outranks everything on cost; measuring its closeability
rather than assuming it is what routed it away. Exactly one production call
site — so it looks trivial — but the only way to drop the fork is the parallel
accumulator `sample_ggrs_accumulator_phase`'s own doc already rejected: it
*"would diverge during run-slow catch-up, stalls, several advances in one frame,
and rollback resimulation — exactly when a wrong phase shows most"*. Stating the
rule is ours and is done; removing the need waits on upstream. That is
awaiting-maintainer-decision, not a slice.

> **DECIDED 2026-07-30 (Jon): defer it.** Revisit once upstream merges the
> `GgrsFrameTiming` accessor to crates.io. Recorded in
> [`maintainer-decisions.md`](../maintainer-decisions.md). It does **not** block
> slice B, and it stays listed as the top-ranked candidate in the derivation on
> purpose — deleting the highest-cost item would make slice B look like it won a
> race it never ran.
>
> The half that IS ours is done: `docs/sdk/README.md` carries the required
> `[patch.crates-io]` entry with the pinned rev, so a third party is told before
> they hit `cannot find type GgrsFrameTiming`. What remains is small and real —
> rule 2 says a doc marker is not enough, so the documented rev needs a test
> pinning it to the workspace's actual one, or the paragraph rots the first time
> the fork moves.

---

## Slice B — the movement-only minimal game (IN PROGRESS, 2026-07-30)

**Derived, not sketched.** The ranking is in
[`slice-evidence/slice-b-selection.json`](slice-evidence/slice-b-selection.json);
the sketch below (§B) said content-model-first, and the evidence said
consumer-first. Both agree on the domain, and §3 warns that agreement is
confirmation rather than derivation.

**The leak:** a consumer cannot stand up a SMALL game. The visible host demanded
content a minimal module had no way to supply, and nothing told you whether the
host came up.

### What landed

`fixtures/minimal_game` — its own workspace and lockfile, exactly like
Outlander, so what compiles is what a third party gets.

* **`ModuleDraft::characters(ron)` / `no_characters()`.** ⚠ The fix is NOT an
  empty default. `PlatformerAssetsPlugin`'s refusal is deliberate — *"silently
  substituting an empty catalog is how a game ships with its bosses drawn as the
  fallback body and nobody notices"* — so slice B made the true answer
  **sayable** rather than making the demand disappear. Saying nothing while
  preparing art is now a structured `CompositionError` naming both fixes.
  `EMPTY_CHARACTER_ROSTER_RON` is published because the blind agent had to
  recover it by feeding the parser `"()"` and reading the errors back.
* **`ambition::app::host_status`** — the read-model the blind agent went looking
  for. `Running { prepared }` is two facts, and `is_running()` requires both, so
  the type cannot agree with the empty host it exists to expose.

### What it measured

**5 modules against Outlander's 14** — and they are not a smaller sample of the
same problem. Four are one hole: `PlatformerExperienceAuthoring`,
`PreparedPlatformerSource`, `RoomSpec`, `engine_core` geometry. **A minimal game
can COMPOSE through the SDK and still cannot DECLARE a room or an experience
through it.** The fifth is `audio`.

It needs **no `bevy` dependency at all** — it derives nothing. Outlander does.
Invisible with one consumer, and it is why the SDK README's blanket "you also
need bevy" was wrong.

### ⚠ Two failures worth more than the feature

**The read-model found a bug in the commit before it.** Slice B's first boot
tests asserted `try_build` succeeded and a `FixedUpdate` schedule existed. *They
never ran a tick.* The first test that stepped the host found it stuck in
`Activating` for 600 ticks — the game composed, booted, and never started. That
is the blind agent's empty host, reproduced in our own new consumer, one commit
after the matrix row was marked proven.

The cause: preparation validation refuses an experience whose provider
registered no explicit audio fragment. **Outlander's own comment already knew,
including the worst part — *"a good message that a headless host surfaced
NOWHERE"*.** A known error-quality gap sat there and the second consumer walked
into it. `HostStatus` now names the stuck state; the REASON is still swallowed,
and that is slice-C material.

**The baseline was measured against a game that did not work.** Recorded as 4,
corrected to 5 when the game actually ran. The number is not the lesson — the
instrument is: *a consumer's baseline must be measured against a WORKING
consumer*, because measured against a compiling one it reads low, in the
flattering direction. The ratchet caught the growth unprompted on its first live
use, at the exact file and line.

---

## Slices B–D — the ORIGINAL sketch, kept for comparison

⚠ **Superseded by what actually happened, and kept because the difference is
the campaign's own evidence.** The sketch below predicted B as content-model
work: `ModuleDraft`, `ContentPackDraft`, preparation, module-qualified
namespaces, with "ContentPack and namespaces BEFORE CharacterSpec".

What the evidence selected instead was consumer-first every time — a minimal
game (B), legible failure (C), multi-experience composition (D), host policy
(E). Namespaces were never reached, and D showed why: the blocker was not a
naming rule but that two experiences could not coexist at all. A namespace
scheme designed from this sketch would have solved the second problem while the
first stayed open.

The domain was right and the shape was wrong, which is exactly what
[api-growth-method.md](api-growth-method.md) §3 warns a sketch is for: legibility,
not selection.

### The original sketch

Re-derived before starting. Recorded here so the shape of the whole is legible
and so A does not quietly absorb them.

### B — declarative content and character authority

`ModuleDraft`, `ContentPackDraft`, preparation, module-qualified namespaces.

**`ContentPack` and namespaces come BEFORE `CharacterSpec`** — the container and
its identity rules are foundational, and `CharacterSpec` is one schema family
inside a prepared pack rather than the root transaction. The pack design must
answer, in one pass: module and pack identity; pack version; explicit source
manifest; canonical document ordering; duplicate and symlink handling
(`game/ambition_content/assets/sprites` is a symlink into the engine tree that
has already caused a double-registration bug); module-qualified content ids;
module-relative asset identities; schema and provider identities;
unresolved→resolved typed references; merge-conflict behavior; capability
requirements; content fingerprint.

Then: a **small stable character core plus capability-owned extension facets** —
not an anonymous facet bag, which is open but very hard for an agent to author
correctly against. Host-behavior kits become a **validated, versioned binding
identity**, not a global `HostCode` flag.

Migrates **all** production character contributors — Outlander, Mary-O, Sanic,
versus fighters, robot lineage — deletes the parallel catalog/preparation
authority, and only here may claim the `PreStartup` backstop deletion criterion.

Positive **and negative** validation: a character naming a missing schema, an
unregistered preset or an uninstalled capability must FAIL, not boot with a
silently missing facet. The test in the suite is the authority;
`ambition content validate` is a second front door.

### C — capability and rollback federation

A real `PreparedCapabilityPlan` (see ADR 0032 — Ambition owns the plan; a Bevy
`PluginGroup` is only its lowering). Domain-owned `RollbackSchemaFragment`s.
Freeze the legacy central registration **and codec** sets. Migrate **one
complete domain** end to end and prove module-order-independent schema assembly
and fingerprints.

### D — runtime content revision

`ContentRevision` through the same draft → validate → prepare path as initial
publication, with a real replacement consumer (LDtk reload).

⚠ **Content revision and session transition are different transactions** that
share a confirmed commit boundary. A room transition selects from *existing*
prepared content; it does not edit a draft or publish a new content fingerprint.
The first draft of ADR 0032 conflated them; it has been corrected.

---

## Deferred, with reasons

* ~~**`Simulation::Rollback` as a public knob.**~~ **DELIVERED, slice F,
  2026-07-30.** It was deferred for the right reason and stayed deferred for
  four slices while it was the only thing between the campaign and §4.1 — the
  pressure to curate a module and call it done was real and continuous, and
  `slice-d-selection.json` records the refusal as "closeable: Yes, trivially
  and WRONGLY".

  The six properties are now `ambition::rollback`, with a test each in
  `fixtures/external_consumer/tests/rollback_is_a_promise.rs`. The named
  hazards are structural rather than documented: `start` activates and settles
  before rebasing frame zero, so the un-rebased write and the
  activation-on-frame-1 cases are unreachable from the public path; the
  participant count is declared at composition, so a restart cannot re-sample
  it. `PlatformerApp::rollback(participants)` replaced the `#[doc(hidden)]`
  `unstable_rollback_session`, and the waiver that hid it from the SDK
  reference was deleted with it.

  ⚠ The prerequisite was an internal carve, and it was not visible from the API
  side. `SnapshotState` sat in `ambition_runtime`, above every crate whose
  types it encoded, so the orphan rule had forced ~100 foreign impls into one
  2688-line file — and no consumer could implement it for their own type
  without naming an engine crate. Moving it to `ambition_engine_core::snapshot`
  is what made the public promise expressible at all. §4's carve authorisation
  covered it exactly: a leak that cannot be closed without moving code between
  crates authorises the boundary the leak names.
* **Any `ambition_actors` decomposition.** See
  [api-growth-method.md](api-growth-method.md) §4 for the two conditions that
  authorise it.
* **The capability-composition doctrine.** Derived at the end, not written at
  the start (ADR 0031, Alternatives).

---

## The consumer matrix — required before 1.0 is declared

Outlander is the right *first* consumer and cannot establish the API alone. An
API proven only against Outlander is an API shaped like Outlander. Before the
compatibility surface is declared complete, each category needs a proof; the
*order* stays evidence-driven, the *categories* are not optional.

| Consumer | What it proves |
|---|---|
| Outlander | external dependency + host composition |
| a movement-only minimal game | optional-capability closure — does a small game link menus, persistence, audio, bosses? |
| a noncombat game or actor | "actor" is not secretly combat-shaped |
| Sanic or Mary-O, standalone **and** embedded | reusable module + namespace identity; same content and schema fingerprints both ways |
| Smash | participants, character selection, atomic match lifecycle, scoped rules, rollback |
| Ambition itself | full integration |

This list is enforced by [api-growth-method.md](api-growth-method.md) §4: the
campaign may not terminate with categories unexercised.
