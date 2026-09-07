# Actor-monolith decomposition — executable SCC frontier

> **Measured at `625fa79af45e6eff40cbefabd8cdae33c5b5e9db` on 2026-09-07.**
>
> This page is the executable D33 handoff. Git history is the execution diary.
> Do not append old carve narratives here.

**State:** ACTIVE.

Owners and scope:

- [`actor-monolith-decomposition.md`](actor-monolith-decomposition.md) owns the
  durable decomposition rules and the meaning of success.
- [`controlled-character-actor-kernel.md`](controlled-character-actor-kernel.md)
  owns what the residual actor/body kernel is allowed to contain.
- [`../queue.md`](../queue.md) decides when D33 runs.
- **This page says exactly what to do next.**

## Re-measure before every packet

Run:

```bash
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80
```

At the receipt above, the nontrivial SCCs are:

```text
11: abilities, actor_spawn, character_runtime, construction, control,
    features, items, projectile, session, shrine, world

 2: assets, character_sprites
```

The single edges whose removal shrinks the 11-module SCC are:

```text
->  9    1 ref   character_runtime -> features
-> 10    1 ref   features -> projectile
-> 10    2 refs  actor_spawn -> character_runtime
-> 10    2 refs  projectile -> features
-> 10    6 refs  shrine -> session
```

The raw reference count is a locator, not a priority. Prefer the direction that
restores ownership. In particular, `features -> projectile` is one reference,
but a feature observer reading projectile allegiance is a plausible downward
consumer edge; the two `projectile -> features` calls are the more suspicious
upward dependency.

After every packet:

1. re-run the graph;
2. verify the predicted SCC change;
3. inspect any surviving re-export or alias before declaring the cut ineffective;
4. update this page from the new graph before starting another packet.

If the SCC does not change as predicted, stop. Do not compensate by taking a
second unrelated cut.

## P1 — READY: sever `character_runtime -> features`

**Why first:** one production reference splits two modules out of the large SCC.
It is the highest-leverage current cut.

Current edge:

```text
character_runtime/live_match_clock.rs
    -> features::stocks_match::StocksMatchSettled
```

`LiveMatchTicks` needs the rollback-stable fact that the current match has
settled. It does not need the `features` module or the systems that decide a
stocks match.

### Ownership decision

Move the **settlement value type** downward to the stock/match vocabulary owner;
do not move `LiveMatchTicks` into `features` and do not add a callback/service
locator.

Preferred destination order:

1. `ambition_combat::stocks` if the type remains stocks-ruleset vocabulary — it
   already owns `MatchVerdict` and stock-count semantics;
2. a lower match vocabulary module only if that avoids adding a reverse
   dependency.

Keep `decide_stocks_match` and the ruleset systems where their policy belongs.
`SuddenDeathEntered` should move with the settlement vocabulary if the resulting
owner is coherent, but P1 does not require bundling unrelated code just to make
one commit larger.

### Required migration

- update the rollback registration to the new type path;
- update Smash/read-model consumers to the semantic type path;
- update snapshot codecs without changing the wire meaning;
- do not leave a `features` re-export that remains the discovery path for the
  moved type;
- a facade-level compatibility export is acceptable only if it does not restore
  the monolith dependency.

### Acceptance

```text
character_runtime -> features == 0
```

Expected SCC result:

```text
9: abilities, construction, control, features, items, projectile,
   session, shrine, world
2: actor_spawn, character_runtime
2: assets, character_sprites
```

Keep the existing live-match-clock rollback, pause/hitstop and settled-match
acceptance green.

## P2 — READY AFTER P1: peel projectile by removing `projectile -> features`

Current production calls:

```text
projectile/systems.rs
    -> features::ecs_hit_event_hits_breakable(...)
    -> features::ecs_hit_event_hits_boss(...)
```

The reverse edge is one read from feature perception to
`projectile::ProjectileAllegiance`.

### Direction

Preserve the useful direction:

```text
feature/observation code -> projectile state vocabulary
```

Remove the upward direction:

```text
projectile simulation -> feature implementation
```

The projectile step should emit/forward a generic hit fact or ask a lower combat
receiver/disposition seam. Breakable/boss feature policy then consumes that fact.
Do not teach the projectile domain a growing list of feature kinds.

Do **not** move `ProjectileAllegiance` merely because the one-reference cut is
cheaper. Move that type only if an independent ownership analysis says its
current module is wrong.

### Acceptance

```text
projectile -> features == 0
```

Expected result after P1 + P2:

```text
8: abilities, construction, control, features, items, session, shrine, world
2: actor_spawn, character_runtime
2: assets, character_sprites
```

Preserve same-tick projectile hit settlement, consumed-projectile termination,
breakable/boss hits and rollback determinism.

## P3 — READY AFTER P2: move lifecycle-intent vocabulary below `shrine`

The six `shrine -> session` references are one conceptual dependency:
`shrine.rs` records a confirmed-frame lifecycle operation through
`session::lifecycle_commit`.

The deterministic vocabulary involved is:

- `LifecycleIntent`;
- `RoomTransitionIntent`;
- `RoomReconstitutionIntent`;
- `Admission`;
- `PendingLifecycleCommit` and its record/query API.

A shrine is a **producer** of a lifecycle intent. It should not import the
session implementation that later commits it.

### Direction

