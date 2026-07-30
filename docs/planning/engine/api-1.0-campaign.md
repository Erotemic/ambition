# API 1.0 campaign — slice 1

**Status:** not started (2026-07-30). This is the executable plan for
[ADR 0031](../../adr/0031-public-facade-is-the-compatibility-boundary.md) and
[ADR 0032](../../adr/0032-authoring-is-declarative.md), both *Proposed*.

**How to get slice 2:** do not extend this document. Slice 2 is *derived* from
what slice 1 measures, by the procedure in
[api-growth-method.md](api-growth-method.md). That separation is the point — a
plan that predicts five slices is a plan whose last four rot.

**Thesis, settled across three review rounds:**

> Do not split `ambition_actors` by today's internal topology. Build and
> mechanically enforce the public API first, let real consumers reveal the
> durable capability boundaries, and reorganise behind them.

---

## Rules for every row in this campaign

These are not aspirational; each is a scar.

1. **One authority, migrate, delete, guard.** From
   [architecture-campaign-2026-07-28.md](../architecture-campaign-2026-07-28.md):
   *"Introduce one authority, migrate all production consumers, delete the
   displaced authority, and guard the absence. Every one of the five parts is
   required."* A new path living beside an old path is this campaign's defining
   failure mode, and the character catalog / prepared registry pair is the live
   proof.
2. **Name a test, not a doc marker.** "Approximately ten lines" is prose.
   Prose-asserted absences have gone red on prose three times here.
3. **Seen red before green.** A check that has never failed is a check whose
   subject you have not verified.
4. **No parallel paths across a slice boundary.** If slice 1 ends with both the
   new facade and the old raw paths in use by Outlander, slice 1 did not land.

---

## Row 1 — the contract, red

**Add `outlander-names-no-internal-module` to `scripts/check_absence_contracts.py`.**

Forbid game/consumer crates from naming implementation-shaped facade modules:

```text
ambition::actors     ambition::runtime    ambition::platformer
ambition::game_shell ambition::load       ambition::host
```

`DEPENDENCY_CONTRACTS` already does this class transitively over Cargo metadata,
with four live rows. The extension is **module-path granularity**, and the
file's own docstring says how to build it without repeating the three
prose-grep failures: production source only, explicit paths, a predicate rather
than a cleverer parser.

* **Lands RED.** Today Outlander names seven of these; the contract's failure
  output is the campaign's work queue and its exit status is the finish line.
* **Acceptance:** the contract fails, names every offending path, and the count
  is recorded here as the starting number.

⚠ Do **not** exempt `ambition_content` yet. It depends on the facade today, and
whether that should stop is a measurement question this campaign defers — the
existing `engine-crates-do-not-consume-the-umbrella-facade` row already records
that scoping decision and its reason.

## Row 2 — `docs/sdk/api-prototype.md`, call sites only

No implementation. One file a reader can judge:

* a minimal visible game (`main.rs`);
* the same game headless;
* one `GameModule` with a content pack and a rules plugin;
* one character document;
* one world document;
* one rules system using a stable `SimPhase`;
* one game-owned rollback component;
* one headless test.

**Judged by reading, not by compiling.** The question is whether this reads like
a library someone would choose, and whether an agent could write the second
character by pattern-matching the first.

Two things to get right here because they are expensive to change later:

* **the `GameModule` trait shape is `fn manifest(&self)` + `fn define(&self, ..)`.**
  An associated const or a receiver-less `define` is not dyn-compatible and
  forecloses a parameterised module (`SanicModule { difficulty }`). Whether
  modules are ever stored as trait objects is a separate question — the engine
  can erase into `PreparedModule` — but the trait must not *prevent* it.
* **the worked example uses a shared preset.** The catalog is 141 characters
  over 8 action-set presets; `peaceful` covers 83 of them. An example showing
  `preset: "mallory"` teaches one preset per character, and the sharing
  collapses. For an agent-facing API the example *is* the specification.

## Row 3 — design `CharacterSpec`, and the transaction shape with it

`CharacterSpec` first among the content APIs, because it forces an internal
decision that is currently deferred: `CharacterCatalog` and
`PreparedCharacterRegistry` are both alive, *"which is precisely why C3 has been
'nearly done' for three days."*

Decide **in the same pass**:

* the unified authored character document (absorbing the catalog row and the
  definition), with an open `facets:` list;
* the **content transaction** shape — `candidate` / `validate` / `commit` /
  epoch — because it reuses the draft type, and retrofitting it later changes
  every authoring signature (ADR 0032 §6);
* how a **host-code kit** is expressed in data, so `PlayableKitSource::HostCode`
  is an authored choice rather than a Rust exception;
* what happens to a facet whose schema **no installed capability claims**. The
  working answer is: the pack declares its required capability profile, an
  unclaimed facet is a hard validation error, and the symmetric check runs too
  (a registered schema no consumer reads is an error). This is the single
  highest-risk open question in the design — *ignore* recreates the
  prepared-but-unconsumed portrait field at scale.

