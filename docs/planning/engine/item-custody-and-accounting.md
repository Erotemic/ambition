# Item custody and accounting

**The writer census this page's packet held for is delivered:**
[`item-writer-inventory.md`](item-writer-inventory.md) (re-run 2026-09-17, **64**
write-capable sites; 61 on 2026-09-10). ⇒ Its headline is that the shape is the
INVERSE of A7's framing — the monolith's checkpoint-baseline family is 10 of 10
inside its own crate, while `OwnedItems` is written from four crates with 3 of 18
sites in the crate that defines it. ⚠ Both of those ratios were 9 of 9 and 3 of
16 a week ago: occurrence and custody did not move at all, and the inventory
ratio got WORSE, which strengthens the finding rather than dating it. And
`GroundItem` had no constructor, so seven sites minted an occurrence; it is
`#[non_exhaustive]` with two constructors as of `7108a57b1`.

**State:** OPEN / NARROW — and as of 2026-09-04 the exploration side of every
migration row is CLOSED. I1 (2026-09-02) and I4 are done; I2's exploration half
is done; I3 is a maintainer DECISION rather than an implementation
([Q45](../awaiting-maintainer-decision.md)). ⇒ **Re-derived 2026-09-17: what is
left on this page is two maintainer decisions and one repair.** The decisions are
Q45 and [Q141](../awaiting-maintainer-decision.md) — may a runtime-spawned ground
item ever be durable — which I4 had claimed was filed and was not. The repair is
folding minted items into `set_durable_horizon` so the write is atomic rather
than guarded.

⚠ The residual this line used to name — a "fighter call site" at
`match_spawn.rs:113` — is gone: it was the WIDE-versus-NARROW registry lookup,
and I2b left one registry, so the call is correct and only its comment was
describing a distinction that no longer exists. Corrected in place, with the
same stale comment at `features/ecs/spawn/mod.rs:750`.

⛔⛔ **THE THIRD ITEM WAS FALSE AND HAD BEEN FOR SOME TIME — re-derived 2026-09-06.**
This sentence used to end *"and the gauntlet-drop road's missing end-to-end arm,
which is blocked on `force_kill_boss` producing no drops — also fighter side."*
The arm EXISTS:
`boss_lifecycle::a_defeated_boss_drops_its_signature_gauntlet_on_the_real_kill_road`
delivers a real `HitEvent` in a vulnerable phase and asserts the gauntlet lands as
a pick-up-able `GroundItem`. Run today: **ok, 1.21 s.**
⚠ And `force_kill_boss` producing no drops is TRUE — it writes HP to zero and never
enters `apply_boss_hit`'s `killed` branch. That fact is why the test was written;
the row recorded it as the reason the test could not be. **The blocker and the
motivation were the same sentence, and the row kept the wrong half.**

⭐ **A filed BLOCKER is structurally different from a filed MEASUREMENT: a
measurement is the output of LOOKING, a blocker is the output of STOPPING.** So a
blocker is written at the moment its author knew least about it, and is then
quoted rather than re-derived. ⇒ re-deriving the blockers is the first step of
picking up a deferred row, not a courtesy. (The fighter lane closed D-CUT-VOICE the
same day by doing exactly this: three blocking claims, all three false.)

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
ITEMS** — noticed while measuring I3, and worth fixing when [Q45](../awaiting-maintainer-decision.md) is
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
answering Q45 is what makes them precise.

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

## ✔ AUTHORED FIELDS CARRIED END TO END WITH NO CONSUMER — CLOSED 2026-09-17

⭐⭐ **ALL FOUR ARE DELETED.** A sweep of every field `spawn_static.rs` threads
found four that reached a runtime representation and stopped there. Three went on
2026-09-12 and the fourth on 2026-09-17:

| field | what it claimed | what actually decided it |
|---|---|---|
| `InteractableSpec.requires_facing` | this interactable must be faced | nothing — it could be used from behind |
| `PickupSpec.collected` | this pickup is already taken | the `ambition_combat::components::Collected` marker |
| `ChestSpec.persistent` | the save system remembers this chest | `encounter_reward_looted_flag`, which never read it |
| `BreakableSpec.debris_cue` | which debris/SFX a break emits | `emit_breakable_destroyed`, which writes `PhysicsDebrisCue::Breakable` and `WORLD_CRATE_BREAK` as literals |

