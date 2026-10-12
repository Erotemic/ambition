# GNU-ton as a fighter, and a boss that fights back (started 2026-10-11)

Jon, 2026-10-11: integrate GNU-ton as a Smash character; in his boss fight
he fights back on the shoulder platform like a Smash character; he needs a
full expressive moveset. "It lets us shift to the next big thing that
ambition really needs improvement on: combat feel. If we can prove combat
feels good in smash that is the big win, because that translates to good
single player combat. But it also blends this pattern style boss with a
fighting style boss."

This document is the state of that push and what comes next. The fight's
design is in `bosses.md`; what is not elegant in the fight is in
`gnu-ton-fight-debts.md`.

## One man, three sheets, one table

- **The drawing.** One SVG rig (the renderer's `gnu_ton_scholar_villain`:
  the "D2" scholar, 19 bones). Three targets publish rows from it:
  - `gnu_ton_scholar_villain`: his ten boss rows (no reader in the game).
  - `gnu_ton_scholar_fighter`: 94 fighter rows. The sheet of the roster
    fighter `npc_gnu_ton`.
  - `gnu_ton_scholar_boss`: the ten boss rows, then the rows of the duel.
    The sheet the boss body wears (`boss_art_keys.ron`, `gnu_ton_rider`).
- **The table.** `game/ambition_content/assets/data/movesets/gnu_ton.ron`:
  26 moves (jab, three tilts, three smashes, five aerials, dash attack,
  taunt, five specials, the grab kit). The roster fighter plays it. The boss
  plays FIVE of its moves on the giant's back: the same `MoveSpec`, made for
  the main game (hearts and a fixed launch) and for his larger body. So a
  move's times are tuned in one place and are felt in both games.

## The fighter (`npc_gnu_ton`)

A light, fast fencer with two tools. The quill is long and thin: reach and
speed, little hurt, no finish. The book is short and slow: the finish. His
specials are his laws: Opticks (a flash, the same launch at each percent), a
fluxion (he goes forward through you), levity (a save), the Third Law (a
counter whose reaction is a strike), universal gravitation (down, in the
air).

First cut. The table was derived from the patent clerk's for its format, and
each move's times, hurt, launch and reach were set for this body. No number
is a result of play.

## The boss fights back

With the giant's arms on it (phase 1) and the player on its back:

- The conductor module chooses a move by where the player is (at his elbow:
  the jab, each third time the low sweep; at the quill's length: the thrust,
  each fourth time the book; over him: the flourish), and when.
- A native system (`bosses/gnu_ton/duel.rs`) plays that move on his body and
  draws its row. The giant does not punch then: he is the fight.
- A blow on him stops his move if it has not struck yet, and he goes to the
  other shoulder. The third blow: the giant throws.

He has no guard, no dodge that is his choice, and he is not launched: a boss
body takes hearts only. Those are the next steps (below).

## Combat feel: what was measured

**A move must hit when its picture hits.** A row is slaved to its move
(frame = progress x frames). `scripts/measure_move_art_sync.py` compares the
live part of each move with the part of its row where the limb is out.

- First measure: 10 of his 18 strikes were live BEFORE the picture's limb
  was out (the jab: live at 0.05 to 0.10 s, the quill out at 0.11 to 0.19).
- Cause 1: each attack clip of the rig had ONE rhythm (anticipation at 24%,
  contact at 48%, follow-through at 72%). A contact at the middle of a clip
  can not be a fast jab. Now each clip has its own (`BEATS` in the pose
  library): a jab's contact is at 26%, a smash's anticipation is 36% long
  and its contact is a snap.
- Cause 2: the table's windows were the clerk's, scaled. Seven moves had
  their live window moved to the picture's.
- Now: 4 of 18 under half (`smash_down`, `air_back`, `levity`,
  `air_neutral`). For `air_back` and `smash_down` the measure reads the
  wrong side (it measures reach against the first frame, any way); those
  need a look, not a number.

**What the engine has for feel** (it is a real model, not a gap): hitlag
from damage (`hit_response.rs`, 0.070 s at 11 damage, 0.5x to 1.5x), a
whole-match freeze, hitstun from launch, the launch law with growth, rage
and stale moves, DI / SDI / ASDI, screen shake by launch, a launch trail, a
finish zoom, an impact flash, slash arcs, hit and swing sounds.

**What it does not have** (found on the way; each is a feel lever):

- No per-move hitlag or shake multiplier: a heavy and a light hit differ
  only by their damage (`smash-parity-inventory.md`, row S0.3).
- His own effects: each of his moves draws the common slash. No ink, no
  apple, no page.
- The sheet publishes no hit polygons for him, so his reach is the table's
  box and not the quill in the picture.
- Rows that no state asks for: `shield_hold`, `hit_heavy`, `launch`,
  `turnaround`, `walk_stop`, `jab_2`, `jab_3`, `land`. A held guard draws
  `idle` (the sheet has no `block` row by that name).

## Next

1. **Play it.** Nothing here was seen in a window on a GPU.
2. **He guards and he dodges** in the duel (`BossGuard` is the port; a spot
   dodge row is on his sheet), and a hit on his guard is not a blow.
3. **He is launched.** A boss body takes hearts only. For "like a Smash
   character" he needs hitstun and a launch on the giant's back, and a way
   back up. This is the large step: a fighter's body under a boss's
   encounter (see the debts document).
4. **The jab is a string** (`jab_2`, `jab_3` are drawn), and his rows for
   guard, heavy hit and turn are bound.
5. **Hit polygons from the rig** for the quill, and his own effects.
6. **A facet** (`data/fighters/npc_gnu_ton.ron`): his weight (light) and
   his damage in percent. Until then he fights with the table's numbers.
7. **The hall**: a row in the hall of characters and a hall dialogue.
