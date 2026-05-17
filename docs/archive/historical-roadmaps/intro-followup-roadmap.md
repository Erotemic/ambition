# Intro vertical slice — follow-up roadmap

Status: v1 of the intro vertical slice shipped 2026-05-16 (commits
`c9349e1` … `cc4512b`). This doc tracks the structural items the
v1 round called out but deferred — each is a multi-commit refactor
rather than a small patch, so they live here instead of being
bundled into the v1 close-out.

Owner: whoever picks up the intro polish next pass.

## 1. GridVania layout + walk-through edge exits

**Current state:** intro.ldtk uses `worldLayout: Free` with five
levels spaced ~2000 px apart on the x-axis. Inter-room transitions
go through `LoadingZone` entities wired as doors (activation: Door
or walk). The wake → raid → escape → drain → gate-stack flow always
requires entering a doorway.

**Target state:** `worldLayout: GridVania` with rooms adjacent in
the grid. Side-scrolling exits — walk off the right edge of one
room, the camera scrolls and the next room loads. No doors for
intra-zone traversal; doors stay only when the design wants the
beat ("a hatch you have to interact with") or when zones cross.

**Why:** Movement-first Metroidvania readability. Doors break flow;
edge exits build a strong continuous map. The design doc explicitly
calls for "side scrolling exists, so you just walk through to the
next room, or the room just opens up as a very big room."

**Work:**
- Flip `worldLayout` in intro.ldtk to `GridVania` (re-author or
  surgical patch). `worldGridWidth` / `worldGridHeight` need to
  match level dimensions.
- Repack level world coords so adjacent rooms share an edge.
- Remove the door-style `LoadingZone` entities between adjacent
  intro rooms; rely on bevy_ecs_ldtk's natural neighbour loading
  for screen scroll.
- Decide which rooms truly become one big room (e.g. wake +
  raid corridor + escape shaft could be one continuous lab
  silhouette).
- Update the test-access LoadingZone in `central_hub_basement`
  to point at the new entry zone.

**Risk:** The runtime currently picks active areas through
`LdtkRuntimeIndex` + per-area `RoomSet` switching. GridVania edge
crossings still need to fire a "room changed" event for cutscene
bindings + camera framing. May need a small system that listens to
the player's containing LDtk level and updates `RoomSet.active`.

## 2. LDtk tileset rendering (visuals match game)

**Current state:** intro levels are pure IntGrid for collision +
rectangle entities for content. bevy_ecs_ldtk is configured with
`level_background: LevelBackground::Nonexistent` and the Ambition
renderer draws blocks itself, so the LDtk GUI editor view and the
in-game view diverge a lot. The intro_lab_tileset and town_tileset
PNGs exist on disk but aren't referenced by any LDtk project.

**Target state:** intro.ldtk has `defs.tilesets` entries pointing
at intro_lab_tileset.png and town_tileset.png. IntGrid layers
carry auto-tile rules tied to neighbour values so a Solid cell
renders the matching wall/floor tile in both the LDtk GUI and at
runtime via bevy_ecs_ldtk's native tile renderer.

