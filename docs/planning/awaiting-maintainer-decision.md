# Awaiting a maintainer decision

This file holds only open maintainer and product questions. Engineering work
that can proceed without a ruling goes in [`queue.md`](queue.md).

- Give each question one unique `Q<number>`. Do not reuse a retired number.
- Use the heading shape `## Q<number> — <question>`. Other documents link to
  these anchors, so do not change the text of a heading.
- Each entry states the question, the context needed to choose, the owner, and
  what changes for each answer.
- A queue row that waits on a question names it in a `**Blocked by:**` field.
- When a question is answered, record the ruling in
  [`maintainer-decisions.md`](maintainer-decisions.md), update the owner plan or
  source, and delete the question here.
- Before you file a question, make sure that no ruling already decides it. A
  decided direction with unfinished implementation is a queue row, not a
  question.
- Do not file "nothing uses X: keep or delete?". Current usage is not a deletion
  criterion (Q74 ruling, 2026-10-03); ask about meaning, authority or cost.

## What actually blocks architecture work today

This is the blocking set, not the important set. A question can matter and
block nothing.

**The blocking set is empty (2026-10-19).** The 2026-10-19 rulings closed
`Q157`, its one entry.

`scripts/check_blocking_set_names_every_gate.py` reads the queue's
`**Blocked by:**` fields and fails if a gate has no row in a table here. Each
table row links a `Q` that has a section on this page with at least two
options, written `* **(a) …`. Derive gates from the field, not from row prose.

## Gameplay and content

## Q159 — a player that wears a character with no posed body: is its art fitted to the box it keeps, or drawn at the size that character is built at?

Filed 2026-10-06. Blocks nothing: (a) is what the game does today.

A body that changes character keeps its collision box, unless the new
character is a posed body (a sheet-authored scale: the robots, Mary-O, Sanic),
which brings its own box. The renderer then fits the frame of the new
character's sheet to the box the body kept. Only a player changes character in
the shipped games. Measured in the shipped app, the main game's player
(a 30 by 48 box) wearing each character:

| worn character | drawn at (the frame's quad) | built at, when it is an NPC | drawn size over built size |
| --- | --- | --- | --- |
| kernel guide | 32.7 by 33.5 | 50.4 by 51.6 | 65% |
| companion dog | 36.0 by 30.0 | 108.6 by 90.6 | 33% |
| pirate raider | 90.9 by 58.9 | 90.9 by 58.9 | 100% (its body is 48 tall) |
| pirate admiral | 90.9 by 58.9 | 90.9 by 58.9 | 100% |
| the robots (posed) | their own scale | the same | 100% |

So a player that wears the dog is a dog a third of the size of the dog beside
it, in a box of the player's size. The simulation states the quad that is
drawn (`ActorRenderSize`, since 2026-10-06), so a hand or a head of that art
is where it is drawn in either answer.

* **(a) Fitted to the kept box.** The art is the size of the body's collision
  box, as today. A worn character is as big as the player, whatever it is.
* **(b) Drawn at the built size.** The art is the size the character is built
  at, and the collision box stays the player's. The quad rule then moves to
  the quad the seed resolves, and the box no longer decides the drawn scale
  after a change of character. A dog-sized art on a player-sized box needs a
  second decision: whether the box follows the art too
  (`BaselineBoundary::Replacement` says it does not, on purpose).

**Default in force until you rule:** (a), the current behaviour.

**Maintainer guidance, 2026-10-19 (not a ruling):** keep (a) provisionally,
and read no approval of it as the permanent rule. Do not change a body's
physics box to match a worn character either. Keep visual representation and
mechanical body geometry distinct, and add no new scale authority. The
question stays open until a gameplay and visual comparison across several
character shapes exists. It blocks nothing.

## Architecture and engine policy

## Q168 — should the robot family's forward slash reach six body widths?

**Owner:** the robot family's art (`robot_side.rs` `attack_hitboxes` in the
sprite renderer) and the drawn-reach brains held in REACH-VIEW ("two
geometries for one move").

**Context (measured 2026-10-19):** the robot family (`robot`,
`player_robot_v2`, the duel robot) authors its forward slash as a cone
`1.34 × frame width` long, drawn at the body's own scale, so it reaches 5.9
body widths: 137 px in the world for a 48 px body. Its strike box is its
drawn slash (§7.2), so it has always hit from there. Of the 23 published
sheets with hit polygons, every other one reaches about one body width or
less (Alice's air forward 2.25). The player's v3 robot swing reaches about
3.4. Brains that read the box that lands (built, held) make the duel robot
poke from 150 px, and the PCA then never swings in the duel.

**The choice:**

- **(a) Shorten it (recommended):** author the robot family's slash at about
  the player robot's reach, then republish those targets. The robot keeps a
  long laser sword, and duels and the drawn-reach brains work as pinned.
- **(b) Keep it:** the robot is a deliberately long-ranged duelist. The
  drawn-reach brains land, the PCA brain must learn to approach a longer
  reach, and the duel's pins change until it does.

**What it blocks:** nothing in the blocking set. REACH-VIEW's re-apply waits
on it.
