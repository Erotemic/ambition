# Proposal: a game may extend the AUTHORED vocabulary, not just the converter

**Status: PROPOSED — awaiting Jon's review. Nothing here is implemented.**

> ⛔ **CORRECTED 2026-08-04, before review, and the correction is most of the
> value.** The first draft concluded that five of Mary-O's concepts were BLOCKED
> on an engine change. **They are not.** I found the gap by reading type
> definitions — closed enums, no extension slot — and did not check what the
> shipped code does about it. **Sanic already authors exactly this class of thing
> in LDtk today**: its monitor boxes are named `Solid` entities, and
> `break_monitor_boxes` matches them by `block.name` (`monitors.rs:119`), with
> breakage fed back through the collision overlay's `removed_block_names`. A
> monitor box IS a ?-block. The engine gap in §2 is real, but it is a **taste**
> question about where a payload lives, not a blocker — see §4, rewritten.
> ⚠ the lesson for me: a closed enum proves the *typed* channel is closed, not
> that the concept is inexpressible. Verify a "blocked" finding against the code
> that ships, by a different route.

Raised by Jon, 2026-08-04: *"I would like to make maryo an ldtk level so I can
manually play with it and lay it out. That means it needs to have the right
entities exposed for it and make it easy to author in."* — and then: think about
what makes this hard vs easy, and whether an engine change would make it better.

This document answers that. **Short version: all of Mary-O 1-1 can move to LDtk
with no engine change. What has to change is Mary-O's own runtime, which today
re-derives the level from Rust constants instead of reading it. There is one
genuine engine wart underneath, and it can be decided later, with experience.**

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
got round to LDtk, but because her RUNTIME reads constants, not the level.**

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
authored *payload* — "this block contains a wand" — has no **typed** place to
land.

⚠ **But it does have an UNTYPED one, and it ships.** An authored block keeps its
`name`, and the name is a game-readable channel: Sanic's `break_monitor_boxes`
does `block.name.starts_with("monitor_")` then `block.name == "monitor_super"`
(`monitors.rs:119`, `:129`) to decide which powerup a broken box grants — one
game's noun, authored in LDtk, with no engine vocabulary for it at all. Breaking
it is fed back through the collision overlay's `removed_block_names`
(`monitors.rs:235`), which is the same mechanism Mary-O's bricks would want.

So the accurate statement of the gap is narrower than the first draft claimed:
**an authored block carries a name and nothing else** — `Block` is
`{id, name, aabb, kind, velocity, art_color}` (`world.rs:50`), and a Surface
entity's custom fields are consumed by `parse_surface_spec` and never reach the
runtime. A game that wants structured authored data on a block must encode it in
the name string.

⚠ **This is still why the engine keeps being asked to absorb one game's nouns.**
The pressure that produced `PickupKind`, `ChestSpec` and friends is the same
pressure; the typed enum grows because it is the only *typed* place anything can
go, and everything else becomes a name convention.

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

## 4. The five special concepts — expressible today, with one wart

**?-block, brick, quasar block, warp pipe, flagpole goal.** All five are the same
shape: *authored geometry carrying game-defined meaning, which the runtime must
still find after the author has moved it.*

Both halves are already available:

- **Identity.** An entity-authored surface gets
  `GeoId::placement(PlacementId(iid))` (`surfaces.rs:324`); the LDtk iid survives
  moves and edits. And the block keeps its authored `name`.
- **Meaning.** The game matches on the name. This is not a workaround I invented
  for this document — it is what Sanic ships for the same problem
  (`monitors.rs:119`).

So Mary-O's blocks become named `Solid` entities added AFTER `area create`, for
the reason Sanic's generator already states in so many words: *"Monitors stay
ENTITY instances (never IntGrid): the demo identifies them as named blocks to
break + grant, so `area create`'s static-collision lowering must not eat their
names."* That comment is the whole design, already written down.

⭐ **Mary-O keeps her BETTER detection.** Sanic overlap-tests every block against
the player each frame; Mary-O matches a real head-bonk contact
(`ContactSource::Block` carries the struck `GeoId`, and her comment calls out
"with no point-matching" as the point). She keeps that and adds one lookup:
`GeoId` → the block → its name. Contact precision plus authored identity.

