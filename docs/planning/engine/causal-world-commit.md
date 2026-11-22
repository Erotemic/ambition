# Causal world changes across independent simulation histories

**State:** PLANNED architecture correction (2026-11-21). No implementation or
new runtime result is claimed by this review. The source baseline is
6895e516863872eb6d38093a27225ed8e2973d70. [The queue](../queue.md) selects work.

**Owners:** room history and confirmation remain in the selected host. Items,
quests, flags and world values keep their typed mutation owners. This plan owns
the agreement between those owners when a consequence leaves a history that can
still change. It revises the shared-store direction in
[SIMULATION-ISLANDS](island-rollback-and-compact-peer-input.md), and extends the
existing [committed world boundary](addressed-behavior-inbox.md#committed-world-boundary-between-islands).
It does not select a different network transport.

## Decision

A room may predict its own consequences. Another independent history may act
on those consequences only after they are final, or after the two histories
explicitly share a rollback authority.

An undo record can restore a value. It cannot restore the decisions that
another room made after reading that value. Keep this distinction in the
native gameplay road as well as in script ports.

The planned model has three parts:

1. A domain applies local speculative work inside its current rollback scope.
2. A typed domain operation carries a consequence across the confirmation
   boundary. The world owner accepts or refuses it once.
3. The result enters each affected room at a recorded simulation input cut.
   Replaying that room consumes the same result at the same cut.

This uses ordinary domain functions, the existing action-result identity and
the existing world-post admission pattern. It does not require a general
distributed transaction framework or an event-sourced replacement for every
store.

## Why the current shared-store plan is insufficient

The [ISL-2d design](island-rollback-and-compact-peer-input.md) chose immediate
world writes plus retraction. It explains how to undo an append, a keyed
overwrite or one room's delta to a shared count. Those operations solve
duplicate writes on replay. They do not establish causal isolation.

Consider the shared bag:

| Step | Room A | Committed world / room B |
| --- | --- | --- |
| 1 | Predicts a pickup that adds one cell | The shared bag exposes the cell |
| 2 | Has not confirmed that pickup | B spends the cell and heals |
| 3 | Corrected input removes the pickup | A's retractor removes its grant |
| 4 | A replays without the pickup | B's healed body and decision remain |

Subtracting the grant cannot undo B's heal. Remembering which writer set a
flag has the same limit: a different room may already have opened a door,
spawned an enemy or advanced a script because the flag was visible. A
commutative update does not make arbitrary readers commutative.

This is a source-derived counterexample to the plan, not a new measured
runtime failure. The present witnesses mostly prove a writer's own replay.
The first implementation slice must run the dependent-reader case.

### Source evidence

| Existing owner | What it establishes | Limit relevant to this plan |
| --- | --- | --- |
| [OwnedItems](../../../crates/ambition_items/src/lib.rs), [item uses](../../../crates/ambition_platformer2d_items/src/narrative.rs) | One shared bag; native room actions consume it and change a body | A bag delta does not identify all later decisions that used the bag |
| [RoomWrite and retractors](../../../crates/ambition_platformer2d_shared_tangle/src/lifecycle/room_writes.rs) | Room/tick provenance and ordered undo helpers | They track writes, not foreign reads or causal dependents |
| [Hosted room replay](../../../crates/ambition_hosted_world/src/room_rollback.rs) | One room can restore and replay while other rooms are hidden | Hiding another room during replay does not undo its earlier observations |
| [world_posts](../../../crates/ambition_platformer2d_runtime/src/world_posts.rs) | Holds speculative posts, records recipient admission, repeats admission at the original replay tick | This is a useful existing pattern, not yet a complete world-state mutation service |
| [world_values](../../../crates/ambition_platformer2d_runtime/src/world_values.rs) | Holds writes from independent rooms until confirmation | Held rows lack the final reply identity; a later refusal is only logged |
| [ActionMailbox / ActionReply](../../../crates/ambition_platformer2d_shared_tangle/src/action_results.rs) | Scoped action identity, pending work and typed result stages | A held world operation must retain this identity through its final decision |

The current world-value adapter reports terminal **Applied** for
**world_value_held**. Its actual compare-and-set can later fail at confirmation.
Keep the useful confirmation mechanism; change the outcome contract so a
caller can distinguish pending acceptance from a performed world change.

## The three state spaces

| State | Owner | What a room rewind may change |
| --- | --- | --- |
| Local speculative state | The room's rollback authority | Its body state, local predictions and unconfirmed outgoing proposals |
| Committed world state | The typed world/domain authority | Nothing already committed by that authority |
| Admitted inputs to a room | The world-to-room admission road | The room reconsumes the same admitted inputs; it cannot erase their source decisions |

An outgoing proposal is not a second copy of canonical world state. It is
bounded pending work. If a game needs immediate local feedback, the owner can
derive a private presentation preview from committed state plus that room's
proposals.

Input confirmation does not mean the world owner accepted a spend or a
compare-and-set. The first implementation applies the dependent local
mechanical consequence only after that acceptance returns. Otherwise A could
predict a heal, make further decisions, then receive a refused spend after
its input is final. Removing the heal would not repair those decisions.
A committed reservation can authorize immediate local mechanics. Predicting
an unreserved shared operation would require keeping its entire dependent
history provisional; that larger protocol is not selected here.

A replay must not read today's mutable committed world as if it were the
world seen on the original tick. Use recorded revisions, a retained read view,
or explicit admitted deltas, as appropriate to that domain. State queries,
notifications and action receipts must agree about the selected read cut.

Do not build one global history database to enforce this rule. Start with the
world-value and item owners and the existing per-tick admission log.

### One history remains a simpler case

In local play or one bounded GGRS history, all affected authoritative state
and observers can rewind together. A domain can keep same-tick application
inside that history. External IO still follows its existing confirmation rule.

The distinction is the rollback authority, not the number of rooms, whether a
player is human, or whether a request came from Rust, WASM or Rhai.

### Independently rewindable rooms

An operation that leaves one room follows this transition:

| Stage | Required fact |
| --- | --- |
| Propose | Capture the typed intent, issuer occurrence, source history/epoch, tick and operation identity |
| Hold | Keep it private to the source history; source rewind removes or replaces that speculative proposal |
| Confirm source | The host states that the source tick cannot be revised |
| Admit at world owner | Validate current domain preconditions; assign the resulting world revision or a typed refusal |
| Deliver | Record each destination's input cut, result and stable recipient |
| Replay destination | Readmit the same result at that cut without executing the world mutation again |
| Retire | Drop pending proposals and retained input history only when no supported replay or retry can need them |

Identity must survive source retirement and repeated delivery. Reuse the
existing action ticket/occurrence contract where it supplies that meaning.
Add the session or history epoch where a tick or room number can be reused.
Do not make a Bevy Entity, a pointer address, arrival order, or a source-code
name the operation identity.

Within the retained decision horizon, a duplicate with the same identity and
payload returns the prior decision. After that horizon, refuse stale work
without applying it again. Keep the owner's retirement cutoff or equivalent
watermark so dropping a receipt cannot make an old request new.

A conflicting payload under a committed identity, or in a duplicate
submission on the same timeline, is refused. A source rewind can replace an
uncommitted proposal; its corrected payload is not that conflict. The domain
owns ordering among competing operations; identity alone does not define
gameplay priority.

## Resource ownership is part of the solution

Do not route every local action through the world authority. A body's wallet,
mana, health and ordinary body-owned inventory can remain in that body's
rollback scope.

For a genuinely shared consumable pool, choose one of two concrete domain
contracts:

- **Committed spend.** The shared owner accepts the spend and reports the
  result before another history can rely on it.
- **Reserved allocation.** The shared owner allocates a bounded amount to one
  room/body. That room may spend the allocation locally. Transfer, return and
  retirement of the allocation are committed operations.

A reservation is a real custody fact. It has one owner, a stable identity and
a return/cancellation rule. It is not a mutable copy of the whole bag. Do not
add reservations to all items in advance; use them if measured interaction
latency makes the first contract insufficient.

Preserve the item owner's atomic-use meaning. Accepting a shared spend is not
proof that a later heal or launch succeeded: the target may have died,
departed or changed before its admitted input cut. If the action requires
spend and benefit together, transfer a committed reservation first. The local
owner then debits that allocation and applies the benefit in one accepted
local transaction. A refused local action leaves the allocation unused; its
return or custody handoff is itself committed. Speculative local consumption
can rewind without refunding the world's still-valid allocation.

Use consume-on-world-accept instead only for an action whose domain explicitly
defines that outcome. Do not silently change an existing atomic use into
paid-but-refused work, or call reservation acceptance the completed use.

For a grant from A that B can use, B sees it only after the grant commits.
For a quest or gate result, the receiving room sees the committed revision
at its next admitted cut. This makes responsiveness a visible gameplay
choice instead of an accidental cross-history dependency.

If immediate mutual interaction is essential, place those participants in
one causal domain. Keep the current deterministic refusal for unsupported
cross-island live portals. This plan does not add automatic history merging
or cascading rollback.

## Action results and observation

Use the existing stages honestly:

- **Submitted**: the adapter accepted the request envelope.
- **Admitted**, nonterminal: a durable pending operation exists.
- **Applied**: the owning domain performed the specified mutation.
- **Rejected / Canceled / Failed**: a named terminal outcome under the owner's
  existing lifetime contract.
- **Completed**: use only where the action has a later completion meaning.

For a held world-value write, retain its reply identity and report Admitted.
At world admission, report Applied or the actual refusal, including a changed
compare-and-set revision. A warning is useful diagnostics; it is not the
requester's result.

A source object or entry can disappear after it submits an operation. The
operation retains its identity and data; the domain decides whether a living
subject remains a precondition at admission. This differs from abandoning an
unconfirmed source history: a reset or retired history epoch discards its
unconfirmed proposals. Confirmed operations follow the world's decision
contract, not the continued existence of the source's mailbox.

ActionMailbox owns delivery history, not durable world deduplication. The
world/domain owner retains the decision or retirement watermark independently.
Delivery to a retired mailbox cannot cancel a committed mutation or allow a
duplicate to execute again. A traveling recipient is resolved by occurrence
under the admitted scope, not by an old Entity handle.

Notifications wake behavior. The typed committed state and retained outcome
remain the recovery authority. Preserve bounds and refusal semantics when a
mailbox is full; do not silently claim that all recipients observed a change.

## Implementation packets

These packets revise the existing networking and domain work. They are not
a parallel queue or a prerequisite for unrelated local gameplay.

### CW1 — Prove the dependent-reader case and complete held outcomes

Use the existing two-room host fixture. A predicts a shared grant; B attempts
to consume it or act on a flag; A's late input removes the cause. Record
whether B could use the provisional state and which owner allowed it.

In parallel, carry ActionReply or the equivalent stable operation identity in
held world-value work. Preserve a nonterminal result until actual admission.
Exercise two held compare-and-set operations against one revision: one can
apply and the other must receive its real refusal.

The fixture attacks the authority crossing. Do not add a source-text rule
about which functions may mention RoomWrite.

**Exit:** a reader cannot mistake held acceptance for an applied mutation;
the two-room causal case is an explicit failing baseline or a demonstrated
existing refusal, not an assumed result.

**CW1 record (2026-11-21).**

*The dependent-reader case is an explicit failing baseline.* The probe used
the two-room fixture of `a_room_rewinds_alone.rs` (Alice in
`basement_breakables`, Bob in the hub) with an empty bag. Alice's conversation
grants a health cell in a window her room later rewinds. Bob spends it, and
his hp goes from 1 to 2. Alice's room then rewinds with the retractors and
replays without the grant. Results:

| Point | Bag | Bob's hp |
| --- | --- | --- |
| At N | 0 | 1 |
| After Alice's grant | 1 | 1 |
| After Bob's use | 0 | 2 |
| After Alice's replay | 0 | 2 |

The item owner (`OwnedItems`, through `apply_item_uses`) let Bob spend a
grant that a later input removed. The grant's inverse could not take back a
cell that was gone. So Bob kept a heal paid for by a cell that never existed,
as the table above predicts. CW2 replaces this road. The probe was deleted
after this measurement.

*World values cannot be read before they are final.* An island's write is
held, and no reader sees it until its tick is confirmed (the existing
`a_world_count_from_a_rewinding_room_applies_once_when_confirmed`). The
dependent-reader defect is the shared stores that write eagerly: the bag,
flags and quest steps.

*A held write is not reported as applied.* `world_values` now answers a held
write `Admitted`, which is not terminal, and keeps the writer's
`ActionReply` with it. When the tick is confirmed, the world applies the
write and tells the writer, under the same ticket, `Applied` or the real
refusal. A replay of the writer's room repeats that decision at the tick it
was first told, into the mailbox its rewind restored
(`HeldWorldWrites::decided`, kept for the rewind horizon). The island witness
now holds three writes in one window: a count and two compare-and-sets
against revision 0. Each writer hears `admitted` and no outcome while held.
At confirmation the count reports `world_count_added`, the first set reports
`world_value_set`, and the second reports `revision_changed`. The value is the
first set's, at revision 1.

### CW2 — One native shared-item operation uses the committed road

Choose one cell grant/spend path in
[item custody/accounting](item-custody-and-accounting.md). The item owner
validates and applies the operation; the host supplies confirmation. Native
and portable callers enter the same owner.

Keep pending presentation explicit. A dependent heal, launch or other local
mechanical result waits for accepted admission, unless a prior committed
allocation already authorizes it. Rewind replaces only the source proposal.
Another room receives only the committed decision.
If the spend requires reservation, specify the allocation before changing its
local consequence.

For the existing atomic cell-use customer, use a one-item committed
allocation and perform its debit plus heal at the destination's accepted
local cut. Exercise a target that becomes invalid before that cut. It must
not lose a cell for a refused heal; the unused allocation remains owned or
returns through the explicit committed retirement path.

Delete the selected path's shared-world inverse retractor only after no
caller uses the old eager write. Keep retraction for private pending work and
room-owned reversible data where it is still correct.

**Exit:** dependent grant/spend, two competing spends, source retirement,
source replay and destination replay agree with one canonical item decision.
No quantity is minted to repair an already exposed speculative grant.

**CW2 record (2026-11-21).** The cell grant (`<<give_item>>`,
`apply_item_grants`) and the cell use (Health and Mana Cells,
`apply_item_uses`) take the committed road when the body is in a room that
rewinds alone. The owner is `ambition_platformer2d_items::committed_bag`.

- **Proposal.** The applier proposes `Grant` or `Use { body: SimId }` under
  the body's room and tick. A rewind of that room drops the proposals it made
  after its save (retractor `resource.held_bag_operations`). Its replay
  proposes again what it does again.
- **Decision.** On the live tick that confirms a proposal's tick, the bag
  decides it once. Order is by tick, then by room. A grant enters the bag. A
  use takes one cell from the committed bag, or is refused `bag_empty`. The
  bag reads no room's speculative state to decide.
- **Allocation and cut.** An allocated cell is spent in the proposing room on
  that tick: the effect together with the cell. If the cell does nothing for
  the body there (gone from the room, no health, no Mana), the room proposes
  `Return`, which commits like any proposal. A replay of the room that runs
  the cut spends the allocation again by the same rule, and takes no second
  cell.
- **One authority.** With one rollback authority (local play, GGRS) both
  appliers keep their same-tick write.
- **Visible latency.** A held use shows the cell in the bag until the bag
  decides it, which is the input confirmation latency of the slowest guest.
  This is the plan's visible responsiveness choice. Pending presentation (a
  dimmed cell) is not built.

