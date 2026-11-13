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

## Q174 — What does a death cost in Ambition?

**Context (2026-11-12, PLAYER-DEATH).** Jon asked for a presented death and
a rule for its cost, and named the options below; "it would be nice if we
could put a twist on it". Ambition's death beat is 0 s today and costs
nothing: the room goes back to the checkpoint. Default taken: **(b) with
(c)'s fee**: just before the blow lands, a portal opens and yoinks the
body away ("You're too valuable to be destroyed"); the gate network's
insurance charges a safety fee from the currency, and the fee is left at
the spot as a recoverable cache (a sealed parcel at the rescue portal's
mouth) that going back returns. Too little to pay: the bigger penalty is
that the parcel holds what was carried and the rescue lands at the last
checkpoint in debt (the next currency earned pays the debt first).

**Options:**

* **(a) Hollow Knight style:** a penalty undone by finding your way back to
  your body and fixing it.
* **(b) The rescue portal:** a portal yoinks you away before the death
  (now, with (c)'s fee).
* **(c) A lost currency you can win back,** with a bigger penalty when you
  die without enough to pay the safety fee.
* **(d) No cost:** the presentation only; the checkpoint restore is the
  penalty.

**Owner:** `session/death.rs` (the beat), the bag (currency) and the
presentation. Engineering is the PLAYER-DEATH row.

**Built under the default (2026-11-13):** the fee is 50 coins
(`room_rules.ron`, `death_fee`); the parcel lies at the last safe ground
where the body was taken; one parcel a participant (a second death before
the fetch loses the first, as Hollow Knight's shade); the debt is a negative
wallet of at most one fee. The rescue portal's own look is still to come.

## Q175 — Is the final boss's giant one of the named agents?

**Context (2026-11-12, FINAL-BOSS).** Jon: the bossofthegame stands on the
shoulders of a giant square robot with an electric guitar. The brainstorm
has an agent carry the Boss on its shoulders, and names the agents Gepetto,
Clause and Wren. Default taken: **(b)**, the giant is its own agent, **Full
Stack**, built like a guitar amplifier stack (he stands on the full stack).

**Options:**

* **(a) A named agent at its largest setting** (Gepetto, Clause or Wren,
  "extra high"), so the fight is that agent's temple boss too.
* **(b) Its own agent, Full Stack** (now).

**Owner:** `docs/planning/game/final-boss.md`; the catalog row
`npc_full_stack` and its barks. Each option is content only (a name, barks
and art).

## Q176 — Does emptying the final boss's bar count as his voluntary retirement?

**Context (2026-11-12, FINAL-BOSS).** The endgame brainstorm: "the only
definitive in-game victory is getting the Boss to retire voluntarily (not
depleting a health bar)". The first cut is a boss fight with a bar. Default
taken: **(a)**, the bar is his **resolve**; when it empties he does not die,
he sits down and chooses to retire (the green room's scene).

**Options:**

* **(a) The bar is his resolve** (now): a worthy challenge is what makes him
  retire.
* **(b) A condition that is not damage**: for example, survive his encore
  for a time, or make him laugh, while the bar only gates the phases.
* **(c) Both**: the bar empties, and then a last beat that is not damage
  (he offers the crown; the player must take it or refuse it).

**Owner:** `boss_encounters/bossofthegame.ron`, the stage's rooms and the
cutscene library. (b) needs an encounter trigger that is not health (an
`External` gate); (a) and (c) are content only.

## Q177 — Does a living exploding mite go off?

**Context (2026-11-12, MATH-POLISH report 5).** Jon: "The exploding mites
should only trigger their explode fuse when they die. Right now they just
explode when I enter the room." The mite ran at its foe, planted a 0.9 s
fuse in reach and ended itself, and the blast came at that kill. Default
taken: **(a)**. Under both options the blast comes at the end of the death
row, which is the fuse.

**Options:**

* **(a) It never goes off alive** (now): it snaps with its mandibles
  (`exploding_mite_snap`); killed, its fuse lights and it goes off when the
  death row ends. Its threat is the corpse: kill it at range or step away.
* **(b) It still goes off alive**: in reach it ends itself at once (the
  `Detonate` move with a short plant), so its death row, the fuse, starts
  there and the blast follows.

**Owner:** `character_catalog.ron` (`npc_exploding_mite`, its action set).
(b) is content only: `melee: Some(Detonate(fuse_s: 0.15, reach_px: 40.0))`.
