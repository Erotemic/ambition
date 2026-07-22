# ADR 0030: Spawn provenance is data, and construction is planned before mutation

## Status

**Accepted; implemented for three origin families** (2026-07-22; revised twice
the same day after two rounds of external review. The second round found four of
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
`ambition_platformer_primitives::construction` owns the content-free vocabulary:
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
`ConstructionDomain::recipe_of` derives a row's recipe from its parameters, so
`ConstructionRequest` has no `recipe` field to set wrongly, and
`ConstructionDomain::construct` is one exhaustive match, so a variant with no
construction arm is a compile error. The registry keeps its ADR-0026 identity
role — ownership, idempotent re-registration, conflict rejection, fingerprint
contribution — and no longer dispatches.

**The executor allocates every authoritative root; a recipe never chooses one.**
A recipe used to return an `Entity` the executor then stamped, guarded by a check
that it did not already hold a `SimId`. That guard was weak three ways: a
pre-existing entity WITHOUT an identity passed it and was commandeered silently;
it ran at flush, so it was a panic after other rows had queued their mutations
rather than a refusal; and nothing tied the returned entity to this commit. The
executor now calls `spawn_empty`, stamps identity and provenance onto the result,
and hands the recipe a [`ConstructionRoot`] it cannot forge. Freshness is
structural, so there is no check left to get wrong.

Recipes may still create deliberate *child* entities. Where such a child is
itself authoritative — a giant's hand limbs mint their own `SimId` — the plan
does not yet name it, and that is recorded as Phase-4 work rather than claimed
as parity.

**A domain supplies what core cannot know** — `ConstructionDomain::Parameters`
(what a row carries) and `Services` (the frozen catalogs its recipes read).
Recipes never downcast, and a plan cannot be committed against the wrong domain.

## Consequences

Three families are migrated (an authored `GroundItemSpec`, a provider-staged
`SpawnActorRequest`, an `Effect::Summon` minion) — one per origin kind, as the
campaign specifies. The three silent skips above are now preflight failures, and
a summoned minion takes `SimId::spawned` under its summoner, so two summons
reusing one authored id no longer collide.

Provider-staged actors stopped being deferred. They were written as
`SpawnActorRequest` messages and applied a system later; they are plan rows
committed with the rest of the room. `apply_spawn_actor_requests` survives for
programmatic scene setup (RL episode reset, demo spawns), which legitimately
wants a message.

`ContentEpoch` moved from `ambition_runtime` to `ambition_engine_core`.
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

Most of a room is still built by family-specific loops. That is Phase 4's
migration order, not an oversight: a partial sweep would have forked families
rather than moved them.

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
- **A new parameter variant needs an arm in `recipe_of` AND in `construct`.**
  The compiler enforces the second; the first is what keeps the dump and the
  registry honest.
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
- When migrating a family in Phase 4, delete its family-specific spawn loop in
  the same commit that adds its recipe. A family that is planned *and* looped is
  a duplicate spawn, not a transition state.
- The recipe registry is open: a provider may register its own recipes into
  `ActorConstructionRegistry` before the first room is planned. Registration is
  idempotent for a byte-identical entry and rejects a conflicting one.
