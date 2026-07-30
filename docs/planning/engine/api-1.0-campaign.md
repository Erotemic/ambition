# API 1.0 campaign

**Status:** not started (2026-07-30). Executable plan for
[ADR 0031](../../adr/0031-public-facade-is-the-compatibility-boundary.md) and
[ADR 0032](../../adr/0032-authoring-is-declarative.md), both *Proposed*.

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
| **Module allowlist violations** | every `ambition::…` path production consumer code names that is not in the reviewed public surface | the set may not gain a member | consumers name only the SDK |
| **Central rollback registrations + codecs** | the current explicit set of stable schema names in `register_engine_rollback_state` and the codecs in `rollback/codecs.rs` | `current ⊆ frozen_legacy`; **no new stable name may enter** | rollback ownership is federated |
| **Undeleted compensating mechanisms** | ADR 0032's deletion criteria | the list may not gain a member | the seams took ownership |

⚠ **A count is not a ratchet.** Freezing only the *number* of central rollback
registrations permits deleting one and adding another. Freeze the **set**, by
stable schema name. Same for codecs — otherwise registration federates outward
while `ambition_runtime` remains the implementation owner of every domain's
snapshot, which is exactly the state
`impl SnapshotState for ambition_actors::…::MatchSeat` describes today.

---

## Slice A — host facade and external composition

**The leak:** a consumer must know the engine's assembly order.
`build_windowed_app` in `fixtures/external_consumer` is ~65 lines whose ordering
is load-bearing in at least four places, three of them recorded in that file's
own comments as leaks found the hard way.

**Bounded to host composition.** No content model, no character authority, no
capability staging, no rollback federation. The minimal experience definition is
whatever host assembly needs and no more.

### A1 — the public-surface allowlist, with a baseline

Extend `scripts/check_absence_contracts.py` to module-path granularity and add
an **allowlist** contract: production game/consumer code may name only reviewed
public SDK modules; everything else under `ambition::` is a violation.

⚠ **Allowlist, not denylist, and the numbers settle it.** Outlander names **19
distinct top-level `ambition::` modules** — `actors`, `asset_manager`, `audio`,
`characters`, `engine`, `engine_core`, `entity_catalog`, `game_assets`,
`game_shell`, `input`, `platformer`, `presentation`, `provider`, `runtime`,
`sprite_sheet`, `time`, `windowed_host`, `world`. The first draft of this
campaign forbade six. It would have gone green with **thirteen leaks still
open**, which is worse than no contract because it would have been believed.

Lands **green against the recorded baseline of 19**, non-increasing. Not red on
`main`.

Exact public module names stay **provisional** until A2 is accepted.

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

### A3 — implement `PlatformerApp`

Over current machinery; no crate moves. It owns asset-source install,
foundation, simulation host, platformer runtime, window/device host, shell,
experience registration, asset preparation, presentation and optional audio in
the one correct order.

`Simulation`/`SessionMode` exposes **fixed-step only**. Rollback is not a public
knob in A — see §Deferred.

### A4 — migrate Outlander, delete its composition path

Windowed and headless. **Delete, not deprecate.**

### A5 — first blind agent run (baseline)

Fresh context, `docs/sdk/` + facade only: *stand up a new minimal game against
this engine.* Record completion, **which engine file it opened first**, and
elapsed context. This run establishes the baseline the later ones improve on;
it is not expected to succeed at authoring content, which does not exist yet.

### Slice A exit criteria

* [ ] allowlist ratchet green, baseline 19, and **strictly lower** than 19;
* [ ] Outlander's composition is policy, not ordering;
* [ ] Outlander's manual composition path deleted;
* [ ] blind-agent baseline recorded;
* [ ] §2 evidence collected per the growth method.

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
