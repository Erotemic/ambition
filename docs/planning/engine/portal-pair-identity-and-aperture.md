# Portal connections: identity, topology edits and lifetime

**State:** stable channel assignment and held pair ports are built. The
connection identity, edit admission and scoped lifetime work below is
**PLANNED**, 2026-11-21.
**Delivery owner:** [portal parts and similarity](portal-parts-similarity-and-chirality.md),
under [PORTAL-SIMILARITY](../queue.md#portal-similarity--canonical-parts-and-scaled-mirror-portals).
This page owns the topology detail, not a second delivery campaign.

This revision comes from source review of `b91b29d35935`. No runtime change,
build, benchmark or reproduction was performed for this review. The failure
scenarios below are derived from the named production paths. Existing test
receipts prove only the cases they exercised.

## Decision

A physical portal is a connection with two directed ends, one live physical
residence and an explicit lifetime source. Its endpoint geometry can move.
Its color and its address in a query snapshot are separate facts.

The portal domain owns which connections are admitted and how they change.
The material owner supplies occupancy and geometric admission. Source adapters
state placement and lifetime intent. Presentation reads the accepted result.
No domain independently reconstructs the connection from colors or positions.

Use the existing room, occurrence, construction and rollback contracts. This
requires no universal scene graph, entity registry, spatial link database or
new public type names before the first migrated callers prove the boundary.

## Current implementation and retained receipts

- `ambition_portal2d::resolve_portal_links` keeps each living link group's
  channel pair and each endpoint's assigned role. Another group's arrival or
  removal does not renumber it. Position selects roles only at first assignment.
- Capacity is 63 explicit link groups **per live room**. A further group gets
  dead channel 255. Authored `c128` and above are refused. The October 12
  insertion, role-crossing and per-room capacity witnesses remain useful.
- `HeldPortalPairPort` and its domain adapter are built. GNU-ton, GNU2 and the
  Smash recovery use the port. `PortalLook` separates their look from pairing.
  The earlier plan to add this port is complete.
- Each endpoint keeps its full authored aperture. Unequal endpoints use the
  directed similarity ratio. The old derived-minimum aperture rule and its
  witnesses are historical; they are not future acceptance criteria.
- Room-scoped host attribution, carves, transit and portal-frame retraction
  exist. `PortalFramesInRooms` restores changed history for a room rewind.
- Transit, backside, cooldown and frame history still refer to channels.
  Stability while a pair lives does not prove continuity across its retirement,
  slot reuse, room transfer or replacement.

Source: [`link.rs`](../../../crates/ambition_portal2d/src/link.rs),
[`extension.rs`](../../../crates/ambition_portal2d/src/extension.rs),
[`types.rs`](../../../crates/ambition_portal2d/src/types.rs),
[`eviction.rs`](../../../crates/ambition_portal2d/src/eviction.rs).
The [main owner](portal-parts-similarity-and-chirality.md) records the material,
chirality and moving-frame implementation scope.

## The missing relation identity

The same physical relation currently acquires several different keys:

| Current key | Reader or producer | Limit of its meaning |
| --- | --- | --- |
| Hash of `PortalSchema.link` | `PortalLink`, `resolve_portal_links` | Unqualified authored name grouped within a live room |
| `PortalChannel` | Transit, backside, cooldown, host depth, frame history | Reusable channel slot; also carries visual history |
| Index from `links.len()` | `BodyGeometry::add_pair_with_steps` | Address within that publication's links |
| Index from position-sorted ends | `raycast_through_portals` | Independently numbered raycast snapshot |
| Equal aperture values | `add_straddler`, `prepare_body_crossing` | Join by mutable geometry |
| Exit point inside an end's box | Held close-on-use adapter | Inferred identity of the pair used |

Keep authored definition, persistent control/source slot, admitted connection
incarnation, endpoint role, placement revision, snapshot address and look
distinct. A local pair name resolves within its assembly. A live incarnation
includes its source and room lifetime; A/B is an explicit role. Use existing
occurrence vocabulary, not entity allocation or registration order.

Continuous host motion retains the incarnation and varies its pose sample.
A gun refire may retain the control slot and A/B role. Its discontinuous
physical binding must either renew the incarnation or publish an explicit
placement revision. No old straddle, cooldown or history may silently resolve
to a different opening. Retiring then reopening a slot cannot alias the old
physical binding merely because its slot or dense address was reused.

Publish accepted directed adjacency once for its consumers. Keep structural
relations separate from the endpoint pose samples that change each tick.
Pure kernels may use compact indices, but an index is valid only with its
snapshot. Retained paths, outcomes, refusals and views must retain or resolve
their directed occurrence identity. No reader joins links by equal geometry.

When raycasts adopt this snapshot and the connection's map convention, keep
their implemented initial-chart reach: each world segment spends
`world_distance / cumulative_scale`. Preserve deterministic first-hit order,
bounded traversal and an exact exit-face start without a world-unit advance.
The [aperture trace](../../../crates/ambition_platformer2d_core/src/cast/aperture.rs)
and its nearby-target/scaled-reach witnesses own that behavior.

Sources: [`BodyGeometry`](../../../crates/ambition_platformer2d_shared_tangle/src/body/geometry.rs),
[`passages.rs`](../../../crates/ambition_portal2d/src/passages.rs),
[`placement.rs`](../../../crates/ambition_portal2d/src/placement.rs),
[`transit.rs`](../../../crates/ambition_portal2d/src/transit.rs).

### The connection owns its map convention

The [2026-08-08 maintainer decision](../maintainer-decisions.md) requires
authored portal orientation per portal, not a global setting. The November 6
similarity decision did not replace that rule. Bind the map convention in the
prepared connection, with its endpoint geometry. At each sample the reverse
map is the mathematical inverse of the forward map; it is not an independently
selected policy. A game can supply an authored default while each admitted
connection records its resolved choice.

Current `PortalTuning.convention` and `BodyGeometry.convention` apply one
convention globally. Treat that as remaining implementation debt. A global
developer override is an explicit admitted experiment, with normal occupied
edit rules; it must not become the future authoring authority or reinterpret
an already accepted path. Changing convention is a topology/map edit.
Query and view consumers read that connection's resolved map. Delete the
query-global convention as they migrate; adding an authored field alone
cannot make mixed-convention paths correct.

## Edits of topology are one domain operation

Opening, replacement, resizing, release, owner loss and host loss change a
connection. The accepted old state must stay available until the change is
admitted and its material consequences have settled.

The first customer is a Smash recast in place. `lower_held_portal_pairs`
queues old slot ends for despawn, then constructs its overlap list from the
same ECS queries before those commands apply. A fresh slot at the same place
therefore sees the retiring pair as an obstacle and becomes spent. The existing
`a_second_cast_by_one_fighter_retires_its_first_pair` witness moves the body
from x=100 to x=400 before recasting; it does not exercise this case.

The operation must:

1. Collect the source's desired removals, replacements and additions against
   one accepted version. A retiring predecessor is part of its replacement,
   not an unrelated obstacle. Contending sources have deterministic precedence.
2. Validate the complete candidate pair, residence, overlap, finite geometry
   and capacity before changing live endpoints. Distinguish an invalid request
   that preserves the old pair from an accepted game policy that fizzles it.
3. Ask the material owner about occupancy and the consequences of the edit.
   Canonical limbs, attachments and chained paths count. A root's single
   `PortalTransit` latch cannot prove that every directed connection is empty.
4. Retain the old accepted relation, endpoint samples and affected occupancy
   through drain or eviction. Do not delete the ends first and reconstruct the
   closing material from a new pose, reused channel or new map.
5. Publish endpoints, relation and applied result together. Deferred command
   timing cannot expose one end, a stale relation, or a false open result.

Closure policy remains explicit. The current held release may drain existing
occupants; a forced close may use the portal-close eviction authority. A
future lethal close is a different policy, not a side effect of dropping data.
A draining connection stops new admissions while preserving the map needed by
material already through it. If that cannot be represented, refuse the edit
and keep the accepted state. Do not add geometry-repair pushout elsewhere.

A resize is an edit even when it grows an end. Changing only one aperture
changes the ratio and thus the map of existing material. The current held
adapter delays shrink while a root straddles but permits growth. This is not
proof of valid continuous material motion for an arbitrary animated ratio.
The material owner must admit the change or retain the previous map.

### Publication timing and truthful outcomes

The host gives each edit an effective physical instant as well as a publication
boundary. A phase is not a fraction of the tick. `boss_conduct` currently runs
in `WorldPrepSet::AfterIntegrate`; `technique_execution` runs in
`CombatSet::ContentSpecials`. Both consume requests this tick. Consumption does
not make a late request effective in an interval that has already passed.

An edit published after full body integration takes effect at tick-end for
any full-tick trace. An opening exists at that boundary and may be presented
there; an earlier interval cannot traverse it. A source created at tick-end
has its admitted birth pose and starts its lifetime there, with no earlier
travel. An instantaneous effect may query that boundary. A deactivation at
tick-end retains the old relation for completed paths and their later readers.
Edits prepared before motion can govern that motion; no blanket next-tick
delay is required.

A later-scheduled projectile tracing the full tick must use the topology that
was effective during that interval. If a conducted pair must affect an
interval before tick-end, produce its request earlier or explicitly split
physical time and the affected motion. A new phase snapshot alone does not
establish causality. Name the time of each segment and retain its relation
sample; test this through the recovery and conducted-limb clients. Preserve
the port's same-tick consumption and make any changed effect timing explicit.

`held_pairs_of` currently reports `open = !record.spent`. This can report open
when capacity left the ends on dead channel 255, or when the endpoint entities
are gone. Its result must describe admitted state, including pending closure
and refusal. The exact state encoding can remain small. A record's presence
is not proof of an active connection.

Close-on-use consumes an outcome naming the exact directed occurrence. Specify
whether use means root completion or another material event. Do not infer use
from an exit point's overlap with the present endpoint box.

## Lifetime source is separate from physical residence

### Reset keeps its admitted scope

`RoomReplayAdmitted.subject` carries the body's live room. The Ambition
[`reset adapter`](../../../game/ambition_content/src/portal/reset_adapter.rs)
reduces every manual replay to unit `ClearPortals`.
[`clear_portals_on_reset`](../../../crates/ambition_portal2d/src/lifecycle.rs)
then removes all gun portals and all body cooldowns. Alice's room retry can
therefore change Bob's independent room.

Carry the resolved room into portal reset admission. A bodyless sole-room
operation can use an explicit sole-room rule. A whole-session reset has an
explicit whole-session scope. Reuse the pattern already present in
[`reset_gravity_on_room_reset`](../../../crates/ambition_platformer2d_runtime/src/gravity/lifecycle.rs).
Do not infer whole-session authority from an absent subject.

### Generator lifetime names its actual source

[`despawn_orphaned_portals`](../../../crates/ambition_portal2d/src/generator_lifecycle.rs)
collects a global set of generator pair numbers. A same-numbered generator in
another room can keep an orphaned pair alive. The inventory adapter gives a
new holder and a ground gun pair 0; `OwnedPortalPair` stays on the body after
the item leaves. This does not establish that a pair belongs to a gun occurrence.

State whether the source is an item occurrence, an actor capability or an
explicit shared room network. Then name that source and its residence. A
pickup, drop or stow changes custody or permission through that declared
policy. Matching a color number is not ownership. Do not assume the product
requires one unique pair for every physical gun.

### A room transfer cannot carry a hidden old-room connection

[`apply_world_replacement`](../../../crates/ambition_platformer2d_publication/src/transaction.rs)
retains arrival bodies and moves their `InCustodyOf` closure. Held portal ends
use `HeldPortalOf`, not custody; their `HeldPortalPairs` record stays on the
body. The adapter matches owner/module/slot without retaining that lease's room.
If the old room stays live, a continued request can update old-room endpoints
with new-room coordinates. If it retires, the record can retain dead ends,
skip respawn and still report open. A module that stops asking can mask this;
the domain contract cannot require every module to remember the cleanup.

Bind each connection lease to its physical residence. Settle or terminate
source-room bindings before destination use; a destination request creates a
local occurrence. Reuse lifecycle admission and custody hooks, not cleanup in
every travel caller. A refused transfer changes neither leases nor custody.

Compulsory caster/host retirement cannot be handled by refusing an edit while
leaving ownership on a dead entity. Portal-domain closure state, owned by the
source room, must retain the old binding and material facts and may outlive
the caster. Before body/room retirement commits, settle each occupied binding
or give its drain an explicit owner and retained source-room residency. A room
with unresolved material cannot retire. If travel cannot meet this condition,
refuse or hold it before commit; never publish the destination and then try
to repair the source. Forced settlement is an admitted operation, not a later
best-effort cleanup. Retire old data only after its consumers finish.

A draining relation still uses the operational topology budget. Retained
closed samples use a separate bounded query/replay lifetime; freeing a visual
slot does not release material or history that still refers to the binding.

Rollback restores the relation, lease and closure policy together. The current
`HeldPortalPairs` clone restores its fields, but its semantic cursor omits
`phase` and `closes_on_use`. Include behavior-affecting policy in the revised
checksum; this is checksum coverage debt, not missing clone state.

## Authored linking and executable preparation

`PortalSchema.link` passes unchanged through LDtk conversion and is hashed at
spawn. Runtime grouping discovers malformed cardinality. Two reusable
assemblies with the same local pair name in one active area would form one
four-end group. Different live rooms are already separated correctly.

Prepare pair definitions through the portal facet of the existing
[selected-generation linker](capability-linking-and-prepared-facets.md).
Use [lexical assembly keys](reusable-authored-world-composition.md), explicit
A/B roles and typed external endpoint slots. Construction binds occurrences.
Diagnostics retain source, assembly, pair and endpoint. Source-only editing
may expose an incomplete pair; executable preparation must refuse malformed
relations or require an explicitly authored inactive terminal with a reason.
Dynamic pair requests still use runtime admission, not content recompilation.

Color becomes appearance only. Remove `author_portal_pair`'s obsolete
`channel_index >= 8` restriction: the current Smash module supplies that value
as `HeldPortalPair.looks`, while owner/module/slot determines pairing. Retire
legacy color pairing as callers migrate; do not preserve two permanent
mechanics authorities. A finite admission budget remains explicit and
independent of how many colors are available.

Both ends of a continuously interactive portal remain in one physical island.
Several LDtk levels composed into an active area can share it. Two instances
of the same area definition cannot. `GatePortalRegistry` and `GateNetworkState`
control loading-zone travel: they transfer committed body/custody between
rooms. Their shared name and appearance do not justify merging that protocol
with live material transit. See [islands](island-rollback-and-compact-peer-input.md).

Mechanical reload keeps [whole-generation publication](content-generation-and-reload.md).
Never bind retained latches to new dense indices by position. Preserve state
only through an admitted mapping of stable definitions and occurrences.

## Delivery slices and deletion requirements

The main PSIM owner schedules these cuts. The table states implementation
ownership; it does not claim these mechanisms have landed.

| Cut | Owner, writer and installation | Failure guarantee and deletion |
| --- | --- | --- |
| T1: scoped lifetime | Portal domain reset/lease consumer; Ambition reset and item adapters supply scoped intent; lifecycle publication supplies accepted transfer | Alice's retry or transfer leaves Bob's topology intact. Refusal preserves source leases. Delete unit scope loss and global pair-number liveness. |
| T2: occurrence identity | Portal domain binds pair/end identity; portal simulation plugin owns its state and rollback declarations | Reuse cannot retarget an old latch. Delete geometry/exit-position identity joins; channels become a temporary projection. |
| T3: one adjacency snapshot | Portal publisher writes accepted links; host schedules it before consumers | Every query names its snapshot. Delete independent passages/raycast enumeration and repeated partner discovery; retain compact indexed kernel access. |
| T4: topology edit admission | Portal domain consumes held/shot/host/reset edits; material owner supplies occupancy; host declares publication boundaries | Invalid edits preserve accepted material and both ends. Start with held recast and gun refire, then delete direct ordinary close/replace mutations in adapters. |
| T5: authored binding | Portal authored facet links local references and map convention; construction binds occurrences | Malformed executable pairs fail before publication. Delete raw room-wide local-name grouping and obsolete palette restrictions. |

The first production witnesses are recast in place, pair-slot reuse with an
old cooldown, two same-numbered gun pairs in separate rooms, and a held owner
transferring while its source room is retained and while it is retired.
Then exercise two simultaneous limbs, an attached item and a chained third
pair during close or resize. Repeat accepted edits and refusals across rewind.
Capacity refusal must be visible; invalid second-end geometry preserves the
accepted pair. Two pairs with different authored conventions must coexist,
compose and reverse correctly. Views keep the same directed occurrence when
an unrelated pair is added.

Measure query construction, cloning, sorting and partner-search work before
claiming a speed gain. Retaining structural adjacency must not force a full
repartition of every body each tick. Reuse the resource-meter plan's identity
and policy discipline, not its numeric bank: connections are spatial relations,
and their closure semantics remain the portal domain's.