⚠ note the split that falls out: **terrain must be IntGrid** (tiled art,
paintable) and **special blocks must be entities** (durable identity, name
preserved). IntGrid blocks are keyed by a *row-major merge ordinal*, which
renumbers when you paint a cell, so an IntGrid ?-block could never hold an
identity. That is not a compromise; it is the engine's existing design being
right, and `area create` does the entity→IntGrid lowering for terrain so nothing
has to be hand-painted.

### The wart, stated plainly

**An authored block carries a name and nothing else.** A ?-block that contains a
wand has to say so *in its name* — `power_block_wand_0` — and warp-pipe pairing
has to be a shared name suffix. That is stringly-typed, and it is the honest cost
of doing this with no engine change.

Three ways to remove it, if and when it earns removal:

- **A — extend the engine enums** (`PlacementSchema::PowerBlock`, …).
  ⛔ **Recommend against.** These are one game's nouns; the engine would
  accumulate every game's vocabulary, and it contradicts the federation direction
  ADR 0032 just set for content schemas.
- **B — one opaque game channel**: `RoomSpec` gains
  `authored: Vec<{id, kind, aabb, fields}>` the engine carries and never
  interprets. Small and honest; still untyped, but *structured* and validated at
  the game's own load.
- **C — open `PlacementKind` from an enum to an id**, with a game-registered
  lowering interpreter. The mechanism is already the stated direction: the
  placements channel is *"the schema-over-record channel every family converges
  onto"* and records are *"inert until an interpreter is registered for their
  kind"* (`conversion/mod.rs:409`). ⚠ but `PlacementKind::stable_id` is a declared
  compatibility contract *"[that] may only change with a fingerprint-schema
  bump"* (`placements.rs:441`), so a game-defined kind would need the collision
  discipline content-pack schema ids already have.

⭐ **My recommendation is to decide none of these yet.** Author the level first
with names, and let the wart argue for itself. If after a week of authoring the
name conventions read fine, the engine change was never owed; if they read badly,
you will know exactly which fields you wanted and B or C stops being a guess.
Choosing an extension mechanism before there is a second user is how the closed
enum in §2 got closed in the first place.
## 5. Warp-pipe pairing, and what a name costs

Pipe pairing is the one place the name convention is genuinely weaker than a
typed field, so it is worth being concrete.

LDtk supports entity references and the engine already resolves one across the
seam: `mount_links` carries `(rider_id, mount_id)` from an authored `mounted_on`
ref (`conversion/mod.rs:401`). That is *exactly* the shape a pipe wants —
`warp_to: <the other pipe>`, picked in the GUI, impossible to dangle.

With names only, a pair is a shared suffix (`pipe_down_vault_a` ↔
`pipe_up_vault_a`), which means: a typo pairs nothing, and only a load-time check
catches it. **So write that check** — every pipe's partner must exist, refused
loudly at load, which is §6's `every warp pipe's partner exists` invariant. That
is the mitigation, and it is worth having regardless of how the pairing is
expressed.

⭐ If any single thing tips §4 toward B or C later, I expect it to be this one.

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

No step here waits on a decision from you.

1. **The file, and the terrain.** Author 1-1 and 1-2 into
   `game/ambition_demo_mary_o/assets/worlds/mary_o.ldtk`, generated from today's
   constants by `game/ambition_demo_mary_o/tools/author_mary_o_ldtk.py`, then
   loaded and grafted the way Sanic does. Terrain, spawn, zones, coins, enemies,
   props. The five special concepts stay Rust-side and keep working untouched.
   **Jon can start laying the level out as soon as this lands.**
2. **The special blocks, as named entities.** ?-block, brick, quasar block,
   flagpole, pipes — moved one family at a time, each one deleting its Rust
   constant table rather than running beside it. `FlagPole` becomes derived from
   the loaded room instead of a mirrored `Resource`.
3. **The invariants** of §6, replacing the three exact-layout pins.
4. Then 1-2's elevators, and whatever §4 has earned by that point.

Step 1 is not a throwaway toward step 2 — it is the same file, gaining
vocabulary.

## 8. What I need from you

- **Nothing, to start.** This is the change to §8 the correction made: steps 1–3
  need no architecture decision, so I am doing them.
- **§4 is a taste call I would rather you make with the level in front of you.**
  When the name conventions are real — `power_block_wand_0`, a pipe pair sharing
  a suffix — tell me whether they read fine or badly. Badly means B or C, and by
  then we will know which fields we actually wanted instead of guessing.
- Anything in §3 you would rather keep in Rust. I have assumed you want the
  terrain and the enemy/prop/coin placement in the GUI and do not care where the
  vault's coin loop lives.