Deleted: the grant and use writes into `BagUsesInRooms`, and with them the
retraction tests (`a_rewound_room_puts_back_the_cell_its_replay_does_not_use`,
`a_rewound_room_takes_back_the_grant_its_conversation_gave` and their ignored
controls). Those tests modelled an island rewind without the island policy.
A merchant's deal still writes `BagUsesInRooms` and keeps its retractor:
the deal's wallet side is in the room's snapshot, and moving it to the
committed road is a later packet.

Witness: `a_room_rewinds_alone::the_shared_bag_decides_each_cell_once_its_tick_is_confirmed`.
It runs on a hosted host that rewinds guest rooms, with Alice in the hub and
Bob in the lab:

| Case | What happens | Result |
| --- | --- | --- |
| CW1's dependent case | Alice's grant, then Bob's use, while Alice's input is late; her late input rewinds her room | Bag 0; Bob's hp stays 1; decision `bag_empty`; no mint |
| Competing spends | A confirmed grant (bag 1), then uses by Alice and Bob on one tick | `granted, allocated, bag_empty`; one heal |
| Refused heal | Bob's health is removed before his cut | `allocated, returned`; bag back to 1 |
| Destination replay | Bob's use is decided while his room is predicted; his late input rewinds the room across the cut | Healed again in the replay; one cell spent; no second decision |

