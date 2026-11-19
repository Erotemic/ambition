# Results from scripted actions

A request can enter a queue without changing the world. Use a correlated reply
when a later script step depends on the change.

The `story_command` request accepts an optional `reply`:

```rhai
this.requests.push(request("story_command", #{
    line: "world.set_flag bridge_ready true",
    reply: #{ request: 0, selector: "my_game::bridge/result" },
}));
```

The selector is the full path of an entry in the same module. Declare that entry
with the `action_result` trigger in `actor_think`. Its trigger selector can be a
local name, such as `result`. The host directs the reply to the named entry in
the asking scope. It does not broadcast to other entries with that local name.

```rhai
#{ key: "result", phase: "actor_think",
   trigger: #{ port: "action_result", selector: "result" },
   writes: ["bridge.state"] }
```

The callback reads `this.trigger.stage`, `terminal`, `code`, `message`,
`subjects`, `tick`, and `ticket`. A script can advance its success state on
`applied` or `completed`, as defined by the action domain. It must match the
request number when it has several pending actions.

| Stage | Meaning |
| --- | --- |
| `submitted` | The adapter recorded the request. This does not prove admission. |
| `admitted` | The command owner accepted the work or placed it in its native queue. |
| `applied` | The native owner applied the action. A one-step change can end here. |
| `completed` | A longer action reached its defined end. |
| `rejected` | Admission or application refused the request. |
| `canceled` | The caller canceled pending work through its native owner. |
| `failed` | Admitted work could not finish, for example because its target left. |

`terminal` states whether that result ends the action. The domain supplies the
code and typed subjects. Each subject has a semantic `key` and an optional
live `object` reference. See [object references](object-references.md). They
contain no native entity handles.
The generic protocol does not decide combat, grant, spawn, or travel rules.
Each domain must publish its own actual result.

## Validate reply wiring

A submitting entry can declare its required callbacks:

```rhai
#{ key: "tick", phase: "actor_think",
   trigger: #{ port: "story_tick", selector: "bridge_room" },
   replies: ["my_game::bridge/result"],
   requests: ["story_command"] }
```

Admission checks each path against the selected module composition. It must
name an `action_result` entry in the same module in `actor_think`. A missing
entry, wrong phase or port, or foreign module refuses admission. This includes
reload: removing a required callback keeps the last valid program. `replies`
is a dependency list, not a scheduling order or a list of aliases.

Request values can choose callbacks dynamically. Before it commits an
invocation, the host checks every action request's actual callback. An invalid
address discards all staged script state and requests from that invocation.
It cannot commit an awaiting flag, spend costs, or submit native work.
Linked Rust, Rhai, and WASM use this same preparation boundary.

The submitting scope can read `this.observe.action_history.refusals`. Each row
contains `entry`, `domain`, `request`, `handler`, `signature`, `tick`, `sequence`,
and code `invalid_reply_entry`. This is a submission refusal, without a reserved
ticket or pending action. Repair can reuse its request number. Equal refusals
are retained once. The scope retains 32 distinct refusal rows; replacing the
oldest increases the shared `dropped` count. Refusals and results share the
sequence counter. These rows rewind with the scope's mailbox.

The local host fault also names the request port and callback path. A refusal
row does not need the broken callback to run. Already accepted work follows its
domain lifetime if a dynamic callback later becomes unbound; its retained
result exposes `unbound` until the handler is available or its source retires.

SDK API 0.4 includes static reply dependencies. The loaded descriptor format
is `ambition-ext-2`; rebuild portable module files with this SDK. The history
observation is version 4. Cancellation is version 3 because its ticket also
carries the optional allocation room. There is no old descriptor decoder.

## Installed command results

- [`consume_contact`](contact-consumption.md) reports terminal
  `contact_consumed` after both the native destructible state and inventory
  exchange apply. `subjects[*].key` names the consumed contact. A refusal
  changes neither owner.

- [`inventory_exchange`](inventory-exchanges.md) reports terminal
  `inventory_exchanged` after the exact native bag and wallet exchange applies.
  A refused cost or reward leaves both owners unchanged.

- `projectile` reports `projectile_queued` at native admission and terminal
  `projectile_created` after actual construction. `subjects` contains the shot key and its live object reference. Invalid shape, callback, or source context is refused. Explicit
  cancellation removes a queued emission before construction starts.

- `world.set_flag` reports `flags_written` after the native flag consumer applies
  the batch and its quest updates.
- The room-look cycle uses one flag batch. Its reply follows both changes.
- `encounter.start` and `encounter.signal` report from the encounter reducer.
  A missing target or an invalid lifecycle phase cannot report application.
- Commands that apply their own state before they return report application
  from the command runner. For example, applying an actor direction is an
  intent change. It does not promise arrival at that direction's destination.

This support does not add results to every other request port. In particular,
`play_cutscene` does not yet report playback completion. See the open
[capability census](../planning/engine/script-first-capability-census.md).

## Identity and retries

Use a new, increasing `u32` request number for each action from one submitting
entry and domain in one scope. Retrying the same number with the same request
joins its pending work. For a quarantined terminal callback, it starts another
bounded set of delivery attempts. It does not call an acknowledged handler or
apply the action again. Reusing a number for different work is rejected. An expired
old number is also rejected.