Move the rollback-safe intent/slot vocabulary to the lower lifecycle owner —
prefer the existing shared lifecycle layer unless a dedicated lifecycle crate is
already justified by another customer.

Keep host/session commit execution in the session domain. The lower type may say
what is pending; it must not execute room/session policy.

Do not invent a shrine-specific transition request as an escape hatch. All
producers should continue to compete for the same earliest-sticky lifecycle
slot.

### Acceptance

```text
shrine -> session == 0
```

Expected result after P1–P3:

```text
7: abilities, construction, control, features, items, session, world
2: actor_spawn, character_runtime
2: assets, character_sprites
```

Preserve checkpoint save/resume, death reset, admission refusal and confirmed
commit rollback tests.

## P4 — READY AFTER P3: move actor placement-lowering specialization out of `world`

The four `construction -> world` references all point to the same adapter family
currently filed under `world/placements.rs`:

```text
ActorPlacementContext
LoweringCtx
LoweringFn
PlacementLoweringRegistry
```

`ambition_platformer2d_world` already owns the generic placement-lowering
machinery. These aliases specialize it with actor construction/catalog state.
That specialization is actor construction vocabulary, even though room staging
also consumes it.

### Direction

Move the actor-specific specialization to `construction` (or a lower dedicated
actor-construction vocabulary module if one already exists by then).

Expected dependency direction afterward:

```text
world/session/features -> construction placement vocabulary
```

not:

```text
construction -> world implementation
```

Moving the adapter is allowed to create a one-way `world -> construction` edge.
The objective is to remove construction from the strongly connected core, not to
make every consumer independent of construction vocabulary.

### Acceptance

```text
construction -> world == 0
```

Expected result after P1–P4:

```text
6: abilities, control, features, items, session, world
2: actor_spawn, character_runtime
2: assets, character_sprites
```

At this point **STOP THE MECHANICAL PEEL PHASE**.

## P5 — DESIGN CHECKPOINT: the six-module hard core

After P1–P4, no single edge shrinks the remaining six-module SCC. At the current
head its internal edges are:

```text
abilities -> control   2       control -> abilities 3
abilities -> features  1       control -> features  1
features  -> control   2
features  -> world    12       world   -> features  6
features  -> items     3
items     -> session   2       session -> items     5
items     -> abilities 1
session   -> world     5       world   -> session   3
session   -> features  4
session   -> abilities 3
```

The direct two-way knots are therefore:

```text
abilities <-> control
features  <-> control
features  <-> world
items     <-> session
world     <-> session
```

Do not continue by deleting whichever reference count is smallest.

### Required design pass

Before another code carve, classify every edge in those five two-way pairs as
one of:

```text
DATA/VOCABULARY    a lower type consumed upward
POLICY             one domain deciding another domain's result
SCHEDULING         concrete system ordering/installation
CONSTRUCTION       authored lowering/materialization
LIFETIME           session/reset/rollback ownership
```

For each pair, answer:

1. which side owns the fact or decision;
2. whether the two modules should actually become one package;
3. whether one direction is legitimate downward consumption;
4. which exact opposite-direction references violate that ownership;
5. what production poison proves the cut did not change semantics.

Write the result into this page as P5a/P5b/etc. **before implementation**.

Likely coherent groups to test, not conclusions to assume:

```text
abilities + control      actor-local control/action kernel
world + session          world/session lifecycle
features                 residual orchestration that should dissolve by owner
items                    residual adapters/policy after prior item carves
```

A good P5 result may choose to extract a two-module group together. SCC reduction
is evidence about boundaries; it does not require every top-level module to
become its own crate.

## Satellite SCCs — do not confuse “left the big knot” with “already separable”

P1 is expected to leave a new two-module SCC:

```text
actor_spawn <-> character_runtime
```

That is a successful peel: the pair no longer participates in the central actor
knot. It is not evidence that either module should immediately become its own
crate. Treat the pair as a grouped actor-construction/runtime package candidate
and decide its internal seam only when a package carve needs one. Do not delay
P2–P5 to make this pair acyclic.

The current tree already has another independent two-module SCC:

```text
assets <-> character_sprites
```

Current directions there are:

```text
assets -> character_sprites   sprite enumeration/loading
character_sprites -> assets   platformer asset catalog/ids
```

Treat it as a **grouped extraction question**, not a prerequisite to the actor
kernel peel. Before changing it, decide whether the two modules are one asset
preparation domain or whether the catalog/loader dependency should be inverted.
The existing external `ambition_character_sprites` crate owns pose/geometry
derivation, so do not dump asset loading into it merely because the names match.

## Post-carve checks

For every D33 packet:

```bash
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80
python3 scripts/modules_md.py
python3 scripts/check_doc_links.py
python3 scripts/check_planning_citations.py
```

Also run the focused production tests named by the packet. Run broad Rust gates
only when the environment supports the repository's target/disk preconditions.

If a carve adds or changes a crate boundary, also run the capability/absence and
compile-cost ratchets required by the owning planning pages. Do not re-freeze a
red baseline merely because a carve changed topology.

## Definition of progress

D33 progress is one of:

- a module/group leaves the large SCC for a coherent ownership reason;
- a reverse authority edge becomes one-way downward consumption;
- a residual catch-all (`features`, `items`, etc.) loses a responsibility to its
  actual owner;
- an explicit design checkpoint proves two modules belong together.

Line count, number of crates, number of dependencies and raw edge-reference
count are supporting measurements. None is the goal.
