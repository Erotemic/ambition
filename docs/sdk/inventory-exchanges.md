# Inventory exchanges

`inventory_exchange` applies one exact exchange through the native item owner.
It can spend and grant stored catalog quantities and move currency in the
calling body's wallet. All costs must exist before any grant. Every grant must
fit its catalog stack. A refusal changes neither owner.

The bag is the existing shared `OwnedItems` ledger. The wallet belongs to the
invocation body. Controlled actors and NPCs use the same service. Stored
quantities and held object instances remain separate authorities.

## Author an exchange

```rhai
this.requests.push(request("inventory_exchange", #{
    reply: #{ request: 1, selector: "my_game::craft/result" },
    exchange: #{
        costs: [#{ id: "healthcell", count: 2 }],
        grants: [#{ id: "manacell", count: 1 }],
        pay_coins: 5,
    },
}));
```

The `exchange` record contains the reusable inventory terms.
`costs` and `grants` default to empty lists. `pay_coins` and `receive_coins`
default to zero. Quantities must be positive whole numbers. Currency amounts
must fit a non-negative `i32`. Duplicate item lines are added before checking
costs or capacity; integer overflow is refused. A grant cannot fund a cost in
the same exchange. Catalog IDs are exact; dialogue aliases are not required.

The request is installed in `actor_think`, `technique_execution`, `wielded_use`,
`module_entity_tick`, `boss_conduct`, `body_contact`, and `hit_reaction`.
Room `story_tick` entries run in `actor_think`. Declare the request in the
entry's `requests` list and give it a request budget, as with other ports.

A reply is required. Declare its selector as an `action_result` entry in the
same module, in `actor_think`. Use a new request number for each new exchange.
An exact retry does not apply the exchange again. A terminal `applied` reply
with code `inventory_exchanged` means that the native owner changed the bag
and wallet. `subjects` lists the catalog IDs used by the request. The reply
becomes eligible at the next actor read cut.

Refusal codes include `unknown_item`, `invalid_quantity`, `insufficient_items`,
`stack_full`, `missing_wallet`, `insufficient_funds`, and `wallet_overflow`.
A missing catalog, bag, history service, or live scope is also refused. An
invalid callback cannot apply an exchange. See [action results](action-results.md).

## Read current inventory

Declare `inventory` in an entry's `reads` list. Read it as
`this.observe.inventory`. Its `items` list contains each real catalog item's
`id`, stored `count`, and stack `capacity`. `wallet` is the calling body's
balance, or `()` when it has no wallet. The observation uses the normal phase
read cut. Its quantities do not count held objects.

Prices and quantities are script or content data. Stack capacities come from
the authored item catalog. The native quantity core scans the request lines and catalog. The adapter
resolves each ID against the current catalog. Its 24 slots bound the observation.

## Rollback, checkpoints, and saving

Native application and its actual history are synchronous. Failed preparation
leaves no partial cost or reward. Bag changes carry their source room and tick
for room replay. The generic action mailbox and script records rewind through
the normal host path.

Checkpoint history records the authority of each change. Narrative grants
keep their existing fact-retention rule. Script exchanges follow the
participants who retain their source room. A restore drops exchanges whose
source room is rebuilt and keeps those whose room remains owned. A checkpoint
commit banks the retained changes into its baseline. Currency-only exchanges
need no invented item identity.

The existing durable mirror saves stored quantities and the local primary
body's wallet. Game restore policies, including developer starting unlocks,
still apply. It does not add NPC wallet persistence. Action tickets belong
to the live caller and session; they are not a durable one-time reward claim.
For a contacted destructible, use [contact consumption](contact-consumption.md)
to commit object state and exchange terms together. Do not imitate that
transaction with separate flag and inventory requests.

## Measured cost

A local native-only probe on 2026-11-18 ran five samples of 40,000 exchanges
in the cached development build. Each exchange spent one quantity and granted
one other quantity; the reverse exchange restored the starting state. Median
cost was 0.674 microseconds per exchange, with a 0.671–0.690 range. This includes
bag validation, copies, and receipt allocation. It excludes ID resolution, Rhai, history
publication, rendering, rollback, and asset startup. Measure the assembled rule
before choosing a backend for a costly mechanic.

## Scope and remaining work

This service does not change equipment or held-item custody. Their native
owners remain responsible: a body's custody of an item goes through `take` and
`release`, and equipping an owned item through `equip` and `stow`; each
reports its own [results](action-results.md#installed-command-results).
The current bag also retains its fixed 24-slot catalog. General catalog growth
remains open in the
[capability census](../planning/engine/script-first-capability-census.md).

The loaded [exchange fixture](../../game/ambition_app/tests/scripted_inventory_exchanges.rs)
uses a controlled actor and an NPC, checks actual application, retry identity,
full-stack and unknown-item refusal, observations, GGRS replay, and save/load.
The checkpoint witness also drives an ordinary fall/death restore under
GGRS and checks the banked bag and wallet. Owner tests cover duplicate costs,
exact capacity, overflow, checkpoint retention, and catalog identity round trips.
