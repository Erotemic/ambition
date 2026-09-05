# Item custody and accounting

**State:** OPEN / NARROW — and as of 2026-09-04 the exploration side of every
migration row is CLOSED. I1 (2026-09-02) and I4 are done; I2's exploration half
is done and its one residual (`match_spawn.rs:113`) belongs to the fighter lane;
I3 is a maintainer DECISION rather than an implementation (question 45). What is
left on this page is that decision, that one fighter call site, and the
gauntlet-drop road's missing end-to-end arm, which is blocked on
`force_kill_boss` producing no drops — also fighter side.

⛔ **So a reader should not take "OPEN" as "unbuilt".** Physical custody, the
instance/count boundary and persistent occurrence behaviour across
residency/restore all exist and are guarded; the openness is two decisions and a
test-reachability problem, and each is named in its own row below.

> ⭐ **THIS PAGE IS THE PRESSED HALF, and since 2026-09-02 that is a crate
> boundary rather than a distinction in prose.** Items split by COLLECT TRIGGER:
> a `GroundItem` taken with a deliberate `Attack` press — everything below — is
> `ambition_held_items` — ⚠ this sentence said *"`actor_monolith::items::pickup`,
> which stayed in the kernel"* until 2026-09-03, and the pickup carve made that
> false the next day; a `WorldItem` you merely walk into is
> `ambition_world_items`, carved out by `69641a83f`. ⛔ BOTH HALVES ARE CRATES
> NOW, and what stayed in the kernel is only the plugin that composes them plus
> `restore_custody_to_checkpoint`. ⚠ The
> two are easy to conflate from either side and the vocabulary does not warn
> you: both are "items", both are "pickups", and `ItemPickupSet` belongs to the
> pressed one alone. ⇒ Nothing on this page is about the touched collectible; if
> the question is "what happens when a body walks over it", the answer is in
> `crates/ambition_world_items/MODULES.md`, which carries the same orientation
> from the other side.
>
> ⭐ **UPDATED 2026-09-03: THE PRESSED CARVE IS DONE.** The paragraph above said
> the pressed half "stayed in the kernel" and that its carve had stopped at a
> schedule-ownership fork — true when written, and answered the next day.
> `ambition_held_items` now owns `GroundItem`, `ItemCustody`, the held specs and
> the pickup / use / throw / physics / residency chain, and its plugin configures
> `ItemPickupSet::CoreHeldItems` end to end. What the kernel keeps at
> `items/pickup/` is the RESIDUE: the three-variant `.chain()` — an edge that
> orders sets owned by two other crates, so neither owner can name both sides —
> plus `restore_custody_to_checkpoint`, `minted_horizon`, and the shrine /
> puppy-slug-gun / match-spawn systems that attach to the domain's steps.
>
> ⚠ **So "custody" now spans a crate line, which is the thing to hold onto when
> reading below.** The types are `ambition_held_items`'s; the CHECKPOINT POLICY
> over them is the kernel's, deliberately — it is checkpoint policy, not item
> policy, and the carve checklist says it must be stated wherever it appears or
> the next reader "fixes" it by dragging the function after the domain.

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

A body may physically hold or inventory an item without that fact becoming a
participant-wide entitlement. A participant may know/unlock a capability without
a particular physical manifestation being indestructible.

## Settled ownership

The body owns physical inventory/equipment/capabilities that describe that
body. Participant-owned knowledge/keys/theorems and possession-transfer policy
are separate authorities with different lifetimes.

Do not reopen `OwnedItems` as an undecided global authority. It is migration
pressure where one process-global representation duplicates body-local held or
equipped state and cannot represent several independently driven bodies.

## Current item partition

Most catalog item classes are naturally fungible counts. The difficult residual
is the held weapon/ability population that currently participates in both worlds:

- there is a physical instance with provenance/custody that can be dropped,
  carried or moved between rooms;
- there is also durable accounting/availability represented as a count or
  entitlement.

That dual role is legal. The architecture should make the transition explicit
rather than pretending every item is purely a stack or purely a unique ECS
entity.

## Current invariants

- one live physical occurrence has one authoritative custody/disposition;
- custody transfers are explicit and atomic at the item/body domain boundary;
- room unload does not silently delete a persistent occurrence;
- body despawn follows an explicit drop/transfer/retention policy;
- pickup may merge into fungible accounting only when item identity/provenance no
  longer matters;
