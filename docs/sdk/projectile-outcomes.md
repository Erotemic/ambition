# Native projectile outcomes

`projectile_outcomes` reports native creation, interception, clash spending,
and retirement. It is a trigger in `hit_reaction`. A held status binds an
observer on any body. A game's `room_rules` binds room observers:

```ron
(projectile_outcomes: Some((
    max_events: 32,
    room_observers: ["my_game.flight"],
)))
```

```rhai
#{ key: "flight", phase: "hit_reaction",
   trigger: #{ port: "projectile_outcomes", selector: "my_game.flight" },
   writes: ["flight"] }
```

Each event has `observed_at`, `projectile`, `credit`, `previous_credit`,
`interceptor`, `kind`, `terminal`, and `position`. Each participant has an
optional semantic `id` and live `room`. Native entity handles and host session
counters do not enter the portable or Rhai value. An unidentified native
source can have no ID; it does not acquire an invented identity. `position`
is the committed shot root in `projectile.room`. It is not a contact witness
or a point in the receiver's chart.

| Kind | Native fact | Terminal |
| --- | --- | --- |
| `created` | The materializer constructed the shot. | No |
| `reflected` | Interception changed trajectory, credit and allegiance. | No |
| `absorbed` | Interception consumed the shot. | Yes |
| `spent_by_clash` | Clash resolution exhausted the shot's lifetime. | No |
| `expired` | The flight owner removed a shot with exhausted lifetime. | Yes |
| `body_contact` | The flight owner removed a shot on body contact. | Yes |
| `feature_contact` | The flight owner removed a shot on feature contact. | Yes |
| `world_contact` | The flight owner removed a shot on world contact. | Yes |
| `room_retired` | A committed room transition removed a shot left behind. | Yes |
| `attempt_reset` | An admitted replay removed a previous attempt's shot. | Yes |

`credit` is the credit after the operation. Reflection also supplies
`previous_credit` and `interceptor`. Absorption keeps the caught shot's credit
and names the interceptor separately. Retained credit does not need a live
body link. Creation provenance stays with the shot ID through reflection.

`terminal` means that this operation removed the projectile entity. It does
not report damage. A queued direct hit or landing splash can resolve after
the shot is gone. Use [hit receptions](hit-receptions.md) for native defense
and damage results. A returning shot can touch a body without retiring, so
that contact does not produce a `body_contact` retirement. A clash first spends
a shot, then the flight owner runs its normal expiry path and effects.

## Delivery and lifetime

An installed observer receives future outcomes for its live credit or interception role. Reflection
also reaches the previous credited body. These observations follow the same
live body through room travel. A replacement with the same authored ID does
not receive the previous body's facts. A room root receives outcomes for its
own session and live room. Other rooms and sessions do not receive them.

Native commands capture identity before removal. Pending script facts live on
their observer as bounded canonical state. Late creation remains pending across
a snapshot and room travel. Retiring an observer removes its pending delivery.
Events for each shot keep
their operation order. Independent shots are ordered by native projectile
sequence. Each nonempty batch is delivered once at the next `hit_reaction`
read cut. The default budget is 64 events per scope and read cut; a game can
set a positive `max_events`. `truncated` reports further matching events.

An observer removed before that read cut receives nothing. Session teardown
removes the observers and their queues; it does not emit callbacks into a
retired game. Whole-world rollback restores pending delivery and clears
abandoned native messages. Room replay restores each observer with its own
room. Hidden live rooms receive no replay output. Script records rewind through
the normal extension-state path.

[Creation replies](projectile-ownership.md#observe-creation) correlate one
request with its actual created ID. Outcome batches observe native operations,
including shots made by native gameplay. They do not replace action tickets.

## Witnesses

The loaded [reflection fixture](../../game/ambition_app/tests/scripted_projectile_credit.rs)
checks creation, reflection, retained credit after deflector retirement, final
shot retirement, damage, body delivery, room delivery, and GGRS replay. A
second loaded fixture snapshots pending creation and clears native transport
before delivery. Both controllers still receive it once. Native
owner tests check absorption before removal, bounded delivery, room/session
isolation, replacement refusal, and delivery after body travel.
