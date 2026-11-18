# Contact consumption

`consume_contact` consumes a destructible prop and applies an inventory
exchange as one native operation. A failed cost, grant, or object check
changes neither state. Scripts select the contact and supply the exchange
terms. Native breakable, item, collision, and occurrence owners keep authority.

## Author a rule

Bind a `body_contact` entry through a held status or custom brain key, as in
[physical contact rules](body-contacts.md). Declare `consume_contact` in its
`requests` list with a request budget. In that entry:

```rhai
for index in 0..this.trigger.world.len() {
    let hit = this.trigger.world[index];
    if hit.role != "head" || hit.block == () { continue; }
    if hit.block.geometry.origin != "placement" { continue; }
    if hit.block.geometry.key != "my_authored_prop" { continue; }
    this.requests.push(request("consume_contact", #{
        reply: #{request: 1, selector: "my_game::props/result"},
        contact: index,
        exchange: #{grants: [#{id: "healthcell", count: 1}], pay_coins: 2},
    }));
    break;
}
```

Use host records to track pending actions and to allocate a new request number
for each new operation. The example assumes that its first operation is ready.
Do not submit this fixed request number with different terms on later steps.
Declare `result` as an `action_result` entry in the same module, in
`actor_think`. [Action results](action-results.md) defines ticket scope and
retry identity. An exact retry does not consume or pay again. A retry can deliver its cached
terminal result again, so a result handler must be idempotent. Its ticket can
retain the original contact index solely to repeat that same request.

`exchange` defaults to empty terms. Its costs, grants, and currency fields use
[inventory exchange](inventory-exchanges.md) semantics. Controlled and
autonomous callers use the same service. The inventory bag is shared; the
wallet belongs to the caller. Currency terms require a native `BodyWallet`
capability. The fixture installs that optional capability on its NPC. Current
character creation does not expose wallet configuration to scripts. Item-only
terms do not require a wallet.

## Contact and object authority

The request is available only in `body_contact`. A new operation uses an index
from that invocation's bounded `world` list. Native movement output must match
the current tick, live room, and session. A held room, inactive body, refused
movement step, or index outside the contact budget cannot authorize consumption.

The contact must identify the ordinary placement collider of exactly one
live `FeatureSimEntity` with native `BreakableFeature` state in that scope.
Solid and one-way breakables are supported. Tiles, chains, ordinary walls,
pogo volumes, and unknown placement owners cannot be consumed with this port.
An already consumed or protected object is refused before any payment.

Consumption is a domain operation. It consumes the whole eligible object;
it does not simulate a combat strike or depend on the object's damage trigger.
The native health protection still applies. The supplied exchange is the
reward. This operation does not add the normal combat breakage bounty.

Competing callers can reference the same intact contact sample. The first
successful native operation consumes the object. Later operations refuse it
and make no inventory change. Submission alone is not evidence of a reward.

## Results, collision, and lifetime

A terminal `applied` result with code `contact_consumed` means both the native
object change and the exchange applied. `subjects` contains the consumed
object's actual simulation identity. The result is eligible at the next actor
read cut. It does not claim that the object was deleted.

The object enters the normal native broken state. Existing view publication
updates its art and visibility. The next collision rebuild removes its intact
collider. Native respawn rules still apply. A timed object can return and can
then be consumed by a new action. `OnRoomReload` objects return when their room
is rebuilt. Use authored `Persistent` respawn for a durable claim.

For an authored persistent object, consumption writes its `Spent` occurrence
and room-owned checkpoint history in the same operation. Save/load and room
rebuild retain that claim. Inventory history follows its existing checkpoint
retention rules. The live action ticket is not the durable claim. Persistent
dynamic objects are refused until their native lifetime owner supports them.
This port does not add dynamic actor or NPC wallet persistence.

Refusals include `contact_unavailable`, `stale_contact`,
`contact_outside_budget`, `motion_refused`, `unsupported_contact_object`,
`unowned_contact_geometry`, `object_already_consumed`, `object_protected`,
`object_already_recorded`, `persistent_dynamic_object_unavailable`, and the
inventory refusals. Missing native services and inactive scopes also refuse.
An invalid callback cannot consume the object.

The portable contract is `ambition.world.consume_contact@1`.
`ambition.world.inventory_exchange@2` uses the same `InventoryTerms` record.
The [loaded fixture](../../game/ambition_app/tests/scripted_contact_consumption.rs)
uses one Rhai library with separate host records for support claims and head
contact purchases. It exercises actual native movement and object state.

The [local cost observation](../../dev/benchmark-candidates/scripted-contact-consumption-cost.md)
records backend and invocation time, including replay calls. It excludes native
movement, contact collection, and transaction application.
