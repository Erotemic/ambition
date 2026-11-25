# Item custody and accounting

**2026-10-26 object extension:** Item occurrence and custody contracts are useful starting
points for persistent carried/placed objects, but weapon-specific `GroundItem` and held-item
combat are not by themselves the general prop ontology. Evaluate a solver-neutral object
identity and transfer contract before consolidating item crates or making Avian mandatory; see
[ADR 0038](../../adr/0038-persistent-objects-and-optional-physics.md) and [architecture
reassessment](architecture-reassessment-and-composable-multiplayer.md).
**State:** OPEN / NARROW. Physical custody, the instance/count boundary and
persistent occurrence behavior across residency and restore are built and
guarded. The atomic durable-horizon write is done (2026-11-21). This page owns A7
(separate item custody and accounting from lifecycle orchestration) in the
[frontier](actor-monolith-work-frontier.md).
The follow-up at `f41218979625` selects sufficient item recipes, native
continuity, and one exchange rule for shops and procedural callers below.

## Scope

This page is the **pressed** half of items: a `GroundItem` taken with a
deliberate `Attack` press. That is `ambition_held_items`. The **touched** half
(a `WorldItem` you walk into) is `ambition_world_items`; see
`crates/ambition_world_items/MODULES.md`. Both are "items" and "pickups".
`ItemPickupSet` belongs to the pressed half only.
This page also owns the accounting boundary over stored quantities. A shop
exchange does not, by itself, transfer a held physical occurrence.

## Goal

Keep these concepts separate:

```text
item definition
    != physical item occurrence
    != fungible quantity
    != body inventory
    != participant entitlement/unlock
    != current custody/equipment
    != durable occurrence disposition
```

A body may hold or inventory an item without that fact becoming a
participant-wide entitlement. A participant may unlock a capability without a
particular physical manifestation being indestructible.

## Current shape

| Fact | Owner |
| --- | --- |
| `GroundItem`, `ItemCustody`, held specs, pickup/use/throw/physics/residency chain, `ItemPickupSet::CoreHeldItems` | `ambition_held_items` |
| `WorldItem` and the touched pickup | `ambition_world_items` |
| `OwnedItems` (session `Resource`), `Inventory { bag, in_hand }`, `ItemCategory` | `ambition_items` |
| The three-variant `ItemPickupSet` `.chain()`, `restore_custody_to_checkpoint`, `minted_horizon` | kernel (`ambition_platformer2d_items/src/pickup/`) |
| The held-item registry | one table, `HELD_ITEMS` (`ambition_characters::brain::action_set`), read by `held_item_by_id` and `held_spec_by_id` |
| Durable whereabouts | `AuthoredOccurrences` and `OccurrenceWhereabouts` (`shared_tangle/src/lifecycle/continuity.rs`) |
| The starter roster | one insert, in `AmbitionContentPlugin` (`game/ambition_content/src/plugin.rs`) |

- **The hand is the record.** `item_in_hand` projects a body's `HeldItem` to
  a catalog `Item`. The menu reads the primary player's
  hand. `Inventory` answers count, has and is-equipped over the bag and the
  hand. There is no process-global "equipped" mirror.
- **`GroundItem` is sealed.** It is `#[non_exhaustive]` with two constructors,
  `at_rest` and `released`. This is a component-construction seal. It does not
  give `ambition_held_items` authority over the occurrence: callers such as
  `drop_held_weapon`, the Smash bomb and mine, and the match spawn still mint
  the identity, room scope, provenance and attempt state.
- **Custody durability is a field.** `InCustodyOf` carries a required
  `CustodyDurability { Restored, SessionOnly }`. Item pickup and custody
  restore write `Restored`. Riders, limbs and possessions write `SessionOnly`.
  The durable custody rows read the field.
- **Death drops.** A death drop that becomes an object carries
  `SimId::death_drop(parent, kind)`. Drops that grant a quantity stay
  anonymous, because `OwnedItems` is their durable record.
