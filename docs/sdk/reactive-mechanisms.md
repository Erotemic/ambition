# Reactive local mechanisms

The `switch_lab` room has a toll lever and a destructible barrier. LDtk defines
both placements, the coin pickup, the barrier shape and its return policy.
[The shipped script](../../game/ambition_content/rhai/switch_lab.rhai) defines
the connection and the price. A source edit can change the behavior without a
host build. The room's older persistence lever keeps its own behavior.

The toll lever uses the native `Activate` action. The interaction owner checks
reach, facing, the acting body and target preference. Each accepted press
updates the target's canonical `ActivationState`. The record contains a revision
and the latest acting occurrence. This action does not write a save switch.

`object_activated` is a typed notification. A behavior subscription names its
source reference and an existing method. The shared inbox delivers the method
in a later `actor_think` cut. Host commit acknowledges the notification.
`object_activation` inspects the native evidence on the current subject.
The example treats notifications as wakeups and also reconciles the latest
activation twice per active simulation second. It can combine several presses into one request.
It does not promise one transaction for every press under queue overflow.

## Hear a defeat

`object_defeated` is a typed notification, like `object_activated`. The damage
owner commits it on the hit that kills a body, before any other owner takes the
death (loot and respawn, a match ruleset, a fall out of the world). The payload
names the defeated `object` and, when the hit had an addressed attacker, `by`.

Subscribe to it as to an activation. Name the body as a binding reference and
add a subscription from that reference to a method whose entry has the trigger
`#{port: "object_defeated", selector: "defeated"}` in `actor_think`:

```rhai
bindings: [#{key: "watch", references: #{guard: guard_object},
    methods: [#{name: "down", entry: "my_mod::room/down"}],
    subscriptions: [#{key: "guard_down", source: "guard", method: "down"}]}]
```

The method runs once per defeat, at a later actor read cut, with the binding's
private state. Provenance is captured on the killing tick, so the defeated body
need not still exist when the method runs. A body that respawns in place and
is killed again produces a second fact. The notification is bounded like any
other: a mailbox at capacity drops it and reports the drop in `behavior_events`.
Keep the native state you need (for example, a body query of the living guards)
when the mechanism must survive a dropped wakeup.

## Hear a break

`object_broken` is the prop counterpart of `object_defeated`. The breakable
owner commits it on the tick a breakable is spent, whatever spent it: a strike,
a contact consumption or its own `consume_self` exchange. The payload names the
`object`. A breakable that returns and is broken again produces a second fact.
A breakable built already broken (a restored save) produces none.

Subscribe as to a defeat. The method's entry has the trigger
`#{port: "object_broken", selector: "broken"}` in `actor_think`, and the
subscription's source is the breakable's reference. Find a placed breakable
with `resolve_authored_objects`. The witness is
`a_subscribed_method_hears_its_barrier_break_once` (`switch_lab`).

## Hear a take

`object_collected` names a pickup that a body took, and `by`, the body that
took it. The pickup owner commits it on the tick of the take, at the one place
a pickup is collected, so a scripted "when the key is taken" needs no polling.
A pickup that regrows and is taken again produces a second fact. Subscribe as
to a break: `#{port: "object_collected", selector: "collected"}`. The witness is
`a_subscribed_method_hears_its_pickup_taken_once_and_by_whom` (`switch_lab`).

## Send a signal

A signal is your own named message, as a Godot signal is: the object a script
runs for emits it, and each method subscribed to that object for that topic
hears it. Unlike `object_defeated`, a signal is not a fact the engine vouches for.
It is a request that the runtime admits or refuses.

Emit with `emit_signal` from an `actor_think` entry that declares it in
`requests`:

```rhai
this.requests.push(request("emit_signal", #{topic: "opened", data: #{by: "lever"},
    reply: #{request: 0, selector: "my_mod::door/sent"}}));
```

Subscribe as to a defeat. The difference is that the method's entry names the
topic as its selector on the `object_signal` port:

```rhai
#{key: "hear_open", phase: "actor_think", trigger: #{port: "object_signal", selector: "opened"}, writes: ["gate"]}
// in the binding:
references: #{door: door_object},
methods: [#{name: "on_open", entry: "my_mod::gate/hear_open"}],
subscriptions: [#{key: "door_open", source: "door", method: "on_open"}]
```

The handler reads `this.trigger.from` (the emitting object), `this.trigger.topic`
and `this.trigger.data`. The runtime names the sender. A script cannot claim
to be another object. A topic names only signals: it is kept apart from native
facts, so no topic can be heard as `object_defeated` or any other fact.

Admission is all or nothing. The sender's `reply` is `applied`, with one
`recipient` subject for each object that will hear it (none is fine). If the
signal cannot reach every subscriber, the reply is `rejected` and nobody hears
it. The refusal codes are:

- `invalid_signal_topic`: the topic is not 1 to 48 bytes of `a-z0-9_.-`;
- `signal_data_bound`: the data exceeds `ObjectBehaviorLimits.max_data_bytes`
  (4096) or a recipient's mailbox bound;
- `signal_recipient_bound`: there are more than 16 subscribers;
- `signal_queue_full`: a subscriber's mailbox holds only undelivered rows;
- `cross_island_signal_unsupported`: a subscriber is in another rollback
  island.

Delivery waits for the next actor read cut and shares the receiver's per-tick
budget with every other addressed payload.

A subscription names one exact occurrence, never a placement. A room left and
entered again is a new live copy: its placements have the same keys and new
occurrences. A behavior attached to the travelling body keeps its old
subscriptions, and they hear nothing from the new copy. Look the placement up
again (`resolve_authored_objects` answers for the caller's current room) and
subscribe to what it names. The witness is
`a_room_entered_again_is_a_new_copy_and_only_its_own_watch_hears_its_barrier`.

## Find authored objects

`resolve_authored_objects` takes `placements` and a normal `reply`. It uses the
caller's current live room, that room's source definition and native
`SpawnOrigin` records. It returns one `subjects` row per requested placement,
in request order. Each row has the placement key and an exact `object` address.
The lookup request is all-or-nothing. It reports absent, ambiguous and unpublished
placements with distinct refusal codes. It does not select an object by its
position, display name or current capabilities.

`ObjectBehaviorLimits.max_authored_lookups` sets the batch bound. The default
is 64. Placement keys must be nonempty and at most 256 bytes. The registry is a
read-cut projection of native provenance. Travel or retirement changes the next
lookup. A reference already returned continues to name the same occurrence.

## Spend a destructible object

`destructible` inspects the current subject's health and consumed state.
`consume_self` asks that subject's native destructible owner to spend it and
apply inventory terms to an exact `recipient` body in the same live room and
rollback authority. A behavior cannot use this request to spend another prop.
The request uses the same native transaction as movement-contact consumption.
It preflights lifecycle and services, applies the exchange and destructible
state together, and uses the existing collision, art and return paths.

Binding parameters are constructor data. A script reload keeps them. Edit the
method or rebuild the room to change a constructor seed.

The barrier method requests `consume_self` with a coin cost. Its native result
completes the original object call. Insufficient funds, an already consumed
barrier, an inactive body or a foreign recipient refuses without a charge.
Competing calls receive explicit outcomes. A successful callback does not apply
the exchange again. The barrier can also be broken through normal combat.

This is a destructible toll mechanism. A reusable locking door with independent
lock and open states remains a separate native capability. Direct foreign-island
calls refuse. Cross-island consequences still require a committed world domain.


The integration check uses real Interact input, native collision publication,
refused and applied exchanges, duplicate-charge prevention and GGRS replay.
A second fixed-host run sets notification capacity to zero, reconciles the
native revision and changes the method through a watched Rhai file. The same
barrier occurrence remains live. Caller-owned sync-test timelines refuse live
mechanical edits; the edit check uses the fixed host.