- drop/rematerialization mints or restores an occurrence according to item
  policy, not by fabricating an unrelated replacement;
- save/load persists only relationships the durable road can reconstruct;
- rollback reproduces live custody state without becoming the durable save
  format;
- participant entitlement and physical custody are not inferred from each other.

⚠ **TWO OF THESE ARE WRITTEN AS UNIVERSALS AND HOLD ONLY FOR OCCURRENCE-MODEL
ITEMS** — noticed while measuring I3, and worth fixing when question 45 is
answered rather than guessed at now:

- *"drop/rematerialization … not by fabricating an unrelated replacement"* — the
  portal gun's drop spawns a fresh `PortalGunPickup` unrelated to the token that
  was consumed. Under the ENTITLEMENT reading nothing is fabricated in place of
  an occurrence, because there is no occurrence; under the OCCURRENCE reading it
  is exactly what the invariant forbids;
- *"participant entitlement and physical custody are not inferred from each
  other"* — ⛔ and this one is looser than it looks for EVERY item, not just the
  gun: `MenuAction::Equip` grants physical custody straight from the roster
  (`equip_portal_gun`, or `held_spec_for_item` → `equip_held_spec`), which is
  inferring custody from entitlement unless the invariant is read as being about
  the SIMULATION only.

⇒ Neither is a defect to fix today; both are the same under-specification, and
answering question 45 is what makes them precise.

⭐⭐ **AND THE LAST INVARIANT GAINED A SHIPPED CASE WHERE IT GENUINELY DIVIDES
(2026-09-05), which is the strongest evidence it is a real distinction and not an
unexercised sentence.** The fighter lane's placed mine
(`game/ambition_demo_smash/src/mine.rs`) keys DETONATION ENTITLEMENT to a SEAT —
`PlacedMine { owner_seat: usize, .. }`, deliberately not an `Entity` — while the
object itself is an ordinary `GroundItem` that anyone can pick up. ⇒ **an
opponent can be holding the mine while it remains the placer's to set off.**
Physical custody moved; entitlement did not; neither was inferred from the other.
⚠ Re-derived here by reading `mine.rs`, not taken from the report: the owner
field is a `usize` seat and the detonation test is `mine.owner_seat ==
owner_seat`.

ⓘ Note the shape it shares with the boss-cleared split recorded on
[`simulation-authority-and-determinism.md`](simulation-authority-and-determinism.md):
one object, two facts, and the temptation is to unify them because the types line
up. Custody answers *"who is holding this"*; entitlement answers *"whose is
it"*. A held item whose owner is elsewhere is exactly the case that makes the
difference visible, and until this it had no shipped instance.

## ⛔⛔ AUTHORED FIELDS CARRIED END TO END WITH NO CONSUMER (2026-09-05)

ⓘ **THE FULL POPULATION LIVES ON [question 63](../awaiting-maintainer-decision.md),
not here — FOUR fields, found by sweeping every field `spawn_static.rs` threads
(13 distinct, nine of them read).** This section keeps the three that are
custody- and interaction-shaped, because they are what this program owns; the
fourth (`BreakableSpec.debris_cue`, whose value is hardcoded at the consumer) is
breakable VFX and belongs to the decision rather than to this page. ⇒ one
population, one home, and this row cites it rather than carrying a second copy
that will disagree.

Found by investigating hits of `scripts/authored_parameter_modes.py` one at a
time rather than publishing its list. Both are in `ambition_interaction`, both
are the SAME SHAPE, and neither is dormant-in-the-census sense:

| field | authored | threaded to | read by |
|---|---|---|---|
| `ChestSpec.persistent` → `Chest.persistent` | ✔ | `spawn_static.rs:108` | **nothing** (one test asserts the default) |
| `PickupSpec.collected` → `Pickup.collected` | ✔ | `spawn_static.rs:79` | **nothing** |
| `InteractableSpec.requires_facing` → `Interactable.requires_facing` | ✔ (content sets it) | `spawn_static.rs:231` | **nothing** |

