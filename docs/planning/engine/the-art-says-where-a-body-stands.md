# The art says where a body stands: debts of the GNU-ton giant, and the design that pays them

Written 2026-10-11, when the new giant gnu went into the game. Jon: the
work that put it in was "fine for now"; the debts are to be written down,
with the architecture they should have, so that an agent can make it right
later. The content of the fight comes first.

Status of each debt: OPEN, except as the receipts below say.

**Built 2026-10-25 (Jon: "let's make it possible for platforms to follow the
art"): standing surfaces per frame (A `ground`, B for flat lines).** A sheet
publishes a standing surface as two anchors of each frame,
`surface:<name>:a` and `:b` (left end first, parts of the frame, as the
portal seams `seam:<slot>:a` / `:b` are); the sheet's mechanical digest
carries them (`ambition_sprite_sheet::surfaces`). A mount's
`drawn_ledges` names surfaces; `BackLedge` holds the table and, per ledge,
where it was drawn at the end of the last tick and the tick before
(rollback state, read after every row pin: `follow_drawn_ledges`). The
ground is a one-way line from end to end, TILTED as the drawing is, moved
and turned by the mount's step and the drawing's (2026-10-26); its ends are
ledges a body can hang from while they tilt less than 20 degrees
(`slopes-feet-ride-surfaces.md`, "ledges that tilt").
`SurfaceTable::segment` puts a surface at its frame in the middle of the
frame's time and goes smoothly to the next. The giant's two shoulders are
the first (`giant_art.surfaces`). `mount.back` is now a span, not ground.
Tests: `gnu_ton_fight::a_body_on_a_shoulder_rides_it_through_a_hammer`.

## The rule the debts break

The repository has the rule already, for a body's box: **the art is the
body**. A sheet says the box of its body in its own pixels
(`body_pixel_bbox`), a character says a scale (`WorldPerPixel`), and the
game multiplies. No one types the size of a body.

For the new giant that rule holds for the box, and for nothing else. Each
other fact of where things are on the giant was read from the drawing by a
person (an agent, with a script it threw away) and typed into another file.
When the drawing changes, those files do not.

## The debts

### 1. Where a body can stand on the giant is typed by hand

- `mount.ledges` of `npc_giant_gnu`
  (`game/ambition_content/assets/data/character_catalog.ron`): twelve
  boxes, ten for the slope of the back and one for each horn. `mount.back`
  (the line of the shoulders) and `mount.saddle` (the scholar's seat) are
  typed the same way.
- The numbers came from the drawing (`giant_art.STANCES`, `SHOULDER_L`,
  `SHOULDER_R`, `gnu_horns`) through a script that is not in the tree.
- They are true for ONE pose: the giant at rest. When it bucks (the row
  `buck` draws it reared up) the ledges stay where they were.

### 2. The slope of the back is steps

- A mount's ground is flat one-way blocks (`BackLedge::ground`,
  `mount_backs_are_ground` in `crates/ambition_mount/src/lib.rs`). So the
  slope the art draws is ten steps, and a body goes up it in hops.
- `BackLedge` got a list of ledges beside its `back`. It was put in the
  component that was there so that no rollback row had to be added. That
  was the cheap place, not the right one: `back` is "what the conductor
  reads as on its back", and `ledges` is "more ground"; they are two facts
  in one component.
- The same debt, older: the ramp of the wake room's cart is fourteen steps
  (`intro.ldtk`).

### 3. A mount's back is carried at the wrong rate

`70b516476` fixed the units: ground now moves by `SweepSample::curr - prev`.
Both blocks and surface chains use that displacement. The guard
`a_ridden_mount_s_ledges_are_ground_in_its_own_live_room_only` gives the mount
a velocity different from its measured step and checks the step in the overlay.
Keep carry based on measured displacement, including when a collision stops a mount.

### 4. The room has a copy of the giant's size

- The arena's `EnemySpawn` for the giant is a box of 630 x 464
  (`sandbox.ldtk`, `gnu_ton_arena`; `specs/gnu_ton/fit_the_new_giant.yaml`):
  a copy of the body's box times its scale. A body is built with its feet
  on the bottom of its box, so a wrong copy puts the giant in the wrong
  place.
- The scholar's `BossSpawn` was moved by hand to the seat.
- Three stones of the arena were moved by hand from under and over the
  giant.
- The hall of characters has a constant for the height of the giants' row
  (`GIANT_SLOT_HEIGHT_PX = 832`), a copy of how tall the art is.
- `specs/gnu_ton_arena_area.ron` is behind the level it says it describes
  (it has the stones as entities; the level has them as painted cells).

### 5. The fight's own numbers do not know the giant

- The fists' rest (`hands.rest`, 400 to each side) is typed by hand from
  the width of the body.
- The choreography (`game/ambition_content_modules/src/gnu_ton/
  choreography.rs`) has heights of the ROOM: `HOVER`, `CEILING`, the pivots
  of the pendulum and the cradle, the orbit about the scholar. The scholar
  is 247 higher than he was, so the orbit is higher over the floor. The
  fight was not tuned again.
- A test has a copy of the seat (`SEAT` in `game/ambition_app/tests/
  gnu_ton_fight.rs`), and five fixtures have the old giant's numbers.

### 6. The giant is posed by code, and its sheets carry what no one reads

- The new giant is drawn and posed by a Python module
  (`targets/characters/gnu_ton_boss/giant_art.py`): a pose is a table of
  numbers, and a row is a function of them. Each other new creature is an
  SVG that a person can open and change, with a rig and clips. The giant
  can not be opened in Inkscape.
- `_svg.py` in that folder is a COPY of the helpers of
  `scripts/svg_art/svgkit.py` and `birdkit.py`, because a target can not
  import a script.
- `sprite_generator.py` still makes five pages of 6311 x 5301 in one pack
  (`gnu_ton_boss` and its `_full`, `_body`, `_hands`; `giant_gnu`;
  `giant_gnu_body`; `giant_gnu_hands`). The fight reads ONE of them
  (`giant_gnu`), the hall reads one more (`gnu_ton_boss`, the giant with
  its scholar and its fists in one picture), and three have no reader.
  The fused picture's boxes (`parts`, the head's hurt box, the fists' hit
  boxes) are made for a boss that no fight uses.
