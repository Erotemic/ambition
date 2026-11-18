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
code and semantic subject identities. They are not native entity handles.
The generic protocol does not decide combat, grant, spawn, or travel rules.
Each domain must publish its own actual result.

## Installed command results

- [`inventory_exchange`](inventory-exchanges.md) reports terminal
  `inventory_exchanged` after the exact native bag and wallet exchange applies.
  A refused cost or reward leaves both owners unchanged.

- `projectile` reports `projectile_queued` at native admission and terminal
  `projectile_created` after actual construction. `subjects` contains the shot
  ID. Invalid shape, callback, or source context is refused. Explicit
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
joins its pending work or repeats its retained terminal reply. It never applies
the action again. Reusing a number for different work is rejected. An expired
old number is also rejected.

The ticket includes the submitting entry, native domain, request number, live
room, caller generation, and occurrence. The room assigns caller generations.
A recreated body with the same authored identity cannot complete an old body's
work. A surviving body's replies follow that body through room travel.

`cancel_action` takes a ticket and cancels pending work in the caller's scope
and module. The native consumer checks the ticket before each change. A
terminal applied action cannot be canceled.

## Delivery and lifetime

Results become eligible in the next `actor_think` read cut after their publish
tick. A held room does not consume its replies. Canonical snapshots contain the
pending actions, caller sequence, history, and delivery cursor. Rewind restores
all of them.

A scope retains 64 result rows and admits at most 32 pending actions. The
`action_history` observation contains the retained rows, their sequence numbers,
and a count of dropped rows. Scripts that need recovery can inspect that count
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
