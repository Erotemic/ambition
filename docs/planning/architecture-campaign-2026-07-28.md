# Architecture campaign — GPT-5.6 review, 2026-07-28

> # ⛔ SUPERSEDED STATUS — READ THIS FIRST (2026-07-29)
>
> **The "Campaign 1 closeout" below is INVALID.** It reports the character
> authority work as finished. It is not, and a reader who trusts it will build on
> a seated-fighter baseline that was broken when the closeout was written.
>
> Specifically wrong in the text below:
>
> * **"The inventory is 7 of 7"** — it was 4 of 7. The false count is what let
>   H1 ship: a catalog-playable character with no authored action set worked as
>   the worn player and got an EMPTY kit when seated as player two.
> * **X3 — "resolve at preparation, not wear time … would bake the value against
>   ONE catalog, manufacturing the staleness X4 was added to detect."** This
>   landed on 2026-07-29 anyway, and the objection was answered rather than
>   ignored: the fold happens at a `Plugin::finish` BARRIER, after every
>   provider's `build`, so during initial composition there is exactly one
>   catalog to bake against and no ordering to get wrong. A later cast change is
>   an explicit transaction, which is what `CharacterCatalogGeneration` is for.
>   The objection was right about eager resolution; it was wrong that
>   *preparation* had to mean *eager*.
>
> **The canonical current status is:**
>
> * [queue-24h-2026-07-26.md](queue-24h-2026-07-26.md) § H — the open rows
> * [character-preparation-finalization-plan.md](character-preparation-finalization-plan.md)
>   — the design, and what of it has landed
>
> Everything else in this file is kept for its REASONING, which is still good.
> None of its status claims are.

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

> ✔ **SUPERSEDED 2026-07-28, and by X2 rather than by doing it.** No struct was
> built, so "slice one touches the solver" never applied. `CharacterDefinition`
> carries `motion_model` and `movement_tuning`, and both resolve
> definition-first through named one-caller resolvers. ⛔ this said "the
> inventory is 7 of 7"; it was 4 of 7, and the false count is why H1 shipped.
> Left below because the REASONING is still the right reasoning for the next
> campaign that wants to fold a solver-cadence field into an identity.


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

### R-g. ⛔ `ResolvedCharacterIdentity` as a NEW type would be a seventh authority

**Found while starting X2, 2026-07-28. This is the most important revision on the
page and it inverts the step as written.**

The review's target struct lists `character_id`, `provider`, `action_set`,
`moveset`, `hurtboxes`, `presentation`. Compare `PreparedCharacterDefinition`,
which exists and is published today:

```rust
pub struct PreparedCharacterDefinition {
    pub id, pub display_name, pub provider, pub lineage,
    pub sheet, pub portrait, pub body, pub hurtboxes,
    pub vitals, pub moveset, pub action_set,        // ← as of 2026-07-28
    cue_dependencies, vfx_dependencies, checked, unresolved,
}
```

That IS the resolved identity, and `PreparedCharacterRegistry` is the prepared
catalog minus a generation counter. **Adding a second type carrying the same
fields would ADD a production authority, which is the one thing this campaign
exists to stop** — and the review's own rule says so twice ("fewer production
authorities"; "use the repository's actual current types and ownership
boundaries"). Building it because a step said "define a type" would be the
purest possible instance of the failure being migrated away from.

So X2 is **redirected, not skipped**. What is genuinely missing is not a struct:

1. **Precedence is decided at WEAR TIME, not at preparation.** `apply_worn_character_kit`
   resolves prepared-vs-catalog every time a body wears an identity. That is
   C3.6's "runtime arbitration", and moving it into preparation is the real work.
2. **No generation counter.** The registry is mutable and replaceable in place, so
   nothing can say "this body was built from generation N".
3. **`hurtboxes` and presentation are not consumed from it** by body construction.

### R-h. ⚠ The compat kit is ABILITY-derived, so it cannot move to preparation

A constraint the review could not see from outside, and it bounds R-g's item 1.

`resolve_playable_action_set` has three arms. The `Authored` arm is a pure
function of identity. The `HostCode` arm and the unknown-id arm both call
`default_player_action_set(base_abilities)` — and `base_abilities` is the
BODY's persisted `AbilitySet`, not the character's. Two bodies wearing the same
identity with different unlocks legitimately get different kits.

