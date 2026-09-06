# ADR 0030: Spawn provenance is data, and construction is planned before mutation

## Status

**Accepted; construction-family migration complete** (2026-07-22; migration
completed 2026-08-18; revised twice the same day after two rounds of external
review. The second round found four of
the first round's five repairs incomplete and one — permitting a subset to cut a
relation's target — actively wrong. The Decision section below describes the
mechanism as it now stands, not as either round intended it.) Completes
Milestone B of
[`../planning/engine/immutable-content-and-transactional-construction.md`](../planning/engine/immutable-content-and-transactional-construction.md)
and the provenance/planning half of that campaign's Phase-0 ADR obligation.
ADR 0026 settled registration lifecycle and content epochs; this settles entity
provenance and construction planning.

## Context

Two facts about a simulated entity had no home.

**Where it came from.** `SimId` is identity — *which* entity this is. It is a
string built from the game's own facts, deliberately legible so that a desync
report reads as a sentence: `placement:duel_pca/0` is the duellist's zeroth
child. That legibility quietly became load-bearing. `heal_projectile_owners`
recovered a projectile's firer with `id.as_str().rsplit_once('/')` — the only
place in the tree that parsed a `SimId`, and directly contrary to
`SimId::as_str`'s own doc comment, which claimed the string was "never parsed"
while it was being parsed one crate away.

That coupling had three costs. The id grammar could not change without silently
changing reconstruction. An entity whose spelling *lied* about its family was
unreconstructable in principle — and every summoned minion was such an entity,
because it carried a `FeatureId` and so `ensure_sim_id` filed it under the
AUTHORED `placement:` namespace, the one namespace a runtime summon categorically
is not in. And the registered derived-state justification for `ProjectileOwner`
was simply wrong: it named `ProjectileOwnerId`, which is empty for every player
projectile and therefore could not have carried the owner for the largest
projectile pool in the game.

**Whether it can be built at all.** Construction was decided while it happened.
`spawn_ground_item` resolved an authored pickup's held-item registry id at spawn
time and `return`ed on a miss, so an authored item naming an unregistered or
feature-gated entry produced no entity and no diagnostic. `wire_staged_grudges`
resolved a staged fighter's `grudge_against` against one message batch and
skipped anything it could not find, so a typo produced two duellists who ignored
each other. Both failures were invisible *because* the decision and the mutation
were the same step: by the time anything could have complained, the room was
already being replaced.

## Decision

**Provenance is a component.** `SpawnOrigin` — `Authored { source, instance }`,
`ProviderStaged { provider, room, instance }`, `Dynamic { parent, sequence }` —
is snapshot-registered state that travels with the entity, because a blob-rebuilt
entity is precisely the case where nothing around it can still say where it came
from. `SimId` spelling stays a human convenience and nothing may recover a fact
from it.

A dynamic entity's `parent` is **not optional and is stored exactly once**.
"Dynamic, parent unknown" is not a state worth being able to spell — it is
unreconstructable by definition — so a spawn site that cannot name its spawner's
identity refuses to spawn rather than minting provenance that says nothing. And
because a construction request could carry the same parent a second time beside
the origin, it does not: preparation validates `SpawnOrigin::parent` directly, so
the fact that is checked is the fact the world receives. Two fields that mean one
thing eventually disagree, with nothing to say which one reconstruction should
believe.

**Construction is planned as a pure value, then committed.**
`ambition_platformer2d_shared_tangle::construction` owns the content-free vocabulary:
`RecipeId`, `ConstructionRequest`, `ConstructionPlan`, `PlannedEntity`,
`PlannedRelation`, a recipe registry on ADR 0026's registration lifecycle, and a
byte-stable dump. Planning validates identity collisions (against the plan and
the live world), unknown recipes, unresolved parents, and unresolved relations —
all of it borrowing the world rather than mutating it, so a rejected plan cannot
have half-built anything. Rows are ordered canonically by identity, so request
order is not an input.

**A parent may be an already-live identity; a relation target may not.** A
summoner outlives the summon it plans, so a live parent is ordinary. A relation,
by contrast, is wired at commit from the entities the plan just constructed, so
the executor holds nothing for an outsider. Both ends must be rows in the same
plan. Relating to a live entity is a real need and belongs with the commit
boundary Phase 4 gives a live identity index.

