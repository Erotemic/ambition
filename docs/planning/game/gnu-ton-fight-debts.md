# GNU-ton's fight with arms and portals: what is not elegant, and what it should be

Written 2026-10-11, when phase 1 (arms on the giant, a punch through a real
pair of portals) and the clamp went into the game. Jon: "if there is
anything inelegant that you've added, log it in docs/planning for refactor
later." The fight's design is in `bosses.md`; the debts of the giant's
PLACES are in `../engine/the-art-says-where-a-body-stands.md`.

Status of each: OPEN. The content comes first.

## What was elegant

- The pair of the punch is two `PlacedPortal` entities on partner channels.
  The engine did the rest: the lines, the window that opens when a body is
  near, the transit, the push of a body that is on the plane when the pair
  closes. No engine code was changed for it.
- The phase-2 fight is the fight of before, tick for tick (the parity test
  holds it), so the new phase did not cost the old one.

## The debts

### 1. The fist does not go through the portal. Two drawings pretend it does

- The giant's sheet draws the arm going INTO the near line (rows
  `punch_*`, cut at the line). The fist's sheet draws a fore arm and a fist
  coming OUT of the far line (rows `through_*`, cut at its frame's top).
  The conductor pins the two rows at the same times. They agree because
  both read `giant_art.punch_keys`, and because the conductor's times
  (`arms::STRIKE`, `RECOVER`) are copies of the rows' lengths.
- The fist's body is put under the far line by the conductor. Its damage is
  a box there.
- **Should be:** one arm, one animation, and the ENGINE cuts it at the near
  line and draws the rest at the far line, as it does for a body that is in
  a pair (the transit pieces of `ambition_portal2d_presentation`). Then an
  arm can go through a pair at any place and any angle, the far side can
  not be out of step, and a damage box that is mapped through the pair
  hurts where the fist is seen. This needs: a limb that is part of its
  host's picture, a held damage box that a portal maps, and angled portals.

### 2. A module can not open a portal

- The conductor is an extension module. It has no port for a portal, so it
  writes where each line is into its RECORD, and a native system of the
  game (`ambition_content::bosses::gnu_ton::portals`) reads the record and
  keeps the entities. The record is used as a channel.
- **Should be:** a port. `PortalPair { slot, near, far, half }` and
  `LightLine { slot, at, normal, half }` as held requests (as
  `HeldDamageBox` is): the host owns the entities, their identity, their
  end on defeat and on a reset. Then the Mockingbird or any module can use
  portals, and `portals.rs` goes.

### 3. A line that is a look needed a new thing

- A `PlacedPortal` with no partner takes nothing through, but when it MOVES
  the engine pushes each body on its plane to one side
  (`evict_straddlers_on_portal_change`). The lines at the shoulders are on
  the giant and the lines at the wrists move with the fists. So the look
  lines are `PortalLightLine` (new, `ambition_portal2d_presentation`): the
  material of a portal's line, on an entity that the simulation spawns and
  the presentation dresses.
- It opens as a portal does. It does not dissolve: it is gone in one frame.
- **Should be:** one of two things, to be decided with the portal lane.
  (a) A portal has a flag "occupied": it is a real aperture with its glow,
  its dissolve and its window, and it takes nothing and pushes nothing.
  That is the lore (the arm is in it) and it is one concept, not two.
  (b) `PortalLightLine` stays, and gets the dissolve, and is spawned by the
  presentation from a view, not by the simulation.

### 4. The place, the length and the angle of each line are typed by hand

- `choreography::arms` has: the near line of the punch, the two shoulder
  lines, the wrist line, how far the far line is over the fist, and the
  width of the arm at each line (MEASURED from a rendered frame by a script
  that is not in the tree). When the drawing changes, they do not.
- Each line is upright or flat. The arm at a shoulder's line goes down by
  about 8 degrees; the line does not turn with it.
- When the giant bucks or rears, the lines stay where they are for the
  giant at rest.
- **Should be:** design A of `the-art-says-where-a-body-stands.md`, with one
  more kind of place: a `line` (a point, a direction, a width), per row and
  per frame. The renderer knows all three: it draws the arm and cuts it
  there. The conductor reads them through `MountView`. With angled portals
  the line turns with the arm.

### 5. The fists are bodies when they are on the arms

- In phase 1 a fist is in the giant's picture. Its body is still there: the
  conductor holds it in the middle of the giant, pins the row `none` (a
  frame of nothing in its sheet), and says it is no opening.
- A fist is built at its limb's home, and the conductor first runs on the
  second tick of the room. For those ticks the two fists are at their homes
  (the arena test `entering_the_arena_the_fists_are_on_the_giants_arms`
  bounds it at 3). Not seen in a window.
- A name tag ("Giant GNU Hand") is over the giant's chest for each.
- **Should be:** a limb has a state "on its host": no body in the world, no
  view, nothing to hit. Loosing it makes the body. (Or: the conductor can
  say a body is not presented, which is less.)

### 6. The trail of a fist joins two lines, by a resource

- Jon (2026-10-11): the trail is part of the scholar's spell, and joins the
  two portals an arm is in. So the wisps (`limb_trail.rs`) start at the
  shoulder's line (or at the near line of the punch), not in the giant.
- How: a presentation resource, `LimbTrailStarts` (a start by the limb's
  feature id), which a system of the game writes each tick from the
  conductor's record (`join_gnu_ton_lines`). A simulation system writes a
  presentation resource.
- The trail ends at the middle of the fist, not at the line behind its
  wrist (the fist's picture covers the rest).
- **Should be:** with the port of debt 2, a line that a module holds can be
  "the start of this limb's bond" and "its end", and the presentation reads
  both from the view of the limb.

### 7. A pinned row does not loop

- On the actor road a pinned row plays once and holds, and its `elapsed`
  and `looping` are not read. So the giant's rest with its arms on it is
  two rows that are the same (`armed_rest`, `armed_rest_b`), and the
  conductor goes from one to the other.
- **Should be:** the actor road reads `looping` (and `elapsed`), as the boss
  road does. Then `armed_rest_b` and the turn go.

### 8. Two sets of rows for one body

- Each row of the giant is drawn with its arms in the lines; `armed_*` is
  the same row with whole arms. Only four rows have an `armed_*` (rest,
  its copy, hit, buck): a stomp in phase 1 would draw the arms in lines.
- **Should be:** with the giant as an SVG rig (debt 6 of the other
  document), the arms are parts, and "arms on" or "arms in lines" is a
  state of the rig, not a second set of rows.

### 9. Whether the arms are on is found from signs

- The trigger of a conductor does not say the encounter's phase. The
  conductor has `loosed` in its record and sets it: when the clamp is
  through, or when it sees a move of the free fists or an enrage with no
  clamp before it (a fight that starts late; the parity test starts so).
- A punch that is in progress when the clamp begins is cut: the arm goes
  from the punch to the clamp in one frame.
- **Should be:** `BossConduct` says the phase. And the pattern can end a
  move well before a phase ends (a property of the pattern's steps).

### 10. The hall of characters

- The giant in the hall has no conductor: its arms are cut at two lines
  that are not drawn, and its right fist rests on its right arm.
- **Should be:** the hall shows a character's states (arms on; arms in
  lines with the lines), from the same data the fight uses.

### 11. The way up is a narrow thing

- A test plays it (two jumps up into the far line, out of the near line,
  to the side onto the back). No person has played it.
- It works by numbers that are near their limits: two jumps rise about 148
  and the far line needs 117; out of the near line the player has about
  0.45 s to go 60 to the side.
- **Should be:** a way up with room in it. With the arm through the pair by
  the engine (debt 1) the near line can be where the design wants it, and
  the pair can give a body that comes out a push along its normal.

## An order

1. The actor road reads `looping` (7). Small; one copy of a row goes.
2. A person plays the way up (11), and its numbers get room.
3. A limb "on its host" (5), and the trail (6).
4. The port for a pair and a line (2), with the decision of 3.
5. The lines from the sheet (4), with step 1 of the other document.
6. The arm through the pair by the engine (1), with angled portals.
