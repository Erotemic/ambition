# API 1.0 campaign

**Status:** slice A in progress (2026-07-30) — **A1–A4 landed, A5 open.**
Executable plan for
[ADR 0031](../../adr/0031-public-facade-is-the-compatibility-boundary.md) and
[ADR 0032](../../adr/0032-authoring-is-declarative.md), both *Proposed*.

The allowlist ratchet stands at **14 of 18**; the four modules host composition
owned are retired and `ambition::app` is the first allowed SDK name. What remains
in slice A is the blind-agent baseline and the §2 evidence collection that
selects slice B.

**How slices after A are chosen:** by the procedure in
[api-growth-method.md](api-growth-method.md), from what the previous slice
measured. B/C/D below are *sketched* so the shape of the whole is legible; only
**A is specified**, and B/C/D are re-derived before they start.

**Thesis, settled across four review rounds:**

> Do not split `ambition_actors` by today's internal topology. Build and
> mechanically enforce the public API first, let real consumers reveal the
> durable capability boundaries, and reorganise behind them.

> ### ⚠ Revision, 2026-07-30 — the first draft of this file broke its own rule
>
> It had seven rows spanning host composition, semantic facade design, content
> compilation, character authority, open schemas, capability staging, rollback
> federation, migration, agent evaluation and validation tooling — while
> [api-growth-method.md](api-growth-method.md) §3c says *one slice closes one
> leak end to end*. Written and violated in the same sitting.
>
> The proof that it was too big is specific and was not mine: **an
> Outlander-only slice cannot honestly make the global
> character-preparation backstop deletable, because Mary-O, Sanic, the versus
> fighters and the robot lineage all still stage characters through the old
> completion mechanism.** `CharacterPreparationPlugin`'s `PreStartup` barrier is
> process-global; deleting it needs every contributor migrated. So the sharpest
> exit criterion in the campaign was in the wrong slice.
>
> Split into A–D below. That criterion now belongs to **B**.

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
| **Central rollback registrations + codecs** | the current explicit set of stable schema names in `register_engine_rollback_state` and the codecs in `rollback/codecs.rs` | `current ⊆ frozen_legacy`; **no new stable name may enter** | rollback ownership is federated |
| **Undeleted compensating mechanisms** | ADR 0032's deletion criteria | the list may not gain a member | the seams took ownership |

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

### A5 — first blind agent run (baseline)

Fresh context, `docs/sdk/` + facade only: *stand up a new minimal game against
this engine.* Record completion, **which engine file it opened first**, and
elapsed context. This run establishes the baseline the later ones improve on;
it is not expected to succeed at authoring content, which does not exist yet.

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
* [ ] blind-agent baseline recorded — **NOT DONE, and deliberately not
      self-reported.** §2c disqualifies an agent that has touched engine
      internals: it "measures its own memory", and the result is falsely green
      "in the direction that feels good". The session that landed A1–A4 read the
      movement kernel, the sim-view seam and the render cluster, so it IS that
      population. Needs a fresh agent given `docs/sdk/` + the facade only; drop
      the record in `docs/planning/engine/slice-evidence/blind-agent-runs/` and re-run
      `scripts/collect_slice_evidence.py`;
* [~] §2 evidence collected per the growth method — **four of five**, by
      `scripts/collect_slice_evidence.py` into
      `docs/planning/engine/slice-evidence/slice-a-evidence.json`. 2a/2b/2d/2e measured; 2c is the
      row above. ⚠ the goal check for this row tests KEY PRESENCE only, so it
      reads green while 2c is uncollected — the JSON's own `collected: false` is
      the authority, not the gate.

**Slice B is therefore NOT derived, deliberately.** §2c's first-engine-file-opened
field is the one that "names the next leak … from the population the API is *for*",
so picking B from the other four would be picking it by taste — which this
campaign's method forbids. What the collected four CONSTRAIN B to is recorded in
`selects_slice_b.constraints_from_the_collected_four`, so the eventual derivation
gets checked against evidence that predates it.

⚠ **The most valuable number slice A's evidence produced is not the ratchet.**
§2e: the consumer declares TWO dependencies and links **41** `ambition_*` crates,
and that figure did not move at all while the ratchet went 18 → 14. A module
allowlist cannot see the capability footprint — which is precisely the blind spot
§2e exists to record, and it says the semantic surface improving is not evidence
that the linked surface did.

---

## Slices B–D — sketched, not specified

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

* **`Simulation::Rollback` as a public knob.** A far larger promise than a
  clock: frozen schema, complete authoritative baseline, stable participants,
  deterministic activation, lifecycle rebasing, confirmation boundaries. Its
  hazards are recorded and real — an un-rebased `world_mut` write replays a
  world that never had it; seating completes on the session's first frame so
  activation lands on GGRS frame 1 where nothing can rewind across it; a
  confirmed lifecycle commit rebases mid-run and resets execution counters.
  Its own slice, its own acceptance tests.
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