**One constructor serves ordinary construction and reconstruction.** Preparation
resolves the recipe identity and confirms it is registered, so commit cannot
discover a missing recipe after the outgoing world has begun to retire. There is
one executor — `commit_subset` — and a full commit is it over every row while a
single-entity rebuild is it over one. They cannot drift because there is nothing
for them to drift between.

**A partial commit that would CUT a relation is refused, in either direction.**
A subset containing exactly one end of a planned relation is rejected before the
first recipe runs. Both directions matter and only one is obvious: rebuilding the
SOURCE alone leaves it unwired, while rebuilding the TARGET alone leaves the
untouched source holding a handle to the entity that just died. A relation is an
`Entity`, so the row that "still holds it" holds a corpse. In both cases the
roster is the right length and only the wiring is wrong — the failure mode that
survives every count-based check. `ConstructionPlan::relation_closure` grows a
seed set until nothing crosses its boundary, so the refusal is solvable.

**A recipe is infallible by type, and preparation makes that true.** It returns
nothing at all — not an `Entity` to be distrusted and not a `Result` to be
half-applied. Everything that can miss — a registry lookup, a catalog id, a
relation target — resolves in the request builder, where failing is free and the
live world is still whole.

That signature alone was not enough to earn the claim, and neither was the
`AcceptsFn` first used to shore it up. A validator registered independently of
its constructor stores the same variant-compatibility fact twice, so the two can
disagree — and one that wrongly returns `true` still reaches the constructor's
`unreachable!` mid-commit. Storing a fact twice was the very thing this ADR
rejects elsewhere, for the same reason.

So the pairing is not checked, it is unrepresentable.
`ConstructionDomain::dispatch` is ONE exhaustive match returning both a row's
recipe identity and the constructor that builds it, so `ConstructionRequest` has
no `recipe` field to set wrongly and a variant with no arm is a compile error.
Two matches — one for the label, one for the behaviour — would have been able to
drift while still compiling; one arm names both.

**Preparation freezes the resolved constructor onto the row.** `dispatch` is
expected to be pure, but nothing makes it so: an implementation may read an
atomic or any other mutable process state. Re-resolving at commit therefore let
a plan validate recipe A, dump recipe A, fingerprint recipe A, and execute
constructor B. `PlannedEntity` stores the resolved `ConstructFn` and commit runs
that. The pointer is never canonicalised — a `fn` address is runtime execution
state, not content identity; the stable `RecipeId` is what the dump and the
fingerprint carry.

**The construction registry contributes to the prepared-content fingerprint.**
A recipe table decides how authoritative entities are built, so two sessions
whose recipe schemas differ are not interchangeable and a snapshot taken under
one is not safe to restore under the other. `prepare_platformer_content` hashes
the registry's canonical dump as the `construction.recipes` section. Only stable
semantic metadata is hashed — recipe id, owner, source, schema id, relation kind
and owner — never a pointer or anything process-local. One consequence worth
stating: a relation whose WIRING FUNCTION changes while its owner does not will
not move the fingerprint, so a behavioural change to a relation must come with a
schema-id bump.

**The executor allocates every authoritative root; a recipe never chooses one.**
A recipe used to return an `Entity` the executor then stamped, guarded by a check
that it did not already hold a `SimId`. That guard was weak three ways: a
pre-existing entity WITHOUT an identity passed it and was commandeered silently;
it ran at flush, so it was a panic after other rows had queued their mutations
rather than a refusal; and nothing tied the returned entity to this commit. The
executor now calls `spawn_empty`, stamps identity and provenance onto the result,
and hands the recipe a [`ConstructionRoot`] it cannot forge. Freshness is
structural, so there is no check left to get wrong.

⚠ **This still does NOT make plan-to-world roster parity a type-level
property.** A recipe receives raw `Commands` and the root `Entity`, so a broken
recipe can despawn that root, overwrite or remove its `SimId`, mutate unrelated
entities, or invent another authoritative identity. `ConstructionRoot` prevents
a recipe NOMINATING a pre-existing entity as a row's root; it is not a capability
that confines what a recipe may otherwise do.

The production migration has nevertheless crossed its deletion gate: every
authoritative room-construction family, including giant hand limbs, is an
explicit plan row. `verify_committed_roster` remains the enforcement backstop at
the real room boundary and withholds `RoomLoaded` for any violation. The former
`LegacyConstructionRoot`/known-family exemption and `Unmigrated` severity class
were deleted rather than kept as an empty compatibility path.