⛔⛔ **THE RULING THEY WERE WAITING ON WAS NOT THE ONE BLOCKING THEM.** Each was
recorded as *"wiring it or deleting it is a design call"* and routed to
[Q63](../awaiting-maintainer-decision.md). That framing kept three no-op
fields alive in ONE crate. **Deciding what the future feature should do is a
different question from making the field impossible to misuse**, and only the
first was ever blocked on anything. The questions themselves stay open on Q63;
the false capabilities do not wait for them.

⚠ **THIS IS THE STRANDED CATEGORY, not "dead" and not "restraint."** The value
was authored, validated, threaded through construction and stored, and only the
last hop was missing. A census of unused symbols cannot see it — every hop has a
caller — and the dormancy census
(`scripts/authored_parameter_modes.py`) cannot either, because the field IS named
in content. Finding them took reading the consumer side, one hit at a time.

⭐ **AND ONE OF THE FOUR HAD A CONSEQUENCE A PLAYER COULD SEE**, which is why
"bookkeeping" is not a safe default reading: `requires_facing` was set by content
(`game/ambition_content/src/bosses/cut_rope/victory.rs`) and read by nothing, so an interactable that declared it
must be faced could be used from behind. `persistent: false` on a chest and
`collected: true` on a pickup changed nothing; `debris_cue` was never authored at
all, in any format, which makes it the weakest of the four and the last found.

⛔ **THE CHEST AND THE PICKUP CARRIED PROSE ASSERTING A CONSUMER.** The chest's test
comment said it defaults true *"so the save system records them automatically."*
That is the expensive part: a field with no reader is cheap, and a COMMENT
promising a reader is what makes the next author build on it.

⇒ Sibling check, and it is why the table is four rows and not five: `pogo_refresh`
is the same shape and IS read (`features/ecs/damage/mod.rs:611`, `:978`,
`damage_predicates.rs:52`, `target_volumes.rs:122`, `features/ecs/world_overlay.rs:58`), so it
is merely dormant — never set true by content — not stranded. The difference took
reading the consumer, which is the only way to tell them apart.

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

> **Re-derived against `9df7991e1` (2026-09-17). Every number below is a
> measurement taken that day, not a claim carried forward.**

⭐ **THE POPULATION IS 22 IDS IN ONE TABLE.** `HELD_ITEMS`
(`crates/ambition_characters/src/brain/action_set/mod.rs:210`, read by
`held_item_by_id` at `:569`) is the whole registry, and
`ambition_held_items::held_spec_by_id` / `held_spec_for_item` are pass-throughs
to it. ⇒ **the CLAIM is the durable part and the number is not**: this population
moves whenever content lands a held item, so re-derive it rather than quoting
this line.

⛔⛔ **AND DO NOT RESTATE THE SHAPE EITHER.** This row first said the population
was NINE — the count `Item::held_item_id()` answers, which is the item catalog's
view and not the resolver's — and the true figure was 21, then 22 the next day
when the map gained `polygon_mine`. Having caught that, the row closed with what
it called the permanent half: *"there are TWO registries and consulting one
silently halves the answer."* The second table was DELETED the same day — see
I2b. ⇒ A restatement can survive re-measurement because nobody has yet tried to
REMOVE the thing it describes; correctly identifying which half of a row is
durable is not the same as that half being true.

⛔ **THE GAUNTLET ABILITIES ARE NOT A SEPARATE CLASS.** The row's hypothesis was
that an ability may have no world form. It does: 14 of the 22 ids are authored as
`GroundItem` placements under
`game/ambition_map_assets/ambition_content/worlds/` — 16 placements, all of them
in `sandbox.ldtk`, with the two demo worlds authoring none. That set includes all
seven boss signature gauntlets (`volley`, `meteor`, `beam`, `shockwave`,
`vortex`, `sentry`, `dive`) and the four gauntlet abilities `blink`, `grapple`,
`mark_recall` and `bomb`.

