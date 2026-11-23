# Native action completion and cancellation

**State:** PLANNED, 2026-11-22. Source reviewed at production commit
`f412189796254f280e9f5018b685314b6994060c`, with planning overlays through
`d5b1e9d28e8ec3d19a1f7adf532ac614b8f63a99`. These are source-derived
failures. No new runtime or timing result is claimed.
**Selection:** [ARCHITECTURE-COMPOSITION](../queue.md#architecture-composition--link-owned-facts-and-preserve-causal-boundaries).

## Decision

The native owner reports the result of an admitted operation. Its adapter
must retain the relation between that request and the result. A later query
about an object's existence cannot recover that relation.

Cancellation controls work already admitted. It needs its own request
identity and result, but it must not need a spare slot in the pending-work
pool that the target already occupies.

Use the existing construction receipt, action mailbox and addressed delivery
pass. These two corrections do not need a general transaction service, new
dispatcher or second identity allocator. Construction still provides the
bounded guarantees in [immutable content and construction](immutable-content-and-transactional-construction.md).

## Current evidence

| Boundary | Current source | Lost fact or incorrect dependency |
| --- | --- | --- |
| Ride adapter to summon executor | [`lower_summon_rides`, `answer_ride_summons`](../../../crates/ambition_abilities/src/ride.rs) | The adapter predicts an ID from only its own requests, then treats any matching live entity as success |
| Shared summon allocation and construction | [`apply_summon_effects`](../../../crates/ambition_platformer2d_actor_monolith/src/body/summon.rs) | All producers share the sequence; the executor discards the actual construction receipt |
| Receipt to post-construction initialization | [`ConstructionReceipt::entity`](../../../crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs) and the summon executor | Reservation and birth statuses scan live IDs even though the receipt already names each new entity |
| Portable cancellation to native cancellation | [`cancel_actions`](../../../crates/ambition_platformer2d_runtime/src/action_replies.rs) | A new pending action is reserved before native cancellation can run |
| Action admission | [`ActionMailbox::begin`, `reject`, `record`](../../../crates/ambition_platformer2d_shared_tangle/src/action_results.rs) | The 32-pending bound blocks cancellation; the same journal already supports terminal refusal without a pending slot |

The existing fixtures prove isolated summon outcomes and native cancellation.
They do not prove mixed summon allocation or cancellation through a saturated
portable adapter. Preserve their receipts and add the missing cases.

## AC1 — Complete summons from the construction receipt

### Failure and customer

A body can fire the
[puppy-slug gun](../../../crates/ambition_abilities/src/thrown/puppy_slug_gun.rs)
and ask for a ride through a status method in one tick. The gun queues its
ordinary `SummonRequest` in `PlayerSimulation`. The ride request lowers
later in Combat's `technique_execution`. Neither producer advances the
summoner's counter; the common executor owns that advance.

If the counter is `n`, the ride adapter predicts `n`. The executor assigns
`n` to the slug and `n + 1` to the mount. On the next tick, the ride poll can
report the slug as `ride_summoned`. This is a legal composition derived from
the [installed schedule](../../../crates/ambition_platformer2d_runtime/src/extension_composition.rs),
not a reproduced playtest failure.

The poll also confuses construction with later lifetime: a mount can be built
and then retire before the poll. Absence at that later cut does not mean the
construction failed. The
[ride contract](../../../crates/ambition_combat_port/src/ride.rs) already
separates creation/reservation from boarding.

### Implementation

1. Carry the admitted caller correlation through the native summon request.
   Keep it optional for native callers that need no reply. The executor
   remains the only owner of sequence reservation and allocation order.
2. Retain the `ConstructionReceipt` returned by `ConstructionPlan::commit`.
   After the existing command flush, use `receipt.entity(id)` for mount
   reservation and birth statuses. Capture the new object's result subject
   at this cut, while its exact occurrence is known.
3. Publish terminal success after required initialization and counter
   advance. Report that this request constructed and reserved its mount.
   Boarding, lease expiry and later survival retain their existing owners.
4. Publish the existing failure result on each correlated request affected
   by a supported refusal. Cover missing summoner identity, preflight,
   planning and counter revalidation. Preserve current whole-batch refusal;
   do not quietly change it to partial success.
5. Deliver through the existing mailbox at the next eligible `actor_think`
   cut. Update the port card to state the real production and read cuts.
   Remove the extra tick used only to infer the result.

**Allocation is not body completion.** The receipt supplies an exact root
entity; it does not prove that its recipe populated a usable body. In current
source, [`spawn_runtime_minion_into`](../../../crates/ambition_platformer2d_actor_spawn/src/actor_spawn/mod.rs)
can return early for a limbed host after the root has received its `SimId`.
[`preflight_planned_bodies`](../../../crates/ambition_platformer2d_construction/src/lib.rs)
checks registration and the body blueprint but misses that family refusal.
Move this existing refusal into summon preflight, or carry the constructor's
typed outcome. Refuse before root allocation and counter spending. Completion
requires both the exact receipt and the supported constructor's completed
body/initialization contract. Do not replace a false existence proof with a
false receipt proof or add a general rollback engine for this bounded case.

**Layer constraint:** `ambition_characters` deliberately has no dependency
on `ambition_platformer2d_shared_tangle`. Do not put that crate's ECS-facing
`ActionReply` in `SummonRequest` and restore the edge. Factor a small pure
reply address from the existing SDK `ObjectRef` and `ActionTicket`, or place
the native envelope at the existing integration boundary. Keep ECS lookup
and publication with the action owner. This does not require a new crate.

`ObjectRef` identifies an occurrence; its allocation room is not its current
rollback authority after travel. AC1 covers the current same-tick native
executor. Preserve its source/summoner ownership checks. It does not solve
the [deferred cross-island publisher problem](addressed-behavior-inbox.md#committed-world-boundary-between-islands).

A missing observer cannot invent cancellation. Ride cancellation stays
unsupported. If the observer retires, ordinary mailbox retirement decides
delivery; the completed construction is still a completed construction.
Capture the result subject before later target retirement. Never resolve an
old result against a replacement at the same authored name.

The command flush is not a call to authored reply code. Replies remain
scheduled deliveries. Native construction messages already clear on rewind,
and the source reissues them; the action journal already rewinds. Retain those
boundaries instead of adding a durable pending-summon result table.

### Removal and proof

Delete `RideSummon`, `RideSummons`, their codec and rollback registration,
the adapter's `taken` map and counter prediction, `answer_ride_summons`, and
its scheduled pass. Delete the executor's post-commit identity scans for
reservation and birth statuses. Do not replace those scans with a new
global index when the exact receipt is already available.

Extend [the real summon fixture](../../../game/ambition_app/tests/scripted_summon_outcomes.rs)
with a mixed same-body summon batch. The reply must name the mount, not the
earlier creation. Include an unsupported limbed summon refused before birth,
a refused batch, and retirement after commit but before reply delivery.
Use the existing action retry and populated replay
fixtures to prove one construction and one terminal result per request.
Keep unsupported cancellation as a control.

The source establishes removed work: one next-tick pass, per-request live-ID
searches, and extra rollback state. Count visited entities and snapshot bytes
before claiming a runtime saving. The receipt need not be retained after this
commit; completion does not add a permanent object registry. Existing action
recipient lookup still has its current cost; AC1 removes summon inference
scans, not every lookup in the action system.

### AC1 first slice done (2026-11-22)

`apply_summon_effects` keeps the `ConstructionReceipt` that
`ConstructionPlan::commit` returns, and reads the mount reservation and the
birth statuses from `receipt.entity(id)`. The two post-commit scans of every
live `SimId` are gone.

### AC1 done (2026-11-23)

**Reply.** `SummonRequest` carries an optional `SummonReply`. That is the
recipient's SDK `ObjectRef`, its `ActionTicket`, and the domain's own
completed and failed codes. `ambition_characters` gained an edge to the
dependency-free SDK, not to shared_tangle.

**Answering.** `apply_summon_effects` pairs each reply with the identity it
derives and publishes the terminal result itself through
`publish_action_result_to`:

- **Success:** after initialization and the counter advance, it publishes
  `completed`, with the body the receipt names as the subject.
- **Refusal:** each supported refusal (summoner without identity,
  preflight, planning, counter revalidation) publishes `failed` on every
  correlated request of the batch. Whole-batch refusal is unchanged.

**Limbed hosts.** `preflight_planned_bodies` refuses a `SummonedMinion`
naming a limbed host (`SummonedLimbedHost`) before the root is allocated.
Staged `Enemy` rows keep their recipe-time refusal, because room
transactions flag that unbuilt row.

**Deleted.** `RideSummon`, `RideSummons`, their rollback row
(schema 531 → 532, both baselines re-recorded), the adapter's `taken` map
and counter prediction, `answer_ride_summons` and its scheduled pass.

**Temporary probe.** A copy of `scripted_summon_outcomes` also asked for
`npc_giant_gnu` on step 6. It was then removed. It heard
`6:ride_summoning;6:ride_not_made;`, no giant body existed, and the
summoner's counter went 0 → 1, spent only for the shark. The shark's
`ride_summoned` arrived on the same step as its `ride_summoning`, a tick
earlier than before. The original fixture passes unchanged.

**Not proved here.** The mixed same-body batch (another summon by the
same summoner in one tick), retirement between commit and delivery, and
the replay fixtures. The old adapter's wrong mount for a mixed batch and
its false `ride_summoned` for a limbed root are source-derived; they were
not measured on HEAD.

## AC2 — Cancel without reserving more pending work

### Failure and customer

`CancelAction` has a mandatory reply. Its adapter calls ordinary action
admission before `cancel_action`. At 32 pending actions, admission refuses
with `too_many_pending_actions`, so native cancellation never gets a chance
to release the target's slot.

A concrete supported case is 32 queued object calls from an active caller
to a paused receiver. Use two live rooms in one local/GGRS session authority,
with simulation islands disabled. The
[call owner](../../../crates/ambition_platformer2d_runtime/src/object_behaviors.rs)
admits same-authority calls and gives the receiver 32 slots per binding.
The [inbox](../../../crates/ambition_platformer2d_runtime/src/behavior_inbox.rs)
pauses both delivery and active-time expiry with the receiver's room. The
caller stays active and can request cancellation, but its own full mailbox
refuses every attempt. Native `cancel_call` could stop the queued method.

This is independent of foreign queue pressure in CW2 and of the inbox's
per-tick delivery budget. Raising any of those limits does not fix it.

### Implementation

Factor action identity admission from pending-work admission in the existing
mailbox. Add one synchronous cancellation operation which:

1. checks the request's producer, scope, signature, selector and high watermark;
2. returns an existing retained decision for an identical retry;
3. claims the new cancellation occurrence without adding a pending-work row;
4. asks the existing native cancellation owner to suppress the target;
5. records terminal `Applied` or `Rejected` for the cancellation in the
   existing bounded journal.

Keep the target pending until its owner accepts cancellation. On success,
the target publishes `Canceled` through its own ticket and frees its slot.
On refusal, the target stays unchanged. Do not free the slot first or mark
work canceled before the native owner can suppress its later mutation.

`ActionMailbox::reject` already allocates an occurrence and writes a terminal
row without a pending slot. Share its identity and journal machinery; do not
create a cancellation queue, reserve a hidden extra slot, or temporarily
increase capacity. Pending capacity bounds outstanding native work, not the
ability to control that work.

Preserve the existing callback contract. A cancellation still has its own
result and later-cut callback. A quarantined identical retry can retry
delivery, but cannot run the native cancellation again. Preserve payload
reuse checks, stale request watermarks, module ownership and exact target
occurrence. A retained terminal target refuses as already terminal; an
expired or unknown target refuses as unknown. Neither can create target work.

Keep the existing 64-row journal and its overflow policy, including dropped
unacknowledged rows. This packet does not promise unbounded receipt retention.
If the current adapter exposes a submitted progress row, retain that journal
meaning without retaining a pending operation. Cancellation itself finishes
synchronously inside one native step.

### Removal and proof

Remove `CancelActionPort`'s dependence on ordinary pending-action admission.
Keep the current cancellation policies and addressed callback dispatcher.

Extend [the object-call cancellation fixture](../../../game/ambition_app/tests/scripted_object_behaviors.rs)
through the portable adapter: fill the caller with 32 admitted calls to the
paused same-authority receiver, then cancel one. The target becomes Canceled,
the cancellation becomes Applied, and the other 31 calls remain pending.
Resuming the receiver must not invoke the canceled method. Retry the same
request and then a reused request with another payload. Use the existing
journal/replay tests for bounded history, rollback and authorization refusal.

### AC2 done (2026-11-22)

`ActionMailbox` admits a request as work or as control. Both share the
identity checks (payload reuse, retained retry, stale watermark) and write the
submitted row; only work takes a pending slot and meets the 32-slot bound.
`begin_action_control` and `settle_action_control` are the control path, and
`cancel_actions` uses them instead of ordinary admission, recording its
terminal `Applied`/`Rejected` row directly. The target stays pending until its
native owner accepts and publishes `Canceled`. Witnesses: the mailbox unit
`a_full_mailbox_still_admits_a_control_request` (identical retry finds the
retained result; ordinary work is still refused), and the app fixture
`cancellation_reports_refusals_and_suppresses_native_flag_work_after_replay`,
which now fills the caller's 32 slots (29 fillers beside the three pending
originals) before it cancels, under GGRS sync-test replay. The paused
same-authority receiver case with object calls is not added: the fixture's
full mailbox exercises the same admission path.

## Delivery order and limits

AC1 and AC2 can ship independently. Both retain existing owners and delete a
specific false inference or admission dependency. They need no changes to
portal geometry, world-post confirmation, general object delivery or the
cutscene occurrence work in SS7/SS8.

Update the SDK explanations and the script-first census when each packet is
implemented. The earlier isolated-case receipts remain valid evidence; they
do not prove the composition cases above. This plan does not reopen the
whole script-first program.
