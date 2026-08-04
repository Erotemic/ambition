# Proposal: a game may extend the AUTHORED vocabulary, not just the converter

**Status: PROPOSED — awaiting Jon's review. Nothing here is implemented.**

Raised by Jon, 2026-08-04: *"I would like to make maryo an ldtk level so I can
manually play with it and lay it out. That means it needs to have the right
entities exposed for it and make it easy to author in."* — and then: think about
what makes this hard vs easy, and whether an engine change would make it better.

This document answers that. **Short version: about two thirds of Mary-O 1-1 can
move to LDtk today with no engine change at all, and the remaining third is
blocked by one specific gap that is worth fixing properly.**

---

## 1. What actually makes it hard

Not LDtk. The blocker is that **Mary-O's runtime never reads the level** — it
re-derives every gameplay-significant thing from Rust constants, and the geometry
is only the shadow of those constants.

| System | How it finds its subject today | Survives Jon editing in the GUI? |
|---|---|---|
| ?-blocks | `power_block_id(i)` = `GeoId::tile_layer("mary_o_ground", 10+i)`, re-derived from `POWER_BLOCK_COLUMNS` (`lib.rs:810`) | ❌ an index into a Rust array |
| bricks | `brick_id(i)`, same shape | ❌ |
| quasar blocks | same, base ordinal 30 | ❌ |
| warp pipes | `pipe_halves()` — four hardcoded halves; the descent/ascent PAIRING is the tuple order (`lib.rs:310`) | ❌ |
| flagpole | a `FlagPole` **Resource** mirroring the authored block, *"so the sequence never has to search the world for it — the level knows"* (`flag.rs:40`) | ❌ |
| AI slop | `stair_slop_spawn_positions(player_spawn)` (`ai_slop.rs:192`) | ❌ derived, never authored |
| vault coins | a loop over `VAULT_COINS` from `vault_bounds()` | ❌ |
| ground / platforms / stairs / vault walls | `Block::solid_tiled(...)` from constants | ✅ pure geometry |

So a naive migration — move the blocks to LDtk, keep the Rust — moves the floor
and walls into the GUI, leaves everything that matters behind, and **silently
breaks the couplings**: `power_block_id(i)` names a `GeoSource::TileLayer` id,
and an LDtk *entity* gets `GeoSource::Placement` (`surfaces.rs:324`). The match
would simply stop firing. No error, no bonk, no wand.

**The one-sentence version: Mary-O's level is written in Rust not because nobody
got round to LDtk, but because the vocabulary she needs cannot be authored.**

## 2. The gap, precisely

The LDtk pipeline is **open at the converter layer and closed at the data
layer.**

A game can already register converters for its own entity identifiers —
`install_ldtk_entity_converters` (`conversion/mod.rs:688`), documented, tested,
and today used by **nobody**. But a converter's only output is `RoomEmission`
(`conversion/mod.rs:353`), whose ~20 channels are all engine-owned types, and the
one general channel — `placements: Vec<PlacementRecord>` — carries
`PlacementSchema`, a **closed engine enum** of six variants
(`placements.rs:416`): Hazard, Interactable, Pickup, Chest, Breakable, Portal.
`RoomSpec` and `Block` have no extension slot either.

So the door is open and the room behind it is engine-only. A game-specific
authored concept — *a block that contains a wand*, *a pipe that warps to **that**
pipe*, *the flagpole that ends the level* — has nowhere to land.

⚠ **This is also why the engine keeps being asked to absorb one game's nouns.**
The pressure that produced `PickupKind`, `ChestSpec` and friends is the same
pressure; the enum grows because it is the only place anything can go.

## 3. What is FREE today (and I would do this regardless)

More than I expected. With no engine change, LDtk can already express:

- **All terrain** — ground runs, one-way platforms, stairs, vault floor and
  walls — as an **IntGrid collision layer**, painted per cell.
  ⭐ and this is strictly *better* than today: IntGrid-derived blocks carry
  `GeoSource::TileLayer`, which is what selects the **tiled** art path
  (`rendering/world.rs:695`). Entity-authored surfaces take the stretched
  128 px entity texture, which on a 104-tile ground run is the smear that path
  exists to avoid. Terrain wants IntGrid on art grounds alone.
- **Player spawn** (`PlayerStart`), **loading zones** (`LoadingZone`), **vault
  coins** (`PickupSpawn` → `Currency`, already the channel they use), **AI slop
  and snakes** (`EnemySpawn`; Sanic already does exactly this and renames the
  display identity after load, `demo_sanic/src/lib.rs:276`), **all decorative
  props** (`Prop`).

That is the whole of the layout Jon wants to push around by hand, minus the
special blocks. **Sanic is the proven template for the shape** — spatial layout
in the demo's own `.ldtk`, generated once by a script, with the parts LDtk has no
vocabulary for grafted on in Rust after load (`demo_sanic/src/lib.rs:14`).

## 4. What is NOT free — the decision

Five concepts have no authoring path: **?-block, brick, quasar block, warp pipe,
flagpole goal.** All five are the same shape: *authored geometry that carries
game-defined meaning and must be addressable by the runtime after the author has
moved it.*

