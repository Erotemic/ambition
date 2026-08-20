# World geometry and spatial semantics

**State:** ⭐ **WARRANTED 2026-08-15 — a trigger is MET (see the correction
below); SCHEDULING is open.** Not promoted into an execution row yet, and that is
an ordering call rather than a verdict on the plan. Planning-only extension of existing Engine 1.0 programs. This document
is not execution authority and does not promote itself into `queue.md` or
`tracks.md`.

## ⛔⛔ CORRECTION 2026-08-20 — trigger #5 is REAL and names the WRONG LAYER

⭐ **promoted to D169 on 2026-08-20** after a stranding census found this document
referenced by nothing in the repository — 1 of 267 planning docs. That is the
routing failure `tracks.md` exists to end, recurring five days after it was
measured clean.

⇒ and measuring the trigger before executing it changed the work. **The MECHANISM
is already generic.** `apply_world_hazard_gate`
(`platformer2d_core/src/movement/kernel.rs:422`) computes a per-axis distance past
the world AABB and emits `ResetCause::LeftTheWorld`, and its own header says
*"policies flag; the body's owner applies its reset policy."* Smash loses a stock,
Mary-O respawns, Ambition calls it out of bounds — three meanings over one engine
fact, which is this plan's principle 1 **already satisfied**. `blast_margin`'s doc
says the same thing in its first sentence: *"a platformer's pit depth and a
platform fighter's blast zone — the same number, and it belongs to the STAGE."*

⛔ **so there is no bespoke platform-fighter PRIMITIVE to remove, and the section
below claiming one is wrong about the layer.** What is genre-specific is the WORD.

⭐⭐ **and the word leaks furthest where an author meets it.** The LDtk converter
reads the authored key by that name, and **all six shipped worlds carry all three
fields in `defs.levelFields`** — while **zero levels author a value.** Eighteen
schema entries, no data. Every author of every world in this project is shown
three platform-fighter fields nobody has ever filled in.

