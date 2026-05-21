# LLM spatial authoring discipline

> **Scope.** This page is binding for any LLM doing spatial/layout
> work in this repository — placing entities in LDtk, adding gates,
> dropping walls, choosing spawn positions, picking platform widths,
> sizing hitboxes. It exists because spatial work has more implicit
> intent than textual work, and getting it right requires *reading
> the map*, not just executing the prompt.

## The discipline (read this first)

When the user asks for "a gate", "a wall", "a one-way", "a
breakable", or any other spatial game component, your job is **not**
to ask them where to put it. Your job is to:

1. Read the existing map enough to understand what the player can
   currently do and where they can go.
2. Infer the *purpose* of the component being added (block exit?
   block entry? gate progression? mark a checkpoint? force a route?).
3. Place it where it intuitively fulfils that purpose — usually
   along an existing traversal seam.
4. State your reasoning when you do it so a reviewer can challenge
   the placement, not the existence.

If the answer truly isn't inferable from the map (e.g. two equally
good spots, or the purpose is genuinely ambiguous), then ask. But
ask with concrete options grounded in the map (`"at the top of the
ladder, or at the LoadingZone door?"`), not open-ended (`"where do
you want it?"`).

## Why this exists

> "We need to figure out a good way to build maps such that placing
> these gates isn't a thing you need to ask me about. You can look at
> the map: figure out what the purpose of the game component I'm
> adding there is, and place it intuitively." — Jon Crall, 2026-05

Asking "where exactly?" for every gate forces the human to do
spatial reasoning the LLM should be doing from the LDtk file plus
the screenshot. Gate doors have a use: **they block player
progression or exit**. One-way mechanisms (breakable floors, gated
exits) and blocking entry mechanisms are core map vocabulary; the
game will need many of them, and asking about each one is a paper
cut that adds up across a session.

## Practical rules

### 1. Map the player's *graph*, not just the geometry

Before placing a gate, work out (briefly, in your head or in a tool
call) the connectivity:

- Where does the player enter this room?
- Where can they exit (LoadingZones, EdgeExits, ladders, hidden
  passages)?
- Which exits are currently *un-gated* (i.e. the player can leave
  any time)?

A "gate" is almost always meant to close one of those exits during
some lifecycle state (encounter active, boss alive, switch off).

### 2. Match the gate to the seam it closes

- **Boss arena retreat:** gate the climb-up path (ladder slot above
  the entry ledge), not the bottom of the ladder. The player needs
  to descend INTO the fight; you're stopping them climbing back OUT.
- **Mob encounter:** gate the hallway-to-arena passage at the
  doorway, mirroring `LockWall` in `mob_lab` / `goblin_cantina`.
- **One-way descent:** prefer `BreakablePlatform { trigger: OnStand,
  respawn: Never }` over a Solid that gets removed — the breakable
  pattern carries the "you can only go down" semantics in the entity
  type itself.

### 3. Match the gate's *width* to the thing it blocks

- A ladder-floor gate should be the same width as the ladder.
- A doorway gate should match the doorway jamb spacing.
- A floor passage gate should match the passage cell width on the
  IntGrid.

Run `intgrid summarize` and look at the bbox of the relevant
IntGrid value, or query the entity widths via `entity query`, before
picking a size out of thin air.

### 4. Prefer named entities for runtime-mutable terrain

If a gate needs to be removed at runtime (boss defeat, switch flip),
author it as a **named `Solid` entity instance**, not as an IntGrid
cell. IntGrid cells become generic `"ldtk solid"` blocks the runtime
cannot find by name; named entities preserve the field name into
`ae::Block::name` so a gating system can `world.blocks.retain(|b|
b.name != "ladder_floor_gate")`.

Punch the matching IntGrid slot with `intgrid erase` so the named
entity is the *only* collider in the slot — otherwise removing the
named block leaves the IntGrid solid behind.

### 5. State your placement reasoning

When you commit a gate, the commit message must explain *what the
gate closes off* and *what condition opens it*, e.g.:

> "Floor-gate above the ladder column. Blocks climb-back-to-exit
> while the boss is alive; removed from world.blocks on defeat by
> the gnu_ton arena gating system."

That sentence is the spec — a reviewer should be able to challenge
the placement without re-reading the LDtk file.

## Tooling the LLM needs to do this well

The repo should grow `ambition_ldtk_tools` subcommands that answer
the spatial questions an LLM has to answer before placing a gate.
This list is the running TODO; add to it as you find friction.

- `intgrid summarize` — exists. Per-value cell counts + bboxes.
- `intgrid erase --px X,Y --size W,H` — exists. Cell-aligned erase.
- `entity query --level X` — exists. Lists entity instances.
- **Wanted:** `paths describe --level X` — for each LoadingZone /
  EdgeExit / ladder, print "from spawn, you can reach exit Y by
  doing Z (walk / fall / climb / jump)." Lets an LLM pick the
  closeable seam without guessing.
- **Wanted:** `intgrid query --px X,Y --size W,H` — what IntGrid
  cells overlap this rect, and what is their value? Mirror of
  `entity check` for collision authoring.
- **Wanted:** `room measure --level X --entity foo` — width / height
  / center of an entity by name, plus the nearest solid below / left
  / right. Saves the LLM from doing pixel arithmetic in its head.
- **Wanted:** `gates audit --level X` — list every named Solid /
  BlinkWall / BreakablePlatform along with which encounter/boss/
  switch (if any) gates it. Cross-check the gating systems for
  orphans.

When you witness friction here, add the missing subcommand or, if
that's out of scope, add a bullet to this list.

## Authoring loop summary

1. Read the LDtk YAML spec and look at the geometry.
2. Look at any screenshot or in-game capture the user shared.
3. Identify the closeable seam that matches the prompt.
4. Pick a width that matches the seam.
5. Pick "named entity" vs "IntGrid" based on whether the runtime
   needs to mutate it.
6. Apply via the matching `ambition_ldtk_tools` subcommand (no
   hand-editing `sandbox.ldtk`).
7. State the placement reasoning in the commit message.

If you find yourself about to type "where exactly would you like the
gate?", stop and run this loop instead.
