# Character preparation: the finalization barrier (plan, 2026-07-28)

**Status: PHASES A, B AND C LANDED 2026-07-29.** Written to survive compaction.
What remains is named at the bottom under "Related, deliberately after" — match
activation's repair half, and the `RangedExecution` switch. This is
the settled outcome of a three-round argument with GPT-5.6 over how to close
H1 — the finding that a catalog-playable character with no authored action set
works as the worn player and gets an EMPTY kit when seated as player two.

Read `queue-24h-2026-07-26.md` (retired 2026-08-07; read at `9a66996ce~1`) section H for the
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
`ambition_platformer2d_runtime::{finalize, finalize_and_update}`, with a test pinning that a
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

Still open: extract the common body construction, and remove path-specific
baseline insertion and per-field retraction.

⛔ **DO NOT "delete the seating placeholders". This step, as originally written,
would reintroduce H1.** Checked against the code on 2026-07-29 rather than
inherited:

`apply_worn_character_gameplay` takes `&mut Name`, `&mut ActionSet`,
`&mut ActorMoveset`, `&mut IdentityKit` and `&mut MotionModel` as REQUIRED,
non-`Option` query columns. A body missing any of them does not match the single
persona writer AT ALL — it wears a character and derives nothing from it, which is
precisely the failure this campaign exists to remove.

So seating's `ActionSet::default()` and empty `ActorMoveset` are not residue and
not a second opinion. They are the components the writer needs to exist before it
can overwrite them, and the comment beside them says so. The thing that WAS a
second opinion — the projection writing kits — is already gone.

The honest version of this step: extract the shared construction so both paths
insert the same required columns in one place, rather than delete the columns.

✔ **the GUARD for it landed first** (2026-07-29), because it is the part that
would have caught H1:
`a_seated_body_matches_every_column_the_persona_writer_requires` seats a fighter
and asserts it matches a query with the derive's exact required-column set. It is
STRUCTURAL on purpose — it does not ask whether the fighter got the right kit, it
asks whether it is even VISIBLE to the writer that hands kits out. Red-probed by
removing one placeholder column.
That guard makes the extraction safe to do later: if the shared constructor drops
a column, a test says so instead of a fighter silently losing its persona.

✔ **first extraction landed 2026-07-29: the HOST KIT's moves.**
`PlayerSimulationBundle::from_scratch` built them inline and the persona derive
built them again in `build_host_code_moveset` — two places deciding what the
protagonist swings, agreeing only because a comment said so. Both call the one
constructor now, pinned by
`the_spawned_and_the_rewarn_host_kit_are_one_construction`.
⚠ that test needed THREE attempts before it could fail: comparing move ids missed
the SFX stamp entirely, comparing the whole contract was vacuous because
`AbilitySet::basic()` grants no attack, and only granting attack made the probe go
red. The divergence it now catches is the entire robot-blade cue family.
✔ **per-field retraction is gone (2026-07-29).** The projection kept three loose
booleans — `granted_hurtboxes`, `granted_movement_tuning`, `granted_posed_body` —
each with a field declaration, a grant expression and a retract branch. Three
places to edit per fact, and forgetting one is SILENT: the body keeps a retired
hurtbox document, or loses a live one.
They are one `GrantedBodyFacts` record now, with one producer (`of`) and one
consumer (`retract`). `retract` DESTRUCTURES it, so adding a fact is a compile
error until it is handled — verified by adding a field and watching
`E0027: pattern does not mention field`. The coupling was real, so it is enforced
rather than remembered.

✔ **path-specific baseline insertion is gone (2026-07-29), and PHASE B IS
CLOSED.** Both paths built `IdentityKit { action_set, moveset }` as a struct
literal — the spawn bundle and the persona derive. Harmless at two fields; the
failure it invites is a THIRD field added in one place and defaulted in the other,
and a defaulted baseline silently REVOKES the body's own kit the first time it
picks anything up. The spawn site's own comment already warned about exactly that
outcome. `IdentityKit::of` is the one constructor now.