- **Unloaded rooms.** A `Placed { room, at }` row crosses the durable horizon.
  Construction reinstates the item in the room it lies in and suppresses it
  elsewhere (`outlook_for`). A runtime mint enters the ledger in the tick it
  appears (`AuthoredOccurrences::admit_mints`).
  `republish_placements` refuses an id whose row is not `InCustody` or
  `Placed`, and returns the refusals `#[must_use]`. Position is kept at integer
  pixels.
- **The portal gun is an ordinary held item** (`portal_gun`, 2026-11-11). It
  lies in a room as a `GroundItem` (an LDtk `PortalGunSpawn` lowers to one),
  takes the occurrence road like the axe, and is put down with
  `Shield + Attack`. The portal crate knows only `PortalGenerator`: the
  portal adapter (`game/ambition_content/src/portal/inventory_adapter.rs`)
  makes whatever holds the gun a generator, and the gun lying in the world
  one too, so its pair's portals stay open. Q45 (entitlement behavior is
  acceptable during engine development) still covers the starter bag.
- **Seat-keyed entitlement.** A placed mine (`game/ambition_demo_smash/src/mine.rs`)
  keys detonation to a seat (`PlacedMine { owner_seat }`) while the object is
  an ordinary `GroundItem`. An opponent can hold it while it remains the
  placer's to set off.
