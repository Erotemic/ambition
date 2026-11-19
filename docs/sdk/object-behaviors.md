# Reusable object behaviors

A behavior binding attaches declared methods, parameters and private records to
one live object. Several instances can use the same code and schema on that
object. Each instance has its own records, action mailbox and call queue. The
native subject supplies observations, time pace and domain actions. A wallet
action still changes that subject's wallet through the inventory owner.

The current ports run in `actor_think`. They share portable Rust, Rhai and
linked Rust contracts. A loaded Rhai witness runs through the assembled engine
and GGRS. The new behavior ports do not yet have an actual WASM behavior witness.

## Bind an instance

A method entry declares `object_method` as its trigger:

```rhai
#{key: "use", phase: "actor_think",
  trigger: #{port: "object_method", selector: "use"},
  reads: ["behavior"], writes: ["memory"],
  requests: ["inventory_exchange", "complete_object_call"], limit: 1}
```

The caller uses an [object reference](object-references.md) as the target:

```rhai
this.requests.push(request("attach_behaviors", #{
    reply: #{request: 1, selector: "example::factory/attached"},
    target: target,
    bindings: [#{key: "shop", params: #{price: 3},
        methods: [#{name: "buy", entry: "example::shop/use"}]}]
}));
```

Keys are unique on one target. An attachment batch validates all methods,
parameters and initial state before it creates any instance. State seeds use
`#{schema: "memory", value: ...}`. The schema must be a private body-attached
schema declared by a method. A full `provider::key@revision` disambiguates equal
local names. Seed values must contain all fields. Unknown fields are refused.

Parameters are bounded data. `this.observe.behavior` supplies `object`, `binding`
and `params` to a method and its action callbacks. Parameters do not change
when the method changes its records. Normal schema reload rules apply to each
instance's `BodyRecords`.

Attachment is a separate action on an existing object. Creation-time behavior
attachment through the actor and prop construction transaction remains open.

## Call and complete a method

```rhai
this.requests.push(request("call_object", #{
    reply: #{request: 2, selector: "example::factory/called"},
    target: target, binding: "shop", method: "buy", data: #{count: 1}
}));
```

Admission requires a live target in the caller's session, one binding, an
available method and queue capacity. Missing or ambiguous targets and bindings
produce terminal refusal. A method call is communication. It does not grant a
physical interaction, a remote inventory transfer or authority to bypass a
native domain's rules. The native domain still checks its action.

An admitted call enters a later read cut. Its trigger contains `sequence`,
`caller`, `method` and `data`. The sequence is exact unsigned decimal text in
Rhai. It is local to the receiving instance. A method can retain it in a `u64`
field while it waits for a native action result.

Successful handler delivery commits private state and requests. It does not
complete the call. The handler, or an action callback in its module, completes
it explicitly:

```rhai
this.requests.push(request("complete_object_call", #{
    reply: #{request: 3, selector: "example::shop/completion_ack"},
    sequence: this.state.memory.pending,
    accepted: this.trigger.stage == "applied", code: this.trigger.code
}));
```

The original caller receives `completed` or `rejected`. The completion action
also receives its own applied or rejected reply. It refuses an unknown,
terminal, undelivered or foreign-module call. Code text must be nonempty and at
most 128 bytes. General typed return data remains open.

A behavior must manage concurrent work in its private records. It can reject a
second call as busy, or keep a bounded collection of pending call sequences.
Do not overwrite the first sequence while its native action is outstanding.

## Delivery, cancellation and retirement

Method delivery uses the same `DeliveryProgress` as action-result delivery.
Only successful host commit acknowledges a handler. A fault discards staged
state and requests. Bounded retries end in an explicit `object_handler_fault`
result. An unavailable method waits for repair until the call's active wait
budget expires. Paused rooms do not consume that budget.

Cancellation suppresses a queued method before handler commit. It is refused
after commit, including while the method waits for another native result.
A canceled call cannot later invoke its method. A timeout is a failed call; it
does not undo an already committed handler or cancel that handler's native work.

Instances retain the subject's exact occurrence reference across travel. A room-owned instance uses the room's allocation counter. A session-owned
instance uses the session counter. Allocation does not change during travel. Retirement of an old room cannot delete
a traveling subject's instance. Subject retirement fails outstanding calls and
retires the private instance at the next collection cut. Caller retirement or
removal of its pending ticket retires the queued request. Rollback restores
instance state, occurrence allocation, queues and acknowledgements together.
`ObjectScopeOwner` is the single owner link. Room snapshots and visibility cuts
include private state through that link, including after owner reconstruction.

## Game-authored bounds

The `object_behavior_rules` room schema supplies bounds for instances per
object and session, methods, initial records and bytes, parameter and call data,
pending calls per binding, and active wait ticks. `action_delivery_rules`
supplies attempt and per-tick delivery budgets. Defaults permit 16 instances
per object, 1024 per session, 32 pending calls per binding and 600 active wait
ticks. A game can reduce these bounds or disable an allowance with zero.

Call collection builds one occurrence index per read cut. It does not scan the
world for each pending call. Its per-tick queue and policy reads use cached
native queries. This does not establish the full workload cost; a focused
crowded-object measurement remains required.

The current witness is
[`scripted_object_behaviors.rs`](../../game/ambition_app/tests/scripted_object_behaviors.rs).
It checks private instances on controlled and autonomous bodies, actual native
wallet application and refusal, busy refusal, fault exhaustion without a staged
reward, cancellation, GGRS, and private-state reconstruction through room
snapshots. General discovery and inspection, native event subscriptions,
authored signals, creation-time composition, return data and a cross-object
event composition witness remain open. Cross-room message causality under
independent room replay also remains open; snapshot membership alone does not
prove that contract.