⚠ The other eight — `admiral_gun_sword`, `axe`, `fireball`, `gun_sword_heavy`,
`javelin`, `polygon_bomb`, `polygon_mine`, `polygon_ponytail` — are an AUTHORING
fact and nothing more. Two of them used to be a construction refusal wearing a
design decision's clothes: `authored_ground_item_requests` refuses an unknown id
with `UnknownHeldItem`, and `axe`/`javelin` were built in a crate it could not
reach. Since I2b they resolve, pinned by
`a_room_may_author_the_weapons_that_used_to_live_in_the_other_registry`. The rest
reach a hand through a moveset's `equips` or the menu/grant road, not off the
ground.

### I2b — there is ONE held-item registry now — ✔ CLOSED 2026-09-05

⭐⭐ **THE TWO-REGISTRY FACT THIS PAGE HAS BEEN RESTATING SINCE I2 IS NO LONGER
TRUE, because the second table is DELETED.** `axe` and `javelin` were built in
`ambition_held_items` while the other twenty ids were rows in
`ambition_characters` — and `ambition_held_items` DEPENDS on
`ambition_characters`, so every consumer upstream of it could reach only the
narrow half. `held_spec_by_id` existed to paper over that, and its own doc said
*"Consulting one alone silently loses half the items"* — a rule no upstream crate
was able to follow.

⇒ **The repair was not a better resolver. Both were ROWS, not code:**
`axe_spec()` and `javelin_spec()` were plain `HeldItemSpec` literals that needed
nothing from the item catalog, so they moved into the one table. ⓘ And
`gunsword_spec()` never needed an arm at all — it was already
`held_item_by_id("gun_sword")`, so the doc's "three wired weapons" was two.

**What that closed, beyond the tidiness:**

- I2a's defect class is gone at the root — no upstream consumer can resolve
  through a half-registry, because there is no half.
- ⭐ **An authored `GroundItem { held_item: "axe" }` now RESOLVES.**
  `authored_ground_item_requests` refuses unknown ids with `UnknownHeldItem`, and
  it asks the narrow table — so before today an author who placed an axe in a
  world got a construction refusal. That is why this page could say `axe` and
  `javelin` "reach a hand only through the menu/grant road": not a design
  decision, a consequence of the split.

⚠ Guard: `every_catalog_item_with_a_held_form_resolves_in_the_one_registry`
enumerates the POPULATION from `Item::ALL` and requires the unresolvable
remainder to be empty, rather than naming the two ids — asserting "axe resolves"
would pass again the moment somebody re-split the tables for a third item.
Poison-verified by deleting the axe row: it fails naming `["axe"]`.

### I2a — the brandish restore destroyed an unresolvable weapon — ✔ CLOSED 2026-09-05

⭐ **THE TWO-REGISTRY CLAIM ABOVE HAD A CONSUMER, and it was losing items.**
`MoveBrandishedItem.previous` stored the displaced item's ID, and the restore
resolved it through `ambition_characters::brain::held_item_by_id` — the NARROW
registry — removing the body's `HeldItem` when that returned `None`. MEASURED:
that table holds 20 ids and `axe`/`javelin` are in neither of them; both are
built by `held_spec_for_item`. ⇒ a body carrying an axe and playing a move that
brandishes had the axe DELETED rather than handed back, and by I1 the hand is
the record, so it was not in the bag either.

⛔ **THE WIDE RESOLVER COULD NOT BE CALLED FROM THERE.** `held_spec_by_id`
consults both, but it lives in `ambition_held_items`, which DEPENDS on
`ambition_combat`. The split is forced by a dependency edge, so no care at the
call site could have fixed it — which is the general lesson: when a resolver is
downstream of its caller, "remember to use the wide one" is not available as a
rule.

⇒ **The lookup is GONE rather than corrected.** The body HAD the spec; storing
its id and deriving the spec back was a second authority for *what this body was
holding*, and the derivation was the half that could fail. `previous` is now the
spec, the restore is infallible, and `None` recovers its honest meaning (the
body was carrying nothing) instead of doubling as *the lookup failed*.

