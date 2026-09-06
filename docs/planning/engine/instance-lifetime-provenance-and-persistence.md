# Instance lifetime, provenance and persistence

**State:** DISTILLED — the provenance/lifetime foundation is implemented; the
remaining product semantics are owned by open-world, custody, and reconstitution
plans.

## Current model

Authoritative instances have explicit provenance/lifetime semantics rather than
being classified by which constructor happened to spawn them. Important current
vocabulary includes session/room scope, authored placement provenance, runtime
spawn identity, occurrence/disposition facts, and persistent ledgers used by a
fresh construction/restore path.

The durable lesson is:

> existence, residency, rollback history, durable occurrence identity, and
> presentation are different lifetimes.

A relationship may cross a durable save/load horizon only when the durable road
can restore the authority for that relationship. For example, item custody may
be persisted because item inventory/custody has a durable reconstruction path;
transient possession of an actor is not made durable merely because both happen
to project through a generic live relationship component.

Do not infer a universal `InstanceId` requirement from shared vocabulary.
Domain-specific actor/item/world-object IDs remain acceptable until common
operations demonstrate a real reusable core.

## Remaining owners

- [`construction-and-reconstitution.md`](construction-and-reconstitution.md) —
  which populations are retained/reconstructed at session/room/replay/restore
  boundaries.
- [`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md) —
  resident versus nonresident world state.
- [`item-custody-and-accounting.md`](item-custody-and-accounting.md) — item
  occurrence, inventory and physical custody.

## Still-open questions

- terminal versus resettable occurrence/tombstone semantics;
- stable identity required across a fresh process versus identity that may be
  deterministically regenerated;
- persistent relocation of actors/items away from authored home placement;
- world/per-owner uniqueness without conflating identity with definition;
- how much provenance is product state versus diagnostics.

The implementation campaign and dated measurements that established this model
remain recoverable through git history; they are no longer active planning.
