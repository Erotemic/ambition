# Awaiting a maintainer decision

Only questions whose next step is **Jon's product/authoring judgement** belong
here. Engineering work goes to [`queue.md`](queue.md) or [`tracks.md`](tracks.md).
Answered rulings belong in [`maintainer-decisions.md`](maintainer-decisions.md);
the investigation that led to an answered question remains available in git
history.

This file intentionally does not retain answered decision transcripts.

> ✔ **PREMISES RE-MEASURED 2026-09-03. Every question here still rests on a fact
> that is still true**, which is the thing that decides whether answering them is
> a good use of your time. A decision resting on a premise the code has moved past
> costs you the answer AND the discovery that it was moot.
>
> Checked against HEAD: the windbox vocabulary is still authored by nothing (39);
> the puppy slug, stochastic parrot and burning flying shark still carry no
> `standing_height` (36); `REACH_TOLERANCE` is still one global `2.0` at
> `ambition_characters/src/brain/fighter/options.rs:183` (35);
> `BodyMelee::ranged_cooldown` is still the implemented half with presentation
> unresolved (33); the `"gauntlet_fireball"` visual still reproduces the old
> sprite rather than the catalog energy ball (42). ⇒ **And none of the fifteen has
> quietly been answered in [`maintainer-decisions.md`](maintainer-decisions.md)**,
> which this file's own rule would make a defect.
>
> ⚠ Two entries gained a fact worth having before you answer: 36 needs FOUR rows
> rather than three (the shark has a separate hall variant), and 39's answer costs
> one authored field on one move rather than an implementation.
>
> ✔ **AND THE QUOTED NUMBERS WERE RE-RUN, not just the premises.** Several entries
> name the script that produced their figure, which makes checking them one
> command each:
>
> | instrument | entry's figure | re-run 2026-09-03 |
> |---|---|---|
> | `measure_character_data_coverage.py` | 23 of 149 | **23 of 149** ✔ |
> | `measure_fx_row_reachability.py` | 34 of 35 rows unnamed | **34 of 35** ✔ (`george_booul_vfx` 20/21, `pirate_admiral_vfx` 14/14) |
> | `measure_orphan_shipped_pages.py` | 487 files, 14.2 MB | **438 files, 13.4 MB** — drifted, corrected in place |
>
> ⇒ Two of three exact after a day, one drifted by 49 files. **The entries that
> name their instrument are the ones that could be checked at all**, which is the
> argument for naming it every time a number goes into this file.

⛔ **NUMBERING: TAKE ONE ABOVE THE HIGHEST NUMBER PRESENT, and check first.**
The entries are NOT in numeric order — the top block runs newest-first and an
older ascending block follows it — so the number nearest the insertion point is
not the highest. Two sessions have now collided by assuming it was
(`cce4f764b` "Renumber my decision to 30: I collided with an existing 28", and
again on 2026-09-03). ⚠ And because answered entries are DELETED rather than
archived, gaps are normal and a missing number is not a free one: re-using it
silently re-points every page that cited the original.

    grep -oE '^### [0-9]+\.' awaiting-maintainer-decision.md | tr -d '#. ' | sort -n | tail -1

## Open decisions

### ~~50. May a fighter leave the frame, or must the camera always contain the cast?~~ (WITHDRAWN 2026-09-04 — no fighter was ever outside a frame)

⛔⛔ **DO NOT ANSWER THIS. ITS PREMISE IS FALSE, and this file's own header names
that as the failure worth avoiding: *"A decision resting on a premise the code
has moved past costs you the answer AND the discovery that it was moot."***

**The frame nobody framed was `ResolvedCameraSnapshot::default()`, verbatim.**
`local_view_facts()` puts the default on every view at spawn so a reader never
meets a view whose state is missing; the resolver honours *"callers must not
invent a world-origin fallback"* by returning without writing when the cast is
unresolvable; and `CameraSnapshot2d::default()` is a ZERO centre with
`default_base_view()`, whose own comment records that the default moved to
568x320 on 2026-09-03 — the exact dimensions every failure reported.

⇒ **Two individually correct decisions composed into a snapshot that was
syntactically present and semantically a lie. Nobody wrote the fallback.** So no
fighter was ever outside a frame: the test was measuring a default as though it
were a camera, on the one tick (`t3`) where bodies exist and the cast has not
resolved.

✔ **Fixed rather than ruled on** (`92f2f597b`): `ResolvedCameraSnapshot` is
`Option<ResolvedCameraFrame>`, so an unframed view says so, the compiler asks
every reader, and the frame checks skip that tick — which is what their
`continue` always meant.

✔✔ **THE FIX IS MEASURED NOW (2026-09-04). The union at `5c320ebb5` reads
**7,137 passed / 0 failed**, `cargo exit: 0` — the first fully green union this
repository has recorded, and the framing test is in it. So the caveat below is
DISCHARGED: `✔ Fixed` may be read as measured. ⛔ It was a mechanism argument for
about a day, and the paragraph that said so is kept below rather than deleted,
because the reason it was written is the reusable part.

✔✔ **AND THE FIX IS NOW MEASURED, not argued** (was flagged as unproven for
several hours, deliberately). The failing arm was always a UNION-features run, and
the default-features suite passed both before and after for different reasons, so
it was never evidence. ⇒ Re-run on the fixed tree:
`cargo test --workspace --features "<the 82-entry union>" --test smash_it` →
**43 passed, 0 failed**, where the identical command previously gave 42 passed and
`every_live_fighter_stays_inside_the_frame` FAILED. ⭐ Independently, the gate's
full union at `5c320ebb5` is **7,137 passed / 0 failed** — the first fully green
union in that row's history.

⚠ Kept as a note rather than deleted, because the gap between *mechanism* and
*measurement* is what this page recorded twice today in the other direction: a doc
comment that named the symptom while describing a branch that never ran, and my
own placement-race inference that a four-minute probe refuted.

⚠ **Neither original reading survives**, which is the point. A containment
CONTRACT has nothing to contain; a tolerance MARGIN would have been chosen
against a number that measures nothing, and would have hidden the defect
permanently by making the test pass. ⛔ Both cheap fixes were wrong-shaped, and
not for the reason the entry gave.

ⓘ The investigation below is kept because the retractions in it are the useful
part — a `follow_world` diagnostic, a probe that refuted the placement inference
it seemed to support, and a doc comment that named the symptom exactly while
describing a branch that never ran.

### ⓘ 50 (superseded, for the record). May a fighter leave the frame by 16 units for one body-frame?

`the_stage_kills::every_live_fighter_stays_inside_the_frame`
(`game/ambition_demo_smash_app/tests/the_stage_kills.rs:1094`) is RED under the
gate's feature union, and it is the only survivor of the smash target — thirty-odd
failures went to one. Its message:

> *"a live fighter was drawn OUTSIDE the frame on 1 body-frames, worst 16 units
> past the edge — the knockout that decides the match happens off-screen:
> t3 seat 1 at (416,204) is 16 units outside a 800x450 frame centred (0,0)"*

⇒ **The engineering half is not in question.** The test measures what it says it
measures, one body-frame is genuinely outside, and 16 units on a 800-wide frame
is 4% of the half-width. What is unanswered is whether that is a defect at all.

Two readings, and they lead to different work:

- **A containment CONTRACT** — the camera must always contain every live
  fighter, so one frame outside is a bug in the camera's follow/zoom and the
  test is correctly red.
- **A tuning MARGIN** — a platform fighter's camera is allowed to lag a fast
  body briefly, the assertion's tolerance is the thing that is wrong, and the
  test should carry a stated allowance instead of zero.

⛔⛔ **RE-MEASURED 2026-09-04 AND THE NUMBERS ARE EIGHT TIMES WORSE — the two
readings above may BOTH be wrong.** Reproduced with one command on an
already-built tree, no full union run needed:

```
cargo test --workspace --features "$(the 82-entry union)" --test smash_it
```

> *"a live fighter was drawn OUTSIDE the frame on 2 body-frames, worst 132 units
> past the edge …*
> *t3 seat 0 at (224,204) is 44 units outside a 568x320 frame centred (0,0)*
> *t3 seat 1 at (416,204) is 132 units outside a 568x320 frame centred (0,0)"*

⇒ **Three things changed from the reading above, and each one moves the answer:**

1. **16 units → 132.** On this frame that is **46% of the half-width**, not 4%.
   A camera that lags a fast body by 4% is a tuning margin; one that misses it by
   nearly half a screen is not.
2. **One fighter → BOTH.** Neither seat is inside the frame. A follow camera
   trailing a launched body leaves ONE fighter behind; it does not lose both.
3. **The frame itself changed, 800x450 → 568x320.** ⚠ **CORRECTED same day** — I
   first wrote that the viewport is "not stable across runs" and used it to argue
   any tolerance would be chosen against noise. That was wrong and it overstated.
   THREE runs since (mine, and two of the peer session's, one of them under
   `bevy_ecs/debug`) all report `568x320` and the **identical** seat coordinates
   `(224,204)` and `(416,204)`. ⇒ The frame changed ONCE, between the original
   800x450 measurement and now, and has been stable at the new value since. A
   tolerance would be chosen against a reproducible number — the objection to
   choosing one is the paragraph below, not this.

⭐⭐ **AND THE CENTRE IS THE TELL: `(0,0)`, with both fighters at y≈204.** The
camera is not lagging — it is sitting on the WORLD ORIGIN while the match happens
elsewhere. ⛔ `CastFraming`'s own doc forbids exactly this: *"Empty or
unresolvable casts return `None`; **callers must not invent a world-origin
fallback**."* And `desired_target_world` is `input.focus.stable_center()`, which
reads `center_world` — origin when the focus has not resolved.

⛔⛔ **MEASURED, NOT INFERRED — and it clears the camera.** The test now reports
what the camera was FOLLOWING (`ResolvedCameraSnapshot::follow_world`), and under
the union it prints **`following (0,0)`** on both failing body-frames. ⇒ The
camera is not lost and is not inventing anything: it is faithfully framing a body
that IS at the world origin, while the two fighters are at y≈204.

⚠ **I then inferred a placement race from that, and a probe REFUTED it. Recorded
because the wrong inference is instructive.** The reasoning was: the resolver's
unresolvable-cast arm `return`s without publishing, so the origin must come from
the arm ABOVE it, which follows a real body — therefore a body must be sitting at
the origin, unplaced.

⛔ **No body is ever at the origin.** `probe_where_bodies_are_before_the_match_settles`
walks every entity with `BodyKinematics` for the first twelve ticks of a match:

```
[probe] t0  0 bodies:
[probe] t1  0 bodies:
[probe] t2  0 bodies:
[probe] t3  2 bodies: seat0@(224,204)  seat1@(416,204)
[probe] t4  2 bodies: seat0@(224,204)  seat1@(416,204)
```

⇒ Zero bodies for three ticks, then exactly two, **both already at their spawn
points**. There is no third body, no unplaced body, and nothing at `(0,0)` for a
camera to follow. ⚠ The probe runs at DEFAULT features, where the test is green —
deliberately, because the union is what makes a snapshot exist at `t3`, not what
puts something at the origin. A probe under the union features is running to close
that gap.

⇒ **And that reopened it in the right place.** `follow_world` reports `(0,0)`
while nothing in the world is at `(0,0)` — not a camera following an unplaced
body, but a follow point corresponding to nothing.

⭐⭐⭐ **SOLVED — the failing frame is the component's `Default`, verbatim.** Four
checked facts, and the last one is the proof:

1. `local_view_facts()` (production, `camera_snapshot.rs:1712`) puts
   `ResolvedCameraSnapshot::default()` on every view at spawn. Its comment: *"a
   reader must never see a frame where the view exists and its state does not."*
2. `CameraSnapshot2d::default()` is `center_world: Vec2::ZERO` with
   `visible_view: default_base_view()`.
3. The resolver at `camera_snapshot.rs:1567` is the **only** production writer,
   and it `return`s without writing when the cast is unresolvable — honouring
   *"callers must not invent a world-origin fallback"* by staying silent.
4. ⭐ **`default_base_view()` carries a comment reading *"the default moved to
   `Duel` (568x320) on 2026-09-03"*.** ⇒ The failure reports a **568x320 frame
   centred (0,0)** — that is not a camera that resolved badly, it is the Default
   with its own dimensions, and the 800x450 in the original report is simply the
   PREVIOUS default. ⚠ That also explains the frame-size change I first mistook
   for instability: the default changed, not the camera.

⛔⛔ **So two individually correct decisions compose into the behaviour the
contract forbids.** The resolver refuses to invent an origin — correctly. The
bundle guarantees a snapshot always exists — also reasonable. Between them, a
reader gets a snapshot that is syntactically present and semantically a lie: a
real-looking frame, at the world origin, containing nobody. ⇒ Nobody wrote the
fallback; it fell out of `Default`.

⚠ **Why `t3` and only `t3`**: bodies first exist at `t3` (the probe shows zero for
`t0`–`t2`), the cast is not framable yet on that tick, so the resolver returns and
the Default stands. By `t4` it resolves. Under default features the test skips
those frames for an unrelated reason, which is why this only reddens under the
union.

⇒ **THE QUESTION IS NOW SMALL AND IT IS ENGINEERING, NOT FEEL.** Should
`ResolvedCameraSnapshot` be able to say *"not resolved yet"*? ⭐ A flag or an
`Option` lets a reader distinguish "the view has not been framed" from "the view
is framed on the origin", which is the distinction that does not currently exist.
⚠ It touches `ambition_render`, `local_view` and two tools, so it is a small
cross-crate change rather than a local one — which is why it is here rather than
already done.

⇒ **So the likely reading is a THIRD one neither option names: at `t3` the cast
has not resolved yet, and a snapshot is published anyway describing a frame that
contains nobody.** ⚠ Both failing frames are at `t3` out of 600+ observed, so
whatever it is, it is a one-tick startup transient and the camera is correct for
the rest of the match. The ease path is not the culprit — it already ADOPTS on
first resolve rather than easing from zero (`!state.target_initialized` →
`live_target_world = desired_target_world`), which is the same rule
`presented_roll_radians` documents as *"a view must open already oriented, not
spin up from zero."*

