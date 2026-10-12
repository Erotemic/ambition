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
  28 moves (a jab string of three, three tilts, three smashes, five aerials,
  dash attack, taunt, five specials, the grab kit). The roster fighter plays it. The boss
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

**His smash attacks are the giant's** (Jon, 2026-10-11: "bayonetta style
smash attack where the gnu hand pops out and punches out of a portal ... in
the smash game that is, obviously he doesn't get those in the ambition
game"). He opens the line of a portal (before him, on the floor, over the
ground) and the fist of his giant comes out of it and goes back.

- The art is a sheet of his own, `gnu_ton_vfx` (the renderer's
  `targets/props/gnu_ton_vfx.py`): rows `gnu_fist_forward`, `gnu_fist_up`,
  `gnu_fist_down`. The fist is the boss fight's (`giant_art.fist`), so the
  two games show one giant.
- **The sheet has no portal in it.** It has the fore arm and the fist, cut
  where the line is. The engine draws the line: `bosses/gnu_ton/
  fist_lines.rs` puts a `PortalLightLine` (the portal presentation's look
  line, as at the giant's shoulders in the boss fight) where the arm is cut,
  for as long as the row plays. (The first cut of the sheet had a line
  painted in it. Jon: "we should not be authoring 3d black holes for
  portals. Use the portal presentation tool even though these are fake
  portals.")
- The move's `Vfx` event starts the row after the charge hold, so the fist
  is fully out when the move goes live. The hit volume is the fist's (the
  forward smash reaches 134 from him; his book reached 72).
- In the boss fight the same move is a blow of his book: `duel_spec`
  drops the event and puts the volume back at the book's reach.
- The fist is a picture and a box. It is not a body: nothing can strike it
  or stand on it.

**His weight.** `data/fighters/npc_gnu_ton.ron`: 0.86, a light fighter. His
table's damage is percent already, so the facet names no move.

## The boss fights back

With the giant's arms on it (phase 1) and the player on its back, or over it
in a jump, the back is a platform and he is a fighter on it. His conductor
(the module) says where he is and what he does; a native system
(`bosses/gnu_ton/duel.rs`) plays the move on his body. The giant does not
punch then: he is the fight.

- **He goes to the player.** Out of his reach he walks; from far he runs,
  and he strikes out of the run (his dash attack, with a slide to the
  player).
- **He chooses a move by where the player is**: at his elbow the jab (each
  third time the low sweep), at the quill's length the thrust (each fourth
  time the book), over him the flourish.
- **The jab is a string of three**: with the player still at his elbow, the
  next of it starts.
- **He rolls through the player** (each fifth choice at his elbow), with his
  book up in the middle of the roll, and strikes from the other side. With
  no room behind the player he rolls away.
- **His book is up between his moves** (`ambition.boss.guard`; the row
  `shield_hold`). A blow on it is a clang and no health. For each move the
  choreography says how long he is open: the move, and a time after it
  (`DUEL_MOVES`). So the way to hurt him is the way to hurt a fighter: make
  him miss, and strike before the book comes up.
- **A blow launches him**: away from the blow, in an arc, with no guard and
  no move until he has his feet again. The launch grows with each blow since
  the giant last threw (300, 450, 600 px/s), with the hearts of the blow, and
  with the part of his health he lost. The first blow moves him a step, the
  third throws him off the back.
- **Thrown off the back, he comes back**: he rises to its nearer end (the
  row `special_up`). A blow on the way launches him again.
- The third blow: the giant throws the player, as before.
- With the player gone from the back for 1.1 s, or when the arms go into
  their lines, he hops to his saddle.

What this is NOT: his launch is a path the conductor computes (gravity, the
ends of the back, the floor of the hall), not the engine's launch of a
fighter. The player's blow gives its hearts and its side, not its launch
vector; he has no DI and no tech. The roll and the string are his choice by
count, not an answer to what the player does: the conductor does not see the
player's swing. A fighter's body under a boss's encounter is still the large
step (the debts document).

To photograph a blow with no player that swings: `capture_scene
--boss-blow TICK[xHEARTS]`.

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

**A take of each verb** (`moveset_takes`, the sandbag at 40 px, 0 %; ticks
of 1/60 s; measured 2026-10-11 after the changes above):

| Verb | Live at | Result |
|---|---|---|
| jab | 4 | a string of three (`jab`, `jab_2`, `jab_3`) that cancels on hit; the second starts in the hitstun of the first |
| tilts (forward, up, down) | 5, 4, 3 | each hits |
| smashes | 44 to 46 | includes the take's charge hold |
| aerials | 4 to 7 | each hits but the forward air |
| specials | 6 to 14 | each hits |

The forward air does not hit in the take: the fighter is OVER the sandbag
then (0.2 px to the side), and the box starts 10 px before him. That is the
take's staging. A fighter who jumps at a standing one is hit.

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
- **An effect's frame is stretched and fades.** `draw_effect_clip` draws
  each frame into one square, so a frame the sheet trimmed to another shape
  is stretched (each effect sheet is trimmed). And `update_effects` fades
  each effect from its first instant (`alpha = 1 - t`), so the fist is at
  about 0.6 when it lands. For the fist the sheet is published with whole
  frames (`whole_frames` of the publisher) and declares that its frames own
  the fade (`FxSheet::fade`). Each other effect is as it was: a change of
  those needs a look at each. Debt 14 of `gnu-ton-fight-debts.md`.
- Rows that no state of the Smash game asks for: `hit_heavy`, `launch`,
  `turnaround`, `walk_stop`, `land`. (The boss fight draws `launch`, `fall`
  and `land` itself.) A held guard draws `idle` (the sheet has no `block`
  row by that name).

## Next

1. **Play it.** Nothing here was seen in a window on a GPU. The numbers of
   the fight on the back (`choreography.rs`, `arms`: speeds, the launch, the
   roll, how long he is open) are a first cut for a hand to tune.
2. **He answers the player**: the conductor's view of the player has a place
   and a size, and no swing. With the swing in the view he can roll or guard
   on a tell, and punish a miss. A guard that BREAKS is not built.
3. **A fighter's body under the boss's encounter**: the engine's launch (the
   blow's own vector, DI, a tech), in place of the conductor's path.
4. His aerials and his specials in the fight on the back (he has no jump
   there).
5. **Hit polygons from the rig** for the quill, and his own effects for the
   quill and the book (ink, a page). The fist has its own sound to get: it
   plays the giant's stomp.
6. **The fist as a thing of the fight**: a charged smash that makes the fist
   larger; a fist that a strong hit can clang with.
7. **The hall**: a row in the hall of characters and a hall dialogue.