⭐ **THREE NOW, AND THE THIRD IS THE ONE WITH A CONSEQUENCE A PLAYER COULD SEE:**
`requires_facing` is set by content (`cut_rope/victory.rs`) and read by nothing,
so **an interactable that declares it must be faced can be used from behind.**
The other two are bookkeeping; this one is a rule that does not exist.
⚠ Its sibling `pogo_refresh` is NOT in this table and was checked: it IS read
(`features/ecs/damage/mod.rs:592`, `:938`), so it is merely dormant — never set
true by content — not stranded. The difference took reading the consumer, which
is the only way to tell them apart.

⇒ **ONE FACT, TWO REPRESENTATIONS, and in the first two cases the live one is elsewhere.**
Whether a pickup was collected is the `Collected` MARKER COMPONENT
(`ambition_combat::components::Collected`, rollback-registered as
`feature.collected`, inserted and queried by `pickups.rs`). Whether an opened
chest is remembered is `encounter_reward_looted_flag`, a per-encounter save flag
that never consults `Chest.persistent`. ⇒ **setting `persistent: false` on an
authored chest changes nothing today.**

⛔ **AND BOTH CARRIED PROSE ASSERTING A CONSUMER.** The chest's test comment said
it defaults true *"so the save system records them automatically"*. That is the
expensive part: a field with no reader is cheap, and a COMMENT promising a reader
is what makes the next author build on it. Both corrected in place.

⚠ **This is the STRANDED category, not "dead" and not "restraint"** — the value
is authored, validated, threaded through construction and stored, and only the
last hop is missing. A census of unused symbols cannot see it (every hop has a
caller) and the dormancy census cannot either (the field IS named in content).
It took reading the consumer side.

ⓘ **Not deleted here.** Removing them changes the authored spec schema, and
whether the intent is per-chest persistence and per-pickup collected state is a
design question. Recorded so the answer is made rather than inherited.

## Remaining migration pressure

### I1 — body inventory replaces process-global equipped mirrors — ✔ CLOSED 2026-09-02