- Two files in the published folder are in git and are stale
  (`gnu_ton_boss_spritesheet_manifest.json`, `gnu_ton_boss_parts.json`);
  two tools still read the first.
- The sheets have no arms and no `free` face; the fight that Jon wants
  needs both (`docs/planning/game/bosses.md`).

## The design that pays them

**One sentence: a sheet says, in its own pixels, each place on its body
that the game needs; a character says a scale; nothing else says a
number.**

### A. The sheet has named places, per row

The sheet's record (`*_spritesheet.ron`, what `body_metrics` is today) gets
a small vocabulary of places, in frame pixels:

| Place | What it is | Who reads it |
|---|---|---|
| `body` | the box of the body (today's `body_pixel_bbox`) | the body's size and where its art is drawn |
| `seat` | a point: where a rider's soles are | the mount's seat (today's `mount.saddle`) |
| `ground` | lines of points: where a body can stand, each with a name | the mount's ground (today's `mount.back` and `mount.ledges`) |
| `rest` | points with a name: where a limb rests | the limbs' homes (today's `hands.rest`) |
| `reach` | a box: all that the art covers (horns, tail) | how much room a placement needs |

- The renderer writes them from the drawing: the same numbers that draw
  the epaulet say where its top is. `giant_art.seat_place` and `to_frame`
  are the start of this.
- A place can differ by row and by frame (the giant reared up has its
  shoulders higher). The record has the places of the row at rest, and a
  row or a frame says only what differs. This is the shape the fused
  sheet's `parts` had, made general and given to the sheets that are used.
- The catalog keeps what is BEHAVIOUR: the scale, the class of a mount,
  which line of `ground` the conductor reads as "on its back" (by name),
  who conducts the limbs. It has no pixel and no offset.