The identity half is **already solved** and worth saying plainly: an
entity-authored surface gets `GeoId::placement(PlacementId(iid))`, the LDtk iid
survives moves and edits, and `ContactSource::Block` already reports the struck
block's `GeoId`. **What is missing is only the payload** — nowhere to author
"this one contains a wand" — and a way for the game to read its own records back.

⚠ note the split that falls out: **terrain must be IntGrid** (tiled art,
paintable) and **special blocks must be entities** (durable iid). IntGrid blocks
are keyed by a *row-major merge ordinal*, which renumbers when you paint a cell,
so an IntGrid ?-block could never hold an identity. That is not a compromise; it
is the engine's existing design being right.

### Option A — extend the engine enums

Add `PlacementSchema::PowerBlock`, `::WarpPipe`, `::Flagpole`.

⛔ **I recommend against it.** These are one game's nouns. The engine would
accumulate every game's vocabulary, which is the pressure described in §2 given
its way, and it contradicts the federation direction ADR 0032 just established
for content schemas. It also does not scale to the next demo.

### Option B — one opaque game channel on the room

`RoomEmission`/`RoomSpec` gain a single `authored: Vec<AuthoredRecord>` where the
record is `{ id: PlacementId, kind: String, aabb, fields }`. The engine carries
it from LDtk to the room and **never interprets it**; the game reads its own
records at load.

Small, and honest about what it is. The cost is a stringly-typed bag at the seam
with nothing validating it.

### Option C — make the placements channel game-extensible ⭐ recommended

Open `PlacementKind` from a closed enum to an id, and let a game register a
**lowering interpreter** for its own kind.

⭐ **The mechanism already exists and is already the stated direction.** The
placements channel is documented as *"the schema-over-record channel every family
converges onto"*, and records are explicitly *"inert until an interpreter is
registered for their kind"* (`conversion/mod.rs:409`). Six engine families
already converge there. The only thing making it engine-only is that the kind is
an enum.

This gets B's extensibility while keeping one channel, one registry, and one
lowering path shared with the engine's own families — and it lands `?-block` and
`brick` as *the same kind of thing* the engine already lowers, rather than beside
it.

⚠ **the honest cost, stated up front**: the closed enum is currently what makes a
placement's construction schema stable, and `PlacementKind::stable_id` is a
declared *"compatibility contract [that] may only change with a fingerprint-schema
bump"* (`placements.rs:441`). Opening it means a game-defined kind participates in
that identity, so kind ids need the same collision discipline content-pack schema
ids have — plausibly by registering them **through the content pack**, which is
where a capability already declares what it owns. That link is the part most
worth your opinion.

⚠ and `install_ldtk_entity_converters` is a process-global `OnceLock` whose
endpoint is already written down as "should be an App resource selected through
provider context" (queue G3). Mary-O would be its **first real user**. I would
not fix that as part of this, but it stops being hypothetical.

## 5. What entity refs buy for free

LDtk supports entity references, and the engine already resolves one across the
seam: `mount_links` carries `(rider_id, mount_id)` from an authored `mounted_on`
ref (`conversion/mod.rs:401`). **That is exactly warp-pipe pairing.** Under
option B or C a pipe authors `warp_to: <the other pipe>` and Jon can lay out a
new pipe pair in the GUI with no code at all — where today the pairing is the
order of a four-element Rust tuple list.

## 6. Consequence worth accepting deliberately

Three tests pin 1-1's exact shape today —
`level_1_1_carries_the_grammar_it_claims`,
`level_1_1_authors_the_bricks_the_break_runtime_expects`,
`level_1_1_claims_the_mary_o_mode`. Once Jon authors the level, an exact-layout
pin becomes a tripwire that fires every time he moves a brick.

They should become **invariant** checks against the loaded room — *every ?-block
sits at a bonkable height over reachable ground*, *every warp pipe's partner
exists*, *the goal is reachable from the spawn* — which is the version that
actually protects the level and survives being edited. That is a small win in
itself: those are the properties, and today they are only implied by constants.

## 7. What I would do, in order

1. **Phase 1 — no engine change, no decision needed.** Author terrain, spawn,
   zones, coins, enemies and props for 1-1 and 1-2 into
   `game/ambition_demo_mary_o/assets/worlds/mary_o.ldtk`, generated once by
   `game/ambition_demo_mary_o/tools/author_mary_o_ldtk.py` from today's
   constants, then loaded and grafted exactly the way Sanic does. The five
   special concepts stay Rust-side and keep working, unchanged.
   **Jon can start laying out the level as soon as this lands.**
2. **Decide §4.** If option C, the special blocks become authored entities, the
   Rust constant tables and `FlagPole`'s mirrored resource are deleted rather
   than kept in parallel, and the tests in §6 become invariants.
3. Only then: pipes-by-reference, and 1-2's elevators.

Phase 1 is not a throwaway step toward phase 2 — it is the same file, gaining
vocabulary.

## 8. What I need from you

- **§4: A, B, or C** — and if C, whether a game-defined placement kind should be
  declared through the content pack (so kind ids get the collision discipline
  schema ids already have) or registered directly at plugin-build time.
- Anything in §3 you would rather keep in Rust. I have assumed you want the
  terrain and the enemy/prop/coin placement in the GUI and do not care where the
  vault's coin loop lives.