`OwnedItems::equipped` is gone. It was a process-global mirror of "some body
holds X", written by every equip road (`equip_held_spec`, the portal-gun twins,
the checkpoint restore) and read by the menu — and four seats could not share
it: seat two picking up a gun-sword marked it equipped in seat one's menu. Now
the hand IS the record: `ambition_held_items::item_in_hand(held, portal_gun)`
projects a body's `HeldItem` / active `PortalGun` to the catalog `Item`, the
menu reads the PRIMARY player's through `menu::effects::PrimaryHand`, and
`ambition_items::Inventory { bag, in_hand }` is the one view that answers
count / has / is_equipped over both — `OwnedItems::count` is the bag alone.
`inventory.holds` (the authored condition) asks the bag and then any player or
driven body's hand. Guards: `another_seats_weapon_is_not_the_primary_players_
equipped_item`, `a_wielded_weapon_with_no_stored_copy_is_owned_and_stowable`,
`a_weapon_in_the_players_hand_is_held_with_nothing_in_the_bag`; the two
persistence-authority tests read the grid's count through the projection. No
schema bump: the clone snapshot lost a field the session checksum never saw.

### I2 — held weapon/ability occurrence continuity

> **Verified against `06b25ee87` (2026-09-04).**

⭐ **RE-MEASURED 2026-09-04, and both of this row's own numbers were wrong — in
opposite directions.**

⛔ **THE POPULATION IS 21, NOT NINE.** `Item::held_item_id()` answers `Some` for
nine rows, but that is the ITEM CATALOG's view and the road's resolver is
`held_spec_by_id`, which consults the catalog AND
`ambition_characters::brain::held_item_by_id`. Its own doc comment says why:
*"both registries, in that order, because there are two … Consulting one alone
silently loses half the items."* `HELD_ITEMS` holds the bulk; `axe` and
`javelin` are built separately in `held_spec_for_item`
(`ambition_held_items/src/lib.rs:802`, `:820`). This row consulted one registry —
the exact failure that comment warns about.

⚠ **RE-DERIVED 2026-09-05: it is 20 + 2 = 22 now, and it was 19 + 2 = 21 when
written — the map gained `polygon_mine` the same day.** ⇒ **the CLAIM is the
durable part and the number is not**: this population moves whenever content
lands a held item, so re-derive it rather than quoting this line. The claim that
does not move is that there are TWO registries and consulting one silently
halves the answer.

⛔ **AND THE FOUR GAUNTLET ABILITIES ARE NOT A SEPARATE CLASS.** The row's
hypothesis was that an ability may have no world form. It does:
`sandbox.ldtk` authors `GroundItem` placements whose `held_item` is `blink`,
`grapple`, `mark_recall` and `bomb`, and two shipped fixtures already use
`ground_grapple` as their subject. A census of authored `GroundItem` entities
across `game/ambition_map_assets/ambition_content/worlds/` gives **14 distinct
`held_item` values**, including all seven boss signature gauntlets (`volley`,
`meteor`, `beam`, `shockwave`, `vortex`, `sentry`, `dive`). ⚠ `axe`, `javelin`
and `fireball` are authored nowhere, so those three reach a hand only through
the menu/grant road.

⛔ **"WHETHER THE ROAD PRESERVES IDENTITY ACROSS THE FIVE OPERATIONS IS
UNMEASURED" WAS WRONG. Every operation has a live arm:**

```text
pickup            a_thrown_item_is_the_same_object_that_was_picked_up
room transition   carried_item_crosses_rooms.rs (7 arms, incl. possession + re-entry)
drop              a_grab_press_while_holding_drops_the_item_where_the_body_stands
save / load       a_weapon_in_your_hands_is_still_in_your_hands_after_a_load
replay            canonical_reconstitution.rs:716
```

⇒ **What is unmeasured is the POPULATION those arms run on**, and re-measuring
2026-09-04 says exactly which axis matters. It is not "more items". Every one of
the five arms runs on a spec the ITEM CATALOG knows: the authored axe and
gun-sword, the menu-minted javelin, and — checked, because it looked like the
exception — the room-transition arm's subject, which is `blink_run_pickup ->
blink`, a row in BOTH registries and therefore dual-resolvable.
⛔ **`held_spec_by_id` consults two registries and its own comment says
"consulting one alone silently loses half the items". No arm had ever run on a
spec only the SECOND one knows** — the boss gauntlets (`volley`, `meteor`,
`beam`, `shockwave`, `vortex`, `sentry`, `dive`) are `HELD_ITEMS` rows with no
`Item` row at all, so `Item::from_held_item_id` answers `None` for every one.
✔ One operation covers that path now:
`a_boss_gauntlet_banked_at_a_checkpoint_returns_to_the_hand_that_banked_it`
restores a `volley` through `held_spec_by_id`, so a regression that dropped the
brain-registry arm would redden it.
✔ **SAVE/LOAD CLOSED 2026-09-04 for a catalog-unknown spec.**
`a_gauntlet_the_item_catalog_never_heard_of_is_still_in_your_hands_after_a_load`
drives the whole durable road across a process boundary: a real boss kill mints
the gauntlet, the pressed pickup takes it, a shrine rest puts the MINTED
description in the save file, and a fresh harness boots with that file and finds
it in the hand. Poison-verified on the claim the test names — delete
`held_spec_by_id`'s `.or_else(held_item_by_id)` arm and the hand comes back empty
(`got []`), which is the only failure it can produce.
⛔ **AND THE RESIDUAL IS NOT "THE OTHER THREE OPERATIONS" — I had that wrong the
same day I wrote it.** The registry only matters where a spec is REBUILT FROM AN
ID, and pickup, room transition and drop never do that: the pickup reads the
`GroundItem`'s own `spec`, a carried object crosses a door as an ENTITY, and a
drop re-derives custody from the hand. Widening those three by registry would be
test theatre.
⇒ **`held_spec_by_id` has three production callers, and that is the real
population:**

```text
items/pickup/mod.rs:309         restore_custody_to_checkpoint's minted arm  ✔ covered
                                (by BOTH the death restore and the save load)