⇒ the slice is a RENAME, it costs no content migration, and the authoring half is
a maintainer decision because the `.ldtk` files are hand-edited:
[`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md) §24.
⛔ **do not do the Rust half alone** — the struct field and the authored key are
one name, so renaming one needs a mapping, and a mapping is the shim this project
refuses.

⭐ **the lesson this plan already taught, applied to itself**: *"the measurement
was sound; the sentence after it was not."* Trigger #5's measurement — three
platform-fighter fields in every game's `World` — is exactly right. The sentence
after it, that the engine provides a bespoke primitive, is not.

## ⭐⭐ COORDINATOR TRIAGE 2026-08-15 — the strongest trigger was TESTED and did NOT fire

⭐ **the `BlockKind` diagnosis in Symptom B is CORRECT and was re-measured against
HEAD** (`crates/ambition_platformer2d_core/src/world.rs:20`). The enum really does
mix contact law, traversal permission, world consequence and contact affordance on
one axis, and a closed taxonomy of that shape really does grow `OneWayConveyor` and
`BlinkWallRebound` as customers arrive. Nothing below is a disagreement with the
analysis.

## ⛔⛔ CORRECTION 2026-08-15 — the triage below CHECKED ONE TRIGGER OF FIVE

⭐⭐ **trigger #5 is MET, has been for a while, and is shipping.** Jon named it:
the blast zone is an ENGINE-level concept, which is a smell, and Smash should be
able to say *"this region is my blast zone"* over generic geometry instead of the
engine providing a bespoke platform-fighter primitive.

Measured at HEAD, and it is worse than "a game concept leaked upward" —
`crates/ambition_platformer2d_core/src/world.rs`:

```text
:888   pub blast_margin: f32,            + a serde default and DEFAULT_BLAST_MARGIN
:900   pub side_blast_margin: Option<f32>,
:906   pub ceiling_blast_margin: Option<f32>,
       three builders, an LDtk lowering pass, and a render overlay
```

⇒ **every game built on this engine carries three platform-fighter fields.**
Mary-O's `World` has a `blast_margin`. Trigger #5 says *"a provider game needs a
spatial noun and must edit a closed engine switch"*; this is that, inverted and
worse — the Smash-specific noun is already inside the engine, permanently, for
everyone.

⭐ **and the generic fact underneath it is a boundary region with a CONSEQUENCE.**
Smash calls it a blast zone and loses a stock; Mary-O calls it a pit and respawns;
Ambition calls it out-of-bounds. Engine owns the geometry, game owns the meaning —
which is this plan's own principle 1. ⭐ **the slice is shaped like a DELETION**,
which is the strongest form a first customer can take.

⛔ **the error to learn from, since it is the cheap kind to repeat:** the triage
below tested trigger #1, found it negative, and generalised to all five. The
measurement was sound; the sentence after it was not. **A negative on one trigger
is not a negative on the plan.**

⚠ **this correction is about JUSTIFICATION, not schedule.** The plan is warranted;
whether it outranks the other engine work in flight is a separate ordering question
and is left open deliberately. ⛔ do not read "warranted" as "next".

---

⚠ **the paragraph below is the superseded reasoning, kept because the error is
more instructive than the conclusion.**

⛔⛔ **this plan's own trigger #1 ran as a real experiment today and came back
NEGATIVE, and that is the finding.** A worker authored a complete new Mary-O room
through LDtk end to end — terrain, a bonk row, a hidden block, six enemies, coins,
a moving platform, a warp-tube pair, spawn and flagpole. It hit **no** surface
combination `BlockKind` could not express, and asked for **no** new species. Its
friction was somewhere else entirely: discovery of legal field values,
hand-kept registration sites, and a silent `Custom(…)` fallthrough on
`String`-typed fields.

⇒ ⭐ **an untested trigger is a hypothesis; a tested one is evidence.** The
authoring customer this plan named as its strongest reason to start does not want
compositional surfaces yet — it wants to *see what it may place*. That is the
authoring-and-tools program, and it is where the session budget goes.

The other four triggers are also unmet at HEAD: no second surface-motion customer
(`kinematic-world-objects` is resting), no zone family repeating identity plumbing,
no inspection feature blocked on a geometry descriptor, no provider game blocked on
a closed switch.

### ⭐ What is done instead — arm the tripwire, do not run the campaign

⛔ **do not schedule S0–S5.** The cheap, correct action is to put the trigger where
the person who trips it will be standing: a note at `BlockKind` itself saying that
adding a **combinatorial** species — a variant that is two existing axes crossed —
IS trigger #1, and pointing here. ⚠ a plan that fires when someone reaches for
`OneWayConveyor` is worth more than a campaign that runs before anyone has.

⇒ **re-triage when that note is tripped**, or when any other trigger above is
measured. ⛔ not because the analysis looks compelling — it already does, and that
is precisely why it needs a customer rather than an advocate.

## Goal

Give the engine an owned architectural program for **shared spatial geometry and
compositional surface semantics** without prematurely forcing unrelated domains
through one universal runtime object model.

The governing principle is:

> **Semantic convergence is required; type unification is not assumed.**

The evidence now warrants convergence work around geometry identity, geometric
queries, surface/contact properties, authoring, validation, provenance and
inspection. It does **not** yet prove that water, gravity, loading zones, camera
zones, stage boundaries and collision surfaces should all become instances of a
single `SpatialRegion`, `SpatialBoundary`, or other universal runtime type.

This plan exists to answer what genuinely deserves to be shared, preserve typed
domain semantics above that shared substrate, and let real authoring/runtime
customers determine how far the convergence should go.

## Relationship to existing plans

This plan extends existing work. It does not replace or reopen it.

- [`authoring-and-tools.md`](authoring-and-tools.md) owns agent-native discovery,
  semantic inspection, intent-level mutation, preparation diagnostics and concise
  review artifacts. This plan contributes a coherent spatial vocabulary to those
  surfaces rather than building a second tooling stack.
- [`ldtk-authoring-and-world-tools.md`](ldtk-authoring-and-world-tools.md) owns
  LDtk as a first-class spatial backend and already requires provider-owned
  spatial nouns/facets to be extensible without editing closed engine switches.
  This plan investigates the runtime/preparation semantics those authored facets
  should lower into.
- [`kinematic-world-objects.md`](kinematic-world-objects.md) owns deterministic
  moving world geometry. Its measured `Block::velocity` split into geometry
  displacement plus surface drag remains there. This plan asks the broader
  question of how surface motion composes with collision law, traversal and
  contact affordances.
- [`collision-and-ccd.md`](collision-and-ccd.md) owns collision correctness,
  casts, CCD and any measurement-justified broadphase. This plan must use those
  primitives rather than create a competing collision kernel or speculative
  spatial index.
- [`inspection-diagnostics-and-workbench.md`](inspection-diagnostics-and-workbench.md)
  owns composed read-only discovery and visualization. This plan makes spatial
  geometry discoverable/inspectable without moving semantic authority into the
  inspector.
- [`architecture.md`](architecture.md) remains the canonical ownership guide. Any
  carve or new reusable crate must satisfy the same dependency, authority and
  external-consumer criteria as other Engine 1.0 work.

The integration/coordinator agent decides whether this proposal is linked from
umbrella plans, added to `tracks.md`, promoted into `queue.md`, folded into an
existing program, or held until another customer arrives.

## Why this plan exists

The current engine has multiple independent spatial customers and a collision
surface taxonomy whose members encode unrelated semantic dimensions.

The problem is not that these features have different names. The problem is
that repeated low-level work is beginning to appear around:

- shape/extent representation;
- stable spatial identity;
- transforms and reference frames;
- authoring provenance;
- preparation and validation;
- containment, overlap and crossing tests;
- surface motion;
- contact directionality;
- traversal permissions;
- contact affordances/responses;
- debug/inspection visualization;
- editor/LLM schema discoverability.

When each domain reinvents these pieces, adding a feature is more expensive than
its semantics justify, the authoring surface accumulates historical special
cases, and generic engine layers are tempted to learn game-specific nouns.

Two concrete symptoms motivate the investigation.

### Symptom A: repeated spatial geometry families

Current customers include at least:

| Customer | Geometry shape | Geometric question | Semantic owner |
|---|---|---|---|
| water | region | overlap/depth/surface | movement/water |
| climbable | region | overlap/boundaries | movement/climb |
| loading zone | region | crossing/containment | room transition |
| gravity zone | region | containment | gravity |
| camera zone | region | containment | presentation/view policy |
| solid block | area/surfaces | sweep/contact | collision |
| one-way block | surface | directional contact | collision |
| blink wall | surface/area | traversal permission | blink |
| pogo surface | surface | contact affordance | movement/combat |
| rebound surface | surface | contact response | movement |
| stage/world boundary | extent/boundary | containment/exit | world/ruleset policy |

These customers prove that spatial semantics are repeated. They do not prove one
runtime storage type.

In particular, domain APIs such as `water_at(...)` or `climbable_at(...)` are
valuable because they return domain-meaningful answers. Convergence must not
replace useful typed APIs with a generic `RegionKind` switch merely to claim a
single abstraction.

### Symptom B: `BlockKind` mixes unrelated semantic axes

The current block taxonomy includes concepts equivalent to:

```text
Solid
OneWay
BonkOnly
BlinkWall
Hazard
PogoOrb
Rebound
```

These are not alternatives along one dimension.

They mix:

```text
contact law
    full solidity
    one-way contact
    opposite-side contact

traversal permission
    blink permeability

world/domain consequence
    reset-to-spawn

contact affordance/response
    pogoable
    rebound impulse
```

At the same time, `Block::velocity` already has a measured second meaning:
geometry displacement and tangential/surface motion coincide for a moving
platform but diverge for a conveyor.

A closed taxonomy naturally grows combinatorial species such as
`OneWayConveyor`, `OneWayReboundConveyor`, or `BlinkWallRebound` as customers
arrive. The desired direction is compositional semantics rather than enumerating
every useful combination.

The exact Rust representation remains an outcome of this plan, not an input.

## Architectural principles

### 1. Share geometry facts; keep semantic ownership typed

A domain may reuse shared shape, identity, transform, provenance and geometric
operations while continuing to expose domain-specific data and query results.

A plausible layering to investigate is:

```text
prepared spatial geometry
    identity
    shape
    transform/reference frame
    provenance
    generic geometric operations
        |
        v
domain-owned semantics
    water
    gravity
    loading
    camera
    traversal
    contact response
    ruleset boundary policy
        |
        v
domain-specific query/result APIs
```

This is a hypothesis to test, not a required class hierarchy.

### 2. No central behavior enum

Do not solve the problem with a new authoritative census such as:

```text
RegionBehavior::Water
RegionBehavior::Gravity
RegionBehavior::LoadingZone
RegionBehavior::BlastZone
...
```

or:

```text
SurfaceBehavior::Solid
SurfaceBehavior::OneWay
SurfaceBehavior::Pogo
SurfaceBehavior::Rebound
...
```

A new domain must be able to own its semantics without editing a generic
foundation switch. Shared infrastructure may expose derived discovery descriptors
through the inspection/authoring programs, but those descriptors are not semantic
authority.

### 3. Composition should correspond to independent facts

If two properties can vary independently for real customers, authoring/runtime
representation should make that independence visible where practical.

Candidate axes to investigate include:

```text
geometry

contact policy
    full
    one-sided/directional
    nonblocking
    ...

surface motion
    geometry displacement
    tangential/surface transport

traversal facets
    blink permeability
    future capability-specific permeability

contact affordances/responses
    pogoable
    rebound
    future typed contact traits

domain consequence
    reset
    damage
    transition
    ...
```

This list is a census scaffold, not a prescribed type layout. Domain consequences
may remain entirely outside the collision geometry representation.

### 4. Authoring is a first-class acceptance customer

A good abstraction is not only easier for runtime code. It should make spatial
content easier for a human or LLM to discover, author, validate and inspect.

The target experience is that an author can express a combination such as:

> a one-way moving surface that transports supported bodies and is pogoable

without needing a bespoke block species whose name encodes every property.

Likewise, typed spatial entities such as water, gravity, loading and camera zones
should be able to share coherent geometry/reference/provenance conventions while
retaining their domain-specific schemas and meanings.

### 5. Inspection consumes authoritative geometry

Invisible spatial features should not require duplicate debug-only rectangles to
be understandable.

Where shared geometry exists, inspection/debug tooling should consume the same
prepared/runtime geometry and identity that simulation uses, then ask the owning
domain for semantic descriptors when needed.

This does not require the renderer or inspector to know game nouns such as
"blast zone".

### 6. Performance structures follow measurement

Do not turn this plan into a spatial-index campaign.

A shared geometry representation may later enable one broadphase/index to serve
multiple query families, but that is a possible optimization consequence, not a
reason to force semantic unification. `collision-and-ccd.md` remains the owner of
measurement-driven collision acceleration.

## Deliberately unresolved questions

The following are explicitly **not decided** by creating this plan:

- whether a runtime `SpatialRegion` type should exist;
- whether boundaries and filled regions share one representation;
- whether domain spatial data is stored centrally, per-domain, or in ECS
  components;
- whether shared geometry exists only at preparation/inspection boundaries or is
  also the runtime storage representation;
- whether all customers need durable spatial identity;
- whether enter/exit events belong in shared infrastructure or are derived by
  domains;
- whether shape support should remain AABB-first or generalize to polygons/other
  primitives;
- whether collision surfaces and non-colliding regions share any runtime type;
- whether a generic named spatial anchor/pose is useful;
- whether stage/world out-of-bounds policy should remain an extent-derived policy
  rather than become authored region geometry;
- whether any new reusable crate is warranted.

No implementation should answer these globally before the customer census.

## Spatial anchors: census only

There are plausible related customers for a durable named pose/anchor concept:

- default spawn;
- room-transition arrival;
- checkpoint/recovery position;
- multiplayer start position;
- encounter position/socket;
- camera target/rail anchor;
- mechanism socket;
- future authored attachment points.

A candidate minimal fact might be described as stable identity plus pose plus
reference frame, but the current evidence does not justify creating a
`SpatialAnchor` type.

The first task is to determine which of these customers genuinely share
preparation, identity, validation and inspection requirements. Spawn/checkpoint
policy may remain deliberately domain-owned even if their authored pose data can
share tooling.

## Stage boundaries and blast zones

Platform-fighter blast zones are useful evidence that game-specific spatial nouns
can leak downward, but they are **not** the primary proof for a generic region
abstraction.

A stage KO envelope may reasonably remain a ruleset/stage boundary policy derived
from world extent rather than an arbitrary spatial entity. The investigation
should ask what low-level facts it shares with other boundaries -- extent,
containment, exit/crossing, inspection -- without assuming it belongs in the same
storage model as water or gravity.

The architectural requirement is that generic engine layers own generic facts
and the platform-fighter game/ruleset owns the KO meaning.

## Phases

### S0 - inventory real customers before defining types

Build a source-backed census for every current spatial/surface family that may
participate.

For each customer record:

- owning domain/crate;
- authored source(s);
- prepared/runtime representation;
- shape/geometry representation;
- stable identity needs;
- transform/reference-frame needs;
- provenance needs;
- geometric query operations;
- domain semantic query/result;
- mutation/lifetime rules;
- rollback/save participation;
- inspection/debug representation;
- LDtk/tool schema;
- known consumers;
- whether another customer already duplicates any of the above.

At minimum include water, climbable, loading, gravity, camera, static blocks,
one-way/bonk-only contact, blink traversal, pogo/rebound, moving platforms and
stage/world out-of-bounds policy.

**Exit condition:** the census can state, with evidence, which facts are shared by
at least two materially different domains and which similarities are only
superficial.

### S1 - define the smallest shared spatial geometry contract

Using S0, identify the minimum reusable pieces that remove proven duplication.
Candidate pieces include:

- canonical shape/value representation at a chosen boundary;
- durable spatial identity or identity adapter;
- transform/reference-frame representation;
- authoring provenance;
- generic containment/overlap/crossing operations;
- validation helpers;
- inspection descriptors for geometry.

Do not require every domain to migrate. Prove the contract with the smallest pair
of materially different customers that benefit from it.

**Falsifier:** if the proposed abstraction forces one domain to discard a useful
typed query/result API, creates a central behavior switch, or adds more adapter
code than duplicated geometry it removes, narrow or reject it.

### S2 - decompose compositional surface semantics

Inventory every `BlockKind` consumer and classify what semantic question each
consumer actually asks.

Derive a decomposition that lets independent facts vary independently while
preserving the unified collision/movement kernel.

The first implementation slice, when authorized, should be driven by a real
second-combination customer -- for example a conveyor, a moving one-way surface,
or another authored surface whose requirements cannot be expressed cleanly by
one current variant.

The existing `Block::velocity` displacement/surface-drag split remains owned by
`kinematic-world-objects.md`; coordinate with it rather than duplicating that
migration here.

**Falsifiers:**

- adding a new enum that still enumerates cross-product surface species;
- teaching the movement kernel game-specific content names;
- changing contact behavior without preserving existing collision tests;
- creating a generic effect/evaluation VM to avoid ordinary typed domain code;
- introducing an authoring field that claims a combination the runtime cannot
  faithfully simulate.

### S3 - make LDtk/agent authoring prove the abstraction

Extend the existing LDtk/tool schema and discovery mechanisms; do not create a
parallel editor or parallel spatial schema authority.

Acceptance customers should include:

1. discover the installed spatial/surface vocabulary without reading converter
   Rust;
2. inspect a feature's shape, identity, domain owner and composable facets;
3. author a supported combination through semantic operations or stable typed
   fields;
4. reject invalid combinations at preparation time with source provenance;
5. render/describe the authoritative geometry and semantic facets in review
   output;
6. allow provider/game-owned spatial nouns to participate without editing a
   closed engine switch.

The current Mary-O LDtk authoring proof is the preferred source of real friction.
If that work demonstrates that a composition such as a one-way moving rebound
surface or several parallel zone families requires repetitive plumbing, promote
the smallest relevant slice from this plan. If it does not, keep this plan at
rest rather than inventing infrastructure.

### S4 - inspection and debug visualization

Integrate shared spatial descriptors with the existing inspection program.

A machine-readable spatial inspection response should be able to answer, where
applicable:

- what geometry is here;
- which prepared/authored object produced it;
- which domain owns its meaning;
- which generic geometric properties are shared;
- which domain-specific semantic properties are present;
- what reference frame/transform applies;
- why a query or transition did/did not match.

Debug visualization should project this authoritative data. Domain-specific
labels/styles may be contributed by domains, but a generic renderer should not
need a hardcoded list of game mechanics to draw invisible spatial geometry.

### S5 - evaluate anchors and broader runtime unification only after proofs

After S1-S4 have real adopters, revisit the unresolved questions:

- Do named spatial poses deserve a reusable anchor abstraction?
- Do multiple domain regions genuinely benefit from one runtime storage/query
  structure?
- Does a common geometry index have measured performance value?
- Is there a coherent independently consumable Bevy crate boundary?

A "no" answer is acceptable. The plan succeeds if it creates coherent semantics
and authoring even when domains retain different runtime types.

## Acceptance criteria

This program is successful when real customers demonstrate all of the following:

1. **No game noun in generic geometry infrastructure.** A platform fighter can
   implement KO-on-boundary-exit without making "blast zone" a foundational
   engine concept.
2. **No central semantic census.** A provider can add a typed spatial noun/facet
   without adding another variant to a generic region/surface behavior switch.
3. **Independent surface facts compose.** At least one real surface combines
   properties that formerly would have required a new `BlockKind` species.
4. **Typed domain APIs survive.** Water/gravity/transition/etc. may share geometry
   while continuing to expose useful domain-specific semantic answers.
5. **Authoring is simpler.** LDtk/agent tooling can discover and author supported
   combinations without converter archaeology or raw JSON surgery.
6. **Preparation is authoritative.** Invalid combinations/references fail during
   the existing validate/resolve/prepare path with provenance rather than being
   guessed by a Python inspector or visible host.
7. **Inspection is derived, not authoritative.** Structured inspection/debug
   views show the same geometry simulation/preparation uses and identify the
   owning domain.
8. **Existing behavior remains pinned.** Collision, moving-platform, water,
   climb, transition, gravity and presentation behavior retain focused regression
   coverage through migrations.
9. **No speculative broadphase.** Any indexing/acceleration work has a measured
   workload and remains coordinated with the collision/performance programs.
10. **External-game extensibility improves.** At least one secondary/demo/provider
    customer uses the shared vocabulary without requiring an Ambition-content
    dependency or a central switch edit.

## Migration strategy

When an implementation slice is eventually selected:

1. choose one proven duplicated fact or one impossible-to-compose real customer;
2. introduce the narrow shared representation/helper alongside existing domain
   APIs;
3. adapt one or two customers while keeping domain semantic ownership intact;
4. point authoring/preparation and inspection at the same canonical fact;
5. delete the superseded duplicate representation only after consumers converge;
6. preserve compatibility facades only when they have real external consumers;
7. stop after the proof and re-measure before expanding the abstraction.

Prefer deletion of duplicate policy over relocation of it.

Do not perform a flag-day rewrite of all world geometry.

## Trigger and priority policy

Creating this document does not make it the next execution campaign.

The strongest near-term triggers are:

- Mary-O/LDtk authoring encounters a real surface combination that the current
  `BlockKind` taxonomy cannot express without a new species;
- two or more zone families repeat geometry identity/preparation/validation or
  inspection plumbing in a way that the shared contract can demonstrably remove;
- a second surface-motion customer makes the displacement/surface-drag split
  necessary and exposes broader composition pressure;
- a generic debug/inspection feature must duplicate spatial shapes because no
  authoritative geometry descriptor can be consumed;
- an external/provider game needs a spatial noun/facet and currently must edit a
  closed engine switch.

Until one of those triggers is measured, keep this as a focused architectural
owner and let current higher-priority product-pressure work continue.

## Non-goals

This plan does not authorize:

- a universal `SpatialRegion<T>` or `SpatialObject` hierarchy;
- an ECS rewrite of all room/world data;
- a new physics engine;
- a new broadphase/spatial database without measurement;
- a generic effect/rule VM;
- replacing domain-specific semantic query APIs with `RegionKind` switches;
- moving every spatial datum into one crate;
- a GUI editor campaign;
- a new LDtk schema authority beside existing preparation/tooling;
- a full material system without real customers;
- rewriting stage/blast-zone policy solely to make it look like another region;
- implementing `SpatialAnchor` before the anchor census;
- changing current queue/track priority merely because this plan exists.

## Integration note

This document is intentionally self-contained and additive so an integration
agent can triage it safely while other agents are working.

The expected integration choices are one of:

- accept as a focused Engine 1.0 plan and add links/track entries separately;
- fold selected sections into an existing planning owner and delete this file;
- keep it unpromoted until Mary-O/Smash/provider pressure supplies a trigger;
- reject or narrow any proposed abstraction after the S0 census.

No existing planning document needs to be edited for this proposal to be useful
as a reviewable planning artifact.
