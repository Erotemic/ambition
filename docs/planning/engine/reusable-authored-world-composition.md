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

An invocation can bind existing objects through the same occurrence vocabulary
without constructing them. [Story SS7](story-sequences.md) selects lexical
roles for a scene and conversation. Definition/local-child names, a bound
invocation role and a live population query are different values. The shared
operation is checked reference binding; construction publication and narrative
playback remain separate owners.

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

**Landed (2026-11-22).** The shipped script
(`examples/projects/bridge_warden/modules/bridge_warden.rhai`) now creates the
warden with its `ward` binding in the same `create_actors` node. The binding
names the posts by their local keys (`#{local: "north"}`), so binding and
subscriptions are complete when the batch commits, and no post can fall
before the warden is listening. Removed: the indexing into the result vector,
the `attach_behaviors` follow-up, the `attached` entry and its state field.
`made` keeps the refusal check and records what was made. The test's inline
caller makes the warden the same way. Its own `clearer` watch is still
attached after creation, because the caller is an existing body outside the
batch, which is an external reference, not a local one. No helper was needed:
the declaration is one function. Witnesses: the five `scripted_bridge_warden`
tests, unchanged in what they assert (defeats, retirements, world count, the
gate in the next room, the live edit, the portable-module methods).

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

**Progress (2026-11-24): the native binding a switch makes to a lift.**
Before this, a lift moved only when ridden, and no switch could drive one. Now:
- A lift authored with `lift_called` (LDtk field; spec
  `MovingPlatformMotionSpec::Lift { called }`) is sent by a switch instead.
  It waits at both stops (`LiftStage::Parked` at the far one), ignores
  riders, and `MovingPlatformState::send` takes it to the stop it is not at.
  A lift already travelling, or a rider's lift, refuses the send.
- A switch whose action is `MoveLift` names the lift's id as its target.
  `ambition_encounter_features` resolves the press in the switch's own live
  room, so the switch explicitly binds to the lift, and the platform domain
  still owns the motion. Each floor's switch sends the lift away from where
  it stands, so two switches serve both stops.

Witnesses:
- `a_called_lift_goes_only_when_sent_and_waits_at_each_stop` (world lib);
- `a_lift_switch_sends_only_the_lift_in_its_own_live_room` (app_it): the
  same lift id in two live rooms, and Bob's press sends only Bob's.

**Progress (2026-11-24): one request builds the mechanism.**
`create_mechanism` (`ambition_world_port::mechanisms`, lowered in
`ambition_platformer2d_runtime::mechanism_creation`) makes lifts and switches
together. A switch's `sends` names a lift of the same request by local key
(`#{local: "lift"}`, the `ObjectTarget` W1's bindings use). Each part keeps
its native owner:
- a switch is the interactable placement an authored `Switch` lowers to,
  committed through the runtime construction road, with `MoveLift` bound to
  its lift's construction identity;
- the lift is a switch-sent row added to the caller's room's
  `MovingPlatformSet` once the switches have committed.

Everything is checked before anything is made: keys unique, boxes finite,
each `sends` naming a lift of the request. A refusal names the switch, the
field and the key, and makes nothing. Both kinds of part have the caller
room's lifetime, as scripted breakables do. The switch wire parser now keeps
the rest of the payload as the target, because a construction identity can
contain its separator.

Witness: `a_scripted_mechanism_binds_each_switch_to_its_own_lift`
(`scripted_mechanism`, under sync-test rollback). Two copies made with the
same keys in one room each bind their switch to their own lift. A player's
press on the first sends only its lift, which parks at its far stop. A third
request with a dangling key is refused whole. Then the caller retires the
first copy's switch (`retire_creation`) under the same rollback. It leaves,
its lift stays where it was, and the second copy's switch still sends only
its own lift: a rollback through one child's retirement leaves the other
instance alone.

Still open:
- ~~the door~~ (landed 2026-11-25, below: a door is its own occurrence);
- ~~a lift has no object occurrence~~ (landed 2026-11-25, below);
- the third copy in a second live instance of the same authored room. This
  waits on the session: nothing yet makes two copies of one room live at
  once (the re-entry witness `a5ad66041` records that as open). The per-room
  binding is already witnessed across two different live rooms
  (`a_lift_switch_sends_only_the_lift_in_its_own_live_room`).

**Progress (2026-11-25): a lift is an object of its own.**
Each lift of a mechanism is now a row of the same construction batch as its
switches: `ActorConstructionParams::MechanismLift` (recipe
`ambition.mechanism-lift`) builds its occurrence, a `MechanismLift` naming
the platform row. Its identity is minted like every runtime creation's
(the caller's identity and a sequence), and a request names it by local
key.

- **The answer.** `create_mechanism` lists the switches, then the lifts'
  occurrences.
- **Retirement.** `retire_creation` takes a lift away alone. The platform
  domain's observer `a_retired_lift_leaves_its_room` removes the row from
  the occurrence's own live room only, whichever road removed the
  occurrence.
- **Rollback.** GGRS reproduces the occurrence (`entity:mechanism_lift`,
  schema 543), so a rewind across a creation or retirement brings back the
  object and its row together. The platform set is already rollback state.

Witness: `a_scripted_mechanism_binds_each_switch_to_its_own_lift` goes on,
under the same sync-test rollback, to retire the first copy's lift by its
occurrence. Its row leaves the room, the second copy's lift stays where it
was, and only the retired occurrence is gone.

**Progress (2026-11-25): a door is an object of its own room.**
An authored gated lock wall or loading-zone door is room data, shared by
every live instance of the room, so it could not be one copy's door. A
mechanism door is an occurrence instead:

- **The part.** `create_mechanism` takes `doors` (`DoorNode`: key, centre,
  size, `open`). Each is a construction row of the same batch
  (`ActorConstructionParams::MechanismDoor`, recipe `ambition.mechanism-door`),
  and the occurrence holds its own box and whether it is open
  (`ambition_platformer2d_world::mechanism_doors::MechanismDoor`).
- **The gate road.** A shut door is a solid in its own live room's
  `gate_solids`, under the block name `mechanism_door:<id>`. Its contributor
  is the third writer `WorldGatingSchedulePlugin` schedules, and it runs only
  while a door exists. The lock-wall visual draws it like any gate solid.
- **The switch.** A switch's `sends` names a lift or a door by local key. A
  door's switch carries the `ToggleDoor` action, resolved in the switch's own
  live room, so a press opens or shuts only that copy's door. The switch
  stores nothing; the door keeps its state.
- **Rollback.** GGRS reproduces the occurrence (`entity:mechanism_door`), and
  its state rewinds with it (`entity.mechanism_door`, canonical; schema 544).
- **The answer.** The switches, then the lifts' occurrences, then the doors.

Witness: `a_scripted_mechanism_binds_each_switch_to_its_own_lift` goes on,
under the same sync-test rollback, to make a door and its switch in one
request. The new door is shut and stands in its room's gate solids; a press
opens it and removes the solid, and the next press shuts it again.

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
