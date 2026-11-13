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

**The blocking set is empty (rechecked 2026-11-06).** The last blocking
question, `Q157`, was ruled on 2026-10-19. Neither `Q168` nor `Q170`
blocked the execution queue.

`scripts/checks/check_blocking_set_names_every_gate.py` reads the queue's
`**Blocked by:**` fields and fails if a gate has no row in a table here. Each
table row links a `Q` that has a section on this page with at least two
options, written `* **(a) …`. Derive gates from the field, not from row prose.

## Open questions

`Q168` and `Q170` were decided on 2026-11-06 and are recorded in
[`maintainer-decisions.md`](maintainer-decisions.md). Their remaining
engineering and content work is indexed in [`tracks.md`](tracks.md), not held
as a product-decision gate.

## Q171 — Does holding down crouch at once, or look down first?

**Asked 2026-11-11 (Jon, unsure):** *"pressing down maybe shouldn't
immediately put you into a crouch, maybe it just has your head look down. And
after a bit of time it also translates the camera a little bit down so you can
actually 'look down'. The same 'look up' should happen for when you press and
hold up."*

**What exists:** up held while standing turns the head up (`idle_look_up`,
over a walk too). The camera already leans: a full look up held still, or a
full crouch held still, for `LOOK_DELAY_SECS` (0.8 s) leans the view by
`LOOK_SHARE_OF_VIEW` (0.3) of its height over `LOOK_RAMP_SECS` (0.5 s)
(`ambition_sim_view::camera_snapshot::camera_look`). Down crouches at once:
a smaller hurtbox, the crawl, the down+jump drop through a one-way, and the
crouching attacks all key on it.

**Options:**

* **(a) Keep the crouch at once** (now). Down is the crouch; the camera's
  look down is the crouch held still. Nothing changes.
* **(b) Down tilts the head first, and crouches after a hold** (as long as
  the camera's delay, or shorter). A crouch to dodge comes later by that
  hold, and every crouch-keyed verb (crawl, drop through, crouch attacks)
  waits for it or keys on the down input instead.
* **(c) Down tilts the head, and the crouch moves to another input** (or to
  down while moving). The largest change: the crouch's verbs are re-bound.

**Owner:** the movement kernel's crouch (`update_body_mode`) and the pose
view (`next_gaze_secs`). Engineering for (b) or (c) is a queue row once ruled.

## Q172 — Do melee strikes and guards need swept (continuous) contact?

**Asked by:** the portal plan's remaining acceptance (PSIM, "swept guard
contact" and "moving-target hurt contact"), 2026-11-12.

**What exists:** projectiles and loose items meet moving targets on the
world clock, through portal cuts, scale and reflection (`b3c975837`,
`70c9fbf75`). A melee strike and a guard are tested by overlap once a tick,
in each body's admitted material through the cuts, and the deflect spark and
knockback read the midpoint of the two touching pieces. No shipped strike
has been seen to pass a guard or a body in one tick: a strike that reaches a
body's hurt parts also meets the guard in front of them that tick, and the
fastest measured limb (GNU-ton's portal blows, 41–60 world units a tick)
moves less than its own volume plus the smallest body.

**Options:**

* **(a) Keep overlap contact for melee** (now). The swept rules stay with
  projectiles and items, where a tick's travel exceeds the volume. The PSIM
  rows for melee CCD close as not required.
* **(b) Sweep melee strikes against hurt parts and guards** over the tick,
  with contact time and point, as projectiles do. The cost is one swept
  query per active strike and recipient; it changes no shipped outcome seen
  so far, and would matter for a faster or thinner strike authored later.

**Owner:** `ambition_combat::hitbox` (`apply_hitbox_damage`, `turned_aside`).
Engineering for (b) is a queue row once ruled.

## Q173 — Does the creator always die in the raid?

**Context (2026-11-12, INTRO-RAID).** Jon: the creator follows the robot
into the second room, "where he dies, though he doesn't actually die now".
Built: he fights beside the robot (he can fall in the fight, 12 HP), and
when the robot reaches the corridor's far end his last words play and end
on a scripted `Fall`. Default taken: **he always dies there**; if he fell in
the fight already, the same words play as the ones he had left.

**Options:**

* **(a) Always, at the way out** (now).
* **(b) Only if he falls in the fight**; at the way out, a living creator
  says something else and stays behind (or goes another way).
* **(c) Never in the intro**; his death moves later in the story.

**Owner:** `data/cutscenes/intro.ron` (`creator_final_fragment`) and the
volume at the corridor's far end in `intro.ldtk`. Each option is content
only.