So "preparation decides, runtime consumes" is achievable for the authored path
and **false by construction for the host-code path**. The correct end state is
not one resolver at preparation; it is:

* identity-authored kits resolved ONCE at preparation, consumed verbatim;
* the host-code compat kit derived per body, from abilities, and NAMED as the
  one legitimate runtime derivation rather than left looking like arbitration.

A campaign row that says "delete runtime arbitration" without this distinction
would delete a correct derivation along with the incorrect ones. X12 is amended
accordingly.

### R-f′. Campaign 3 Part B — CHECKED 2026-07-28, and the tree is AHEAD of the plan

The check R-f asked for, done before building anything.

**The review's proposal:** a `BodyLifecycleTransition` carrying body, reason, and
old/new generation, with *"every semantic restart path must emit the transition
from one shared engine helper."*

**What exists:** `BodyRestarted`, and its announcement is **DERIVED rather than
emitted**. `reset_body_clusters` raises `BodyLifetime::restart_pending` on state
it already owns, and one engine system turns that into the trigger once per tick.
That is strictly stronger than the proposal: "every path must emit" relies on
every caller remembering, and this repo has the scar — the event existed, seven
production resets did not send it, and Sanic could respawn holding a ball-dash
charge while the provider observers sat there with nothing invoking them. A
caller that has never heard of the type announces correctly, and so does one
added next year.

**The two fields the proposal adds, assessed against customers rather than
against tidiness:**

* **generation** — already exists as `BodyLifetime::resets`, incremented by the
  same reset. Not carried ON the event, and nothing has asked for it there.