⛔⛔ **MY FIRST LATENCY ARGUMENT WAS FALSE, corrected 2026-09-05 by
YardratAmbition.** I wrote *"no shipped move authors `equips`; the only
non-`None` writer is a test helper"*. **TWO shipped movesets author it** — the
pirate admiral's side-B (`pirate_admiral_moveset.rs:354`, Jon's own 2026-08-27
design, whose comment says *"THE DRAW is `MoveSpec::equips`"*) and Projectile
Polygon's (`projectile_polygon_moveset.rs:469`).

⚠ **THE METHOD IS THE LESSON.** Both write `side_b.equips = Some(..)` — an
ASSIGNMENT — and I grepped for `equips: Some`, the struct-literal form. My
follow-up scan would have caught them and was cut off by a `head -10` before it
reached `game/ambition_content`. ⇒ A NEGATIVE claim ("nothing authors this")
cannot be made from a truncated scan or from one spelling of an assignment.

✔ **The fix is unaffected and neither equipped id was ever at risk**:
`polygon_ponytail` and `admiral_gun_sword` are both in the NARROW table, so the
DRAW resolved either way. The destruction path was only ever `previous`, the
DISPLACED item.

◐ **What safety actually rested on, which is far more fragile than "nobody
authors it":** nobody who authors `equips` is holding a WIDE-registry item at the
moment they press. In `ambition_demo_smash` that is structural today (it authors
no `HeldItem` at all). In the platformer — where `axe` and `javelin` exist and
the admiral is a placed NPC — it is UNMEASURED, and this row does not claim it.
⇒ Closing it while it was cheap was right for a better reason than the one I
gave.

