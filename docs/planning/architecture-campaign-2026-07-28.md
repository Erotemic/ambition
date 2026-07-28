# Architecture campaign — GPT-5.6 review, 2026-07-28

**Provenance.** Jon handed this over mid-run on 2026-07-28 with the instruction:
*save it so it survives compaction; take it on next, after the immediate bugs;
revise the plan based on your own understanding of the codebase.* The review text
is preserved verbatim in [§ The review as received](#the-review-as-received). My
revisions against the actual tree are in
[§ Revisions](#revisions-against-the-actual-tree) and the executable rows are in
[§ Concrete task list](#concrete-task-list).

**Relationship to the 24h queue.** This does not replace
[queue-24h-2026-07-26.md](queue-24h-2026-07-26.md) — that stays the open-items
ledger the guard reads. This is the *campaign* document: the ledger says what is
open, this says in what order and under what rules the character-identity work
happens. Rows land in the ledger as they are picked up.

**The one-line thesis, which is worth more than the rest of the document:**

> Introduce one authority, migrate all production consumers, delete the
> displaced authority, and guard the absence.

Every one of the five parts is required. A new path living beside an old path is
the failure mode this campaign exists to end, and this repo has shipped it
repeatedly — the character catalog and the prepared registry are both alive right
now, which is precisely why C3 has been "nearly done" for three days.

---

## Revisions against the actual tree

I agree with the campaign as written. Six adjustments, each from something the
review could not see from outside.

### R-a. `ResolvedCharacterIdentity` should not carry `motion_model` in slice one

The review's target struct lists `motion_model: MotionModelSpec`. In this tree
movement tuning is `CharacterCatalog`'s and is read by the movement solver on a
different cadence than identity — folding it in on the first slice means the
first commit touches the solver, and the review's own rule says *first relocate
authority, improve semantics later*. Land the struct with `action_set`,
`moveset`, `hurtboxes`, `provider`/`presentation`, and leave motion tuning as a
catalog read with a **named row** to move it in slice two. Otherwise slice one
cannot be reviewed in one sitting, which is how the "one coherent commit per
step" rule gets abandoned.

### R-b. The precedence table's "safe peaceful default" already has a name here

Do not invent a fallback. Sanic is the standing example of an *intentionally
empty* combat kit (queue C3: *"the momentum ride + ball dash ARE the kit; no
combat moveset"*), and the review's own C3.2 requires distinguishing
"not authored" from "authored as empty". That distinction is the single most
load-bearing detail in the whole campaign, because getting it wrong hands Sanic a
punch. It must be `Option<ActionSet>` on the definition, never a defaulted value,
and there must be a test named after Sanic.

### R-c. "Reject irreconcilable duplicate authorities" needs the `insert_ron` lesson

C3.2 says the resolver must reject duplicates. It should reject them the way
`AuthoredSheets::insert_ron` now does (fixed the same day, from the same
reviewer): **refuse a second, different claim; permit a byte-identical
re-registration as idempotent.** Two providers in one process is normal; two
providers *disagreeing* is a decision only a human can make. Last-writer-wins
resolved by plugin-build order is the bug, not the duplicate itself.

### R-d. Campaign 1's guards belong in `scripts/check_absence_contracts.py`

The review's "W1 architecture predicates" section describes exactly the file
landed on 2026-07-28 — stable id, narrow production paths, exact forbidden
symbol, excluded preparation/diagnostic paths, stated architectural reason. C3.7
should ADD ROWS to that table rather than build a second mechanism. The one
addition the review asks for that the file does not yet do is **Cargo metadata
for dependency edges**; that is a real gap and is row G4 below.

⚠ The file's hard-won rule, which C3.7's rows must obey: **strip comments before
matching.** Three separate absence checks in this repo went red on prose because
somebody documented the removal. Documenting a removal must not break the guard
that verified it.

### R-e. The metrics section needs a baseline captured BEFORE slice one

"At least one relevant complexity metric must decrease" is unfalsifiable without
a before-number, and after the migration nobody can reconstruct it. C3.1's
inventory *is* the baseline — it must be committed with counts, not just a list.

### R-f. Campaign 3 Part B is closer to done than the review assumes

`BodyLifecycleTransition` describes something this tree partly has: the versus
round restart and the sandbox reset already share a reset path, and the "one
caller forgot an event" failure is the known shape. Worth checking before
building — the review is reasoning from outside and the sandbox-reset work in
queue M1 moved this. Do not schedule it until Campaign 1 is done, but do not
assume it is greenfield either.

---

## Concrete task list

Campaign 1 only. Rows are executable; each maps to the review's numbered step.
**Nothing here starts until the immediate-bug rows are committed** (queue R1–R4).

| id | row | maps to | done when |
|---|---|---|---|
| **X1** | Inventory every production read of `CharacterCatalog`, `PreparedCharacterRegistry`, action-set/moveset/provider-owner maps. Classify each: preparation / body construction / runtime / diagnostics / tests / migration. **Commit the counts** as the metric baseline (R-e). | C3.1 | a committed table with per-class counts |
| **X2** | Define `ResolvedCharacterIdentity` — `action_set`, `moveset`, `hurtboxes`, provider/presentation. **No `motion_model` yet** (R-a). | C3.2 | type exists, unused, compiles |
| **X3** | Write the resolver at the preparation boundary. `Option<ActionSet>` so "authored empty" ≠ "not authored" (R-b). Refuse conflicting duplicate authorities, permit identical ones (R-c). All-or-nothing: one character failing publishes nothing. | C3.2 | test: a Sanic-shaped definition authoring an EMPTY action set does not receive a punch |
| **X4** | Publish `PreparedCharacterCatalog` — immutable per generation, atomically replaceable. Character-specific; **not** a `PreparedExperience`. | C3.3 | resource exists with a generation counter |
| **X5** | Action-set precedence: definition outranks catalog. Move the two arena duelists' empty-but-intentional action set onto their definitions — the first production callers. | step 4 | test: a definition action set beats a conflicting catalog row |
| **X6** | Moveset derives from the **winning** action set when no explicit moveset is authored — never from the displaced catalog value. | step 5 | test: prepared action set + no prepared moveset ⇒ moveset matches the prepared set |
| **X7** | Preparation flags obvious incompatibilities: an authored ranged move whose resolved action set supplies no ranged payload. | C3.2 | test: the mismatch is reported with character id + field |
| **X8** | Migrate primary-player construction to the resolved identity. | step 6 | player body's kit comes from `ResolvedCharacterIdentity` |
| **X9** | Migrate match/secondary-fighter seating. | step 7 | versus seats read the resolved identity |
| **X10** | Migrate NPC/enemy construction where the same authority applies. | step 8 | named production paths listed and moved |
| **X11** | Equipment becomes an overlay: `live = resolved baseline + grants`. Identity change atomically replaces the baseline before reapplying equipment. | C3.5 | test: an identity swap mid-equipment does not resurrect the old kit |
| **X12** | Delete runtime arbitration — no production system asks "prepared or catalog?" after body construction. | C3.6 | the branches are gone, not bypassed |
| **X13** | Confine the legacy catalog to preparation, or delete it. | criteria | no runtime read remains |
| **X14** | Add C3.7 guard rows to `scripts/check_absence_contracts.py` (R-d), comment-stripped, prod-only, each RED-probed. | C3.7 / W1 | each new contract has a red-probe test |
| **X15** | Update C3 docs and the design doc — **only after** the old production path is gone. | step 13 | doc names ONE character identity authority |
| **G4** | New guard capability the review asks for and the file lacks: **dependency-edge contracts from `cargo metadata`** (crate A must not depend on crate B). Grep cannot express this and it is the strongest available enforcement. | W1 | one real edge contract passing + red-probed |

**Stop after Campaign 1 and reassess** before starting Campaign 2 (rollback
adapter relocation). That instruction is the review's and I agree with it: the
rollback registry is the most load-bearing thing in the repo and moving it while
character identity is half-migrated would make a bisect impossible.

### Deferred, recorded so they are not re-derived

- **Campaign 2** rollback domain adapters — after Campaign 1.
- **Campaign 3A** `OwnedByRound` scope for transient combat entities.
- **Campaign 3B** `BodyLifecycleTransition` — check what exists first (R-f).
- **Campaign 4** prepare/commit reset transactions, sandbox reset first.
- **Campaign 5** conformance capability groups; needs a **noncombat** provider
  and a **first-party** provider beside Outlander, or the harness just encodes
  one fixture's assumptions.
- **Facade restructuring** — blocked on measurements, deliberately. Likely first
  restriction: *internal content crates may not depend on the full `ambition`
  facade.*
- **`ambition_actors` decomposition** — not during the authority campaigns. A
  rename to `ambition_platformer_gameplay` may beat a speculative split.
- **Match extraction** — only the deterministic presentation-free state machine,
  and only once seats/rounds/score/countdown/KO stop changing.

---

## The review as received

Preserved verbatim below. Where it and the revisions above disagree, the
revisions are the plan of record and say why.

### Objective

Reduce architectural duplication without destabilizing working engine behavior.

The goal is not to introduce a new universal framework. The goal is to remove
competing authorities, central type lists, and independently implemented
lifecycle transitions one bounded slice at a time.

Every completed slice must leave the repository with:

* fewer production authorities;
* fewer compatibility paths;
* fewer central lists;
* an explicit guard preventing the removed architecture from returning.

Do not work on multiple campaigns simultaneously.

### Operating rules

#### Keep each migration bounded

Each slice must contain all five parts:

1. Introduce the replacement authority or mechanism.
2. Move named production consumers to it.
3. Delete the displaced production path.
4. Add an architectural guard against restoring the old path.
5. Update documentation only after the old path is gone.

A new path working beside an old path is not completion.

#### Preserve behavior before improving it

During architectural relocation:

* preserve existing public behavior;
* preserve rollback schema fingerprints where applicable;
* preserve save compatibility;
* preserve provider composition;
* avoid unrelated tuning or gameplay changes.

First relocate authority. Improve semantics in a later commit.

#### Do not introduce universal abstractions prematurely

Do not introduce:

* a universal lifecycle-scope hierarchy;
* a universal session-transaction trait;
* one monolithic `PreparedExperience`;
* a new crate for every subsystem;
* generalized service traits around registries.

Extract a common abstraction only after at least two concrete implementations
demonstrate the same stable structure.

#### One coherent commit per step

Prefer commits with scopes such as:

* `ARCH resolve prepared character identity`
* `MIGRATE player construction to resolved identity`
* `DELETE runtime catalog arbitration`
* `GUARD forbid runtime character authoring reads`

Do not combine authority migration, crate splitting, lifecycle redesign, and
gameplay work in one commit.

### Campaign 1 — Make resolved character identity authoritative

This is the immediate campaign.

#### Problem

Character behavior is still assembled from overlapping sources:

* `CharacterDefinition`;
* prepared character registries;
* legacy character catalogs;
* independently authored action sets;
* independently authored movesets;
* runtime identity projection;
* equipment reconciliation.

This permits the body's action capabilities, moveset, motion model, hurtboxes and
presentation provider to come from different authorities.

#### Target

Introduce one resolved character identity produced during preparation and
consumed by production body construction.

A suitable shape is:

```rust
pub struct ResolvedCharacterIdentity {
    pub character_id: CharacterId,
    pub provider: PresentationSourceId,
    pub action_set: ActionSet,
    pub moveset: ActorMoveset,
    pub motion_model: MotionModelSpec,
    pub hurtboxes: AuthoredHurtboxes,
    pub presentation: ResolvedCharacterPresentation,
}
```

Use the repository's actual current types and ownership boundaries. Do not
duplicate expensive immutable values unnecessarily; use handles or shared values
where appropriate.

#### Required precedence

Resolve precedence once during preparation:

```text
action set:
    CharacterDefinition explicit value
    else legacy catalog fallback
    else safe peaceful default

moveset:
    CharacterDefinition explicit value
    else derive from the resolved action set
    else empty moveset

motion model:
    CharacterDefinition explicit value
    else catalog fallback
    else engine default

hurtboxes:
    CharacterDefinition explicit value
    else catalog or pose fallback
    else documented safe fallback

provider and presentation:
    prepared provider declaration
    else catalog migration fallback
```

An explicit action set on `CharacterDefinition` outranks the catalog.

If the prepared action set wins and no prepared moveset is supplied, derive the
moveset from the winning prepared action set. Do not derive it from the displaced
catalog value.

#### Implementation steps

**C3.1 — Inventory current authorities.** Before changing behavior, identify
every production read of `CharacterCatalog`, `PreparedCharacterRegistry`,
character action-set registries, character moveset registries, provider-owner
maps, and character sheet or presentation-owner maps. Classify each occurrence as
preparation, production body construction, runtime behavior, diagnostics, tests,
or migration compatibility. Record the production callers in the queue or
implementation note.

**C3.2 — Add the resolved type and resolver.** Create the resolver at the
preparation boundary. It must produce one result per character ID; reject
irreconcilable duplicate authorities; report the character ID, provider, field,
and source fragment; preserve explicit empty values when they are intentional;
and distinguish "not authored" from "authored as empty". Do not publish partial
results if one character fails preparation.

**C3.3 — Publish one immutable prepared character catalog.**

```rust
pub struct PreparedCharacterCatalog {
    generation: CharacterCatalogGeneration,
    characters: BTreeMap<CharacterId, ResolvedCharacterIdentity>,
}
```

This is not yet a global `PreparedExperience`. Keep it character-specific. The
published catalog should be immutable for one generation and replaceable
atomically by a later generation.

**C3.4 — Move body construction to the resolved identity.** Migrate all
production fighter construction paths: primary player construction; secondary
player or match seating; NPC/enemy actor construction where applicable;
checkpoint or room reconstruction; transformation or character replacement paths.
The body receives its baseline from one resolved identity — `IdentityKit`, base
`ActionSet`, base `ActorMoveset`, motion model, authored hurtboxes,
provider/presentation source, relevant route markers.

**C3.5 — Apply equipment as an overlay.** Equipment must operate after the
resolved baseline is installed:

```text
live identity = resolved character baseline + equipment grants
```

Equipment reconciliation must never reconstruct the baseline from the old
catalog. Identity changes must atomically replace action set, moveset, route
markers, hurtboxes, motion model and provider presentation state before
reapplying equipment.

**C3.6 — Remove runtime arbitration.** Delete production code that independently
chooses between prepared and catalog values after body construction. Production
simulation should not repeatedly ask whether the prepared value exists, whether
the catalog value should stand, which provider owns the character, or which
moveset should overwrite another moveset. Preparation decides. Runtime consumes.
Legacy catalogs may remain as preparation inputs until migration finishes.

**C3.7 — Add architectural guards.** Add explicit negative predicates that fail
if production runtime code reads the displaced authoring authorities. Examples:
runtime body systems do not read `CharacterCatalogRegistry`; simulation systems
do not read `PreparedCharacterRegistry`; provider ownership is not independently
reconstructed outside preparation; no downstream system overwrites the baseline
`ActorMoveset` from legacy catalog state. Exclude preparation modules, migration
adapters, diagnostics and authoring tools. Prefer dependency or compiler
enforcement where possible; use exact grep predicates as an interim mechanism.

#### Completion criteria

Campaign 1 is complete only when every production character body is constructed
from `ResolvedCharacterIdentity`; explicit definition action sets outrank catalog
action sets; equipment overlays the resolved baseline; runtime systems no longer
arbitrate between prepared and catalog authorities; the legacy catalog is either
preparation-only or deleted; guards prevent new production reads of the displaced
authorities; and documentation names one character identity authority.

### Campaign 2 — Move rollback registration into domain-owned adapters

Begin only after Campaign 1 is complete.

**Problem.** Central rollback registration knows too many gameplay-domain types.
This makes the runtime a mandatory edit point for new domains and separates
rollback semantics from domain ownership.

**Target.** Each domain owns its rollback schema adapter. The runtime aggregates
adapters and hosts rollback execution. Do not change the rollback state model
during the initial migration.

**R1 — Separate schema vocabulary from runtime hosting.** Identify the smallest
reusable rollback-schema surface: descriptor registration; clone/checksum
registration; entity remapping; probe registration; schema fingerprints;
stable-value projection helpers. Initially this may remain a module rather than a
new crate. Extract a crate only if dependency direction requires it. The schema
layer must not depend on GGRS session hosting.

**R2 — Add domain registration plugins or functions.**

```rust
pub struct ActorRollbackSchemaPlugin;
pub struct CombatRollbackSchemaPlugin;
pub struct ProjectileRollbackSchemaPlugin;
pub struct WorldRollbackSchemaPlugin;
```

The adapter may live in the domain crate or in a higher-level companion module if
adding the schema dependency to the primitive crate would invert dependencies.

**R3 — Migrate one domain without changing its schema.** For each domain: record
the existing descriptor list and fingerprint; move registrations to the domain
adapter; preserve registration order and projections; verify the resulting schema
fingerprint is unchanged; remove the corresponding central registrations. Do not
strengthen probes or alter snapshot behavior in the same commit as the
relocation.

**R4 — Delete central domain enumeration.** The central runtime registration
function should contain only runtime-owned resources and aggregation. It must not
name actor, combat, item, portal, boss or provider-specific components.

**R5 — Add dependency guards.** Prevent the central runtime from reacquiring
domain-specific registration knowledge.

**Completion criteria.** Rollback schema remains behaviorally unchanged through
migration; every gameplay domain registers its own authoritative state; the
central runtime aggregates rather than enumerates; external and internal
consumers use the same registration vocabulary.

### Campaign 3 — Introduce narrow lifecycle ownership

Do not begin with a universal scope hierarchy.

#### Part A — Round-scoped transient entities

**Problem.** Round resets currently need to know every projectile,
strike-volume, summon, and temporary combat-effect family.

**Target.**

```rust
pub struct RoundScopeId(/* ... */);

#[derive(Component)]
pub struct OwnedByRound(pub RoundScopeId);
```

Attach it to entities that must not survive a round boundary: projectiles; strike
volumes; temporary combat summons; round-local ability effects; other explicitly
classified transient combat entities. Closing the round removes entities owned by
that round. Do not attach persistent fighter bodies to the round scope.

**Completion criteria.** Starting a new round does not require a central list of
transient entity component families.

#### Part B — Body lifecycle transitions

**Problem.** Provider-owned state can survive death, respawn, versus restart,
sandbox reset, or room reconstruction unless each path remembers to send the same
event.

**Target.**

```rust
pub struct BodyLifecycleTransition {
    pub body: Entity,
    pub reason: BodyRestartReason,
    pub old_generation: BodyLifeGeneration,
    pub new_generation: BodyLifeGeneration,
}
```

Reasons include death respawn; versus round restart; sandbox reset; checkpoint
restore; room reconstruction; and transformation, only if transformation
semantically begins a new body life. Every semantic restart path must emit the
transition from one shared engine helper. Providers clear or reconstruct
provider-owned state from this event.

**Completion criteria.** All body restart paths pass through one helper;
provider-owned round/life state cannot survive because one caller forgot an
event; the event's name and documentation match its actual reach. Generalize
round and body lifetimes into a broader scope system only if a later room or
encounter migration demonstrates identical mechanics.

### Campaign 4 — Establish prepare/commit reset transactions

Do not start by defining a universal transaction trait.

**First operation: sandbox reset.**

```rust
pub struct PreparedSandboxReset {
    // validated replacement room
    // scopes to close
    // player transfer or reconstruction
    // resource replacements
    // expected identities
}
```

Preparation must use immutable world access; perform all validation; resolve
replacement content; allocate no live entities; enqueue no commands; and mutate
no resources. Commit consumes the prepared value and performs teardown and
replacement. All cleanup currently ordered before reset preflight must move into
commit.

**Second operation: room transition.** After sandbox reset is stable, apply the
same convention to room transitions (`PreparedRoomTransition`). Only after both
operations exist should the agent consider extracting a shared trait. A shared
trait is justified only if both implementations naturally expose the same useful
interface.

**Guards.** Add a guard or structural restriction ensuring teardown systems
cannot run without a prepared transaction.

**Completion criteria.** A failed preparation leaves the current session
byte-for-byte semantically unchanged except for diagnostics.

### Campaign 5 — External-consumer conformance

Expand the successful fixture, but do not canonize one game's optional features
as universal requirements.

**Core provider contract:** provider registration; preparation and activation;
source-qualified assets; stable identity; headless session construction;
diagnostics with source and field attribution.

**Character capability:** resolved character identity; consumer-owned sheet
metadata; body construction; presentation binding; collision and hurtbox
derivation.

**Combat capability:** attacks; damage; projectiles; rollback state;
provider-owned lifecycle state.

**Visible capability:** consumer asset source; rendered texture resolution; view
construction; declared HUD integration.

A provider should declare which capability groups it implements. Unsupported
optional capabilities are not failures.

**Consumers.** Run the conformance harness against the existing external combat
fixture; a minimal noncombat provider; and one first-party game provider. This
prevents the harness from merely encoding the assumptions of one fixture.

### Campaigns to defer

**Facade restructuring.** Do not change the public facade until measurements
exist. First measure clean external-consumer build time; incremental build after a
consumer-only source change; crates compiled by a minimal headless consumer;
unconditional dependencies that provide no used surface; and use of the full
umbrella by internal content crates. The likely first restriction is: *internal
content crates may not depend on the full `ambition` facade.* Choose crate
splitting or optional features only after the measurements identify a real build
boundary.

**`ambition_actors` decomposition.** Do not split the crate during the authority
and lifecycle campaigns. After those migrations: measure its remaining
responsibilities and dependencies; identify whether a coherent actor kernel
exists; prove that kernel can compile without room, menu, persistence, shrine and
host integration; migrate a real consumer; then decide between a rename and a
split. A rename to `ambition_platformer_gameplay` is preferable to a speculative
split if it remains an intentional integration layer.

**Match extraction.** Do not move the entire versus implementation into a new
crate yet. First extract only a deterministic, presentation-free state machine
when these semantics are stable: seats and teams; round counter; score;
countdown; KO; win resolution; rollback state. Leave routing, HUD styling, clock
effects and arena composition outside until the state machine no longer changes
frequently.

### W1 architecture predicates

Add explicit checks for known absence contracts now. Each predicate must include
a stable ID; narrow production paths; an exact forbidden symbol or dependency;
excluded preparation, diagnostic and generated paths; and an explanation of what
architecture the absence protects.

Use Cargo metadata for dependency edges; compiler/module privacy for forbidden
API access; exact grep for legacy symbols reaching zero callers; and runtime
audits for uniqueness of active authorities.

Do not add more natural-language parsing to the roadmap evidence script.

### Required metrics

Track these before and after each campaign.

**Character authority:** production reads of raw character authoring registries;
number of character precedence resolvers; number of separately stored provider,
action-set and moveset mappings.

**Rollback:** domain types named by the central runtime; state-bearing
registrations lacking value-sensitive probes; schema fingerprint before and after
each migration.

**Lifecycle:** bespoke round teardown queries; body restart paths that
independently reset state; provider-specific edits required to add a new restart
path.

**Build structure:** clean external fixture build duration; incremental
consumer-only rebuild duration; number of crates compiled by the headless
fixture; unconditional facade dependency count.

A campaign is not successful merely because code moved. At least one relevant
complexity metric must decrease.

### Immediate execution order

1. Inventory character identity authorities and production consumers.
2. Define `ResolvedCharacterIdentity`.
3. Define the immutable prepared character catalog generation.
4. Implement explicit action-set precedence.
5. Derive movesets from the winning action set when no explicit moveset exists.
6. Move primary-player construction to the resolved identity.
7. Move match and secondary-fighter construction.
8. Move NPC/enemy construction where the same character authority applies.
9. Make equipment overlay the resolved baseline.
10. Remove runtime moveset and action-set arbitration.
11. Remove or preparation-confine the displaced catalog reads.
12. Add W1 predicates preventing their return.
13. Update C3 documentation and mark it complete only after the old production
    path is gone.

Stop after Campaign 1 and reassess the repository before beginning rollback
relocation.

### Definition of success

This architecture work succeeds when adding a new third-party character requires:

* one provider-owned definition;
* one preparation result;
* no central runtime edit;
* no duplicate catalog row;
* no separate provider-owner registration;
* no downstream moveset overwrite;
* no bespoke rollback central-list edit;
* no bespoke reset-path edit.

The guiding rule is:

> Introduce one authority, migrate all production consumers, delete the displaced
> authority, and guard the absence.

Do not leave both generations alive indefinitely.