⇒ **Which makes the question sharper and cheaper than it was — but it is NOT yet
the question I claimed.** I narrowed it twice and the second narrowing was wrong:
*"may a match present a tick in which the followed body has not been placed?"*
assumed an unplaced body that the probe shows does not exist. ⚠ What is
established is only the first narrowing: it is not a tolerance question, because
132 units on a 568-wide frame with BOTH fighters outside is not a camera lagging.
⇒ The live question is still an engineering one and not yours until the union
probe reports. ⭐ If no, this is an engine defect
with a named contract already written down, the fix is upstream of the camera,
and the test is correctly red. If yes, the test should skip unresolved frames —
and that is a one-line change reading a fact that exists, not a tolerance pulled
out of the air.

⚠ **What I have NOT established.** Why the union feature set makes this appear at
all: under default features the whole target is green (42 passed, verified), and
the test's `let Some(view) else { continue }` means the snapshot simply is not
there at `t3` by default. Something in the 82 features publishes it a tick
earlier. ⛔ I did not bisect the features to find which — that is real work and it
is only worth doing if the answer to the question above is *"no"*.

⭐ **One thing this DOES settle, and it was a live disagreement:** the failure is
not a load or contention artifact. This arm ran ONE test binary with nothing else
building, and it reproduced. The cause is the feature set.

⭐⭐ **And the values are reproducible even though the occurrence is not.** Three
separate runs — this one, and two from the peer session including one under
`bevy_ecs/debug` — produce the SAME two seats at the SAME coordinates in the SAME
frame. ⛔ A load-sensitive failure does not land on identical coordinates three
times. ⇒ Whatever this is, it is deterministic given the feature set, which means
it can be chased directly rather than sampled.

⚠ **A related reading has been retracted at the source.** This test was filed for
a while alongside two others as "three tests sharing a load signature". The peer
session withdrew that grouping (`dbf07bd6f`): one was a parameter panic and is
fixed, one is a parameter panic that has fired once in three runs, and this one
is an assertion about a camera centre. ⇒ They share no mechanism, and grouping
them let each stand as evidence for the others — which is how a load story
survived three investigations without ever producing one.

⚠ **Why it is yours rather than mine**: which one is right depends on how a
knockout should READ, and the test's own text says the stake — *"the knockout
that decides the match happens off-screen"*. That is a feel judgement about the
moment the match is decided on, not an engine fact. ⛔ And the cheap fixes are
both wrong-shaped without the ruling: widening the tolerance answers it by
accident, and chasing the camera answers it by assuming.

⚠ Related but separate: entry 49 is also a `the_stage_kills` question. They are
independent — that one is about CPU divergence, this one about framing.

### 49. Is near-identical CPU play on a symmetric stage acceptable, or a defect?

A test has been deferring this to you since before its queue row was pruned, and
the deferral is the only record left of it.

`two_cpus_wearing_one_character_stop_being_a_perfect_reflection`
(`game/ambition_demo_smash_app/tests/the_stage_kills.rs:1806`) seats two CPUs on
the SAME character on a mirrored stage and asserts they diverge by more than a
pixel. It passes. Its failure text says what a pass means and hands you the
question in the same breath:

> *"⛔ this is NOT one mind played twice — the two seats draw from different
> streams, and the sibling guards listed above prove it. What it says is that a
> symmetric stage plus symmetric information leaves two different streams almost
> nothing to diverge ON at this difficulty. Whether that is acceptable is a
> product decision (queue D167); do NOT answer it by unmirroring the spawns or by
> adding noise."*

⇒ **The engineering half is settled and guarded** — the determinism is real, the
divergence is real, and the test forbids the two cheap fixes that would hide the
question. What is unanswered is whether two CPUs that play almost identically on
a symmetric stage read as a broken AI or as a fair mirror match.

⛔ **Queue row `D167` no longer exists.** It is in no live planning document — not
`queue.md`, not `tracks.md`, not this file — and survives only in the archived
pre-prune queue. So the question was never answered and never re-filed; it fell
out of the planning system and its only trace is an assertion message nobody
reads unless the test fails. Filed here 2026-09-03 to put it back where a
decision can be made.

### 47. TwinTrack's simultaneity limit: where does it live while the exhibit is parked? (re-filed 2026-09-03; was 28)

Both TwinTrack panes render ONE instant of the simulation's coordinate time, so
they can disagree about optics (light delay, aberration, Doppler) and NOT about
simultaneity — which is what the twin paradox actually is. This was question 28
here and the entry vanished — not answered and archived, just absent (yardrat,
2026-09-03: no `### 28` in this file and no row in `maintainer-decisions.md` names
it; the parked demo page `demos/twintrack.md` was the only record). Re-filed so
it has a home again. **The question:** while TwinTrack is parked, does the limit
belong in `engine/relativity.md` beside the spacetime-diagram design (which uses
"simultaneity" in a different sense and otherwise reads as though the exhibit
already shows it), or does the parked page stay the record? Default if nobody
rules: a one-line "known limit" note in `engine/relativity.md` pointing at the
demo page.

⇒ **Convention from the same finding, applied from here on:** when a question
leaves this file, its answer row in `maintainer-decisions.md` NAMES THE NUMBER
it closes ("closes 47"). Only 2 of that file's 100 rows do today, which is why a
dropped question and an answered one look the same; with the number on the
answer, a dropped one is a set difference.
### 48. The boss-crate reassessment you asked for on 2026-07-16 is now due

Your ruling that day (`maintainer-decisions.md`, 2026-07-16) was *"defer any boss
crate carve until boss behavior converges onto the canonical moveset/action
path"*, with a follow-up in the same row: *"reassess afterward whether a separate
boss crate still exists as a coherent subsystem."*

⭐ **The carve landed on 2026-08-17** — `725de8c26`, *"Carve the boss domain out
of the actor monolith into ambition_boss_encounter"* — so "afterward" arrived
seventeen days ago and nothing asked the follow-up question.

Measured at HEAD, as the input to it:

| | |
|---|---|
| size | **47 files, 14,635 lines** |
| in-tree consumers | **9** — `actor_monolith`, `runtime`, `provider`, `damage`, `sim_view`, `abilities`, `content_cli`, the `platformer2d` facade, and `ambition_content` |
| closure | in `never_asked_for`: a movement-only game links it |

⇒ **The engineering reading is that it does cohere** — a domain with nine
consumers is not a grab-bag someone forgot to delete. ⚠ But nine consumers is
also a lot of surface for one domain, and whether that breadth is the boss
domain being genuinely central or the carve having taken too much with it is a
judgement about what a boss IS, which is why this is here rather than in
`tracks.md`.

⛔ **Related and stale:** `tracks.md`'s trigger list still carries *"Boss crate
extraction — wait until boss vocabulary/ownership is coherent"* under the heading
**"Do not promote these until the trigger exists"**. The trigger fired a month
ago. That row wants deleting or rewriting as the reassessment; it is in another
agent's hot file, so it is reported rather than edited.

### 46. Does 1-1 want a fourth ?-block over floor, so the fire form's floor-refusal can be played?

`refuse_a_weaker_form_pickup` is Mary-O's rule that a form on the FLOOR may not
replace a stronger one. It now has a played acceptance test on the shipped app
(`weaker_form_refusal.rs`), but only for the EQUAL rung — a tall Mary-O meeting
a second wand. The strictly-weaker rung (fire meeting a wand) is **unreachable
in 1-1 as authored**, and the obstacle is level geometry rather than engineering.

Reaching fire spends TWO of 1-1's three `Toward(Lantern)` ?-blocks
(small→wand→tall, tall→lantern→fire), so a wand left walking needs the third.
⛔ **The third, at x=1920, stands over a pit.** Measured by dropping a body into
its column: from above she lands ON the block (centre 256, feet on its top
face); from below the face she falls to y≈969 in a 448-tall room, dies, and
respawns at the level start. No body can stand under it to bonk it.

⚠ **This is not a hole in the rule and not a missing test of it.** The equal
rung is the stronger arm for the comparison the rule actually makes — a `<`
written where `<=` belongs still refuses a wand offered to a fire Mary-O, and
lets a second wand re-equip a tall one — so the boundary is covered and the
interior is not. The question is only whether the interior is worth authoring
for.

Choose one:

- **author a fourth ?-block over floor in 1-1** — smallest change, and it also
  gives a player a place to reach fire without crossing the pit;
- **build the fixture in 1-3 instead** — it already authors a `Question` at 256
  AND an `AlwaysWand` at 1696, so the scenario exists there with no new content;
- **leave it** — the boundary arm covers the comparison, and the interior arm
  would cost a scripted pit-edge jump, which is a test about jump tuning wearing
  this rule's name.

⚠ **AND THIS QUESTION IS OLDER THAN THIS ROW.**
[`demos/super-mary-o.md`](demos/super-mary-o.md) has carried a product question
since 2026-08-14 about the fire form's DISCOVERABILITY — *"should the beacon
walk to you like the wand, should 1-1 place a reachable second block earlier, or
is 'the reward waits up there and you must climb for it' the intended feel?"* —
reached from the other direction, a player who could not find the reward. Same
level, same three blocks, and answering either answers both. The new fact this
row adds is that the third block has no standing position under it at all.

⛔ No content was authored to answer this: authored levels are Jon's.

### 37. Should the F9 rollback proof pulse survive a gameplay-session change?

`LocalSessionPolicy::check_distance` is raised by the F9 proof pulse and returns
to normal only when that pulse finishes. If the player quits to title during the
pulse and launches another game, the elevated verification distance currently
survives.

The recent session-ownership fix deliberately did **not** decide this because the
value is developer tuning rather than gameplay authority.

Choose one policy:

- **session-scoped:** a new gameplay session always starts with the ordinary
  check distance; or
- **process-scoped developer intent:** the proof pulse deliberately spans a
  relaunch until it completes/cancels.

This is primarily a developer-iteration/expectation decision. The gameplay
rollback authority itself is already session-owned by ADR 0027.

### 36. What are the authored standing heights of the puppy slug, stochastic parrot and burning flying shark?

These are the remaining characters whose old size derivation cannot be replaced
by preserving one existing placement size because their authored spawn boxes
disagree substantially across rooms.

✔ **PREMISE RE-MEASURED 2026-09-03 — still open, and here are the exact rows to
fill.** None of them carries a `standing_height` in
`game/ambition_content/assets/data/character_catalog.ron`:

| your name for it | catalog id(s) |
|---|---|
| puppy slug | `npc_puppy_slug` |
| stochastic parrot | `stochastic_parrot` |
| burning flying shark | **two** — `npc_burning_flying_shark` and `hall_npc_burning_flying_shark` |

⚠ The shark is the one to watch: it has a hall variant as a separate catalog
entry, so a single number either goes in twice or the two are deliberately
different sizes — which is itself part of the answer rather than a detail below
it.

Representative placement variation:

```text
npc_puppy_slug            (48,22), (32,48), (64,32), (48,32),
                          (64,16), (52,66), (42,42), (28,44)
stochastic_parrot         three different boxes
npc_burning_flying_shark  mostly (108,96), also (32,48)
```

The needed value is one character-authored `standing_height` in world units for
each. Do not choose by majority box size: the box was editor/layout data, not a
stature authority.

Decision 32 still applies: there is no standard adult/humanoid height and no
bulk normalization. Character stature is authored individually.

Related visual followups that should be judged by playtest rather than another
population average: the cove pirates relative to Robot v3, slop size, and the
Mary-O snake's post-rescale size.

### 35. What should own fighter reach during move startup?

The current fighter brain uses one global `REACH_TOLERANCE = 2.0`, effectively
allowing a move to remain viable out to roughly three times its authored reach.
The bug that exposed this proxy is fixed; the constant is not currently known to
cause a product defect.

The design choices are:

1. keep the proxy until platform-fighter option ranking has its own capability
   boundary;
2. add a per-move tolerance field;
3. derive reachable distance from move startup plus the body's movement
   capability, which requires threading capability/top-speed information into
   perception;
4. for moves with authored startup impulse/travel, derive that part directly
   from the move and keep a fallback for ordinary movement.

**Default if no change is desired:** leave it. Do not widen generic actor data for
a proxy that is not currently hurting play merely to close a planning row.

### 34. Should external/launch-owned motion become an explicit cross-game fact?

Three shared movement decisions have historically inferred "this velocity belongs
to a launch rather than locomotion" from speed magnitude: initial-dash settling,
shield braking and body-contact resistance. The approximation fails when a launch
has decayed below ordinary run speed.

Smash already has a live tumble mechanic and therefore a genre-specific fact that
can represent external/launch-owned motion. Ambition does not necessarily want
Smash tumble semantics for ordinary bodies.

The decision is whether to:

- keep the current thresholds until a visible defect requires more;
- introduce a generic carried/external-motion ownership fact with Smash tumble as
  one producer; or
- let the platform-fighter capability own the richer rule while the shared kernel
  keeps the simpler behavior for other games.

Do not solve this by simply reading Smash's `tumble_speed` from the generic
kernel; the question is exactly whether that game-specific semantic is shared.

### 33. How should a recharging ranged weapon communicate that it is unavailable?

The firing cadence is implemented: `BodyMelee::ranged_cooldown` follows the
weapon's authored refire interval, and an early press is refused before spending
the proposer so ordinary combat buffering can retry when the weapon becomes
ready. The unresolved part is presentation.

Choose the product channel when this becomes important in play:

- character/muzzle VFX driven by recharge fraction;
- a presentation treatment on the firing limb/body; or
- a HUD indicator.

The mechanic does not need to block architecture work while the unavailable
state is merely invisible rather than incorrect. Prefer character-local
presentation if it reads clearly; do not add another gameplay authority to show
the cooldown.

### 38. Does an actor released in a foreign room stay there?

Today an actor moved away from its authored home and then left in another room is
retired when that room unloads and is authored again at home when encountered
later. The current construction road honestly refuses to claim the actor was
persistently relocated.

Two valid product policies:

- **go home:** authored home placement is restored when the actor is no longer
  live/resident;
- **stay where left:** persist a `Placed`/relocation occurrence for actors as is
  already done for relevant item occurrences, and teach reconstruction to honor
  it.

If choosing “stay,” the producer and reconstruction consumer must land together;
recording a moved placement that construction refuses would only add warnings and
still teleport the actor home.