Three vocabulary terms keep these apart, and are used consistently throughout:

| term | what it means | status |
|---|---|---|
| **executor invariant** | the executor allocates each nominal planned root and freezes its constructor | held, mechanically |
| **verification invariant** | the production boundary detects missing, duplicate, replaced, corrupted, or unexpected authoritative roots and incorrect relationships | held, as a detector |
| **production roster invariant** | every authoritative root produced by room construction is an explicit plan row; helpers are explicitly non-authoritative | held by the migrated production families and verified at commit |

**A domain supplies what core cannot know** — `ConstructionDomain::Parameters`
(what a row carries) and `Services` (the frozen catalogs its recipes read).
Recipes never downcast, and a plan cannot be committed against the wrong domain.

## Consequences

The original three representative families (an authored `GroundItemSpec`, a
provider-staged `SpawnActorRequest`, and an `Effect::Summon` minion) proved the
three origin kinds. The migration then moved authored placements, static room
families, every enemy and boss, and derived giant limbs onto the same planner.
There is no room-construction family-specific authoritative spawn loop left, and
there is no legacy exemption at the verifier. The three original silent skips
are preflight failures, and a summoned minion takes `SimId::spawned` under its
summoner, so two summons reusing one authored id no longer collide.

Provider-staged actors stopped being deferred. They were written as
`SpawnActorRequest` messages and applied a system later; they are plan rows
committed with the rest of the room. `apply_spawn_actor_requests` survives for
programmatic scene setup (RL episode reset, demo spawns), which legitimately
wants a message.

`ContentEpoch` moved from `ambition_platformer2d_runtime` to `ambition_platformer2d_core`.
Construction planning sits far below the crate that owns content identity and
must be able to state the generation a plan was prepared against; allocation
stayed where it was, and only the stamp moved. That stamp is **recorded, not
enforced** — turning a stale plan into a refusal belongs to the commit boundary,
which Phase 4 owns.

Sequence numbers are **taken while planning and written back only on commit**.
`SimIdCounter` is snapshot-registered authoritative state, so advancing it while
assembling requests would let a rejected batch consume dynamic identities that no
entity was ever built for — a mutation that outlives the refusal and rides into
the next snapshot. "Preparation is pure" has to be true of the system that calls
the planner, not only of `prepare`.

A summon whose emitter has no simulation identity is now refused and logged
rather than given a parentless dynamic id. Every body carrying a `FeatureId` is
identified at the head of the tick, so this cannot arise for authored content;
reaching it means the emitter is outside the identity migration.

Authoritative room construction no longer has family-specific spawn loops.
Family migrations landed by moving each family completely — plan row and recipe
in, old authoritative loop out — so the planner is the one room-construction
road rather than a second road beside the old one.

## Alternatives considered

**Keep parsing the id, and fix the grammar instead.** Rejected: it preserves the
coupling that made a legibility convenience into a reconstruction contract, and
it cannot help an entity whose spelling is wrong for its family — which was the
actual bug, not a formatting accident.

**Store the owner's `SimId` in `ProjectileOwnerId`.** Rejected as too narrow. It
would have fixed the projectile family and left every other dynamic family with
no provenance at all, and it is the field whose incorrect justification hid the
problem in the first place.

**Type-erased recipe parameters (`Box<dyn Any>`) for an open registry.**
Rejected for this slice. Downcasting turns a domain mismatch into a runtime
failure inside execution — after mutation has begun — which is the failure mode
planning exists to remove. The campaign explicitly warns against freezing public
APIs early; provider-extensible recipes are Phase 6's problem, and the generic
domain can grow into them.

