# Awaiting a maintainer decision

Questions that reached the point where the next step is **Jon's judgement, not
more engineering**. Each one was scoped far enough that the choice is real and
the work after it is small; none is blocked on unknowns.

This is the counterpart to [`maintainer-decisions.md`](maintainer-decisions.md),
which records what Jon HAS decided. Nothing here is a decision. When one is made,
it moves there and this row is deleted.

⚠ **These are not bugs, and they are not agent preferences waiting for a rubber
stamp.** Every one is a case where two defensible answers exist and picking
between them is authoring or product design. Where I have a recommendation it is
marked as such and it is weak — the reason the row is here is that I should not
be the one choosing.

---

## ⇥ INDEX — **2 open** (recounted 2026-08-09)

This file is 1,800+ lines and had no index. Its own header warns that a decision
file which stops being readable stops being read; length does that as surely as
stale rows do. **⚠ ordered by what each one BLOCKS**, not by date, because that
is the only ordering a maintainer can triage from.

⭐ **the count has been wrong in BOTH directions now.** It said *"1 open"* while
omitting the row that blocked most (corrected 2026-08-08), then said *"2 open"*
after the blocking one had shipped (corrected 2026-08-09). ⇒ **recount against
the sections when you touch this file**; an index nobody recounts is the thing
its own header warns about, one level up.

| Question | Blocks | State |
|---|---|---|
| **Is an enemy a CHARACTER, or an ARCHETYPE wearing one?** (queue D48) | goblins/Iron Mary art + behaviour | ▢ **scoped 2026-08-09, needs Jon** — see below |
| ~~Is a crate name part of the rollback wire format?~~ (S30) | ~~the WHOLE carve campaign~~ | ✔✔ **IMPLEMENTED 2026-08-09 as (b′), schema v20** — `3333a4b0f`. Nothing owed. |
| Is a SESSION scope marker construction provenance? | tracks K2b-i | ⚠ answered by agent — see below |
| **Should the superproject's measurement-submodule pointer advance?** (2026-08-09) | nothing — but the parent tree reads dirty until it is settled | ▢ **needs Jon**, 30 seconds — see below |
| **Give rust-analyzer its own target dir?** (queue D59, 2026-08-09) | nothing — a build-hygiene setting in Jon's untracked `.vscode/settings.json` | ▢ **needs Jon**, one line — ⚠ **offered as hygiene, NOT as a fix** |
| **HOW were you dying when the level did not restart?** (queue D68, 2026-08-09) | the death-restart investigation — ⛔ **it cannot proceed without this** | ▢ **needs Jon** — a fact only he has, not a decision |

⇒ **4 open**, and **only the last one blocks anything.**
⚠ the heading said *"2 open"* while this line said *"1 open"* for a day — the
third miscount, and this time the heading was accidentally right for the wrong
reason. ⇒ **the two numbers are now derived from the same table in one edit.**

⚠ **S30 shipped as (b′), NOT the (b) this file recommended** — the recommendation
was refuted by the diff it cited. The section below carries both, newest first,
because *how* a stated answer turned out wrong is the part worth keeping.

⛔ **THE INDEX SAID "1 OPEN" AND OMITTED S30, WHICH IS THE ONE THAT BLOCKS
MOST.** Corrected 2026-08-08. An index that undercounts is worse than no index:
its own header says a decision file that stops being readable stops being read,
and a row invisible to the index is a row nobody triages. S30 had been sitting
below the fold gating every crate carve in the workspace — including the
cheapest three-crate cut available anywhere (`ambition_input`, which imports
exactly ONE item from core).

Both rows are being **answered rather than asked**, per Jon's 2026-08-07 note
(*"I've left rollback design to agents"*). ⚠ S30's answer is a behaviour change
to a determinism-critical hash and is flagged as such in queue D37 rather than
presented as settled — Jon can overrule it. Everything below the table is
scoped, small, and waiting; none is blocked on unknowns, which is this file's
entry condition.

---

## Is an enemy a CHARACTER, or an ARCHETYPE wearing one? (queue D48, 2026-08-09)

Jon, from play: *"In the sky enemy the instance of iron marry doesn't use her
swordgun, she shoots fireballs, which is not something her character should be
able to do. **I suppose we do need a distinction between unique characters and
re-spawning archetype characters.**"*

### The fact

`EnemySpawnSpec` separates two questions in its own doc comments and provides no
third:

```rust
/// What it DOES: the roster brain key the archetype is selected by.
pub brain: CharacterBrain,
/// What it LOOKS LIKE: a catalog character id.
pub character_id: Option<String>,
```

⇒ **there is no slot for *which character this is*.** Iron Mary's catalog row
declares `default_brain: "melee_brute_brute"` and
`default_action_set: "brute_lunge"`; nothing on the enemy-spawn path ever reads
them. Her fireballs are the shark-rider brain doing exactly what it was told.

### ⭐ The authored content ALREADY makes the split — measured 2026-08-09

Every `EnemySpawn` name checked against the catalog's 125 display names:

```text
65 EnemySpawn instances, all four worlds
  41 (11 distinct)  the name IS a catalog character   Iron Mary, Pirate Raider,
                                                      Burning Flying Shark, …
  24 (21 distinct)  the name is NOT                   Skirmisher, Target,
                                                      pg_goblin_a/b/c, small skitter,
                                                      medium striker, large brute,
                                                      lab finite sandbag, patrol cutter
```

⇒ **whoever authored these was already making your distinction**, as a naming
convention nothing reads. ⚠ and `medium striker` / `large brute` appear as
*names* — the same strings the encounter system uses as `kind`s — so the
archetype vocabulary is already leaking into the name field.

### The fork

* **(a) an enemy IS a character**, and the brain is an override on its catalog
  default. Iron Mary in the sky is Iron Mary, so she lunges unless the level says
  otherwise.
  **Cost: ~24 spawns** need a character named or an explicit archetype marker;
  the other 41 already name one. Makes unique characters cheap, archetypes
  slightly more verbose.
* **(b) an enemy is an ARCHETYPE wearing a character** — today's model, made
  explicit and honest. Iron Mary in the sky is a shark rider that looks like Iron
  Mary.
  **Cost: near zero to declare**, but it recasts 41 authored spawns as costumes
  and unique characters then need a marker to get their own kit.

⭐ **the numbers lean (a)**, and so does the content: (b) would make *"Iron Mary"*
a costume, which is plainly not what whoever typed it meant.

⚠ **this is a product question, not an engineering one** — it decides whether
naming an enemy in a level grants it a kit. Both arms are implementable; I am not
choosing which game you are making.

⛔ **do not answer this by fixing the goblins** — that already landed (queue D39)
and it deliberately made `character` mean ART ONLY, cross-referenced to
`EnemySpawnSpec::character_id`, precisely so this question stayed open and yours.

⚠ **whichever arm wins, queue D56 must land with it**: the renderer keys its
sheet lookup on the DISPLAY NAME, so authoring character ids without fixing that
un-arts the very spawns it is meant to fix.

⭐ **and the pairing is FORCED, not a preference — measured 2026-08-09.** The
field D56 needs is already on the struct the renderer reads
(`ActorConfig::sprite_character_id`), and its own doc says it is resolved
*"via its display name"*. ⇒ **the two hold the same value today**, which makes
the deadlock exact and symmetric:

* **D56 alone is invisible** — nothing yet makes the two differ, so the fix looks
  like a no-op and a reviewer would be right to reject it as churn;
* **this decision alone is a REGRESSION** — authoring `character_id` is precisely
  what makes them differ, and the renderer would still key on the name.

⇒ **neither is landable alone, and each looks pointless or broken without the
other.** Nothing is owed from you here beyond the answer above; this is recorded
so that whoever implements it does not ship half.

⚠ **one thing this does NOT fix, so you are not surprised**: the sky Iron Mary
also lost her shark, because Jon's 2026-07-06 editor session nulled the mount
references in `sandbox.ldtk`. That is queue D49, a separate mechanical restore of
four values, already in flight. **Same entity, different bug.**

> **Rows 1–7 were deleted 2026-08-05.** Jon answered all seven on 2026-07-29
> and every answer is a row in [`maintainer-decisions.md`](maintainer-decisions.md)
> — nameplates are low priority, `Interact` needs a design discussion, BUILD the
> portrait resolver, use enemies that already exist, DI is on and Smash physics
> is a direction for the flagship game, generic versus ends on health while Smash
> Siblings is 3 stocks, and the reserved-surround branch is moot because there is
> no score. ⛔ they sat here for a week wearing their answers as a "Jon's
> Thoughts" paragraph under an open question, which is exactly what this file's
> own header forbids: *"When one is made, it moves there and this row is
> deleted."* **A decision file that still contains answered questions trains its
> owner to stop reading it.**


> **Rows 8, 10 and 12 were deleted 2026-08-05, and each had CLOSED IT ITSELF.**
> #8 (which block did you stand on) says *"✔ CLOSED — not reproducible at HEAD"*;
> #10 (does the app-local policy bind presentation) says *"RESOLVED … option (a)
> was implemented"* in its own title; #12 (fixed timestep on the web) says
> *"✔ DONE"* and quotes Jon's answer. All three kept their whole write-up under a
> heading that still read as an open question. ⚠ **the numbering is left with
> gaps on purpose** — the queue cites these by number, and renumbering would make
> every one of those citations point at a different question.
>
> Their reasoning is in git. What a decision file is FOR is the list of things
> Jon still has to rule on, and every closed row on it makes that list less true.


## HOW were you dying when the level did not restart? (queue D68, 2026-08-09)

⚠ **This is a QUESTION, not a decision** — a fact only you have. It is in this
file because the investigation genuinely stops without it, and guessing would
mean building a third fixture for a route you may never have taken.

**Your report:** *"when you die the level doesn't restart, you just stay right
where you were."*

### What is now proven, so you know what NOT to re-report

A test composes Mary-O exactly as `mary_o_app` does, moves her 600 px off spawn,
kills her **with a real hit**, runs past `DEATH_DWELL`, and asserts she is back
at `world.spawn`. **It passes.** It was poisoned rather than trusted — deleting
the replay request turns it red with

```text
she is still at Vec2(664.6, 384.1)
```

⇒ **that red is your sentence.** The test can see exactly the bug you describe,
and on the fatal-hit route it does not happen.

### The one thing that would explain it

**Three routes publish a death**, and only two are covered end to end:

| route | reaches the beat? | tested end to end |
|---|---|---|
| a hit (`death_respawn_player`) | yes | ✔ new, `5fddacd76` |
| the level timeout (`spend_lives_on_death`) | yes | ✔ pre-existing |
| **a kernel reset — a pit, a spike, any hazard** (`publish_kernel_reset_death`) | yes | ⛔ **NO** |

⇒ **if you were falling in a pit or landing on a spike rather than being hit,
that is the untested route** and the likely home of your bug.

### What I need from you — one line

**Roughly how were you dying?** *"an enemy hit me"* / *"I fell in a pit"* / *"a
spike"* / *"I don't remember"*.

⭐ **"I don't remember" is a fine answer** — it just means I build the third
fixture speculatively rather than aimed. ⛔ **What I will not do is go hunting
through the composed app for a failure the tests say does not occur**; that is
how a day gets spent proving something already true.

⚠ **and one thing your answer does NOT settle**: you also reported that *"some
blocks from the last run remain spent"*. That was coupled to this question and is
now **un-coupled** — the replay provably fires, so those stale blocks are their
own bug (queue D70) whatever you answer here.


## Give rust-analyzer its own target dir? (queue D59, 2026-08-09)

**One line in a file only you can edit, and I am deliberately NOT selling it as a
bug fix — because the theory behind it just got weaker.**

`.vscode/settings.json` is untracked (your local config, hands off), so this is
yours:

```json
"rust-analyzer.cargo.targetDir": true
```

It puts RA's artifacts under `<target>/rust-analyzer` instead of the shared build
directory.

### What is actually established