⚠ The probe still hashes the id alone, so no checksum moved and no schema
version bumped. Guard: `a_carried_weapon_no_registry_knows_is_returned_and_not_
destroyed`, which uses an id NO registry answers to — pinning it with `axe` would
pass again the moment somebody added `axe` to the narrow table, fixing one weapon
and leaving the shape.

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
⛔ **`held_spec_by_id` consulted two registries and its own comment said
"consulting one alone silently loses half the items" (ONE registry as of
2026-09-05, I2b — the sentence is kept because the reasoning below was done under
it). No arm had ever run on a spec only the SECOND one knew** — the boss gauntlets (`volley`, `meteor`,
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
which is reached from one call site (`damage/mod.rs:858`), so
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

⭐⭐ **MEASURED 2026-09-02, re-checked 2026-09-17 — every road below is still
spelled the same way — and the row's own escape clause — *"when that model can
express their semantics"* — is the whole answer. It cannot, because the portal
gun is not an occurrence.** Read side by side:

| | ordinary held item | portal gun |
|---|---|---|
| in the world | `GroundItem` with a `SimId` | `PortalGunPickup`, no identity |
| pickup | `HeldItem` + `ItemCustody::Held { holder }`; the object PERSISTS | pickup entity despawned; `PortalGun { active }` on the body, repertoire refolded for that hand; `owned.grant(Item::PortalGun, 1)` |
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
is a unique capability item an ENTITLEMENT or an OCCURRENCE? — is
[Q45](../awaiting-maintainer-decision.md).
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
answers about the same object. Filed as
[Q141](../awaiting-maintainer-decision.md) on 2026-09-17.
⚠ This row said "question 51" for thirteen days, and Q51 is the boss-reward
durability boundary — an unrelated question. The question I4 describes had never
been filed at all. A route to a wrong number reads exactly like a route to a
right one; checking the route means reading the TARGET, not the number.

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
  ⓘ **The population is ONE, measured 2026-09-05 and re-derived 2026-09-17 with
  the same split.** The catalog holds 24 items (Ability 7, Weapon 6, KeyItem 5,
  Consumable 5, Reserved 1 — counted off `ITEM_META`'s rows, one per `Item`) and
  exactly one — the portal gun — takes the ENTITLEMENT road: `equip_portal_gun` /
  `unequip_portal_gun` (`crates/ambition_held_items/src/lib.rs:996`, `:1016`,
  `#[cfg(feature = "portal")]`), with `OwnedPortalGunPair` deliberately
  outliving the hand. The other 23 are ordinary held items on the occurrence
  road, which is the one measured above as a complete write/read/build round
  trip.
  ⇒ So this is not *"classify 24 items"*; it is *"decide what the second one
  does"*, and it is cheap while there is no second one. Filed as
  [Q45](../awaiting-maintainer-decision.md).
  ⛔ ⚠ **And "unique" already names two different properties, which will confuse
  any classification made in its terms.** `ItemCategory::is_unique()` is
  `!matches!(self, Consumable)` (`crates/ambition_items/src/lib.rs:42`), so all 19
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
  WRITE    session/durable_horizon.rs:556  `Placed { room, at }` crosses the durable
                                           horizon UNCONDITIONALLY — "a fact about the
                                           world itself"; only `InCustody` is filtered,
                                           to a hand the file can reconstruct
  READ     session/durable_horizon.rs:188  restored to `OccurrenceWhereabouts::Placed`
  BUILD    lifecycle/continuity.rs:205     placed in THIS room → `Reinstated { at }`;
                                           placed elsewhere    → `Suppressed`
           ⚠ line numbers re-derived 2026-09-17; the three roads are unchanged
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
| entitlements (`OwnedItems`) | a Bevy **`Resource`** (`crates/ambition_items/src/lib.rs:530`) | the SESSION — one per world, not per body |
| the physical hand | per-body components; *"the hand is read where it lives"* | the BODY |

  ⇒ **Possession moves the DRIVER, not the goods.** `OwnedItems` never belonged
  to a body, so possessing one cannot transfer it; the held item stays on the
  body it is attached to, because that is where it is. Neither
  `control/possession.rs` nor `control/authority.rs` names
  `OwnedItems` at all — checked, not assumed.

  ⭐ **And this is [Q45](../awaiting-maintainer-decision.md)'s split
  visible at the STORAGE level.** An entitlement is session-scoped and an
  occurrence is world/body-scoped, and the portal gun is exactly the item that is
  BOTH — `Item::PortalGun` in the global `OwnedItems` and a `PortalGun` component
  on the body. ⇒ That is not a bug; it is why the question is a question. The
  ruling decides which of the two storages is the AUTHORITY when they can
  disagree, and today they cannot, because dropping never revokes the grant.
- What is authoritative/predicted for item custody in online multiplayer?


## Two writers of the durable horizon — RE-MEASURED 2026-09-17, and the code moved under this section

⭐ **The invariant is written down on `set_durable_horizon`** (`ambition_persistence/src/save_data.rs:583`):
it takes occurrences and custody TOGETHER because *"a custody row without its occurrence row
names nothing"*. `PersistedMintedItem` carries the same `occurrence` key and still has NO
equivalent protection — its own setter (`set_minted_items`, `:598`), its own production
writer (`items/pickup/minted_horizon.rs:383`), separate from the writer that owns occurrences
and custody (`session/durable_horizon.rs:625`). Each compares only its OWN field before
writing, so nothing reconciles them, and no reader of `occurrences()` cross-checks either.
**That half is unchanged.**

⛔⛔ **THE FILTER TABLE THIS SECTION CARRIED IS NOW WRONG, AND IT WAS THE PREMISE.** It said
the minted writer applies *"`SpawnOrigin::Dynamic` — no custody check at all"*. Measured at
HEAD: `live_minted_descriptions` (`minted_horizon.rs:433`) runs over
`Query<(&SimId, &SpawnOrigin, &GroundItem, &ItemCustody), With<RoomScopedEntity>>`, so
`ItemCustody` is REQUIRED to be in the population at all — the same component
`durably_held` matches. ⇒ The two filters gate on the same custody component by two
spellings, one `With<>` and one a fetched tuple member, so the divergence this section was
named after does not exist in the code today.

⚠ **A REQUIRED QUERY MEMBER IS A FILTER THAT DOES NOT LOOK LIKE ONE**, which is how it was
read as absent: the `SpawnOrigin::Dynamic` test is visible in the body and the custody
requirement is in the signature. Read the whole query, not the `filter_map`.

✔ **AND THE SENTENCE THIS SECTION DEFERRED WAS LANDED AS A FIELD, which is better.** It
ended *"`InCustodyOf` has TWO producers with different durability. Nothing marks that
difference at the marker itself … worth a sentence at
`project_custody_onto_authored_occurrences` if anyone touches it, and is not worth a change
today."* The marker now STATES it: `CustodyDurability { Restored, SessionOnly }`
(`shared_tangle/src/lifecycle/markers.rs:25`) is a required field on `InCustodyOf` (`:54`)
with no `Default`, so a third producer cannot inherit a durability by omission. Item pickup
and custody restore write `Restored`; `body_custody.rs:111` writes `SessionOnly` for riders,
limbs and possessions, exactly the reading traced here.

⇒ **The custody rows now ask the RELATION, not the subject's domain**
(`durable_horizon.rs:608`): `custody.durability == CustodyDurability::Restored`. The
OCCURRENCE rows above it still ask `restorable` — `With<ItemCustody>` — and the code says why
in place: the two run over different populations (rows versus live entities), and an
occurrence recorded `InCustody` whose entity carries no `InCustodyOf` is kept by the wider
marker and would be DROPPED by the field. **Dropping a save row is the dangerous direction.**

⚠ **The component-versus-variant observation this section recorded is now a comment at the
query** (`durable_horizon.rs:521-529`), and it cites this page. `durably_held` matches
`ItemCustody` the COMPONENT, and `ItemCustody::InWorld` is a variant, so the set also holds
items lying on the ground — wider than the filter comment's *"a hand it can reconstruct"*.
Wider drops FEWER occurrence rows, so it cannot strand anything; narrowing it to `Held` is a
durability decision, not a tidy-up.

⛔ **THE ORIGINAL QUESTION, kept because the reasoning is the useful part:** can an
occurrence be `InCustody` while its holder carries no `ItemCustody`? The orphan needs that
state. `InCustodyOf` is inserted for held items by `ambition_held_items` (`crates/ambition_held_items/src/lib.rs:716`), the
crate that owns `ItemCustody`, so a minted ITEM carries both. The subject that has
`InCustodyOf` without `ItemCustody` is a carried BODY — `project_body_custody` writes it
`Without<GroundItem>` because *"the item domain owns its custody projection"* — and dropping
it is correct: the loader does not put a rider back on a mount, so a durable row claiming
somebody holds that body is a claim the file cannot honour. ⇒ Two custody projections
writing ONE marker with different durability is the design, and since 2026-09-06 the marker
says which is which.

✔ **A guard exists meanwhile** —
`no_durable_row_names_an_occurrence_the_save_does_not_hold` (`ambition_persistence/src/save_data.rs:1262` <!-- cite-test: a named guard, and the sentence says it is a unit test -->,
poison-verified) — but it is a unit test over a fixture, not a production check. The repair
that would remove the class is folding minted items into `set_durable_horizon` so the WRITE
is atomic rather than guarded; that is still not done, and it is what remains of this
section.

## Checkpoint and construction integration boundary

Packet A1 in the [frontier](actor-monolith-work-frontier.md) removes checkpoint
startup/reset restoration from shrine/item installation. Items still own item
occurrence, custody, entitlement and minted-history semantics. Session owns the
save/replay boundary at which those domains restore or persist; it must not
implement the item transition by copying fields itself.

Before A7, write the lifetime matrix for authored room baseline, runtime minted
occurrences, inventory, equipment, stock loss, same-room replay, room exit and
new-session/durable restore. A generic persistence event does not answer which
of these survives. An owner installer can register known lifetime work against
published phases without a universal callback service.

Item custody's atomic transfer means the domain's accepted transition is
indivisible to its consumers. It does not extend the raw-Commands construction
interface into an undoable transaction. Failed construction and consumption must
have an explicit order and regression; do not grant or spend an occurrence just
because a construction request was made.

## Procedural callers and residency handoff

Modules request acquisition, transfer, cost or custody through the existing owning
transaction; no extension-owned inventory mirrors are introduced. A submitted
request is not a receipt of acquisition. A paid action uses the owning compound
admission/reservation rule, not two independent requests that can disagree.

[Open-world planning](open-world-runtime-and-residency.md) requires exactly one
writer while a live occurrence becomes dormant or becomes live again. A durable
item reference does not force a body/entity to stay resident. Preserve occurrence,
holder and checkpoint scope; a candidate reconstruction attempt is not a new
persistent identity. FI5/FI9 in [acceptance](fast-iteration-acceptance.md) exercise
refused/accepted transfer and replay between submission and acknowledgement.
