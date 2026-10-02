# Instance lifetime, provenance and persistence

**State:** DISTILLED. The provenance and lifetime foundation is implemented.
The open-world, custody and reconstitution plans own the remaining product
semantics.

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
- world/per-owner uniqueness without conflating identity with definition;
- how much provenance is product state versus diagnostics.

Persistent relocation is settled (Q38 and OW3 in
[`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md)).
An item or persistent character left in another room has a durable
`Placed { room, at }` row and is built there. A population body away from home
is held as carried while it lives, and its home builds the replacement when its
room retires.

## A claim is released on every frame nobody claims it

A shared claim (a music owner tier, a hidden-mesh latch) must be released by a
system that can reach the "nobody claims" arm on every frame. A release that
runs only inside the one-shot that took the claim, or only while a live script
emits it, is never reached after a despawn or a death. An effect fires once. A
despawn fires nothing.

- Ask on which frames the release is reachable, not whether a release call
  exists. A "every claim has a release" grep cannot catch this, so this is a
  rule, not a checker.
- The correct shape: a system with no run condition that reaches the "no owner"
  arm on every frame (the boss-music owner is the pattern).
- Release only the owner's own claim. Clearing the whole tier silences whoever
  legitimately holds it.

Stale music claims are player-visible, because `EncounterMusicRequest`
priority tiers rank above room music.

## Qualify identity by the authority that interprets it

Do not merge authored content IDs, live entity IDs, item occurrence IDs,
construction-attempt IDs, session IDs and rollback wire IDs into one universal
identity type. They have different equivalence and lifetime rules. Every state
move must name its scope, accepted writers, restoration and retirement boundary.

A1 keeps checkpoint restoration progress with session; item baselines remain
item-owned under A7. Two live instances of one authored room are qualified by
their live room (`InRoomInstance`, `LiveBodyId`), not by a universal identity.
A10 cannot infer reversible global
mutation from a construction-attempt marker. Stale async preparation/agent
responses must be rejected by the appropriate generation/revision owner.

## Extension state uses these lifetimes, not an independent taxonomy

A schema selects attachment, deterministic initialization, rewind participation,
retirement and save eligibility separately. Reuse domain occurrence/session scope;
do not collapse them into a module-global integer or an ECS Entity. A suspended
world record can retain a durable reference without a live entity.

Promotion/demotion transfers write ownership at the accepted lifecycle boundary.
A mutable dormant ledger cannot be read as an unregistered simulation oracle.
Use admitted immutable revisions or registered active state. Candidate attempts
own cleanup, not durable namespace. See [state/execution](extension-state-and-execution.md)
and [open-world planning](open-world-runtime-and-residency.md).
