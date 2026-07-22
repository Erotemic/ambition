# ADR 0030: Spawn provenance is data, and construction is planned before mutation

## Status

**Accepted; implemented for three origin families** (2026-07-22). Completes
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
the executor holds nothing for an outsider — and accepting such a relation only
to skip wiring it would be a new silent drop inside the machinery built to
delete silent drops. Both ends must be rows in the same plan. Relating to a live
entity is a real need and belongs with the commit boundary Phase 4 gives a live
identity index.

**One constructor serves ordinary construction and reconstruction.** A recipe is
a plain `fn` pointer resolved at planning time and stored beside its row, so
commit repeats no registry lookup and cannot discover a missing recipe after the
outgoing world has begun to retire. Rebuilding one entity is the same call with
one row.

**A recipe is infallible by type.** It returns `Entity`, not
`Result<Entity, _>`. Everything that can miss — a registry lookup, a catalog id,
a relation target — resolves in the request builder, where failing is free and
the live world is still whole. Expressing that as a signature rather than a
convention means a recipe author cannot quietly move a content error inside the
mutation, and it leaves commit with no failure path to half-apply.

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
  with `ConstructionPlan::construct_one`. Two constructors drift, and the drift
  only shows up after a restore.
- When migrating a family in Phase 4, delete its family-specific spawn loop in
  the same commit that adds its recipe. A family that is planned *and* looped is
  a duplicate spawn, not a transition state.
- The recipe registry is open: a provider may register its own recipes into
  `ActorConstructionRegistry` before the first room is planned. Registration is
  idempotent for a byte-identical entry and rejects a conflicting one.
