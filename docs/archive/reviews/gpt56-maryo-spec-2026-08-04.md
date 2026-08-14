Review and advance the unfinished Mary-O LDtk migration with the goal of making the level genuinely pleasant to author in LDtk.

Do not turn Mary-O’s power progression into authored data or an engine-interpreted progression language. Small/tall/fire semantics, wand/lantern reward selection, damage downgrades, and maximum-power behavior belong in the Mary-O game crate as ordinary Rust code.

## Primary goals

### 1. Establish the intended LDtk authoring workflow

Trace the current relationship among:

* `mary_o.ldtk`;
* `author_mary_o_ldtk.py`;
* the Rust-built `level_1_1()` world;
* the migration comparison tests;
* the live Mary-O provider.

The current Python generator was useful for bootstrapping the migration, but it should not remain a destructive source-of-truth workflow that deletes and reconstructs the LDtk project from mirrored Rust constants.

Move toward this endpoint:

```text
authored LDtk project
    → validated/lowered room content
    → live Mary-O session
```

The migration is not finished, so do not prematurely delete the Rust world or switch the live game before the necessary LDtk concepts exist. Instead:

* identify what still prevents `mary_o.ldtk` from becoming the authoritative level source;
* preserve hand-authored LDtk levels and entity instances;
* separate any schema/definition synchronization from destructive level regeneration;
* avoid requiring authors to edit matching coordinates in both Rust and LDtk.

Document the remaining migration stages clearly.

### 2. Make enemy placement authorable in LDtk

The current Mary-O LDtk level apparently contains no enemy placements. AI Slop and Solid Snake positions are still generated from Rust coordinate arrays and staging systems.

Use the existing LDtk enemy-placement infrastructure where appropriate, but improve the authoring seam if its current fields are too indirect.

Prefer an author-facing concept such as:

```text
ActorSpawn
  archetype_id
  facing
  optional brain override
  optional path
  optional variant
```

The stable archetype/content ID should determine the normal:

* character definition;
* brain;
* body;
* sprite;
* combat behavior;
* dormancy behavior;
* respawn behavior.

Do not use a human-readable display name as gameplay identity. Mary-O-specific systems such as shell or stomp handling should recognize stable character/archetype identity rather than strings such as `"Solid Snake"`.

Migrate AI Slop and Solid Snake placements into LDtk when the authoring representation is ready, then remove the corresponding Rust coordinate arrays and staging closures.

Do not introduce a large generic actor-authoring framework unless the current engine structures already justify it. A small explicit extension is preferable.

### 3. Make level transitions expressible

Authors need to express at least:

* pipe warp to another location in the same room;
* pipe warp to another room or LDtk level;
* ordinary door/walk/edge transitions where useful;
* victory transition to the next level, world map, or replay target.

Unify destination semantics without forcing every trigger through one gameplay mechanism.

A useful structural split is:

```text
RoomAnchor
  stable ID
  arrival position
  arrival facing
  optional emergence direction/presentation data

RoomExit or LoadingZone
  stable ID
  activation policy
  destination room
  destination anchor
  transition presentation
```

For pipes, support directional activation and pipe-specific presentation, for example:

```text
activation = directional press down/up/left/right
transition style = pipe
travel/emergence axis
```

Mary-O’s pipe animation may remain Mary-O-specific, but it should consume an authored destination and transition specification rather than hard-coded coordinate functions.

Victory is not fundamentally a spatial loading zone. The flag sequence should publish a level outcome such as victory, and the current level/session content should resolve that outcome to an authored destination:

```text
victory
    → next level
    → world map
    → replay current level
```

Replace the current hard-coded “flag tally replays this room” behavior with an explicit destination policy when the migration reaches that stage.

Keep transition routing generic enough for the engine, but keep Mary-O-specific flag and pipe choreography in the Mary-O crate.

### 4. Give reactive blocks stable semantic LDtk identities

