# Character preparation: the finalization barrier (plan, 2026-07-28)

**Status: PHASE A LANDED 2026-07-29.** Phases B and C are open. Written to
survive compaction. This is
the settled outcome of a three-round argument with GPT-5.6 over how to close
H1 — the finding that a catalog-playable character with no authored action set
works as the worn player and gets an EMPTY kit when seated as player two.

Read [queue-24h-2026-07-26.md](queue-24h-2026-07-26.md) section H for the
defects; this doc is only the plan.

## The invariant being bought

> No production body becomes live until every authoritative decision needed to
> simulate it has been made.

Most of the defects found on 2026-07-28 are one root cause — **spawn now,
repair later**: seated fighters spawned with placeholder action sets, countdowns
started before all seats existed, controls suppressed only after a fighter had
already simulated, GGRS and the roster independently sampling device topology,
registry generations changing without rebuilding projected bodies.

## Three things this design REFUSES, and why

Each was proposed, argued, and rejected. Recorded so they are not re-proposed.

- ⛔ **No `PreparedBodyBlueprint` stored beside `PreparedCharacterDefinition`.**
  A second stored type carrying action set / moveset / motion / hurtboxes /
  provider is R-g's *seventh authority* with a new name. A borrowed view or a
  function return is fine; a stored one is not.