No script port uses an item, so the native appliers are the only callers.
**Not yet exercised:** source retirement (the proposing room unloads before
confirmation). A proposal is keyed by room and tick and is decided by
confirmation alone, so it commits without its room. No run has shown it.

### CW3 — Route world consequences through their actual owners

Start with world-value queries and the gate readers that consume them.
Record the committed revision or admitted delta at each affected room's
input cut. Rewind a room across that cut while another room advances the
world value again: replay must see the original sequence, not the latest
global value. The state query, notification and final action result must
describe the same admitted revision.

Migrate flags and quest transitions, encounter/boss world rewards, occurrence
disposition and the relevant save projections one invariant at a time.
Do not turn these into untyped flag writes. Each owner states its
preconditions, conflict rule, outcome, destination cut and retention horizon.

A single domain operation can coordinate inseparable facts, such as item
custody and its grant. It does not make the generic extension host a
transaction coordinator. Reuse existing accepted-action and construction
boundaries for their own operations.

Remove the selected shared-store undo protocols as their producers move.
The domain census becomes a migration aid and is retired when complete.

**Exit:** no supported independent-room operation exposes a reversible world
cause to an independent reader. Save/checkpoint creation pins committed world
facts; speculative body state is handled by its declared checkpoint policy.

### CW4 — Replace the global confirmation floor only when useful

