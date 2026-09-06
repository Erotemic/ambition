# The three sheet-target collisions, pair by pair

Measured 2026-08-19 against the **real** generated baked table
(`target/debug/build/ambition_sprite_sheet-*/out/baked_sheet_rons.rs`,
812 entries), not against a model of it. 166 unique targets, 39
geometry-differing collisions — the same 39 the code's own comment records.

## What the pairs actually are — and it is not what "stale manifest" implies

Neither file in any pair is a stale duplicate of the other. **They are
different characters that share a rig target string.** `tech_bro_disruptor`
declares `target: "robot"` because it was drawn with the robot rig, and its
manifest is as current as any other. So there is no "which one is stale" to
answer; the question is whether the character named `robot` should keep its
own name as a registry key while seventeen other characters also claim it.

`SheetRegistry::from_baked_table` inserts in table order and **the last one
wins**. The table is sorted by file root with `_spritesheet` stripped, so
`robot` sorts *before* `robot_archivist`, and the alphabetically-last claimant
takes the key.

## Pair 1 — `robot`, claimed by 18 manifests

```text
     robot_spritesheet.ron            256x256   <- the character's OWN sheet, LOSES
     robot_archivist_spritesheet.ron  230x256
     ... 15 more ...
  -> tech_bro_disruptor_spritesheet.ron 215x256  <== WINS THE KEY
```

The catalog authors `robot` as `sprites/robot_spritesheet.png` (256x256).
`tech_bro_disruptor` is not a catalog id at all.

## Pair 2 — `goblin`, claimed by 9 manifests

```text
     goblin_spritesheet.ron              239x253 <- the character's OWN sheet, LOSES
     goblin_brute_hammer_spritesheet.ron 232x256
     ... 6 more ...
  -> ranged_skirmisher_spritesheet.ron   235x229 <== WINS THE KEY
```

The catalog authors `goblin` as `sprites/goblin_spritesheet.png` (239x253).
`ranged_skirmisher` is not a catalog id.

## Pair 3 — `sandbag`, claimed by 3 manifests

```text
     sandbag_spritesheet.ron                128x128 <- the character's OWN sheet, LOSES
     sandbag_armored_review_spritesheet.ron 256x256
  -> sandbag_full_review_spritesheet.ron    256x256 <== WINS THE KEY
```

The only pair where all three files are the same character's own review
variants. The shipped 128x128 sheet loses its key to a 256x256 review render.

## Two more collisions that are NOT defects, stated so the set is closed

- **`toon` (16 claimants)** and **`ninja` (2)** — neither name is a catalog id,
  so no character resolves art by them.
- **`shrine` (2)** — both entries are the same file reached through two
  directories, identical 88x160 geometry. Not a collision at all.

## Nothing is visibly broken today, and here is why

The three losers are latent, not live. Every consumer of the target-keyed
`SheetRegistry` looks up a name where the file root equals the target:
projectile visuals look up a visual id, shrine visuals look up `"shrine"`, and
the boss road resolves `boss` / `giant_gnu` / `gnu_ton_rider` /
`mockingbird_boss`. None of those is `robot`, `goblin` or `sandbag`. The
character-geometry road (`posed_body_geometry`, the animation road) uses
`record_index()`, which keys by **file root** — 196 unique keys, no ambiguity.

So the defect is that a shared engine lookup cannot answer *"give me sheet
`robot`"* correctly, in a tree where nothing currently asks it.

## What the ledger row got wrong

`awaiting-maintainer-decision.md` §19 names the winners as `robot_archivist`
over `robot`, `goblin_brute_hammer` over `goblin`, and
`sandbag_armored_review` over `sandbag`. In each of the three, that is the
**first** non-own claimant, not the last — the row read the collision list with
last-wins inverted. The count of three is right; every named winner is wrong.

The same slip nearly caught this write-up: a first pass modelled the sort with
`_spritesheet` still attached, which puts `robot_archivist` before `robot` and
reproduces exactly the row's mistake. Reading the real generated table is what
separated them.
