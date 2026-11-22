# Causal world changes across independent simulation histories

**State:** CW1 and the ordinary-path CW2 pilot are implemented (`87c2170e`,
`317287c2`, 2026-11-21). CW2 completion remains open: the source review at
`3ed50e318392` found admission-pressure paths that still mutate the shared bag
eagerly. The follow-up below precedes expansion to other world owners. CW3/CW4
remain PLANNED. The historical receipts are retained; this review ran no new
runtime witness. [The queue](../queue.md) selects work.

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

## Why the original shared-store plan was insufficient

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

This was a source-derived counterexample at the initial review baseline,
`6895e516863872eb6d38093a27225ed8e2973d70`. CW1 subsequently ran the
dependent-reader case and recorded the failure below. CW2 changed the ordinary
cell path; it did not complete every admission and retirement case.

### Initial source evidence and subsequent corrections

| Existing owner | What it establishes | Limit relevant to this plan |
| --- | --- | --- |
| [OwnedItems](../../../crates/ambition_items/src/lib.rs), [item uses](../../../crates/ambition_platformer2d_items/src/narrative.rs) | One shared bag; native room actions consume it and change a body | A bag delta does not identify all later decisions that used the bag |
| [RoomWrite and retractors](../../../crates/ambition_platformer2d_shared_tangle/src/lifecycle/room_writes.rs) | Room/tick provenance and ordered undo helpers | They track writes, not foreign reads or causal dependents |
| [Hosted room replay](../../../crates/ambition_hosted_world/src/room_rollback.rs) | One room can restore and replay while other rooms are hidden | Hiding another room during replay does not undo its earlier observations |
| [world_posts](../../../crates/ambition_platformer2d_runtime/src/world_posts.rs) | Holds speculative posts, records recipient admission, repeats admission at the original replay tick | This is a useful existing pattern, not yet a complete world-state mutation service |
| [world_values](../../../crates/ambition_platformer2d_runtime/src/world_values.rs) | Holds writes from independent rooms until confirmation | At the initial baseline, held rows lacked final reply identity and later refusals were only logged; CW1 corrected this |
| [ActionMailbox / ActionReply](../../../crates/ambition_platformer2d_shared_tangle/src/action_results.rs) | Scoped action identity, pending work and typed result stages | A held world operation must retain this identity through its final decision |

The initial world-value adapter reported terminal **Applied** for
**world_value_held**, although its compare-and-set could later fail at
confirmation. CW1 now preserves the useful confirmation mechanism and reports
nonterminal **Admitted**, followed by the actual decision under the same ticket.

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

### CW2 follow-up — preserve authority under pressure

**State: IMPLEMENTED for admission, credits and settlement (2026-11-21);
source and recipient retirement are not yet exercised.** The record is the
"Completion receipt" at the end of this section. The plan as written follows.

The receipt
above establishes the cases it names. Source inspection at `3ed50e318392`
found the following definite branches; the pressure/replay witnesses below
have not been run by this review.

| Current source | Admission-pressure behavior |
| --- | --- |
| [committed_bag::propose](../../../crates/ambition_platformer2d_items/src/committed_bag.rs) | Returns `false` when 64 proposals are retained |
| [apply_item_grants / apply_item_uses](../../../crates/ambition_platformer2d_items/src/narrative.rs) | Continue only after a successful proposal; refusal falls through to immediate bag mutation and, for use, the local effect |
| [committed_bag::spend](../../../crates/ambition_platformer2d_items/src/committed_bag.rs) | A refused `Return` proposal falls through to immediate `put_back`, even for a room that rewinds alone |

The grant/use inverse-log writes were deleted by CW2. These fallback mutations
therefore neither wait for confirmation nor acquire the old retraction entry.
This repeats the dependent-reader problem under pressure; it is not a reason
to restore shared-world undo. A full queue cannot change rollback authority.

#### Classify the authority, then admit within it

The item owner selects the execution road from explicit source authority:
one shared rollback history, an independently rewinding history, or refusal
because required authority/address evidence is unavailable. Admission failure
cannot select another road. In particular, a missing room-operation identity,
target occurrence or required tick does not authorize the immediate path.

Use a typed domain outcome that distinguishes immediate application, held
admission and hard refusal, instead of a boolean that means both “not held”
and “safe to apply now.” The existing
[world-value admission](../../../crates/ambition_platformer2d_runtime/src/world_values.rs)
already classifies authority before returning `world_write_queue_full` or
`writer_without_authority`; use that separation without copying its payloads.

