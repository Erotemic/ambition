# Reusable object behaviors

A behavior binding attaches declared methods, parameters and private records to
one live object. Several instances can use the same code and schema on that
object. Each instance has its own records, action mailbox and call queue. The
native subject supplies observations, time pace and domain actions. A wallet
action still changes that subject's wallet through the inventory owner.

The current ports run in `actor_think`. They share portable Rust, Rhai and
linked Rust contracts. A loaded Rhai witness runs through the assembled engine
and GGRS. The same witness also loads a Rust method module as actual WASM. Its
schemas are new to the host; the fixture crate is not linked into the game.

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

Parameters are bounded data. Admission puts named fields in canonical key order,
including inside lists and nested records. Native and portable execution keep
the same data order before and after rollback. `this.observe.behavior` supplies
`object`, `binding` and `params` to a method and its action callbacks. Parameters do not change
when the method changes its records. Normal schema reload rules apply to each
instance's `BodyRecords`.

`attach_behaviors` adds instances to an existing object. Actor and prop factories
can instead put the same binding records in each node's `bindings` list:

```rhai
#{key: "guard", character: "practice_sandbag", at: [200.0, 100.0],
  bindings: [#{key: "shop", params: #{price: 3},
      methods: [#{name: "buy", entry: "example::shop/use"}]}]}
```

The complete batch validates method addresses, private state and game bounds
before native materialization. The constructor reserves one occurrence range
for roots and their private instances. The native plan declares private instances
with their owners before it allocates them. Roster verification checks their
identity and ownership. All remain hidden until native
verification and occurrence assignment succeed. The first native birth notice
can read every initialized instance. Invalid state or a missing method refuses
the entire creation batch. The creation result returns the physical roots.

Actor root `state` seeds still require the factory entry's write permission.
Private binding seeds require the declared methods' write permissions. The
private instances are separate state owners, even when they use the same schema.
Creation ports are `create_actors` version 4 and `create_breakables` version 3.
`attach_behaviors` and the `behavior` observation are version 2.
Rebuild portable module files that use these ports.

## Connect objects at construction

A binding can declare named `references`. A live reference names an existing
occurrence. `#{local: "node_key"}` names a root in the same factory request:

```rhai
#{key: "relay", character: "practice_sandbag", at: [200.0, 100.0],
  bindings: [#{key: "logic",
    methods: [#{name: "activate", entry: "example::relay/activate"}],
    references: #{peer: #{local: "worker"}, owner: #{local: "relay"}}}]}
```

A second node can name `relay` in its own references. The native construction
plan resolves both names against its checked roots and reserved occurrence
range. Internal character clusters do not change the meaning of a node key.
All root and private initialization is prepared before any entity is built.
An unresolved name refuses the entire batch without births or a spent creation
counter. The first native publication notice follows reference initialization.

A method reads `this.observe.behavior.references.peer`. This is an exact live
object reference. It can be the target of an ordinary `call_object` request.
The native call owner still checks admission and the target method. A reference
does not grant a physical capability or prove that a domain action succeeded.
The reference map uses canonical key order and rewinds with the private instance.
Target retirement leaves a stale reference; native operations refuse that lifetime.

`attach_behaviors` can use live references. It has no factory namespace, so a
local name in that request is refused. References must belong to the receiver's
session. Their names are unique, nonempty and at most 64 bytes. Local node keys
are at most 128 bytes. `object_behavior_rules.max_references` defaults to 32 per
binding. These bindings are transient; they do not create durable save addresses.

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
    accepted: this.trigger.stage == "applied", code: this.trigger.code,
    data: #{object: this.observe.behavior.object}
}));
```

The original caller receives `completed` or `rejected`. The completion action
also receives its own applied or rejected reply. It refuses an unknown,
terminal, undelivered or foreign-module call. Code text must be nonempty and at
most 128 bytes.

`data` returns bounded structured values and defaults to `()`. The completion
owner checks it against `object_behavior_rules.max_data_bytes`. In the original
call's terminal callback, `this.observe.object_call_result` supplies the matching
`ticket` and `data`. Admission failures, cancellation and native terminal failures
have no method return value. Nonterminal callbacks also have no return value.
Check the action stage and the optional observation.

Return values can contain exact object references. A caller can keep them in
transient `object` fields and use them in later native requests. Return payloads
follow the mailbox's delivery retention. Faulted and quarantined callbacks keep
their payload while their journal row can recover. Later acknowledged calls
cannot evict an older payload whose callback is still retained. Body-query
payloads use the same mailbox rule.

A behavior must manage concurrent work in its private records. It can reject a
second call as busy, or keep a bounded collection of pending call sequences.
Do not overwrite the first sequence while its native action is outstanding.

## Delivery, cancellation and retirement

Method calls and action results use one addressed delivery pass. Each private
receiver shares `action_delivery_rules.max_per_tick` across both payload types.
Admission orders candidates by publish tick, then outcome before request, then
local sequence. The host keeps its declared handler order for execution.
Delivery uses the same `DeliveryProgress` for both payload types.
Only successful host commit acknowledges a handler. A fault discards staged
state and requests. Bounded retries end in an explicit `object_handler_fault`
result. An unavailable method waits for repair until the call's active wait
budget expires. Paused rooms do not consume that budget.

Cancellation suppresses a queued method before handler commit. It is refused
after commit, including while the method waits for another native result.
A canceled call cannot later invoke its method. A timeout is a failed call; it
does not undo an already committed handler or cancel that handler's native work.

Instances retain the subject's exact occurrence reference across travel. A
room-owned instance uses the room's allocation counter. A session-owned instance
uses the session counter. Allocation does not change during travel. Retirement
of an old room cannot delete
a traveling subject's instance. Subject retirement fails outstanding calls and
retires the private instance at the next collection cut. Caller retirement or
removal of its pending ticket retires the queued request. Rollback restores
instance state, occurrence allocation, queues and acknowledgements together.
`ObjectScopeOwner` is the single owner link. Room snapshots and visibility cuts
include private state through that link, including after owner reconstruction.

## Game-authored bounds

The `object_behavior_rules` room schema supplies bounds for instances per
object and session, methods, named references, initial records and bytes, parameter and call data,
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
authored signals, local references between nodes in a factory batch, and a
cross-object event composition witness remain open. Cross-room message causality under
independent room replay also remains open; snapshot membership alone does not
prove that contract.