* ✔ **RA competes for the build lock and for cores.** Observed directly: a
  verification run blocked on *"Blocking waiting for file lock on build
  directory"*, and `ps` showed RA's `cargo check --workspace --all-targets
  --keep-going` holding it. **11 `rust-analyzer` processes were live** at a later
  check (the first count was 3 — it has grown, not shrunk). ⇒ **the setting is
  worth taking on those grounds alone.**
* ⚠ **What is NOT established is that it fixes D59.** The theory was that RA's
  `check` writes `.rmeta` where a test build needs `.rlib`, explaining why
  `cargo test -p ambition_render` could not link. **Then it linked clean — 94
  tests passed — with 11 RA processes running.** ⇒ the hypothesised cause was
  present in force and the failure did not occur. That is evidence *against*
  *"RA makes this fail"*, and only neutral toward *"RA can make this fail
  sometimes"*.

### What I am asking

**Just take it or don't** — it is hygiene either way. ⛔ **Do not treat it as
closing D59.** The only thing that settles that is: set the override, remove the
main target dir once, and see whether the link stays green *through a day of
editing*. A single green run is not that test, and treating one as such is how an
intermittent bug gets closed twice.

⭐ **I am flagging my own weakened evidence rather than letting the
recommendation ride on it**, because this row was written when the theory looked
strong and would otherwise read as more confident than it now deserves.


## Should the measurement submodule's POINTER advance in the superproject? (2026-08-09)

**Thirty seconds to answer, blocks nothing, and I deliberately stopped short of
deciding it because the submodule is your GitHub repo, not mine.**

`dev/ambition_dev_measurements` accumulates append-only measurement ledgers. Two
of them picked up one machine-generated row each during the 2026-08-08 runs:
`run_tests_cost.jsonl` (a full 9-job sweep, 813.1 s wall / 177.2 s executed) and
`compile_graph.jsonl` (the per-crate line census).

**I committed those rows INSIDE the submodule** (`9a570d7`, on its `main`) so a
clean cannot lose them — that is what an append-only ledger is for. **I did not
move the superproject's pointer.**

⇒ the consequence, stated plainly: **`git status` in the parent reads
`M dev/ambition_dev_measurements` and will keep reading that** until the pointer
moves. Everything of mine is committed; this one line is the whole residue.

* **(a) advance the pointer with each measurement commit.** The parent tree goes
  clean, and the superproject records which measurements a given commit was
  reasoned from — which is arguably the point of vendoring them at all.
  ⚠ cost: every agent measurement run then produces a parent-repo commit whose
  only content is a submodule bump, and your own in-flight work in that submodule
  becomes something a bump could surprise.
* **(b) leave the pointer pinned and let it read dirty.** ⚠ cost: a
  `git submodule update` checks out the older commit and my appended rows leave
  the worktree (recoverable — they are on the submodule's `main`, not lost).
* **(c) stop vendoring measurements as a submodule** — out of scope for an
  answer here, but if the pointer question keeps recurring, that is the fork
  underneath it.

**My recommendation: (a)**, and I did not just do it because my standing
instruction for this run is *never commit a submodule pointer move* — written
back when that submodule carried your uncommitted work. It carries none now
(`git status` inside it is clean as of `9a570d7`), so the instruction may simply
have outlived its reason. ⇒ **if you say (a), I will also retire that rule**;
if you say (b) I will keep it and stop re-raising this.


## Should DIALOGUE stop the world, or only the talker? (2026-08-01) — ✔ CLOSED

> **JON, 2026-08-03: "dialogue should have the option to stop the world. I'm not
> decided on what I want it to do in game."**
>
> ✔ **JON, 2026-08-06 — the default is decided: SPLIT IT.** Asked to choose
> between building the policy with today's stop-the-world default, defaulting to
> per-seat, or leaving it, he picked *"Build it, default to per-seat"* — over the
> recommendation, which was the conservative default. So dialogue claims only the
> TALKER's input, NPCs keep patrolling through a conversation, and a couch seat
> is no longer frozen by somebody else talking. The per-experience opt-in to stop
> the world stays, because his 2026-08-03 ruling made both expressible a
> requirement. Recorded in `maintainer-decisions.md`; the section below is kept
> for the mechanism it documents, not as an open question.

✔ **The ENGINE answer is settled: both must be expressible.** "Stop the world" is
a capability the dialogue system owes, not a behaviour it picks — so this is a
policy with a default, exactly like #9 above, and the two share a shape.
✔ **and the game-level default is settled too** — Jon, 2026-08-06, *"Build it,
default to per-seat"*, recorded in
[`maintainer-decisions.md`](maintainer-decisions.md) (row 2026-08-06, "Dialogue
claims only the TALKER's input by default").
⛔ **this line said `▢ the game-level default stays open` until 2026-08-07**,
three days after he answered it and directly above the note recording his answer.
⚠ that is worse than an ordinary stale line in THIS file: a `▢` here is what a
sweep looks for, so it invites re-raising a question the maintainer has already
ruled on — the exact failure this file's own preamble describes about rows 1–7.

**This is a product question, not a repair**, and it took a wrong turn to work
that out — so the mechanism is written down here rather than re-derived.

**What is true today.** `GameMode::{Paused, Dialogue, RoomTransition, Cutscene}`
all suspend gameplay, and the mechanism is not the input gate everyone reaches
for first: `apply_suspended_time_scale_system` runs under the
`gameplay_suspended` run condition and sets `ClockState::time_scale = 0.0`. The
world genuinely freezes — a suspended world still advances its TIMELINE (ticks
still count, so hashes and recorded input frames stay aligned) but moves zero
sim seconds.

⚠ I first concluded the opposite, because `allows_gameplay()`'s nineteen call
sites are almost all input routing and prompts, and the two run conditions built
on it look unused. They are not: `player_schedule.rs` uses `gameplay_suspended`,
and that one line is the whole pause.

**Why it is now a question.** Per-seat input contexts landed
(`SeatInputContexts`), so a surface CAN own one participant's input while
another keeps playing — and the test that proves the seam also pins that
`GameMode` overrides it (`dialogue_still_stops_the_world_as_well_as_claiming_
the_input`). On a couch, one player talking to an NPC currently freezes the
other player mid-jump.

**The decision:**

* **keep it** — dialogue is a world-stopping beat, and couch multiplayer
  tolerates the freeze because conversations are short. Costs nothing; the
  per-seat seam simply waits for a surface that is not world-stopping
  (inventory, a select screen).
* **split it** — `Dialogue` stops claiming to stop the world, and the talker's
  input is claimed by `DIALOGUE_CONTEXT` instead. Then NPCs keep patrolling and
  hazards keep ticking during a conversation, which is a real change to how
  every existing scene plays, not just a multiplayer fix.

⚠ **it is not a small mechanical change either way**, and the branch that looks
harmless is the wrong one: leaving `Dialogue` in the suspend set while adding
per-seat contexts gives a seam that silently does nothing, which is worse than
either answer.

⚠ **`RoomTransition` and `Cutscene` are NOT the same question.** Both are
genuinely global — a room is loading, or a scripted beat owns the screen — and
nothing about per-seat input makes them per-seat.

---

## Should the Hall cast and the content bosses declare dormancy? (2026-08-07)

Carried from `JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`, where it reads *"nobody has
decided, and silence currently reads the same as 'always awake' whether or not
that was chosen."* ⭐ **it is answerable now, because the two facts it turns on
are measured rather than assumed.**

### The facts

1. **The Hall is 129 `NpcSpawn`s and every one is `brain: stand_still`**
   (counted out of `hall_of_characters.ldtk`). So "should the Hall sleep" is a
   question about 129 NO-OP brain ticks per frame, not about 129 actors thinking.
   ✔ **re-counted 2026-08-07: 129 spawns, 129 `stand_still`, unchanged.** ⚠ the
   authored field is `brain_override`, not `brain` — a re-count that queries
   `brain` returns a clean ZERO and reads exactly like "this fact is now false".
2. ⭐ **dormancy would NOT silence them.** `Dormant` is filtered by
   `tick_actor_brains` (`actors/update.rs`, `Without<Dormant>`) and by nothing
   else — `tick_npc_idle_barks` carries no dormancy filter at all. A sleeping
   statue keeps its voice. That kills the obvious objection to sleeping a gallery.
   ✔ **re-verified 2026-08-07**: still zero `Dormant` references in that system.

### The fork

- **Hall cast** — `AwakeNearObservers` is SAFE (barks survive, the brain is a
  no-op, the bodies do not move) but saves only 129 no-op ticks. `Never` states
  the intent and saves nothing. ⚠ **neither is wrong and the difference is
  measurable**: if 129 StandStill ticks do not show in a frame profile, this is
  purely about whether silence should be allowed to mean a choice.
- **Content bosses** — `Never` is the answer that matches what they are. A boss
  that stops deciding because the player stepped back is a bug, and silence
  already means "always awake", so declaring it is BEHAVIOUR-PRESERVING and only
  converts an omission into a statement.

### Recommendation

Declare `Never` on the content bosses (free, states intent, cannot regress), and
leave the Hall until somebody has a frame profile that says 129 no-op brain ticks
are worth removing. ⛔ **do not declare `Never` on the Hall as a tidy-up** — that
would spend the decision without the measurement and make the later `AwakeNear
Observers` change look like a reversal rather than a finding.

⚠ what this does NOT decide: whether the dormancy policy should eventually take
SECONDS rather than pixels. That is recorded at the Sanic constant and needs a
third game, not this row.

---
## ✔ ANSWERED 2026-08-08 — Size the character quad from the BBOX instead of the FRAME?

> **Jon: *"I think the bbox route sounds ok."*** — TAKE IT. Moved to
> [`maintainer-decisions.md`](maintainer-decisions.md). ⚠ a softer yes than his
> capability ruling the same day; it authorises the route, not haste. The
> analysis below is kept because the FOUR coupled sites and the stretch
> measurement are what the work needs, not the fork.

### (the analysis that produced the answer)

Carries two of Jon's reports that turn out to be one decision: *"the Hall
characters are inconsistent sizes"* and *"the snake's sprite does not match its
box"*. ⭐ **the arithmetic makes them the same question**, and answering it
deletes an authored field rather than retuning it.

### The fact

`sprite_render_size_scaled`
(`ambition_sprite_sheet/src/character/sheets/geometry.rs:25`) is two lines, and
both take their number from the PADDED FRAME:

```rust
let height = collision.x.max(collision.y).max(8.0) * spec.collision_scale * visual_scale;
let width  = height * (spec.frame_width as f32 / spec.frame_height as f32);
```

- `collision_scale` — with `fill = body_pixel_bbox.h / frame_height`, the report's
  formula is `figure = collision_scale x fill`, so `collision_scale = target /
  fill`. **It is a reciprocal-of-padding fudge and nothing else.** That is why the
  116 authored values "do not compensate for anything": they are hand-tuned
  approximations of a computable quantity.
- the width line takes the FRAME's aspect. Its own comment says it "preserves the
  cropped frame's aspect ratio" — which is the defect stated as the intent.

⭐ **measured consequence**: 13 scientist sheets on an explicit `1.0` render at
figure height **0.84** while 33 high-fill sheets on the `1.5` default render at
**1.42** — two populations of people, in one Hall, **1.69x apart**.

### The fork

- **TAKE IT.** Size and crop the quad from `body_pixel_bbox`. `fill` becomes 1.0
  by construction, so every `collision_scale` collapses to ONE global constant
  (the target figure height), and the snake's quad matches its box for the same
  reason. ⚠ three coupled sites, and doing fewer is the stretched-sprite failure
  this repo has already paid for:
  1. `sheets/geometry.rs:25` — height off a constant, width off `bbox.w / bbox.h`;
  2. the character sprite must draw the bbox SUB-RECT rather than the whole frame
     (`ambition_render/src/rendering/actors/`);
  3. `feet_anchor_norm` (`character/sheets/mod.rs:476`, consumed at
     `rendering/actors/mod.rs:334`) is normalised against the FRAME and has to
     move with the crop.

### MEASURED 2026-08-08 — the coupling is real, and the site list was an undercount

Site 1 was done ALONE and photographed, precisely because "three coupled
changes" was an argument rather than an observation. It is an observation now.

**The art stretches, first try, in opposite directions on two characters** —
measured on the ink by colour-masked bounding box, against the prediction from
each sheet's `bbox / frame`:

| subject | x scale | y scale | aspect change | predicted |
|---|---|---|---|---|
| snake (117x52 in 128x128) | 0.898 | **0.409** | **2.20x** vertical squash | 0.914 / 0.406 |
| Mary-O (64x120 in 160x192) | **0.414** | 0.636 | **0.65x** horizontal squash | 0.400 / 0.625 |

Both match the arithmetic to measurement error, so the mechanism is certain
rather than suspected: `Sprite::custom_size` scales the WHOLE atlas frame into
the quad, per axis, so shrinking the quad to the bbox without cropping the
source divides the frame's padding into the body. The snake becomes a 9px green
smear and the box on its back a flat plank; Mary-O's arms retract into stubs.
⭐ **site 2 is the load-bearing one** — without the sub-rect, site 1 is not "a
quad that matches its box", it is a non-uniform scale applied to the art.

⚠ **and site 3 is misidentified for the characters that matter.** The snake and
Mary-O both take the `sprite_offset` branch (`rendering/actors/mod.rs:345`,
`:771`), which sets `Anchor::CENTER` and never reads `feet_anchor_norm` — that
is the fallback for sheets publishing no body. What actually broke placement was
`sprite_offset` itself, still computed at FRAME scale while the art drew at bbox
scale, floating the snake ~8px off the floor. Same defect class, different
field: a frame-normalised placement outliving the crop. Both must move.

### ⭐⭐ THE ROUTE **IS** THE REMAINING FIX — after a reading error that reversed it

⛔ **this heading used to read "AND THE ROUTE DOES NOT FIX JON'S ACTUAL
COMPLAINT"**, which was the wrong conclusion below. The body was corrected the
same day and the heading was not — so a reader scanning headings got the
opposite answer from a reader reading prose. Fixed 2026-08-08 when Jon accepted
the route.

`enemy_body_scale::print_enemy_bodies_against_the_player`, run for the first time
since it was written:

```text
           target     collision        render   x_vs_p   y_vs_p
  player_robot_v3     57x91        224x224       1.00x    1.00x
      solid_snake    117x52        128x128       2.05x    0.57x
          ai_slop    257x167       271x232       4.51x    1.84x
        mary_o_v2     64x120       160x192       1.12x    1.32x