This decision feeds
[`engine/construction-and-reconstitution.md`](engine/construction-and-reconstitution.md)
and [`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md).

### 39. Which authored move, if any, should adopt the dormant windbox/armor vocabulary?

The windbox mechanic is implemented and can express outward gust or inward
suction. `WindowTag::Armor` also exists but has no shipped authored customer.
There is no engine defect merely because the vocabulary is currently unused.

If one should become product-visible, name a fighter/move. Otherwise leave the
mechanism dormant until a character design asks for it. Do not invent a customer
to make an adoption count nonzero.

✔ **RE-MEASURED 2026-09-03 — still exactly true, and here is what is waiting.**
The windbox vocabulary is real API across **16 files**: `WindboxVolume`
(`ambition_entity_catalog/src/lib.rs:415`), `MoveSpec::windbox`
(`ambition_combat/src/strike.rs:94`) and `is_windbox`
(`platformer2d_core/src/hit_response.rs:112`). **Zero** authored movesets under
`game/ambition_content/src/*moveset*.rs` name it. ⇒ So the question is unchanged
and the answer costs one authored field on one move — not an implementation.
⚠ Measured because "dormant" is the premise this decision rests on, and a
premise that has quietly acquired a customer would make the question moot without
anyone noticing.

### 40. Should a held gun-sword kick the player the way it kicks the pirate?

The K2 fold puts the player's held gun-sword and fireball on the ONE projectile
road, and that road applies the weapon's authored `Discharge`.
`gun_sword_discharge()` authors **380 px/s of recoil**, written for the pirate
who carries it.

The held-shot path that was deleted applied **no recoil at all** when the player
fired. That difference was a property of the second code path, not an authored
decision — nobody chose it. The fold preserved the old feel by zeroing recoil
for hand-fired held items, so today the same weapon kicks its NPC wielder and
not its player wielder.

Choose one:

- **the weapon kicks whoever fires it:** delete the zeroing; a player firing the
  gun-sword takes the authored 380 px/s. One weapon, one authored number.
- **recoil is a wielder property:** keep the zeroing, and say so in the weapon
  vocabulary rather than as a special case — an NPC braces, a player does not.
- **the player number is simply different:** author a separate player recoil
  value; name it.

⚠ This is a FEEL ruling on a shipped weapon, not an engineering question. The
engineering is done either way; the fold currently encodes "no kick for the
player" only because that is what the deleted path happened to do. The zeroing
is `fire_held_ranged_system` in `crates/ambition_held_items/src/lib.rs` (it was
`items/pickup/mod.rs` <!-- cite-ok: the pre-carve path, kept as the record -->
until the pickup carve moved the domain on 2026-09-03 — the old path still
EXISTS as the kernel's schedule residue, so that citation resolved while
pointing at code that had left); the guard is
`a_hand_fired_gun_sword_bolt_flies_the_one_projectile_road`
(`game/ambition_app/tests/hand_fired_held_shot.rs`) — retarget it with the
ruling.

✔ **ANSWERED by Jon, 2026-09-02, verbatim:** *"Weapons can have recoil and they
will kick whoever is firing it. The kick might depend on the mass property of
the actor doing the firing."*

⇒ **The first sentence settles this entry: delete the zeroing.** One weapon, one
authored number, and it kicks its wielder whoever that is. `fire_held_ranged_system`
stops special-casing hand-fired items and the guard retargets to expect recoil.

▢ **The second sentence opens a NEW question and must not be smuggled into this
one.** "Might" is a direction, not a ruling. Recording what it would have to
attach to, because the input already exists and is not vestigial:
`ActorDefinition.mass` is `Option<f32>`, read at spawn as
`definition.vitals.mass.unwrap_or(1.0)`, merged by
`ambition_body_seed/src/physical_baseline.rs` (it was
`character_runtime/physical_baseline.rs` <!-- cite-ok --> until D33 cut 2a moved
the file whole, `7ba40886e`), and rollback-registered under the
stable name `mount.mass`. So a mass-scaled recoil would read a live,
rollback-safe value rather than needing a new authored field. ⛔ What is NOT
decided: the curve (linear in mass? inverse? clamped?), whether an unauthored
mass of 1.0 means "average" or "unscaled", and whether this generalises to all
knockback or only to discharge recoil. Do not pick one by inference — a shipped
feel change to every weapon is a bigger ruling than the one asked here.

### 41. Where should a hand-fired fireball leave the body?

The deleted held-shot path spawned the fireball from a side muzzle at
`(size.x / 2 + 8, -0.12 * size.y)` — offset forward and slightly toward the
head. The projectile road's default is `Muzzle::BodyOrigin`, a few pixels away
from that point.

The difference is small and entirely cosmetic, but it is visible on a wide body
and it changes where the shot clears the player's own silhouette.

Choose one:

- **body origin:** accept the road's default and retire the old offset; one
  spawn rule for every projectile.
- **keep the authored muzzle:** register the old offset as an authored
  `Muzzle` on the fireball so the fold preserves the shipped look exactly.
- **a muzzle is a per-weapon authored fact:** every hand-fired weapon names its
  own, and the fireball's is the old offset.

The fireball's spec (`held_item_by_id("fireball")`, `action_set/mod.rs`) authors
`Muzzle::default()` today.

### 42. Should the gauntlet fireball keep its own sprite, or become the catalog's energy ball?

The deleted path drew the fireball as the 30 px `gauntlet_fireball.png` sprite.
The projectile catalog's `"fireball"` is a tinted energy ball — a different
look, shared with every other fireball in the game.

To avoid changing a shipped visual inside a refactor, the fold registers a
`"gauntlet_fireball"` Image visual that reproduces the old sprite.

Choose one:

- **keep the gauntlet's own sprite:** the registered Image visual stays, and the
  gauntlet reads as its own weapon.
- **adopt the catalog energy ball:** delete the registration; one fireball look
  across the game, and the gauntlet loses its distinct art.
- **the sprite is right but the catalog should own it:** promote
  `gauntlet_fireball` into the shared catalog as a first-class projectile visual
  other weapons may also use.

⭐ **THE SIZE/ANCHOR COMPARISON IS DONE, IN CLOSED FORM, 2026-09-02 — and it
found the difference somewhere else.** A capture was queued for this; reading the
two paths answers it more exactly than a screenshot could.

- **SIZE: exact parity, at every quality tier.** The deleted renderer drew
  `Vec2::splat(30.0)`. The new registration is
  `ProjectileRenderSize::FixedWidth(30.0)`, which resolves as
  `Vec2::new(w, w / frame_aspect)`. `gauntlet_fireball.png` ships at **64 x 64**
  (base), 32 x 32, 16 x 16 and 8 x 8 (`sprites_potato`) — **all square**, so the
  aspect is 1.0 and the height is 30.0 whichever tier the quality budget loads.
  Identical, not merely close.
  ⚠ An earlier revision of this note said the sprite was 16 x 16. That was the
  `sprites_0_25x` copy — one of four, read without checking the others. The
  conclusion was right and is now stronger, but the number was a sample quoted as
  the population.
- **ANCHOR: exact parity.** The old sprite took `..default()` (centre); the new
  Image path passes `anchor: None`, which is the same centre.
- ⛔ **DEPTH CHANGED, and nobody asked about depth.** The old fireball drew at
  `z = 9.5` — below `WORLD_Z_DUMMY` (10.0) and well below `WORLD_Z_PLAYER`
  (20.0), so it passed BEHIND the player and behind enemies. The projectile road
  draws every shot at `projectile_z()` = `WORLD_Z_PLAYER + 2.0` = **22.0**, in
  front of the player. `world_to_bevy` passes `z` straight through, so these are
  the same scale and directly comparable.

⇒ The fold moved the fireball from behind the cast to in front of it. That reads
like a repair rather than a regression — a thrown fireball vanishing behind a
body is hard to defend — but it IS a visible change, it was not authored as part
of the fold, and it is not what decision 42 asks about. Say whether the new
layering is what you want; if it is, nothing to do.

⚠ **What a capture would still add, and only this:** that the asset RESOLVES at
runtime rather than drawing the magenta placeholder. The geometry above needs no
picture.

The registration is `GAUNTLET_FIREBALL_VISUAL` in
`game/ambition_content/src/projectiles.rs`.

All three (40–42) were opened by the K2 fold on 2026-09-02; the fold itself is
landed and none of them blocks it.

### 43. Does a body hanging on a ledge inside a hazard volume die?

Spikes under a ledge lip is an authored shape, and a hanging body's box can
overlap one. Today it does not die: an active ledge grab consumes the simulation
frame before the hazard/OOB gate runs, so the gate never judges it.

That was not authored — it is a consequence of where the gate sits. It surfaced
2026-09-02 when the gate moved to fix an ordering bug and would have started
judging three populations it never had; the move was constrained back to its
original population rather than deciding this by accident. See
[`engine/collision-and-ccd.md`](engine/collision-and-ccd.md) §1.

- **stays immune:** hanging is a committed state, and a body that cannot act
  cannot be asked to escape; or
- **dies:** the hazard is a volume and being inside it is being inside it,
  whatever the body is doing.

⚠ Recorded, not recommended. Whichever way it goes it is a FEEL ruling and wants
authoring deliberately, not acquiring from a refactor.

✔ **ANSWERED by Jon, 2026-09-02, verbatim:** *"They get hit by the spikes (as
long as they are not immune - e.g. they might have iframes from a ledge grab).
Spikes may or may not insta-kill, they could just do damage."*

⇒ **"Dies" was the wrong framing of the second option and the ruling corrects
it.** The body is HIT; whether that kills it is the hazard's authored damage,
not a property of hanging. Both halves are already expressible and need no new
vocabulary: `HazardSpec` carries `damage: i32`, `knockback`, `kind`, `team`,
`hitstop_seconds` and `respawn`, so a spike that merely hurts is authored, not
built.

⛔ **AND THE EXEMPTION MOVES.** It is no longer "hanging is a committed state" —
a hanging body is judged like any other, and if it survives that is because it
is IMMUNE, through the ordinary invulnerability road. Jon names a ledge grab
granting iframes as an example, not as a fact about today's code.

▢ **So what remains is one question this entry did not ask:** does a ledge grab
grant iframes, and if so for how long? ⭐ It has a home already — invulnerability
is a REASON SET, not a flag (`features/empowerment.rs` delegates
`Empowerment::UNTOUCHABLE` to the body's invulnerability-reason set, beside
`Invulnerability::EMPOWERED`), so "hanging on a ledge" would be another reason
rather than a new mechanism. That is a separate authoring decision
from the gate ordering, and it is the one that decides whether the visible
behaviour actually changes. Until it is answered, moving the gate makes hanging
bodies take spike damage — which is now the intended behaviour, so the
constraint that held the gate back is lifted.

### Two fighters' bespoke effect art is never requested (2026-09-02)

`npc_pirate_admiral` has a 14-row effect sheet and `smash_george_booul` a 21-row
one, and **nothing in the repository names 34 of those 35 rows** — measured by
`scripts/measure_fx_row_reachability.py`, which asks which fx row names any
tracked `.rs`/`.ron`/`.yarn`/`.json` mentions (an effect is drawn by name, so a
row nothing names cannot be requested).

⛔ IT IS NOT DEAD ART, which is why this is a question and not a slice. The rows
were drawn FOR those kits: `grapeshot_cloud`, `heave_to_anchor`,
`heave_to_brake`, `cutlass_wake`, `boarding_wake`, `captains_mark` sit beside an
admiral moveset whose moves are named `grapeshot`, `heave_to`, `gun_sword` — and
that moveset asks for `muzzle_flash` and `air_slice` from the GENERIC sheets
instead. George is the same: `bivalence_weak`/`bivalence_strong`,
`excluded_middle_windup/launch/ascent/gate/tail`, `modus_ponens_*`, `reductio_*`,
beside moves called `bivalence`, `excluded_middle`, `commitment`.

✔ **RE-MEASURED 2026-09-02 evening and the claim reproduces exactly**:
`pirate_admiral_vfx` 14 rows / 0 named, `george_booul_vfx` 21 / 1 named — 34 of
35, unchanged. ⓘ Wider context the row did not have: across all 13 fx sheets it
is **196 rows, 120 named, 76 named by nothing**, and `pirate_admiral_vfx` is the
only sheet with NO row named by anything. So the two named here are the extreme
of a spread, not isolated cases — which strengthens "wire them" and weakens
"they were superseded", because eleven other sheets are partly wired.

⇒ **Wire them, or were they superseded?** The row→move pairing is unambiguous
from the names, but WHEN in a timeline each fires and at what scale is a feel
ruling, which is why it is here rather than done. Residency's interest was the
size (9.4 MP resident in every room); since 2026-09-02 the two sheets are never
decoded, because no realized character's moveset names a row of them.

### The LDtk editor-preview tileset is 7.6 MP the runtime never draws

**FIVE** world files declare `sprite_player_robot_v3 = ../sprites/player_robot_v3_
spritesheet.png` so the editor can draw entity previews, and `bevy_ecs_ldtk`
decodes every tileset of a project when the project loads — on every boot and
every world load, at FULL tier, beside whatever tier the game actually realizes.

⭐ **MEASURED 2026-09-02 (`scripts/measure_ldtk_tileset_usage.py`), and the two
things that were assumed are now checked.** NO LEVEL LAYER in any of the five
uses the tileset (`layerInstances[].__tilesetDefUid`, across 1/11/60/1/1
levels); its only consumer is one entity definition per world, `PlayerStart`,
cropping the top-left tile. So "editor previews only" is measured — which
matters, because a layer using it would have made a cheaper tier a QUALITY
decision instead of a free one. And it is five worlds, not four: the four
`ambition_content` ones plus `ambition_demo_sanic/worlds/sanic_speedway.ldtk`.
`mary_o` does not reference the sheet.

⛔⛔ **AND IT IS NOT "one line per world".** `tileRect`, `uiTileRect`,
`tileGridSize`, `pxWid` and `pxHei` are all TILESET PIXEL coordinates. Change
only the `relPath` and the 256×256 crop that framed one animation frame spans a
third of an 832-pixel image — the preview breaks while the JSON still looks
plausible, and nothing in the game reports it. The tiers are also not exact
fractions and the x and y factors differ (`sprites_0_25x` of 3072×2468 is
832×653, not 768×617), so there is no constant to scale by.

⇒ **A prepared patch is waiting: `dev/patches/ldtk-player-tileset-retarget-20260902.patch`**
(`patch -p1 <` or `git apply`, both verified; regenerate with
`scripts/propose_ldtk_tileset_retarget.py --tier <tier>`). It recomputes every
pixel field from the real PNG header and preserves each crop as a FRACTION of
the image. Boot decode for this tileset goes 7.6 MP → ~0.54 MP.

ⓘ It also fixes two stale declarations found by reading the real header: the
four content worlds declare `pxHei 2484` against a 2468-pixel file, and
`sanic_speedway` declares `1681×1728, tileGridSize 224` for that same file. ⚠ The
patch preserves sanic's framing as a fraction, i.e. whatever that stale
declaration was already showing.

⚠ **IF THE SUITE TELLS YOU THIS PATCH NO LONGER APPLIES, CHECK THE INDEX BEFORE
BELIEVING IT.** The targets live in the `game/ambition_map_assets` SUBMODULE, and
on 2026-09-03 `test_a_patch_against_this_tree_still_applies` failed on it —
"either the tree moved under it or the patch was never test-applied" — while
`git apply --cached --check` inside that submodule accepted it cleanly. Four
`.ldtk` worlds were simply dirty from another session's work in progress. Acting
on the failure as written would have withdrawn a correct patch, or regenerated
it on top of somebody's unfinished edits. The checker now separates the two
causes and SKIPS with the dirty files named, so this should not recur; the note
stays because the wrong reading was one sentence away from a bad decision.

⛔ **Jon's submodule, so it waits** — applying it needs a commit in
`game/ambition_map_assets` plus a pointer bump. ▢ And it is untested against the
LDtk editor itself, which needs Jon opening a world.

### How tall is a puppy slug? The base is unauthored, its variants are not (2026-09-02)

⭐ Found while re-measuring the pirate-sizing report's *"zero rows author
`standing_height`"* claim, which is now **23 of 149**
(`scripts/measure_character_data_coverage.py`). Two of the twenty-three are
puppy slugs, and the third member of the family is not:

```text
npc_puppy_slug            (falls to the 48.0 default — nobody chose it)
npc_puppy_slug_variant2   41.7
npc_puppy_slug_velvet     30.9
```

⛔ **So the BASE crawler stands taller than both of its own variants, at exactly
the player robot's height** — which is the symptom Jon reported for the pirates
(*"as tall as the player robot who is supposed to be chibi"*). `npc_puppy_slug`
and `npc_puppy_slug_velvet` publish the SAME 128 px frame, and land at 48.0 and
30.9: a 55% difference between two characters drawn at one size, where one side
is a decision and the other is a default.

⭐ **AND IT IS THE ONLY FAMILY IN THE CATALOG LIKE THIS.** Swept every
base/variant pair (one row name a strict `_`-prefix of another) across all 149
rows: `npc_puppy_slug` is the *sole* case where one side authors a height and
the other does not. Every other family is all-authored or all-default. ⇒ That
uniqueness is what makes it read as an omission rather than a convention — if
leaving the base to `body_kind` were the house style, it would be visible
somewhere else.

⚠ **It is still not obviously a bug** — a base breed may be meant to be larger —
which is why this is a question and not a fix. What is odd is the SHAPE: the two
variants were deliberately sized and the thing they are variants OF was left to
`body_kind`. ⇒ **What height should `npc_puppy_slug` be?** It is placed in
`hall_of_characters`, `intro` and five levels of `sandbox`, so whatever it is
now is visible in play.

### The shared sprite pack is 442.6 MB and one prop reads it (raised 2026-09-02)

⭐ **MEASURED** (`scripts/measure_pack_reachability.py`).
`build_prop_sprite_asset_packed` is the ultrapack's ONLY production consumer; it
has ONE call site, the intro prop loop; and it runs only for
`intro_prop_sprite_rows()` entries whose 4th tuple element is `Some(target)`.
**Exactly one row is: `intro_cart`.** Characters have no pack road at all —
`load_character_sprites_in` takes the per-target `*_spritesheet.ron` every time.
All four tiers pack the same 197 targets. ⚠ **Re-run on calculex 2026-09-03 the
script reports 164 targets, and the pack directory measures 318 MB rather than
442.6.** The load-bearing claim is unchanged and verified — *"1 target(s) opt
into the pack — intro_cart"* — but BOTH size figures are generated-artifact
numbers, and `measure_orphan_shipped_pages.py` says of its own kind: *"these are
gitignored generated files, and this is ONE machine's tree."* ⇒ So treat 197/442.6
and 164/318 as two machines' trees rather than a change over time; the argument
this entry makes does not turn on which. On one machine: **442.6 MB of pack
pages, 5.2 MB on a page any consumer can reach — 98.8% unreachable.**

⚠ **NOT A DEFECT REPORT.** Packing every target is what a packer should do; the
finding is that adoption never followed. Reachability is a SOURCE fact and reads
the same on any checkout; the megabytes are generated, gitignored, per-machine.

⇒ **Three answers are all reasonable and none is an agent's call:** adopt the
pack for characters (it was built for that, and `project_ultrapack` design intent
says the two roads should converge); narrow the generator to pack only what a
consumer opts into; or leave it, on the grounds that a packer that packs
everything is correct and the cost is disk nobody is paying attention to. ⛔ What
is NOT reasonable is dropping the per-target PNGs to "save" the duplication —
they are every character's only road.

### Portraits are generated at four tiers and only full resolution is readable (raised 2026-09-02)

`bake_portrait_manifests` collects portrait manifests from `assets/sprites` ONLY
and says why: *"Portraits are presentation products and currently have no
quality-tier variants"*. The generator emits the PNGs at all four tiers anyway —
**438 files, 13.4 MB, with no road**
(`scripts/measure_orphan_shipped_pages.py`, re-run 2026-09-03; its
`REDUCED-TIER PORTRAITS` section). ⚠ This entry read **487 files, 14.2 MB** when
raised on 2026-09-02 — the figure drifted by 49 files in a day, which is what a
generated population does. ⇒ The decision is unaffected; the drift is only worth
noting because the entry quotes a size to argue the cost is worth acting on, and
that size is a moving number with a one-command instrument beside it.

ⓘ The missing `.ron`s are POLICY, not a bug —
`check_quality_variants_are_fresh.py` records that portraits are *"published
SELECTIVELY"*. ⛔ But the 9 that ARE published per reduced tier cannot be read
either: `PortraitSheetRegistry` is built `from_baked_table(BAKED_PORTRAIT_RONS)`
and `build.rs` bakes from `assets/sprites`. A deliberate selective publication
produces files no build can load.

⭐⭐ **AND THE MEASUREMENT NARROWS THE ANSWER — 2026-09-02,
`scripts/measure_portrait_tier_headroom.py`.** Portrait draw size is chosen by
VIEWPORT, never by quality tier: `DialogLayoutProfile::for_viewport` picks
**56×62** (phone landscape), **82×94** (phone portrait / small tablet) or
**104×120** (everything else), consulting no quality setting. So no quality tier
can select a portrait resolution — the window size does. Against `alice`:

```text
tier              frame     @1x display          @2x display
sprites          256x320    covers every box     covers every box
sprites_0_5x     128x160    covers every box     smallest box only
sprites_0_25x      64x80    smallest box only    UPSCALES ALWAYS
sprites_potato     16x20    UPSCALES ALWAYS      UPSCALES ALWAYS
```

⇒ **Nothing wants a Potato portrait**: at 16×20 it is under even the 56×62 box
at 1×. `sprites_0_25x` is defensible only on a phone-landscape viewport at 1×
display scale — the least likely combination, since phones are high-DPI. Only
`sprites_0_5x` has a real case, and only at 1×. ⚠ A tier under the box it is
drawn into is not a cheaper portrait; it is a blurrier one, which is the
failure Jon's standing rule forbids.

⭐ **THE ANSWER IS ALREADY WRITTEN — `dev/patches/portrait-tiers-are-never-baked-20260902.patch`**
(`git apply` it from the repo root). It stops
`scripts/generate_visual_quality_variants.py` copying `*_portraits.png` into the
three reduced tiers, which is 487 files / 14.2 MB that nothing can load, since
`bake_portrait_manifests` collects from `assets/sprites` only. It is a patch and
not a commit because it changes generated assets on the next unfiltered regen —
that is Jon's call, not mine. ⓘ It touches a MAIN-REPO script, so applying it
needs no submodule commit and no pointer bump.

⚠ Filed as a pointer 2026-09-03 because the patch existed for a day with NOTHING
in the repository naming it — the decision row asked the question and the answer
sat in a directory nobody had reason to open. The other three patches in
`dev/patches/` are each named by a doc; this one was not.

ⓘ Residency is already bounded independently: `RetainedHudImages` holds one
entry per portrait ACTUALLY SHOWN (~1.3–2.0 MP each), not the 163 baked
manifests — so the tiers would save package size, not runtime memory.

⭐ **AND THEY ARE STILL BEING PRODUCED, not left over.** Age signal
(`measure_orphan_shipped_pages.py`): 439 of 475 comparable portrait files were
written in the same run as their full-resolution twin or after it, median +3.07
days — against the stranded sheet pages, where 44 of 44 predate their manifest.
⇒ **A clean regen on another machine will reproduce these**, so the "wait for
yardrat" answer that covers the stranded pages does not cover this row.

⇒ **Stop generating them, start baking them, or leave them?** The measurement
says at most one tier (`0_5x`) could ever be wanted and two certainly cannot.
The comment says portraits have no tier variants *currently*, which reads as
intent that may change — and that is the part an agent cannot know.

### 44. Should `SmashChargeSpec` keep a game-mode name for a general mechanism?

Jon raised the general shape of this on 2026-08-28, about a different type
(*"it might be a good idea to rename the actor given its conflation with a very
core concept in the architecture. But we can do that in a different pass"*). The
`Actor` half was done — it is `Performer` now. This half was never put to him and
should not be decided by an agent, because a rename is Jon's vocabulary call.

⭐ THE CONFLATION IS MEASURED, not assumed. `SmashChargeSpec` is named for one
game mode and its own doc comment describes something general: *"How a chargeable
move HOLDS: where on its own timeline the charge waits, and how long it may wait
before it fires itself."* It carries `roots`, `sustain`
(`WhileHeld` / `UntilPressedAgain`) and two seconds-valued clocks in the owner's
proper time — none of which is Smash-specific — and the Trap (the Performer's
down-B, an Ambition move, not a Smash one) uses it for a three-second
subterranean beat, which is what made the name visible.

⚠ THE SIZE, so the answer can be costed: **36 references** across `crates/` and
`game/`. A rename is mechanical and touches authored content, so it wants to
happen in one pass or not at all.

⇒ Three answers are all reasonable and the choice is not an agent's: keep the
name (the mechanism was authored for Smash and the association is useful), rename
to something like `TimelineHoldSpec` / `ChargeHoldSpec`, or keep it and let a
future Smash-specific type take the name back. ⛔ No engineering is blocked
either way — this is recorded so it stops being asked and forgotten.

### Interact dialogue for the characters the Hall's authoring did not cover (raised 2026-09-02)

`triage/character-dialogue-from-suggestions.md` re-measured: 149 catalog rows,
124 with a `hall_dialogue_id`, 131 authored `hall_*` Yarn nodes. The Hall was
solved by hand-authoring, the escape the 2026-07-26 decision left open, so a
generator built to that decision would generate over 124 characters that
already have nodes. What remains is ~25 rows with no hall id, and every room
that is not the Hall, where a character with a real `fallback_dialogue` voice
still opens `generic_npc` on interact.

Needed: keep authoring by hand (then the triage closes as superseded), or
build the generator for the remainder only (a per-character node synthesized
from `fallback_dialogue`, overridden by any authored node of the same title).
Content call; the engine side is unchanged either way.

### 45. Is a unique capability item an ENTITLEMENT or an OCCURRENCE? (2026-09-02)

`item-custody-and-accounting.md` I3 asks special pickup roads to "converge
toward the same occurrence/custody model as ordinary held items **when that
model can express their semantics**". Measured, it cannot express the portal
gun's, and the difference is a product decision rather than an engineering one.

An ordinary held item is an OCCURRENCE: a `GroundItem` with a `SimId` that
persists through pickup and drop, its location remembered by custody and the
whereabouts ledger. The portal gun is an ENTITLEMENT: picking it up despawns the
world token, grants `PortalGun` on the body and `Item::PortalGun` in
`OwnedItems`, and **dropping never revokes the grant** — it unequips and spawns
a fresh, room-scoped token. The menu re-equips straight from `OwnedItems`. The
code states the intent where it is decided: *"The gun is a single item: it
doesn't exist until you pick it up — picking up the one world item IS getting
the portal gun."*

✔ Nothing is broken: `OwnedItems::grant` clamps a unique item to 1, so the two
roads cannot inflate a count, and the measured behaviour is self-consistent.

⇒ **The question is what you want a unique capability item to MEAN**, and the
two readings differ observably in exactly one place: **can dropping the portal
gun and walking away ever lose it?**

- **Entitlement** (what ships today): no. Once acquired it is yours; the world
  token is a convenience for re-equipping in place. Zelda's hookshot.
- **Occurrence**: yes. The gun is a thing that exists somewhere, can be left in
  a room, stolen, or lost down a pit, and the durable record is where it IS.

⛔ Not an agent's call — it decides whether a whole category of future item is
losable. Recorded rather than implemented; I3 stays open behind it.

### ▢ Should cast framing become BIDIRECTIONAL — a target rather than a floor?

⭐ Split out of the 2026-09-03 camera-zoom change, which landed the part that was
measurable and left this part alone because it is a FEEL ruling.

**What landed:** `CameraZoomPreset::Duel` (568x320) is the new default, sized from a
measured reference — `pointed_polygon` is `body_kind: Standard`, standing height 48.0
world units, so 48/320 = **15.0%** of screen height. The previous `Combat` default
(800x450) gave **10.7%**.

**What is open, and it is the half that would actually read as Smash-like.** Our camera
can only ever widen from the base:
* `camera_scale` is clamped `.max(1.0)` (`camera_snapshot.rs:320-350`);
* cast framing is documented as *"the view is a FLOOR, so authored zoom still wins
  whenever it is already wider"* (`CAST_FRAMING_MARGIN`);
* encounter `camera_zoom` is a zoom-OUT multiplier.

⇒ `base_view` is **the most zoomed-in the camera ever gets.** Ultimate's ~15% is the
MIDDLE of a dynamic range — roughly 19% when fighters close, 11% when they separate. Ours
is now tighter than Ultimate when fighters are apart, and never as tight when they meet.

**The decision:** leave framing floor-only and accept a fixed 15%, or make cast framing a
target that tightens toward ~19% and relaxes toward ~11%?

⛔ Not taken unilaterally because the floor-only property is load-bearing: authored zoom
winning whenever it is already wider is what lets a room or an encounter guarantee a
minimum view, and a bidirectional camera can override that guarantee. It is a genre/feel
ruling with an architectural cost, not a tuning knob.

ⓘ Cheaper alternative if the answer is "not now": `Tight` (640x360, **13.3%**) is one line
away and is the conservative version of the same change.

⚠ One measurement caveat on the 15%: 48.0 is the `Standard` body-kind DEFAULT, so every
standard humanoid is 48 units — `pointed_polygon` is representative by construction rather
than by having been measured individually. The drawn sprite including its margin is taller
(~89 units, the 118px body in a 218px frame), so "character size" means the BODY here, not
the art. If the intent was the art, every figure above shifts and the tier wants resizing.

### 51. Does a boss's reward survive a death that un-fights the boss? (2026-09-04)

A defeated boss drops its signature gauntlet. Since 2026-09-04 that object has a
durable identity (`SimId::death_drop`), so a checkpoint taken while the player
HOLDS it now describes it and a death gives it back — measured end to end by
`a_boss_gauntlet_banked_at_a_checkpoint_returns_to_the_hand_that_banked_it`.

⛔ **The question is the object LYING ON THE FLOOR, and the engine currently
answers it by omission rather than by decision.** `capture_minted_item_baseline`
records only occurrences IN CUSTODY, and says why: *"a minted object lying in a
loaded room is answered by the object itself."* But a death is a room replay,
and `retire_the_previous_attempt` despawns everything the attempt spawned. So a
gauntlet the player took and then put down before resting is destroyed by the
replay with no description anywhere that could rebuild it.

⚠ **Whether that is a loss depends on a fact this file cannot decide:** if the
death restores the boss to un-fought, the player simply kills it again and gets
another gauntlet, and destroying the old one is CORRECT — two would be a
duplication. If the checkpoint recorded the boss as Cleared, the boss does not
come back, and the reward is gone permanently with no way to earn it again.

The two readings differ observably in exactly one place, which is what makes
this a decision rather than a bug report: **beat a boss, pick up its gauntlet,
put it down, rest at a shrine, die.** Under one reading you find it where you
left it; under the other it is gone forever.

⇒ Jon's call. It is a rule about what a checkpoint PROMISES — "the world as it
was when you rested" versus "your body as it was when you rested" — and the same
answer settles the dropped coin, the dropped heart and any future attempt-scoped
reward. ⚠ Do not answer it by widening `capture_minted_item_baseline` to
in-world occurrences: that changes what a checkpoint means for every dynamic
object in a loaded room, which is a larger decision wearing this one's clothes.

⭐ **NARROWED 2026-09-04 by measuring the durable mechanism, which turns out to
exist and to have exactly one door.** `AuthoredOccurrences` already reconstitutes
an item put down in a room that is not loaded — `Placed { room, at }` freezes at
the room boundary, `outlook_for` reinstates it where it lies and suppresses it
where it was authored, and `durable_horizon.rs` saves it. But an occurrence gets
its FIRST row only from custody (`project_custody_onto_authored_occurrences`,
reading `InCustodyOf`), so anything that enters the world already on the ground
and is never picked up cannot be remembered by construction either. That rule is
now enforced by the ledger rather than by a producer's comment, and it refuses
by name (`republish_placements` returns its refusals `#[must_use]`).

⇒ **So a "yes" here is not a new mechanism, it is a second entry road, and it
must be stated as deliberately as the first.** The gauntlet would need to enter
the ledger on the drop rather than on the pickup — and it would then also need
to stop carrying `SpawnedThisAttempt`, because an object the attempt reclaims
and an object the durable world remembers are contradictory answers about the
same thing. That contradiction is the real content of the question.

Related: 45 (entitlement versus occurrence) is the same family of question one
level up — that one asks what an item IS, this one asks what a checkpoint owes.
⚠ Renumbered from 46 to 51 on 2026-09-04: it was filed as 46 while 46 already
existed further up the file, so two different questions answered to one number.

### 54. In co-op, does a body gate open for the party or for the body? (2026-09-04)

`capability-progression-and-world-gating.md` has carried *"how should co-op gates
behave when one participant can traverse and another cannot"* as an open design
question since the page was written. ⭐ **It stopped being hypothetical on
2026-09-04: the code now answers it, by default, in one direction.**

`driven_bodies` (`actor_monolith/src/body_conditions.rs`) asks
`ControlledSubject` first and then falls back to ANY holder of
`DrivingParticipant`. With one participant that names the one driven body and
there is nothing to decide. **With two seats it is an existential: a wall gated
on `body.can wall_climb` opens when EITHER driver can climb, and the seat that
cannot then walks through a wall its own body never satisfied.**

⚠ **This is not new and it is not a regression.** The same OR was there while the
predicate read `PlayerEntity`, and the defect it hid was worse — the wall asked
the RESTING home avatar, which possession has explicitly stopped driving. Moving
to the driven population fixed that and made this visible; it did not create it.
⇒ Which is the shape worth naming: **widening a population makes a latent ruling
live.**

**The three answers, and what each costs:**

1. **PARTY (today's behaviour).** One body qualifies, the wall is down for
   everyone. Cheap, and it is what a shared-screen co-op usually wants — nobody
   is stranded behind a wall their partner walked through. ⛔ But it makes
   `body.can` mean "somebody here can", so a route designed around a capability
   is satisfied by a party that contains one of anything.
2. **BODY.** The wall stands for the seat that cannot. ⛔ **This one is not a
   predicate change.** A gate solid is contributed to
   `FeatureEcsWorldOverlay::gate_solids`, which is ONE collision world read by
   body collision, projectiles and rendering alike. A per-participant answer
   needs the wall to stop being a property of the world — per-seat overlays, or
   a solid that filters by who is touching it. That is a mechanism change with a
   rendering question attached (does the wall LOOK present to one player and not
   the other?).
3. **PRIMARY ONLY.** Ask `ControlledSubject` and stop. Sharpest to state, and it
   silently makes seat 1 a passenger in a world gated on seat 0's body.

⇒ Jon's call, and it is a design ruling rather than an engineering one: it says
what a capability route MEANS when there is more than one body. ⚠ Do not answer
it by making `driven_bodies` stricter on its own — a wall that stands for one
player and not another is not expressible today, so answer 2 without the
mechanism would produce a gate that refuses everyone.

ⓘ The same question reaches every body condition at once (`body.can`,
`body.fits`, and anything later that reads the driven population), which is why
it is filed against the population rather than against a condition.

### 55. The route-gate vocabulary went 1 family to 5. Should the WORLD grow to use it? (2026-09-04)

`capability-progression-and-world-gating.md` records that five of seven gate
families became reachable from an authored route on 2026-09-04 —
`world.flag_set`, `inventory.holds`, `held.is_held`, `body.can`, `body.fits`,
`world.switch_on` — each with an end-to-end wall test walking the authored road.
It also records, honestly, that **no shipped level authors any of the new ones**,
and names the risk: the dormant-cluster shape, a vocabulary correct in every test
and reached by nothing.

⭐⭐ **THE DENOMINATOR IS THE THING NOBODY HAD COUNTED, and it changes the
question.** Measured 2026-09-04 by `scripts/authored_route_gates.py` (committed;
re-run it rather than trusting this):

```text
worlds scanned: 6
LockWall instances: 3  (2 gated, 1 encounter)
conditions actually authored:  2  world.flag_set
```

⇒ **The whole authored corpus of route gates is THREE WALLS.** Two carry a
`gated_by`, both in `intro.ldtk`, both naming the same story flag
(`bob_field_survey_received`); the third is the goblin encounter lock, which
belongs to a different writer and correctly has no `gated_by`.

⛔⛔ **CORRECTED WITHIN THE HOUR, BY THIS QUESTION'S OWN RULE: I COUNTED ONE
CONSUMER.** `ConditionCatalog` has a SECOND authored road, and it is the busier
one — `ambition_conversation/src/dialog/authored_conditions.rs` installs a Yarn
verb `condition(id, arg)`, so every `.yarn` line calling it is an authored use of
the same vocabulary. Re-measured with the instrument extended to both roads:

```text
LockWall instances: 3  (2 gated, 1 encounter)
dialogue files: 7  (using condition(): 2)
condition() calls: 10     7 inventory.holds     3 world.flag_set

TOTAL authored uses: 12  (2 route gates + 10 dialogue lines)
published but authored NOWHERE (5 of 7):
  world.switch_on  held.is_held  body.can  body.fits  encounter.cleared
```

⇒ **The correction makes the question SHARPER, not weaker, and moves where the
answer probably lives.** Twelve authored uses rather than two — still small — but
**five times more of them are DIALOGUE than routes**, and dialogue already uses
the vocabulary actively while routes barely do. ⭐ So framing this as a level-design
question was itself the narrow reading: *"you look like you could climb that"* in
a `.yarn` line is a `body.can` customer that costs no level geometry at all, and
nobody has considered it because the owner page is written around routes.

⚠ **And the arithmetic that survives: five of seven published conditions are
authored NOWHERE, by either road.** That is the dormant-cluster number, and it is
now measured across both consumers rather than one.

⛔ **So "five families reachable and none reached" is NOT a migration backlog.**
There is nothing to migrate. Converting both existing walls would empty the
story-gate family — which is the one family the page says should stay available
*"when sequencing is actually the design"* — and would still leave four families
unused. **The vocabulary is not unused because authors chose flags; it is unused
because the world has almost no gates at all.**

⇒ **Jon's call, and it is a CONTENT question rather than an engine one:**

1. **Author them in DIALOGUE first.** The cheapest road by a wide margin: ten of
   the twelve existing uses are already `.yarn` lines, the verb is installed, and
   a line conditioned on `body.can` or `inventory.holds` costs no level geometry.
   ⇒ Promoted to first because the corrected measurement says this is where the
   vocabulary actually lives.
2. **Grow the world.** New rooms author routes gated on body size, a carried
   tool, a cleared arena. The engine is ready and each family has a working
   example test to copy. ⇒ This is the answer the Goal implies (*"exploration
   emerges from what the body can do"*) — it costs level design, and it is the
   only one that makes the EXPLORATION claim true rather than the vocabulary
   used.
3. **Leave it.** Twelve authored uses is what the current game needs; the
   vocabulary waits for the content that wants it. ⚠ Then the dormant-cluster
   risk is real and should be accepted explicitly rather than by default — this
   repository has retired clusters for less (`GatePortalRegistry`,
   `GravityFlipSwitch`).
4. **Convert the two walls.** ⛔ Not recommended and recorded so it is not tried:
   both name a story flag because the sequencing IS the design there, so
   converting them would be authoring a worse gate to exercise a better
   mechanism.

⚠ **Do not answer this by adding walls to a demo world to make the count go up.**
A gate authored to exercise the vocabulary is the dormant cluster wearing a
level's clothes — the count would rise and nothing about the game would.

ⓘ The engine side needs nothing either way: the vocabulary is guarded by
`every_authored_gate_condition_prepares_against_the_composed_catalog`, which
walks every `gated_by` in every shipped world against the catalog the game
composes, so a misspelt condition fails a build rather than a playthrough. That
guard gets stronger automatically as the corpus grows.

### 53. Are the five declared-but-unnamed optional dependencies SEAMS or DEBT? (2026-09-04)

Five crates declare an optional `ambition_*` dependency that no file in their
crate directory names — re-derived three times across ~90 commits and five
carves, most recently 2026-09-03, zero source files each. The scan's limits are
known and checked: a `cfg(feature)`-gated `use` IS visible to it (control:
`ambition_content_pack` is optional in four crates with `cfg` blocks and was
correctly not flagged), only `ambition_app` has a `build.rs` and it does not
mention `causal`, and no crate renames a dependency with `package = "…"`.

⛔ **The engineering half is finished and it does not decide anything.** Removing
these cuts NOTHING from the capability footprint — the plain sixth edge was cut
2026-09-03 and the closure was unchanged at 47/20. The value is only that the
graph stops claiming edges nobody uses.

⇒ **What is left is intent, which is why it is here.** An optional dep is wired
into a feature definition, so removing one edits a declared seam:

- `ambition_characters` → `ambition_causal` is a NO-OP feature — it pulls the dep
  and does nothing else, and its comment says *"Publish this capability's causal
  facts (brain decisions, for now)"*. **A seam declared ahead of its use is not
  debt, and deleting it deletes the intent.**
- `ambition_platformer2d` → `ambition_sfx_bank` is one line of
  `all_capabilities`, a ROSTER of ~20 crates the facade can offer, and it is the
  crate's `default`. Removing it narrows what `default` MEANS.
- `ambition_touch_input` → `ambition_cutscene` sits among nine `dep:` lines that
  are all used; only this one is unnamed, so it reads as a wire planned and never
  run.
- `ambition_sim_view` → `ambition_portal2d` and `game/ambition_app` →
  `ambition_causal` are the two where the feature does more than pull the dep, so
  dropping the `dep:` alone leaves the feature meaningful — the smallest safe
  edits of the five.

⚠ **Whichever way this goes, do not remove blind.** Dropping an optional dep
changes feature RESOLUTION, not just a line, and only a build says what that
does; a feature that becomes empty may still be a marker something above
forwards. The feature-union build is where that surfaces.

⇒ One sentence settles it: **is a declared-ahead-of-use seam something this
project keeps, or something it deletes until the day it is wired?** Answer that
and all five follow, in either direction.

### 52. Does a second bark set for one enemy REPLACE the first, or conflict? (2026-09-04)

`CombatBanterRegistry` is the last of the seven "silent overwrite" registries
whose policy nobody has stated. Four of the seven state the replace in place, one
is a test hatch, and `GatePortalRegistry` is moot — it has no production producer
at all (measured 2026-09-04: the only `register` call is in a render test, no
LDtk world contains a `GatePortal`, no lowering names one).

This one is live: `game/ambition_content/src/dialogue/mod.rs:44` registers bark
sets in production. So the question is real and it is one sentence.

⇒ It is a CONTENT question, not an engineering one, which is why it is here
rather than in the queue. **Replace** means the later registration wins and an
author can override a shipped enemy's barks from their own content pack;
**conflict** means two packs that both speak for one enemy is an error somebody
must resolve rather than a silent last-writer-wins. The engine can express
either — `ambition_registry_core::classify` exists precisely to make the choice
explicit — so nothing is blocked on it. What is blocked is calling the registry
migrated, because a registry whose conflict policy is unstated is the drift the
crate was built to stop.

⚠ Whoever answers should note that this is the same shape as the four already
settled, and those four all chose replace-and-say-so. A different answer here
needs a reason specific to barks.

## A top platform and the respawn point want the same 60 pixels

⛔ **Not a tuning mistake — a structural collision, and all three ways out are
yours.** The fighter's reachable band and the respawn height overlap:

| | rise above the stage |
|---|---:|
| single jump apex | **88.2px** |
| air jump taken at the apex (the ceiling) | **148.3px** |
| ⇒ a top platform must sit between them | 88.2 – 148.3 |
| the respawn PLATFORM sits at | **130px** |

`RESPAWN_HEIGHT_PX` is 160 and the platform hangs 30px under the returning body,
so it lands at rise 130 — **inside the only band a reachable top platform can
occupy.** `smash_platform_stage`'s top tier is at rise 120, which is why it ends
up ten pixels under the respawn platforms; moving it anywhere else in the band
buys at most ~35px of clearance, and a fighter is ~40px tall.

⇒ **The three exits, none of which I should pick for you:**

- **(a) Accept a tight stage.** Put the tier at rise ~95 for ~35px of clearance
  and let a respawning fighter drop onto it. Genre-normal in spirit — Battlefield
  has a platform under the respawn point — but tighter than Battlefield's.
- **(b) Raise the respawn point.** `RESPAWN_HEIGHT_PX` 160 → ~220 opens the band.
  ⚠ It is a SHARED constant: this changes how returning to the flat stage feels,
  which is the stage every recorded measurement was taken on and the one you have
  been playing.
- **(c) Change the jump arc.** A higher `JUMP_SPEED` or lower `GRAVITY` widens the
  band. ⛔ That is the fighter's feel, and it moves every spacing, recovery and
  edgeguard result at once.

⚠ **Why it is being asked rather than fixed:** the flat-vs-platforms measurement
in [`engine/fighter-brain.md`](engine/fighter-brain.md) was taken on the rise-120
geometry, and moving the tier by guesswork would leave a recorded number
describing a stage that no longer exists while not actually resolving the
collision. Answer this and the geometry and the re-run move together.

## ⭐ THE SMASH FIGHTER: FOUR DECISIONS, ALL MEASURED (index, 2026-09-04)

Four fighter questions below, each carrying numbers and options rather than a
shrug. ⇒ **They are independent — answering any one is useful — but two of them
interact and it is worth knowing which:**

| # | question | the measurement behind it | where |
|---|---|---|---|
| 1 | **Which of two ladder authorities should exist?** | `profile_for_level` forks between the shipped `.ron` and an engine floor; **four separate defects were symptoms of that one fork**. Removing the loser rewrites **no authored content** and there is exactly **one** production installer. | below |
| 2 | **Do the two Robots get kits?** | At the same rung, on the shipped ladder and clock, **George significantly outfights a stand-in (318% : 199%)** with a correctly-null control. Two of three roster characters had no special button. | ↓ |
| 3 | **`read_weight`: wire it up or delete it?** | Authored 0.0→0.9 on all nine rungs and **read by nothing** — its only live consumer sits behind a rollout those rows disable. ≤184 bytes/fighter/snapshot, so the cost argument is weak; the legibility one is not. | ↓ |
| 4 | ✔ **mostly answered** — does "harder" mean deals more damage, or is harder to beat? | Settled by fixing the clock: at the shipped 480s limit bouts RESOLVE, and rung 5 neither out-damages nor outlives rung 3. | ↓ |

⇒ **1 and 3 interact**: `read_weight` is inert *because* the shipped rows disable
the rollout, so an answer to 1 that made the demo compose `ambition_content`
changes nothing for 3 — but an answer that removed the floor would make every
composition's ladder explicit, which is the context 3 is decided in.

⛔ **NOT on this list, because it is a defect rather than a decision:**
`D-BRAIN-MENU` in [`queue.md`](queue.md) — the brain scores movement and attack
independently, so it approaches-and-attacks on the same tick, every neutral press
converts to a dash attack, and **the shipped fighter throws one move 81% of the
time and never a smash or a tilt**. ⚠ Its remedy is a scoring-shape change and
*will* need a design call, but the defect is established and does not need one.

---

## Who owns Smash's CPU difficulty ladder — and the demo has been fighting the floor

⭐⭐⭐ **THE QUESTION HAS A BETTER FORM, FOUND 2026-09-04 AFTER FOUR SEPARATE
DEFECTS TURNED OUT TO BE ONE MECHANISM.** `profile_for_level` is:

```rust
ladder.and_then(|l| l.level(level)).cloned()
    .unwrap_or_else(|| FighterBrainProfile::for_level(level))
```

⇒ **Two authorities answer "what does rung N mean", and both ship.** The authored
`.ron` wins where a composition installs one; the engine floor wins where none
does — and **nothing at the call site says which you got.** ⚠ The function's own
doc names the shape: *"a rule about which of two sources wins cannot be enforced
by the source that loses."* It knows it is arbitrating.

⇒ **Four defects I reported separately are four exits from that one fork**, each
"the losing authority answered and looked plausible": a rig measuring the floor's
weights under a header claiming the authored rows; `UtilityWeights::default()`
turning out to BE the level-9 row, so every rung scored identically; the floor
arming the L3 rollout at level 6 where the shipped rows disable it everywhere; and
a long characterisation of rollout behaviour no player could ever reach.

⭐ **So the question to answer is not "who owns the ladder" — which has no
principled answer — but "WHICH OF THESE TWO AUTHORITIES SHOULD EXIST", which has
one, and it is this repo's own: _one authority per question_.**

⇒ **And the price of the current answer is now measured.** A composition that must
supply a ladder, or a floor that REFUSES rather than substitutes, would have made
those four defects **impossible instead of findable**.

⭐⭐ **THE COST OF REMOVING THE LOSER IS ENUMERATED, not estimated.** Two questions
decide how expensive a fork removal is, and both have answers here:

1. **Does it rewrite authored content?** ⇒ **No.** The nine rungs in
   `fighter_brain_ladder.ron` stay exactly as written — the floor is a *fallback*,
   not a spelling, so removing it changes which compositions must supply a ladder
   and touches no authored row. ⚠ A sibling fork closed the same day
   (`boss.cleared` retiring a mirror slice) had the same property, and it is what
   made that one an afternoon's work.
2. **How many compositions rely on the fallback?** ⇒ **There is exactly ONE
   production installer of `AuthoredFighterLadder`** — `game/ambition_content/src/plugin.rs:61`.
   Every `AuthoredFighterLadder` insertion in
   `crates/ambition_platformer2d_actor_monolith/src/features/ecs/brain_builders.rs`
   is inside `#[cfg(test)]`. ⇒ **So every composition that does not include
   `ambition_content` fights the floor**, and that set is exactly the standalone
   demo apps — including the smash demo, which is the one that has been measuring
   wrong all along.

⇒ **Which makes the decision concrete.** Removing the floor means the smash demo
app must supply a ladder: either compose `ambition_content` (the 38-dependency
option already costed above) or ship its own nine rows. ⭐ **The second is a small
file and no dependency**, and it would also let the demo diverge deliberately from
the game's ladder instead of accidentally. ⚠ Against that: the floor
exists so a demo with no authored content still runs, which is a real requirement
and not one I am proposing to drop. ⇒ The trade is *convenience for one class of
composition* against *four defects that each took a day of instrument work to
see*, and that is a trade you can now make with numbers on both sides.

⚠ Pinned by `the_floor_and_an_authored_ladder_disagree_and_the_caller_cannot_tell`,
which fails loudly if the two ever agree — with a message saying to re-derive the
findings rather than adjust the test.

⛔ **The fact first, because it is worse than the question.** The standalone smash
demo app gives **every CPU rung the same utility weights**. `profile_for_level`
prefers `Res<AuthoredFighterLadder>` and falls back to
`FighterBrainProfile::for_level`, whose `utility_weights` is
`UtilityWeights::default()` — which *is* `v1()`, i.e. **the level-9 row** — for
every level. The authored rows are inserted by `ambition_content`, and neither
`ambition_demo_smash` nor `ambition_demo_smash_app` depends on it.

⇒ In the shipped game (`ambition_app` composes `ambition_content`) your CPUs get
the authored ladder. In the demo app — **and therefore in every ladder-rig
measurement this project has ever recorded** — they do not. The rig prints the
condition now and two tests pin it (`the_ladder_the_demo_runs`), but neither
repairs it, because the repair is your call:

- **(a) The demo composes `ambition_content`.** The demo then matches the shipped
  game exactly, which is what a measurement rig should do. ⚠ Costs the demo a
  38-dependency crate it currently does without, which cuts against the
  capability-composability doctrine landed the same day.
- **(b) Smash ships its own nine rows.** `for_level`'s own doc invites this —
  *"a game that cares ships its own nine rows"* — and
  [`demos/super-smash-siblings.md`](demos/super-smash-siblings.md) already puts
  *"CPU-fill/difficulty policy"* in what Smash owns. ⛔ But `ambition_content`
  also inserts one, so with both present the winner is a plugin-order accident
  unless the rule is made explicit. It also makes Smash's ladder a second thing
  to tune.
- **(c) Leave it, knowingly.** The demo is a fighting sandbox, not the product,
  and the floor's reflex-only ladder is a legitimate thing to iterate against.
  ⇒ Then the rig's numbers must never be quoted as being about the shipped
  fighter, which is exactly the mistake this page exists to prevent.

⚠ **What the answer changes.** Every recorded ladder result — including
`fighter-brain.md`'s finding that the ladder inverts at the 5→6 boundary — is a
statement about the floor's reflex ladder (reaction, APM, noise, read weight) and
not about the difficulty ladder you authored. Nobody should tune the brain
against those numbers until this is answered.

⭐⭐ **AND THERE IS NOW A REASON TO PREFER (a), found 2026-09-04 after this
question was written.** The floor turns the L3 rollout ON at level 6
(`for_level`: `rollout_depth: if level >= 6 { 12 }`); **your authored ladder turns
it OFF on all nine rows**, with the comment *"Rollout fields remain zero until
rollout fidelity is good enough to enable them without changing lower-level
behavior."*

⇒ So the demo app is not merely measuring a different WEIGHT set — it is running a
search your ladder deliberately disables. Measured consequences, only under the
floor: a rollout fighter selects `Dodge` and `Shield` **zero times in 662
decisions**, and a `6 vs 5` match goes from unresolvable in 60 seconds to both
fighters losing every stock.

⇒ **Option (a) would fix that as a side effect** — composing `ambition_content`
gives the demo the authored rows, which zero the rollout, which removes the defect
from every future rig measurement without touching the brain. ⇒ That is a second
argument for (a) beyond "the demo should match the game", and it is the reason the
38-dependency cost may be worth paying.

⭐ **Reassurance in the same finding:** because your ladder zeroes rollout
everywhere, **no player has ever met this defect.** The precaution in that comment
was right, and the measurement is the evidence for a call you made without it.

## Two of the three Smash characters have no special button — what should they have?

⭐ **The measurement, first, because it is not what anybody assumed.** The demo's
select roster is three characters: *Robot v3* (`smash_duelist_a`), *Robot v2*
(`smash_duelist_b`), and *George Booul*.

⭐ **The DENOMINATOR is checked, not assumed** — "two of three" is only alarming if
three is the whole roster. `SMASH_CATALOG_RON` contains exactly those three
character ids; the `duelist_l1` / `l3` / `l5` / `l6` / `l9` entries beside them are
difficulty POLICY keys, not characters, and counting them would have inflated the
roster to eight. ⇒ Checked because a sibling finding the same day turned on the
opposite error: a *"five gate families unused"* row that nobody had counted the
denominator for, where the world turned out to contain three gates in total. `register_character` gives George's
authored table to George and `fighter_moveset()` to the other two — and counted
off the contracts themselves, that stand-in table binds **18 verbs to George's
26**.

⇒ The eight the Robots did not have: `special`, `special_forward`, `special_up`,
`special_down`, `special_air_down`, `attack_forward`, `attack_dash`, `taunt`.

⛔ **So the special button did nothing at all for two of the three characters** —
not a weak special, a press that resolved to no move. Same for the forward tilt
and the dash attack. ⚠ And the catalog default is one of the two, so it is the
fighter a player gets without choosing.

✔ **One of the eight is closed and it did not need you.** `P11`'s command grab
was already named in the parity inventory as authoring-with-no-engine-work, so
`special_forward` is now `lunge_grab` on both Robots — a slower, longer-reaching,
lunging cousin of the standing grab that pummels and throws through the same four
verbs. That was a queue item, not a design call.

⛔ **THE SEVEN THAT ARE LEFT ARE DESIGN AND THEY ARE YOURS.** Every one is
authoring against a seam the engine already resolves — George is the proof, since
his table binds all eight and needed no engine work to do it. What is missing is
not capability, it is a decision about what these characters ARE.

⇒ **The question is which of these you want**, and they are genuinely different
games:

- **(a) The Robots are stand-ins and should stay thin.** They exist so a match
  has bodies in it; George is the fighter. Then seven dead buttons are correct
  and the command grab was arguably one too many. ⚠ If this is the answer, say so
  and I will revert `lunge_grab` — it is one commit.
- **(b) The Robots are characters and should be finished.** Two robots that
  differ from each other and from George. This is the most work and the most
  game: it is three fighters instead of one.
- **(c) The Robots share one kit that is not George's.** Cheapest of the three
  real answers — one special table, both Robots, deliberately simpler than George
  so the roster reads as "the fighter and the sparring partners".

⚠ **What I am NOT asking.** Not which moves. If you pick (b) or (c) I can author
plausible ones and bring them back for a look — the same way the command grab
went in. The decision I cannot make is whether these characters are supposed to
be fightable at all.

⭐⭐⭐ **NOW MEASURED PROPERLY, AND IT IS NOT CLOSE: at the SAME rung, on the
shipped ladder and the shipped clock, George significantly outfights a stand-in.**

| arm (rung 5 vs rung 5, paired, 12 seeds) | dealt | survival | verdict |
|---|---|---|---|
| **George vs Robot** | **318% : 199%** | 62.7s : 58.2s | ⭐ **higher outfights — SIGNIFICANT** |
| Robot vs Robot *(null control)* | 369% : 389% | 134.3s : 131.5s | *(within spread)* — correctly null |

⇒ **The stand-ins are not merely plainer, they are measurably weaker at identical
difficulty settings.** Difficulty rung, ladder, clock, stage and seed set are all
held equal; the only thing that differs is the kit. ⭐ And a George match RESOLVES
in about 63 seconds where two Robots take 134 — **George kills twice as fast.**

⭐ **The null control is what makes that a measurement rather than a label.** Two
mechanically identical fighters (both Robots receive `fighter_moveset()`) swapped
between seats come back *within spread*, exactly as a null should. So the
instrument is not simply declaring whichever seat it prefers.

⚠ **This measurement did not exist until the rig was repaired to take it**, which
is worth one line because it explains why nobody had it: `--paired` swapped the
RUNGS, so a *fighter* comparison got a control that cancelled the wrong term. The
unpaired attempt gave a 329% : 225% gap and still reported `(within spread)`. ⇒
The question could be asked and could not be answered.

⇒ **What it means for the three options above.** It does not choose one — that is
still a design call — but it removes "the Robots are fine as they are" as a
*measured* position. Anyone picking **(a) keep them thin** is now choosing a
roster where the default character is significantly weaker than the alternative at
the same difficulty setting, which may be exactly right for sparring partners, but
should be chosen rather than inherited.

⭐ **One measurement that should inform this, because it surprised me — and it is
PARTIAL, stated as such.** Two arms of the same run, both on the shipped ladder,
paired, 12 seeds, differing only in who is fighting. Two of the four rung cells
have reported so far:

| rung cell | Robot vs Robot (stocks left) | George vs George |
|---|---|---|
| 3 vs 1 | 2 : 1 | **0 : 0** — both eliminated, ~57s |
| 5 vs 3 | 2 : 2 | **1 : 1** |

⇒ George is markedly more lethal in both cells, in the same direction. Robot
matches do not resolve: each fighter loses one stock of three in sixty seconds
and the match continues. ⚠ **Two cells is not four**, the remaining two are still
running, and I have not run the swap that would rule out this being about
George's damage numbers rather than his kit.

⇒ What it suggests, at the strength two cells support: **a large part of "the CPU
ladder produces sluggish matches" may be that every measurement of it was taken
between two fighters with no specials.** Not proof the Robots need kits — but it
does mean the pacing complaints and the roster question may be one question.

⇒ **This also touches the ladder-ownership question above.** `ladder_rig` defaults
to the two Robots, so every number in `fighter-brain.md` describes the thin
fighters and not the authored one.

## `read_weight` is authored on all nine rungs and does nothing — wire it up, or delete it?

⭐ **The measurement, and it is not a judgement call.** `read_weight` rises 0.0 →
0.9 across the shipped `fighter_brain_ladder.ron` and reads like one of the
ladder's five difficulty axes. It has two consumers:
`HabitModel::read_bonus`, which has **no production callers at all**, and
`habits.read(situation)` inside `refine_by_rollout` — which returns immediately
unless `rollout_depth > 0 && rollout_k > 0`, and the shipped rows set both to zero
everywhere.

⇒ **So no fighter has ever used it.** Confirmed three ways: the source chain, the
absent caller, and a rig arm with rung 5's `read_weight` zeroed that came back
**byte-identical** to its control. See
[`engine/fighter-brain.md`](engine/fighter-brain.md).

⚠ **It is not free — but I said that more forcefully than the number deserves, so
here is the number.** `habits.observe` runs on every decision and the model is
written into every rollback snapshot. The state is a `BTreeMap` over
(situation × choice) = **5 × 6 = 30 rows maximum**, each serialized as
`u8 + u8 + f32` = 6 bytes, plus a `u32` count. ⇒ **At most 184 bytes per fighter
per snapshot**, and fewer in practice since only observed pairs exist.

⇒ **So the cost argument is weak and should not drive the answer.** 184 bytes is
not a reason to delete anything. ⭐ The real argument is correctness and
legibility: a ladder with five knobs of which one silently does nothing is a
ladder nobody can reason about, and every rung was tuned by someone who believed
all five worked.

⇒ **Two fixes, and which one is right is yours because it is a question about what
the ladder is FOR:**

- **(a) Wire it into L2.** `read_bonus` already exists and does the obvious thing
  — shade a choice's score by `read_weight × (frequency − uniform)`. Calling it
  from the L2 scorer would make opponent-modelling a real difficulty axis at
  every rung, independent of the rollout. ⚠ This CHANGES how every CPU above
  level 3 plays, and it is the largest behavioural change anyone has proposed to
  the fighter.
- **(b) Delete it.** Drop the field, the nine authored values, the `observe` call
  and the snapshot rows. The ladder keeps four honest knobs instead of five, and
  the rollback snapshot gets smaller. ⚠ This gives up an axis the ladder was
  clearly designed around — the rows were tuned as though it worked.

⚠ **What I am NOT asking**: how strong the reads should be. If (a), I can wire it
and measure the ladder before and after; the rig can now do that properly.

⭐ **One thing that should inform it.** The `.ron`'s comment keeps the rollout
fields zero *"until rollout fidelity is good enough to enable them without
changing lower-level behavior."* ⇒ Zeroing them already changed lower-level
behaviour — it switched off the entire read system — so the precaution caused
what it was guarding against. Option (a) is also the one that makes that comment
true again, because it moves reads out from behind the rollout.

## ✔ MOSTLY ANSWERED BY MEASUREMENT — was: does "harder" mean *deals more damage* or *is harder to beat*?

⭐⭐ **I raised this as a definition question I could not settle by measuring. I was
wrong about that, and the thing that settled it was fixing the clock.**

The rig ran 60-second bouts; the shipped match is **eight minutes**. On the short
clock no bout could end, so stocks tied everywhere and every verdict fell through
to the damage tiebreak — which is exactly what made the question unanswerable. At
the shipped clock every bout **resolves** (`0 : 0` stocks, both fighters
eliminated), and **survival time becomes a real second signal**:

| cell | survived (hi : lo) | dealt (hi : lo) | verdict |
|---|---|---|---|
| 3 vs 1 | 85.1s : 80.6s | 299% : 208% | ✔ higher outfights |
| 5 vs 3 | **97.3s : 98.7s** | **300% : 360%** | ⛔ **LOWER outfights** |
| 6 vs 5 | 103.6s : 104.7s | 346% : 361% | LOWER *(within spread)* |
| 9 vs 6 | 112.0s : 116.5s | 407% : 390% | higher *(within spread)* |

⇒ **Rung 5 does not survive longer than rung 3 either** — 97.3s against 98.7s.
⭐ That is what answers it: a patient-but-stronger rung 5 would have to beat rung 3
on damage or on survival, and it beats it on neither. **The "patience is invisible
to a damage metric" defence required rung 5 to be winning on some axis, and there
is no axis.**

⚠ **What is left for you is smaller and it is real.** Two things:

1. **Only `5 vs 3` is established, and that is now REPLICATED rather than
   provisional.** ⭐ I said this needed more seeds before anyone retuned a curve,
   so I ran it: **28 seeds at the shipped clock returns an identical picture** —
   `3 vs 1` significant, `5 vs 3` significant, `6 vs 5` and `9 vs 6` within
   spread, every qualifier unchanged. ⇒ More than doubling the evidence moved
   nothing, so **`6 vs 5` is noise and the shipped ladder has exactly ONE bad
   rung.** ⚠ `5 vs 3` is now significant at 12, 24 and 40 seeds on the short clock
   and at 12 and 28 on the shipped one — it is not a sampling artifact.
2. **The fix is still a design call — but it now comes WITH candidates measured
   at the shipped clock**, so you are choosing between numbers rather than
   guessing. Three rung-5 settings for the pair, 16 seeds each, everything else
   untouched:

   | rung 5's `frame_advantage` / `expected_payoff` | dealt (5 : 3) | verdict at `5 vs 3` |
   |---|---|---|
   | **0.50 / 0.30** *(shipped)* | 306% : 360% | ⛔ **LOWER outfights** |
   | 0.40 / 0.20 *(halve the rise)* | 300% : 362% | ⛔ **LOWER outfights** |
   | **0.30 / 0.10** *(hold flat at rung 3's)* | **336% : 313%** | ✔ **higher outfights** *(within spread)* |

   ⇒ **Halving the rise does not help at all** — it is as inverted as the shipped
   value. Only removing the rise flips the verdict, and when it does, rung 5 wins
   on damage instead of losing.

   ⚠ **And rung 5 does NOT collapse into rung 3 if you take that.** The two still
   differ on reaction (300ms vs 400), APM cap (200 vs 120), execution noise (0.20
   vs 0.30), `kill_potential` and `stage_risk`. ⇒ What the flat setting removes is
   only the pair that was making rung 5 refuse to commit — the reflex advantage
   then asserts itself, which is what the third row shows.

   ⭐⭐ **AND THE WHOLE-LADDER VERSION IS NOW MEASURED TOO.** Holding the pair at
   `0.30 / 0.10` on **every row from level 4 up** (six rows; `kill_potential`,
   `stage_risk` and every reflex knob left rising as authored), shipped clock, 12
   seeds:

   | cell | shipped | pair held flat |
   |---|---|---|
   | 3 vs 1 | ✔ higher | ✔ higher |
   | 5 vs 3 | ⛔ **LOWER** | ✔ higher *(within spread)* |
   | 6 vs 5 | LOWER *(within spread)* | LOWER *(within spread)* |
   | 9 vs 6 | higher *(within spread)* | ✔ **higher** *(significant)* |

   ⇒ **No cell is significantly inverted, two cells improve, none regresses**, and
   the survival medians rise monotonically (85 → 101 → 116 → 122s) where the
   shipped ladder's top pair went backwards (85 → 98 → 114 → **113**).

   ⛔ **What this does NOT settle, and it is the part that is yours.** Holding
   those two flat means **higher rungs no longer weight frame safety or move
   power more heavily than rung 3 does** — that is a statement about what a harder
   CPU IS, not a bug fix. ⚠ The winning cells are partly *within spread*, it is 12
   seeds, `6 vs 5` stays undetermined and still leans LOWER, and this tests ONE
   alternative rather than the space. ⇒ Whether the ladder should raise these
   two weights AT ALL, at any rung, is the question the numbers cannot answer,
   because it is a question about what a harder CPU is supposed to be.

   The cause is `frame_advantage` + `expected_payoff` jointly (isolated
   byte-for-byte). `frame_advantage` is SIGNED,
   so raising it makes a rung penalise its own slow, hard-hitting moves; raising
   `expected_payoff` withholds the power bonus from exactly those. ⇒ Whether higher
   rungs SHOULD weight frame safety more is a question about what your ladder
   means. The measurement says the current progression makes rung 5 worse; it does
   not say what the right progression is.

⭐ **And the rise in survival medians across the ladder — 85s, 97s, 104s, 112s — is
a quiet vote of confidence.** Higher rungs take measurably longer to kill each
other, which is what a working ladder should do, and it holds across every cell
including the inverted one.

⛔ **What I had written here before, and why it is wrong:** that both readings
predict the same numbers and more measuring could not separate them. They do not
— they differ on survival time, which the 60-second clock could not show because
nobody ever died. ⇒ The lesson is the day's lesson again: the question looked like
a definition problem and was an instrument problem.

## The original framing, kept because the reasoning is still the reasoning

⛔ **This is the confound under every ladder number I have produced today, and it
cannot be measured away — it is a definition.**

⭐ The rig's verdict is *"who OUTFOUGHT: stocks taken, then damage dealt."* In
every cell where the shipped ladder measures as INVERTED, the stocks are **tied at
`2 : 2`**, so the verdict falls through to damage. ⇒ The significant result is that
**the higher rung deals less damage in 60 seconds.** It is *not* that the higher
rung loses.

⚠ **And the mechanism makes that ambiguity worse rather than better.** The
inversion is carried by exactly two weights (`frame_advantage` and
`expected_payoff`, isolated byte-for-byte — see
[`engine/fighter-brain.md`](engine/fighter-brain.md)). `frame_advantage` is a
SIGNED feature: slower moves score more negative, so raising its weight makes a
rung penalise committing to its hardest-hitting moves. `expected_payoff` is gated
by the positive part of it, so it withholds the power bonus from exactly those
moves. ⇒ **Higher rungs jab more and smash less.**

⭐⭐ **Which is either a defect or the intended design, and the same numbers
support both readings:**

- **(a) A fighter that refuses bad commitments is PLAYING BETTER.** The ladder is
  doing what it was authored to do; a patient rung-6 is harder for a human to
  open up, and it only looks weak because two CPUs poking at each other for a
  minute produce less damage. ⇒ Then nothing is wrong, and the rig's tiebreak is
  the thing to change.
- **(b) A CPU that will not commit is EASIER, not harder.** A human punishes
  passivity; a rung that never throws its kill move cannot close a stock. ⇒ Then
  the weight progression is backwards above rung 3 and the ladder needs retuning.

⚠ **I cannot settle this by measuring more.** Both readings predict the same
damage numbers. What separates them is what a human experiences, and the honest
statement is that CPU-vs-CPU damage over 60 seconds may simply be the wrong
instrument for "difficulty".

⇒ **What I can do once you answer.** If (b), the fix is scoped and small — the two
weights are named and the arms to validate a new progression already exist. If
(a), the work is on the RIG: a stock-decided verdict, or a longer clock, or both,
and every ladder conclusion in `fighter-brain.md` gets re-derived under it.

⭐⭐ **AND THE EVIDENCE MOVED TOWARD (b) WHILE THIS ENTRY WAS BEING WRITTEN.** I
first wrote that `3 vs 1` being correctly ordered was a weak argument the metric
is not blind, since those rungs differ in reflexes too. Three more arms make it
much stronger:

| comparison | result on the same metric |
|---|---|
| `9 vs 1` | ✔ higher outfights, **significant** |
| `3 vs 1` | ✔ higher outfights, significant |
| `5 vs 3` | ⛔ LOWER, significant |
| `6 vs 5` | ⛔ LOWER, significant |
| `9 vs 6` | ✔ higher outfights, significant |

⇒ **The metric orders the ladder's ENDS correctly and reports a trough in its
middle.** A metric that simply mistook patience for weakness would mis-order the
top of the ladder too — rung 9 is the most patient rung of all
(`frame_advantage: 0.60`+) and it beats both rung 1 and rung 6 decisively.

⇒ So the honest headline is **the ladder SAGS at rungs 5–6**, not that it inverts,
and reading (b) now has to explain only two rungs rather than a whole progression.
⚠ It is still not proof: rung 9's reflexes are far ahead of rung 6's and may be
carrying it past the weight penalty. But "the metric is blind to patience" no
longer fits the data as comfortably as it did an hour ago.

⭐ **And the fix, if (b), is now scoped to two knobs on two rungs**, with a
measured prediction attached: stepping `frame_advantage` and `expected_payoff`
back one rung at rung 6 already removes that cell's significance, and back two
rungs at rung 5 flips it. Whatever the right progression is, those are the two
dials and the middle is where they are wrong.

## Waiting on maintainer measurement, not a decision

### The residency limit open work 4 needs

A budget policy that keeps the last room's cast resident needs a ceiling, and the
ceiling is a host number: `resident_mb` at Full on the 3090 after a
hub→hall→hub walk. Everything else in that section is measured; this is the one
input that cannot be taken on a software rasteriser.

> ⭐ **THE NUMBER, MEASURED ON CALCULEX 2026-09-03 — no GPU, and it was already
> being produced every gate run.** At the hall entry, through the real authored
> door, at FULL texture resolution:
>
> | run | images | megapixels | **resident** |
> |---|---|---|---|
> | `hall_transition_cover` (control) | 224 | 363.1 | **1452.3 MB** |
> | same, `AMBITION_QUALITY_PROFILE=ultra` | 225 | 363.9 | **1455.8 MB** |
> | `leaving_the_gallery…` — the RETURN leg, hub→hall→hub, at 5.0s | 236 | 503.0 | **2012.0 MB** |
> | same run at 10.0s | 244 | 507.7 | **2030.9 MB** |
>
> `gpu +0 … awaiting gpu N` on every one — nothing uploaded, which is the point:
> `resident_mb` is decoded CPU-side bytes and needs no adapter.
>
> ⓘ **Why these are the FULL-tier figures, stated rather than assumed.** The test
> composition does not seed from the adapter, so it takes
> `default_visual_quality_profile()` = `High` on non-Android, and
> `VisualQualityBudget::for_profile` maps `High` to
> `TextureResolutionScale::Full` — the unsuffixed sprite tree. The hall obeys the
> same tier by Jon's 2026-09-02 ruling, pinned in
> `the_halls_cast_is_realized_at_the_users_tier_never_lower`: *"the hall draws at
> the user's tier, never lower … not want a lower quality tier for gallery
> previews."* ⚠ Open work 6 still describes this leg as *"the gallery (Quarter)
> for the hub (Full)"*, which was the PRE-repair behaviour that same section
> reports fixing.
>
> ⚠ **The 10.0s row said "after the retire" until it was checked, and that was an
> assumption, not an observation.** Residency GREW between the two samples — 236
> images to 244, 2012.0 MB to 2030.9 — and a retire drops pages, so both readings
> are almost certainly BEFORE the retire lands. ⇒ Which makes ≈2.03 GB a PEAK
> while both rooms' casts are held, not a settled steady state, and the settled
> figure is a third measurement nobody has taken. The peak is the right input for
> a budget ceiling either way, which is why the correction does not change the
> answer — only what the number is called.

> ⭐ **SO THE ASK IS ANSWERED: ≈2.03 GB is `resident_mb` after a hub→hall→hub
> walk at full texture resolution, and it needed no 3090.** The round trip peaks
> ~580 MB above the hall entry alone, because it holds both rooms' casts before
> the retire — which is precisely the pressure the "keeps the last room's cast
> resident" policy has to budget for, and precisely why the entry asked for the
> walk rather than the room.
>
> ⚠ **Read the detail below before quoting these.** The first two rows differ
> only by an env var that turns out to be INERT — one configuration measured
> twice, not a tier comparison — and `capture_scene` reports 119.4 MB for the
> same room because ITS composition seeds quality from the Cpu adapter and loads
> `sprites_potato`. The number is a property of the composition as much as the
> room.

ⓘ **2026-09-03, calculex — the ADAPTER may not be what blocks this, which would
make the ask smaller than 3090 time.** Two things were checked, and neither is a
claim that the number has been taken:

* **`resident_mb` is adapter-independent by construction.** The census computes
  it as `width * height * per_pixel`, where `per_pixel` comes from the image
  FORMAT's `block_copy_size` (`crates/ambition_render/src/asset_census.rs:218`).
  It is decoded-image arithmetic, not anything the GPU reports, so llvmpipe and a
  3090 should agree for the same assets and the same walk.
* **The census DOES emit on this host.** `capture_scene` renders offscreen
  through lavapipe and prints the full `[image-census]` line including
  `…MB resident`; the headless room harness does not, because it composes no
  render app. So the missing piece is a WALK DRIVER, not an adapter:
  `scripts/profile_desktop.sh` is the documented driver and its windowed path
  refuses without a display *("a windowed run needs a display, and failing here
  is the point")*.

⚠ **Two caveats that keep this an ⓘ and not a ✔.** The walk has not been driven
here — `capture_scene` takes `--press`/`--route` but room-to-room transitions
through doors were not attempted. And quality seeds itself to `potato` on a Cpu
adapter, so a Full measurement needs `AMBITION_QUALITY_PROFILE=Full` and would
have to be checked on the `[census] visual_quality` row rather than assumed.
⇒ The decision this entry asks for is unchanged. What changed is the cost of the
input: possibly a driver on any host rather than time on the one 3090.

> **MEASURED 2026-09-03, and the two caveats above resolve in opposite
> directions.**
>
> ✔ **The census is takeable here, and the number is real.** `capture_scene
> hall_of_characters player … 640x360 --warmup 60` on lavapipe prints
> `[image-census] total 235 images, 29.9MP, 119.4MB resident | gpu +235 | awaiting
> gpu 0 | re-decodes 0`. So `resident_mb` after a room is loaded needs no GPU.
>
> ✔ **AND THE WALK DRIVER EXISTS — the first caveat was simply wrong.** I checked
> `capture_scene` and `profile_desktop.sh` and concluded no driver could cross a
> door here, without looking at the test suite.
> `game/ambition_app/tests/hall_transition_cover.rs` builds
> `build_visible_app(VisibleRenderMode::NoWindow, …)` and drives *"the REAL
> transition, resolved through the room graph rather than synthesised: stand in
> the Hall door and press interact"*. It is a module of `app_it`, so **that hub →
> hall crossing already runs on this machine in every gate run**, and it reads the
> ledger in process (`resident_character_pages`) rather than parsing a printed
> line. Its own comment records `22 → 226 resident at the hall entry`.
>
> ⛔ **The second caveat is CONFIRMED, and by measurement rather than the reason I
> gave.** Two runs, one with `AMBITION_QUALITY_PROFILE` unset-equivalent and one
> with a VALID `ultra`, both log *"visual quality seeded to `potato` for a Cpu
> adapter (llvmpipe)"* and produce a byte-identical census — same 235 images, same
> 29.9 MP, same 119.4 MB. So the tier does not move through that lever in this
> tool, and **a high-tier residency figure is not takeable this way**. ⇒ That is
> the one thing still genuinely blocked, and it is a quality-selection question,
> not an adapter one.
>
> ⚠ **My own error, recorded because it cost two runs:** the caveat above said
> `AMBITION_QUALITY_PROFILE=Full`. There is no `Full`. The labels are
> `potato|low|medium|high|ultra` (`settings/video/quality.rs`), and
> `capture_scene`'s own help already says `AMBITION_QUALITY_PROFILE=ultra`.
> `from_label` returns `None` on an unparseable value *"so a typo boots the user's
> OWN setting instead of silently substituting a tier they did not choose"*.
>
> ⓘ **And the warning for that case EXISTS and is well written** —
> `log_quality_profile_override` (`ambition_render/src/quality.rs:182`) warns
> *"…is not a profile; using the saved setting instead. Expected one of: potato,
> low, medium, high, ultra"*. It simply never fired: **neither** run printed it,
> the valid one included, and neither printed the success message either. So
> `VisualQualityPlugin::build`'s logger has nowhere to write.
>
> ⇒ **AND THE MECHANISM IS EXACT.** `build_visible_app` drops `LogPlugin` for
> `NoWindow`/`OffscreenGpu` — its comment says why: *"tests build several Apps
> per process; the tracing subscriber is process-global."* `capture_scene` adds
> it back, but **after** the plugin group
> (`capture_scene.rs`, `app.add_plugins(bevy::log::LogPlugin::default())`
> following `build_visible_app_with`). So every line a plugin emits during
> `build()` is written with no subscriber installed and is lost, while anything
> logged later from a SYSTEM survives — which is exactly the pattern observed
> here: no override message, but the adapter-seeding line printed normally.
> ⚠ This is not specific to quality. **Any** build-time log in any plugin is
> silent in this tool, which is worth more than the one message that led me to
> it.
>
> ⛔ **AND THE LEVER IS INERT HERE, NOT MERELY UNREPORTED.** Both runs loaded a
> byte-identical asset set — 2 images from `sprite_packs/full/` and 14 from
> `sprites_potato` — so `AMBITION_QUALITY_PROFILE` changed nothing about what
> became resident, valid label or not.
>
> ⛔ **AND RESIDENCY IS TIER-DEPENDENT BY DESIGN, so that inertness is the whole
> blocker.** `VisualQualityBudget::for_profile`
> (`ambition_persistence/src/settings/video/quality.rs`) sets
> `resolution_scale` per tier: the potato/low tiers take
> `TextureResolutionScale::Potato`, medium takes `Half`, and **high and ultra
> take `Full`**. Each maps to a different sprite tree — `sprites_potato`,
> `sprites_0_5x`, or the unsuffixed base — so the tier decides which pixels
> become resident, and `resident_mb` must move with it.
>
> ⇒ **So `capture_scene`'s 119.4 MB is the POTATO figure** — it seeds from the
> Cpu adapter and loads `sprites_potato`. The full-resolution figure in the table
> at the top comes from `hall_transition_cover`, whose composition does NOT seed
> from the adapter and loads the base tree: **363 MP against 29.9 MP, twelve
> times the pixels, for the same room.**
>
> ⛔ **AND THE ENV VAR EXPLAINS NEITHER.** Running the test WITH and WITHOUT
> `AMBITION_QUALITY_PROFILE=ultra` gives 1455.8 MB and 1452.3 MB — the same
> configuration twice. The lever is inert in both tools; the twelve-fold gap is
> which composition seeds quality from the adapter, not which tier was asked for.
> ⇒ **Two tools on one machine disagree about residency by 12× for reasons that
> have nothing to do with hardware**, which is worth more to open work 4's budget
> policy than either number alone: a ceiling is meaningless until the composition
> that produced it is named beside it.
>
> ⚠ **Recorded because I got this wrong once in the other direction too:** an
> earlier version of this note said the asset set "does not move with the tier at
> all", inferring an invariant from two runs that were both potato. Two identical
> measurements of the same configuration are one measurement.

### D-RASTER-3's remaining half

Splitting the weak-GPU 2.54× between framebuffer scale and MSAA needs an
interleaved A/B on real weak-GPU hardware with the independent
`AMBITION_MAX_SCALE_FACTOR` and `AMBITION_MSAA` knobs, multiple reps per arm,
build/features/profile held constant. ⛔ Explicitly not lavapipe: the row says so
and the substitution is what made the original result unattributable.

### Switch Pro outer stick range

The remaining cross-machine controller question needs the actual hardware
measurement: run the existing `Shift+F6` axis probe on both machines, push the
Switch Pro to each extreme/corner, and compare reported peak magnitude.

The proposed shared outer-saturation fix should be judged only after that number
exists. This is tracked in the execution queue as an external measurement, not a
maintainer design decision.

### ~~Which character owns each per-fighter FX sheet~~ (raised 2026-09-02, withdrawn 2026-09-02 — no decision was needed)

The question assumed the demand seam needed a sheet → CHARACTER ID table. It
does not: a realized character carries its own prepared moveset
(`PreparedCharacterDefinition.kit.projectable_moveset()`), and the moveset's
`Vfx` events name the rows. `character_sprites::demand_character_fx_sheets`
asks that moveset the frame the character realizes and decodes whichever
character-owned sheets its rows live on (`fx::owned_fx_sheets_named_by`) —
ownership is read off the content that fires the effect, not off a name.
Landed in `asset-preparation-and-residency.md` §2. The two never-wired sheets
below are unaffected: a sheet no moveset names is now never decoded, which is
the correct residency for art nothing can request, and the wiring question
stays the owner's.

## ✔ WITHDRAWN 2026-09-02 — "is 8% of the floor crate worth an encoder split?"

Raised here the same day and withdrawn the same day: it is an ENGINEERING
question, not a product one, so it is answered in
[`engine/control-authority-and-ai-policy.md`](engine/control-authority-and-ai-policy.md)
instead. Short answer: do not split. Of the three available shapes, two are
refused by rules the repository already holds (no service locator; the orphan
rule again one crate up) and the third fails the plan's own acceptance criterion
4, because `ambition_mount` holds a `Brain` by value and does not depend on
`ambition_combat` — so moving `Brain` into a combat crate makes a movement-only
game link one. Recorded here only so the question is not raised a third time.