For a new grant/use that exceeds admission capacity, the selected policy is
hard refusal with no shared mutation or local mechanical effect. A caller
may submit a later new attempt under its domain policy. Do not silently run
the request eagerly or leave a consumed input looking like applied work.
Menu/participant adapters resolve their subject and report the outcome; the
same item-owner operation must remain usable by native body/object callers.
The single-history path keeps its existing same-tick behavior.

This is a deliberate behavior correction for overflow and missing authority.
A narrative grant or other caller must receive the refusal; it must not
record successful fulfillment merely because it emitted the request. Reuse
the existing domain/action outcome road where it supplies that meaning.

#### Admission capacity belongs to the replay contract

Hard refusal alone is not the complete model. If a replay rechecks today's
shared proposal queue, another room's pending work can change Held to Refused
for the same source inputs. The source can branch on that result. This is an
unrecorded cross-history observation even though refusal changed no bag value.
It is a design consequence of the replay contract, not a newly run failure.

Use a bounded proposal budget owned by the source history for this packet.
A proposal consumes a credit; its recorded decision or acknowledgement cut
returns the credit. These transitions restore with the source history.
Bound aggregate storage when admitting source/history epochs. Do not infer
the replayed credit balance by counting today's globally retained rows, even
if filtered to one room: an earlier proposal may now be committed and gone
while its acknowledgement had not yet arrived at the replayed tick.

The confirmed world owner can still refuse an allocation because its own
settlement capacity is full. It records that decision once and delivers it
at the source/recipient cut; no debit precedes reservation of that state.
This is different from source-side proposal admission. The same distinction
applies to world-value admission when its queue refusal is visible to replayed
behavior. Its existing typed authority classification is useful, but is not
proof that a shared pending-count read is a replay-stable budget.

An alternative is a host-owned admission decision retained as an explicit
external input. That policy may keep a refusal even after the foreign proposal
that occupied capacity is withdrawn. This packet selects source-owned credits
instead; do not introduce that stronger host-service policy implicitly.
Keep the accounting with the existing histories and typed domain operations.
This does not require a general distributed flow-control service.

#### An allocation owns value until its disposition is settled

A committed debit creates an identified item allocation: the originating
operation, exact recipient occurrence, admitted local cut and outstanding
quantity. Reuse stable occurrence identity, not the current `SimId` spelling;
the bag decides what that address may receive. Changing address representation
does not make an allocation follow a migrating body into another history.
Keep the declared destination-cut policy explicit.

Several uses can occur in one room on one tick. Room/tick provenance is not
their operation identity. A return names the allocation it discharges; it is
not an unqualified `Return { item }` that can create value independently.
The item owner retains one canonical debit/disposition record for the replay
and duplicate-delivery horizon, with the existing stale-work cutoff rule.

The allocation's debit is committed; its local use/return choice can still be
provisional until the recipient cut confirms. Replay repeats the local
effect or proposed return from that allocation without another debit. At
committed settlement it is consumed or returned once, never both. Returning
value changes the shared bag only at that settlement boundary.

Record that provisional disposition in the recipient's history at the cut.
It rewinds and recomputes with that history; the world debit does not.
Confirmation settles the recorded disposition. It must not re-evaluate
eligibility against the body's newer state. A body that loses its health or
Mana capability after accepting a cell has not made that earlier use unused.
A terminal disposition cannot reopen on repeated delivery. Recipient replay
still reapplies the authorized local result at its original cut without a
second world debit; a global consumed marker must not skip that local replay.

Unlike a new request, an already issued allocation cannot be discarded when
a return queue is full. Its owner retains the pending return obligation
(or reserves that obligation's capacity when allocating). Pressure must
neither refund the bag eagerly nor lose the cell. Do not infer terminal
consumption merely from having emitted a heal request; retain the current
domain rule for whether that local effect was accepted.

Reserve the bounded state needed to settle an allocation before debiting the
bag. If that capacity is unavailable, refuse the new allocation without a
debit. Outstanding value does not expire with old replay receipts. Only a
settled allocation can retire after its supported replay/duplicate horizon;
keep the stale-work cutoff so an old return cannot become a new grant after
its detailed receipt is gone. This bounds admission without dropping an
existing custody obligation.

This is item custody/accounting, with typed outcomes and a concrete conserved
quantity. It does not install a generic transaction coordinator in the host
or turn health, Mana and all world values into one mutation protocol.

#### Completion witnesses and retirement

- Fill the source's declared proposal capacity (the current shared bound is
  64), then request a grant: the extra request is refused without exposing a
  cell. Rewind the source and let
  another room attempt a use; no speculative grant can fund its effect.
- Under the same pressure, request use of a committed cell: hard refusal
  leaves both bag and body unchanged. Missing required identity follows
  refusal, not eager execution.