```

⛔⛔ **I FIRST READ THIS TABLE WRONG, AND THE CORRECTION REVERSES THE CONCLUSION.**
I reported the slop's body as *"4.51x the player's width"* and concluded this
decision would not fix Jon's complaint. **That comparison was invalid**: the
`collision` column is in SHEET PIXELS, and these sheets have completely different
pixel densities, so a cross-sheet ratio of sheet pixels means nothing. Comparing
in WORLD units, which is what the game and the player actually see:

| body | world size | vs Mary-O's width |
|---|---|---|
| Mary-O | 25.6 x 48.0 | 1.00x |
| AI slop | 28.0 x 18.2 | **1.09x** |
| snake | 25.6 x 11.4 | **1.00x** |

**The bodies are already the right size.** `AI_SLOP_BODY_WIDTH = 28.0` is *"the
one authored number; its height follows from the art"*, and `snake_body_width()`
returns `mary_o_body_width()` — derived from her, *"which is what a Koopa does
beside Mario."* Both landed **2026-08-06**, and the snake's own doc names Jon's
report as what it fixed: *"before this it was 41.0 world units against her 25.6 —
1.6x her width … that is the 'way too big' Jon reported twice."*

### ⭐⭐ THE "DATA BLOCKER" DOES NOT APPLY TO THIS ROUTE — measured 2026-08-08

The observations row records *"the blocker is data: 2 of 190 sheets declare
`authored_body`"*. **That count is correct and it is about the wrong field.**

```text
  sheets in assets/sprites/     190
  declaring authored_body         2   (player_robot_v3, vera_ruin)
  declaring body_pixel_bbox     184   ← 97%