- One reader in the game (`posed_body_geometry` in
  `crates/ambition_sprite_sheet` is where the box is read today) turns the
  places into world units about the body's centre. `MountBack`,
  `CharacterMount::saddle` and `CharacterHands::rest` become what that
  reader gives, not what a person types.

### B. A mount's ground is surfaces, not boxes

- The `ground` lines of the sheet go to the room as SURFACES with an
  identity, at the mount's pose, moved by what the mount did move in the
  tick. This is phase C of `slopes-feet-ride-surfaces.md` (the support
  that lasts, chains in the room's overlay). The slope of the giant's back
  is then one line, and a body walks it.
- Until that is built, the same lines can be cut into steps BY CODE, in
  one place, so the debt of today (steps typed by hand) is gone before the
  slope engine is there. That is a small change and needs no change of
  the kernel.
- `BackLedge` becomes two things: the mount's ground (the surfaces) and,
  for a conductor, the NAME of the line that is "its back". The carry uses
  the displacement the mount did make (debt 3).

### C. A room places a body by its feet

- A spawn says a point: where the body's feet are. It does not say the
  body's size. (`placed_as_it_starts` reads the box's bottom today; a
  point is the same fact with no copy of the size.)
- A tool that makes a room (the hall's generator) asks the sheet for
  `reach`, and makes the row as tall as that. No constant.
- A check for the levels (in the tools, where the other checks of rooms
  are) says when the `reach` of a placed body goes through a block, so a
  giant that grew is found by a tool and not by a photograph.

### D. The fight measures from the giant

- A conductor reads the places of its mount (it has `MountView` for the
  body, its back, its limbs and the hall): the seat, the rest of each
  limb, the top of the art. The choreography's heights become distances
  from those (so high over the scholar, so far out from a flank), with the
  room as a bound, not as the origin.
- Tests read the seat from the giant, not from a copy.

### E. The giant is an SVG with a rig, and one sheet per reader

- The drawing moves to an SVG that owns the art
  (`data/characters/giant_gnu/`), with the rig pipeline that the other
  creatures use: parts, bones, clips. Its faces are swap sets (the parts
  of `EXPRESSIONS`: lids, pupils, ears, mouth), its arms are parts that a
  row can hide, its two stances are poses of one rig. The named places of
  A are joints of the rig.
- The pack makes the sheets that have a reader: `giant_gnu` (with rows
  for the fight that is to come), `giant_gnu_fist`, `gnu_ton_rider`. The
  hall shows the giant, its rider and its fists as the fight does (three
  bodies), so the fused picture and its three unread pages go, and
  `sprite_generator.py` with them.
- One SVG kit, in the package; the scripts in `scripts/svg_art/` import
  it. `_svg.py` goes.
- The two stale JSON files go, with the two readers of the first changed
  to read the sheet's record.

## An order that an agent can take

Each step is of use alone, and none needs the one after it.

1. **Steps from lines, by code** (B, the second point). The renderer
   writes `ground` and `seat` into the giant's sheet record; the game makes
   the mount's seat, back and ledges from them; the catalog's typed
   numbers and the test's `SEAT` go. Small; no change of the kernel; one
   rollback row may change shape (`BackLedge`), so check the three
   baselines.
2. **The carry at the right rate** (debt 3), with a test of a mount that
   moves with a body on its back.
3. **The room places by feet; the tools read `reach`** (C).
4. **The fight measures from the giant** (D). Do this WITH the fight's
   rework, not before: the numbers change there anyway.
5. **One sheet per reader** (E, the second point): delete the unread
   pages, the fused picture and its boxes, the stale files.
6. **The giant as an SVG rig** (E, the first point). The largest step.
   After the fight's rework has said which rows and parts the giant needs.
7. **A mount's ground as surfaces** (B, the first point). With the slope
   work (`slopes-feet-ride-surfaces.md`), not before it.

## What is NOT a debt

- The body's box is authored, not the union of the art (`BODY_BBOX`): the
  beast and its chest, not the horns. That is a decision: a body that
  stands by a wall stands by its flank.
- The fists are bodies of their own. That is the fight's design.