features/ecs/spawn/mod.rs:594   a room BUILD reinstating a minted occurrence  ✔ covered
items/match_spawn.rs:113        a match's authored spawn table                ▢ fighter side
```

✔ **THE EXPLORATION SIDE OF THIS AXIS IS CLOSED (2026-09-04).**
`a_gauntlet_left_in_a_room_is_rebuilt_when_the_room_is` banks a gauntlet at a
shrine, puts it down, walks out the door and comes back, and the rebuild
reinstates it where it fell. ⭐ **The poison proves the two tests cover DIFFERENT
call sites, which is the whole argument for having both:** narrowing only the
room build's lookup to the catalog reddens that test alone — the save-load
gauntlet test and the other four in its file stay green.
⚠ That call site's own comment records the mirror-image failure it already fixed:
the NARROW `held_item_by_id` answered `None` for a javelin from the inventory and
"lost it a second time". Both arms of `held_spec_by_id` are now load-bearing at
this site with a test each side.
⇒ What remains is `match_spawn.rs:113`, and it belongs to the fighter side.

✔ **AND RE-MEASURING FOUND ONE REAL DEFECT, FIXED 2026-09-04: a death drop had
no identity, so the same object was an occurrence or not depending on how it was
acquired.** `drop_held_weapon` spawned a `GroundItem` with provenance and no
`SimId`, and every durable road that could give the object back is keyed by one
— `capture_minted_item_baseline`, `capture_custody_baseline`,
`TransactionBaseline::capture`. So a checkpoint taken while the player held a
boss's signature gauntlet had no description of it, and the `SpawnedThisAttempt`
sweep a death runs destroyed it with nothing able to rebuild it — while the
identical gauntlet authored as a room placement carried an identity all along.
Fixed by minting `SimId::death_drop(parent, "weapon")`, derived from
`(parent, kind)` exactly as the drop's provenance is. Guarded by
`only_the_death_drop_that_becomes_an_object_carries_an_identity`, which also
pins the other half of the rule: the three drops that grant a QUANTITY stay
anonymous, because `OwnedItems` is their durable record and an identity there
would be a second authority over it.
⛔ The gauntlet drop road has NO end-to-end coverage and the reason is worth
keeping: every boss drop is spawned inside `apply_boss_hit`'s `killed` branch,
which is reached from one call site (`damage/mod.rs:819`), so
`boss_lifecycle`'s `force_kill_boss` — writing HP to zero — produces no drops
at all. Filed on the fighter side.

The 21 held weapon/ability specs are the real customer. A pickup, room
transition, drop, save/load and replay should preserve whatever occurrence
identity/provenance the item's policy says matters while still supporting durable
quantity/entitlement accounting. Use canonical reconstitution rather than a
transition-specific rematerialization hack.

### I3 — portal-gun/special pickup convergence

Special pickup roads that despawn on pickup and manufacture a replacement on
drop should converge toward the same occurrence/custody model as ordinary held
items when that model can express their semantics.

⭐⭐ **MEASURED 2026-09-02, and the row's own escape clause — *"when that model
can express their semantics"* — is the whole answer. It cannot, because the
portal gun is not an occurrence.** Read side by side:

| | ordinary held item | portal gun |
|---|---|---|
| in the world | `GroundItem` with a `SimId` | `PortalGunPickup`, no identity |
| pickup | `HeldItem` + `ItemCustody::Held { holder }`; the object PERSISTS | pickup entity despawned; `PortalGun { active }` + `StashedActionSet` on the body; `owned.grant(Item::PortalGun, 1)` |
| drop | the same object returns to the ground | `unequip_portal_gun`, then `spawn_room_scoped(PortalGunPickup { … })` — a FRESH token |
| durable record | custody + the whereabouts ledger, rebuilt by `restore_custody_to_checkpoint` | `OwnedItems`, granted on pickup and **never revoked on drop** |

⇒ **The gun is an ENTITLEMENT with a cosmetic world token.** "Dropping" it is
unequip-plus-spawn-a-re-pickup, and it never loses you the gun: the menu
re-equips straight from `OwnedItems`
(`menu/effects.rs` → `equip_portal_gun`, no check that a token exists), and the
dropped token is room-scoped so leaving the room destroys it with no
consequence. The code says this out loud where it is decided — *"The gun is a
single item: it doesn't exist until you pick it up — picking up the one world
item IS getting the portal gun."*

✔ **AND THE ACCOUNTING IS SAFE, checked rather than assumed:**
`OwnedItems::grant` clamps a `is_unique()` category to 1, so the two roads
cannot inflate a count. What they can do is coexist — after a drop there are two
independent ways to hold the gun again (the menu, and the token), which is
harmless today precisely BECAUSE the gun is an entitlement.

⛔ **SO I3 IS A DECISION, NOT AN IMPLEMENTATION.** Converging the gun onto the
occurrence/custody model would give it an identity its semantics never use, and
would make "drop" mean something it does not mean for this item. The question —
is a unique capability item an ENTITLEMENT or an OCCURRENCE? — is recorded in
[`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md).
⚠ Whoever answers it should note that the two readings differ observably in one
place only: whether dropping the gun and walking away can ever lose it.