- ⛔ **No `BodyUnderConstruction` marker.** Its safety would be `Without<..>` in
  every present and future body query — the identical shape to
  `apply_worn_character_gameplay` silently not matching seated bodies (which IS
  H1's root cause) and to the touch test querying `TouchActionButton` while the
  d-pad was not one. Where construction must be deferred, defer a **request**
  rather than publishing an incomplete body entity.
- ⛔ **Rollback never samples device discovery.** A plan assembled from live
  devices inside a rollback tick does not reproduce on resimulation. External
  topology is frozen BEFORE the session (`LocalSeatTopology`, landed); only
  deterministic selections and activation phase live inside it.

## The shape: a lifecycle type split (Choice A)

```text
CharacterDefinition + CharacterCatalog
              ↓
PreparedCharacterOverrides      preparation-only, runtime CANNOT import it
              ↓ composition finalization
PreparedCharacterDefinition     complete, immutable, the runtime authority
```

**Choice B — one resolver taking `(&prepared, &catalog)` — is unavailable**, and
this is the decisive fact. It works for the WORN path, which already holds the
catalog, and cannot reach the SEATED path: `project_prepared_character_definitions`
is engine-side and `engine.character-authority-is-app-local` forbids it a
`CharacterCatalog`. The broken path is the one B cannot serve.

**Choice C — mutate and seal the existing registry — was rejected for the same
reason as the marker.** Its safety condition is "no code reads the registry
before sealing": a load-bearing negative nothing enforces, spread over every
present and future reader.

**A is chosen because the phase split can be COMPILER-enforced.** If the partial
type is `pub(crate)` or lives where runtime cannot import it, "runtime cannot
read the partial one" stops being a review convention. Two types with the same
visibility would just be the blueprint objection wearing a new name — **that
visibility is the acceptance test for whether this was done or merely renamed.**

These are not competing authorities iff ALL hold:

1. `PreparedCharacterOverrides` is private to the preparation layer;
2. runtime crates cannot import or query it;
3. finalization CONSUMES or removes the overrides;
4. only the complete `PreparedCharacterDefinition` is published;
5. no runtime catalog fallback remains after publication.

### Authored emptiness must survive the fold

The single most load-bearing detail in the whole campaign:

```text
None        = inherit the catalog row
Some(empty) = an authoring DECISION; do not inherit
Some(value) = explicit replacement
```

Getting this wrong hands Sanic a punch. `Option<T>` can carry it, but an
authoring-specific `enum FieldOverride<T> { Inherit, Replace(T) }` makes it
harder for a future resolver to normalize `Replace(empty)` into absence.

## The barrier: `Plugin::finish`

Bevy runs every plugin's `build()` during registration, then every `finish()`
once all are ready. **It exists in `bevy_app` 0.18.1 and this workspace has ZERO
uses of it** (verified 2026-07-28). It answers the ordering hazard — a provider
registering its cast before the app installs `CharacterCatalog` would fold in an
empty row and bake the absence permanently — with a mechanical hook instead of a
documented ordering rule.

Contract:

1. providers contribute definitions and legacy catalog rows **only during
   `build`**;
2. one engine finalizer prepares characters during `finish`;
3. it publishes the complete prepared generation;
4. it removes or seals the preparation-only resources;
5. any later contribution **fails loudly**.

⚠ **Caveat: other plugins may also implement `finish`.** So authoring must be
defined as a `build`-phase operation. If contributions during `finish` ever
become necessary, the barrier moves to `cleanup`, which runs after all `finish`
calls.

⚠ **Manually driven apps must be audited — VERIFIED, not assumed** (read in
`bevy_app` 0.18.1 on 2026-07-29):

* `run_once`, the default runner, does `app.finish(); app.cleanup();
  app.update();`
* `ScheduleRunnerPlugin` calls `app.finish()` before its loop
* **`App::update()` does NEITHER** — it checks no plugin is mid-build and runs
  the sub-apps

This repository drives `App::update` by hand almost everywhere: every rendered
test, the external-consumer fixture, the rollback harnesses, the headless
acceptance runners. `capture_scene` and the new `OffscreenGpu` mode are fine,
because both go through `ScheduleRunnerPlugin` + `run()`.

Today that costs nothing — nothing in the workspace implements `finish` (zero
occurrences). It stops costing nothing the moment preparation seals its registry
there: production would get a sealed complete registry while every test and tool
kept only preparation fragments. Green tests, wrong game — and it would read as
a preparation bug rather than a lifecycle one.

✔ **LANDED AHEAD OF THE WORK THAT NEEDS IT:**
`ambition_runtime::{finalize, finalize_and_update}`, with a test pinning that a
hand-driven `update` leaves plugins unfinished. The audit now has one place to
point at instead of scattered hand-written `finish()` calls, and if Bevy ever
changes this the test says so rather than the helper silently becoming dead
weight.

⚠ **`finish` solves INITIAL composition only, not hot reload.** A later catalog
or provider change is a separate explicit transaction: new authoring generation
→ prepare completely → atomic generation replacement → bodies observe the
mismatch → complete baseline replacement. Never reopen the startup preparation
resource in place.

⚠ **Finalization is TRANSACTIONAL.** If one character cannot resolve, do not
publish a partially updated registry.

## Execution order (do not reorder — it is what makes a bisect possible)

Resolution and construction must not change in the same commit.

### ✔ Phase A — LANDED 2026-07-29

1. ✔ the partial type is `PreparedCharacterOverrides`;
2. ✔ **it is declared with NO visibility modifier at all** — not `pub`, not
   `pub(crate)` — inside `character_runtime::definition`. `presentation`,
   `seating` and `avatar::starting_character` are SIBLINGS of that module, so
   they cannot name it. The acceptance test passes by construction rather than by
   review. `definition_tests` was re-parented as a CHILD of `definition` for the
   same reason: widening the visibility so the tests could reach it would have
   been widening the thing that IS the design;
3. ✔ `None` vs `Some(empty)` survives — the fold matches on the `Option`, never
   on whether the set looks empty;
4. ✔ `CharacterPreparationPlugin::finish`, installed automatically by
   `try_register_character` so it is never a composition requirement;
5. ✔ `finalize_character(overrides, catalog)` — the fold that used to run per
   body inside `apply_worn_character_kit`;
6. ✔ one registry, built whole and inserted once (transactional);
7. ✔ the staged overrides are consumed by the fold and the barrier latches
   `finalized`;
8. ✔ both paths read `PreparedKit`. Spawn mechanics unchanged.

**`PreparedKit` is the one honest concession, and it is not a partial value.**
The host's code-side kit is built from the BODY's own `AbilitySet`, so no
per-character value can hold it. Naming that case (`HostCode` vs `Authored`)
beats a "complete" definition that quietly is not.

#### Two things that only showed up on contact with the tree

⚠ **`App::finish` re-runs EVERY plugin's `finish`, every time it is called.** It
does not track which have run — it walks the registry and sets
`plugins_state = Finished` (read in `bevy_app` 0.18.1). A barrier that CONSUMES
its input must guard itself, or the second call republishes an empty registry
over a good one. It did, and the whole cast vanished on a fixture's second step.
Pinned by `finishing_twice_runs_every_plugin_finish_twice`.

⚠ **The audit of hand-driven apps could not be an audit.** The external-consumer
fixture drives `App::update` by hand, never published its cast, and every
character silently fell back to the host compatibility kit — a consumer's
peaceful wanderer came out swinging the protagonist's sword. Fixing that one
fixture would have left every future one exposed, so the barrier gained a
`PreStartup` BACKSTOP calling the same idempotent finalizer. `PreStartup` runs
after every plugin's `build`, which is the entire ordering hazard `finish`
exists to remove, so it is not a weaker barrier — what `finish` still buys is a
registry that exists before ANY system runs.

#### What Phase A did NOT do

The catalog fold still runs at wear time for ids nothing REGISTERED — most of
the legacy cast. Those have no prepared value to disagree with, and a seated
fighter cannot be one (seating requires the registry). Condition 5 of the design
("no runtime catalog fallback") therefore holds for every character the campaign
is about, and not yet for the migration tail. Both readers are named in
`the-catalog-default-action-set-is-confined-to-one-file`.

### ✔ Phase B slice 1 — LANDED 2026-07-29

The second kit writer is gone. `project_prepared_character_definitions` wrote
seated bodies' action sets and movesets because seated bodies were believed not to
match `apply_worn_character_gameplay`.

⛔ **that belief was FALSE and had been repeated into three places** — a source
comment, the 24h queue, and the campaign inventory. `WornCharacter` is
`#[require(IdentityKit)]` and `BodyAbilities` arrives with
`AncillaryMovementBundle`, so both "missing" columns were present. Seated bodies
always matched the single writer; the second one was built on a diagnosis nobody
re-read.

The projection now serves only the population the derive genuinely cannot see:
bodies with NO `WornCharacter`, identified by `CombatTuning.sprite_character_id`.
The gate is that system's exact required-column set, not a proxy.

✔ **the `PreparedKit::HostCode` seated case is closed by this, and verified rather
than assumed.** `HostCode` is the one kit a per-character value cannot hold — it is
built from the BODY's `AbilitySet` — so while the projection was the writer a
seated fighter resolving to it got nothing. The derive has the abilities.
`a_seated_fighter_on_the_host_kit_is_not_left_empty_handed` drives it end to end
and goes red when the derive is removed from the fixture.

### Phase B remainder — consolidate construction

Still open: extract the common body construction, then delete seating
placeholders, path-specific baseline insertion, and per-field retraction.

### Phase C — generation invalidation (folds in H6)

Stamp bodies with `{ character_id, generation }`. A mismatch in EITHER triggers
atomic baseline replacement plus equipment reapplication — not several
projection systems patching selected fields.

## Related, deliberately after

- **`RangedExecution`** replacing `charges_projectiles` as the single switch
  BOTH the legacy projectile path and the moveset path consult — with
  preparation rejecting contradictory authored configurations. Useful only if
  it becomes the sole switch; otherwise it is a third name for the same implicit
  split.
- **Versus activation** (`PreparedMatchPlan`-shaped): validate all required
  participants, activate the roster atomically, publish `ActiveMatch`, and start
  the countdown from THAT rather than from the presence of a requested roster
  (queue H5).

## Provenance

Argued over three rounds; the reviewer accepted the three refusals and the A vs
C reasoning. One correction worth keeping: the reviewer attributed a
`BodyRestarted` concern to me that I never raised, and acknowledged it as
context contamination. It did not affect the A-vs-C reasoning, but it is why the
checkable claims in this doc were checked rather than taken on faith.