### What Phase B actually consisted of, once the false step was removed

1. delete the second kit writer (the projection no longer writes persona kits);
2. prove the seated `PreparedKit::HostCode` case, which slice 1 closed as a side
   effect;
3. a STRUCTURAL guard that a seated body matches every column the single writer
   requires — the guard H1 needed;
4. one construction for the host kit's moves;
5. one record for what the projection granted, with retraction destructuring it so
   a new fact cannot compile until it is handled;
6. one constructor for the identity baseline.

⛔ and NOT "delete the seating placeholders", which the step originally said and
which would have reintroduced H1 — those placeholders are the required columns
from (3).

### ✔ Phase C — generation invalidation (folds in H6) — LANDED 2026-07-29

Bodies are stamped with `{ character_id, generation }` and a mismatch in EITHER
re-derives. Two records, not one, because two writers own two different things:
`ProjectedCharacterKit` for the body facts the projection grants, and
`avatar::PersonaBaseline` for the kit the persona derive applies.

⚠ **that split IS the lesson.** The first version had the projection stamp a
single marker for both — so a cast replacement recorded a persona body as CURRENT
while the writer that owns its kit had not run and, filtered on
`Changed<WornCharacter>`, never would. The body read as up to date and nothing
revisited it, which is worse than a missed update. One writer, one record, each
stamped only after its own work lands.

⚠ **and the generation is per PUBLICATION, carried across rebuilds.** It used to
advance per insert, which was fine while registration inserted one character at a
time; the barrier publishes the whole cast into a fresh `Default` registry, so a
counter starting from its own zero would republish generation 1 over a body
stamped with generation 1 from the previous cast. A monotonic counter that
restarts is worse than no counter, because it looks like one.

⚠ **`PersonaBaseline` is rollback state**, registered and probed. A rewind that
restores an earlier `WornCharacter` while leaving the record at the abandoned
future's id makes the derive skip, and the resimulation runs a fighter with
somebody else's moves.

## Related, deliberately after

- ✔ **`RangedExecution`** replaced `charges_projectiles` on 2026-07-29, and the
  condition this entry set — *useful only if it becomes the sole switch* — is
  what made it worth doing. It could be: HOW a body fires decides four things
  (which presets fold into the moveset, whether the blade SFX is stamped, and
  whether the body carries `ChargesProjectiles`), and they were decided in three
  places in two spellings. One enum decides all four, and the two moveset
  builders collapsed into one function that takes it.
  ⚠ what this entry ALSO asked for is not done and is deliberately not being
  done: *preparation rejecting contradictory authored configurations*. There is
  no contradiction to reject yet — a `HostCode` kit's action set is built by the
  host, so content cannot author a ranged preset into it. A validator for an
  unreachable state is a test of itself. It becomes real the day content can
  author over a host kit.
  ⛔ **WRONG, and it was already reachable when this was written.** A `HostCode`
  kit carries `authored_moveset` precisely so a character can take the host's
  capabilities and bring its own timelines — and if that moveset binds `ranged`,
  the press is owned by the host's charge-projectile path AND by the moveset
  verb. Both fire. `default_player_action_set` always sets
  `ranged: Some(bolt)` and `simple_ranged`'s fire event samples the owner's live
  `ActionSet.ranged`, so it is two projectiles, not one that quietly does nothing.
  ✔ **CLOSED 2026-07-29, and reporting was not enough.** The first fix logged the
  contradiction and published the kit anyway, which named the bug without
  preventing it. `revoke_host_owned_ranged` now strips the whole ranged verb
  FAMILY at finalization — `ranged`, and every `ranged_*` the directional chain
  can resolve — so the invalid ownership cannot reach a body and
  `RangedExecution::HostCharge` is the sole authority for that press. The moves
  themselves survive; a timeline nothing presses is inert, and deleting authored
  content on a reachability argument is the more expensive mistake.