**Carrying `RecipeId` inside `SpawnOrigin`** (as the campaign's sketch showed).
Rejected: the planned row already names the recipe, and storing it twice creates
a state where the two disagree with nothing to say which wins.

## Current implications for agents

- **Never recover a fact from a `SimId` string.** No `split`, `strip_prefix`,
  `starts_with`, or delimiter arithmetic. If reconstruction needs to know where
  an entity came from, read `SpawnOrigin`; if the fact you need is not in it,
  add it there rather than encoding it in the id.
- **A new dynamic spawn site stamps `SpawnOrigin::Dynamic` with its parent and
  the spawner's own `SimIdCounter` sequence**, at the point that already has
  both. A dynamic entity with no stated parent is unreconstructable.
- **Resolve in the plan, not in the recipe.** Anything that can fail — a
  registry lookup, a catalog id, a relation target — belongs in the request
  builder, where failing costs nothing. A recipe that can fail has moved a
  content error inside the mutation.
- **Do not add a second constructor for reconstruction.** Rebuild one entity
  with `ConstructionPlan::construct_one` (a `commit_subset` of one), and when it
  is refused for cutting a relation, rebuild `relation_closure` of what you
  wanted rather than reaching past the refusal.
- **A new parameter variant needs one arm in `dispatch`**, naming both its
  recipe identity and its constructor. The compiler enforces that the arm
  exists; `every_parameter_variant_matches_its_descriptor` enforces that it
  names the right pair.
- **A relation whose wiring behaviour changes needs a schema-id bump.** Kind and
  owner alone do not distinguish two behaviours, and the fingerprint hashes
  metadata rather than pointers, so without the bump the change is invisible to
  content identity.
- **After a room transaction commits, run `verify_committed_roster`.** It is a
  detector, not a preventer: recipes hold raw `Commands` and Bevy commands do
  not roll back, so a violation stops the transaction being published rather
  than undoing it. `RoomFeatureConstructionPlan::spawn` already does this by
  queueing a capture command before its construction and a verify-and-publish
  command after it; a new construction boundary must do the same rather than
  writing its own success message.
- **Never take the authoritative scope from a caller-supplied list.** A caller
  enumerates what it remembers building, and the roots worth catching are the
  ones nobody remembered. `AuthoritativeScope::gather` queries the world, and
  treats an unclassified identity-bearing entity as a finding rather than as
  absent. An entity that is genuinely not authoritative says so with
  `PresentationOnly`.
- **Registration identity is metadata; never compare function addresses.** The
  compiler may merge identical functions to one address and emit one function at
  several, so `fn_addr_eq` makes a registry contract depend on optimisation
  level. Behaviour is governed by `schema_id`, which is also what makes a change
  visible to the prepared-content fingerprint.
- **A relation declares its postcondition alongside its wiring.** A receipt
  records that a wiring function was CALLED, which a no-op, a write to the wrong
  entity, and a later overwrite all satisfy identically. `RelationOps` carries
  `wire` and `verify` together so the two cannot be edited apart.
- **A bidirectional relation wires and verifies BOTH ends in one function.**
  `Limb`/`LimbRig` and `RidingOn`/`MountSlot` are pairs that must agree, and the
  way they break is a half-write. Checking only the forward side accepts a limb
  its host's rig does not drive (inert — `fan_out_limb_intents` iterates the rig)
  and a mount that does not point back (disobedient —
  `steer_mount_from_rider` queries `With<MountSlot>`).
- **Facts stated relative to one end belong on the RELATION, not the entity.**
  `Limb`'s `slot` and `home_offset` are both host-relative, so they ride on
  `ConstructionDomain::RelationPayload`. Putting them in the limb's construction
  parameters would place host-relative data on a body that does not learn its
  host until wiring — the same shape as the duplicated `parent` field this ADR
  already deleted.
- **Never store a relationship between two authoritative entities as a bare
  `Entity` outside the plan.** `Limb`/`LimbRig` and `RidingOn`/`MountSlot` still
  do, which is why partial reconstruction cannot see them. Declare it as a
  planned relation so cut-detection and `relation_closure` cover it.
- **Assert plan-to-world parity against the WORLD.** Comparing a receipt to the
  plan compares the executor's bookkeeping with itself and stays green even if a
  recipe built nothing. Query the identities that are actually alive.
- **Take authoritative counters while planning; write them back on commit.** Any
  spawn site that advances snapshot-registered state before its plan is validated
  has mutated on the failure path, whatever its error branch claims.
- A new authoritative room-construction family enters through the planner from
  its first implementation. Reintroducing a family-specific authoritative spawn
  loop would create a second construction road, not a transition state.
- **The actor construction domain is CLOSED.** `ActorConstructionParams` is a
  closed enum and `ConstructionDomain::dispatch` is a closed exhaustive match, so
  a provider cannot add an executable recipe — only *metadata*. The registry
  accepts a recipe identity (owner, source, schema id) and that identity reaches
  the prepared-content fingerprint, but nothing outside this crate can supply a
  parameter variant or construction behaviour. Do not tell a provider author
  otherwise. Opening it needs an erased prepared-payload design that couples
  validation, metadata, and execution in one registration; that is Phase 6's
  problem and is deliberately not attempted here.
