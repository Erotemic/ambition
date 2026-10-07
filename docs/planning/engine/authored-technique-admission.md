# Authored techniques: installed support, checked flow and activation

**Scope:** move-scoped technique admission and execution: installed support,
the effect-reference traversal, `TechniqueFlow` version 1 and revision
activation. It does not define an encounter or dialogue sequencer.
**Packets:** A11 and A12 in [the packet catalog](actor-monolith-work-frontier.md).
**Related:** [authoring and tools](authoring-and-tools.md),
[extension model](extension-model.md). **Priority:** [the queue](../queue.md).

**State:** A11a, A11b and A12a are landed. A12b is half landed. A11c is open.
Move-contact attribution (`landed_hit` crediting the move that happens to be
playing, Q101) is tracked in
[`queue.md`'s A12 row](../queue.md#a12--finish-move-contact-attribution-and-reflection-identity),
not here.

## Decisions

- An authored move becomes executable only after it is checked against the
  selected capability profile. The typed installation that supplies a
  technique handler also supplies its support declaration. Omitting a parameter
  schema cannot make a misspelled key valid.
- `TechniqueFlow` version 1 is a bounded, acyclic, move-local program. Its
  cursor and contact latches belong to one `MovePlayback` occurrence. It emits
  requests to existing domain owners and never owns body, projectile, capture or
  item state. No dynamic code loading, component access, recursion, blocking
  model calls or service lookup.
- Rejection preserves the last-good prepared definitions. A different
  mechanical revision activates at an explicit session or reconstruction
  boundary. No live arbitrary-world replacement.
- Trusted Rust providers are trusted code. A typed installer does not sandbox
  them.

## Trust levels and pipeline

| Input | Guarantee |
| --- | --- |
| Authored data and bounded `TechniqueFlow` | Passes structure, installed-support, parameter and reference validation before activation |
| Native Rust provider, validator or handler | Has application privileges; reviewed and tested, not proven safe by metadata |

```text
source decode and domain expansion
    -> collect final authored reference sites
    -> bind to a frozen installed-support profile
    -> structural + semantic validation
    -> checked flow / technique representation
    -> candidate prepared character or content revision
    -> headless fixture and review receipt
    -> explicit activation at a supported lifecycle boundary
```

Raw decoding can happen before App composition. Installed-profile validation
cannot: the support set exists only after selected capabilities declare it.

## Installed support (landed)

`ambition_entity_catalog::TechniqueSupport` is the pure authority. `declare`
refuses a second claim on a key and reports the key and the claimed owner; it
never compares check functions. The composition's table is
`InstalledTechniques` (`ambition_combat::technique`).
`combat_schedule::install_technique` adds the handler system and declares its
key in one call. `game/ambition_app/tests/installed_techniques_are_declared.rs`
asks the built app.

Refusals are distinct: unknown key (`TechniqueRefusal::Unknown`), unsupported
site (`WrongSite`), invalid parameters including non-finite numbers
(`BadParams`), parameters on a paramless technique (`UnexpectedParams`). A
disabled capability is `Unknown` by construction. `admit_at` refuses `NaN` and
infinities before any declaration's own predicate, because RON hydrates them and
a non-finite value poisons rollback-canonical state permanently. A domain rule
lives in one `problems()` that every authoring road reads.

`ParamSchemaRegistry` still exists in `ambition_entity_catalog`. It is not the
admission authority.

Rules:

- Paramless means the canonical empty parameter map, not ignored fields.
- Duplicate keys fail even when metadata matches. No last-write-wins, no
  function-address comparison. A replacement implementation is an explicit
  composition choice that omits the original offer.
- Freeze support before semantic preparation. A capability change makes a new
  profile and requires revalidation.
- A discovery catalog may describe available but absent capabilities; only the
  installed set authorizes a call.

## One exhaustive effect-reference traversal (landed)

`MoveSpec::effect_refs` returns every authored `EffectRef` with its
`EffectSite` path: `windows[w].volumes[v].on_hit`,
`windows[w].sustain_effect`, `events[e].kind`, `flow.nodes[n]`. Every level
destructures without `..`, so a new field is a compile error at the walk.
`admission_refusals` joins the walk to the installed table, and
`activate_staged_revision` refuses the whole candidate. The same traversal feeds
admission, forward and reverse references and discovery. Do not keep a second
list of effect-bearing fields.

Validate final expanded moves and keep prefab and override provenance. A
technique schema declares no nested references or enumerates them with paths.
Three parameters name another definition: `SummonRideParams::character_id`,
`DropBombParams::item_id` and `PlaceMineParams::item_id`. Bound a move's
expanded reference traversal to 1,024 sites and depth 16. These are engineering
limits, not measured costs.

## TechniqueFlow version 1

### Admission rules (A12a, landed)

1. A present flow has 1 to 256 nodes (`MAX_TECHNIQUE_FLOW_NODES`), starts at
   node 0 and every node is valid. An absent flow is an ordinary timeline move.
2. Every transition is in range. Edges are `u16`, the cursor's own width.
3. Every node is reachable from entry when both branches count.
4. The reachable graph is acyclic. Reject a back edge even when another branch
   reaches `Finish`.
5. Every `Wait` timeout is finite and strictly positive.
6. Every `Emit` passes installed-key, site, parameter and nested-reference
   checks.

`TechniqueFlow::problems` enforces 1 to 5, and `admission_refusals` refuses a
move whose flow reports any; `TechniqueFlow::successors` is the
one edge enumeration. Repeat windows are the supported way to repeat timeline
actions. A bounded repetition construct needs an explicit iteration budget; do
not add it as a back edge or a VM.

### Prepared representation (A12b)

Landed: `FlowNode` edges are `u16`, and `MovePlayback::spec` is an
`Arc<MoveSpec>`, so a playing move cannot edit its definition and the per-tick
graph clone is gone.

Open:

- prepared constructors are public and infallible; make them private and
  fallible;
- no prepared revision is pinned on the playback (`Arc` is a reference, not an
  identity; identity follows content binding);
- a move start deep-clones a `MoveSpec`.

Assessed 2026-10-07 as a fallback packet and NOT started. The three items were
read as decisions for a maintainer; they are the implementing agent's, and the
decision on the first is recorded below this list:

- *Constructors.* Landed 2026-10-07: `PreparedCharacterDefinition` is
  `#[non_exhaustive]`, so no crate but `ambition_characters` can build one by
  struct expression or functional update, and the barrier is "fallible" in the
  sense that matters: a structurally invalid flow is REFUSED at admission and its
  definition withheld (see the A12b invariant below; an earlier version of this
  note said the barrier reported and published anyway, and it did until this
  repair). Not done: the fields are still public to read and edit (about 130
  field reads across the tree, and tests edit prepared values).
- *Pinned revision.* Pinning it adds an occurrence field, so it moves the
  rollback registration and checksum inputs, and it needs the identity rule
  (content binding, not pointer address) chosen first. Read 2026-10-07, not
  run: the hazard it guards is a restored blob naming a move of a revision other
  than the live one (`MovePlayback::resumed` resolves the spec from the owner's
  current `ActorMoveset`). A publication cannot sit across a timeline:
  admission refuses a foreign or unhealthy one (`publication_boundary`), and a
  rebasable local timeline is stopped in the same step as the publication
  (`rebase_local_timeline_onto_the_new_generation`), so no snapshot taken before
  it is resimulated after it. The pin would matter to a host that rolls back
  across a reload, which the reload refuses today. The cost is the registration
  change; the benefit is a guard on a road that is closed.
- *Move-start clone.* The clone is the `.cloned()` of a `MoveSpec` out of
  `MovesetContract::moves: Vec<MoveSpec>` at the six selection sites in
  `moveset/mod.rs`, once per accepted move, not per tick. Removing it means
  `Vec<Arc<MoveSpec>>` in the contract: about 310 `.moves` uses, 78 `moves:`
  constructions and 260 `ActorMoveset` mentions. Measured 2026-10-07 (a
  throwaway test over `authored_movesets::tables()`, hot cache, 2,000 clones
  per move, dev profile, which is optimized here): 470 shipped moves, median
  0.81 us, p99 2.03 us, worst 2.29 us (`npc_bob/bulkhead_drop`). That is under
  0.015% of a 60 Hz frame for a move start, which happens at most once per
  accepted move. Pre-registered at under 20 us and held. A cold cache could cost
  a few times that and is not measured. On this number the clone is not worth
  the type change; revisit only if a profile of a real fight shows move starts.

**A12b: the invariant, and a correction (2026-10-07).** Authored data stays
permissive and a prepared value is trustworthy; the line between them is
ADMISSION, not a new type.

- ⛔ **CORRECTION.** An earlier version of this section said the production
  boundary already withheld malformed flows. That was false. `prepare_character`
  called `TechniqueFlow::problems`, pushed each problem onto the definition's
  `unresolved` list ("reported, not refused") and the definition was published
  anyway, so a flow with an edge past its node list, a never-expiring `Wait`, a
  cycle or an unreachable node became part of the prepared catalog, and the
  interpreter's step guard was the only thing standing between it and a fighter.
  `#[non_exhaustive]` on `PreparedCharacterDefinition` only stopped OTHER crates
  building one; it did not make the authorized road sound.
- **The invariant now:** a `PreparedCharacterRegistry` that the production roads
  publish holds no move whose `TechniqueFlow::problems` is non-empty. The check
  is `admission_refusals` (formerly `unsupported_authored_effects`; the refusal
  type is `AdmissionRefusal`), the one function the boot barrier's fixpoint, a
  revision and a revision over a candidate catalog all call, so there is no road
  that checks effects and skips flows. The admission unit follows the road:
  the barrier withholds exactly the CHARACTER whose move is refused (the rest of
  the cast publishes), and a reload revision is refused WHOLE with the last-good
  cast and its generation untouched. Preparation no longer reports flow problems
  (one authority for the fact); the refusal carries each problem with the
  character, move and rule, so an author sees every one at once.
- **Why not a stored `CheckedFlow` type.** Considered and rejected on cost and
  on what it would add. The interpreter reads `MovePlayback::spec: Arc<MoveSpec>`
  from `MovesetContract::moves: Vec<MoveSpec>`, so storing a distinct checked flow
  splits `MoveSpec` into authored and prepared forms (about 310 `.moves` uses, 78
  `moves:` constructions, 260 `ActorMoveset` mentions); a newtype checked per use
  makes the interpreter fallible at runtime, which the execution rules forbid.
  And it would add nothing here: every road that can publish a registry is the
  admission road (`insert_prepared` and the unchecked barrier are
  `#[cfg(any(test, feature = "test-support"))]`, enabled only from
  dev-dependencies), and every production `ActorMoveset` comes from a prepared
  kit, a prepared match seat, the repertoire fold of those, a persona derived from
  an action set, or the boss attack table (`flow: None`). Read from source, not
  proven by a census.
- **Witnesses.** `prepared_tests::technique_flow_admission`: for each structural
  rule (edge past the end, cycle, `Wait` that never expires, unreachable node, no
  nodes, and the `MAX_TECHNIQUE_FLOW_NODES` budget), a flow that round-trips through the authored RON format is refused
  with a diagnostic naming the move and the problem, its character is absent from
  the registry, a sibling character with a sound flow keeps that flow as authored,
  an unrelated character is untouched, and the same flow in a revision refuses the
  whole edit and leaves the generation. Poisoned by dropping the check, which
  fails the withholding and revision arms and leaves the floor and control arms
  green. The shipped corpus is held by
  `the_shipped_composition_withheld_nothing_at_its_barrier`.
- **What would reopen the type question:** a production road that builds a
  `MovesetContract` carrying a flow from something other than a prepared
  definition. Today there is none. If one appears, the shape is a prepared move
  table from character preparation with `TechniqueFlow` unchanged on the authored
  side.
- **Still open in A12b:** a prepared revision is not pinned on the playback
  (the road it guards is closed today, see above), the move start deep-clones a
  `MoveSpec` (median 0.81 us), and `PreparedCharacterDefinition`'s fields are
  still public to read and edit (tests edit them; no production code does).

The interpreter has a defensive step guard equal to the checked node count. A
missing node or an exhausted guard is an invariant failure: report it and
cancel through normal teardown.

### Execution semantics

| Operation | Behavior |
| --- | --- |
| Emit | Emit once on entry through the move-effect lane; advance |
| Branch | Test the occurrence's latch snapshot now; choose one successor |
| Wait already satisfied | Take the success edge, even when the timeout is also reached |
| Wait timeout | Take the timeout edge only when the signal is not satisfied |
| Wait unresolved | Keep cursor and elapsed time; return for this tick |
| Finish | Mark the flow done; the timeline, recovery and cancellation rules still own the move |
| Timeline end | Tear down normally even if a `Wait` is unresolved |
| Interruption, landing cancel, owner retirement | Normal move teardown; no detached flow |
| Repeat window wraps | Timeline windows rearm; the flow does not restart |

`MovePlayback::finished()` is `t >= spec.duration_s`: the timeline ends the
move. A broken flow costs authored intent, not a trapped fighter.

Advance the wait clock once per move update with the effective proper-time
delta after charge and hold adjustments. Accrue, then follow instantaneous
nodes; every taken edge resets elapsed wait. Contact has priority over timeout.
Contact signals are latched per move occurrence; two `Wait`s on one latched
signal can both succeed. Do not clear a shared latch on read.

### Delivery phases

Move and flow emission happen before hit resolution. A hit confirmed in
Resolve updates the latch and is read at the next eligible flow update, never by
a same-tick rerun. A handler whose phase has passed supports next-tick delivery
or refuses that site at preparation. An emitted operation that can outlive the
occurrence is either independently owned after acceptance or cancelled with the
occurrence, and it carries move-instance and owner identity. Do not add
projectile expiry, self-hit, control leases or named signals to the version-1
latch set.

## Candidate revisions and activation

A rejected edit leaves the active prepared registry, its generation, the
content binding and live `MovePlayback` references unchanged. Build the
candidate beside the active one. An accepted candidate records its source,
profile, mechanical content identity, validation result and preparation
version. Existing moves keep the definition they started with until a
supported boundary. In rollback, a new revision starts a new baseline.

**A11c (open):** exercise a real provider-defined technique through edit,
profile validation, headless test, review and explicit session-boundary
activation. The route rejects a stale apply base. Use an existing authoring
tool, not a new CLI.

## Witnesses

| Acceptance row | Guard |
|---|---|
| invalid or uninstalled calls cannot publish | `prepared::admit_and_finalize_cast` withholds the refused definition |
| a flow the interpreter cannot run cannot publish | `prepared_tests::technique_flow_admission::{a_character_whose_flow_cannot_run_is_withheld_and_the_diagnostic_names_the_move, a_revision_with_a_flow_that_cannot_run_is_refused_whole_and_the_last_good_cast_stays}` |
| nested references resolve | `prepared_tests::{nested_references, held_item_references}`, `the_techniques_that_name_other_definitions_declare_that_they_do` |
| rejection leaves the generation unchanged | `a_refused_revision_leaves_the_active_generation_unchanged` |
| shipped flows keep their traces | `every_shipped_flow_still_runs_the_trace_it_was_authored_for` |
| `Finish` keeps recovery | `a_finished_flow_leaves_the_move_playing_out_its_recovery` |
| `Wait` does not extend the move | `a_move_ends_on_its_timeline_with_its_flow_still_waiting` |
| a late connect is not credited to the next move | `a_late_connect_is_not_credited_to_the_move_that_replaced_the_one_that_earned_it` (`connected` and `blocked` only) |
| an edge wider than the cursor is refused | `an_authored_edge_wider_than_the_cursor_is_refused_at_the_boundary` |
| the shipped composition withholds nothing | `the_shipped_composition_withheld_nothing_at_its_barrier` |

`close_preparation_barrier_without_admission` is
`#[cfg(any(test, feature = "test-support"))]`, and only dev-dependency rows
enable `test-support`. A shipping app does not compile the bypass.

Diagnostics carry a stable code, stage, provider and profile, source definition
and path, expected contract, observed value and revision, sorted by source,
site and code.

## Forbidden regressions

- No executable service locator: `InstalledTechniques` maps a key to a
  declaration; the handler is an ordinary system.
- No scan of raw RON for strings named `key`.
- No last-write-wins registration.
- Do not advertise sandboxing or resource isolation from the 256-node bound.
- A loaded or procedural technique provider installs the same parameter and
  reference contract as its native counterpart. It does not rebuild the
  interpreter or keep a second validation registry.
