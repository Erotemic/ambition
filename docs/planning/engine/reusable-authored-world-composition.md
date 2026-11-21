# Reusable authored world composition — Engine 1.0 design program

**State:** PLANNED, 2026-11-21. Start from the existing local-reference
construction operation. A general authored format still requires two
materially different uses. [The queue](../queue.md) selects work.
**Source review:** `6895e5168638`; no new implementation or runtime result is
claimed here.

## Goal

An author can place or create a reusable mechanism whose parts refer to each
other. The mechanism has local names, explicit parameters and explicit external
bindings. Each occurrence owns its state. It uses ordinary domain construction,
behavior and publication.

The useful shared operation is to bind a definition's local references to one
instance. An authored hierarchy, a spatial parent tree and a lifecycle owner
are separate relations. None requires the others to have the same shape.

## Current authority and evidence

The former hold relied partly on the absence of a type named `Assembly`,
`Prefab`, `Composition` or `Motif`. That is no longer a useful design test.
The authored `PlacementKind` is still flat, but current construction can bind
references among several objects before they become visible:

| Existing operation | Owner |
| --- | --- |
| Distinguish an existing occurrence from a name local to a creation batch | [`ObjectTarget::Live` and `ObjectTarget::Local`](../../../crates/ambition_world_port/src/object_target.rs) |
| Prepare root state and state owned by each object | [`ComposedInitialization`, `OwnedObjectState`, `ObjectInitialization`](../../../crates/ambition_platformer2d_construction/src/runtime.rs) |
| Allocate the batch's occurrence references, resolve local names, prepare state, then commit hidden roots | `RuntimeConstruction::commit` in the same owner |
| Bind behavior references and initialize subscriptions from the complete local map | [`BehaviorInitialization::prepare`](../../../crates/ambition_platformer2d_runtime/src/object_behaviors.rs) |
| Lower actor creation, behavior bindings and native actor relations through that construction | [`actor_creation::lower`](../../../crates/ambition_platformer2d_runtime/src/actor_creation.rs) |

These are implementation evidence for local binding, not evidence that a
reusable world format is complete. Actor batches do not yet prove that a lift,
switches and door can share one authored composition.

The [Bridge Warden script](../../../examples/projects/bridge_warden/modules/bridge_warden.rhai)
is the first customer. It creates a warden and two posts, reads their identities
from the result vector, then attaches a behavior with references and
subscriptions. This works. It also makes the author write construction and
binding order. Creation-time behavior bindings with local targets already
exist, so the first improvement can remove that order from this script without
adding a new engine execution model.

## Decision: local names and explicit bindings

A reusable definition describes its parts using names local to the definition.
Its placement or creation supplies parameters and any external references. The
owning domains prepare the parts. Construction binds their local names to new
occurrence references and publishes the complete instance through the current
transaction.

Keep these identities separate:

| Identity | Meaning |
| --- | --- |
| Definition | Shared authored content, with its admitted generation |
| Local child key | A part within that definition |
| Authored placement | One persistent use of the definition in authored world content |
| Live occurrence | The live object's identity under the owner's session and lifetime rules; distinct from a current Entity handle |
| External binding | A reference supplied by the caller or placement under a stated access rule |

A child key resolves only in its instance. It cannot fall back to a room-wide
name lookup when a local target is absent. A reference outside the instance
must use an explicit external slot. The slot declares the domain or capability
the target must provide; successful name lookup alone is not admission.

For persistent authored instances, derive child identity from the authored
placement and stable child key, within the existing room/session scope.
Reordering declarations must not change those identities. Ordinary dynamic
creation retains its occurrence and provenance rules. Do not give two dynamic
invocations the same live identity because they use the same definition.
`RuntimeConstruction` currently uses a creation sequence and row index; adapt
the authored-instance input deliberately instead of treating that index as a
persistent child key.

A persistent child-key rename changes authored identity unless an explicit
mapping says otherwise. Store the durable placement/child key in persistent
data, not a session-local ObjectRef or Entity. Restore and reconstitution
re-establish live references through the existing occurrence policy; a
rollback can restore the same occurrence, while a retired reference must not
become valid merely because a later child has the same authored name.

Parameters are typed inputs to owner preparation. Each domain states which
values can be supplied. Reject unknown parameters, duplicate local keys and
conflicting assignments. Start with a flat definition. Do not add inheritance,
arbitrary field-path patches or implicit override precedence to obtain reuse.