### I4 — unloaded-room disposition

Persistent dropped items and carried items in nonresident rooms need a durable
location/disposition that construction can reconstitute. This is jointly owned
with [`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md)
and [`construction-and-reconstitution.md`](construction-and-reconstitution.md).

⭐⭐ **MEASURED 2026-09-04, and the mechanism already exists — what was missing
is the rule about who may ENTER it.** `AuthoredOccurrences` is the durable
disposition I4 asks for: `OccurrenceWhereabouts::Placed { room, at }` is frozen
at the value it last held when the room unloads, `outlook_for` turns one row
into the two answers construction needs (the room it lies in reinstates, every
other room suppresses), and `durable_horizon.rs` serializes it. So a carried
item put down in a nonresident room IS reconstituted.

⛔ **THE POPULATION THAT IS NOT, and it is a class rather than a bug:** the
ledger has exactly ONE entry road — custody. An occurrence gets its first row
from `project_custody_onto_authored_occurrences`, reading `InCustodyOf`. So
anything that enters the world **already lying on the ground and is never picked
up** can never be remembered, no matter what identity it carries. The clearest
member is the death drop: `drop_held_weapon` spawns a fresh `GroundItem` with a
fresh `SimId::death_drop` and `RoomScopedEntity`, nobody has held it, the ledger
has no row, and leaving the room destroys it.

✔ **AND THAT IS CORRECT FOR THAT DROP, checked rather than assumed** — it also
carries `SpawnedThisAttempt`, so the attempt reset takes it back. An object the
attempt reclaims must not be durable, and the two answers agree. The defect was
that nothing SAID so, and nothing stopped a second producer from writing a
`Placed` row for something no hand ever carried.

✔ **LANDED: the entry rule moved from a producer's comment into the ledger.**
`republish_placements` now refuses any id whose current row is not `InCustody`
or `Placed` and RETURNS the refusals `#[must_use]`, so a caller cannot lose an
occurrence silently; `ambition_held_items` keeps only the half the ledger cannot
decide — whether a `Placed` row naming another room is a relocation or a stale
duplicate, which needs custody history a single call does not see. Guarded in
`lifecycle/continuity.rs` by three tests (a never-carried id is refused BY NAME,
a `Consumed` row is not resurrected, and both legal roads still pass so the
guard cannot pass by refusing everything); poison-verified by forcing the
predicate true — the two refusal tests fail, the three positive ones do not.

⇒ **WHAT REMAINS OF I4 is now one question, not a mechanism:** should a
runtime-spawned ground item ever be durable, and if so it needs a road into
custody or a second entry point stated as deliberately as this one — and an
object that gains one must stop carrying `SpawnedThisAttempt`, since "the
attempt reclaims it" and "the durable world remembers it" are contradictory
answers about the same object. Recorded as question 51 in
[`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md).

## Relationship to session and possession

Possession/control does not itself make a physical item participant-owned. If a
possessed body carries an item through a room transition, the item travels
because its holder/custody/lifetime policy says so.

A generic live relationship component must not be written to durable save merely
because another domain adopted the same relationship vocabulary. Persist it only
when the durable road can restore the relation.

## Agent-native surface

Useful inspection eventually includes:

```text
item where <occurrence>
item list --room <room>
item list --body <body>
item audit
item explain <occurrence>
```

The inspector should report definition, quantity/stack identity, physical
occurrence, custody owner, provenance and durable disposition without requiring a
reader to infer them from unrelated components.

## Open design questions — deliberately unresolved

- Which item classes are rematerializable entitlements versus lossable physical
  occurrences?
  ⓘ **The population is ONE, measured 2026-09-05.** The catalog holds 24 items
  (Ability 7, Weapon 6, KeyItem 5, Consumable 5, Reserved 1) and exactly one —
  the portal gun — takes the ENTITLEMENT road: `equip_portal_gun` /
  `unequip_portal_gun` (`ambition_held_items/src/lib.rs:955`,
  `#[cfg(feature = "portal")]`), with `OwnedPortalGunPair` deliberately
  outliving the hand. The other 23 are ordinary held items on the occurrence
  road, which is the one measured above as a complete write/read/build round
  trip.
  ⇒ So this is not *"classify 24 items"*; it is *"decide what the second one
  does"*, and it is cheap while there is no second one. Filed as
  [question 45](../awaiting-maintainer-decision.md).
  ⛔ ⚠ **And "unique" already names two different properties, which will confuse
  any classification made in its terms.** `ItemCategory::is_unique()` is
  `!matches!(self, Consumable)` (`ambition_items/src/lib.rs:43`), so all 19
  non-consumables clamp at 1 in `OwnedItems::grant` — you can never hold two
  axes. That is a STACKING property. The portal gun's uniqueness is a LIFECYCLE
  property: acquired once, never revoked, re-equipped from `OwnedItems`. One
  word, two meanings, and only one item has the second.
- When do stack merge/split operations preserve provenance?
- What is the policy for unique-item destruction, recovery and reset?
- What happens to a persistent dropped item in an unloaded room?
  ✔✔ **ANSWERED IN CODE — measured 2026-09-05, all three links verified.** It is
  remembered and reinstated where it lies:

```text
  WRITE    session/durable_horizon.rs:246  `Placed { room, at }` crosses the durable
                                           horizon UNCONDITIONALLY — "a fact about the
                                           world itself"; only `InCustody` is filtered,
                                           to a hand the file can reconstruct
  READ     session/durable_horizon.rs:130  restored to `OccurrenceWhereabouts::Placed`
  BUILD    lifecycle/continuity.rs:207     placed in THIS room → `Reinstated { at }`;
                                           placed elsewhere    → `Suppressed`
```

  The construction rule states itself in place: *"Lying in some OTHER room. Not
  alive — that room unloaded and took it with it — but not this room's to author
  either: it comes back when the room it is lying in is built, from the record
  this room holds."*
  ⇒ **Position survives at integer pixels** (deliberately — a float would cost
  the save's `Eq` and rewrite the file every frame on a NaN).

  ⭐⭐ **AND THIS IS THE WORKED EXAMPLE
  [question 38](../awaiting-maintainer-decision.md) SAYS ACTORS LACK.** That row
  offers *"stay where left"* for actors and warns that *"the producer and
  reconstruction consumer must land together; recording a moved placement that
  construction refuses would only add warnings and still teleport the actor
  home."* ⇒ For ITEMS both halves already exist and agree. Whoever rules 38 is
  not designing a mechanism — they are deciding whether actors join one that
  ships, and `continuity.rs`'s three dispositions are the shape to copy.
- How should possession transfer body inventory, equipment and participant
  entitlements?
  ⭐⭐ **THERE IS NOTHING TO TRANSFER TODAY, and the reason is a storage fact
  worth knowing before answering: the two layers have different scopes.**
  Measured 2026-09-05:

| layer | where it lives | scope |
|---|---|---|
| entitlements (`OwnedItems`) | a Bevy **`Resource`** (`ambition_items/src/lib.rs:529`) | the SESSION — one per world, not per body |
| the physical hand | per-body components; *"the hand is read where it lives"* | the BODY |

  ⇒ **Possession moves the DRIVER, not the goods.** `OwnedItems` never belonged
  to a body, so possessing one cannot transfer it; the held item stays on the
  body it is attached to, because that is where it is. Neither
  `abilities/traversal/possession.rs` nor `control/authority.rs` names
  `OwnedItems` at all — checked, not assumed.

  ⭐ **And this is [question 45](../awaiting-maintainer-decision.md)'s split
  visible at the STORAGE level.** An entitlement is session-scoped and an
  occurrence is world/body-scoped, and the portal gun is exactly the item that is
  BOTH — `Item::PortalGun` in the global `OwnedItems` and a `PortalGun` component
  on the body. ⇒ That is not a bug; it is why the question is a question. The
  ruling decides which of the two storages is the AUTHORITY when they can
  disagree, and today they cannot, because dropping never revokes the grant.
- What is authoritative/predicted for item custody in online multiplayer?
