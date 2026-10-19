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
