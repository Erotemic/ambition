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

**The blocking set is empty (2026-11-26).** Q178, its last question, was
ruled on 2026-11-26 with the rest of Q171–Q187 except Q179, which is
deferred and blocks nothing. A ruled question's remaining implementation is
a queue row, not a gate here.

| Question | Blocks |
|---|---|

`scripts/checks/check_blocking_set_names_every_gate.py` reads the queue's
`**Blocked by:**` fields and fails if a gate has no row in a table here. Each
table row links a `Q` that has a section on this page with at least two
options, written `* **(a) …`. Derive gates from the field, not from row prose.

## Open questions

Q171–Q178, Q180–Q183 and Q185–Q187 were ruled on 2026-11-26 and are
recorded in [`maintainer-decisions.md`](maintainer-decisions.md); their
implementation work is in [`queue.md`](queue.md). Q179 stays open, deferred.

`Q168` and `Q170` were decided on 2026-11-06 and are recorded in
[`maintainer-decisions.md`](maintainer-decisions.md). Their remaining
engineering and content work is indexed in [`tracks.md`](tracks.md), not held
as a product-decision gate.

## Q179 — What does a quick press of Precision Blink do, if two quick presses are the directional blink?

**Deferred (Jon, 2026-11-26).** Not decided now, and it blocks no other
work. The current quick press stays until it is ruled.

**Context (2026-11-18, ABILITY-VARIANTS, [loadout open 2](game/ability-loadout.md#open)).**
Jon's notes: "a quick press does the intuitive thing, and two quick presses
give the directional blink". Today a quick press (released before
`BLINK_HOLD_THRESHOLD`) already is a directional blink: 190 px along the
movement stick, or forward along the facing with no stick
(`movement::control::handle_blink_clusters`). A hold aims (levels 2 and 3).
So the notes ask for a NEW single-press behaviour, and what "intuitive"
means is not in the notes. Contextual Blink (behind the nearest foe) is its
own row of the grid, so it is not the answer here.

**Options:**

* **(a) The quick press keeps the body's travel** (recommended default): it
  blinks 190 px along the body's current velocity when it moves, else along
  the facing, ignoring the stick; a double press within 0.25 s is the
  stick-directed blink of today. A running or falling body goes on the way
  it was going, and a deliberate direction takes a double press.
* **(b) The quick press finds a footing**: it blinks forward to the nearest
  place the body can stand within 190 px (a ledge ahead, the floor below a
  gap), else forward; the double press is the stick-directed blink.
* **(c) Keep the quick press as it is** (stick-directed) and give the double
  press a different meaning (for example the full 430 px precision distance
  along the stick, with no aim).

**Owner:** the movement kernel's blink control (`ambition_platformer2d_core`)
and the loadout plan. Engineering adds the double-press window as body
maneuver state (rollback, codec) whichever option is chosen.