Power blocks, quasar blocks, and bricks should not be recognized from old Rust-generated tile-layer ordinal IDs once the LDtk room becomes live.

LDtk placements naturally have stable instance identities. Introduce an authorable Mary-O block concept, preferably Mary-O-specific rather than a speculative engine-wide behavior language.

Possible representations include separate entities:

```text
MaryOPowerBlock
MaryOQuasarBlock
MaryOBrick
```

or one Mary-O entity with a small enum:

```text
MaryOBlock
  kind = Power | Quasar | Brick | FixedWand | FixedLantern
```

The LDtk conversion should emit:

* collision geometry;
* stable authored block identity;
* the Mary-O block kind;
* presentation metadata needed by the block renderer.

Mary-O Rust systems should interpret the block kind. Do not make the engine interpret Mary-O’s progression rules.

Before switching collision authority to LDtk, remove the dependency on hard-coded `GeoId::tile_layer(...)` ordinals for identifying reactive blocks.

### 5. Fix power-block hit behavior

Trace the reported behavior where a powered-up Mary-O appears unable to hit a power block.

The current reward-selection flow appears to return early when no next reward is selected. That can prevent all of the normal hit feedback:

* block flinch;
* spent-state transition;
* changed block art;
* reward or score outcome.

A valid head bonk should always be acknowledged, even when Mary-O is already at maximum power.

Keep progression logic in Mary-O Rust code. The intended baseline behavior is:

```text
small + progression block → wand
tall + progression block → lantern
fire + progression block → explicit top-tier outcome
```

Use an explicit top-tier outcome rather than `None`. For now, spawning another lantern is acceptable unless the existing design clearly prefers score, a reserve item, or another reward.

Do not silently ignore the bonk.

Also diagnose whether the reported failure occurs:

* only in fire form;
* in tall form as well;
* only after a block has already been spent;
* because visual form and worn equipment disagree;
* because the block geometry ID is no longer recognized.

A narrow debug readout or logging path is sufficient. Report:

```text
block stable ID
block kind
spent state
current Mary-O form/equipment
resolved reward/outcome
```

Do not launch a broad test campaign for this.

### 6. Preserve the intended modern power semantics

Keep these game-specific rules:

```text
small + wand → tall
small + lantern → fire
tall + lantern → fire

fire + damage → tall
tall + damage → small
```

In particular, a directly collected lantern while small must produce the fire form immediately. Do not emulate the original SMB1 behavior where the fire-flower-equivalent behaves like a mushroom for a small character.

The block reward resolver and direct item collection are distinct:

* a progression block chooses a reward based on current form;
* a fixed lantern pickup always grants the fire form.

Do not encode these relationships in LDtk. LDtk should author block kind and item placement; Mary-O Rust should define what those mean.

## Architectural boundary

Engine responsibilities should remain limited to reusable mechanisms such as:

* stable placement identity;
* collision/contact events;
* room and transition destinations;
* generic item spawning;
* rollback/session ownership;
* possibly generic once-consumed state.

Mary-O owns:

* power forms;
* wand and lantern semantics;
* progression-block reward selection;
* damage downgrade rules;
* flagpole victory choreography;
* pipe choreography;
* Mary-O-specific reactive block behavior.

Generalize only where an existing second game already demonstrates the same structural need.

## Deliverable

Implement a coherent next slice of the migration rather than trying to finish every item at once.

At minimum, produce:

1. a clear inventory of what is still Rust-authored versus LDtk-authored;
2. the LDtk schema/design for enemy placements, transitions, anchors, and Mary-O reactive blocks;
3. one meaningful end-to-end migration slice, preferably enemy placement or semantic reactive blocks;
4. the power-block maximum-form hit fix;
5. confirmation that direct lantern collection while small yields the fire form;
6. an updated migration plan that identifies the order for removing the remaining Rust coordinate sources.

Use targeted validation around the touched systems. Do not spend the work session repeatedly running the full workspace suite.