- ◐ **Versus activation** — the publication half LANDED (`ActiveMatch` names the
  seated bodies and the topology they were activated against; the countdown
  starts from it; `MatchSeated(bool)` is gone). What remains is now better
  understood, and worse:
  ⛔ **`ActiveMatch` is authoritative SIMULATION state and is not rollback
  state.** It gates two behaviours inside the sim — seating returns early the
  moment it exists, and the countdown advances only when it does — it holds live
  `Entity` values, and it is mutated from ordinary `Update` by topology
  reconciliation. Only `VersusMatch` is rollback-registered. A rewind across the
  activation tick returns the fighters to a pre-activation frame while the
  resource survives, so seating believes the match is already live and the
  countdown advances against stale or nonexistent participants (GPT 5.6,
  2026-07-29).
  ⛔ **treat this as a hard blocker on versus under rollback.** It is the same
  deterministic-boundary problem that was fixed for `VersusMatch`, moved one
  layer outward. Two honest routes: finish activation BEFORE the rollback session
  begins, or make the activation selection rollback-owned and reconstruct the
  participant references deterministically. Clone-snapshotting raw `Entity`
  values is not a third route — it needs correct remapping and does nothing about
  the `Update` mutation.
  ✔ **the ENTITY half is closed (2026-07-29).** The review's own rule — *"do not
  snapshot raw entity references without a complete remapping and reconstruction
  contract"* — has a cheaper reading than building the contract: stop
  snapshotting. `ActiveMatch` now holds a seat COUNT and a topology generation,
  both plain data, and `match_participants` derives the cast in seat order from
  the `MatchSeat` component on the bodies themselves. That rewinds because the
  bodies do, and it cannot go stale because there is nothing to keep in step.
  ⚠ it turned out there was exactly ONE production reader of the entity list, and
  it was a `warn_once!` counting them. The list had been carrying a rollback
  hazard to serve a log line.
  ▢ **the LIFECYCLE half is still open**, and it is the part the review calls
  preferred: activation happens inside the rollback schedule (seating runs on
  `sim`, which under a session IS `GgrsSchedule`), so a rewind across the
  activation tick still leaves a non-rollback latch saying the match is live
  while the fighters are pre-activation. The failure is now bounded — a count and
  a generation cannot dangle, and the disagreement is DETECTABLE by comparing
  `seats()` against `match_participants()` — but it is not gone. The fix remains
  route one: freeze the topology, prepare every fighter, construct the match,
  publish, and start the session afterward.
  ⚠ **and here is the blast radius, measured rather than assumed.** Grepping every
  caller of `start_sync_test_session` / `install_rebased_sync_test_session`: the
  dev **rollback observatory**, the **hot-reload rebase** in `dev_runtime`, two
  demo rollback-restore tests, the sim harness, and the external-effects tests.
  **The versus route starts no rollback session** — consistent with the standing
  decision that netplay waits for Smash. So a player cannot reach this today; a
  developer with the observatory open on the versus stage can. That is the
  difference between "the shipped game is broken" and "a dev tool can reach a
  state the design has not shipped yet", and the row should not be read as the
  first.
  → whoever takes this: the reproduction is the observatory over
  `versus_gameplay`, and the engine already documents the general principle one
  level down — `warn_if_no_world_to_rewind` says outright that *"a rollback cannot
  undo `Commands`, so the frames that build the room will mismatch on every
  resimulation"*. Seating IS construction. The same sentence applies to it, and
  the warning does not yet cover it because the rollback seam has no business
  knowing what a match is.
  ⚠ and the Y′9 stamp repair runs in that same `Update`, on that same resource.
  It happens to be safe: it is idempotent and derives its value from the frozen
  topology, so it re-applies harmlessly after a rewind. That is a property worth
  stating rather than discovering.

## Provenance

Argued over three rounds; the reviewer accepted the three refusals and the A vs
C reasoning. One correction worth keeping: the reviewer attributed a
`BodyRestarted` concern to me that I never raised, and acknowledged it as
context contamination. It did not affect the A-vs-C reasoning, but it is why the
checkable claims in this doc were checked rather than taken on faith.