* **reason** — has **no consumer**. Three observers exist (Sanic's ball dash,
  Mary-O's movement, and a versus test standing in for a provider) and not one of
  them would branch on it. Adding it would repeat exactly the mistake A4 and A13
  were both about: a vocabulary with no customer reads as a feature until
  somebody looks.

⚠ If a reason is ever added it must be a **required parameter of
`reset_body_clusters`**, not a field on the event. The compiler can enforce an
argument; it cannot enforce remembering to populate a field, and the derived
design exists precisely because remembering failed.

**Verdict: ⊘ do not build.** The trigger is a provider that genuinely wants to
clear differently for a round restart than for a death — a real distinction, and
one nobody has needed yet.

### R-f. Campaign 3 Part B is closer to done than the review assumes

`BodyLifecycleTransition` describes something this tree partly has: the versus
round restart and the sandbox reset already share a reset path, and the "one
caller forgot an event" failure is the known shape. Worth checking before
building — the review is reasoning from outside and the sandbox-reset work in
queue M1 moved this. Do not schedule it until Campaign 1 is done, but do not
assume it is greenfield either.

---

## ⛔ Campaign 1 closeout — INVALID, see the banner at the top of this file

*(2026-07-28. Kept because the reasoning about each row is still worth reading;
every status claim and every metric in this section is wrong.)*

> *"Stop after Campaign 1 and reassess the repository before beginning rollback
> relocation."*

**Every X row is resolved**: done, deferred with a written argument, or reframed
because the step as written did not match the tree. The table below is the state;
the paragraphs after it are the reassessment.

**Done:** X1 (inventory + baseline), X4 (cast generation), X5/X6/X7 (action-set
precedence, moveset from the winner, ranged-payload coherence), X8 (player
construction, proven rather than assumed), X11 (equipment overlay under an
identity swap), X15 (docs), plus R-a's successor slice (motion model), plus G4
(dependency-edge contracts).

**Reframed — the step as written did not match the tree:**

| row | as written | what it actually was |
|---|---|---|
| **X2** | define `ResolvedCharacterIdentity` | ⛔ it exists as `PreparedCharacterDefinition`; building it would ADD the seventh authority |
| **X12** | delete runtime arbitration | arbitration is many places deciding; four NAMED resolvers, one caller each, is not that — the content is a guard against a fifth |
| **X13** | confine the legacy catalog to preparation | conflates the KIT authority with names and art, which are legitimately the catalog's |
| **X3** | resolve at preparation, not wear time | would bake the value against ONE catalog, manufacturing the staleness X4 was added to detect |

**Metric:** 5 of 7 precedence-resolver sites resolved; both remainders were
deliberately deferred, not missed. Raw reference counts went UP, which is what a
migration does in the middle — see the inventory's note.

### What the campaign got right, and it is the important part

The one-line thesis held everywhere it was applied: *introduce one authority,
migrate all production consumers, delete the displaced authority, guard the
absence.* **The step that keeps getting skipped is "ALL production consumers."**
It was skipped twice today by me, in the same shape both times:

* the action set was wired into the worn path and not the seated one, and the two
  arena duelists — its first production callers — HID it, because their authored
  set is empty and the placeholder happened to equal it;
* the motion model was then wired into both paths in one commit, specifically
  because the diagnosis of the first was two commits old.

**A first production caller whose authored value equals the default proves the
wiring runs, not that it arrives.** That is the transferable lesson and it
belongs in front of Campaign 2, where the equivalent will be a domain whose
rollback registrations happen to be identical before and after the move.

### Reassessment: do NOT start Campaign 2 yet

Two reasons, one of them the review's own.

1. **The review's:** the rollback registry is the most load-bearing thing in this
   repo. Its own R3 requires the schema fingerprint to be unchanged through each
   domain's migration — a bisect-critical property — and it should not be moved
   while anything else is in flight.
2. **Mine, and it is a precondition:** Campaign 2's completion criterion is
   "rollback schema remains behaviourally unchanged through migration". There is
   no fingerprint comparison harness that a reviewer can run today; the
   fingerprint exists (`lifecycle::tests` moves it on a schema change) but
   nothing captures a BEFORE and asserts an AFTER across a refactor. Building
   that is the first slice of Campaign 2, before any registration moves — the
   same "measure first" that made X1, X12 and D17 come out differently than
   planned.

### What to do instead, in priority order

* **Campaign 3 Part B first, and CHECK before building** (R-f). It is
  independent of rollback relocation, and the tree already has a shared
  body-restart path — `reset_body_clusters` raises a pending flag and one engine
  system announces it. The review assumes greenfield; it is not.
* **Campaign 5's second consumer.** The conformance harness needs a NONCOMBAT
  provider and a first-party one beside Outlander, or it encodes one fixture's
  assumptions. That is a real gap and it does not touch rollback.
  ✔ **a first slice landed 2026-07-28, and it did not need a second crate.** The
  gap that mattered most was narrower than "a second fixture": the
  character-DEFINITION seam — everything C3 spent the day making authoritative —
  had **every one of its callers inside the workspace**. Two arena duelists. That
  makes it a claim about this repo rather than about an engine.
  → Outlander registers a `CharacterDefinition` now, authoring an EMPTY
  `ActionSet`, which is the harder half of the claim rather than the lazy one.
  Its catalog row declares `playable_kit: HostCode`, so a resolver that collapsed
  "authored as empty" into "authored nothing" falls through to the row and
  rebuilds the HOST protagonist's kit onto a third party's character.
  ⚠ **RED-PROBED, and the probe is the argument.** With the authored set removed,
  the wanderer is handed Ambition's own swipe, bolt AND bubble shield. That is
  the Sanic principle — an intentionally weaponless character must not be given a
  punch — demonstrated by somebody outside the workspace, which is the only place
  the claim means anything.
* ~~**The measurements the facade work is blocked on.**~~ ✔ **TAKEN 2026-07-28,
  and they say LEAVE IT DEFERRED — for a reason rather than by inertia.**

  The campaign's likely first restriction was *"internal content crates may not
  depend on the full `ambition` facade."* Measured:

  | measurement | value |
  |---|---:|
  | internal NON-app crates depending on the facade | **1** (`ambition_content`) |
  | facade modules that crate actually uses | 8 |
  | of its 43 facade references, `ambition::platformer` | 36 |
  | direct `ambition_*` dependencies it ALREADY declares | 36 |
  | `ambition_*` crates in the external consumer's graph | 41 of 528 total packages |

  **The restriction has exactly one subject**, and that subject already declares
  direct dependencies on nearly everything it reaches through the facade —
  `ambition::platformer` is `ambition_platformer_primitives`, which is already a
  direct dep. So the rule is cheap to satisfy.

  ⛔ **And it would change nothing measurable.** `ambition_content` is co-built
  with the facade in every composition that contains it: the app depends on both.
  Removing the edge reduces no build, because nothing wants `ambition_content`
  WITHOUT the facade. The benefit the restriction is supposed to buy has no
  claimant.

  ⚠ **Timings NOT taken** — the clean-build and incremental-rebuild numbers the
  campaign also asks for. Disk hit 100% during this run and a clean build of the
  528-package consumer graph was not a good use of what was left. They are the
  half that could still change the answer, and saying they were skipped is the
  difference between a deferral and a claim.

## Concrete task list

Campaign 1 only. Rows are executable; each maps to the review's numbered step.
**Nothing here starts until the immediate-bug rows are committed** (queue R1–R4).

| id | row | maps to | done when |
|---|---|---|---|
| ~~**X1**~~ | ✔ **DONE 2026-07-28** — [character-authority-inventory-2026-07-28.md](character-authority-inventory-2026-07-28.md). The metric that matters is not the 349 `CharacterCatalog` refs: it is **7 precedence-resolver sites, 3 resolved**. Campaign 1 is complete when that table has one row. | C3.1 | ✔ committed with counts + method |
| ~~**X2**~~ | ⊘ **REDIRECTED — do not build the struct.** `PreparedCharacterDefinition` already carries every field the review's `ResolvedCharacterIdentity` lists; a second type would ADD a production authority, which is the one thing this campaign exists to stop (R-g). The work is X3/X4/X12, not a definition. | C3.2 | ⊘ with a stated reason |
| **X3** | Move precedence resolution from WEAR TIME into preparation, for the identity-authored path only — `apply_worn_character_kit` re-resolves prepared-vs-catalog on every wear, and that is C3.6's runtime arbitration. ⚠ bounded by R-h: the host-code kit is ability-derived and legitimately stays per-body. | C3.2/C3.6 | the authored path resolves once; test that two bodies wearing one identity get byte-identical kits |
| ~~**X4**~~ | ✔ **DONE 2026-07-28.** `CharacterCatalogGeneration` on `PreparedCharacterRegistry`. A counter, not a hash: two registries with identical contents assembled at different times are legitimately different casts, and a consumer caching against a hash would keep a stale value across a replacement that happened to reproduce the same cast. Advances on REPLACEMENT too, which is the case an insertion counter misses. | C3.3 | ✔ `the_cast_generation_advances_on_every_published_change` |
| ~~**X5**~~ | ✔ **DONE 2026-07-28** (`34706cf39`). Both duelists author their `ActionSet` on their definitions. ⚠ also fixed: `wears_host_code_kit` was still asking the displaced catalog row. | step 4 | ✔ 3 tests incl. the Sanic authored-empty case |
| ~~**X6**~~ | ✔ **DONE 2026-07-28**. Resolving the set BEFORE deriving is what makes this true; the empty-authored-set-vs-catalog-melee case is the test that tells the two implementations apart. | step 5 | ✔ `a_prepared_action_set_with_no_prepared_moveset_derives_from_the_winning_set` |
| ~~**X7**~~ | ✔ **DONE 2026-07-28**. New `RangedPayload` binding namespace; reported only when the definition authored a set. | C3.2 | ✔ 3 tests incl. both negatives |
| **X8** | Migrate primary-player construction to the resolved identity. | step 6 | player body's kit comes from `ResolvedCharacterIdentity` |
| **X9** | Migrate match/secondary-fighter seating. | step 7 | versus seats read the resolved identity |
| **X10** | Migrate NPC/enemy construction where the same authority applies. | step 8 | named production paths listed and moved |
| **X11** | Equipment becomes an overlay: `live = resolved baseline + grants`. Identity change atomically replaces the baseline before reapplying equipment. | C3.5 | test: an identity swap mid-equipment does not resurrect the old kit |
| **X12** | Delete runtime arbitration — no production system asks "prepared or catalog?" after body construction. ⚠ **amended by R-h:** the ability-derived host-code kit is a legitimate runtime derivation and must be NAMED as one, not deleted alongside the arbitration it resembles. | C3.6 | the arbitration branches are gone; the one derivation is documented as such |
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