**Why:** "Visuals in LDtk somewhat match the visuals in game" was
called out explicitly. Removes the placeholder colored-rect look,
makes level design happen with real art in the loop, and means
the bevy_ecs_ldtk side starts pulling its weight (it currently
loads the asset but doesn't render most of it).

**Work:**
- Add a `world tileset add <ldtk> <png>` Python subcommand that
  registers a tileset definition with the right uid + image path.
- Hook tileset uids into IntGrid layer defs as `autoSourceLayerDefUid`
  with rule groups (LDtk's auto-tile system).
- Remove the `LevelBackground::Nonexistent` / `IntGridRendering::Colorful`
  workarounds in the Bevy plugin config so the native renderer
  takes over for these layers, OR introduce a per-layer flag.
- Replace the lab + town room signage DebugLabels with real tile
  backgrounds where appropriate.

**Risk:** Coordinate frame. The runtime's existing
`world_to_bevy` centers each area at the origin; bevy_ecs_ldtk
draws tiles in raw LDtk world-pixel space. Need to either match
the frames (compose `LdtkWorldBundle` transform to align) or
disable Ambition's own block visuals for layers that have
tiles. The asset.rs comment block already calls out this
disagreement — that's where the seam needs work.

## 3. Lab props + cart visual (Prop entity type)

**Current state:** wake-room "diagnostic cart" is a `DebugLabel`
rect at the spawn position. Lab-prop sheet
(`creator_lab_props_spritesheet`) ships multiple sub-frames
(vat, terminal, table, …) but no entity type references them.
Player spawns standing on the label (good) but the cart isn't
visually there.

**Target state:** new `Prop` entity type in LDtk defs with a
`kind` field (intro_cart, lab_vat, lab_table, lab_terminal, lab_pipe).
A renderer system spawns the matching sprite at the entity's
position from a `Prop kind → (sheet_filename, frame_rect)` table.
Wake room places at least the cart + 2-3 lab props for set
dressing; the basement raid corridor can pull from the same
table.

**Why:** "Lots of lab props that are not used. The door into the
intro should spawn us on the diagnostic cart, just as it would
happen if the game was completed and we started a new game."
The cart needs to actually be visible for the spawn to read.

**Work:**
- `ambition_ldtk_tools def register-entity` invocation for `Prop`.
- Bevy: a `Prop` component + a system that consumes the LDtk
  field instance + looks up the sheet/frame.
- Update wake-room area spec to use `type: Prop, fields: { kind:
  intro_cart }` for the cart, then place 2-4 more props.
- Optional: pose variants for the cart (idle/roll/jolt) keyed
  off a story flag so it shifts visually as the intro progresses.

**Risk:** Low — the sprite atlas loading pattern already exists
for `EntitySprite`. Mostly authoring + a small render system.

## 4. NPC / Enemy unification around a single Actor entity

**Current state:** `NpcSpawn` and `EnemySpawn` are separate LDtk
entity defs with different field schemas. To make a peaceful NPC
turn hostile mid-game we go through a "hostile NPC migration"
path in `rendering/actors.rs` that swaps the runtime entry from
`world.npcs` into `world.enemies`. The intro v1 worked around this
by authoring Framebreaker / Nazi as `EnemySpawn` directly — but the
unification the design doc asks for ("Architecturally there should
be no distinction between an enemy and an NPC other than that for
an enemy the aggressiveness is at a level where they attack") is
still pending.

**Target state:** single `Actor` LDtk entity type with:

- `name` (display)
- `dialogue_id` (optional — peaceful + initial dialogue line)
- `aggression` enum: Peaceful / Wary / Hostile (and maybe Aggressive
  as a faster variant)
- `brain` (optional — overrides default for the aggression tier)
- `path_id` (optional patrol path)

The runtime composes the appropriate behavior from `aggression`
without two separate spawn paths.

**Why:** Cleaner authoring, no two-paths-for-one-thing surface,
trivial "NPC turns hostile" via flipping one field on the same
entity instead of migrating across runtime collections.

**Work:**
- Add `Actor` entity def to LDtk via
  `ambition_ldtk_tools def register-entity` (or a `world migrate`
  helper).
- Migrate existing `NpcSpawn` + `EnemySpawn` rows in sandbox.ldtk
  + intro.ldtk to `Actor` with the equivalent aggression.
- Collapse the two runtime spawn paths into one
  (`world.actors: Vec<Actor>` with disposition).
- The "NPC turned hostile" migration in `rendering/actors.rs`
  becomes a single field flip instead of cross-collection move.
- Retain `NpcSpawn` / `EnemySpawn` as deprecated aliases for one
  release so existing saves / older `.ldtk` files keep loading.

**Risk:** Touches many systems — combat, AI, dialogue, save format,
content_validation. Will likely want an ADR (would slot in as ADR
0010) to capture the schema.

## 5. Real-time barker dialog wiring (combat banter)

**Current state:** `VfxMessage::SpeechBubble { pos, text }` exists
and `fx::update_speech_bubbles` renders them as world-space
transient bubbles. The combat path already fires them in
`features/ecs.rs`, but the intro raiders don't have authored
combat-banter lines yet — they just attack silently after the
cutscene dismisses.

**Target state:** an authored `combat_lines` table keyed by
actor name → set of one-liners that fire as speech bubbles when
the actor lands or takes a hit. Intro raiders' wrong-list lines
should also fire as speech bubbles during the chase, not just in
the cutscene.

**Why:** Design doc says "real-time dialog where characters just
say thing (like when they get hit)" is a first-class mode the
sandbox should support. The infra exists; what's missing is the
content layer + a generic "say a random line from this set on
event X" trigger.

**Work:**
- New `crate::intro::banter` (or sandbox-level `combat_banter`)
  module with a `(actor_name → Vec<&'static str>)` table.
- A small system that listens to combat damage events and fires
  `VfxMessage::SpeechBubble` for the appropriate actor.
- Authored lines for Creator (during intro_raid), Framebreaker,
  Nazi Salvage Guard.

**Risk:** Low — additive content.

## Order of operations recommendation

If picking these up one at a time:

1. **Actor unification (#4)** first — schema change underneath
   everything else.
2. **GridVania (#1)** second — restructures the intro flow, easier
   on a clean schema.
3. **Tileset rendering (#2)** third — visual polish, biggest
   feel-of-game payoff.
4. **Lab props (#3)** fourth — additive on top of #2.
5. **Barker dialog (#5)** last — content-layer polish.

Or pick #2 + #3 + #5 together for a "visual polish" pass before
the structural items, depending on whether the priority is
"feels like a game" vs "architected like a game."