## Row 4 — implement the facade over current machinery

No crate moves. No repository reorganisation. Delegate to the catalogs,
provider lifecycle, engine plugin groups and session builder that exist.

* `PlatformerApp` — owns asset-source install, foundation, simulation host,
  platformer runtime, window/device host, shell, experience registration, asset
  preparation, presentation, optional audio, in the one correct order;
* `GameModule` + `ModuleManifest` + `ExperienceSpec`;
* `ModuleDraft` — content accumulates as a **pure value**; capability methods
  record declarations (ADR 0032);
* role-based facade modules (`ambition::app`, `::experience`, `::character`, …);
* **one** domain extension trait (`CharacterAuthoringExt`), to prove the
  federation mechanism before there are six of them;
* `SimPhase`, so game systems stop naming `SandboxSet`.

**This is not a compatibility shim.** It becomes the canonical public surface on
landing, and the old paths it replaces are deleted in row 5, not deprecated.

### Row 4b — the rollback fragment protocol, with a ratchet

Land `RollbackSchemaFragment { owner, components, resources }` and the
gather → detect-duplicate-names → validate → deterministic-order → fingerprint →
freeze assembly, as part of the capability model. **Do not** mass-migrate the
workspace.

But that leaves the fragment seam and central `register_engine_rollback_state`
alive simultaneously, which is rule 1's violation. So:

> **Freeze the central list and let it only shrink.** Commit the current entry
> count; a test asserts it never rises.

New domains must use fragments because the old door is closed, and the number
going down is the migration's progress bar. Two lines of test; converts "we'll
migrate later" from an intention into a ratchet.

⚠ Vocabulary: what gets fingerprinted is the **schema** a capability
contributes, never the capability. Two builds with the same schema fingerprint
run the same declared schema and possibly different code. ADR 0026 is already
careful about this; the new API must not be looser.

## Row 5 — migrate Outlander, delete the raw paths

Outlander is the first consumer because its stated purpose is to expose SDK
leaks, and its comments are a dated log of every one it has found.

* rewrite `build_windowed_app` (~65 ordered lines) against `PlatformerApp`;
* delete every raw internal path the facade makes unnecessary — **delete, not
  deprecate**;
* where Outlander still needs something structurally unreasonable, that is an
  API finding: record it in the fixture's comments in the established format and
  feed it to [api-growth-method.md](api-growth-method.md).

**Acceptance: row 1's contract goes GREEN.**

## Row 6 — the blind agent run

Fixed task script, **fresh context**, only `docs/sdk/` and `ambition::prelude`
in scope:

1. add a character;
2. add a room;
3. add a mechanic.

Run by an agent with **no prior context of this repository** — otherwise it
measures memory, not the API. Record:

* whether each task completed;
* **which engine file it had to open first** (the useful field — it names the
  next leak);
* whether validation caught a deliberately broken reference.

## Row 7 — the negative acceptance test

A validator that accepts everything satisfies "adding a character is one file".
So pair it:

> Adding a character that names a **missing schema**, an **unregistered
> preset**, or an **uninstalled capability** must FAIL validation — not boot
> with a silently missing facet.

`ambition content validate` is the fast local path; the **test in the suite is
the authority**. One predicate, two front doors — the pattern
`declared_art_resolves.rs` already uses, and the reason that leak class stopped
recurring.

---

## Exit criteria for slice 1

All mechanical:

* [ ] `outlander-names-no-internal-module` green;
* [ ] Outlander's composition is policy, not ordering;
* [ ] no raw internal path remains in Outlander (deleted, not deprecated);
* [ ] the `PreStartup` character-preparation backstop is **deletable** — the
      single sharpest test of whether the completion boundary is real
      (ADR 0032);
* [ ] central rollback registration count committed and non-increasing;
* [ ] blind agent run recorded, including first-engine-file-opened;
* [ ] the negative validation test exists and was seen red.

## Explicitly NOT in slice 1

* Mary-O and Sanic migration — slice 2 at the earliest, and only if the method
  selects them;
* `Simulation::Rollback` as a public knob. It is a far larger promise than a
  clock: frozen schema, complete authoritative baseline, stable participants,
  deterministic activation, lifecycle rebasing, confirmation boundaries. It gets
  its own slice with its own acceptance tests. (Its hazards are real and
  recorded: an un-rebased `world_mut` write replays a world that never had it;
  seating completes on the session's first frame so activation lands on GGRS
  frame 1 where nothing can rewind across it; a confirmed lifecycle commit
  rebases mid-run and resets execution counters.)
* any `ambition_actors` decomposition. See
  [api-growth-method.md](api-growth-method.md) §4 for what authorises it.
* the capability-composition doctrine. It is *derived* at the end, not written
  at the start (ADR 0031, Alternatives).