The current host publishes one conservative confirmation boundary from its
guest-input histories. That can preserve safety while delaying a ready room
behind another guest. Keep it until the semantic crossing works.

Then give the confirmation query the source authority/epoch. A room may commit
when its own history is final. Keep committed-world ordering explicit when
two rooms offer competing operations. Measure actual commit latency and
retained history; do not claim independent progress from a per-room label.

This packet is an optimization of an established contract. It must not
weaken confirmation to make a queue appear faster.

## Integration and deletions

| Existing plan | Revision |
| --- | --- |
| [Island rollback](island-rollback-and-compact-peer-input.md) | Shared-store inversion is bounded migration code, not the final cross-room protocol |
| [Hosted experiment](hosted-world-networking-experiment.md) | Add the dependent-reader witness before broad gameplay parity claims |
| [Addressed behavior](addressed-behavior-inbox.md) | Reuse confirmed source and recorded destination admission for native domain consequences |
| [Extension contracts](extension-domain-contracts.md) | Held acceptance stays nonterminal; source and destination read cuts are explicit |
| [Construction](construction-and-reconstitution.md) | Candidate publication and a cross-history gameplay commit remain distinct protocols |
| [Capability linking](capability-linking-and-prepared-facets.md) | A selected owner supplies its scope and mutation contract; metadata does not make it causally safe |

Expected removals are native shared-store inverse logs and caller-side repair
for the migrated operations, duplicate native/script consequence semantics,
and terminal-success reports for held work. Local rollback logs, existing
domain state and the successful world-post road remain where they serve their
actual semantics.

No compile-time or runtime speedup is claimed here. The benefit is that a new
shared mechanic has one explicit causal contract instead of an expanding list
of stores whose undo handlers must guess what other rooms observed.