- **The shared bag across rooms that rewind alone (CW2, 2026-11-21).** A
  cell grant or cell use on the implemented ordinary path is a proposal
  (`ambition_platformer2d_items::committed_bag`). The bag decides it once
  its tick is confirmed, and spends an allocated cell in the body's room at
  that tick. See [causal-world-commit](causal-world-commit.md#cw2--one-native-shared-item-operation-uses-the-committed-road).
  **Completion remains open at `3ed50e318392`:** queue refusal still falls
  through to eager grant, spend or refund. The
  [CW2 follow-up](causal-world-commit.md#cw2-follow-up--preserve-authority-under-pressure)
  makes authority classification precede admission and gives each allocation
  an identified, bounded settlement obligation. Do not extend the anonymous
  return or eager-on-refusal pattern to merchant and reward operations.
- **Possession moves the driver, not the goods.** `OwnedItems` is session
  scope and the hand is body scope, so possession transfers nothing.

## Writer census (A7)

```bash
python3 scripts/measure/measure_state_writers.py --domain item --sites
```

Every number is a lower bound. A text scan cannot see a rollback codec, a
`serde` restore, or a helper that takes `&mut T`. To enumerate one family
reliably, seal the type and let `rustc` list the callers.

The finding: the monolith's checkpoint-baseline family has no writer outside its
own crate. `OwnedItems` is written from four crates, and most of its sites are
outside `ambition_items`. The separation A7 needs is between the crate that
defines inventory and the crates that schedule writes to it, not between session
and item. Custody is already where A7 wants it: nearly all writes are in
`ambition_held_items`.

Do not build a generic item-request bus to reduce the minting count to one.
Centralize occurrence minting only where it centralizes a real invariant
(identity, custody, provenance, rollback ownership).

### Occurrence creators and why each owns its mint (read 2026-11-21)

Found by every production `SpawnOrigin::Dynamic` site and every runtime
`GroundItem` or pickup spawn outside construction. The restore roads
(`minted_horizon.rs`, `publication/src/stage.rs`) rebuild from a
`MintedItemDescription`; they create nothing.

| Creator | Site | Identity | Provenance | Why it owns the mint |
|---|---|---|---|---|
| Authored placements | construction and publication (`AuthoredOccurrences`) | authored | `Authored` / `ProviderStaged` | the room's record is what rebuilds it |
| Death drops (coin, heart, ability pickup, weapon parcel) | `ambition_platformer2d_items::loot::spawn_death_drop`, the one road all four drop sites call | anonymous, except the weapon (`SimId::death_drop`, derived from its parent) | `Dynamic { parent, sequence }` | a quantity drop is settled by `OwnedItems`; a weapon becomes a carried object, and every durable road that gives it back is keyed by `SimId` |
| A thrown inventory item | `ambition_held_items::throw_held_item_system` | `SimId::spawned(thrower, counter)` | `Dynamic` | throwing turns a quantity into an instance, and an instance owes an identity; the edge of the open `ItemCustody` inventory leg |
| A regrown authored item | `ambition_platformer2d_items::regrowth`, through `loot::grow_ground_item` | a child of the authored source and its growth sequence | `Dynamic` | the growth rule selects the source's physical recipe and creates a new occurrence; its canonical extent can differ from an inventory throw |
| Match item spawns | `ambition_platformer2d_items::match_spawn::spawn_match_items` | `SimId::match_spawn(match ordinal, ordinal)` | none | the match's rules own the table; the identity is derived from terms two peers agree on, with no counter |
| Sanic's scattered rings | `ambition_demo_sanic::scatter_rings_on_hit` | one `SimId` per ring | `Dynamic { parent: the player }` | the game's ring-loss rule: a quantity it turns into instances it can take back |
| A summoned minion | `ambition_platformer2d_construction::summoned_minion_request` | spawned | `Dynamic` | an actor, not an item: construction owns it |

Reward policy only consumes outcomes: `collect_ecs_pickups` reads a pickup's
origin to record its `GrantSource` (`Mint { parent }` or `Authored`), and a
chest is an interactable whose opening grants what its encounter's outcome
names. Neither spawns an item occurrence.

One asymmetry: match items carry an identity and no `SpawnOrigin`, so a
provenance-keyed reader cannot see them. That is stated at the site as
session-only (2026-11-21): they end with the match's session, and no
checkpoint saves a match. Items are off in Smash by Jon's call.

The original census did not establish that a retained mint description is
sufficient. The [IR1 review](item-recipes-and-live-continuity.md#ir1--retain-the-recipe-chosen-at-mint)
finds that a description loses the creator's canonical extent, and both
restore roads substitute 36 × 36. Regrown coconuts and ordinary weapon drops
provide different original sizes. Retain the recipe at its existing creator;
do not centralize all creators merely to fix reconstruction.

## Invariants

- One live physical occurrence has one authoritative custody/disposition.
- Custody transfers are explicit and atomic at the item/body domain boundary.
- Room unload does not silently delete a persistent occurrence.
- Body despawn follows an explicit drop, transfer or retention policy.
- Pickup may merge into fungible accounting only when identity and provenance
  no longer matter.
- Drop or rematerialization mints or restores an occurrence by item policy, not
  by fabricating an unrelated replacement (entitlement items excepted, Q45).
- Save/load persists only relationships the durable road can reconstruct.
- Rollback reproduces live custody without becoming the durable save format.
- Participant entitlement and physical custody are not inferred from each other
  in the simulation. `MenuAction::Equip` grants custody from the roster; that is
  accepted for the demo inventory under Q45.
- An object the attempt reclaims (`SpawnedThisAttempt`) is not durable. An
  object made durable stops carrying `SpawnedThisAttempt`.

## Lifetime matrix (2026-10-22)

Each item lifetime, its owner and its phase. Paths: `AM` is
`crates/ambition_platformer2d_actor/src`, `HI` is
`crates/ambition_held_items/src/lib.rs`, `ST` is
`crates/ambition_platformer2d_shared_tangle/src`, `RT` is
`crates/ambition_platformer2d_runtime/src`. Durable families: **H** is
`set_durable_horizon` (occurrences and custody, `AM/session/durable_horizon.rs`,
`DurableHorizonSet::SessionMirror`), **M** is `set_minted_items`
(`AM/items/pickup/minted_horizon.rs`, `DomainMirror`), **I** is the inventory
(`AM/items/persist.rs`, `DomainMirror`).

| Lifetime | Owner | Phase | State written | Durable |
| --- | --- | --- | --- | --- |
| Authored room baseline | `construct_authored_ground_item` (`AM/construction/mod.rs`), planned by the room feature plan (`AM/features/ecs/spawn/mod.rs`) | Plan in `Update` (room-transition load); built at the commit, `RoomTransitionSet::Apply` | `GroundItem`, `SettledItem`, `ItemCustody::InWorld`; no occurrence row until first carried | H, once moved |
| Runtime mints | Producers: `throw_held_item_system` (`HI`), `drop_held_weapon` (`AM/features/ecs/damage_drops.rs`); admission `record_placed_ground_items` (`HI`); baseline `capture_minted_item_baseline` | `HeldItemStep::Throw`; `ResidencyStep::Record`; `CheckpointCapture` | `AuthoredOccurrences` (mint flag), `MintedItemBaseline`, `BagSpendsSinceCheckpoint` | M, and H's mint flag |
| Inventory | `OwnedItems` (`ambition_items`); capture, checkpoint restore and mirror in `minted_horizon.rs` and `persist.rs` | `CheckpointCapture`, `CheckpointDomainApply`, `DomainMirror`, `DurableRestoreSet::Domains` | `OwnedItems`, `OwnedItemsBaseline`, the primary `BodyWallet` | I |
| Equipment | Sim: `pickup_held_item_system` / `equip_held_spec`, `return_released_items` (`HI`). Menu and script: a stamped `ItemEquipRequested` / `ItemStowRequested` (`apply_menu_action`, `game/ambition_app/src/menu/effects.rs`), applied in the sim by `apply_hand_requests` → `change_hand` (`HI`) | Pickup and Release steps; the hand's requests on the tick they are stamped for | `HeldItem`, `ItemCustody::Held`, `InCustodyOf{Restored}` | H custody rows, for a hand backed by a ground item |
| Stock loss | `spend_fighter_stocks` (`crates/ambition_combat/src/stocks.rs`); elimination despawn in Smash | `FighterStocksSpent` | Stocks, health, markers; no item or bag state | none |
| Same-room replay | `apply_committed_checkpoint_restore` (`AM/session/checkpoint.rs`): `restore_owned_items_to_checkpoint`, then `restore_custody_to_checkpoint`; `retract_mints_of_retracted_boss_defeats`; `retire_the_previous_attempt` | `CheckpointRestore`, `CheckpointDomainApply`, `ContentRoomReplayResetSet` / `ReplayContent`, `RoomReset` | `OwnedItems`, wallet, `ItemCustody`, `HeldItem`, `AuthoredOccurrences`, grants and spends | H, M, I at the next mirror |
| Room exit | The commit's resident sweep (`RT/room_transition/commit.rs`); ledger producers in `HI` and `ST/lifecycle/continuity.rs`; `record_bodies_away_from_home` | `RoomTransitionSet::Apply`; residency steps | Residents despawned; carried items cross with their holder; Placed/InCustody rows | reads M at a door crossing |
| New session / durable restore | Fresh: `start_the_item_domain_fresh`, `begin_fresh_run` (`AM/session/reset/mod.rs`). Load: `adopt_occurrence_checkpoint_from_save`, `restore_inventory_from_save`, `complete_durable_restore` | `CheckpointDomainApply`; `DurableRestoreSet` Lifecycle → Domains → Complete | All baselines, `AuthoredOccurrences`, `SaveRestored`; a fresh run replaces the save | reads H, M, I; a fresh run clears them |

Gaps the matrix shows:

- **The save write is two setters in two sets (was three, 2026-11-21).** I is
  written in `DomainMirror`; H and M together in `SessionMirror`, by one call
  that keeps a minted row only beside its occurrence row (the atomic durable
  horizon row below).
- **Match items have no road back.** `spawn_match_items`
  (`AM/items/match_spawn.rs`) gives an item a `SimId` but no `SpawnOrigin`,
  so the minted baseline and mint admission do not see it.
- **Stock loss has an item owner (2026-11-17).** An item whose holder no
  longer exists is put back into the world by `return_released_items` where
  its body was stored when it was taken, under gravity. A death that mints
  its body's weapon (`drops_held_item`) mints nothing for a body that carries
  an object, so the object is never doubled.
- **Equipment has one road (re-read 2026-11-21).** The menu's Equip and a
  script's `equip` are stamped requests the sim applies through
  `change_hand`, so a rewind replays them. A hand holding a bag item has no
  occurrence and is session-only by statement: the bag is saved and the hand
  is equipped again from the grid (`ambition_platformer2d_items::persist`).
- **A door crossing reads M from the save**, not from `MintedItemBaseline`,
  so re-entering a room that holds an unattended mint depends on the save
  mirror (confirmed by the `f41218979625` source review; no new runtime run).
  [IR2](item-recipes-and-live-continuity.md#ir2--read-native-item-continuity)
  moves this reader and dormant-drop retraction to the native recipe owner.
- **Durable restore has two roads**: the in-sim `DurableRestoreSet` chain and
  the candidate road (`CandidateDurableHorizon`), which leaves out
  `OwnedItemsBaseline` deliberately.

## Open work

| Item | Work | Acceptance |
| --- | --- | --- |
| Atomic durable horizon | **Done 2026-11-21.** `set_minted_items` is gone. `set_durable_horizon(occurrences, custody, minted)` writes all three and keeps a minted row only when its occurrence is among the rows. The item domain still owns what a minted row means (`MintedItemRows`, a system parameter in `minted_horizon.rs`); the session's `persist_occurrence_horizon_to_save` asks it for the rows and writes them in the same call, behind the same change key. Witness: `no_minted_row_names_an_occurrence_the_save_does_not_hold` (persistence) | One write; the guard becomes a property of the writer, not only a fixture test |
| Sufficient mint recipe | **IR1 extent slice done 2026-11-22:** the selected extent is retained and both reconstruction roads consume it; keep the [receipt](item-recipes-and-live-continuity.md#ir1-first-slice-done-2026-11-22). Native end-to-end reconstruction and other retained fields remain open | A non-default-size occurrence preserves identity and canonical geometry through both restore roads |
| Native recipe and continuity authority | **Done IR2/IR3 (2026-11-24):** typed recipes read while entities are dormant; live pose kept in the body; captured at checkpoint commit, operation admission and room retirement ([IR3 done](item-recipes-and-live-continuity.md#ir3-done-2026-11-24)) | One moving item does not decode dormant recipes or rebuild their save rows each tick; checkpoint, exit and rewind retain their selected cuts |
| Held exit context | **Planned REL4 with IR2/IR3:** retain the placement used if a holder disappears, including its chart; rebuild it and explicitly admit its transfer policy | A missing held mint does not later release at zero; crossing does not reinterpret source coordinates in the destination |
| Scoped custodian handoff | **Planned REL4 with A10:** retain the old live occurrence and use accepted reconstruction/succession to resolve its replacement | One publication cannot strip a same-ID, same-spec holder in another live room; legitimate replacement still works |
| Durable output selection | **Done IR4 (2026-11-24):** preserve prompt semantic writes and coalesce pose-only writes with a finite persistence interval | Continuous movement does not trigger full-file output each frame; confirmation, save ownership and checkpoint policy stay intact |
| Shop/native exchange | **Planned below:** one pure check and apply rule over the selected bag and wallet | Subject-bound offer checks and accepted operations share eligibility; exact payout and refusal leave no partial changes |
| `durably_held` width | **Decided 2026-11-21: it stays the component.** It matches `ItemCustody`, so it holds items lying on the ground too. An `InCustody` row for such an item only reaches the file while the ledger says `InCustody`, and the item domain can rebuild both variants, so the width costs nothing a load cannot answer. Narrowing it to `Held` drops rows, the direction that loses an item | Decided explicitly; dropping a save row is the dangerous direction |
| Lifetime matrix | **Done 2026-10-22**, above | Each row names its owner and phase |
| Orphaned holder | **Done 2026-11-17**: the item comes back where it was taken (`return_released_items`); a carrying body's death mints no weapon copy (`FeatureHitWriters::carries_an_object`). Witness: `an_item_whose_holder_is_gone_comes_back_where_it_was_taken` | An item outlives its holder in exactly one stated way |
| Match items | **Done 2026-11-21: stated session-only** at `spawn_match_items`. The item is spawned into the match's session and ends with it, and no checkpoint saves a match, so it has identity and no `SpawnOrigin` | A match item is either minted (described) or stated session-only |
| Menu equip | **Done (re-read 2026-11-21).** The menu writes a stamped `ItemEquipRequested`; `apply_hand_requests` applies it in the sim through `change_hand`, the road a script's `equip` takes. A bag-backed hand is stated session-only in `ambition_platformer2d_items::persist` | One equip road, or the menu's stated as session-only |
| Item carve boundary | See below | The proofs below |
| Agent surface | **Built 2026-11-21, over a save file:** `cargo run -p ambition_app_tools --bin item_report -- [--save PATH] where ID \| list [--room R] [--body B] \| audit \| explain ID`. It reports an occurrence's disposition, the body holding it, a mint's provenance and definition, and the bag's counts; `audit` names the rows a load cannot honour (custody without its row or held while placed, a mint with no minted row, a duplicate row) and exits 1. An authored item's definition is its room's record, which the report names rather than reads. A live-world view is not built | Reports definition, stack identity, occurrence, custody owner, provenance and disposition |

### Held state must retain the next legal exit

**Planned, source `00bf92c22142`:**
[REL4](relationship-endpoints-and-transitions.md#rel4--retain-exit-context-and-authorize-endpoint-succession)
adds a continuation obligation beside the completed orphan and IR1 receipts.
Missing held-mint reconstruction uses a zero body position because hand pose
supplies the current view. Orphan release later uses that stored position.
A crossing likewise changes the carried item's room without changing that
fallback. Preserve same-room return-where-taken behavior, retain checkpoint
exit context, and prepare an explicit destination fallback at accepted
cross-room transfer. This is not another per-frame pose projection.

The same packet qualifies publication handoffs and checkpoint custodian
resolution by their admitted population. Bare authored ID lookup across all
live rooms can select a same-definition twin. A retained old `Entity` alone
cannot follow reconstruction either; native succession must authorize the
new endpoint. Keep handoffs publication-owned and discard only the refused
publication's effects. Spec equality is not evidence of occurrence identity.

### A7 exchange — one rule for an offer and its accepted exchange

**Landed 2026-11-24 (receipt at the end of this section); planned at source
review `f41218979625`, 2026-11-21.**
`ambition_items::transaction::InventoryExchange` already owns atomic stored
quantity and wallet exchange. `shop::{buy,sell}` implement separate laws,
while merchant options ask still other questions. The current paths disagree:

| Customer | Current result | Shared domain rule needed |
| --- | --- | --- |
| Buy a Health Cell with enough money but a full stack | `can_afford(8)` opens the option; purchase refuses | Check the complete offered exchange, including capacity |
| Sell a Gun-Sword that is held but absent from the bag | `inventory.holds` can open the option; sale cannot remove a stored quantity | Check the quantity source that the sale actually consumes |
| Sell a cell for 4 with a wallet at `i32::MAX - 2` | Shop removes the cell, credits only 2 and reports `Sold` | Exact payout or refusal before either owner changes |

`inventory.holds` is a valid general possession question. It includes
physical hands and can see another driven body's held item. Do not change
its meaning to make it a sale check. A physical-object sale would need an
explicit custody exchange; that is not what today's bag sale does.

#### Shared calculation, separate acceptance cuts

Lower buy and sell into the existing `InventoryExchange` terms. Keep a thin
shop terms constructor and the current narrative request transport. Catalog
spelling resolves at its current adapter: `item_by_dialog_id` for dialogue,
`item_by_id` for portable calls. Capacity and exact payout stay in the pure
item owner.

Expose the item-and-price check through a typed shop binding over that
owner, or deliberately extend the existing prepared-call adapter. The current
generic Yarn `condition(id, arg)` accepts one domain argument and cannot
carry both terms. Do not encode them into a query string or add an offer
registry to work around that adapter limit.

Extract one private calculation behind two public operations:

- `check(catalog, bag, wallet)` reads current state and returns the existing
  `ExchangeRefusal` or success. It publishes no change.
- `apply(catalog, bag, wallet)` runs the same calculation against its current
  borrowed inputs, then immediately installs the result and returns
  `ExchangeApplied`.

The calculation can retain today's small fixed-bag copy. The preview does
not return a later commit token or reserve currency. Acceptance always
evaluates current state again. Bind the payer through
[O6a](authored-gameplay-logic-and-orchestration.md#o6a--bind-the-merchants-question-and-action-to-the-same-subject);
a correct exchange against the wrong wallet still gives the wrong offer.

Preserve the exchange order: validate and total repeated terms; require
payment before receipt; remove item costs before grants; require every grant
to fit. Do not net terms first. One owned item cannot fund a cost of two just
because the same operation grants two back. Five coins cannot fund a payment
of ten just because ten will later be received.

Keep `ItemChange { removed: u32, added: u32 }`. Catalog stack limits are `u32`
and have no unlimited sentinel. A signed `i32` quantity delta would narrow
that domain. Merchant supply is currently unlimited because neither road
has a merchant-stock leg; this packet adds no stock system.

#### State the policy changes

This migration is not behavior-equivalent in every old corner case. The
proposed policy follows the existing exchange law:

| Case | Selected policy |
| --- | --- |
| Payout overflows the wallet | Refuse the whole sale; no partial credit |
| Free grant to a body whose wallet is in debt | No positive coin cost means no funds requirement |
| Negative price | Refuse at the shared price boundary; remove the native sale's implicit clamp to zero |
| Reserved/placeholder catalog slot | Refuse through the existing `is_real` rule |
| Multiple reasons to refuse | Use the exchange owner's deterministic refusal order; do not keep a second shop order |

Both Yarn buy/sell commands already reject negative authored prices. The
native signed-price path needs the same explicit conversion before unsigned
exchange terms. Keep whole-coin wallet semantics, including supported debt;
this is not an actor resource meter.

#### Keep history ownership with the caller

`platformer2d_items::narrative::apply_shop_transactions` records
`BagChangeRetention::NarrativeFacts`. The world-facing
`platformer2d_items::inventory::apply` records `RoomOwners` and has different
scope requirements. Share the pure exchange and actual change receipt, not
the complete world-facing wrapper. Each adapter keeps its provenance,
retention, application timing and row grouping.

The shared bag still has the [CW2](causal-world-commit.md) migration. This
packet neither completes causal isolation nor adds another eager write path
on admission refusal. A future committed merchant operation uses the same
pure rule inside its selected authority boundary.

**Remove:** independent shop buy/sell arithmetic, the duplicate `ShopTx`
outcome after callers use `Result<ExchangeApplied, ExchangeRefusal>`, and
narrative before/after reconstruction of changes the receipt already names.
Replace merchant wallet-only and possession-only offer guards with the
subject-bound exchange check. Keep the general questions for other callers.

**Acceptance:** reuse the two-seat merchant witness from O6. Add the concrete
full-stack, held-only item and exact-payout refusal cases to the existing
exchange/merchant fixtures. Change state between preview and consume and
check that acceptance re-evaluates it. Verify that both history adapters keep
their current retention and that no failed exchange changes either owner.
No new global offer registry, cache or economy framework is needed.

#### A7 exchange landed (2026-11-24)

- **One calculation.** `InventoryExchange::check` and `apply` share one
  private `calculate` (validate and total; payment before receipt; costs
  before grants; every grant fits; never netted). `check` publishes nothing
  and reserves nothing.
- **The shop lowers to it.** `ShopTransactionRequested::exchange` (buy: pay
  the price, receive one; sell: give one, receive the price) feeds `check`
  and `apply`, which return `Result<ExchangeApplied, ExchangeRefusal>`. The
  separate `shop::{buy,sell}` arithmetic and `ShopTx` are gone. The policy
  changes above hold: a payout past the wallet refuses the whole sale, a
  negative price is refused (`invalid_quantity`), a full stack refuses the
  purchase (`stack_full`).
- **History stays with the caller.** `apply_shop_transactions` records its
  `NarrativeFacts` rows and room log from the receipt's `ItemChange`s, not
  from a before/after count.
- **The offer asks the same exchange of the same payer.** The typed shop
  binding is two Yarn functions, `can_buy("id", price)` and
  `can_sell("id", price)`, over `narrative::shop_offer(world, payer, ..)`,
  with the payer from `shop_payer` (O6a). The merchant nodes in `kernel.yarn`
  and `pirate_port.yarn` use them in place of `can_afford(price)` and
  `condition("inventory.holds", ..)`; those general questions are unchanged
  for their other callers. `dialogue_lint` reads the new guards and also
  checks the guard's item against the deal's.

Witnesses: `shop::tests::the_offer_and_the_deal_are_one_exchange` (full
stack, exact payout, negative price, a change between preview and deal
that the deal re-evaluates) and the two-seat merchant witness
`a_merchant_deals_with_the_seat_whose_conversation_it_is`, extended with
the offer asked of each payer. A body holding an item the bag does not
store cannot sell it: `shop_offer` reads the bag the sale consumes.

### Item carve boundary

Startup checkpoint restore belongs to session, not to
`ItemPickupSimulationPlugin` (A1b). A checkpoint-only composition resumes
without the item domain
(`a_checkpoint_only_composition_resumes_without_the_item_domain`). Do not move
it back with items, and do not move the room/session slot into
`shared_tangle`. `restore_custody_to_checkpoint` stays a kernel system that
reads the item crate's components: it is checkpoint policy, not item policy.

A later item carve identifies pickup verbs, held-item custody, equipment
capability and minted accounting separately. One generic item service that
receives all world, control and session context would make the crate smaller
without reducing what a change must know.

Prove: item-absent checkpoint restore, two-body interaction, duplicate pickup
suppression, failed construction and consumption, durable occurrence restore,
and rollback. A source guard that forbids an import is supplemental.

## Open design questions

- When do stack merge and split preserve provenance?
- What is the policy for unique-item destruction, recovery and reset?
- What is authoritative or predicted for item custody in online multiplayer?
- "Unique" names two properties. `ItemCategory::is_unique()` is a stacking
  property (all non-consumables clamp at 1). An entitlement's uniqueness is a
  lifecycle property (acquired once, never revoked). Do not classify items in
  terms of the one word.
- Q141 is ruled: durability is per-item and authored, and a runtime-spawned
  item may be durable when authored so.

## Relationship to session and possession

Possession does not make a physical item participant-owned. If a possessed body
carries an item through a room transition, the item travels because its custody
policy says so. A generic live relationship component is not written to the
durable save because another domain uses the same vocabulary. Persist it only
when the durable road can restore the relation.

## Checkpoint and construction boundary

Items own occurrence, custody, entitlement and minted-history semantics. Session
owns the save/replay boundary at which those domains restore or persist. Session
does not copy item fields itself. An owner installer registers known lifetime
work against published phases, without a universal callback service.

Atomic custody transfer means the accepted transition is indivisible to its
consumers. It does not make the raw-Commands construction interface undoable.
Failed construction and consumption have an explicit order and regression. Do
not grant or spend an occurrence because a construction request was made.

## Procedural callers and residency handoff

Modules request acquisition, transfer, cost or custody through the owning
transaction. No extension-owned inventory mirror exists. A submitted request is
not a receipt of acquisition. A paid action uses the owning compound
admission/reservation rule.

[Open-world residency](open-world-runtime-and-residency.md) requires exactly one
writer while a live occurrence becomes dormant or live again. A durable item
reference does not force a body to stay resident. A candidate reconstruction
attempt is not a new persistent identity. FI5/FI9 in
[acceptance](fast-iteration-acceptance.md) exercise refused and accepted
transfer and replay between submission and acknowledgement.

## Forbidden regressions

- No second held-item registry.
  `every_catalog_item_with_a_held_form_resolves_in_the_one_registry` enumerates
  the population from `Item::ALL`.
- No struct-literal `GroundItem` outside `ambition_held_items`.
- No process-global "equipped" mirror.
- A restore that displaces an item keeps the spec it displaced; it does not
  re-derive the spec from an id.
- An authored field that reaches a runtime representation must have a
  consumer, or be deleted. A comment that promises a consumer is worse than an
  unread field.
- A negative claim ("nothing authors this") needs an untruncated scan of every
  spelling (struct literal and assignment).
