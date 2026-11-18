# Native hit reception rules

`hit_receptions` delivers the decisions of native damage receivers. A held
status binds its selector on a body. Authored combat rules bind room
observers. A body receives its incoming and outgoing decisions. A room root receives decisions about
recipients in its own session and live room.

An entry can react with existing `hit_reaction` requests:

```rhai
#{ key: "receive", phase: "hit_reaction",
   trigger: #{ port: "hit_receptions", selector: "my_game.reactive" },
   requests: ["hold_status"], limit: 1 }
```

```rhai
fn receive() {
    for hit in this.trigger.events {
        if hit.incoming && hit.decision == "damaged" && hit.damage > 0 {
            this.requests.push(request("hold_status", #{ kind: "my_game.charge", held: true }));
            break;
        }
    }
}
```

A technique can hold `my_game.reactive` through `hold_status`. A room observer
is declared in its game's combat rules. Portable Rust modules use
`ambition_combat_port::HitReceptionsPort` and the same native adapter.

## Decisions and read cut

The trigger contains `events` and `truncated`. Each event contains:

- `observed_at`: the simulation tick when the native receiver made its decision.
- `victim` and optional `attacker`: captured semantic identity and live room
  within the invocation's session. An identity outside that session is absent.
  A native entity handle is not exposed.
- optional `projectile`: the shot's semantic `id` and contact `room`. Direct
  hits and splash hits retain the same shot identity. An unidentified native
  source has no ID. A shot from another session exposes no identity or room.
- `source`: `melee`, `projectile`, `contact`, `hazard`, `left_the_world`, or `pogo`.
- `decision`: `ignored`, `protected`, `blocked`, `armored`, `wallet_shielded`, or
  `damaged`.
- `raw_damage`: the proposed amount, before native defenses and modifiers.
- `damage`: the native resolver's amount after those rules. It is zero for
  refused, protected, blocked, armored, and wallet-shielded hits. An accepted
  zero-damage push also reports zero. A lethal amount can exceed remaining HP.
- `wallet_spent`: the balance consumed by a wallet shield, otherwise zero.
- `health_depleted`: whether the health resolver reached its limit. The native
  lifecycle or match owner decides defeat; this field does not award a knockout.
- optional `move_instance`: the attacker's move occurrence, when one claimed it.
- `incoming` and `outgoing`: whether the invocation body received or owned the
  hit. Both can be true for self-damage. Both are false for a room observer.

`protected` includes a receiver whose policy accepts the strike without health
loss, such as a protected NPC, a guarded puzzle boss, or a training dummy.
`ignored` includes native invulnerability and the controlled receiver's policy
that processes only the first hit of its pending batch.

The adapter runs in `hit_reaction`, after native damage reception. Each nonempty
batch is delivered once at that phase's next read cut. A controlled body's
pending damage resolves on the next simulation tick; the callback reports that
actual decision time. Hitstop does not suppress a decision that caused the
hold. No empty callback is sent.

Events use tick, session, room, and recipient identity order. A recipient's
native decision order is preserved within that group. Selectors are unique
within a scope, even if a held status repeats the key.

Session activation counters belong to the host. Native routing checks the
session without exposing that local counter to the script.

Native producers use `ambition_combat::HitWriter`. It captures the source before
publication. Detached volumes retain the same `HitOrigin`. This applies to
melee, projectiles, hazards, and other native hit kinds. A retired source does
not erase the credit of a pending hit.

For a projectile, `attacker` is the credited body at contact. Reflection changes
that credit. Direct hits and splash volumes retain it before the shot or its
credited body retires. Deferred reception uses these captured facts. It does
not look up a replacement body or restore credit to the original shooter.

A shot can retire before deferred damage resolves. Retirement does not mean
that no later damage decision can arrive. Accepted damage, interception, and
retirement are separate native facts.

## Bounds, lifetime, and replay

A game's reloadable `room_rules` content can set:

```ron
(hit_receptions: Some((max_events: 32, room_observers: ["my_game.reactive"])))
```

The default is 64 events per scope and phase read cut. The limit must be
positive. Room observer selectors must be nonempty. Each live room root uses
its own rules to select observers; a body status does not subscribe a room.
The batch reports `truncated` if more decisions matched. Omitted
events are not sent later. Absence from a truncated batch cannot prove that a
hit did not occur. Removing the declaration restores the default and removes its room observers.

Native publication captures identity before death or travel changes the
receiver. A retired recipient receives no callback. A new body with the same
semantic name cannot receive its old callback. A live room observer can still
read that recipient's captured decision. A body that changes scope before the
read cut does not receive the old scope's decision.

The native decision messages are transient. Rollback clears abandoned messages,
and replay produces decisions from the restored native state. Script records
use the normal canonical state store. A retained body name must use a declared
`body` or `opt body` field; the captured room remains event context,
not permission to act on a later body.

This is the damage receiver's stream. It does not supply per-shot identity,
reflection, absorption, or projectile retirement. It also does not replace
physical contact, `struck`, stance parries, or move confirmation. Those owners
have different read cuts and effects.

The assembled witness is
[`scripted_hit_receptions.rs`](../../game/ambition_app/tests/scripted_hit_receptions.rs).
It loads a Rhai module and checks native wallet absorption, damage, repeated-hit
refusal, incoming and outgoing delivery, room observation, a held-status
response, game bounds, and rollback. Owning tests check retirement, replacement
bodies, held clocks, and session and room isolation.

A `hit_reaction` entry can also request `projectile`. The native writer captures
the caller's source facts. Late requests wait in rollback state until the next
early materializer. See [projectile ownership](projectile-ownership.md).

Use [projectile outcomes](projectile-outcomes.md) for creation, interception,
clash spending and retirement. Damage reception does not supply those facts.