- Allocate a cell, then replay its recipient cut into an ineligible body
  while ordinary proposal capacity is full. The outstanding allocation
  settles through its reserved return obligation. No eager refund is visible
  to other rooms.
- Replay again across use/refusal and settle the cut: one original debit can
  fund one accepted effect or one committed return. It cannot fund both,
  return twice, or debit twice. Preserve the recorded ordinary CW2 cases.
- Replay the same source inputs and admitted decision cuts after another
  room fills or drains its pending queue. Admission outcomes and source
  credits remain the same. Also cross the original acknowledgement cut after
  the global row has retired: the source gets no early or duplicate credit.

In the current implementation's counterexample, recipient replay need not
free a proposal slot: the first run may have used the allocation without
proposing a return, and retained proposals can belong to other rooms. Live
confirmation instead removes a use proposal before it can produce a return.
The selected design avoids this competition: foreign pending work does not
spend source-owned credits, and an issued allocation retains settlement state.

Source retirement remains unresolved by the existing receipt. Specify and
exercise a valid proposal whose source room retires before confirmation,
alongside recipient retirement/replacement before its cut. Source retirement
does not itself erase a confirmed obligation; an old recipient address cannot
redirect value to its replacement. Refuse a newly unclassifiable request
without conflating it with an already admitted proposal.
Room retirement must supply a final disposition or explicitly transfer the
obligation and the authority that can settle it. Merely keeping the record
does not supply missing confirmation evidence, and disappearance is not
proof of a confirmed unused cell.

**Exit:** authority determines execution; admission only accepts or refuses
within that authority; allocation conservation holds under pressure, replay
and retirement. Delete the eager-on-refusal fallthroughs and replace anonymous
return minting with the item owner's identified settlement. Retain CW1/CW2's
historical receipts and add the completion evidence beside them.

#### Completion receipt (2026-11-21)

[committed_bag](../../../crates/ambition_platformer2d_items/src/committed_bag.rs)
now classifies the authority before admitting. `admit_grant` and
`admit_use` return a typed `BagAdmission`:
- `Immediate`: one rollback authority, so the caller applies it this tick.
- `Held`: a room that rewinds alone.
- `Refused`: one of `no_authority`, `no_identity`, `no_tick` or
  `credits_spent`.

[narrative](../../../crates/ambition_platformer2d_items/src/narrative.rs)
logs a refusal and applies nothing. The eager-on-refusal fallthroughs are
deleted. Neither caller has an outcome road back to its author yet: a refused
narrative grant is a warning, not a reported result.

**Credits.** Each room has `PROPOSAL_CREDITS_PER_ROOM` (16) proposal
credits. A proposal takes one when made and returns it at its decision's
tick. `HeldBagOperations::credits(room, tick)` counts only that room's own
proposals made by the tick and not decided by it. Decisions are retained for
the rewind horizon, so a replayed tick sees the balance it saw live. Another
room's queue cannot change it.

**Identity.** A proposal is identified by its room, tick and order among that
room's proposals on the tick.

**Allocations.** `BagOperation::Return` is gone. A use's allocation reserves
its own settlement row (`MAX_ALLOCATIONS` = 64) before the debit. When no row
is free it is refused with `settlement_full` and nothing is debited. The
body's room records a disposition, used or unused, at the cut. A replay of
the cut recomputes the disposition without another debit. When the cut is
confirmed the recorded disposition settles once: `consumed`, or `returned`
to the bag. Nothing is refunded earlier, and an open allocation never
expires.

The hosted witness
`a_room_rewinds_alone::the_shared_bag_decides_each_cell_once_its_tick_is_confirmed`
keeps its four CW2 cases and adds three:
1. Alice's predicted room spends its 16 credits. The 17th grant is refused,
   neither held nor given, and Bob's room keeps all its credits. Her rewind
   takes the grants back, and Bob's use finds no cell.
2. With Bob's credits spent, his use of a committed cell is refused, with
   the bag and his health unchanged.
3. A Mana Cell is allocated to Bob, who holds no Mana. It stays out of the
   bag while its cut is unconfirmed and his credits are spent. His rewind
   over the cut disposes of it again, and it settles `returned` exactly once.

Not exercised:
- Source-room or recipient retirement before confirmation. A retired room's
  proposals are still decided; an allocation whose body is gone settles
  `returned`.
- Whether a live-room ordinal can be reused, which would let an old address
  name a new body.
- Recipient identity is still `SimId` plus room, not a stable occurrence.
- Admission replay is checked through `credits()`, not through a replayed
  admission (the witness's requests are injected messages a replay does not
  repeat).
- Crossing an acknowledgement cut after the decision row has retired.

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