The ticket includes the submitting entry, native domain, request number,
allocation room, object generation, and action occurrence. The native
`ObjectOccurrence` stamp identifies one live object lifetime in its session.
Its room is an allocation namespace. It does not change when the object travels.
The room assigns a fresh sequence when an object first needs a reference.
The stamp and counter rewind together. Retirement removes the stamp; rollback
reconstruction restores it. A new object with the same authored identity gets
a different stamp.

Both native reply APIs resolve the session and occurrence stamp. An equal
authored identity in another room, with or without a mailbox, cannot receive
or block the reply. Room roots use the same occurrence path and retire with
their live room. A surviving body's replies follow that body through travel.

`cancel_action` takes the target ticket and a separate `reply`. Cancellation
is itself an action in the `actions.cancel` domain:

```rhai
this.requests.push(request("cancel_action", #{
    ticket: pending_ticket,
    reply: #{ request: next_cancel_request, selector: "my_game::bridge/result" },
}));
```

A successful cancellation emits terminal `canceled` for the target and terminal
`applied` with code `action_canceled` for the cancellation request. A refusal
emits terminal `rejected` for the cancellation request and leaves the target's
state unchanged. The cancellation callback uses the same commit and retry
contract as any other action callback. Retrying the same cancellation request
does not repeat its native operation.

The target must belong to this scope and module. Refusal codes distinguish
`cancel_foreign_module`, `cancel_unknown_ticket`, `cancel_already_terminal`,
`cancel_unknown_domain`, `cancel_unsupported`, `cancel_application_started`,
and `cancel_owner_refused`. A retained terminal target cannot be canceled.
After its history expires, it can report `cancel_unknown_ticket`.

Each installed action domain declares one native cancellation policy:

| Domain | Policy |
| --- | --- |
| `authored.command` | Each queued consumer checks the pending ticket before mutation. Cancellation stops submitted or admitted work. |
| `projectiles.spawn` | The native handler removes queued emissions. It refuses once the queue no longer owns the work. |
| `world.create_actors`, `world.create_breakables` | Unsupported. Construction finishes in one native call. |
| `inventory.exchange`, `world.consume_contact`, `combat.body_query` | Unsupported. These operations finish in one native call. |
| `actions.cancel` | Unsupported. Cancellation finishes in one native call. |

An unregistered domain refuses cancellation. It cannot inherit mailbox-only
success. A new queued command must retain its reply and check its pending
ticket immediately before every mutation. A domain that cannot guarantee that
must use a native suppression handler or declare cancellation unsupported.
Cancellation does not reverse changes that already committed.

## Delivery and lifetime

Results become eligible in the next `actor_think` read cut after their publish
tick. A held room does not consume its replies. Canonical snapshots contain the
pending actions, caller sequence, history, and delivery state. Rewind restores
all of them. Queuing a callback does not acknowledge it. The host acknowledges
it only after its state and staged requests commit. A handler fault commits
neither state nor requests.

Each history row exposes `handler`, `delivery`, `attempts`, `last_attempt`, and
`last_error`. Delivery has these states:

| State | Meaning |
| --- | --- |
| `waiting` | The result awaits a later read cut. |
| `queued` | The host has a directed invocation for this row. |
| `invoked` | The host started the handler. |
| `acknowledged` | Handler state and requests committed. |
| `faulted` | The handler failed and can retry on a later tick. |
| `quarantined` | The handler reached its attempt limit. Explicit retry is required. |
| `unbound` | The destination is absent. Rebinding permits delivery without a new action. |
| `retired` | The submitting entry left the composition. Delivery has stopped. |

`action_delivery_rules` sets positive `max_attempts` and `max_per_tick` budgets.
The defaults are three attempts per row and 64 queued callbacks per scope per
tick. Ambition declares these rules in its content pack. A fault retries at
most once per row per tick. Quarantine retains the result for inspection.
Resubmit the same request after repair to retry its quarantined terminal
callback. This does not repeat native work. An acknowledged row stays
acknowledged across code replacement and request retries.

Canonical `last_error` values are fixed codes, such as `handler_fault` and
`reply_entry_unbound`. Full VM errors remain in the local `ExtensionFaults`
diagnostic. Its `receipt_sequence`, scope, and entry identify the failed row.
Local paths in an error do not become rollback state.

A scope retains 64 result rows and admits at most 32 pending actions. The
`action_history` observation contains result and submission-refusal rows, their sequence numbers,
and a count of dropped rows. New rows first replace an acknowledged or retired
row. If neither exists, they replace the oldest row and increase the dropped
count. Scripts that need recovery can inspect that count
and reconcile with the domain's current state.

Code replacement retains replies when their source entry remains installed.
Entry removal retires callback ownership without inventing a physical outcome.
Recipient removal retires its mailbox. Native domains define whether their
work can outlive that observer. An admitted projectile emission does; an
unapplied caller-bound command can stop. The caller sequence prevents an old
reply from completing work in a replacement recipient.

Tickets and mailboxes belong to the live session. They are rollback state, not
saved progress. Do not keep an unfinished ticket in a durable script record.
After a durable load, recover from the domain's saved state. Persistent grants
and consumption need their owning domain's durable transaction rules.

Portable Rust and WASM modules use `ambition_extension_sdk::action` for the same
values and ports. Portable story contracts are in `ambition_world_port`; the
engine facade exposes them as `ambition_platformer2d::extension::world`.