```

`authored_body` gates `authored_body_pixel_size`, which sizes the **collision
box**. **This decision sizes the QUAD, and the quad comes from
`body_pixel_bbox`** — which 184 of 190 sheets already publish, because the
renderer measures the alpha bbox on every regeneration. **So the fork is not
data-blocked.** The six without one are the population to look at, not the 188.

⛔ **and finding this needed a different search, which is a tooling trap worth
carrying**: `grep -r` in this repo **silently skips gitignored files**, and
sprite sheets are gitignored (`.gitignore:110`). A recursive grep for
`body_pixel_bbox` returns **0** across 190 files that contain it. `--no-ignore-files`
fixes it, or `find … | xargs grep`. ⚠ this is the sibling of the known
symlink trap (`grep -r` needs `-S` for symlinked assets) — **two independent
reasons a recursive grep silently misses this repo's art.**

⭐ **and the picture agrees, checked 2026-08-08 against the broken-capture
scare.** `capture_mary_o` composes through `build_windowed_demo_app_entering`
(named `…_with_home` when this was written; it took the entry room on 2026-08-09),
NOT the `capture_scene` path that spent two days photographing a void — and the
snake capture shows full room geometry (floor tiles, both ?-blocks, the spike
platform, the backdrop), so the D3 evidence stands.
⭐⭐ **reading that image gives a second, independent measurement**: the AI slop's
SPRITE stands roughly Mary-O's height while its BODY is 18.2 against her 48 —
about **2.6x picture-over-body**, matching the snake's measured 2.46x. **So the
quad/box gap is general across the cast, not a snake peculiarity**, which is the
strongest argument yet that this fork is worth taking.

⭐⭐ **so the SIZE half is already done, and what remains is exactly this
decision.** The bodies are right; the PICTURE is 2.46x the body. That residue is
the quad/box disagreement and nothing else — **taking this fork is the fix for
"way too big visually", not a separate concern from it.**

⚠ context, not an argument for a rule: the catalog's 136 characters span
**14.58x** in body height (`perfect_cellular_automaton` 452px,
`npc_puppy_slug` 31px). Most of that is creatures, which the humanoid pass
explicitly excludes.

⛔ **THE OTHER CORRECTION: there are TWO independent render-size publishers, and
this row names only one.**

* `posed_body_geometry` → `ActorRenderSize` → `ActorRenderIndex.render_size` —
  the sheet-authored path. **This is what the snake and Mary-O use.**
* `sprite_render_size_scaled` → `build_character_sprite(asset, collision)` — the
  `collision_scale` path at `geometry.rs:25`, which is the one this row names.

They sit side by side in one match arm (`rendering/actors/mod.rs:772` and
`:780`), and the independent proof is the AI Slop: in the same captured frame it
measured **0.99x / 1.02x — completely unmoved** by the change, despite its sheet
publishing a 257x167 body in a 271x232 frame, because `tag_mary_o_ai_slop`
derives only its collision and never attaches `SpritePosedBody`.

**So TAKE IT costs four sites, not three**, and doing only the one named here
would fix the Hall's 1.69x spread while leaving the snake and Mary-O — the two
characters Jon actually complained about — on the old arithmetic. That is the
opposite of the intended outcome, and it is what a careful reading of this row
would have produced.
- **LEAVE IT** and apply `--suggest` per character instead. Cheaper today, but it
  spends a 116-row humanoid judgement on values the first branch would delete.

### Recommendation

⚠ **decide this BEFORE the humanoid pass**, whichever way it goes. The two are
not independent: `--suggest` retunes a field that "take it" removes. If it is
taken, the only judgement left is the handful of creatures whose figure should
not match a human's — slug, snakes, parrot, trex, shark, mites — which is a far
smaller ask than 116 rows.

⚠ what neither branch decides: what the target figure height should BE. 1.21 is
Jon's own Alice/Bob reference and reproduces Alice's authored 1.5 from her 81%
fill, which is evidence it is the right constant, not proof.

---
## Which of the 33 engine design documents have become history? (2026-08-01)

⛔⛔ **THE LINE BUDGET IS NOT JON'S, AND THIS ROW HAD IT BACKWARDS.**

> **Jon, 2026-08-08, verbatim:** *"I never set the budget for doc's planning. An
> agent picked that because I said that it was getting too big. I actually
> changed it from a hard budget to a soft budget so we could continue to expand
> it because we have a lot of ambition through this game and we need to plan. But
> I do think some of the reporting in the docs gets stale and doesn't get cleaned
> up, but that's not the priority right now."*

So `PLANNING_TOTAL_SOFT_BUDGET = 10_500` in `scripts/check_agent_kb.py` is **an
agent's number**, and the script already says what Jon says: *"Size is NEVER a
build-failing gate here … a human decides if a plan actually needs pruning."*
⭐ **Jon deliberately downgraded it from hard to soft so planning could GROW.**

⚠ **so "31,844 against 10,500" was a framing error, repeated by me today.** An
invented threshold got cited as a target, and a doc tree doing its job read as a
violation. ⭐ **the real question is STALENESS, not size** — reporting that
accumulates and never gets cleaned up — **and Jon has said it is not the priority
right now.** Leave the size number out of it.

```text
  docs/planning/*.md   11,508   the queues and live plans
  docs/planning/engine  15,257   33 design documents   ← the budget
  docs/planning/triage   2,713
  docs/planning/demos      899
  docs/planning/game       231
```

The rule that has worked twice — *"a section with no `▢` rows is closed, move it
verbatim to the archive"* — applies to LEDGERS. Applied to the whole backlog
queue it frees 251 lines; it freed 46 from the 72h queue today. Together that is
about **1% of the overage**.

`engine/` has no `▢` rows because those files were never ledgers. Archiving one
means judging that a design is built and stable enough to stop being read, which
is a judgement about the engine's own record rather than a checkable property —
and losing design rationale is the kind of loss nobody notices until they need
it.

**The decision:** which of the 33 are history. A plausible first pass, for
someone who knows what is still live:

* the largest are `immutable-content-and-transactional-construction.md` (2,406),
  `competitive-2d-platformer-engine-roadmap.md` (1,567), `room-transition-loading.md`
  (930), `fighter-brain.md` (873) and `api-1.0-campaign.md` (868);
* `api-1.0-campaign.md` describes a campaign the ledger records as CLOSED, which
  makes it the most likely candidate and still not one an agent should decide.

⚠ **do not resolve this by trimming live plans.** That is what the budget's own
message warns against, and it is what the pressure produces when the only
reachable lines are the ones still being used.

### ⭐ MEASURED 2026-08-08 — the judgement has a defensible proxy

It is **36 docs and 17,020 lines** now, not 33 and 15,257. Three cheap signals
turn "is this history?" into a shortlist, and none of them is taste:
**inbound references** (how many other docs point at it), **last touched**, and
**open marks**.

⭐ **inbound count is the load-bearing one.** A doc nothing points at can be
archived without breaking a reader's path; a doc thirty things point at is
structure, whatever its age.

**Archive candidates — few inbound, untouched since July, nothing open:**

| doc | lines | inbound | last touched |
|---|---:|---:|---|
| `presentation-and-shell-audit.md` | 237 | **1** | 2026-07-27 |
| `sprite-renderer.md` | 174 | **1** | 2026-07-19 |
| `character-bone-rig-conversion.md` | 168 | **1** | 2026-07-24 |
| `shell-vanity-sequence.md` | 217 | **2** | 2026-07-19 |
| `svg-component-character-migration.md` | 375 | 3 | 2026-07-23 |

That is ~1,171 lines on five judgements, each of which is *"is this design still
being read?"* rather than *"is the engine done with it?"*

⛔ **NOT archive candidates, despite being the largest** — these are what the
"biggest files first" instinct would have picked:
`immutable-content-and-transactional-construction.md` (2,406 lines but **12**
inbound), `architecture.md` (218 lines and **30** inbound — the directory's
hub), `decomposition.md` (187 lines, **21** inbound). Size and liveness are
uncorrelated here.

⚠ **and `api-1.0-campaign.md`, the row's own "most likely candidate", has 9
inbound references and one open mark.** It is cited as structure by nine other
documents; archiving it on the grounds that its campaign closed would break all
nine. The row's guess was reasonable and the measurement does not support it.

### ⛔ `triage/` is NOT the easy 2,712 lines it looks like

Ten files, 2,712 lines, **zero open marks in any of them**, and five with **zero
inbound references**. By line count that reads as the cheapest archive in the
directory. It is not, and the reason is the failure this README already names:
*"six sections of `tracks.md` were open assignments with no mark at all."*
**Zero marks is not evidence of closure when a file never used marks.**

Sampled the largest zero-inbound one, `bevy-system-parameter-architecture.md`
(566 lines). Its own header: *"State: TRIAGE — PROPOSED DIRECTION"*, and
*"Promote bounded slices into `../tracks.md` only when they can name the systems,
invariants, performance measurements, and deletion target owned by that slice."*
It carries a **six-card migration plan**. Zero inbound references means **nobody
ever promoted a card** — that is unstarted design direction, not history.

⭐⭐ **and its thesis is being followed WITHOUT it.** `conversation/opening.rs`
groups its dependencies into one `SystemParam` and explains why: *"the interact
system is already at Bevy's parameter ceiling — a signal that a system reaching
for this many worlds should name its sub-worlds."* That is this document's title
(*"name the seams, do not pack the ceiling"*) arrived at independently, in a
2026-08-08 carve, with no citation. **The direction is live; only the document is
orphaned.**

⚠ so the decision for `triage/` is a different question from the one this row
poses about `engine/`: not *"is this design history?"* but *"was this
investigation ever finished, and did anything absorb its conclusion?"* A
zero-inbound triage file is as likely to be dropped work as completed work, and
telling them apart needs reading, not counting.

⚠ only **5 of the 36** carry any open mark at all
(`competitive-2d-platformer-engine-roadmap` 4, `participant-action-system` 2,
`api-1.0-campaign` 1, `character-definition-design` 1), so "has open work" does
not discriminate — which is exactly why this needed a different signal.

⊘ **not proposed: raising the budget.** A soft ceiling nobody can meet stops
being read, but so does one that is raised whenever it is inconvenient. It should
move because somebody decided what belongs in `docs/planning`, not to silence a
warning.

### ⭐⭐ EVIDENCE ADDED 2026-08-07 — nine of them were read, and the answer is not
### "archive the big ones"

Queue lane C4 swept every `engine/` doc carrying an open-work section heading
(`residual` / `debt` / `open question` / `remaining` / `not done` / `gap`) —
eleven of them, nine cited nowhere in the active ledger — and read all nine.

**Eight had nothing open.** Not "mostly done": their open-work sections described
a tree that no longer exists. `room-transition-loading.md`'s section headed
*"Current source-backed gap"* was false in every clause; `character-definition-design.md`'s
"Attack intent gaps (block Smash)" was 3-of-4 closed and Smash has shipped;
`participant-input.md`'s "deliberately not done" list was 5-of-8 landed;
`unified-movement-kernel.md`'s "Residual debt (honest)" was 4-of-5 paid.

⭐ **so "which are history" is the wrong cut, and the sweep says why.** These files
are not history — their DESIGN ARGUMENTS are live and were load-bearing today
(three of them explained why the current code is shaped as it is, and one supplied
the fix for a defect). What has gone stale is a SECTION KIND: the status/gap/debt
lists inside them. Archiving `room-transition-loading.md` would throw away the
argument that room transitions must not fabricate shell routes; leaving it
untouched keeps a paragraph that reads as a current measurement and is not.

⭐⭐ **a cheaper cut is now available**: strike the stale gap sections in place
(as C4 did for nine files, ~200 lines annotated rather than deleted) and archive
only files whose ARGUMENT is spent. That is a different and much smaller list than
"the largest five", and `api-1.0-campaign.md` — the note above's most likely
candidate — is still the best one on that list, because a closed campaign's
argument IS spent.

⚠ **and the budget arithmetic barely moves either way.** `engine/` measures
**16,111 lines today** (up from 15,257 when this row was written, because the
sweep ADDED annotations). Archiving the largest file frees 2,406 — about 8% of the
overage. The decision is worth making for legibility, not for the warning.

⛔ **one thing the sweep settles outright**: it is no longer true that nobody
knows what is live in there. Nine are read and annotated, with each finding dated
and file:line-backed. The remaining question is genuinely a judgement about
argument value, which is what this file is for.

---

## Does `apple_rain`'s damageable box follow the head row when no sprite sample exists? (2026-08-01)

**In your terms:** *during GNU-ton's apple-rain attack, can you hurt it by
hitting its body, or only its head?* The art says head — the sheet author drew a
descending head hurtbox for exactly those 9 frames. The code currently falls back
to the whole body when it has no sprite to sample, which is what happens in
headless tests and for the first frames before sprites load. Honour the art, or
keep the larger box a live boss has always had.

One line of content blocks the `BossAnim`→`CharacterAnim` fold's first slice.
Pinned by `apple_rain_claims_no_animation_rows_which_is_why_the_fold_is_blocked`
(`ambition_content`), which fails on either answer so neither happens silently.

`apple_rain` is a `Special`, so its animation rows come from
`ambition_content`'s `special_animation_keys()` map — and it is not in it. Its
profile therefore claims **no rows**, while the sample writer emits `"head_down"`
for it, with a comment saying that is deliberate: *"GNU-ton's apple rain reads the
head row for its damageable hurtbox."*

So the intent is written down and the catalog does not carry it. Today the row is
found anyway, through a circular path: `runtime_animation_keys` pushes the
sample's OWN key into the list whenever the sample's profile matched. `apple_rain`
works *because of* the check the fold wants to remove.

**The decision:**

* **add `("apple_rain", vec!["head_down"])` to `special_animation_keys()`** — the
  catalog then says what the code already believes, every identity agrees as a
  set, and the fold becomes a rename.
  ⚠ **it is not purely cosmetic**: with no sprite sample (headless tests, and the
  frames before sprites upgrade) `apple_rain` currently falls back to the body
  bbox and would instead sample the authored `head_down` hurtbox. That is a live
  boss's damageable shape changing on that path.
* **keep the profile identity** — accept that `BossAnimationFrameSample` carries
  two facts that legitimately differ (which attack drives, which row renders),
  and fold only the row VOCABULARY (slice 2), leaving `BossAttackProfile` in the
  gameplay geometry path.

⊘ **not proposed: adding the row and calling it a no-op.** The fallback path is
where headless tests live, and "it only changes the case with no sprites" is the
same sentence as "it only changes what every test sees".

### ⭐ How big is the shape change? MEASURED 2026-08-07 (the question was
### answerable all along; nobody had read the sheet)

The entry above says "a live boss's damageable shape changing" without saying by
how much, which is the part that decides it. From
`sprites_0_25x/gnu_ton_boss/giant_gnu_spritesheet.ron` (`giant_gnu`, frame
**192 × 144** at this scale):

* the authored `head_down` hurtbox is **one part, `head`, 42 × 33**, at `x: 75`,
  with `y` descending **36 → 63** across its **9 frames** — the head coming down.
* `rest`'s hurtbox is **the same 42 × 33 head part**, static at `y ≈ 36`. So
  `head_down` is not a different-sized box; it is the same box, animated.
* 42 × 33 is **≈ 5% of the frame area** (1 386 of 27 648 px²), sitting top-centre.

⭐ **so the decision reads: on the no-sample path, `apple_rain`'s damageable
region goes from the whole body box to a head-sized box covering about a
twentieth of the frame, which descends during the attack.** That is a real
change, not a rounding one — a player (or a headless test) hitting GNU-ton's
flank mid-apple-rain would stop connecting.

⚠ **it also cuts the other way, and that is why this is still yours:** the
authored intent is unambiguous — the sheet author drew a descending head hurtbox
for exactly these 9 frames, and `SpikeHalo`'s row is 9 frames too. The body-box
fallback is what nobody chose; it is what happens when the catalog says nothing.
So option 1 is "honour the art", and option 2 is "keep the larger box a live
boss has always had". Both are defensible and the numbers do not pick.

⛔ **and a measurement footgun that nearly buried this**: `grep -r` over the
asset trees in a write-ahead worktree finds NOTHING, because the sheets are
symlinks and recursive grep does not follow them without `-S`. A first pass here
concluded `head_down` "is not authored anywhere", which is the opposite of true.

## ✔ IMPLEMENTED — Is a crate name part of the rollback wire format? (queue S30, 2026-08-01)

> ### ✔✔ SHIPPED 2026-08-09 AS **(b′)**, SCHEMA **v20** — `3333a4b0f`. Jon can still overrule.
>
> **No: the fingerprint now hashes a type's FINAL SEGMENT alone**, so relocating
> a rollback-registered type between crates or modules no longer moves
> `SnapshotSchemaFingerprint`. Every carve in the decomposition campaign stops
> being a netplay compatibility break — which is what this question blocked.
>
> ⛔ **the answer below is (b) and (b) IS WRONG.** It says *"final segment plus
> its module path below the crate"*. Refuted 2026-08-09 by the very diff this
> file cites as evidence: D33 step 2 moved `features::ecs::actor_clusters` →
> `character::anim` and `avatar::components` → `camera_ease`, so **the
> below-crate path moved too** and (b) would have changed the fingerprint on both
> of the moves it was proposed to protect. Only the final segment survived.
> ⭐ **and (b′) was already written in this file** — the section *"A third
> option, which dissolves the trade"* says *"hash the type's IDENTITY without its
> PATH"*, eighty lines before "The fork" offers only (a) and (b). The answer was
> in the document the whole time.
>
> ⚠ **the same-commit companion was NOT optional and NOT free**: once the final
> segment is the identity, two same-named types in different crates hash equal,
> so a duplicate-identity guard landed with it. ⛔ **this file's claim that that
> "leans on existing behaviour" is also false** — `RollbackRegistry`'s duplicate
> **name** check cannot see two `Cooldown`s arriving under two *different* stable
> names, which is exactly the legitimate-looking case.
>
> ⇒ **this row is CLOSED and needs nothing from Jon.** It stays here, marked,
> rather than moving to `maintainer-decisions.md`, because **that file records
> decisions JON made explicitly** and this one was taken by an agent under his
> delegation. Where it lives is not a filing detail — moving it would claim his
> authority for it. Full record: queue **D37**, ADR 0027 (corrected in the same
> change; it still claimed owners were hashed, six weeks after v5 removed them).

> ### ⚠ THE ORIGINAL AGENT ANSWER, 2026-08-08 — **(b)**, superseded above.
>
> Taken under Jon's 2026-08-07 note *"I've left rollback design to agents"*, and
> recorded here rather than only in a commit because it changes a
> **determinism-critical hash**. Implementation and the mandatory same-commit
> probe are queue row **D37**; it waits on D33 step 2/3, which touches the same
> baseline.
>
> ⭐ **What settled it was a measurement, not the argument below.** While this row
> was being read, D33 step 2 moved two rollback-registered components across
> crates and `rollback_schema_baseline.txt` changed exactly two lines — stable
> names `actor.anim_override` and `player.blink_camera_state` **identical**,
> schema still v19, **only the `type_name` column moved**. Nothing a peer can
> observe changed, and under (a) that is a compatibility break. The row predicted
> this exact diff; the campaign then produced it.
>
> The deciding precedent is one level down and already shipped: **v5 stopped
> hashing the registration owner** because *"which module registered this"* is not
> a wire-format fact. `type_name`'s module path is the same category of
> organisational label — and unlike the owner, nobody chose to hash it, it
> arrived inside a string that was being used for identity.
>
> ⛔ **not optional, and it lands in the same commit**: two types with the same
> final segment in different crates must be **rejected loudly**, not silently
> merged.

**In your terms:** *if we rename a crate, do saved replays and netplay against
anyone on the old build break?* Rollback identifies each piece of saved state by
a string that currently contains the crate's name, so renaming a crate silently
changes the wire format. The decision is whether that is fine (rename freely,
bump the schema version, old replays are dead) or whether the name and the wire
identity should be decoupled so renames are free.

**Newly blocking.** This sat as a queue row for days without costing anything.
It now blocks a specific next step, which is why it is here.

### The fact

`descriptor()` stores `std::any::type_name::<T>()`, `schema_dump()` writes it,
and `schema_fingerprint()` hashes the dump. So **moving a type between crates
changes `SnapshotSchemaFingerprint`**, and two peers with byte-identical wire
formats refuse to agree. Measured during the rename: **94 of the 352 lines** in
`rollback_schema_baseline.txt` name a crate.

⚠ this is the same defect v5 fixed one level down. `registry.rs:33` says it in
its own words — hashing the registration OWNER "made 'which module registered
this' a wire-format fact", so v5 stopped hashing it. The crate path inside
`type_name` is the same category of organisational label, and it is still hashed.

### What it blocks now

The `ambition_geometry` carve (done today) took shapes, boxes, reference frames
and swing/combat volumes out of the platformer core, and `ambition_vfx` stopped
declaring a platformer dependency it never had. The census says the **snapshot
codec** — `snapshot::{Reader, SnapshotState, SnapshotCursor, put_*}`, 347 lines,
wanted by `ambition_time` and `ambition_sprite_sheet` — is the next equally
general thing stuck in there.

⛔ **but moving it rewrites the fingerprint**, which is why the geometry carve
went first and this one has not started. Deciding this unblocks it.

### ⭐⭐ RE-VERIFIED 2026-08-07, and it is worse than "a crate name"

Checked to the definitions rather than restated: `schema_dump()` writes
`entry.name`, `entry.kind.canonical_name()`, **`entry.type_name`** and
`entry.detail` — pointedly NOT `entry.owner`, which is v5's fix still holding —
and `schema_fingerprint()` hashes that whole dump (`registry.rs:323-330`).

⛔ **`type_name` carries the MODULE path, not just the crate.** Today's own work
is the proof: registering `ActiveConversation` put this line in the baseline —

```
resource.active_conversation	resource-clone	ambition_platformer2d_actor_monolith::conversation::authority::ActiveConversation	…
```

`::conversation::authority::` is a module layout decision from this morning, and
it is now inside the wire-format hash. So the sensitivity is not "moving a type
between crates" — it is **moving a type between modules**, which is an ordinary
refactor this repo does weekly.

⚠ **and that collides directly with Jon's decomposition instruction** (2026-08-07:
*"if we add things to the monolith, try to do it so it's obvious what the
decomposition should be. we will need to address that bloat in the coming
days."*). Carving the monolith moves rollback-registered types between modules and
crates by definition, so every step of that campaign rewrites the fingerprint and
declares two identical builds incompatible. This row does not block one carve; it
taxes the whole decomposition.

⭐ **the argument the row already makes now has a second instance.** v5 stopped
hashing the owner because *"which module registered this"* is not a wire-format
fact. `type_name`'s module path is the same category of organisational label
wearing a different hat — and unlike the owner, nobody chose to hash it; it came
along inside a string that was being used for identity.

### ⭐ A third option, which dissolves the trade rather than picking a side

What does `type_name` actually BUY the fingerprint? Exactly one thing the other
hashed fields do not: catching **the same stable name bound to a different type
across two peers**. ⚠ within a single build that is already impossible —
`registry.rs:181` raises *"conflicting rollback registration '{name}'"* and
`:365` panics on it — so the value is strictly cross-peer.

⇒ **hash the type's IDENTITY without its PATH.** The last segment
(`ActiveConversation`) or a stable type id keeps the cross-peer check exactly as
strong, because the pair `(stable name, type basename)` is what actually differs
in the case being guarded against — while a module move, a crate carve and the
whole decomposition campaign become free.

⚠ the honest weakness: two types with the SAME basename in different modules,
bound to the same stable name on two peers, would slip through. That requires a
name collision on both the schema name and the type basename simultaneously, and
the schema names are hand-authored and unique. ⭐ this is strictly stronger than
hashing nothing, and strictly cheaper than hashing the path — which is the shape
of an answer rather than a preference.

⛔ **and it needs a `GGRS_ROLLBACK_SCHEMA_VERSION` bump**, exactly as v5 did when
it stopped hashing the owner. The precedent for this change is the change itself,
one level down.

### ⭐ Two things measured since, both of which enlarge this decision

**1. The codec blocks carve-outs by a SECOND mechanism nobody had named: the
orphan rule.** Found 2026-08-02 while carving `ambition_projectile_spec`.
`ambition_platformer2d_shared_tangle` contains

```rust
snapshot_unit_enum!(crate::projectile::WorldHitPolicy { Bouncing = 0, … });
```

— an impl of **core's** `SnapshotState` for its own type. Move that type into a
new crate and the impl becomes a foreign trait on a foreign type and stops
compiling; move the impl with it and the new crate must depend on core for
`SnapshotState`, reintroducing exactly the edge the carve existed to remove.
`ProjectileSpec` is dragged along by its `world_hit` field.

So wherever the codec lives, it anchors every type anyone implements its traits
on. That is independent of the fingerprint, applies to **66 encoded types across
9 crates**, and (b) does not fix it — only moving the codec out of core does.

**2. `ambition_input` is the single highest-value cut in the workspace, and it is
this decision.** `scripts/core_import_census.py --cuts` simulates dropping one
direct edge and reports who leaves the closure:

```text
 3 crate(s)  ambition_input        input, inventory_ui, ui_nav
 3 crate(s)  ambition_characters   characters, content_cli, interaction
 1 crate(s)  ambition_time         time
 1 crate(s)  ambition_platformer2d_shared_tangle
 0 crate(s)  …everything else
```

`ambition_input` imports **exactly one item** from core — `ControlFrame` — and
`ControlFrame` is line 121 of `rollback_schema_baseline.txt`
(`derived.control_frame`). `ambition_characters`, the only cut of equal value,
imports 77. So the cheapest three-crate cut available anywhere in the workspace
is gated on this fork.

⚠ **and be careful reading that table the way I first did.** The `--paths`
column made it look like SEVEN crates reached core only through `ambition_input`;
simulating the cut says three. A shortest path hides every alternative path by
construction. Rank by `--cuts`, never by `--paths`.

**3. Checked the other three cuts, and THREE OF THE FOUR are this decision.**
`ambition_time` takes the snapshot codec and nothing else. `shared_tangle` is
already platformer-named. And `ambition_characters` — the only cut of equal value
— turns out not to be a carve at all: of its 78 imports, 11 already live in
`ambition_geometry`, 3 are the codec, and **20 are genuinely platformer**
(`AxisJumpLaw`, `MomentumParams`, `AIR_ACCEL`, `DASH_SPEED`, `SLASH_RECOIL`,
`AbilitySet`, `MovementTuning`). A crate whose brains reason about jump laws and
air acceleration is a platformer crate with an unqualified name; that is a
renaming question for you, not a carve-out.

So there is no remaining carve-out work that this fork does not gate.

### The fork

**(a) Accept it.** Regenerate the baseline and bump
`GGRS_ROLLBACK_SCHEMA_VERSION` whenever a type moves. Honest, one commit, and
concedes that reorganising crates is forever a compatibility break.

**(b) Hash below the crate.** Fingerprint the type's final segment plus its
module path *below* the crate, so the fingerprint stops caring where a type is
packaged — what v5 already decided for owners, applied consistently.
⚠ two same-named types in different crates then collide, so `RollbackRegistry`'s
duplicate-name check has to stay the thing that catches it. It already rejects
duplicate `name`s, so this is a dependency on existing behaviour, not new work.

The baseline regenerates either way. **Only (b) makes the next move free.**

### It already happened, today

The `ambition_geometry` carve broke the schema baseline, and it took four
commits to notice because `cargo check --all-targets` compiles tests without
running them. The entire diff:

```text
- actor.centered_aabb  ambition_platformer2d_core::geometry::CenteredAabb
+ actor.centered_aabb  ambition_geometry::geometry::CenteredAabb
```

Same type, same projection, same registration, same bytes on the wire. Only the
crate path inside `type_name` moved. Under (a) that is a compatibility break
every peer must be told about; under (b) it is invisible, which is what it
deserves to be, because nothing a peer can observe changed.

⚠ note also how it was caught: not by a guard, but by going looking for
something else. Under (a) the cost of a carve is not just the version bump — it
is that every carve needs somebody to remember to run one specific test.

### Recommendation

**(b)**, for one reason beyond consistency: the engine is at the start of a
carve campaign, not the end. The census still shows 24 crates on the platformer
core and names three more general things inside it. Under (a) every one of those
is a wire-format break, which prices the reorganisation in exactly the currency
that stops it happening.

⚠ against (b): it is a real behaviour change to a determinism-critical hash, and
it wants its own probe — two types with the same final segment in different
crates must be REJECTED loudly, not silently merged. That check is the work item
attached to choosing (b), and it should land in the same commit.

## ⊘ DEFERRED BY JON 2026-08-03 — the top four ladder rungs ship a knob that makes them worse

> **He answered this and the row stayed on the pending index anyway**, so it read
> as owed for five days. *"Not sure about balance, we are most concerned with
> architecture right now."* Removed from the index 2026-08-08. The measurement
> below stands and needs no re-deriving when balance comes up.

> **JON, 2026-08-03: "not sure about balance, we are most concerned with
> architecture right now."**

◐ **Deferred, deliberately — not dropped.** The measurement stands and needs no
re-deriving when balance comes up; do not spend run time re-measuring it.

**Newly surfaced**, and it is a balance call rather than a bug — but it has sat
inside a probe's header comment where nobody was going to weigh it.

### The fact

`profile.rs` ships, since `ed6c55d0e` (2026-07-31 02:23):

```rust
rollout_depth: if level >= 6 { 12 } else { 0 },
rollout_k:     if level >= 6 { 4  } else { 0 },
```

So levels 6–9 — the rungs meant to be HARDEST — run L3 lookahead, and levels 1–5
do not. Every measurement of that knob since it shipped says it makes the fighter
kill itself sooner:

```text
2026-07-31 morning  9/d0  5.0s to first self-KO   9/d12  9.8s    ← d12 better
2026-07-31 evening  9/d0  5.2s                    9/d12  2.7s    ← d12 WORSE
  (three seeds, identical on every seed)
2026-08-02          9/d0  no self-KO in 60s       9/d12  4.8s    ← d12 WORSE
                          0 stocks lost                  1 stock lost
```

The morning run is the outlier and the probe's own header says why it should not
be compared: *"the morning's numbers are not comparable to the evening's."* Two
independent runs, four seeds, agree that the knob costs survival.

### What the measurement does NOT cover

⚠ **the probe's opponent cannot attack.** Its header says so — *"a human seat
with no controller… every stock lost is a self-KO"*. So this measures
self-preservation and nothing else. Rollout could plausibly buy offence and pay
for it in walking off ledges, and **nothing has measured the offence half**.
`fighter-brain.md` §12.6 already records exactly this: the first measurement
*"unblocks authoring a nonzero `rollout_depth` on a survival claim but not on an
attack one"*. The nonzero depth was then authored anyway, and the survival claim
it rests on has since inverted.

### The fork

* **Revert to `rollout_depth: 0` everywhere** until `l3_earns_its_depth` (still
  owed, FB6e) measures an attacking opponent. Restores the state the docs
  describe, at the cost of the top rungs losing their only distinguishing
  mechanic.
* **Keep it and treat self-KO as acceptable** for high rungs — a fighter that
  commits harder and sometimes overshoots may be the more interesting opponent,
  and the probe cannot see that.
* **Keep it and fix the recovery** — and this option changed the same day it was
  written, because the recovery failure turned out to be MEASURED and GEOMETRIC
  rather than a momentum problem.

  ⭐ **the fighter loses its stocks in a 2.5-second limit cycle against the
  platform's side face.** From `ladder_probe`'s unclaimed-velocity detector, 91
  events at one frozen `x`:

  ```text
  fall off the left edge  →  jump (vel_y = -520 = JUMP_SPEED)
                          →  dash RIGHT at 760 to recover
                          →  dash killed instantly, x pinned at 101.84
                          →  fall back, repeat every 150 ticks
  ```

  The platform is `x 110..530`, `y 300..332`. The body sits 8.16 px left of its
  left edge at the platform's own height — **beside the wall, not under the
  lip** — and a horizontal dash cannot climb 32 px of platform. `chose=Some(Recover)`
  appears in the seam on those ticks, so the brain IS trying; it is aiming its
  recovery at a wall.

  ⚠ that makes this option concrete rather than speculative: the fix is a
  recovery that gains HEIGHT (a jump-then-drift, or a dash whose direction rises
  toward the lip), not a tuning change. And it means the survival numbers above
  may be measuring stage geometry as much as lookahead depth — a fighter that
  cannot recover at all will self-KO regardless of what its rollout decides.
  ✔ **SETTLED 2026-08-07 — the first explanation, structurally.** The 8.16 offset
  implies a body half-width nobody authored, and for these characters nobody
  authors one: a catalog entry carries **no `body:` field at all**, only
  `body_kind` and an optional `sprite_tuning: (collision_scale,
  frame_sample_inset)`. So `prepared.body` is `None`, `PhysicalBaseline::of`
  yields `explicit_size: None`, and the box is derived from the SHEET. That is the
  same policy `BodySource::SpriteAuthored` names explicitly, whose own doc says it
  *"is not a size, it is a policy, and its authority is the per-pose
  projection"* — which is exactly why searching for the number finds nothing.
  ⚠ **the arithmetic is not claimed**: 8.16 as `world_per_pixel × pixel_half_width`
  would need the derivation run against that character's sheet. What is settled is
  that an UNAUTHORED half-width is the expected state here, not a symptom — so it
  is no longer evidence for "the stop may be a different contact".

⚠ whichever way: the three prose claims that said `rollout_depth` was zero
everywhere are corrected as of 2026-08-02, so the code and its description now
agree. This decision is about the value, not about the drift.

---

## ⊘ 9. ANSWERED BY JON 2026-08-03; architecture LANDED 2026-08-07 — can a flying fighter shield?

> **He answered the engine question and the row stayed on the pending index**,
> so it read as owed. *"Not in smash. In other games it's up to the game. I'm not
> sure about ambition itself yet."* `SmashCfg::shield_requires_ground` now carries
> the rule and defaults to `true`, so **Ambition's answer is an AUTHORING act, not
> a code change** — the duel fixture states which rule it fights under. ⚠ it went
> live rather than hypothetical on 2026-08-08 when he ruled the PCA can fly.
> Removed from the index 2026-08-08.

⚠ **NO LONGER BLOCKING ANY TEST (2026-08-03).** The two duel tests this was filed
against now pass — `app_it` is 287/0 — and the fix was in ranged AIM, not shields:
every ranged moveset move was firing world-right regardless of facing, so in a
duel one fighter shot away from the other all match. **The shield measurement in
this item is still accurate and still a real design question**; it simply was not
the cause. Answer it on its merits, not under pressure.

> **JON, 2026-08-03: "not in smash. In other games it's up to the game. I'm not
> sure about ambition itself yet."**

⭐ **So the RULE is right and its HOME is wrong.** `cfg.can_shield &&
obs.self_on_ground` encodes Smash's answer correctly — a Smash flier does not
shield — but it encodes it in the BRAIN, where every game inherits Smash's call.
Jon's answer says this is a per-game policy with three states, and Ambition's is
still open.

✔ **DONE 2026-08-07 — the architecture item, which did not need Ambition's
answer.** `SmashCfg::shield_requires_ground` now carries the rule, and the two
sites that hardcoded `obs.self_on_ground` — the reactive-block arm and the hold
that follows it — read the policy. It defaults to `true` in every `SmashCfg`
constant, so nothing changed by accident: `ambition_characters` 468,
`ambition_demo_smash` 50, contracts 25/25, app gate green.

⭐ so **Ambition's answer is now an AUTHORING act rather than a brain edit**, and
the duel fixture can state which rule it fights under instead of inheriting one
silently. ▢ **the product question below stays open and is unchanged** — this
lift deliberately decides nothing about what Ambition wants.

⚠ **worth knowing for the next change of this shape**: `SmashCfg` rides inside
`Brain`, which IS rollback state (`actor.brain`, `component-clone-cursor`) — and
the frozen schema baseline did NOT move. It records type NAMES and snapshot
strategies, not field layouts, so a nested struct can change shape without the
wire-format guard noticing. That is the documented design of a clone snapshot
rather than a hole, but it means "the baseline is green" is not evidence that a
nested type is unchanged.

Two `app_it` duel tests have been failing since before this run. The cause is
measured, not suspected — the same fight, both fighters, identical capabilities:

```
PCA:   shield_frames=535  dash=0   fly=194  fly_toggles=4   blinks=3
robot: shield_frames=0    dash=30  fly=427  fly_toggles=12  blinks=1
```

In `brain/smash/mod.rs`, ARMING the block requires `cfg.can_shield &&
obs.self_on_ground`, and HOLDING it requires `shield_hold_timer > 0.0 &&
obs.self_on_ground`. The robot is airborne for 427 of ~600 frames, so the second
layer of the layered defence (blink → shield) is **unreachable** for it — not
chosen against, unreachable. Nothing says so at the decision site.

- **(a) Air-shielding is legal.** Drop `self_on_ground` from both conditions. The
  layered defence becomes available to a flying body, and shield stops being a
  ground-only tool.
- **(b) It is deliberately ground-only.** Then the DECISION SITE should say so,
  the brain should not keep choosing a defence it cannot enact, and the tests'
  expectations become air-aware rather than symmetric.
- **(c) A flying fighter should land to shield.** The most work, and the most
  like a real fighting game.

⚠ **this is a design call and I should not pick it**: it changes what a fighter
IS, and two of the three answers change the test rather than the game.
⚠ likely fallout from the 08-02 frame-typing fixes (`5ae87b2e8`, `3e7780623`) —
the fighter used to write raw world x into the body-local `locomotion`, so its
movement, closing speeds, and flight decision all changed when that was
corrected. The tests were calibrated against the buggy movement.

> **Row 11 (`bevy_material_ui`: adopted, or 31% of the frame for nothing?) was
> deleted 2026-08-08 — it DISSOLVED rather than being answered, and one of its
> own premises was false.** Working: queue row **D35**.
>
> It asked Jon to rule on his *intent*: leave the dependency trimmed, or put the
> full bundle back because Material is a direction he is part-way into. There is
> nothing to be part-way into. Zero types, components, systems or helpers from
> the crate are named anywhere in the workspace; every menu, the dialogue box and
> the HUD are plain `bevy_ui` with typography this repo owns
> (`MenuFont`, `MenuTextHeightFraction`, `resolve_menu_text_size`). A question
> about intent needs something to intend.
>
> ⛔ **and the row's `⚠ but it is load-bearing` paragraph was WRONG** — the part
> that made the question feel like it needed him. The six-step bisect proved
> *"removing `DialogPlugin` flips the title test"*; the row recorded *"menu
> typography stops resolving"*. Upstream `dialog.rs:25` and `lib.rs:524` install
> no typography at all, and the arithmetic refutes it without reading them: menu
> text spawns at `reference_pixels()` = **60px** and the resolver only corrects
> that, so a dead resolver still passes `>= 32.0`. The 20px reading is Bevy's
> `TextFont::default()` — a different entity.
>
> ⭐ **the lesson is about how the wrong sentence got written down.** The row was
> scrupulous in one direction — *"I am not treating 'unused by grep' as
> 'unused'"* — and credulous in the other, promoting a bisect's correlation to a
> mechanism. Doubting the cheap evidence is not the same as checking the
> expensive evidence.
>
> ⚠ **Jon's override stays cheap and open**: if Material UI *is* a direction he
> wants, saying so puts `MaterialUiPlugin` back in one line. The reason this row
> left is that nothing is waiting on him, not that the door closed.

## 13. Should a crawler's collision volume ORIENT with its attachment? (queue F5, 2026-08-03)

F5 has been open by your call since 2026-07-29 — you saw the contact was wrong
after the motion fix and said to leave it. This puts the number beside it, because
the fix is one decision and it closes two symptoms at once.

### The fact, and it is arithmetic rather than a measurement

The slug is authored **48 × 22**. Its MOTION already resolves extents in the
surface's frame — `body_extents_for` swaps them on a vertical face, which is what
fixed the sticking. **Contact does not**: `kinematics.size` stays 48 × 22 in world
axes while the sprite rotates with the attachment (`FeatureView::rotation_rad`).

On any vertical face the two disagree by 90°:

| | |
|---|---|
| world-axis hurtbox | 48 × 22 = 1056 px² |
| what the drawn slug occupies | 22 × 48 |
| overlap | 22 × 22 = 484 px² |
| **misplaced** | **572 px² — 54% of the hurtbox** |

It protrudes **13 px out of the wall** on each side and falls **13 px short**
along it. So on every vertical surface a player can hit empty air beside the slug
and miss the slug's own head.

⚠ **the same root causes the ~25 px convex pivot pop** — structural for a
non-rotating non-square AABB, since the two placements share no position. It is
currently bounded at 26 px by
`a_slug_wrapping_a_ledge_end_pivots_once_and_keeps_going`.

### The decision

- **(a) Orient the collision volume by the attachment.** Likely closes the contact
  bug AND the pivot pop together, because both are the same disagreement.
  ⚠ it makes the body's world AABB non-constant, which every consumer of
  `kinematics.size` currently assumes. `crawl_chain` carries the same
  transposition and is untouched.
- **(b) Author the slug SQUARE.** The disagreement vanishes at 22 × 22 or 48 × 48
  with no engine change at all, at the cost of the silhouette.
- **(c) Leave it.** Correct if adhesive crawlers stay a curiosity; the cost is
  that every vertical-surface fight against one is wrong in the player's favour in
  one direction and against them in the other.

⚠ **I have not touched it** — F5 is marked open by your call and I am treating that
as standing. This is the number, not a fix.

## ⊘ 14. WITHDRAWN 2026-08-08 — the premise was stale; `?`-blocks already wear their own art

> **Jon: *"I thought the blocks do show an interrobang. It looked fine to me last
> time I saw it."*** He is right and this row was wrong.

⛔ **verified against the tree.** `MaryOBlockLook::Question`
(`game/ambition_demo_mary_o/src/ldtk_vocabulary.rs`) is documented as *"The
?-block. **Wears its own texture**, and an inert one once spent"*, and dresses to
`BonusBlockTile` — the interrobang plate. Mary-O owns a LOOK vocabulary layered
over the engine's `BlockKind`, deliberately (*"the engine must not interpret
Mary-O's progression: LDtk authors WHICH block and WHERE, and this crate decides
what that means"*).

⭐ **what this row got wrong, and it is the day's recurring shape**: it read
`block_tile_sprite(BlockKind)` — the ENGINE resolver, which really does map
`Solid` to one tile — and concluded every block looks the same. It never checked
whether the GAME uses that path for authored blocks. **Mechanism read correctly,
consumer never checked.**

⚠ **the maintainer's recollection beat the document**, which is worth recording:
he answered from having looked at the game, and the row was written from having
read one function.

### (the original analysis, kept for the mechanism it describes)

**In your terms:** *your `?`-blocks do not show a question mark.* Every solid
block in Mary-O 1-1 draws the same tile, because art is chosen from the block's
KIND and a `?`-block is just a solid. The fork is whether to give blocks
per-block art, or to move 1-1 into LDtk where art is authored per tile.

Your `?`-blocks do not show a question mark, and the reason is structural rather
than artistic. `spawn_block` resolves art from `BlockKind` alone
(`block_tile_sprite(Solid) → EntitySprite::SolidTile`), so **every solid block in a
room shares one sprite**.

I added interrobang and spent-block tiles to `super_mary_o_tileset` — and then
found the sheet is consumed by nothing. Chasing that: a generated tileset becomes
live by being named from an `.ldtk` file (`intro.ldtk` references
`intro_lab_tileset.png`; `town_tileset` the same). Mary-O's has no such reference
because **level 1-1 is code-authored**, not an LDtk level. Nothing was lost; there
was never a level to bind it to.

⚠ And that is the exception to AGENTS.md's own stance — *"LDtk owns world/level
authoring"* — which is precisely why a Mary-O block cannot carry its own art.

- **(a) Build the per-block art seam (B13).** `Block::art_sprite: Option<String>`
  resolved through a registry, following `ProjectileVisualId`/`WorldItemArt`, which
  are the same shape for other entity classes. Keeps code-authored levels
  first-class; helps every game; four-part change.
- **(b) Move Mary-O 1-1 to LDtk.** Matches the stated direction, makes the tileset
  live, and the interrobang + spent tiles arrive **for free** — a tile layer already
  does per-cell art. Much larger content migration, and several Mary-O tests drive
  the code-authored level directly.

⚠ **I have not chosen.** (b) is more aligned with where the engine says it is
going; (a) is more aligned with what exists today. That is a product call.


---

## #15 — Mary-O 1-1's first ?-block drops its wand into a pit, and it is uncatchable

**In your terms:** *the first `?`-block in 1-1 gives you a wand, and the wand
falls straight into a pit where you cannot get it.* It is a level-layout
question — move the block, move the pit, or change how the drop travels.

**Measured 2026-08-04, after fixing the two things that were hiding it.** The
acceptance run could never strike that block (it held jump, and a held Mary-O
Classic jump clears a row-4 block by 174 px), and the beat asserted only that it
had not ended early — so three separate diagnoses went looking for the wand
instead of for the bonk. Both are repaired; the block is struck and the wand
spawns.

**The wand then cannot be collected.** It pops at `x = 483` and travels RIGHT,
crossing the first pit's lip at `x = 640` around frame 125 and falling
(`y 400 → 488 → 1132 → 4294`). Reaching it from her post-bonk `x = 222` needs
**334 px/s**; her authored top speed is **300**. Probed by lifting the fixture's
pit-safe clamp to 620: she arrives at 622 by frame 200 and the wand is already at
`y = 1132`. So this is not a fixture problem and not a tuning problem — **the
powerup is placed where the player cannot retrieve it.**

⚠ an item falling into a pit is authentic to the game this converges on; a first
powerup that ALWAYS does is a level-authoring accident. The options are content
calls:

- **(a) Move `power_block_0` left**, away from the pit, so the wand's travel has
  room. Smallest change; the block's column is one constant.
- **(b) Give the wand something to turn on** before the pit — a wall or lip. This
  is what the original does, and `spawn_moving_world_item` already turns at walls.
- **(c) Accept it** and change the acceptance run to get the wand from a different
  block. ⚠ this leaves a first-powerup a player will lose on their first attempt,
  which is the part worth deciding rather than defaulting into.

⚠ **the acceptance test stays RED until this is answered**, and that is now a
faithful red: everything upstream of the collection is proven working in the same
run (`SpentPowerBlocks` records the strike, the item exists at a known position).

---

## Does the world FREEZE during a death beat? (raised 2026-08-05)

**This is the same decision as the dialogue one above, wearing different
clothes** — decide them together or the answers will disagree.

> ⭐ **AND THE DIALOGUE HALF IS NOW ANSWERED (Jon, 2026-08-06), so this row has a
> precedent it did not have when it was written.** Offered the same three-way
> choice, he picked *"Build it, default to per-seat"* — **over** the conservative
> recommendation — so dialogue claims only the TALKER's input, NPCs keep
> patrolling through a conversation, and a couch seat is no longer frozen because
> somebody else is talking (`maintainer-decisions.md`, 2026-08-06).
>
> This row's own argument is that the two must agree, and its "freeze it" branch
> carries exactly the objection he decided against: *"on a couch, one player dying
> freezes the other mid-jump."* Applied consistently, that ruling selects **freeze
> only the ACTORS** or **leave it**, and rules out the global freeze.
>
> ⚠ **it does not close the row**, and deliberately: a death is not a
> conversation, `RoomTransition` and `Cutscene` were explicitly excluded from that
> ruling as a different question, and which of the two survivors to take is still
> a judgement. But the fork is narrower than three, and the "decide them together"
> instruction is half discharged.

**Your report** (`JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`): *"When maryo-dies the
enemies seem to reset before the death animation is finished. The level reset
needs to happen all at once at a time that is easy to express in the game
code."*

**What is measured, not guessed.**
`death_reset_timing::the_room_resets_no_earlier_than_the_death_beat_ends` asserts
the ordering and it holds: the reset is correctly gated behind the beat. The
world does not HOLD STILL during it — the non-player bodies walk through the
dwell and then snap on the frame after it ends. To a player that reads exactly as
*"the enemies reset before the animation finished"*: they were walking the whole
time, and then jumped. So the ordering was never the defect; the absence of a
freeze is.

> ⛔ **CORRECTED 2026-08-08 — two numbers in this row's first draft were wrong,
> and its fixture has since stopped measuring the question.**
>
> **The "~0.55s dwell" was a misreading of the test's own output.** The quoted
> `f163..f195` is the **last screenful of a 194-line log**, not the dwell. The
> beat is `f3..f195` — **192 frames, 3.200s** — and `DEATH_DWELL` has been `3.2`
> since `ecf21b2de` (2026-07-25), so it was already 3.2 when this was written.
> There is no second, shorter beat and no nested animation dwell.
>
> ⛔ **and the drift evidence no longer reproduces in that fixture.** The
> signature now moves only on f4–f15 (0.2s, and *upward*) then sits **constant
> for f15–f195 — 94% of the beat.** The cause is not a fix: the fixture kills her
> at `y = 4000`, ≥3500px from every enemy, and both Mary-O enemies gained
> `AwakeNearObservers { radius: 720 }` *after* this row was written. **Every
> enemy is dormant for the whole fixture beat, so the test no longer exercises
> the freeze question at all** — it would now report a frozen world whether or
> not one was implemented.
>
> ⭐ **the conclusion survives, on new evidence.** In a real death she dies next
> to what killed her, well inside 720px. Across eight captures verified inside
> the beat, the slop sits at screen x = 334, 330, 306, 248, 273, 279, 275, 202
> with the camera anchored. The world keeps living; the snap is real.
>
> ⚠ **so this row needs a fixture that kills her NEAR an enemy** before its
> measurement means anything again. A dormancy radius silently turned a freeze
> instrument into a no-op — the same class as an instrument shaped exactly like
> the bug it hunts.

**The mechanism already exists and is the one to reuse.**
`GameMode::{Paused, Dialogue, RoomTransition, Cutscene}` suspend gameplay through
`apply_suspended_time_scale_system`, which sets `ClockState::time_scale = 0.0`
under the `gameplay_suspended` run condition. A suspended world still advances
its TIMELINE — ticks count, so hashes and recorded input frames stay aligned —
but moves zero sim seconds. That is rollback-safe by construction and needs no
new concept: a death beat is a world-stopping beat in the same family as a
cutscene.

**The decision:**

* **freeze it** — the death beat suspends gameplay like a cutscene does. The
  enemies hold, the reset lands on the frame the beat ends, and "all at once at a
  time that is easy to express" becomes literally true: one mode, one clock.
  ⚠ **but on a couch, one player dying freezes the other mid-jump** — which is
  the identical objection the dialogue row above is stuck on, and the reason
  these two must be answered together.
* **freeze only the ACTORS** — reuse `DormancyPolicy`'s machinery rather than the
  clock: the beat puts non-player brains to sleep and clears their control frames
  (which is exactly what dormancy already does, and why a sleeping actor stops
  rather than drifting). Physics still runs, so a falling enemy still falls, and
  the other player keeps playing. Costs a beat-scoped trigger on a seam that
  exists for a distance-scoped one.
* **leave it** — accept that the world keeps living while you die, and the snap
  is the cost. Cheapest, and the honest reason to pick it is that neither of the
  above is free on a sofa.

⚠ **the test deliberately does NOT assert either way**, and should stay that
way until this is answered: pinning today's behaviour would be a regression test
over unpolished behaviour. It prints the measurement instead.

---

## Hitstun keeps ATTACK and strips RECOVERY — is that right for a body knocked OFF the stage? (raised 2026-08-05, investigated 07-31…08-04)

**The symptom you reported** was the ladder losing all three stocks. It is not a
brain bug and it is not a physics bug; both were measured and cleared. It is this
gate, and the numbers are exact.

`apply_post_hit_input_gates` — whose own doc says it applies to *"ANY body's
FINAL `InputState`"* — runs two windows after a hit:

| window | duration | what the body may do |
|---|---|---|
| recoil lock | **0.12 s** | nothing: axes zeroed, Jump/Dash/Blink/FastFall all `Edge::NONE` |
| hitstun | **0.24 s** (enemy) / **0.36 s** (boss) | movement at `hitstun_control_scale` **0.18**, attack PRESERVED, **Jump/Dash/Blink still `Edge::NONE`** |

So a knocked fighter has **no recovery verb at all for 0.36 s after an enemy hit
and 0.48 s after a boss hit**, while carrying the full launch velocity. The brain
picks `Recover` at weight 1.00 and `Recover`'s emit presses jump; the gate sets
that press to `Edge::NONE`. It was never a question of which verb the brain chose,
and two earlier investigations died proving that.

**The shape is the interesting part.** Hitstun keeps ATTACK and strips RECOVERY,
which is deliberate and documented — *"you can fight back … the instant the
recoil lock ends"*. That is right for a body knocked around ON the stage and
exactly backwards for the case that loses stocks: a fighter launched off the edge
has nothing to attack and one thing to do. **The gate cannot currently tell those
two situations apart**, and that — not a duration — is the thing to decide.

**The decision:**

* **(a) let the recovery verbs through during hitstun**, keeping only the 0.12s
  recoil lock as a hard window. Smallest change. ⚠ **it changes on-stage combat
  feel too** — a juggled fighter gets its jump back mid-combo — which is the
  trade worth stating rather than discovering in playtest.
* **(b) make the strip depend on whether the body is over the stage**, which is
  the distinction the gate is missing. Most correct, and it introduces "over the
  stage" as a concept the gate has to be able to ask about.
* **(c) shorten hitstun for launches that leave the stage.** A tuning answer to a
  structural question; cheapest to try and the easiest to get subtly wrong,
  because it makes the fix a number rather than a rule.

⚠ **this is a FEEL decision and belongs to you.** All three are small to
implement; none is small to judge.

---

## Are the three named robot heavies characters, or is `robot_heavy` variant art? (raised 2026-08-06)

**Why it surfaced now, and why it is not a bug report.** The
`rendered_identities_are_registered` check reads every renderer target's declared
`character_id` and requires a catalog row. It matched the key `"character_id"`
literally — with double quotes — and four targets spell theirs with apostrophes
because a formatter wrote them. `carl_stargan`, `bear_mauler` and
`special_patent_clerk` turned out to be registered anyway. `npc_robot_heavy` is
the one real orphan the blindness was hiding, and it is a question rather than a
fix.

**What the target actually is.** `robot_heavy.py` declares the bare id
`npc_robot_heavy` / "Robot Heavy" and then three fully-specified variants —
**Bastion Bruiser** (red, maul, shoulder pods), **Foundry Ram** (orange, furnace
backpack, pile driver), **Volt Crusher** (blue, electrical). Each is a complete
`VariantSpec`: its own palette, silhouette scales, head, weapon and backpack.
They are drawn as three different robots, not three tints of one.

**What is true today.** None of the four is in the catalog. `regen_sprites.sh`
skips the target entirely with the note *"multi-variant rig whose publisher
doesn't install (renders only to `generated/`) … Catalog entry was dropped along
with the publisher work"*, so no sheet ships either. This is the only rendered
identity in the tree that is uncast in BOTH directions.

**Why it is not the `pirate_heavy` case, which you already ruled on.** Your
answer there was *"the named pirate heavies are the pirate heavies"* — the bare
family id has no row because broadside_bess, iron_mary and salt_annet are the
characters and all three ARE cast. Applying the same words here would leave all
four robot heavies uncast, which is the opposite outcome from the same sentence.

**The decision:**

* **(a) cast the three named ones**, exactly as the pirate heavies were: a row
  each from the target's own metadata, three Hall pedestals. Needs the publisher
  work the regen note describes (the target installs nothing today), so it is the
  most expensive answer and the one that puts three new robots in the game.
* **(b) cast the bare `npc_robot_heavy`** as one enemy archetype and treat the
  three specs as its palette variants. Cheapest, and it matches how the target is
  currently *skipped* rather than how it is *written* — the variants have
  distinct silhouettes, so this discards authored difference.
* **(c) it is variant art, not characters** — keep the waiver and say so once, so
  nobody re-raises it. Free, and it means the render cost stays paid for art
  nothing loads.

⚠ **weak recommendation: (a), but only if these robots have a place to be.** The
art is already drawn and specified; the missing half is a publisher, which is
mechanical. But casting three enemies nobody has a room for is content debt, and
you are the one who knows whether the robot heavies belong to a faction that is
going to exist.

---

## Is a SESSION scope marker construction provenance, the way a ROOM scope marker is? (raised 2026-08-07)

The last open row in `tracks.md`'s rollback-anchor sweep, narrowed today from
"an archetype the engine does not honour" to **one component**.

`no_snapshot_registration_is_inert_*` reports entities whose components are
registered as rollback state while the entity carries no rollback anchor. The
sweep skips a known-safe class via `PROVENANCE_ONLY`, whose argument is that
construction provenance *is written ONCE and never again (ADR 0030), so a rewind
that does not restore it restores exactly the value it already holds.*

The reported archetype is `SpawnOrigin + TransactionId + SimId +
RoomScopedEntity + SessionScopedEntity + Name`. **Five of those six are in
`PROVENANCE_ONLY`.** It is reported at all because of exactly one:
`SessionScopedEntity`, which IS registered rollback state (`scope.session`,
`domains/primitives.rs:110`) and is absent from that list.

**So the question is a short asymmetry.** `RoomScopedEntity` and
`SessionScopedEntity` are both write-once scope markers stamped at construction,
and the provenance argument above reads identically for both. Why is one in and
one out?

⛔ **and there is a real reason it might be deliberate, which is why this is not
mine to answer.** The sibling waiver for `Messages<SessionScopeRetired>` says a
rewind *"must not un-retire a scope"*. Session lifetime has a rewind rule that
room lifetime does not. Whether that rule reaches the MARKER as well as the
retirement message is the entire decision.

**The decision was going to be:**

* **(a) it is provenance** — add `SessionScopedEntity` to `PROVENANCE_ONLY`
  beside `RoomScopedEntity`, on the grounds that a marker stamped once cannot be
  restored wrongly. One line, and the sweep's last open class closes.
* **(b) it is lifecycle** — the marker participates in scope retirement, so a
  rewind restoring it could resurrect membership of a retired scope. Then it
  stays out, and what is owed is a WAIVER that says this in one sentence.

---

### ✔ ANSWERED 2026-08-08 — and BOTH options rest on a premise that is false

Jon, 2026-08-08: *"Yes, read `SessionScopedEntity` to determine what is going on
with that. I've left rollback design to agents."* So this is answered rather than
asked, per his operating note the same day. **Overrule freely; the reasoning is
kept legible below rather than compressed into a verdict.**

⛔ **Neither marker is write-once.** The sentence above — *"both write-once scope
markers stamped at construction"* — is the shared premise of (a) and (b), and
possession falsifies it for **both**. `abilities/traversal/possession.rs`:

```rust
// :342 — on possess, the body is PROMOTED to session scope
target_cmds.remove::<RoomScopedEntity>()
           .remove::<SessionScopedEntity>()
           .insert(SessionScopedEntity(scope_id));

// :385 — on release, the EXACT prior scope is restored
ec.remove::<RoomScopedEntity>().remove::<SessionScopedEntity>();
match restore_scope {
    PossessionRestoreScope::Unscoped => {}
    PossessionRestoreScope::Room => { ec.insert(RoomScopedEntity); }
    PossessionRestoreScope::Session(id) => { ec.insert(SessionScopedEntity(id)); }
}
```

The `PossessionRestoreScope` enum (`:47`) exists *because* the scope changes and
has to be put back. A type whose whole job is remembering the previous value is
proof the value is not write-once.

**So the asymmetry is real, and it points the other way from what the row
assumed:**

| | registered rollback state | in `PROVENANCE_ONLY` | actually write-once |
|---|---|---|---|
| `SessionScopedEntity` (`primitives.rs:110`) | yes | no | **no** |
| `RoomScopedEntity` (`primitives.rs:106`) | yes | **yes** (`rollback_coverage.rs:1954`) | **no** |

⇒ **`SessionScopedEntity`'s exclusion is CORRECT** — it is mutable rollback state
and belongs in the snapshot. Neither (a) nor (b) is the answer; the question
"why is this one out?" had a wrong presupposition.

⇒ ⛔ **`RoomScopedEntity`'s PRESENCE is the defect.** Its exemption is justified
by the list's own argument — *"written ONCE and never again, so a rewind that
does not restore it restores exactly the value it already holds"* — and a rewind
across a possess/release boundary is exactly the case where that is false. The
exemption tells the sweep not to care about the one transition that can be wrong.

**The call, and what it costs.** `RoomScopedEntity` comes off `PROVENANCE_ONLY`.
If the sweep then goes red, the red is correct and names real work — an entity
carrying mutable rollback state with no rollback anchor is the exact thing the
sweep exists to find.

⚠ **not landed yet, deliberately.** Per the standing lesson, the **probe comes
before the guard**: a rewind across a possession that asserts room scope comes
back, watched failing first. A list edit that turns something red without a
demonstrated failure produces a guard nobody trusts, and five first-draft guards
in one day were already green through their own motivating case.

⚠ **the third fact below still stands and is still the more useful work.** Both
`no_snapshot_registration_is_inert_*` assertions PASS, so this archetype appears
in NO composition any test sweeps — it exists only under the shell. The report
now names the offending entity, but that helps only once a test reaches that
composition. **The sweep is green about a class it never looks at**, and that is
true whichever way the classification goes.

⭐ **how the wrong question got asked, since it is the transferable part.** The
row read the two registration sites and the exemption list — three tables — and
concluded from their *shape* that both markers were construction-stamped. Nothing
in those three places records who WRITES the component. The one file that does is
`possession.rs`, which no part of the investigation had reason to open. This is
`validate_the_state_around_the_table`: the tables agreed with each other and were
collectively wrong about the world.

---

## How should the portal map convention stop being a process global? (raised 2026-08-07)

**In your terms:** *portal orientation is decided by one switch shared by the
whole process, so two games running in one binary cannot disagree about it —
and nothing owns the switch.* It works today because only one game uses portals.
The decision is where that convention should live instead.

`closeout-review-followups-2026-07-20.md` §2 recorded this and it is still true:
`ambition_platformer2d_shared_tangle::math` holds
`static PORTAL_MAP_ROTATION: AtomicBool`, and `portal_map_vec` dispatches on it.
That function's own doc calls it *"one orthogonal map shared by velocity,
position, AABB, input, and rays so they always agree"* — so the global is the
single switch under the entire portal map.

⭐ **the harm is concrete, not theoretical.** `sync_portal_tuning_convention` runs
**every frame** (`PortalSet::InputAdapter`), writing that App's `PortalTuning`
into the process-wide static. Two Apps in one process therefore fight over it once
per frame — and a parallel test binary IS two Apps in one process. Today nothing
collides only because every composition defaults to `Reflection`.

**Already done (2026-08-07):** `transfer_step`'s three DERIVED facts — roll,
facing flip, input warp — read `tuning.convention` instead, because
`PortalTuning` was already in its argument list. That is the small half.

**The measured cost of the rest.** `portal_map_vec` has 5 non-test callers, but
the helpers wrapping it do not:

```text
  map_point                          14 call sites
  portal_transform_velocity           3
  map_aabb                            2
  rotate_velocity_between_normals     0  (a rename of portal_transform_velocity)
```

Threading a convention parameter therefore reaches ~19 sites, many in geometry and
presentation code with no `PortalTuning` in scope.

**The decision:**

* **(a) thread it** — a `MapConvention` argument through all ~19. Honest and
  total; ⚠ this is the shape that cascaded on the fighter ladder the same day
  (323 lines, never compiled), and for the same reason: a value needed at leaves
  and owned at a root.
* **(b) leave the global, make its WRITE session-owned** — once at session start
  rather than every frame. Cheap, and ⛔ does not fix two Apps: it narrows the
  window rather than closing it.
* **(c) follow the crate's own precedent** — `portal_map_vec` gains a
  `*_for_convention` form, `map_point` / `map_aabb` / `portal_transform_velocity`
  gain theirs, and the SIMULATION callers migrate while presentation keeps the
  convenience wrappers that read the global. This is exactly what
  `somersault_roll` / `somersault_roll_for_convention` already are, and what made
  today's fix one `let` plus three swaps.

⚠ **weak recommendation: (c)**, because the crate has already solved this shape
once and the result is the reason the derived-facts half was cheap. But it leaves
a global that presentation still reads, so it is a judgement about whether "the
simulation is session-authoritative and the HUD is not" is an acceptable
resting place — and that is a call about the engine's own standard, not a
refactor anyone should pick unilaterally.

⭐ **a canary already exists either way**:
`a_transit_takes_its_convention_from_tuning_not_the_process_global` asserts the
global is at its default before it runs, so the first test that writes it turns
the contamination into a failure with a name.

---

## Does the Perfect Cellular Automaton fly when it fights? (raised 2026-08-07)

Promoted out of [`review-gpt56-through-32eb27a.md`](review-gpt56-through-32eb27a.md)
P5, where it was recorded as *"the fix is not a resolution rule, it is a
DECISION"* and then sat in a review ledger rather than where decisions are read.

⭐ **it became a clean two-way question today**, because the type change that made
it expressible landed: `ArchetypeSpec::is_aerial` is `Option<bool>` now, so an
archetype can say *grounded* distinctly from saying nothing.

### The two answers, both of them stated in content

| source | says | read by |
|---|---|---|
| `character_catalog.ron` — `body_kind: Floating` | it FLIES | `new_peaceful_npc_in` (the NPC placement path) |
| `character_archetypes.ron` — `is_aerial: Some(false)` | it is GROUNDED | the hostile `EnemySpawn` path, and the duel arena |

⛔ **so this is not a silence to resolve with a precedence rule.** Before today it
looked like one — a bare `bool` could not distinguish "the archetype says
grounded" from "the archetype never said", so "the catalog wins when the
archetype is silent" was a plausible fix. It is not available: somebody authored
`false`. Two authors said opposite things and both meant it.

**What it costs either way.** The PCA is a shipped fighter and the duel arena
plays it grounded (`actor_movement_tests` asserts exactly that, and now asserts
`Some(false)` so the deliberateness is pinned). Making it fly changes a shipped
fight; making the catalog's `Floating` a lie changes what a Hall visitor sees.

**The decision:**

* **(a) it flies** — the catalog is right, and `cellular_automaton_fighter` should
  author `Some(true)`. ⚠ changes the duel; `actor_movement_tests`' two assertions
  are the ones that go red, and they would be recording the new answer rather
  than a regression.
* **(b) it is grounded** — the archetype is right, and the catalog's `body_kind`
  should stop saying `Floating` for this character. Cheapest, and it matches what
  ships today.
* **(c) it is BOTH, legitimately** — a body that floats as scenery and fights on
  the ground. Then neither source is wrong and what is missing is that the two
  paths do not know they are answering different questions; the fix is a name,
  not a value.

⚠ **weak recommendation: (b)**, purely because it is what the shipped game does
and the change is one authored word. But (c) is the interesting one — the PCA is a
cellular automaton, and "floats when idle, grounds to brawl" is a real character
idea rather than a reconciliation. That is a design call and it is why this is
here rather than in a queue.

⭐ **what does NOT need the answer**, and is already done: the type change, so the
contradiction is expressible; and the note that assembly must not REJECT
contradictions until this is settled, because rejecting would refuse the PCA
today.

---

## ✔ ANSWERED 2026-08-08 — May a game compose this engine WITHOUT a given capability?

> **Jon, verbatim: *"it's a resounding yes on compose without capability. I think
> the portal crate is close to that. but we need to carefully design it. I don't
> want it to add overhead. it should add conceptual clarity."***

**Moved to [`maintainer-decisions.md`](maintainer-decisions.md).** The analysis
that produced the question is in git; what survives here is the constraint,
because it is the part that shapes the work: **no overhead, and it must add
conceptual clarity.** That excludes the mechanical implementation — `#[cfg]`
threaded through the interact dispatch, the input capture and ten rollback
registrations — and it names an exemplar to generalise from instead.

## Should an AUTHORED id be unspellable by a runtime spawn? (raised 2026-08-08)

**In your terms:** *could something spawned during play accidentally end up
with the same identity as something you authored in a level?* Today the id
formats overlap, so it is possible in principle. **No symptom exists today** —
nothing in the game currently spells a colliding id. Making it impossible costs
a ~70-site change; leaving it means the rule is "nobody has done it yet".

**The last of four identity-contract violations found on one day**, and the only
one that cannot be fixed by construction without a decision.

`netcode.md` N3.1 has said since 2026-07-06 that a dynamically-spawned entity
takes `(spawner SimId, per-spawner counter)`. Three sites diverged: drops minted
nothing, the construction executor minted an id with no counter, and
`spawn_split_offspring` mints **`SimId::placement(..)` — the AUTHORED namespace —
for a runtime spawn.** The first two are now impossible (`#[require(SimIdCounter)]`
welds the pair). The third is not.

**Why it is not a checker.** Measured: **70 `SimId::placement` call sites**, and
nearly all are correct — the construction executor naming authored features, room
staging, probes, tests. The distinction a checker needs is semantic: *was this
entity authored in a room spec, or minted while the sim ran?* Only the call site
knows, and a contract that cannot tell them apart is noise that teaches a reader
to waive it.

**The decision:**

* **(a) Make it unspellable.** `SimId::placement` takes a `PlacementId` carried
  out of the room spec instead of a `&str` anyone can hand it. A runtime spawner
  then cannot name an authored id **because it has nothing to name it with** —
  the same shape as `#[require(SimIdCounter)]`, which is what made the counter
  half stop being a thing to remember. ⚠ **~70 call sites**, and the newtype has
  to be threaded out of the room spec to each one. A real refactor, not a slice.
* **(b) Fix the one site and leave the rule to prose.** `spawn_split_offspring`
  starts minting `SimId::spawned(parent, n)` like everything else, and the
  namespace rule stays a convention `SimId::as_str`'s doc already states. Cheap,
  and the next runtime spawner can make the same mistake.
* **(c) Leave it.** ⚠ **defensible today**: the offspring are claimed as staged
  actors, so nothing is player-visible. It is an identity-provenance defect with
  no current symptom.

⚠ **what makes this yours rather than mine**: (a) is the only answer that matches
how the other three were fixed — make the wrong thing unrepresentable — and it is
also the only one with a cost you would notice. **The three cheap fixes today all
turned out to be one-line consequences of a type; this one is not.**

⭐ **what does NOT need the answer**: the counter half is already closed, and the
whole contract is now written up in [`engine/netcode.md`](engine/netcode.md) with
the three violations and which are impossible now.

---

## Should "this character declared no abilities" mean "give it the dev kit"? (raised 2026-08-08)

You have already ruled on the narrow version of this. Your observations file
(obs:248) says of Sanic's blink: *"it is a QUESTION FOR YOU rather than a bug …
which is the documented 'the box keeps its traversal kit' design. **Whether Sanic
should keep blink is product, not repair** — say the word either way."* That
question is still open and unchanged.

**This row is the BROADER one a GPT 5.6 review raised on 2026-08-08**, and it is
a different question with a different blast radius, so it is asked separately
rather than folded into yours.

### The fact

`session/setup.rs:199-201`:

```rust
let base_abilities = character_catalog
    .ability_set(starting_character.effective_id(default_character_id))
    .unwrap_or_else(|| editable_abilities.as_engine());
```

Its own comment states the rule plainly: *"A row without an authored set keeps
that shared sandbox set."* And `platformer_defaults.ron:18-21` grants
`blink: true`, `precision_blink: true`, `blink_through_soft_walls: true`,
`blink_through_hard_walls: true`.

So **absence of a declaration is read as consent to the full development
traversal kit** — for every character, not just Sanic.

### The fork

* **(a) Absence means the BASE kit, and the dev kit is opt-in.** A character that
  declares nothing gets ordinary movement; the sandbox set becomes something a
  session explicitly asks for. ⭐ this is the direction the decomposition brief
  wants (*"prepared character/body definition → intrinsic body capabilities →
  explicit session/dev restriction or augmentation policy"*), and it makes the
  capability seam natural rather than accidental.
  ⚠ **it changes what every undeclared character can do**, which is a content
  sweep of unknown size until someone counts the undeclared rows.
* **(b) Keep it. Absence means the dev kit, deliberately.** ⚠ **defensible, and
  it is the CURRENT documented design** — the box keeps its traversal kit, and
  Ambition's own protagonist is exactly the case it serves. Then the fix is
  declaration hygiene: characters that should not blink say so.
* **(c) Split the difference**: absence means the base kit for a character worn
  by a *game*, and the dev kit only under the sandbox/dev host. ⚠ two answers to
  one question keyed on composition — the shape this repo keeps finding as a
  defect, so it needs a reason beyond convenience.

### What makes this yours

The review calls the current behaviour a bug. **You have already looked at the
same mechanism and called it documented design.** Both readings are coherent —
it depends on whether "the box keeps its traversal kit" is a property of the
BOX (yours) or an accident of a missing declaration (the review's). That is a
product statement about what a character IS, and no measurement decides it.

⭐ **what does NOT need the answer**: shaping the character-preparation seam so
body-owned intrinsic capabilities are *expressible* is architecture and can
proceed either way. Only flipping the default is blocked.

---

## ✔ ANSWERED 2026-08-08 — Should the guard run a SMALLER suite on a prose-only turn?

> **Jon: *"I want to bias towards running less tests to balance out the agent urge
> to run more. So yes run a much much smaller suite for docs only changes. We will
> catch regressions eventually."*** YES — moved to `maintainer-decisions.md`.

⭐ **this one has measurements, which is why it is askable at all.** `goal_guard.py`
now times its own checks (`.goal/check_cost.jsonl`, landed `0af0e718b`).

### The fact

Five recorded Stop checks:

| # | total | `cargo check` | `app_it` |
|---|---|---|---|
| 1 | 161.1 s | 0.5 | 158.4 |
| 2 | 134.3 s | 0.5 | 131.9 |
| 3 | 212.0 s | 0.7 | 208.8 |
| 4 | 208.8 s | **20.9** | 185.2 |
| 5 | 235.3 s | **16.7** | 216.0 |

**`app_it` is 94.6% of all check time.** `cargo check` is 0.5 s warm and 17–21 s
after a Rust edit — a 41x spread, and irrelevant either way at that size.

`.goal/state.json` records **114 blocks** between 01:48 and 13:05 on 2026-08-08.
At ~158 s that is **≈5 hours of suite inside an 11.3-hour window**, and most of
those turns edited only planning prose.

`app_it` is ONE binary with 96 `mod` submodules and already supports
`--test app_it -- <module_name>`, so a subset needs no restructuring.

### The fork

* **(a) Leave it.** Every turn pays the full suite. ⚠ **defensible, and it is the
  reason the guard is trustworthy**: nothing is ever skipped, so no turn can hide
  a regression behind a judgement about its own diff.
* **(b) Whitelist.** Anything outside `docs/` forces the full suite; a turn that
  touched only `docs/` runs a named smoke subset. ⭐ the failure mode is a slower
  turn, never a missed regression — a generated file, a submodule pointer or a
  `.ron` all fall outside `docs/` and force the full run.
* **(c) Blacklist** ("skip the suite when no `.rs` changed"). ⛔ **not
  recommended, and named only to rule it out**: it makes "only docs changed" a
  claim about a diff, and this repo has been bitten by claims about diffs
  repeatedly. Assets and generated files are not `.rs` and change behaviour.

### What makes this yours

It **weakens a gate on purpose**, and *"we skipped the suite"* is exactly how a
regression hides. The measurement earns the proposal; it does not make the
decision. ⚠ if you take (b), the subset must be **named modules with a reason
each**, not "the fast ones" — a subset chosen by runtime is one nobody can
defend, and it will rot silently as tests are added.

⭐ **what does NOT need the answer**: the timing keeps accruing either way, and
at n≈50 it will also answer whether background subagents materially tax the
gate (five points currently order perfectly by load, but `load_after` is not
independent of the suite, so that is a hint rather than a finding).
