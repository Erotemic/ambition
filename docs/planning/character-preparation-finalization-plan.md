# Character preparation: the finalization barrier (plan, 2026-07-28)

**Status: agreed design, not started.** Written to survive compaction. This is
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

⚠ **Manually driven apps must be audited.** Bevy's runner finalizes, but code
that drives `App::update` directly may need explicit `app.finish()` and
`app.cleanup()`. Audit: headless test app builders, the external-consumer
fixture, custom runners, tools calling `App::update`. Otherwise production gets
a sealed registry while a test or tool silently retains only fragments. **Prefer
one shared app-finalization helper over scattered manual calls.**

⚠ **`finish` solves INITIAL composition only, not hot reload.** A later catalog
or provider change is a separate explicit transaction: new authoring generation
→ prepare completely → atomic generation replacement → bodies observe the
mismatch → complete baseline replacement. Never reopen the startup preparation
resource in place.

⚠ **Finalization is TRANSACTIONAL.** If one character cannot resolve, do not
publish a partially updated registry.

## Execution order (do not reorder — it is what makes a bisect possible)

Resolution and construction must not change in the same commit.

### Phase A — unify the effective baseline, keep the spawn mechanics

1. rename/replace the current partial prepared type as
   `PreparedCharacterOverrides`;
2. restrict it to the preparation module or crate (the acceptance test above);
3. preserve `Inherit` vs explicit-empty;
4. add the character finalizer in `Plugin::finish`;
5. fold catalog fallbacks + overrides into complete
   `PreparedCharacterDefinition` values;
6. publish one immutable registry generation;
7. remove the partial resources after successful publication;
8. make BOTH worn and seated paths consume only the complete definition —
   **without changing how either spawns.**

At the end of Phase A: worn and seated resolve identically, `None` means one
thing everywhere, and no new constructor exists. A regression here is
attributable to authority resolution alone.

### Phase B — consolidate construction

Only once both paths are behaviourally identical: extract the common body
construction, then delete seating placeholders, the late-repair projection,
path-specific baseline insertion, and per-field retraction.

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