## Two binding stages, two lifetimes

The [capability linking plan](capability-linking-and-prepared-facets.md) owns
composition of shared definitions. At that stage, resolve the character,
movement, behavior and other definition references the assembly uses. Preserve
the source and local key in diagnostics. The result is immutable content.

Occurrence construction then resolves local child names and explicit external
bindings to live references. It prepares all required private state and
subscriptions before publication. A required child or binding that fails
preparation refuses the instance; it does not leave the other children live.
Both stages use typed domain contributions, but their reference types and
validity periods are different.

Keep domain semantics with their owners. A lift still uses kinematic motion;
a door still uses its normal gate or portal behavior; a post still receives
native damage and defeat. An assembly does not interpret those mechanics or
add a universal spawn dispatcher. LDtk and scripts can be frontends to the same
prepared construction operation.

## Implementation slices

### W1 — express one complete creation using the existing local map

Change the Bridge Warden example to supply the warden's behavior and post
references in its creation request. Use local keys for the posts. Remove the
result-vector indexing and the follow-up attachment action that exist only to
complete this binding. Keep result handling for actual creation refusal.

If the authored declaration remains repetitive, add the smallest typed
authoring helper that expresses that batch. Do not add another port, runtime
state machine, file format or crate for this slice. Preserve the current
native defeat, retirement, durable world-count and gate behavior.

### W2 — bind a mechanism with different domain parts

Use a lift, two switches and a door as the second customer. The switches bind
explicitly to the lift or its controller; the door uses its existing domain
operation. The caller supplies placement and any external destination. Domain
owners still prepare and execute motion, interaction and gate semantics.

This is a new acceptance customer, not a claim that the current actor-only
request already handles it. Extend existing typed construction contributions
where needed. Do not force a lift or door into a character solely to reuse the
first request type.

The shared part must be local naming, parameter binding, construction and
occurrence isolation. If W1 and W2 share only the spelling of a container,
keep their helpers separate. If they share that operation, define the minimal
authored assembly format and its domain lowering at this point.

### W3 — expose placement and reuse through existing authoring tools

Let an LDtk placement select the prepared definition and supply its parameters
and external slots. Let scripted creation select the same definition under its
normal creation authority. Use one definition expansion and one binding path.
Remove duplicated per-instance reference wiring from the two customers.

Expose the expanded parts, local references, external slots and their source
provenance through the existing semantic inspection tools. This is a projection
of prepared content, not an editor database. Runtime inspection adds occurrence
identity and state from existing domain read models.

Nested definitions, exported interfaces and additional authoring backends can
follow a real need. They are not prerequisites for the first two customers.
Do not create a crate until the shared API and a useful dependency boundary are
established.

## Acceptance

- The Warden mechanism still observes native defeat and retirement, writes
  durable world state and opens the existing gate. Removing the post-result
  binding code does not change that behavior.
- The lift mechanism uses the normal movement, interaction and gate owners.
  It does not add copies of those implementations to the assembly layer.
- Two copies in one room bind internally to their own children. A third copy
  in a second live instance of the same authored room stays separate. One
  explicit external binding is the only permitted connection in the scenario.
- A missing local target or invalid child parameter refuses the complete
  instance before publication. The refusal identifies the definition, local
  key, field and source.
- Reordering authored child declarations keeps persistent child identities.
  A save/restore or rollback through one child's retirement does not affect
  another instance's state or subscriptions.

Use the existing construction and gameplay witnesses at the narrowest useful
scope. Do not add source-name, file-count or planning-text guards.

## Boundaries to preserve

The provider-neutral world input in A3 of the
[frontier](actor-monolith-work-frontier.md) stays separate from actor/catalog
lowering. An authored spatial assembly must not import prepared character
sheets merely to expose placements.

The [open-world residency](open-world-runtime-and-residency.md) contract already
allows two live instances of one authored room. An assembly must preserve the
same distinction: shared content, separate occurrences and mutable overlays.

Do not mirror authored composition in Bevy parenting by default. Spatial
transform inheritance, ownership of private state, lifetime ownership and
semantic references have different rules. Do not add a universal entity tree,
recursive component bag, game-specific dispatch or runtime inheritance chain.
Ordinary direct domain construction remains a valid authoring path.
